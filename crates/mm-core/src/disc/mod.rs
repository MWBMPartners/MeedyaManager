// (C) 2025-2026 MWBM Partners Ltd
//
// Disc image handling.
//
// A disc image (see `classify::MediaGroup::Disc`) is a bit-perfect copy of an
// optical disc, but the image file itself is usually an anonymous blob — an
// .iso or .bin has no title, artist or track list of its own. This module
// exists to read the *sidecar* information that gives a disc a name, rather
// than the image bytes themselves (a later stage reads the image bytes, for
// a fingerprint — see issue #218).
//
// The `cue` submodule parses cue sheets. A cue sheet is the standard sidecar
// for a raw `.bin`/`.iso` image: plain text naming which image file holds the
// audio, where each track starts, and usually the performer and album title,
// so a disc can be named from its own contents instead of guessed at from the
// folder it sits in.
//
// The rest of this file is disc *folder* detection — deciding which files on
// disk belong to one rip, so that nothing ever pulls them apart.
//
// Why this half of the module exists (issue #219, a live data-loss bug):
//
//   A raw CD rip is not one file. It is a `.cue` text sheet that names a
//   `.bin` (or `.iso`) image, usually beside a ripping log, a checksum and
//   some artwork. The cue sheet refers to the image *by bare file name*, so
//   the pair only works while the two sit in the same directory. Move one
//   and the rip is destroyed — the cue still says `FILE "Album.bin"` and
//   that file is no longer there.
//
//   Before this module existed, `meedya scan --execute` had no idea the two
//   files were related. With an ordinary template such as
//   `<Extension>/<Filename>` it cheerfully moved `Album.cue` into `cue/` and
//   `Album.bin` into `bin/`, printed "OK" for both, and reported success.
//   Silent, irreversible, and with no unusual settings involved.
//
//   The fix is `partition_for_scan`: before anything is renamed, disc image
//   files are lifted out of the per-file rename plan entirely. They are
//   reported as *folders* instead. Whole-folder moving is a later stage
//   (issue #217); this stage's whole job is detection plus exclusion, and
//   the exclusion is what makes the data loss stop.
//
// Nothing here opens a disc image and reads its sectors. Everything is
// decided from directory listings, file names, cue sheet text and (for one
// narrow case) the first twelve bytes of a `.bin`. Looking inside an image
// to tell an Audio CD from a DVD film is issue #217's later stage.

/// Cue sheet parsing — track layout, performer/title tags, and the index
/// positions a later stage turns into a disc fingerprint.
pub mod cue;

use crate::classify::{MediaGroup, classify_by_path};
use crate::disc::cue::CueSheet;
use crate::error::MmResult;
use crate::metadata::TagMap;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// The first twelve bytes of every raw 2352-byte CD sector: one zero byte,
/// ten `0xFF` bytes, one zero byte.
///
/// This is the CD-ROM *sync pattern*, part of the physical sector format, so
/// any raw sector-level dump of a CD starts with it. We use it as the single
/// piece of evidence that a lone `.bin` with no cue sheet beside it really is
/// a disc image, rather than a firmware blob, a game data file, a disk-image
/// fragment or any of the thousand other things people call `.bin`. Getting
/// this wrong in the permissive direction would mean quietly refusing to
/// rename somebody's ordinary files, so the test is deliberately strict.
const CD_SYNC_PATTERN: [u8; 12] = [
    0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00,
];

/// A trailing year in a folder name — ` (1997)` or ` [1997]`.
///
/// Rippers and taggers overwhelmingly write the release year this way, so
/// lifting it out of the folder name gives us a `year` for free and stops the
/// brackets ending up inside the album title.
static TRAILING_YEAR_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\s*[\(\[](\d{4})[\)\]]\s*$").expect("trailing-year regex must compile")
});

/// The separator a folder name uses between artist and album, by long-
/// standing convention in music libraries: `Artist - Album`.
const FOLDER_NAME_SEPARATOR: &str = " - ";

// ─────────────────────────────────────────────────────────────────────
// Value types
// ─────────────────────────────────────────────────────────────────────

/// What kind of disc an image holds.
///
/// At this stage only the CD family is ever decided, and only from a cue
/// sheet's track layout — a cue sheet is the one sidecar that states the
/// track types outright. The DVD/Blu-ray variants exist because telling a
/// film disc from a music disc is the whole point of the exercise, but they
/// can only be filled in by reading the image's own filesystem (a `VIDEO_TS`
/// or `BDMV` directory inside it), which is issue #217's later stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscKind {
    /// Every track is CD audio — a plain Audio CD.
    AudioCd,
    /// Audio tracks first, then a data track at the end. This is the Enhanced
    /// CD / CD Extra shape: the bonus data session comes last so an ordinary
    /// CD player never stumbles into it.
    EnhancedCd,
    /// A data track first, then audio tracks. The classic Mixed Mode CD: a
    /// computer sees the data as track 1, and a CD player is expected to skip
    /// it.
    MixedModeCd,
    /// A DVD holding a film (a `VIDEO_TS` structure). Not yet detectable —
    /// needs the image's contents read.
    DvdVideo,
    /// A DVD-Audio disc (an `AUDIO_TS` structure). Not yet detectable.
    DvdAudio,
    /// An HD DVD. Not yet detectable.
    HdDvd,
    /// A Blu-ray disc (a `BDMV` structure). Not yet detectable.
    Bluray,
    /// A stereoscopic ("3D") Blu-ray. Not yet detectable.
    Bluray3D,
    /// Every track is data — a plain data disc, no playable audio.
    DataDisc,
    /// Nothing available told us what this disc holds. This is the honest
    /// answer for an image with no cue sheet, and it is deliberately not
    /// guessed at: a wrong kind would send the folder down the wrong naming
    /// rules later.
    Unknown,
}

impl std::fmt::Display for DiscKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Written out in the form a person would say them, because these
        // strings go straight into the CLI's "Disc Folders" table.
        let text = match self {
            Self::AudioCd => "Audio CD",
            Self::EnhancedCd => "Enhanced CD",
            Self::MixedModeCd => "Mixed Mode CD",
            Self::DvdVideo => "DVD-Video",
            Self::DvdAudio => "DVD-Audio",
            Self::HdDvd => "HD DVD",
            Self::Bluray => "Blu-ray",
            Self::Bluray3D => "Blu-ray 3D",
            Self::DataDisc => "Data Disc",
            Self::Unknown => "Unknown",
        };
        f.write_str(text)
    }
}

/// The on-disk shape of one disc image — how many files it is spread across
/// and what wrote them.
///
/// This is about the *container*, not the content: `BinCue` says "a raw
/// sector dump plus a text sheet describing it", and says nothing about
/// whether the disc holds music or software.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscImageFormat {
    /// A single ISO 9660 / UDF image file (`.iso`).
    Iso,
    /// A raw sector dump plus its cue sheet (`.bin` + `.cue`), or a lone raw
    /// dump we recognised by its sector sync pattern.
    BinCue,
    /// A Nero image (`.nrg`).
    Nrg,
    /// An Alcohol 120% image: the `.mds` descriptor plus the `.mdf` data
    /// file it belongs to.
    MdsMdf,
    /// A Daemon Tools image (`.mdx`).
    Mdx,
    /// An Apple CD/DVD Master image (`.cdr`) — an ISO image under a different
    /// extension.
    Cdr,
}

impl std::fmt::Display for DiscImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::Iso => "ISO",
            Self::BinCue => "BIN/CUE",
            Self::Nrg => "NRG",
            Self::MdsMdf => "MDS/MDF",
            Self::Mdx => "MDX",
            Self::Cdr => "CDR",
        };
        f.write_str(text)
    }
}

/// Where a disc's artist/title/year came from.
///
/// This is recorded rather than thrown away because the owner's rule is that
/// a disc with no confident name must not be renamed at all. A caller cannot
/// honour that rule unless it can see *how* the name was arrived at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NameSource {
    /// Read out of a cue sheet's own `PERFORMER` and `TITLE` lines — the
    /// most trustworthy source, because the ripping tool wrote it from the
    /// disc itself.
    CueSheet,
    /// Parsed out of the containing folder's name. Better than nothing, but
    /// it is only somebody's filing convention, so it is worth less than a
    /// cue sheet.
    FolderName,
    /// Neither source produced a name. The disc stays unnamed.
    None,
}

impl std::fmt::Display for NameSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::CueSheet => "cue sheet",
            Self::FolderName => "folder name",
            Self::None => "none",
        };
        f.write_str(text)
    }
}

/// One disc image and every file that makes it up.
///
/// A "set" exists so that the several files of a single image are never
/// considered separately. A BIN/CUE pair is two files but one disc; an
/// MDS/MDF pair likewise. `members` is the complete list, and every path in
/// it is protected from individual renaming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscImageSet {
    /// The file that best identifies this image — the cue sheet for a
    /// BIN/CUE set (because it is the one carrying the tags), the `.mds` for
    /// an Alcohol pair, and the image file itself for the single-file
    /// formats.
    pub primary: PathBuf,
    /// Every file belonging to this image, `primary` included, sorted by
    /// path so the list is stable between runs.
    pub members: Vec<PathBuf>,
    /// The container shape.
    pub format: DiscImageFormat,
    /// The parsed cue sheet, when this set has one. `None` for every format
    /// that carries its own metadata internally (or carries none at all).
    pub cue: Option<CueSheet>,
}

/// What we managed to work out about the disc a folder holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscInfo {
    /// What kind of disc this is, as far as anything could tell.
    pub kind: DiscKind,
    /// The container shape of the folder's first image.
    pub image_format: DiscImageFormat,
    /// How many separate disc images the folder holds. More than one means a
    /// multi-disc release (a two-CD album, say) filed in a single folder.
    pub image_count: usize,
    /// How many tracks the disc has, when a cue sheet said so.
    pub track_count: Option<u32>,
    /// The disc's own volume label.
    ///
    /// Always `None` at this stage. A volume label lives inside the image's
    /// filesystem header, and nothing here opens an image — that arrives with
    /// issue #217's "look inside the image" stage. The field exists now so
    /// that adding it later does not change this struct's shape for callers.
    pub label: Option<String>,
    /// The performing artist, if a name was found.
    pub performer: Option<String>,
    /// The album/disc title, if a name was found.
    pub title: Option<String>,
    /// The release year, if one was found.
    pub year: Option<String>,
    /// Which of the two naming routes produced the three fields above.
    pub name_source: NameSource,
}

/// A directory that holds one or more disc images and nothing that would
/// make it an ordinary media folder.
///
/// The unit of interest is the *folder*, not the image, because the owner's
/// rule is that the whole containing folder moves as one sealed unit: a rip
/// carries a log, a checksum and artwork that only make sense beside the
/// image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscFolder {
    /// The directory itself.
    pub dir: PathBuf,
    /// The image sets found directly inside it (not in subdirectories).
    pub images: Vec<DiscImageSet>,
    /// What we worked out about the disc.
    pub info: DiscInfo,
    /// How many files the folder holds in total, counting everything in
    /// every subdirectory and including dotfiles.
    ///
    /// The whole subtree is counted, not just the image files, because the
    /// folder will eventually move as a unit — so the figure a user is shown
    /// should be the number of files that move.
    pub file_count: usize,
    /// The total size in bytes of those same files.
    pub total_bytes: u64,
}

// ─────────────────────────────────────────────────────────────────────
// Extension test
// ─────────────────────────────────────────────────────────────────────

/// Whether an extension (given without its leading dot, in any case) is one
/// of the disc image extensions this project recognises.
///
/// The list is the eight `MediaFormat` variants in the `Disc` classification
/// group — `iso`, `nrg`, `mds`, `mdx`, `cdr`, `bin`, `cue` — together with
/// `mdf`, the data half of an Alcohol 120% image.
///
/// Three of those (`bin`, `cue`, `mdf`) deliberately do **not** classify as
/// disc images from their extension alone, because each is ambiguous on its
/// own: `.bin` is used for any binary blob, `.cue` beside FLAC files is a
/// track index rather than a disc image, and `.mdf` collides with SQL
/// Server's data files. This function is the wider, "might be part of a disc
/// image" test used while scanning a directory, where sibling files are
/// available to settle the ambiguity. It is not a classification.
#[must_use]
pub fn is_disc_image_extension(ext: &str) -> bool {
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "iso" | "nrg" | "mds" | "mdf" | "mdx" | "cdr" | "bin" | "cue"
    )
}

/// The lower-cased extension of a path, or an empty string when it has none.
fn lower_extension(path: &Path) -> String {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

/// The lower-cased file name of a path, or an empty string when it has none.
fn lower_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

/// The lower-cased file stem (name without extension) of a path.
fn lower_file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

// ─────────────────────────────────────────────────────────────────────
// Finding image sets in one directory
// ─────────────────────────────────────────────────────────────────────

/// Whether a `.bin` file begins with the raw CD sector sync pattern.
///
/// Reads at most the first sixteen bytes. Any I/O failure answers "no": a
/// file we cannot read is not evidence of anything, and treating it as a
/// disc image would take it out of the rename plan on no evidence at all.
fn bin_has_cd_sync_pattern(path: &Path) -> bool {
    use std::io::Read;

    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = [0u8; 16];
    // `read` (not `read_exact`) so a file shorter than sixteen bytes gives a
    // short read rather than an error — it simply will not match.
    let Ok(read) = file.read(&mut head) else {
        return false;
    };
    read >= CD_SYNC_PATTERN.len() && head[..CD_SYNC_PATTERN.len()] == CD_SYNC_PATTERN
}

/// Pick the container shape a cue sheet's `FILE` lines describe.
///
/// A cue sheet that names an `.iso` is an ISO set; anything else that is a
/// raw sector dump is the BIN/CUE shape. This only ever runs on sheets that
/// already passed `describes_disc_image`, so the fallback is the right
/// answer rather than a guess.
fn format_for_cue(named: &[PathBuf]) -> DiscImageFormat {
    match named.first().map(|p| lower_extension(p)).as_deref() {
        Some("iso") => DiscImageFormat::Iso,
        Some("nrg") => DiscImageFormat::Nrg,
        Some("cdr") => DiscImageFormat::Cdr,
        _ => DiscImageFormat::BinCue,
    }
}

/// Find every disc image set directly inside `dir`.
///
/// The directory is read **once**, without recursing, and the entries are
/// paired up by these rules, in this order:
///
/// 1. Every `.cue` sheet whose `FILE` lines all describe raw disc images and
///    all resolve, case-insensitively, to files sitting beside it becomes one
///    set: the cue is the primary, and the members are the cue plus every
///    file it names. A cue that names a file which is not there is *not* a
///    set — it is a broken sidecar, and pretending otherwise would put a
///    non-existent path into the protected list.
/// 2. Every `.mds` descriptor with a same-stem `.mdf` beside it becomes one
///    set. An `.mdf` on its own is left alone, because `.mdf` is far more
///    commonly a SQL Server database file than an Alcohol 120% image.
/// 3. Every `.iso`, `.nrg`, `.mdx` and `.cdr` becomes a set of its own —
///    these extensions mean exactly one thing.
/// 4. A `.bin` that no cue sheet named becomes a set **only** if its first
///    twelve bytes are the raw CD sector sync pattern. A lone `.bin` without
///    that pattern is left completely alone, so we never claim somebody's
///    firmware blob is a disc image.
///
/// Entries are processed in sorted order, so the returned list is stable from
/// run to run — which matters because the first set decides the folder's
/// reported format and, often, its name.
///
/// # Errors
///
/// Returns `Err` if the directory cannot be read at all. Individual entries
/// that cannot be inspected are skipped rather than failing the whole scan:
/// one unreadable file must never stop a directory being understood.
pub fn find_image_sets(dir: &Path) -> MmResult<Vec<DiscImageSet>> {
    // Sorted by lower-cased file name, which gives both a stable order and
    // the case-insensitive lookup table the cue-resolution step needs.
    let mut by_name: BTreeMap<String, PathBuf> = BTreeMap::new();
    for entry in std::fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        // `file_type` avoids a second stat call, and follows the same
        // "skip what we cannot inspect" rule as everything else here.
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let name = lower_file_name(&path);
        if !name.is_empty() {
            by_name.insert(name, path);
        }
    }

    let mut sets: Vec<DiscImageSet> = Vec::new();
    // Lower-cased names already accounted for by an earlier rule, so a file
    // a cue sheet names cannot also become a set in its own right.
    let mut claimed: HashSet<String> = HashSet::new();

    // ── Rule 1: cue sheets ──────────────────────────────────────────
    for (name, path) in &by_name {
        if !name.ends_with(".cue") {
            continue;
        }
        // A sheet we cannot read or cannot parse is simply not a set. It is
        // still left out of the rename plan by `partition_for_scan`'s second
        // rule, so no cue sheet is ever renamed on its own regardless.
        let Ok(sheet) = cue::CueSheet::parse_file(path) else {
            continue;
        };
        if !sheet.describes_disc_image() {
            continue;
        }

        // Resolve every FILE line against this directory, case-insensitively
        // — cue sheets written on Windows routinely disagree with the actual
        // file name's capitalisation.
        let mut named: Vec<PathBuf> = Vec::with_capacity(sheet.files.len());
        let mut all_resolved = true;
        for file in &sheet.files {
            // A cue sheet may write a path with backslashes; take only the
            // final component, since the sheet is only ever allowed to refer
            // to a sibling.
            let bare = file
                .name
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(&file.name)
                .to_ascii_lowercase();
            let Some(resolved) = by_name.get(&bare) else {
                all_resolved = false;
                break;
            };
            named.push(resolved.clone());
        }
        if !all_resolved || named.is_empty() {
            continue;
        }

        let format = format_for_cue(&named);
        let mut members = vec![path.clone()];
        members.extend(named.iter().cloned());
        members.sort();
        members.dedup();

        claimed.insert(name.clone());
        for member in &members {
            claimed.insert(lower_file_name(member));
        }

        sets.push(DiscImageSet {
            primary: path.clone(),
            members,
            format,
            cue: Some(sheet),
        });
    }

    // ── Rule 2: MDS descriptors with their MDF data file ────────────
    for (name, path) in &by_name {
        if !name.ends_with(".mds") || claimed.contains(name) {
            continue;
        }
        let mdf_name = format!("{}.mdf", lower_file_stem(path));
        let Some(mdf) = by_name.get(&mdf_name) else {
            continue;
        };
        let mut members = vec![path.clone(), mdf.clone()];
        members.sort();
        claimed.insert(name.clone());
        claimed.insert(mdf_name);
        sets.push(DiscImageSet {
            primary: path.clone(),
            members,
            format: DiscImageFormat::MdsMdf,
            cue: None,
        });
    }

    // ── Rule 3: the unambiguous single-file formats ─────────────────
    for (name, path) in &by_name {
        if claimed.contains(name) {
            continue;
        }
        let format = match lower_extension(path).as_str() {
            "iso" => DiscImageFormat::Iso,
            "nrg" => DiscImageFormat::Nrg,
            "mdx" => DiscImageFormat::Mdx,
            "cdr" => DiscImageFormat::Cdr,
            _ => continue,
        };
        claimed.insert(name.clone());
        sets.push(DiscImageSet {
            primary: path.clone(),
            members: vec![path.clone()],
            format,
            cue: None,
        });
    }

    // ── Rule 4: a lone .bin, only on hard evidence ──────────────────
    for (name, path) in &by_name {
        if claimed.contains(name) || !name.ends_with(".bin") {
            continue;
        }
        if !bin_has_cd_sync_pattern(path) {
            continue;
        }
        claimed.insert(name.clone());
        sets.push(DiscImageSet {
            primary: path.clone(),
            members: vec![path.clone()],
            format: DiscImageFormat::BinCue,
            cue: None,
        });
    }

    // One final sort so the caller's "first set" is a stable, predictable
    // choice regardless of which rule produced it.
    sets.sort_by(|a, b| a.primary.cmp(&b.primary));
    Ok(sets)
}

// ─────────────────────────────────────────────────────────────────────
// Kind and name
// ─────────────────────────────────────────────────────────────────────

/// Work out what kind of disc this is from the track layout of the first cue
/// sheet available.
///
/// Only the CD family can be decided this way, because only a cue sheet
/// states track types. Everything else answers `Unknown`, which is the
/// honest result — a guess here would send the folder down the wrong naming
/// rules later on.
fn resolve_kind_from_layout(sets: &[DiscImageSet]) -> DiscKind {
    let Some(sheet) = sets.iter().find_map(|set| set.cue.as_ref()) else {
        return DiscKind::Unknown;
    };
    if sheet.is_all_audio() {
        DiscKind::AudioCd
    } else if sheet.data_track_is_last() {
        DiscKind::EnhancedCd
    } else if sheet.data_track_is_first() {
        DiscKind::MixedModeCd
    } else if sheet.has_data() && !sheet.has_audio() {
        DiscKind::DataDisc
    } else {
        // Audio and data interleaved in some other order — a real disc does
        // not look like this, so we decline to name a shape for it.
        DiscKind::Unknown
    }
}

/// A trimmed copy of a string, or `None` when it is blank.
///
/// Cue sheets in the wild frequently carry `PERFORMER ""` or a line of
/// spaces. Treating that as a name would produce folders called ` - Album`.
fn non_blank(value: Option<&String>) -> Option<String> {
    value
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
}

/// The performer, title, year and where they came from.
///
/// Two routes, in strict order of trustworthiness:
///
/// 1. The first image set whose cue sheet has **both** a non-blank
///    `PERFORMER` and a non-blank `TITLE`. Both are required: half a name is
///    not a name, and accepting one would produce a folder named after an
///    artist with no album, or the reverse. `REM DATE` supplies the year when
///    the sheet has one.
/// 2. Failing that, the folder's own name split on the first ` - `, which is
///    the near-universal `Artist - Album` convention. A trailing ` (1997)` or
///    ` [1997]` is lifted out into the year and stripped from the title.
///
/// If neither works, nothing is returned and `NameSource::None` is recorded,
/// which is the caller's signal that this disc must not be renamed.
fn resolve_name(
    dir: &Path,
    sets: &[DiscImageSet],
) -> (Option<String>, Option<String>, Option<String>, NameSource) {
    // ── Route 1: the cue sheet's own tags ───────────────────────────
    for set in sets {
        let Some(sheet) = set.cue.as_ref() else {
            continue;
        };
        let performer = non_blank(sheet.performer.as_ref());
        let title = non_blank(sheet.title.as_ref());
        if let (Some(performer), Some(title)) = (performer, title) {
            // `REM DATE 1997` is what every mainstream ripper writes; some
            // write a full date, so keep only a bare four-digit year.
            let year = sheet
                .rem_value("DATE")
                .map(str::trim)
                .filter(|value| value.len() == 4 && value.chars().all(|c| c.is_ascii_digit()))
                .map(ToString::to_string);
            return (Some(performer), Some(title), year, NameSource::CueSheet);
        }
    }

    // ── Route 2: the folder's own name ──────────────────────────────
    let folder_name = dir
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default();
    if let Some((artist, rest)) = folder_name.split_once(FOLDER_NAME_SEPARATOR) {
        let artist = artist.trim();
        let rest = rest.trim();
        if !artist.is_empty() && !rest.is_empty() {
            // Lift a trailing year out of the album half, so the title is the
            // album on its own and the year becomes a tag of its own.
            let (title, year) = TRAILING_YEAR_RE.captures(rest).map_or_else(
                || (rest.to_string(), None),
                |caps| {
                    let year = caps.get(1).map(|m| m.as_str().to_string());
                    let stripped = TRAILING_YEAR_RE.replace(rest, "").trim().to_string();
                    (stripped, year)
                },
            );
            if !title.is_empty() {
                return (
                    Some(artist.to_string()),
                    Some(title),
                    year,
                    NameSource::FolderName,
                );
            }
        }
    }

    (None, None, None, NameSource::None)
}

// ─────────────────────────────────────────────────────────────────────
// Whole-subtree measurement
// ─────────────────────────────────────────────────────────────────────

/// Count every file under `dir`, at any depth, including dotfiles, and add
/// up their sizes.
///
/// Dotfiles are counted on purpose. The folder is going to move as a single
/// sealed unit, so the figure shown to a user should be the number of files
/// that actually move — hiding the `.DS_Store` or a `.checksums` file would
/// make the report quietly wrong.
///
/// Written as an explicit stack rather than recursion so that a pathological
/// directory depth cannot blow the call stack, and symlinked directories are
/// not followed, so a loop cannot hang the scan.
fn measure_subtree(dir: &Path) -> (usize, u64) {
    let mut file_count = 0usize;
    let mut total_bytes = 0u64;
    let mut pending = vec![dir.to_path_buf()];

    while let Some(current) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            // `file_type` on the entry does not follow symlinks, so a symlink
            // to a parent directory is counted as a file and never descended
            // into.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                pending.push(entry.path());
            } else {
                file_count += 1;
                if let Ok(metadata) = entry.metadata() {
                    total_bytes = total_bytes.saturating_add(metadata.len());
                }
            }
        }
    }

    (file_count, total_bytes)
}

/// Whether this directory *directly* contains a file that classifies as
/// ordinary audio or video.
///
/// This is the test that keeps a normal music folder out of disc handling. A
/// folder of FLAC files with a stray `.bin` in it is somebody's music folder,
/// not a disc rip, and its FLACs must keep being renamed as usual. (The stray
/// `.bin` is still protected from renaming — see `partition_for_scan`'s
/// second rule — because being wrong about *that* costs a destroyed disc
/// image, while being wrong the other way costs nothing.)
///
/// Only the directory's own files are examined, not its subdirectories: a rip
/// folder may perfectly well have an `Extras/` subfolder with a video in it.
fn holds_playable_media(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        if let Ok(class) = classify_by_path(&path) {
            if matches!(class.group, MediaGroup::Audio | MediaGroup::Video) {
                return true;
            }
        }
    }
    false
}

/// Assemble a [`DiscFolder`] from a directory and the image sets already
/// found in it.
///
/// Split out from [`detect_disc_folder`] so that `partition_for_scan` can
/// reuse a single directory read: it needs the image sets for every
/// directory it looks at (to protect their members) whether or not the
/// directory turns out to be a disc folder.
fn build_disc_folder(dir: &Path, sets: Vec<DiscImageSet>) -> Option<DiscFolder> {
    if sets.is_empty() || holds_playable_media(dir) {
        return None;
    }

    let kind = resolve_kind_from_layout(&sets);
    let (performer, title, year, name_source) = resolve_name(dir, &sets);
    let track_count = sets
        .iter()
        .find_map(|set| set.cue.as_ref())
        .and_then(|sheet| u32::try_from(sheet.tracks.len()).ok());
    // The first set decides the reported format; `find_image_sets` sorts, so
    // "first" is stable rather than whichever entry the filesystem happened
    // to hand back first.
    let image_format = sets
        .first()
        .map_or(DiscImageFormat::BinCue, |set| set.format);
    let (file_count, total_bytes) = measure_subtree(dir);

    Some(DiscFolder {
        dir: dir.to_path_buf(),
        info: DiscInfo {
            kind,
            image_format,
            image_count: sets.len(),
            track_count,
            // Deliberately empty — see the field's own comment.
            label: None,
            performer,
            title,
            year,
            name_source,
        },
        images: sets,
        file_count,
        total_bytes,
    })
}

/// Decide whether `dir` is a disc folder, and describe it if so.
///
/// A directory is a disc folder when both of these hold:
///
/// * it directly contains at least one disc image set (see
///   [`find_image_sets`]); **and**
/// * it directly contains no file that classifies as ordinary audio or
///   video.
///
/// The second condition is what stops an ordinary music folder being
/// swallowed. A folder of FLAC files that happens to have a `.bin` in it is a
/// music folder — its FLACs should keep being renamed normally. The `.bin`
/// itself is still never renamed individually, but that protection comes from
/// [`partition_for_scan`], not from calling the whole folder a disc folder.
///
/// # Errors
///
/// Returns `Err` if the directory cannot be read.
pub fn detect_disc_folder(dir: &Path) -> MmResult<Option<DiscFolder>> {
    let sets = find_image_sets(dir)?;
    Ok(build_disc_folder(dir, sets))
}

// ─────────────────────────────────────────────────────────────────────
// The safety rule: partitioning a scan
// ─────────────────────────────────────────────────────────────────────

/// Whether `path` sits inside `root` (or is `root` itself).
///
/// A plain prefix comparison on path components, with no filesystem access:
/// the caller's paths all come from the same walk, so they are already in a
/// consistent form, and touching the disk here would be both slower and a
/// source of new failure modes.
fn is_within(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

/// Split a flat list of scanned files into the disc folders they belong to
/// and the ordinary "loose" files that remain.
///
/// **This function is the fix for issue #219.** The rule it enforces is
/// absolute and has no opt-out:
///
/// > A disc image file or cue sheet is never renamed individually, by
/// > anyone, anywhere.
///
/// Two separate exclusions carry that rule:
///
/// 1. Every file anywhere inside a detected disc folder's subtree leaves the
///    loose list, because that whole folder is going to move as one sealed
///    unit later (issue #217) and nothing inside it may be moved on its own
///    before then.
/// 2. Every member of every disc image set leaves the loose list **even when
///    its directory is not a disc folder**. This is the exclusion that
///    matters most: a `.cue` and its `.bin` sitting in a folder full of FLACs
///    are still a disc image, and splitting them still destroys it, even
///    though the folder as a whole is ordinary.
///
/// `scan_root` bounds the work: only directories at or below it are examined,
/// so a file list that somehow reaches outside the scanned tree cannot cause
/// unrelated directories to be read.
///
/// A disc folder nested inside another disc folder is reported once, as part
/// of its outermost parent — the parent is what moves, so listing the child
/// separately would offer the same files to a caller twice.
///
/// The returned loose list keeps the order it was given, so callers see files
/// in the same sequence the scan produced them.
///
/// # Errors
///
/// Returns `Err` if a directory holding scanned files cannot be read.
pub fn partition_for_scan(
    files: Vec<PathBuf>,
    scan_root: &Path,
) -> MmResult<(Vec<DiscFolder>, Vec<PathBuf>)> {
    // Every directory that directly holds at least one scanned file. A
    // BTreeSet gives a stable, sorted order — and sorted order also means a
    // parent directory is always visited before its children, which is what
    // makes the nested-disc-folder check below correct.
    let mut dirs: std::collections::BTreeSet<PathBuf> = std::collections::BTreeSet::new();
    for file in &files {
        if let Some(parent) = file.parent() {
            if is_within(parent, scan_root) {
                dirs.insert(parent.to_path_buf());
            }
        }
    }

    let mut disc_folders: Vec<DiscFolder> = Vec::new();
    // Rule 2's protected set: every file that belongs to any image set found
    // in any of those directories, disc folder or not.
    let mut image_members: HashSet<PathBuf> = HashSet::new();

    for dir in &dirs {
        let sets = find_image_sets(dir)?;
        for set in &sets {
            image_members.extend(set.members.iter().cloned());
        }
        // Skip a directory already covered by a disc folder found higher up:
        // its files are excluded by rule 1 anyway, and reporting it again
        // would double-count them.
        if disc_folders
            .iter()
            .any(|folder| is_within(dir, &folder.dir) && dir != &folder.dir)
        {
            continue;
        }
        if let Some(folder) = build_disc_folder(dir, sets) {
            disc_folders.push(folder);
        }
    }

    let loose = files
        .into_iter()
        .filter(|file| {
            // Rule 2 — an image member is never a loose file.
            if image_members.contains(file) {
                return false;
            }
            // Rule 1 — nor is anything inside a disc folder's subtree.
            !disc_folders
                .iter()
                .any(|folder| is_within(file, &folder.dir))
        })
        .collect();

    Ok((disc_folders, loose))
}

// ─────────────────────────────────────────────────────────────────────
// Feeding the rule engine
// ─────────────────────────────────────────────────────────────────────

/// Turn a disc's details into the ordinary tag names the rule engine already
/// understands.
///
/// The point is that no template has to learn anything new. A user's existing
/// `<Album Artist>/<Album>/...` template works on a disc folder exactly as it
/// works on an album of FLAC files, because the disc's performer arrives as
/// `artist` **and** `album_artist`, and its title arrives as `album` **and**
/// `title`. Writing both members of each pair is deliberate: templates in the
/// wild use either name, and a disc has only the one value to offer.
///
/// The number of images in the folder becomes `disc_total`, so a two-CD rip
/// filed in one folder can be templated as such.
///
/// Fields the disc does not have are simply absent from the map, which is
/// what the rule engine's "missing tag" handling already expects — it must
/// never see an empty string pretending to be a value.
#[must_use]
pub fn synth_tags(info: &DiscInfo) -> TagMap {
    let mut tags = TagMap::new();

    if let Some(performer) = info.performer.as_ref() {
        tags.insert("artist".to_string(), vec![performer.clone()]);
        tags.insert("album_artist".to_string(), vec![performer.clone()]);
    }
    if let Some(title) = info.title.as_ref() {
        tags.insert("album".to_string(), vec![title.clone()]);
        tags.insert("title".to_string(), vec![title.clone()]);
    }
    if let Some(year) = info.year.as_ref() {
        tags.insert("year".to_string(), vec![year.clone()]);
    }
    tags.insert("disc_total".to_string(), vec![info.image_count.to_string()]);

    tags
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Write a file, creating its parent directory first.
    fn write(path: &Path, bytes: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, bytes).unwrap();
    }

    /// A raw sector dump: the CD sync pattern, then filler.
    fn sync_bin(len: usize) -> Vec<u8> {
        let mut bytes = CD_SYNC_PATTERN.to_vec();
        bytes.resize(len.max(CD_SYNC_PATTERN.len()), 0x00);
        bytes
    }

    /// A single-track, all-audio cue sheet naming `image`.
    fn audio_cue(image: &str) -> String {
        format!(
            "PERFORMER \"Test Artist\"\nTITLE \"Test Album\"\nFILE \"{image}\" BINARY\n  \
             TRACK 01 AUDIO\n    INDEX 01 00:00:00\n"
        )
    }

    /// The everyday case: a `.cue` and the `.bin` it names, plus the log and
    /// artwork that came out of the ripper with them.
    #[test]
    fn detects_bin_cue_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Rip");
        write(&dir.join("Album.cue"), audio_cue("Album.bin").as_bytes());
        write(&dir.join("Album.bin"), &sync_bin(4096));
        write(&dir.join("Album.log"), b"ripper log");
        write(&dir.join("cover.jpg"), b"art");

        let folder = detect_disc_folder(&dir).unwrap().expect("a disc folder");
        assert_eq!(folder.info.image_count, 1);
        assert_eq!(folder.info.image_format, DiscImageFormat::BinCue);
        assert_eq!(folder.info.kind, DiscKind::AudioCd);
        assert_eq!(folder.info.track_count, Some(1));
        // The whole subtree is counted, not just the image files.
        assert_eq!(folder.file_count, 4);
        assert!(folder.total_bytes > 4096);
        // Both halves of the pair are members of the one set.
        assert_eq!(folder.images[0].members.len(), 2);
    }

    /// A cue sheet that indexes already-extracted FLAC tracks is a track
    /// listing, not a disc image — the folder is an ordinary album.
    #[test]
    fn cue_for_flac_is_not_a_disc_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Album");
        write(
            &dir.join("Album.cue"),
            b"PERFORMER \"A\"\nTITLE \"B\"\nFILE \"01.flac\" WAVE\n  TRACK 01 AUDIO\n    \
              INDEX 01 00:00:00\n",
        );
        write(&dir.join("01.flac"), b"not really flac");

        assert!(find_image_sets(&dir).unwrap().is_empty());
        assert!(detect_disc_folder(&dir).unwrap().is_none());
    }

    /// A music folder with a stray `.bin` in it stays a music folder — but
    /// the `.bin` is still lifted out of the rename plan, because being wrong
    /// about that costs a destroyed disc image while being wrong the other
    /// way costs nothing.
    #[test]
    fn folder_with_flacs_and_stray_bin_is_not_a_disc_folder_but_bin_is_excluded() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Music");
        let flac = dir.join("01 - Track.flac");
        let bin = dir.join("firmware.bin");
        write(&flac, b"pretend flac");
        write(&bin, &sync_bin(2048));

        // Not a disc folder: real audio sits directly in it.
        assert!(detect_disc_folder(&dir).unwrap().is_none());

        // ...but the .bin never reaches the loose list.
        let (folders, loose) = partition_for_scan(vec![flac.clone(), bin], tmp.path()).unwrap();
        assert!(folders.is_empty());
        assert_eq!(loose, vec![flac]);
    }

    /// A `.bin` with no cue beside it counts as an image only on the evidence
    /// of the raw CD sector sync pattern.
    #[test]
    fn lone_bin_with_sync_pattern_is_an_image() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Dump");
        write(&dir.join("disc.bin"), &sync_bin(4096));

        let sets = find_image_sets(&dir).unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].format, DiscImageFormat::BinCue);
        assert!(sets[0].cue.is_none());
    }

    /// Without that evidence, a `.bin` is left completely alone — it is far
    /// more likely to be a firmware blob than a disc.
    #[test]
    fn lone_bin_without_sync_is_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Blobs");
        write(&dir.join("firmware.bin"), &[0x7Fu8; 4096]);

        assert!(find_image_sets(&dir).unwrap().is_empty());
        assert!(detect_disc_folder(&dir).unwrap().is_none());
    }

    /// An Alcohol 120% image is two files with a shared stem; both belong to
    /// the one set.
    #[test]
    fn mds_mdf_pair() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Alcohol");
        write(&dir.join("Disc.mds"), b"descriptor");
        write(&dir.join("Disc.mdf"), b"image data");

        let sets = find_image_sets(&dir).unwrap();
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0].format, DiscImageFormat::MdsMdf);
        assert_eq!(sets[0].members.len(), 2);
        assert!(sets[0].primary.ends_with("Disc.mds"));
    }

    /// An `.mdf` on its own is ignored: SQL Server uses that extension for
    /// database files, and those are vastly more common.
    #[test]
    fn mdf_alone_is_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Database");
        write(&dir.join("Customers.mdf"), b"SQL Server data file");

        assert!(find_image_sets(&dir).unwrap().is_empty());
    }

    /// `.iso` means exactly one thing, so a folder holding one is a disc
    /// folder with no further evidence needed.
    #[test]
    fn iso_alone_is_a_disc_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Films");
        write(&dir.join("Movie.iso"), b"iso payload");

        let folder = detect_disc_folder(&dir).unwrap().expect("a disc folder");
        assert_eq!(folder.info.image_format, DiscImageFormat::Iso);
        assert_eq!(folder.info.image_count, 1);
        // No cue sheet means nothing states the track types, so the kind is
        // honestly Unknown rather than guessed at.
        assert_eq!(folder.info.kind, DiscKind::Unknown);
    }

    /// A two-CD release filed in one folder is one disc folder holding two
    /// image sets.
    #[test]
    fn multi_disc_folder_counts_two_sets() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Artist - Album");
        write(&dir.join("CD1.cue"), audio_cue("CD1.bin").as_bytes());
        write(&dir.join("CD1.bin"), &sync_bin(1024));
        write(&dir.join("CD2.cue"), audio_cue("CD2.bin").as_bytes());
        write(&dir.join("CD2.bin"), &sync_bin(1024));

        let folder = detect_disc_folder(&dir).unwrap().expect("a disc folder");
        assert_eq!(folder.info.image_count, 2);
        assert_eq!(synth_tags(&folder.info).get("disc_total").unwrap()[0], "2");
    }

    /// Half a name is not a name: a cue sheet with only a title falls through
    /// to the folder-name route.
    #[test]
    fn name_from_cue_needs_both_fields() {
        let tmp = tempfile::tempdir().unwrap();

        // Both fields present -> the cue sheet wins.
        let good = tmp.path().join("whatever");
        write(&good.join("d.cue"), audio_cue("d.bin").as_bytes());
        write(&good.join("d.bin"), &sync_bin(512));
        let folder = detect_disc_folder(&good).unwrap().unwrap();
        assert_eq!(folder.info.name_source, NameSource::CueSheet);
        assert_eq!(folder.info.performer.as_deref(), Some("Test Artist"));
        assert_eq!(folder.info.title.as_deref(), Some("Test Album"));

        // Title only -> not enough; the folder name has no " - " either, so
        // there is no name at all.
        let partial = tmp.path().join("nameless");
        write(
            &partial.join("d.cue"),
            b"TITLE \"Only A Title\"\nFILE \"d.bin\" BINARY\n  TRACK 01 AUDIO\n    \
              INDEX 01 00:00:00\n",
        );
        write(&partial.join("d.bin"), &sync_bin(512));
        let folder = detect_disc_folder(&partial).unwrap().unwrap();
        assert_eq!(folder.info.name_source, NameSource::None);
        assert!(folder.info.title.is_none());
    }

    /// With no usable cue tags, `Artist - Album (1997)` gives up all three
    /// fields, and the brackets do not end up inside the title.
    #[test]
    fn name_from_folder_with_year() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Pink Floyd - The Wall (1979)");
        write(&dir.join("Wall.iso"), b"iso payload");

        let folder = detect_disc_folder(&dir).unwrap().unwrap();
        assert_eq!(folder.info.name_source, NameSource::FolderName);
        assert_eq!(folder.info.performer.as_deref(), Some("Pink Floyd"));
        assert_eq!(folder.info.title.as_deref(), Some("The Wall"));
        assert_eq!(folder.info.year.as_deref(), Some("1979"));

        // Square brackets are just as common as round ones.
        let square = tmp.path().join("Portishead - Dummy [1994]");
        write(&square.join("Dummy.iso"), b"iso payload");
        let folder = detect_disc_folder(&square).unwrap().unwrap();
        assert_eq!(folder.info.title.as_deref(), Some("Dummy"));
        assert_eq!(folder.info.year.as_deref(), Some("1994"));
    }

    /// A folder name with no ` - ` in it tells us nothing, and we say so
    /// rather than inventing an artist.
    #[test]
    fn name_none_when_folder_has_no_separator() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Backup Disc 3");
        write(&dir.join("Backup.iso"), b"iso payload");

        let folder = detect_disc_folder(&dir).unwrap().unwrap();
        assert_eq!(folder.info.name_source, NameSource::None);
        assert!(folder.info.performer.is_none());
        assert!(folder.info.title.is_none());
        assert!(!synth_tags(&folder.info).contains_key("artist"));
    }

    /// Everything under a disc folder leaves the loose list, however deeply
    /// nested — the whole folder is going to move as one unit.
    #[test]
    fn partition_removes_whole_subtree() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("Rip");
        let cue = dir.join("Album.cue");
        let bin = dir.join("Album.bin");
        let log = dir.join("Album.log");
        let art = dir.join("Extras/booklet.jpg");
        write(&cue, audio_cue("Album.bin").as_bytes());
        write(&bin, &sync_bin(2048));
        write(&log, b"log");
        write(&art, b"art");

        // An unrelated file outside the rip must survive untouched.
        let outside = tmp.path().join("Loose/song.mp3");
        write(&outside, b"mp3");

        let (folders, loose) =
            partition_for_scan(vec![cue, bin, log, art, outside.clone()], tmp.path()).unwrap();

        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].dir, dir);
        assert_eq!(loose, vec![outside]);
    }

    /// The four CD layouts a cue sheet can state, each read straight off the
    /// track list.
    #[test]
    fn kind_from_cue_layout() {
        let tmp = tempfile::tempdir().unwrap();

        let cases: [(&str, &str, DiscKind); 4] = [
            (
                "audio",
                "FILE \"d.bin\" BINARY\n TRACK 01 AUDIO\n  INDEX 01 00:00:00\n \
                 TRACK 02 AUDIO\n  INDEX 01 01:00:00\n",
                DiscKind::AudioCd,
            ),
            (
                "enhanced",
                "FILE \"d.bin\" BINARY\n TRACK 01 AUDIO\n  INDEX 01 00:00:00\n \
                 TRACK 02 MODE1/2352\n  INDEX 01 01:00:00\n",
                DiscKind::EnhancedCd,
            ),
            (
                "mixed",
                "FILE \"d.bin\" BINARY\n TRACK 01 MODE1/2352\n  INDEX 01 00:00:00\n \
                 TRACK 02 AUDIO\n  INDEX 01 01:00:00\n",
                DiscKind::MixedModeCd,
            ),
            (
                "data",
                "FILE \"d.bin\" BINARY\n TRACK 01 MODE1/2048\n  INDEX 01 00:00:00\n",
                DiscKind::DataDisc,
            ),
        ];

        for (name, sheet, expected) in cases {
            let dir = tmp.path().join(name);
            write(&dir.join("d.cue"), sheet.as_bytes());
            write(&dir.join("d.bin"), &sync_bin(512));
            let folder = detect_disc_folder(&dir).unwrap().expect("a disc folder");
            assert_eq!(folder.info.kind, expected, "layout {name}");
        }
    }

    /// The disc's performer has to arrive under both tag names, because
    /// templates in the wild use either one and the disc has only one value.
    #[test]
    fn synth_tags_maps_performer_to_artist_and_album_artist() {
        let info = DiscInfo {
            kind: DiscKind::AudioCd,
            image_format: DiscImageFormat::BinCue,
            image_count: 1,
            track_count: Some(12),
            label: None,
            performer: Some("Test Artist".to_string()),
            title: Some("Test Album".to_string()),
            year: Some("1997".to_string()),
            name_source: NameSource::CueSheet,
        };

        let tags = synth_tags(&info);
        assert_eq!(
            tags.get("artist").unwrap(),
            &vec!["Test Artist".to_string()]
        );
        assert_eq!(
            tags.get("album_artist").unwrap(),
            &vec!["Test Artist".to_string()]
        );
        assert_eq!(tags.get("album").unwrap(), &vec!["Test Album".to_string()]);
        assert_eq!(tags.get("title").unwrap(), &vec!["Test Album".to_string()]);
        assert_eq!(tags.get("year").unwrap(), &vec!["1997".to_string()]);
        assert_eq!(tags.get("disc_total").unwrap(), &vec!["1".to_string()]);
    }

    /// Every extension the project treats as possibly part of a disc image,
    /// and a couple that must not be.
    #[test]
    fn disc_image_extensions() {
        for ext in [
            "iso", "ISO", "nrg", "mds", "mdf", "mdx", "cdr", "bin", "cue",
        ] {
            assert!(is_disc_image_extension(ext), "{ext} must be recognised");
        }
        for ext in ["flac", "mp3", "mkv", "zip", "dmg"] {
            assert!(!is_disc_image_extension(ext), "{ext} must not be");
        }
    }
}
