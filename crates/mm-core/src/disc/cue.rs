// (C) 2025-2026 MWBM Partners Ltd
//
// Cue sheet parsing.
//
// A cue sheet is the standard plain-text sidecar for a raw disc image: it
// says which image file holds the audio, where each track starts (as an
// mm:ss:ff timestamp, 75 frames to the second — the native unit optical
// discs are addressed in), and usually the performer and album title. Ripping
// tools (EAC, cdrdao, dBpoweramp, etc.) all write slightly different but
// broadly compatible dialects of the same grammar.
//
// Two things shape every decision in this file:
//
//   1. A malformed cue sheet must never stop a scan. Real cue sheets in the
//      wild are inconsistently quoted, inconsistently encoded, and sometimes
//      simply hand-edited into a broken state. `parse_str` therefore only
//      fails when the text contains no recognised cue command whatsoever —
//      everything else parses as far as it can and records a warning for
//      whatever it had to skip. A caller that gets back a sheet with no
//      title just falls back to the folder name; it must never crash.
//
//   2. `index01_frames` is not just a convenience getter. Issue #218 will
//      compute a disc fingerprint (in the style of a MusicBroken/MusicBrainz
//      Disc ID) from exactly the INDEX 01 frame offset of every track, so
//      this parser keeps every track's raw `CueIndex` list rather than
//      collapsing it down to "the start time" early — that work is reused,
//      not thrown away, by the later stage.
//
// British English throughout ("recognised", "behaviour" in comments), per
// house style.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{MmError, MmResult};

/// Maximum size of a file this parser will treat as a cue sheet. A cue sheet
/// is a short plain-text track listing — genuinely enormous ones (megabytes)
/// are never legitimate and are far more likely to be a misnamed disc image
/// itself, so reading the whole thing into memory to decode/parse it would
/// be wasted work at best and a memory-exhaustion footgun at worst.
const MAX_CUE_BYTES: u64 = 1024 * 1024; // 1 MiB

/// The sector format of a single track, from the `TRACK nn <TYPE>` line.
///
/// Variant names mirror the on-disc keyword as closely as Rust identifiers
/// allow (`/` becomes `_`, since `MODE1/2352` cannot be an identifier).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackType {
    /// Plain CD audio (2352-byte sectors, no header/ECC — it's a raw PCM
    /// stream).
    Audio,
    /// CD+Graphics: audio plus an interleaved subchannel carrying karaoke-
    /// style graphics. Not counted as `is_audio` here — see that method's
    /// comment for why.
    Cdg,
    /// Mode 1 data, sector payload stripped down to the 2048 bytes of user
    /// data (header/ECC discarded). The common shape for a plain data CD.
    Mode1_2048,
    /// Mode 1 data, full raw 2352-byte sector (header + user data + ECC).
    Mode1_2352,
    /// Mode 2 Form 2 data (used by Video CD and some mixed-mode discs),
    /// 2336-byte sector.
    Mode2_2336,
    /// Mode 2 data, full raw 2352-byte sector.
    Mode2_2352,
    /// CD-i data, 2336-byte sector.
    Cdi2336,
    /// CD-i data, full raw 2352-byte sector.
    Cdi2352,
    /// Any track type keyword this parser doesn't recognise by name. Kept
    /// verbatim (upper-cased) rather than discarded, so a caller can still
    /// see what the sheet actually said.
    Other(String),
}

impl TrackType {
    /// Whether this track type holds playable audio (as opposed to computer
    /// data).
    ///
    /// `Cdg` deliberately returns `false` here even though it *carries*
    /// audio: its sectors interleave a subchannel of graphics data, which
    /// means its frame addressing is not the plain audio-CD shape that
    /// `index01_frames`/the future disc-fingerprint calculation (issue
    /// #218) assumes. Treating a CD+G disc as "all audio" would silently
    /// produce a wrong fingerprint rather than a missing one.
    #[must_use]
    pub fn is_audio(&self) -> bool {
        matches!(self, Self::Audio)
    }

    /// The sector size in bytes this track type implies, or `None` for an
    /// unrecognised type where no size can be inferred from the name alone.
    #[must_use]
    pub fn sector_size(&self) -> Option<u32> {
        match self {
            Self::Audio => Some(2352),
            Self::Cdg => Some(2448),
            Self::Mode1_2048 => Some(2048),
            Self::Mode1_2352 => Some(2352),
            Self::Mode2_2336 => Some(2336),
            Self::Mode2_2352 => Some(2352),
            Self::Cdi2336 => Some(2336),
            Self::Cdi2352 => Some(2352),
            Self::Other(_) => None,
        }
    }
}

/// The encoding of the file named on a `FILE` line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    /// Raw little-endian binary — a headerless sector dump. This and
    /// `Motorola` are the two shapes that mean "this FILE is itself a disc
    /// image", which is what `FileType::is_disc_image` and, in turn,
    /// `CueSheet::describes_disc_image` key off.
    Binary,
    /// Raw big-endian ("Motorola byte order") binary — otherwise identical
    /// to `Binary`.
    Motorola,
    /// A WAVE audio file (used when a rip is stored as individual/whole
    /// `.wav` rather than a raw image).
    Wave,
    /// An AIFF audio file.
    Aiff,
    /// An MP3 audio file.
    Mp3,
    /// Any FILE type keyword this parser doesn't recognise by name, kept
    /// verbatim (upper-cased).
    Other(String),
}

impl FileType {
    /// Whether a `FILE` of this type is itself a disc image (as opposed to
    /// a compressed/already-decoded audio file the cue sheet merely
    /// indexes).
    #[must_use]
    pub fn is_disc_image(&self) -> bool {
        matches!(self, Self::Binary | Self::Motorola)
    }
}

/// One `FILE "name" TYPE` line: which file holds the track data that
/// follows it in the sheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CueFile {
    /// The file name exactly as written in the sheet (not resolved against
    /// a directory — that's the caller's job, since the cue sheet may have
    /// been read from a different location than the image it describes).
    pub name: String,
    /// The declared encoding of that file.
    pub file_type: FileType,
}

/// One `INDEX nn mm:ss:ff` line within a track.
///
/// Index 0 conventionally marks the start of the pre-gap and index 1 the
/// start of playable audio; higher indexes are rare sub-track markers. We
/// keep every index a sheet declares rather than only INDEX 01, both
/// because it costs nothing and because a caller may one day care about
/// pre-gaps for gapless-playback purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CueIndex {
    /// The index number (0-99 by the spec; stored as written, not range-
    /// checked, since an out-of-range index is still useful information).
    pub number: u8,
    /// Position in frames from the start of the FILE this index's track
    /// belongs to: `(mm * 60 + ss) * 75 + ff` — 75 frames per second is the
    /// native addressing granularity of the CD-DA / CD-ROM sector format.
    pub frames: u32,
}

/// One `TRACK nn TYPE` block: a single track and everything nested under it
/// (its own PERFORMER/TITLE, ISRC, indexes, gaps and flags).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CueTrack {
    /// Track number as written (1-99 by the spec; see `parse_str`'s doc
    /// comment for what happens outside that range).
    pub number: u8,
    /// Sector format of this track.
    pub track_type: TrackType,
    /// Index into the parent `CueSheet::files` of the FILE this track's
    /// audio/data lives in — always the most recently seen FILE line at the
    /// point this TRACK line appeared, which is the standard cue sheet
    /// convention for multi-file (one-file-per-track) rips.
    pub file_index: usize,
    /// Per-track PERFORMER, if the sheet overrides the disc-level one.
    pub performer: Option<String>,
    /// Per-track TITLE.
    pub title: Option<String>,
    /// International Standard Recording Code, if present.
    pub isrc: Option<String>,
    /// Every INDEX line seen for this track, in the order the sheet gave
    /// them (not sorted or deduplicated — a malformed sheet's ordering is
    /// itself diagnostic information a caller might want).
    pub indexes: Vec<CueIndex>,
    /// PREGAP duration in frames, if the sheet declares one explicitly
    /// (rather than via an INDEX 00).
    pub pregap_frames: Option<u32>,
    /// POSTGAP duration in frames.
    pub postgap_frames: Option<u32>,
    /// FLAGS tokens (e.g. `DCP`, `4CH`, `PRE`, `SCMS`) exactly as written.
    pub flags: Vec<String>,
}

/// A parsed cue sheet: disc-level tags, the file(s) it indexes, and the
/// track list.
///
/// Nothing here is resolved against the filesystem — `files[].name` is
/// exactly the string the sheet wrote, and may not even exist relative to
/// wherever the sheet itself was read from. Resolving that is the caller's
/// job, once it knows which directory the sheet came from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CueSheet {
    /// Disc-level PERFORMER (the album artist), if set before the first
    /// TRACK line.
    pub performer: Option<String>,
    /// Disc-level TITLE (the album title).
    pub title: Option<String>,
    /// CATALOG line (UPC/EAN barcode), if present.
    pub catalog: Option<String>,
    /// Every `REM key value` line, key upper-cased, in the order the sheet
    /// gave them. Kept as a list rather than a map because some tools write
    /// the same REM key more than once (e.g. a stray duplicate comment) and
    /// throwing that away would be lossy for no benefit.
    pub rem: Vec<(String, String)>,
    /// Every FILE line, in order — `CueTrack::file_index` indexes into this.
    pub files: Vec<CueFile>,
    /// Every TRACK block, in the order the sheet declared them (which is
    /// assumed, as usual for a cue sheet, to also be playback order).
    pub tracks: Vec<CueTrack>,
    /// Anything this parser had to skip or guess at, in human-readable form,
    /// in the order encountered. An empty list does not guarantee the sheet
    /// was fully understood — it only means nothing it recognised looked
    /// wrong.
    pub warnings: Vec<String>,
}

impl CueSheet {
    /// Parse cue sheet text that has already been decoded to a `String`
    /// (see [`decode_cue_bytes`] for turning raw sidecar bytes into one).
    ///
    /// # Errors
    ///
    /// Returns `Err` only when `text` contains not a single line this
    /// parser recognises as a cue sheet command — i.e. the input is not a
    /// cue sheet at all. Any other malformed content (bad quoting, an
    /// out-of-range track number, an INDEX with no preceding TRACK, ...)
    /// parses as far as possible and is recorded in `CueSheet::warnings`
    /// instead, by design: a scan must never abort because one sidecar file
    /// was slightly broken.
    pub fn parse_str(text: &str) -> MmResult<Self> {
        let mut sheet = Self::default();
        // Whether at least one line was recognised as a cue sheet command,
        // regardless of whether its arguments made sense. This is the only
        // thing that decides Ok vs Err — see the doc comment above.
        let mut recognised_any = false;
        // The track currently "in scope" for PERFORMER/TITLE/INDEX/etc — an
        // index into `sheet.tracks`, or `None` before the first TRACK line.
        let mut current_track: Option<usize> = None;

        for (zero_based_line_no, raw_line) in text.lines().enumerate() {
            let line_no = zero_based_line_no + 1;
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            let (command, rest) = split_command(line);

            match command.to_ascii_uppercase().as_str() {
                "REM" => {
                    recognised_any = true;
                    if let Some((key, value)) = parse_rem(rest) {
                        sheet.rem.push((key.to_ascii_uppercase(), value));
                    }
                    // A bare "REM" with nothing after it is a legitimate,
                    // meaningless comment separator some tools emit — ignore
                    // it silently rather than warning about it.
                }

                "PERFORMER" => {
                    recognised_any = true;
                    let value = parse_value(rest);
                    match current_track {
                        Some(idx) => sheet.tracks[idx].performer = Some(value),
                        None => sheet.performer = Some(value),
                    }
                }

                "TITLE" => {
                    recognised_any = true;
                    let value = parse_value(rest);
                    match current_track {
                        Some(idx) => sheet.tracks[idx].title = Some(value),
                        None => sheet.title = Some(value),
                    }
                }

                "SONGWRITER" => {
                    // Recognised (it is a real cue sheet command), but there
                    // is nowhere to put it: neither `CueSheet` nor
                    // `CueTrack` models a songwriter field, since nothing
                    // downstream currently needs it. Counting the line as
                    // recognised (without storing anything) keeps a sheet
                    // that uses SONGWRITER from being wrongly treated as
                    // "not a cue sheet at all".
                    recognised_any = true;
                }

                "CATALOG" => {
                    recognised_any = true;
                    sheet.catalog = Some(parse_value(rest));
                }

                "FILE" => {
                    recognised_any = true;
                    match parse_file_line(rest) {
                        Some((name, file_type)) => sheet.files.push(CueFile { name, file_type }),
                        None => sheet
                            .warnings
                            .push(format!("line {line_no}: FILE with no name — ignored")),
                    }
                }

                "TRACK" => {
                    recognised_any = true;
                    if sheet.files.is_empty() {
                        sheet
                            .warnings
                            .push(format!("line {line_no}: TRACK before any FILE — ignored"));
                    } else {
                        match parse_two_tokens(rest) {
                            Some((num_str, type_str)) => match num_str.parse::<u8>() {
                                Ok(number) => {
                                    if !(1..=99).contains(&number) {
                                        sheet.warnings.push(format!(
                                            "line {line_no}: TRACK number {number} is outside the usual 1-99 range — kept anyway"
                                        ));
                                    }
                                    // Bound to the most recently seen FILE,
                                    // per the standard cue sheet convention.
                                    let file_index = sheet.files.len() - 1;
                                    sheet.tracks.push(CueTrack {
                                        number,
                                        track_type: parse_track_type(type_str),
                                        file_index,
                                        performer: None,
                                        title: None,
                                        isrc: None,
                                        indexes: Vec::new(),
                                        pregap_frames: None,
                                        postgap_frames: None,
                                        flags: Vec::new(),
                                    });
                                    current_track = Some(sheet.tracks.len() - 1);
                                }
                                Err(_) => sheet.warnings.push(format!(
                                    "line {line_no}: TRACK number '{num_str}' is not a number — ignored"
                                )),
                            },
                            None => sheet
                                .warnings
                                .push(format!("line {line_no}: malformed TRACK line — ignored")),
                        }
                    }
                }

                "INDEX" => {
                    recognised_any = true;
                    match current_track {
                        None => sheet
                            .warnings
                            .push(format!("line {line_no}: INDEX before any TRACK — ignored")),
                        Some(track_idx) => match parse_two_tokens(rest) {
                            Some((num_str, time_str)) => {
                                match (num_str.parse::<u8>(), parse_frames(time_str)) {
                                    (Ok(number), Some(frames)) => {
                                        sheet.tracks[track_idx]
                                            .indexes
                                            .push(CueIndex { number, frames });
                                    }
                                    _ => sheet.warnings.push(format!(
                                        "line {line_no}: unparseable INDEX '{rest}' — ignored"
                                    )),
                                }
                            }
                            None => sheet
                                .warnings
                                .push(format!("line {line_no}: malformed INDEX line — ignored")),
                        },
                    }
                }

                "PREGAP" => {
                    recognised_any = true;
                    match current_track {
                        None => sheet
                            .warnings
                            .push(format!("line {line_no}: PREGAP before any TRACK — ignored")),
                        Some(idx) => match parse_frames(rest.trim()) {
                            Some(frames) => sheet.tracks[idx].pregap_frames = Some(frames),
                            None => sheet.warnings.push(format!(
                                "line {line_no}: unparseable PREGAP time '{rest}' — ignored"
                            )),
                        },
                    }
                }

                "POSTGAP" => {
                    recognised_any = true;
                    match current_track {
                        None => sheet.warnings.push(format!(
                            "line {line_no}: POSTGAP before any TRACK — ignored"
                        )),
                        Some(idx) => match parse_frames(rest.trim()) {
                            Some(frames) => sheet.tracks[idx].postgap_frames = Some(frames),
                            None => sheet.warnings.push(format!(
                                "line {line_no}: unparseable POSTGAP time '{rest}' — ignored"
                            )),
                        },
                    }
                }

                "FLAGS" => {
                    recognised_any = true;
                    match current_track {
                        None => sheet
                            .warnings
                            .push(format!("line {line_no}: FLAGS before any TRACK — ignored")),
                        Some(idx) => {
                            for flag in rest.split_whitespace() {
                                sheet.tracks[idx].flags.push(flag.to_string());
                            }
                        }
                    }
                }

                "ISRC" => {
                    recognised_any = true;
                    match current_track {
                        None => sheet
                            .warnings
                            .push(format!("line {line_no}: ISRC before any TRACK — ignored")),
                        Some(idx) => sheet.tracks[idx].isrc = Some(parse_value(rest)),
                    }
                }

                unknown => {
                    sheet.warnings.push(format!(
                        "line {line_no}: unknown command '{unknown}' — ignored"
                    ));
                }
            }
        }

        // A track with no INDEX 01 has no known start position — it's not
        // an error (the track is still real and still gets kept), but it
        // does mean nothing downstream can trust `index01_frames` for it.
        let missing_index01: Vec<u8> = sheet
            .tracks
            .iter()
            .filter(|track| !track.indexes.iter().any(|index| index.number == 1))
            .map(|track| track.number)
            .collect();
        for number in missing_index01 {
            sheet.warnings.push(format!(
                "track {number} has no INDEX 01 — its start position is unknown"
            ));
        }

        if recognised_any {
            Ok(sheet)
        } else {
            Err(MmError::Disc(
                "no recognised cue sheet commands found — this is not a cue sheet".to_string(),
            ))
        }
    }

    /// Read and parse a cue sheet file.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the file cannot be read, if it is larger than the
    /// 1 MiB cue sheet size limit (a genuine cue sheet is a short text file;
    /// anything that big is not one), or per [`Self::parse_str`]'s error
    /// condition.
    pub fn parse_file(path: &Path) -> MmResult<Self> {
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > MAX_CUE_BYTES {
            return Err(MmError::Disc(format!(
                "cue sheet {} is {} bytes, over the {MAX_CUE_BYTES}-byte limit for a track listing",
                path.display(),
                metadata.len()
            )));
        }
        let bytes = std::fs::read(path)?;
        let text = decode_cue_bytes(&bytes);
        Self::parse_str(&text)
    }

    /// Whether every `FILE` this sheet names is itself a disc image (rather
    /// than, say, an already-extracted `.wav`/`.flac`), and there is at
    /// least one. A sheet with zero FILE lines describes nothing, so it
    /// cannot describe a disc image either.
    #[must_use]
    pub fn describes_disc_image(&self) -> bool {
        !self.files.is_empty() && self.files.iter().all(|file| file.file_type.is_disc_image())
    }

    /// Whether this sheet has at least one audio track, and every track is
    /// audio (a plain Audio CD, as opposed to a data disc, an Enhanced CD,
    /// or a mixed-mode disc).
    #[must_use]
    pub fn is_all_audio(&self) -> bool {
        self.has_audio() && self.tracks.iter().all(|track| track.track_type.is_audio())
    }

    /// Whether this sheet has at least one audio track.
    #[must_use]
    pub fn has_audio(&self) -> bool {
        self.tracks.iter().any(|track| track.track_type.is_audio())
    }

    /// Whether this sheet has at least one non-audio (data) track.
    #[must_use]
    pub fn has_data(&self) -> bool {
        self.tracks.iter().any(|track| !track.track_type.is_audio())
    }

    /// Whether the track layout is the "Enhanced CD" shape: one or more
    /// audio tracks, followed by one or more data tracks, with no audio
    /// track after the first data track. This is the shape used by an
    /// Enhanced CD / CD Extra, where the bonus data session comes last so a
    /// plain audio CD player never sees it.
    #[must_use]
    pub fn data_track_is_last(&self) -> bool {
        if !self.has_audio() || !self.has_data() {
            return false;
        }
        match self
            .tracks
            .iter()
            .position(|track| !track.track_type.is_audio())
        {
            // The layout must actually start with audio (idx > 0) and, once
            // it switches to data, never switch back.
            Some(idx) => {
                idx > 0
                    && self.tracks[idx..]
                        .iter()
                        .all(|track| !track.track_type.is_audio())
            }
            None => false,
        }
    }

    /// Whether the track layout is the "mixed mode" shape: one or more data
    /// tracks, followed by one or more audio tracks, with no data track
    /// after the first audio track. This is the classic Mixed Mode CD shape
    /// (data first, so a computer sees it as track 1; audio after, for
    /// playback on a CD player that skips track 1).
    #[must_use]
    pub fn data_track_is_first(&self) -> bool {
        if !self.has_audio() || !self.has_data() {
            return false;
        }
        match self
            .tracks
            .iter()
            .position(|track| track.track_type.is_audio())
        {
            Some(idx) => {
                idx > 0
                    && self.tracks[idx..]
                        .iter()
                        .all(|track| track.track_type.is_audio())
            }
            None => false,
        }
    }

    /// Look up a `REM key value` by key, case-insensitively (values are
    /// stored with the key upper-cased, but callers shouldn't need to know
    /// that).
    #[must_use]
    pub fn rem_value(&self, key: &str) -> Option<&str> {
        self.rem
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    /// The INDEX 01 frame position of every track, in track order —
    /// `None` for a track that has no INDEX 01 (see `parse_str`'s handling
    /// of that case).
    ///
    /// This is deliberately its own method rather than something folded
    /// into track iteration elsewhere: issue #218's disc fingerprint is
    /// computed from exactly this list, so keeping it as a single reusable
    /// query means that stage doesn't have to re-derive it.
    #[must_use]
    pub fn index01_frames(&self) -> Vec<Option<u32>> {
        self.tracks
            .iter()
            .map(|track| {
                track
                    .indexes
                    .iter()
                    .find(|index| index.number == 1)
                    .map(|index| index.frames)
            })
            .collect()
    }
}

/// Decode raw cue sheet bytes to text.
///
/// Cue sheets have no declared encoding of their own. Ripping tools that
/// write one almost always do one of: plain UTF-8 (rare, but sometimes with
/// a byte-order mark), UTF-16 with a byte-order mark (some Windows tools),
/// or — by far the most common case in the wild — the machine's legacy
/// single-byte code page, which for a Western release is Latin-1
/// (ISO-8859-1). Latin-1 maps every byte 1:1 onto the Unicode code point of
/// the same number, so treating unmarked bytes as Latin-1 can never fail the
/// way a strict UTF-8 decode would the moment an accented performer name
/// appears — it would otherwise turn a French or German artist's name into
/// mojibake, or reject the whole sheet.
#[must_use]
pub fn decode_cue_bytes(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        // UTF-8 byte-order mark.
        return String::from_utf8_lossy(rest).into_owned();
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        // UTF-16 little-endian byte-order mark.
        return decode_utf16(rest, u16::from_le_bytes);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        // UTF-16 big-endian byte-order mark.
        return decode_utf16(rest, u16::from_be_bytes);
    }
    // No byte-order mark: treat every byte as its own Latin-1 code point.
    // (Bytes that happen to also be valid UTF-8 — the common ASCII-only
    // case — decode identically either way, since ASCII is a subset of
    // both encodings.)
    bytes.iter().map(|&byte| byte as char).collect()
}

/// Decode a UTF-16 byte stream (BOM already stripped) using the given
/// per-code-unit byte order, lossily substituting the replacement character
/// for anything invalid rather than failing. A trailing odd byte (a
/// truncated final code unit) is simply dropped instead of read out of
/// bounds.
fn decode_utf16(bytes: &[u8], to_unit: fn([u8; 2]) -> u16) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| to_unit([pair[0], pair[1]]))
        .collect();
    String::from_utf16_lossy(&units)
}

/// Split a trimmed, non-empty line into its command word and the raw
/// remainder (whitespace-trimmed at the start, but not yet unquoted).
fn split_command(line: &str) -> (&str, &str) {
    match line.split_once(char::is_whitespace) {
        Some((command, rest)) => (command, rest.trim_start()),
        None => (line, ""),
    }
}

/// Take the first two whitespace-separated tokens of `rest` — used by both
/// `TRACK nn TYPE` and `INDEX nn mm:ss:ff`, which share that two-token
/// shape. Returns `None` if fewer than two tokens are present.
fn parse_two_tokens(rest: &str) -> Option<(&str, &str)> {
    let mut tokens = rest.split_whitespace();
    let first = tokens.next()?;
    let second = tokens.next()?;
    Some((first, second))
}

/// Resolve a command's value: a `"double-quoted string"` (quotes stripped,
/// content taken verbatim — cue sheets have no escape sequence for an
/// embedded quote) or, if there is no leading quote, the rest of the line
/// as-is.
fn parse_value(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(inner) = trimmed.strip_prefix('"') {
        return match inner.find('"') {
            Some(end) => inner[..end].to_string(),
            // Unterminated quote — a malformed sheet, but there is still a
            // reasonable value to extract: everything after the opening
            // quote.
            None => inner.to_string(),
        };
    }
    trimmed.to_string()
}

/// Parse a `REM key value` remainder into `(key, value)`. Returns `None`
/// for a bare `REM` with nothing after it, which is a meaningless-but-legal
/// comment separator, not a key/value pair.
fn parse_rem(rest: &str) -> Option<(String, String)> {
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let (key, value) = split_command(rest);
    Some((key.to_string(), parse_value(value)))
}

/// Parse a `FILE` line's remainder into `(name, type)`.
///
/// The TYPE keyword is always the *last* whitespace-separated token on the
/// line; everything before it is the name, which may be quoted (and, if
/// quoted, may itself contain spaces — this is why the split happens from
/// the right rather than the left).
fn parse_file_line(rest: &str) -> Option<(String, FileType)> {
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let (name_part, type_part) = match rest.rsplit_once(char::is_whitespace) {
        Some((name, ty)) => (name.trim(), ty.trim()),
        // No whitespace at all: a bare token with no TYPE keyword.
        None => (rest, ""),
    };
    let name = parse_value(name_part);
    if name.is_empty() {
        return None;
    }
    Some((name, parse_file_type(type_part)))
}

/// Map a `FILE` TYPE keyword to a [`FileType`], case-insensitively.
fn parse_file_type(s: &str) -> FileType {
    match s.trim().to_ascii_uppercase().as_str() {
        "BINARY" => FileType::Binary,
        "MOTOROLA" => FileType::Motorola,
        "WAVE" | "WAVEFORM" => FileType::Wave,
        "AIFF" => FileType::Aiff,
        "MP3" => FileType::Mp3,
        other => FileType::Other(other.to_string()),
    }
}

/// Map a `TRACK` TYPE keyword to a [`TrackType`], case-insensitively.
fn parse_track_type(s: &str) -> TrackType {
    match s.trim().to_ascii_uppercase().as_str() {
        "AUDIO" => TrackType::Audio,
        "CDG" => TrackType::Cdg,
        "MODE1/2048" => TrackType::Mode1_2048,
        "MODE1/2352" => TrackType::Mode1_2352,
        "MODE2/2336" => TrackType::Mode2_2336,
        "MODE2/2352" => TrackType::Mode2_2352,
        "CDI/2336" => TrackType::Cdi2336,
        "CDI/2352" => TrackType::Cdi2352,
        other => TrackType::Other(other.to_string()),
    }
}

/// Parse an `mm:ss:ff` timestamp into a frame count:
/// `(mm * 60 + ss) * 75 + ff` (75 frames per second, the CD-DA sector
/// addressing rate). Returns `None` for anything that doesn't parse —
/// deliberately using checked arithmetic throughout (via widening to `u64`
/// first) rather than plain `*`/`+`, since this function is fed directly
/// from file content and must never panic on an overflowing value.
fn parse_frames(s: &str) -> Option<u32> {
    let mut parts = s.trim().splitn(3, ':');
    let minutes: u64 = parts.next()?.trim().parse().ok()?;
    let seconds: u64 = parts.next()?.trim().parse().ok()?;
    let frames_field: u64 = parts.next()?.trim().parse().ok()?;

    let total = minutes
        .checked_mul(60)?
        .checked_add(seconds)?
        .checked_mul(75)?
        .checked_add(frames_field)?;
    u32::try_from(total).ok()
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::*;

    /// A representative EAC-style sheet: REM tags, disc-level PERFORMER and
    /// TITLE, one BINARY file, three audio tracks each with their own INDEX
    /// 01. This is the shape the vast majority of real cue sheets take.
    const EAC_STYLE_SHEET: &str = r#"REM GENRE Alternative
REM DATE 1999
REM DISCID 00123456
PERFORMER "Test Artist"
TITLE "Test Album"
FILE "Test Album.bin" BINARY
  TRACK 01 AUDIO
    TITLE "Track One"
    PERFORMER "Test Artist"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Track Two"
    PERFORMER "Test Artist"
    INDEX 00 03:50:00
    INDEX 01 03:52:00
  TRACK 03 AUDIO
    TITLE "Track Three"
    PERFORMER "Test Artist"
    INDEX 00 07:12:32
    INDEX 01 07:14:32
"#;

    #[test]
    fn eac_style_sheet_parses_performer_title_and_frames() {
        let sheet = CueSheet::parse_str(EAC_STYLE_SHEET).expect("a well-formed sheet must parse");

        assert_eq!(sheet.performer.as_deref(), Some("Test Artist"));
        assert_eq!(sheet.title.as_deref(), Some("Test Album"));
        assert_eq!(sheet.rem_value("DATE"), Some("1999"));
        assert_eq!(sheet.rem_value("date"), Some("1999")); // case-insensitive lookup
        assert_eq!(sheet.rem_value("GENRE"), Some("Alternative"));
        assert_eq!(sheet.rem_value("DISCID"), Some("00123456"));

        assert_eq!(sheet.tracks.len(), 3);
        assert!(sheet.is_all_audio());
        assert!(sheet.describes_disc_image());

        // Hand-computed via (mm*60+ss)*75+ff:
        //   00:00:00 -> 0
        //   03:52:00 -> (3*60+52)*75       = 232*75       = 17400
        //   07:14:32 -> (7*60+14)*75 + 32  = 434*75 + 32  = 32582
        assert_eq!(
            sheet.index01_frames(),
            vec![Some(0), Some(17_400), Some(32_582)]
        );
        assert!(
            sheet.warnings.is_empty(),
            "well-formed sheet should have no warnings, got {:?}",
            sheet.warnings
        );
    }

    #[test]
    fn unquoted_values_are_parsed() {
        let text = "PERFORMER Test Artist\nTITLE Test Album\nFILE test.bin BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n";
        let sheet = CueSheet::parse_str(text).expect("unquoted sheet must still parse");

        assert_eq!(sheet.performer.as_deref(), Some("Test Artist"));
        assert_eq!(sheet.title.as_deref(), Some("Test Album"));
        assert_eq!(sheet.files.len(), 1);
        assert_eq!(sheet.files[0].name, "test.bin");
        assert_eq!(sheet.files[0].file_type, FileType::Binary);
    }

    #[test]
    fn crlf_line_endings_are_handled() {
        let text = EAC_STYLE_SHEET.replace('\n', "\r\n");
        let sheet = CueSheet::parse_str(&text).expect("CRLF sheet must parse the same as LF");

        assert_eq!(sheet.performer.as_deref(), Some("Test Artist"));
        assert_eq!(sheet.tracks.len(), 3);
        assert_eq!(
            sheet.index01_frames(),
            vec![Some(0), Some(17_400), Some(32_582)]
        );
    }

    #[test]
    fn utf8_bom_is_stripped() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"PERFORMER \"Bom Artist\"\nFILE \"x.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n");

        let text = decode_cue_bytes(&bytes);
        assert!(
            !text.starts_with('\u{FEFF}'),
            "BOM character must not remain in the decoded text"
        );

        let sheet = CueSheet::parse_str(&text).expect("BOM-prefixed sheet must parse");
        assert_eq!(sheet.performer.as_deref(), Some("Bom Artist"));
    }

    #[test]
    fn latin1_bytes_decode_as_expected() {
        // "PERFORMER Caf\xE9" — 0xE9 is Latin-1 for 'é', but is not valid
        // standalone UTF-8, so a naive from_utf8 would either fail or
        // require lossy substitution (mangling the character). No BOM is
        // present, so this exercises the Latin-1 fallback path directly.
        let mut bytes = b"PERFORMER Caf".to_vec();
        bytes.push(0xE9);

        let text = decode_cue_bytes(&bytes);
        assert_eq!(text, "PERFORMER Café");

        let sheet = CueSheet::parse_str(&text).expect("Latin-1 sheet must parse");
        assert_eq!(sheet.performer.as_deref(), Some("Café"));
    }

    #[test]
    fn utf16_le_with_bom_decodes() {
        let source = "PERFORMER \"UTF16 Artist\"\nFILE \"x.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n";
        let mut bytes = vec![0xFF, 0xFE]; // UTF-16 LE BOM
        for unit in source.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }

        let text = decode_cue_bytes(&bytes);
        let sheet = CueSheet::parse_str(&text).expect("UTF-16 LE sheet must parse");
        assert_eq!(sheet.performer.as_deref(), Some("UTF16 Artist"));
    }

    #[test]
    fn flac_cue_is_not_a_disc_image() {
        let text = "PERFORMER \"Some Band\"\nFILE \"album.flac\" WAVE\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n";
        let sheet = CueSheet::parse_str(text).expect("cue sheet for a FLAC rip must still parse");

        assert_eq!(sheet.files[0].file_type, FileType::Wave);
        assert!(!sheet.describes_disc_image());
    }

    #[test]
    fn enhanced_cd_shape_detected() {
        let text = "FILE \"disc.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    INDEX 01 03:00:00\n  TRACK 03 MODE1/2352\n    INDEX 01 06:00:00\n";
        let sheet = CueSheet::parse_str(text).expect("enhanced CD sheet must parse");

        assert!(sheet.data_track_is_last());
        assert!(!sheet.data_track_is_first());
        assert!(!sheet.is_all_audio());
        assert!(sheet.has_audio());
        assert!(sheet.has_data());
    }

    #[test]
    fn mixed_mode_shape_detected() {
        let text = "FILE \"disc.bin\" BINARY\n  TRACK 01 MODE1/2352\n    INDEX 01 00:00:00\n  TRACK 02 AUDIO\n    INDEX 01 03:00:00\n  TRACK 03 AUDIO\n    INDEX 01 06:00:00\n";
        let sheet = CueSheet::parse_str(text).expect("mixed mode sheet must parse");

        assert!(sheet.data_track_is_first());
        assert!(!sheet.data_track_is_last());
    }

    #[test]
    fn mode2_2336_sector_size() {
        assert_eq!(TrackType::Mode2_2336.sector_size(), Some(2336));

        let text = "FILE \"disc.bin\" BINARY\n  TRACK 01 MODE2/2336\n    INDEX 01 00:00:00\n";
        let sheet = CueSheet::parse_str(text).expect("MODE2/2336 sheet must parse");
        assert_eq!(sheet.tracks[0].track_type, TrackType::Mode2_2336);
    }

    #[test]
    fn missing_index01_warns_but_keeps_track() {
        let text = "FILE \"x.bin\" BINARY\n  TRACK 01 AUDIO\n    INDEX 00 00:00:00\n";
        let sheet = CueSheet::parse_str(text).expect("sheet missing INDEX 01 must still parse");

        assert_eq!(
            sheet.tracks.len(),
            1,
            "the track must be kept despite the missing INDEX 01"
        );
        assert_eq!(sheet.index01_frames(), vec![None]);
        assert!(
            sheet.warnings.iter().any(|w| w.contains("INDEX 01")),
            "expected a warning about the missing INDEX 01, got {:?}",
            sheet.warnings
        );
    }

    #[test]
    fn index_before_track_is_dropped() {
        let text =
            "FILE \"x.bin\" BINARY\nINDEX 01 00:00:00\n  TRACK 01 AUDIO\n    INDEX 01 00:00:05\n";
        let sheet =
            CueSheet::parse_str(text).expect("sheet must still parse despite the stray INDEX");

        assert_eq!(sheet.tracks.len(), 1);
        assert_eq!(
            sheet.tracks[0].indexes.len(),
            1,
            "only the INDEX inside the TRACK should be kept"
        );
        assert!(
            sheet
                .warnings
                .iter()
                .any(|w| w.contains("INDEX before any TRACK"))
        );
    }

    #[test]
    fn garbage_text_returns_err() {
        let text =
            "this is not a cue sheet at all\njust some ordinary prose\nwith no commands in it\n";
        let result = CueSheet::parse_str(text);
        assert!(
            result.is_err(),
            "text with no recognised commands must be rejected"
        );
    }

    #[test]
    fn random_bytes_never_panic() {
        // A tiny deterministic linear-congruential generator — no new
        // dependency needed just to fuzz-check "does this panic".
        let mut state: u64 = 0x1234_5678_9abc_def0;
        let next_byte = |state: &mut u64| -> u8 {
            *state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (*state >> 56) as u8
        };

        for _ in 0..50 {
            let bytes: Vec<u8> = (0..500).map(|_| next_byte(&mut state)).collect();
            let text = decode_cue_bytes(&bytes);
            // The only assertion that matters here is "this did not panic";
            // the outcome (Ok or Err) is unconstrained for random input.
            let _ = CueSheet::parse_str(&text);
        }
    }

    #[test]
    fn oversize_file_returns_err() {
        let mut file = tempfile::NamedTempFile::new().expect("failed to create temp file");
        let oversize_chunk = vec![b'A'; (MAX_CUE_BYTES + 1) as usize];
        file.write_all(&oversize_chunk)
            .expect("failed to write temp file");
        file.flush().expect("failed to flush temp file");

        let result = CueSheet::parse_file(file.path());
        assert!(
            result.is_err(),
            "a file over the size limit must be refused"
        );
    }

    #[test]
    fn parse_file_reads_real_temp_file() {
        let mut file = tempfile::NamedTempFile::new().expect("failed to create temp file");
        file.write_all(EAC_STYLE_SHEET.as_bytes())
            .expect("failed to write temp file");
        file.flush().expect("failed to flush temp file");

        let sheet = CueSheet::parse_file(file.path()).expect("a real, well-formed file must parse");
        assert_eq!(sheet.performer.as_deref(), Some("Test Artist"));
        assert_eq!(sheet.tracks.len(), 3);
    }
}
