// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Metadata Extraction & Writing Module
//
// This module provides unified metadata reading and writing for audio/video
// files using the `lofty` crate.  It supports ID3v2 (MP3), MP4/M4A atoms,
// Vorbis Comments (OGG/OPUS/FLAC), APE tags, and RIFF INFO (WAV).
//
// Public API:
//   - extract_tags          — read all recognised tags into a TagMap
//   - extract_audio_properties — duration, bitrate, sample rate, channels, etc.
//   - extract_cover_art     — front cover image data + MIME type
//   - write_tags            — write/update tags (preserves existing tags)
//   - remove_tag            — remove a single tag field
//   - embed_cover_art       — embed front cover art
//   - remove_cover_art      — strip all embedded cover art
//   - parse_multi_value     — split "; "-delimited string into Vec<String>
//   - join_multi_value      — join Vec<String> with "; "
//
// ## Relationship to upstream `meedya_core::metadata`
//
// MeedyaManager's local metadata layer is materially different from the
// upstream `meedya_metadata` API and the two are NOT a drop-in swap:
//
//   - Local TagMap = HashMap<String, Vec<String>> — keyed by 40+ MM
//     string IDs (TAG_TITLE, TAG_SORT_*, TAG_PODCAST_*, classical fields, …)
//     that drive the JSON5 template-tag registry used by the rename rule
//     engine and the UI pickers.
//   - Upstream TagMap = HashMap<CommonTag, Vec<String>> — keyed by an
//     enum with ~30 variants, no sort/classical/podcast keys, designed
//     for provider/AcoustID/ReplayGain analyse→write flows.
//
// The local API drives the rule engine, rename templates, FFI tag list,
// and UI panels — all of which depend on the larger string-keyed surface.
// Forcing the upstream enum-only surface here would silently drop sort
// keys, podcast fields, work/movement, original_*, mood, key, etc.  This
// would break user templates that already reference `<SortArtist>`,
// `<Movement>`, `<PodcastTitle>`, and friends.
//
// What we DO migrate in this phase:
//   - Re-export upstream `CommonTag`, `STANDARD_NAMESPACES`, `MetadataError`
//     for use by future provider / fingerprint / ReplayGain integration.
//   - Re-expose upstream tag_io functions under explicit `upstream::` names
//     so MM code that wants the CommonTag-keyed surface can opt in.
//   - Provide conversion helpers between MM string keys and upstream
//     CommonTag enum where the two overlap.
//
// What stays local (and why):
//   - TagMap (String-keyed) — see above.
//   - The full TAG_* constant set — needed by the JSON5 UI registry, the
//     rule-engine tag registry, and downstream consumers.
//   - extract_tags / write_tags / remove_tag / *cover_art / parse/join
//     multi-value — the lofty-backed implementations match upstream's
//     behaviour for the overlapping subset and additionally handle the
//     MM-only tags.
//
// License: GPL-2.0-or-later

/// Tag definition registry — loads from config/tags.json5 at startup.
/// Provides known tag lists for template validation and UI pickers.
pub mod tag_registry;

/// Language tag handling — policy MWBM-MEDIA-LANG 1.0.0. See
/// `docs/standards/media-language-bcp47-policy.md` and this module's own
/// doc comment before touching anything to do with the `language` tag.
pub mod language;

/// A small, bounded reader for a WAV file's RIFF INFO list, so a language
/// save can prove it loses nothing else from that list (Codex's catch-up
/// review of the language-policy branch, finding 1). Crate-private: see
/// its own comment for why it exists and what it cannot do.
mod riff_info;

use std::collections::HashMap;
use std::path::Path;

// lofty 0.22 re-exports — organised by submodule
use lofty::config::WriteOptions; // write-time options (padding, etc.)
use lofty::file::{AudioFile, TaggedFileExt}; // traits: properties(), tags(), etc.
use lofty::picture::{MimeType, Picture, PictureType}; // embedded artwork types
use lofty::probe::Probe; // file format auto-detection
use lofty::tag::{ItemKey, ItemValue, Tag, TagExt, TagItem, TagType};

// The shared implementation of policy MWBM-MEDIA-LANG 1.0.0 — see
// `language`, this module's own submodule, for how it is used here.
use meedya_lang::LanguageTag;

use serde::{Deserialize, Serialize};

use crate::error::{MmError, MmResult};

// ---------------------------------------------------------------------------
// Upstream re-exports — symbols from meedya_core::metadata available alongside
// the MM-local surface.  Consumers that want the CommonTag-keyed enum surface
// (typically provider / fingerprint / ReplayGain integration code) use these.
// ---------------------------------------------------------------------------

/// Upstream `CommonTag` enum — the narrower enum-keyed identifier used by
/// the provider-side write helpers in `meedya_core::metadata::tag_io`.
pub use meedya_core::metadata::CommonTag;

/// Upstream standard namespace aliases (`("itunes", "com.apple.iTunes")`, etc.).
pub use meedya_core::metadata::STANDARD_NAMESPACES;

/// Upstream metadata error type — emitted by upstream tag_io functions and
/// the upstream tag_registry parser.  MeedyaManager keeps wrapping lofty
/// errors into `MmError::Metadata` for its own surfaces, so this is exposed
/// only for callers that opt into the upstream functions via `upstream::`.
pub use meedya_core::metadata::MetadataError;

/// Re-exports of the upstream lofty-backed tag I/O surface.
///
/// These use `HashMap<CommonTag, Vec<String>>` (note: NOT the local TagMap)
/// and are intended for future integration points that need to interop with
/// upstream provider, fingerprint, or ReplayGain code.
///
/// MeedyaManager's own metadata read/write path remains the top-level
/// [`extract_tags`] / [`write_tags`] functions below — those preserve the
/// full MM tag surface including sort keys, classical fields, podcast
/// metadata, and the multi-value semantics that the rule engine depends on.
pub mod upstream {
    // Direct re-exports of the upstream `meedya_core::metadata::tag_io`
    // surface.  Re-exported here so MM code never imports `meedya_metadata`
    // / `meedya_core::metadata` paths directly.
    pub use meedya_core::metadata::tag_io::{
        TagMap, read_tags, write_acoustid_tags, write_registry_tags, write_replaygain_tags,
        write_tags,
    };
}

// ---------------------------------------------------------------------------
// CommonTag <-> MM string-key bridge
// ---------------------------------------------------------------------------

/// Convert an MM string tag key (e.g. `TAG_TITLE`) to the equivalent upstream
/// `CommonTag` enum variant, where one exists.
///
/// Returns `None` for MM tag keys that have no upstream counterpart (sort
/// keys, classical fields, podcast metadata, original_* fields, etc.).
/// These tags exist only in the MM string-keyed surface.
pub fn mm_key_to_common_tag(key: &str) -> Option<CommonTag> {
    match key {
        TAG_TITLE => Some(CommonTag::Title),
        TAG_ARTIST => Some(CommonTag::Artist),
        TAG_ALBUM => Some(CommonTag::Album),
        TAG_ALBUM_ARTIST => Some(CommonTag::AlbumArtist),
        TAG_YEAR => Some(CommonTag::Year),
        TAG_GENRE => Some(CommonTag::Genre),
        TAG_TRACK_NUMBER => Some(CommonTag::TrackNumber),
        TAG_TRACK_TOTAL => Some(CommonTag::TotalTracks),
        TAG_DISC_NUMBER => Some(CommonTag::DiscNumber),
        TAG_DISC_TOTAL => Some(CommonTag::TotalDiscs),
        TAG_COMPOSER => Some(CommonTag::Composer),
        TAG_COMMENT => Some(CommonTag::Comment),
        TAG_LYRICS => Some(CommonTag::Lyrics),
        TAG_ISRC => Some(CommonTag::Isrc),
        TAG_BARCODE => Some(CommonTag::Upc),
        TAG_LABEL => Some(CommonTag::Label),
        TAG_COMPILATION => Some(CommonTag::Compilation),
        TAG_REPLAYGAIN_TRACK_GAIN => Some(CommonTag::ReplayGainTrackGain),
        TAG_REPLAYGAIN_TRACK_PEAK => Some(CommonTag::ReplayGainTrackPeak),
        TAG_REPLAYGAIN_ALBUM_GAIN => Some(CommonTag::ReplayGainAlbumGain),
        TAG_REPLAYGAIN_ALBUM_PEAK => Some(CommonTag::ReplayGainAlbumPeak),
        TAG_ENCODED_BY => Some(CommonTag::Encoder),
        _ => None,
    }
}

/// Convert an upstream `CommonTag` enum variant to the equivalent MM string key.
///
/// This is total — every `CommonTag` variant maps to a defined MM key (the
/// inverse of `mm_key_to_common_tag` for the overlapping subset).
pub fn common_tag_to_mm_key(tag: CommonTag) -> &'static str {
    match tag {
        CommonTag::Title => TAG_TITLE,
        CommonTag::Artist => TAG_ARTIST,
        CommonTag::Album => TAG_ALBUM,
        CommonTag::AlbumArtist => TAG_ALBUM_ARTIST,
        CommonTag::Year => TAG_YEAR,
        CommonTag::Genre => TAG_GENRE,
        CommonTag::TrackNumber => TAG_TRACK_NUMBER,
        CommonTag::TotalTracks => TAG_TRACK_TOTAL,
        CommonTag::DiscNumber => TAG_DISC_NUMBER,
        CommonTag::TotalDiscs => TAG_DISC_TOTAL,
        CommonTag::Composer => TAG_COMPOSER,
        CommonTag::Comment => TAG_COMMENT,
        CommonTag::Lyrics => TAG_LYRICS,
        CommonTag::Isrc => TAG_ISRC,
        CommonTag::Upc => TAG_BARCODE,
        CommonTag::Label => TAG_LABEL,
        CommonTag::Compilation => TAG_COMPILATION,
        CommonTag::ReplayGainTrackGain => TAG_REPLAYGAIN_TRACK_GAIN,
        CommonTag::ReplayGainTrackPeak => TAG_REPLAYGAIN_TRACK_PEAK,
        CommonTag::ReplayGainAlbumGain => TAG_REPLAYGAIN_ALBUM_GAIN,
        CommonTag::ReplayGainAlbumPeak => TAG_REPLAYGAIN_ALBUM_PEAK,
        CommonTag::ReplayGainReferenceLoudness => "replaygain_reference_loudness",
        CommonTag::Encoder => TAG_ENCODED_BY,
        // CommonTag variants without a direct TAG_* constant fall back to
        // their lowercase string name; this only happens for tags the
        // local MM surface doesn't pre-declare as TAG_* constants
        // (Copyright is in the rule-engine extended set but doesn't have
        // a TAG_COPYRIGHT here; the rest are upstream-only fields that
        // round-trip through the upstream tag_io surface).
        CommonTag::Copyright => "copyright",
        CommonTag::MusicBrainzRecordingId => "mb_recording_id",
        CommonTag::MusicBrainzReleaseId => "mb_release_id",
        CommonTag::AcoustId => "acoustid",
        CommonTag::ReleaseDate => "release_date",
        CommonTag::Description => "description",
    }
}

// ---------------------------------------------------------------------------
// Tag key constants — canonical string keys used throughout MeedyaManager
// ---------------------------------------------------------------------------

/// Track title
pub const TAG_TITLE: &str = "title";
/// Performing artist(s)
pub const TAG_ARTIST: &str = "artist";
/// Album name
pub const TAG_ALBUM: &str = "album";
/// Album artist (may differ from track artist on compilations)
pub const TAG_ALBUM_ARTIST: &str = "album_artist";
/// Release year (4-digit string, e.g. "2024")
pub const TAG_YEAR: &str = "year";
/// Genre (free-text)
pub const TAG_GENRE: &str = "genre";
/// Track number within disc (e.g. "3")
pub const TAG_TRACK_NUMBER: &str = "track_number";
/// Total tracks on disc (e.g. "12")
pub const TAG_TRACK_TOTAL: &str = "track_total";
/// Disc number (e.g. "1")
pub const TAG_DISC_NUMBER: &str = "disc_number";
/// Total discs (e.g. "2")
pub const TAG_DISC_TOTAL: &str = "disc_total";
/// Composer / songwriter
pub const TAG_COMPOSER: &str = "composer";
/// Free-text comment
pub const TAG_COMMENT: &str = "comment";
/// Lyrics (unsynced)
pub const TAG_LYRICS: &str = "lyrics";
/// International Standard Recording Code
pub const TAG_ISRC: &str = "isrc";
/// Barcode / UPC / EAN
pub const TAG_BARCODE: &str = "barcode";
/// Catalogue number (label release identifier)
pub const TAG_CATALOG_NUMBER: &str = "catalog_number";
/// Record label name
pub const TAG_LABEL: &str = "label";
/// "1" for compilation / various-artists releases, "0" otherwise
pub const TAG_COMPILATION: &str = "compilation";
/// Beats per minute (integer as string)
pub const TAG_BPM: &str = "bpm";

// ── Sort fields (used by Apple Music / iTunes for correct alphabetical sort) ──
/// Track title sort key (e.g. "Sacrifice, The" → "The Sacrifice" sorts under T)
pub const TAG_TITLE_SORT: &str = "title_sort";
/// Performing artist sort key
pub const TAG_ARTIST_SORT: &str = "artist_sort";
/// Album title sort key
pub const TAG_ALBUM_SORT: &str = "album_sort";
/// Album artist sort key
pub const TAG_ALBUM_ARTIST_SORT: &str = "album_artist_sort";
/// Composer sort key
pub const TAG_COMPOSER_SORT: &str = "composer_sort";

// ── Extended attribution ─────────────────────────────────────────────────────
/// Conductor name (classical music)
pub const TAG_CONDUCTOR: &str = "conductor";
/// Remixer or mix engineer
pub const TAG_REMIXER: &str = "remixer";
/// Primary lyricist
pub const TAG_LYRICIST: &str = "lyricist";
/// Language of the lyrics, as a BCP 47 tag (policy MWBM-MEDIA-LANG 1.0.0).
///
/// E.g. `"en"`, `"pt-BR"`, `"zh-Hant"`; `"und"` means "not known". See
/// `metadata::language` for how a value is read, validated and written —
/// ID3's `TLAN` frame holds the old three-letter form (there being no field
/// in ID3 that can hold a full tag at all), every other format holds the
/// tag itself.
pub const TAG_LANGUAGE: &str = "language";
/// Emotional mood tag (e.g. "Melancholic", "Upbeat")
pub const TAG_MOOD: &str = "mood";
/// Content grouping (iTunes "Grouping" field; used for classical works)
pub const TAG_GROUPING: &str = "grouping";

// ── Classical music fields ───────────────────────────────────────────────────
/// The overarching work title (e.g. "Symphony No. 5 in C minor")
pub const TAG_WORK: &str = "work";
/// Movement name within a work (e.g. "I. Allegro con brio")
pub const TAG_MOVEMENT: &str = "movement";
/// Movement index within the work (integer as string, e.g. "1")
pub const TAG_MOVEMENT_INDEX: &str = "movement_index";
/// Total number of movements in the work
pub const TAG_MOVEMENT_TOTAL: &str = "movement_total";

// ── ReplayGain (loudness normalisation) ─────────────────────────────────────
/// Per-track ReplayGain gain value in dB (e.g. "-6.54 dB")
pub const TAG_REPLAYGAIN_TRACK_GAIN: &str = "replaygain_track_gain";
/// Per-track ReplayGain peak sample value (e.g. "0.987654")
pub const TAG_REPLAYGAIN_TRACK_PEAK: &str = "replaygain_track_peak";
/// Album-level ReplayGain gain value in dB
pub const TAG_REPLAYGAIN_ALBUM_GAIN: &str = "replaygain_album_gain";
/// Album-level ReplayGain peak sample value
pub const TAG_REPLAYGAIN_ALBUM_PEAK: &str = "replaygain_album_peak";

// ── Encoding information ─────────────────────────────────────────────────────
/// Name of the encoder software (e.g. "LAME 3.100", "Apple iTunes 12.9.0.164")
pub const TAG_ENCODED_BY: &str = "encoded_by";
/// Encoding tool / settings string
pub const TAG_ENCODER_SETTINGS: &str = "encoder_settings";
/// Original release year (before remaster), 4-digit string
pub const TAG_ORIGINAL_YEAR: &str = "original_year";
/// Original album title (before remaster/reissue)
pub const TAG_ORIGINAL_ALBUM: &str = "original_album";
/// Original performing artist (before cover/remake)
pub const TAG_ORIGINAL_ARTIST: &str = "original_artist";

// ── Podcast-specific fields ─────────────────────────────────────────────────
/// Podcast title (iTunes podcast feed title)
pub const TAG_PODCAST_TITLE: &str = "podcast_title";
/// Podcast episode identifier / GUID
pub const TAG_PODCAST_ID: &str = "podcast_id";
/// Podcast feed URL
pub const TAG_PODCAST_URL: &str = "podcast_url";
/// Podcast category (e.g. "Technology", "True Crime")
pub const TAG_PODCAST_CATEGORY: &str = "podcast_category";
/// Podcast description / episode notes
pub const TAG_PODCAST_DESCRIPTION: &str = "podcast_description";

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

/// Multi-value tag map: each key can hold one or more string values.
/// For example, multiple artists are stored as `vec!["Artist A", "Artist B"]`.
pub type TagMap = HashMap<String, Vec<String>>;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Technical audio properties extracted from a file's stream header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioProperties {
    /// Playback duration in fractional seconds
    pub duration_secs: f64,
    /// Overall bitrate in kbps (kilobits per second)
    pub bitrate_kbps: Option<u32>,
    /// Sample rate in Hz (e.g. 44100, 48000, 96000)
    pub sample_rate_hz: Option<u32>,
    /// Number of audio channels (1 = mono, 2 = stereo, ...)
    pub channels: Option<u8>,
    /// Bit depth per sample (e.g. 16, 24, 32); None for lossy codecs
    pub bits_per_sample: Option<u8>,
}

/// Embedded cover-art image extracted from a media file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverArt {
    /// Raw image bytes (typically JPEG or PNG)
    pub data: Vec<u8>,
    /// MIME type string, e.g. "image/jpeg" or "image/png"
    pub mime: String,
}

// ---------------------------------------------------------------------------
// Multi-value helpers
// ---------------------------------------------------------------------------

/// Split a string containing multiple values separated by "; " (semicolon
/// followed by a space) into individual trimmed values.
///
/// Empty segments are discarded.
///
/// # Examples
/// ```
/// # use mm_core::metadata::parse_multi_value;
/// let v = parse_multi_value("Rock; Pop; Electronic");
/// assert_eq!(v, vec!["Rock", "Pop", "Electronic"]);
/// ```
pub fn parse_multi_value(value: &str) -> Vec<String> {
    // Split on "; " — the canonical MeedyaManager multi-value delimiter
    value
        .split("; ") // split on exact "; " sequence
        .map(|s| s.trim().to_string()) // trim any stray whitespace
        .filter(|s| !s.is_empty()) // discard empty segments
        .collect()
}

/// Join multiple string values into a single "; "-delimited string.
///
/// # Examples
/// ```
/// # use mm_core::metadata::join_multi_value;
/// let joined = join_multi_value(&["Rock".into(), "Pop".into()]);
/// assert_eq!(joined, "Rock; Pop");
/// ```
pub fn join_multi_value(values: &[String]) -> String {
    values.join("; ")
}

// ---------------------------------------------------------------------------
// Tag key <-> lofty ItemKey mapping
// ---------------------------------------------------------------------------

/// Return the list of (MeedyaManager string key, lofty ItemKey) pairs that
/// we recognise.  This is the single source of truth for field mapping.
fn tag_key_mappings() -> Vec<(&'static str, ItemKey)> {
    vec![
        // ── Core tags ────────────────────────────────────────────────────────
        (TAG_TITLE, ItemKey::TrackTitle),
        (TAG_ARTIST, ItemKey::TrackArtist),
        (TAG_ALBUM, ItemKey::AlbumTitle),
        (TAG_ALBUM_ARTIST, ItemKey::AlbumArtist),
        (TAG_YEAR, ItemKey::Year),
        (TAG_GENRE, ItemKey::Genre),
        (TAG_TRACK_NUMBER, ItemKey::TrackNumber),
        (TAG_TRACK_TOTAL, ItemKey::TrackTotal),
        (TAG_DISC_NUMBER, ItemKey::DiscNumber),
        (TAG_DISC_TOTAL, ItemKey::DiscTotal),
        (TAG_COMPOSER, ItemKey::Composer),
        (TAG_COMMENT, ItemKey::Comment),
        (TAG_LYRICS, ItemKey::Lyrics),
        (TAG_ISRC, ItemKey::Isrc),
        (TAG_BARCODE, ItemKey::Barcode),
        (TAG_CATALOG_NUMBER, ItemKey::CatalogNumber),
        (TAG_LABEL, ItemKey::Label),
        (TAG_COMPILATION, ItemKey::FlagCompilation),
        (TAG_BPM, ItemKey::Bpm),
        // ── Sort fields ───────────────────────────────────────────────────────
        (TAG_TITLE_SORT, ItemKey::TrackTitleSortOrder),
        (TAG_ARTIST_SORT, ItemKey::TrackArtistSortOrder),
        (TAG_ALBUM_SORT, ItemKey::AlbumTitleSortOrder),
        (TAG_ALBUM_ARTIST_SORT, ItemKey::AlbumArtistSortOrder),
        (TAG_COMPOSER_SORT, ItemKey::ComposerSortOrder),
        // ── Extended attribution ──────────────────────────────────────────────
        (TAG_CONDUCTOR, ItemKey::Conductor),
        (TAG_REMIXER, ItemKey::Remixer),
        (TAG_LYRICIST, ItemKey::Lyricist),
        (TAG_LANGUAGE, ItemKey::Language),
        (TAG_MOOD, ItemKey::Mood),
        (TAG_GROUPING, ItemKey::ContentGroup),
        // ── Classical music ───────────────────────────────────────────────────
        (TAG_WORK, ItemKey::Work),
        (TAG_MOVEMENT, ItemKey::Movement),
        (TAG_MOVEMENT_INDEX, ItemKey::MovementNumber),
        (TAG_MOVEMENT_TOTAL, ItemKey::MovementTotal),
        // ── ReplayGain ────────────────────────────────────────────────────────
        (TAG_REPLAYGAIN_TRACK_GAIN, ItemKey::ReplayGainTrackGain),
        (TAG_REPLAYGAIN_TRACK_PEAK, ItemKey::ReplayGainTrackPeak),
        (TAG_REPLAYGAIN_ALBUM_GAIN, ItemKey::ReplayGainAlbumGain),
        (TAG_REPLAYGAIN_ALBUM_PEAK, ItemKey::ReplayGainAlbumPeak),
        // ── Encoding information ──────────────────────────────────────────────
        (TAG_ENCODED_BY, ItemKey::EncodedBy),
        (TAG_ENCODER_SETTINGS, ItemKey::EncoderSettings),
        (TAG_ORIGINAL_YEAR, ItemKey::OriginalReleaseDate),
        (TAG_ORIGINAL_ALBUM, ItemKey::OriginalAlbumTitle),
        (TAG_ORIGINAL_ARTIST, ItemKey::OriginalArtist),
        // ── Podcast ───────────────────────────────────────────────────────────
        // Note: lofty 0.22 only exposes PodcastUrl and PodcastDescription.
        // PodcastTitle, PodcastIdentifier, and PodcastCategory are MeedyaManager
        // keys without a direct lofty ItemKey mapping (handled via custom tags).
        (TAG_PODCAST_URL, ItemKey::PodcastUrl),
        (TAG_PODCAST_DESCRIPTION, ItemKey::PodcastDescription),
    ]
}

/// Look up the lofty `ItemKey` for one of our string tag keys.
/// Returns `None` if the key is not in our standard mapping.
pub fn mm_key_to_item_key(key: &str) -> Option<ItemKey> {
    // Linear scan is fine — the mapping has <20 entries
    tag_key_mappings()
        .into_iter()
        .find(|(mm_key, _)| *mm_key == key)
        .map(|(_, ik)| ik)
}

/// Look up our string tag key for a lofty `ItemKey`.
/// Returns `None` if the ItemKey is not in our standard mapping.
pub fn item_key_to_mm_key(ik: &ItemKey) -> Option<&'static str> {
    tag_key_mappings()
        .into_iter()
        .find(|(_, mapped_ik)| mapped_ik == ik)
        .map(|(mm_key, _)| mm_key)
}

/// Return every MeedyaManager tag key that can actually be persisted to a file.
///
/// Derived from `tag_key_mappings()` (the internal function that is the single
/// source of truth for the key ↔ `ItemKey` bridge), so this list can never drift
/// from what [`write_tags`] and [`remove_tag`] will accept.
///
/// Callers use it to validate user-supplied keys *before* starting any I/O —
/// see `mm-cli`'s `edit` command, which needs the whole batch to be
/// all-or-nothing.
///
/// Note the deliberate omissions: `podcast_title`, `podcast_id` and
/// `podcast_category` have `TAG_*` constants but no lofty `ItemKey`, so they
/// cannot round-trip through any tag container we support and are therefore
/// *not* valid write keys.
///
/// # Examples
/// ```
/// # use mm_core::metadata::{known_tag_keys, TAG_TITLE};
/// assert!(known_tag_keys().contains(&TAG_TITLE));
/// assert!(!known_tag_keys().contains(&"bogus_key"));
/// ```
pub fn known_tag_keys() -> Vec<&'static str> {
    tag_key_mappings().into_iter().map(|(key, _)| key).collect()
}

/// Build the `MmError::Metadata` returned when a caller supplies a tag key
/// with no `ItemKey` mapping.
///
/// The message names every offending key *and* enumerates the valid ones,
/// because the caller is usually a human typing `--set` on a command line (or
/// a UI developer guessing at a key name) and a bare "unknown key" would leave
/// them no way forward.
fn unknown_tag_key_error(keys: &[&str]) -> MmError {
    // Quote each offending key so an empty or whitespace-only key is visible.
    let offenders = keys
        .iter()
        .map(|k| format!("'{k}'"))
        .collect::<Vec<_>>()
        .join(", ");

    MmError::Metadata(format!(
        "unknown tag key{plural} {offenders} — valid keys: {valid}",
        plural = if keys.len() == 1 { "" } else { "s" },
        valid = known_tag_keys().join(", ")
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers — file probing
// ---------------------------------------------------------------------------

/// Open and probe a media file, returning the parsed `TaggedFile`.
/// Wraps lofty errors into our `MmError::Metadata` variant with context.
fn open_tagged_file(path: &Path) -> MmResult<lofty::file::TaggedFile> {
    Probe::open(path)
        .map_err(|e| MmError::Metadata(format!("Cannot open '{}': {}", path.display(), e)))?
        .read()
        .map_err(|e| {
            MmError::Metadata(format!("Cannot read tags from '{}': {}", path.display(), e))
        })
}

// ---------------------------------------------------------------------------
// Extraction functions
// ---------------------------------------------------------------------------

/// Read all recognised tags from the file at `path` and return them as a
/// [`TagMap`].
///
/// Multi-value fields (e.g. multiple artists stored in separate tag frames)
/// are collected into the same `Vec<String>` entry.  If a field appears in
/// more than one tag type (e.g. both ID3v2 and APE in an MP3), values are
/// merged and deduplicated.
///
/// # Errors
/// Returns `MmError::Metadata` if the file cannot be opened or has no
/// parseable tags; returns `MmError::Lofty` for lower-level codec errors.
pub fn extract_tags(path: &Path) -> MmResult<TagMap> {
    // Probe the file — this auto-detects format (MP3, FLAC, MP4, OGG, ...)
    let tagged_file = open_tagged_file(path)?;

    // Initialise the output map
    let mut tag_map: TagMap = HashMap::new();

    // Iterate over every tag container the file has (ID3v2, Vorbis, MP4, ...)
    for tag in tagged_file.tags() {
        // Walk our known mappings and pull matching values
        read_tag_into_map(tag, &mut tag_map);
    }

    // TRACK-070 (review item 5 of the second language-policy review round):
    // the generic loop above treats `language` like any other key, so a
    // WAV `write_tags` has kept consistent across BOTH its RIFF INFO chunk
    // and its embedded ID3v2 tag (see `write_tags`'s own doc comment) came
    // back as TWO values — `["en", "eng"]` — the canonical form from RIFF
    // INFO and the three-letter form from ID3v2, which are the SAME fact
    // told twice, not two different answers. `read_language_values` applies
    // TRACK-070's own priority (a full-tag container's answer is read on
    // its own; ID3v2's narrower one is only read when there is no full-tag
    // answer to prefer) and deduplicates by STANDARD form rather than raw
    // text, so this overwrites whatever the generic loop put there.
    match read_language_values(&tagged_file) {
        values if values.is_empty() => {
            tag_map.remove(TAG_LANGUAGE);
        }
        values => {
            tag_map.insert(TAG_LANGUAGE.to_string(), values);
        }
    }

    Ok(tag_map)
}

/// The read-side counterpart to [`language_write_targets`] (the write
/// side): decides what [`extract_tags`] reports for `TAG_LANGUAGE` when a
/// file carries the value in more than one tag container at once.
///
/// A "full" container (Vorbis comments, the MP4 freeform item, APE, RIFF
/// INFO) can hold a complete BCP 47 tag; ID3v2's `TLAN` can only ever hold
/// the old three-letter form. TRACK-070 says the three-letter field "MUST
/// NOT be read back when the full tag is present", so when a full
/// container has a value, that is read as the answer and ID3v2's narrower
/// one is not also read — reading back a WAV that [`write_tags`] has kept
/// consistent used to give `["en", "eng"]`, two representations of the
/// identical fact, rather than one. When no full container has a value,
/// ID3v2's is read on its own. (This comment used to cite COMPAT-040 for
/// that priority; the rule is TRACK-070's — corrected after the third
/// review round.)
///
/// What this cannot do on its own: say when the ID3v2 value it skips means
/// something DIFFERENT from what it shows (a WAV whose RIFF INFO chunk says
/// `fre` and whose ID3 tag says `ger`). That is [`language_disagreement`]'s
/// job — kept separate so this function's answer, which `write_tags`
/// compares a resent value against (COMPAT-030), stays exactly what
/// [`extract_tags`] shows.
///
/// Within whichever tier is read, values are deduplicated by their
/// STANDARD form ([`language::standardise_for_comparison`]), not by their
/// raw text — so `"en"` and `"eng"` collapse into one entry even though
/// they are different strings, while `"en"` and `"en-GB"` do not, because
/// they genuinely are different languages/regions. The first raw text seen
/// for each standard form is kept (editors show raw text — COMPAT-040), so
/// a value nothing recognises is never silently dropped either: an
/// unrecognised value's own text is its own "standard form" for this
/// purpose (`standardise_for_comparison`'s own LANG-003 guarantee).
fn read_language_values(tagged_file: &lofty::file::TaggedFile) -> Vec<String> {
    let (full_raw, id3_raw) = raw_language_values_by_tier(tagged_file);

    let chosen = if full_raw.is_empty() {
        id3_raw
    } else {
        full_raw
    };

    let mut seen_standard_forms: Vec<String> = Vec::new();
    let mut result: Vec<String> = Vec::new();
    for raw in chosen {
        let standard = language::standardise_for_comparison(&raw);
        if !seen_standard_forms.contains(&standard) {
            seen_standard_forms.push(standard);
            result.push(raw);
        }
    }
    result
}

/// Every `language` value in `tagged_file`, as stored (whitespace trimmed,
/// empty values skipped), split into the two groups TRACK-070 treats
/// differently: first every tag that can hold a full language code (Vorbis
/// comments, the MP4 freeform item, APE, RIFF INFO), then the ID3v2 tag,
/// which only ever holds the three-letter form. File order within each.
///
/// Several values held in ONE stored string, separated by zero characters
/// (an APE item does this; `lofty` splits only ID3's), are split here, in
/// order, by [`language::split_stored_values`] — so every format is read
/// the way the ID3 path already was (Codex's catch-up review, finding 4).
fn raw_language_values_by_tier(
    tagged_file: &lofty::file::TaggedFile,
) -> (Vec<String>, Vec<String>) {
    let mut full: Vec<String> = Vec::new();
    let mut id3: Vec<String> = Vec::new();

    for tag in tagged_file.tags() {
        let bucket = if tag.tag_type() == TagType::Id3v2 {
            &mut id3
        } else {
            &mut full
        };
        for item in tag.get_items(&ItemKey::Language) {
            if let ItemValue::Text(text) = item.value() {
                bucket.extend(language::split_stored_values(text));
            }
        }
    }
    (full, id3)
}

/// When a file's tags disagree about the language: what is shown, and what
/// is hidden that says something different.
///
/// Third review round, item 3: [`read_language_values`] reads a tag that can
/// hold the full language code in preference to the ID3 tag's three-letter
/// one (TRACK-070), and that priority is right — but it meant a WAV whose
/// RIFF INFO chunk says `fre` while its ID3 tag says `ger` was shown as
/// plain `fre`, and the German was invisible everywhere. Worse, `meedya edit
/// --set language=fre` on that file reported "✓ Set" and changed nothing
/// (resending the value that is already shown is, correctly, a no-change —
/// COMPAT-030 — so the ID3 tag kept saying German). COMPAT-040 says doubt
/// like this SHOULD be reported for a person to fix, so this finds it and
/// the notes in [`language`] say it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LanguageDisagreement {
    /// The values [`extract_tags`] shows, exactly as stored.
    pub(crate) shown: Vec<String>,
    /// Each value the file's ID3 tag holds that does not say the same as
    /// any shown value, exactly as stored, in file order, each once.
    pub(crate) hidden_id3: Vec<String>,
}

/// Find a disagreement in `tagged_file`, or `None` when its tags agree (or
/// when nothing is hidden at all — only one kind of tag holds a language).
///
/// "Agrees" means what TRACK-070 would have written: a shown value `fre`
/// agrees with an ID3 value `fra` or `fre` (both French), and a shown
/// `en-GB` or `yue` agrees with an ID3 `eng` or `und` — those are exactly
/// what `write_tags` itself puts in an ID3 tag for them, because the ID3 tag
/// cannot hold a region, and has no three-letter code for Cantonese. So the
/// comparison is between the STANDARD form of the hidden ID3 value and the
/// standard form of what an ID3 tag would hold for each shown value. Without
/// that, every file `write_tags` has correctly kept in step (`en-GB` in the
/// RIFF chunk, `eng` in the ID3 tag) would be reported as disagreeing.
///
/// A shown value nothing recognises (`English`) has no "what an ID3 tag
/// would hold"; it agrees only with an ID3 value that is the same text. So
/// `English` beside an ID3 `eng` IS reported: the two may well mean the
/// same, but MeedyaManager cannot know that without guessing (LANG-003),
/// and COMPAT-040 asks for doubt to be reported, not settled by a guess.
pub(crate) fn language_disagreement(
    tagged_file: &lofty::file::TaggedFile,
) -> Option<LanguageDisagreement> {
    let (full_raw, id3_raw) = raw_language_values_by_tier(tagged_file);
    if full_raw.is_empty() || id3_raw.is_empty() {
        return None; // nothing is hidden, so nothing can disagree
    }

    let shown = read_language_values(tagged_file);
    let what_id3_would_hold: Vec<String> = shown
        .iter()
        .map(|value| {
            let stored = language::parse_stored_language(value);
            if stored.recognised {
                let id3_form = language::language_value_for_tag_type(&stored.tag, TagType::Id3v2);
                language::standardise_for_comparison(&id3_form)
            } else {
                stored.raw
            }
        })
        .collect();

    let mut hidden_id3: Vec<String> = Vec::new();
    for value in id3_raw {
        if !what_id3_would_hold.contains(&language::standardise_for_comparison(&value))
            && !hidden_id3.contains(&value)
        {
            hidden_id3.push(value);
        }
    }

    if hidden_id3.is_empty() {
        None
    } else {
        Some(LanguageDisagreement { shown, hidden_id3 })
    }
}

/// Internal helper: read values from one `Tag` into the tag map.
fn read_tag_into_map(tag: &Tag, map: &mut TagMap) {
    // For each recognised mapping, try to read items from this tag
    for (mm_key, item_key) in tag_key_mappings() {
        // `get_items()` returns an iterator over all TagItems matching
        // this key (handles multi-value frames in ID3v2, multiple Vorbis
        // comment fields, etc.)
        for item in tag.get_items(&item_key) {
            // Only process text values — binary items are ignored here
            if let ItemValue::Text(text) = item.value() {
                // Trim whitespace from the value
                let trimmed = text.trim();

                // Skip empty values
                if !trimmed.is_empty() {
                    // Append to the vector, deduplicating identical strings
                    let entry = map.entry(mm_key.to_string()).or_default();
                    if !entry.contains(&trimmed.to_string()) {
                        entry.push(trimmed.to_string());
                    }
                }
            }
        }
    }
}

/// Extract technical audio properties (duration, bitrate, sample rate, etc.)
/// from the file at `path`.
///
/// # Errors
/// Returns an error if the file cannot be opened or its audio stream header
/// is unreadable.
pub fn extract_audio_properties(path: &Path) -> MmResult<AudioProperties> {
    // Probe and read the file
    let tagged_file = open_tagged_file(path)?;

    // Retrieve the file-level properties (codec-independent)
    let props = tagged_file.properties();

    // Build and return the AudioProperties struct
    Ok(AudioProperties {
        // Duration as fractional seconds
        duration_secs: props.duration().as_secs_f64(),
        // Overall bitrate (may be None for some lossless formats)
        bitrate_kbps: props.overall_bitrate(),
        // Sample rate in Hz
        sample_rate_hz: props.sample_rate(),
        // Channel count (lofty returns u8)
        channels: props.channels(),
        // Bit depth per sample (None for lossy codecs like MP3/AAC)
        bits_per_sample: props.bit_depth(),
    })
}

/// Extract the front cover image from the file at `path`.
///
/// Returns `Ok(Some(CoverArt))` if a front-cover picture is found,
/// `Ok(None)` if the file has no embedded artwork, or an error if the
/// file cannot be read.
pub fn extract_cover_art(path: &Path) -> MmResult<Option<CoverArt>> {
    // Probe and read the file
    let tagged_file = open_tagged_file(path)?;

    // Search every tag container for a front-cover picture
    for tag in tagged_file.tags() {
        for picture in tag.pictures() {
            // We specifically look for PictureType::CoverFront first
            if picture.pic_type() == PictureType::CoverFront {
                return Ok(Some(CoverArt {
                    data: picture.data().to_vec(),
                    mime: mime_type_to_string(picture.mime_type()),
                }));
            }
        }
    }

    // Fall back: if no PictureType::CoverFront was found, return the first
    // picture of any type (some files only tag "Other")
    for tag in tagged_file.tags() {
        if let Some(picture) = tag.pictures().first() {
            return Ok(Some(CoverArt {
                data: picture.data().to_vec(),
                mime: mime_type_to_string(picture.mime_type()),
            }));
        }
    }

    // No pictures at all
    Ok(None)
}

// ---------------------------------------------------------------------------
// MIME type conversion helpers
// ---------------------------------------------------------------------------

/// Convert a lofty `MimeType` to its IANA string representation.
fn mime_type_to_string(mt: Option<&MimeType>) -> String {
    let Some(mt) = mt else {
        return "application/octet-stream".to_string();
    };
    match mt {
        MimeType::Jpeg => "image/jpeg".to_string(),
        MimeType::Png => "image/png".to_string(),
        MimeType::Bmp => "image/bmp".to_string(),
        MimeType::Gif => "image/gif".to_string(),
        MimeType::Tiff => "image/tiff".to_string(),
        // Catch-all for future MimeType variants or Unknown(...)
        _ => "application/octet-stream".to_string(),
    }
}

/// Parse a MIME type string into a lofty `MimeType`.
fn string_to_mime_type(mime: &str) -> MimeType {
    match mime.to_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => MimeType::Jpeg,
        "image/png" => MimeType::Png,
        "image/bmp" => MimeType::Bmp,
        "image/gif" => MimeType::Gif,
        "image/tiff" => MimeType::Tiff,
        _ => MimeType::Unknown(mime.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Writing functions
// ---------------------------------------------------------------------------

/// Obtain a mutable reference to the primary tag of a `TaggedFile`,
/// creating one if none exists.  This ensures we always have a tag to
/// write into.
fn get_or_create_primary_tag(tagged_file: &mut lofty::file::TaggedFile) -> &mut Tag {
    // If the file already has a primary tag, return it; otherwise insert one
    if tagged_file.primary_tag_mut().is_none() {
        let tag_type = tagged_file.primary_tag_type();
        tagged_file.insert_tag(Tag::new(tag_type));
    }
    tagged_file
        .primary_tag_mut()
        .expect("primary tag must exist after insert_tag")
}

/// Write (or update) tags in the file at `path`.
///
/// Existing tags that are NOT present in the supplied `tags` map are
/// **preserved** — only the keys present in `tags` are overwritten.
///
/// Multi-value entries (e.g. `vec!["Artist A", "Artist B"]`) are joined
/// with "; " before writing, because most tag formats store a single text
/// frame per key.  To write truly separate frames you would call the
/// lower-level lofty API directly.
///
/// ## `language` is special (policy MWBM-MEDIA-LANG 1.0.0, TRACK-070)
///
/// Every other key is written exactly as given. `language` is not: the
/// value supplied is read with the LANG-002 rules (so both `en-GB` and
/// `eng` are accepted) and then re-written in whatever form the file's tag
/// container actually wants (`metadata::language::language_value_for_tag_type`)
/// — ID3's `TLAN` frame gets the ISO 639-2 terminology three-letter code,
/// every other container gets the canonical BCP 47 tag itself.
///
/// This means the caller can leave `language` out of `tags` entirely to
/// guarantee it is not touched (the ordinary way to "touch a file for
/// another reason") — but that is not the ONLY way an unchanged value
/// survives. **A real editor does not usually do that.** Every native UI
/// this project has (macOS, Windows, the Linux GTK app) reads the whole
/// tag set into a form and resends every field on Save, changed or not —
/// found, and reproduced on real files, while this rule was being reviewed
/// (COMPAT-030 again): a FLAC whose `LANGUAGE` comment already said
/// `English` (a real value some other tool wrote) had ITS WHOLE SAVE
/// REFUSED merely because the title changed, since resending `English`
/// hit the exact same "not a language we recognise" refusal a person
/// typing it fresh would. So this function does not decide "should
/// `language` be touched?" purely from whether the key is present — it
/// FIRST reads what the file already has for `language` (using the same
/// read logic [`extract_tags`] does, across every tag container the file
/// carries, joined the same way [`join_multi_value`] joins any multi-value
/// field) and compares it, as plain text, against the supplied value. When
/// they are IDENTICAL, `language` is left completely alone — not
/// re-validated, not re-converted, not even re-written with the same
/// bytes — exactly as if the key had never been supplied. Only a value
/// that is genuinely DIFFERENT from what is already there goes through
/// validation and per-container conversion. This is what makes "resend
/// everything unchanged" and "leave the key out" behave the same way,
/// which is the only way COMPAT-030 can hold for a real editor rather than
/// only for a caller that carefully omits untouched keys.
///
/// # Errors
/// Returns an error if the file cannot be opened, read, or saved, or if
/// `tags` sets `language` to a genuinely NEW value (different from what
/// the file already has) that LANG-002 does not recognise at all (see
/// [`language::parse_language_input`]) — refused before any key is
/// written, for the same all-or-nothing reason as an unknown key.
pub fn write_tags(path: &Path, tags: &TagMap) -> MmResult<()> {
    let TagWritePlan {
        mut tagged_file,
        language_change,
        other_tag_types,
    } = TagWritePlan::new(path, tags)?;

    // -- Codex's catch-up review, finding 1 (every save since the stand-in
    // review of round 6): a WAV must lose nothing from its RIFF INFO list.
    // This save rewrites that list only when it holds a language and the
    // language is changing; every other field is written into the WAV's ID3
    // tag. Checked here, BEFORE the first save, so a refusal leaves the file
    // completely as it was — which matters for a Test Mode copy an earlier
    // edit made, which `integrity::mutate_file_safe` keeps rather than
    // deletes on failure — and checked again after the last save, against
    // what is really on disk. See `RiffInfoGuard`. [`check_tag_write`] runs
    // exactly this check, for a preview.
    let riff_change = riff_change_for_write(language_change.as_ref(), &other_tag_types);
    let riff_guard = RiffInfoGuard::before_saving(path, &tagged_file, &riff_change)?;

    // Get (or create) the primary tag for this file format
    let tag = get_or_create_primary_tag(&mut tagged_file);

    // Write each entry from the supplied map into the tag.
    // Every key is known to map — the validation pass above returned early
    // otherwise — so no key can be dropped on the floor here.
    for (key, values) in tags {
        if key == TAG_LANGUAGE {
            // `language_change` already folds together "the key was
            // absent" and "the key was present but identical" into `None`
            // — either way, this branch leaves it completely alone: no
            // remove_key, no re-push, not even with the same bytes. See
            // this function's own doc comment for why "present but
            // identical" must behave exactly like "absent", not merely
            // like "converted to the same canonical form".
            if let Some(change) = &language_change {
                apply_language_change(tag, change);
            }
            continue;
        }

        // Every other key is joined and written exactly as it always has
        // been — no COMPAT-030 special-casing needed there today; this
        // crate does not yet validate or convert any tag but `language`.
        let item_key = mm_key_to_item_key(key)
            .expect("validated above: every key in `tags` has an ItemKey mapping");
        let joined = join_multi_value(values);
        tag.remove_key(&item_key);
        if !joined.is_empty() {
            let item = TagItem::new(item_key, ItemValue::Text(joined));
            tag.push(item);
        }
    }

    // Persist to disk using default write options (preserves format quirks)
    tag.save_to_path(path, WriteOptions::default())?;

    // -- Item 5 (and, since the second review round, item 3 too): when
    // `language` is genuinely being SET to a new value OR cleared outright,
    // keep every OTHER tag container the file already has consistent with
    // that too — not only the primary one. A file that already carries a
    // language value in more than one container (RIFF INFO's `ILNG`
    // alongside an embedded ID3v2 tag on a WAV file, most concretely) must
    // not end up with the change applied to one and a now-stale,
    // contradicting OLD value left behind in the other. Each container
    // that already had a language value gets the SAME treatment the
    // primary one just did — its own per-format TRACK-070 form when
    // setting, a plain removal when clearing, never a copy of the
    // primary's raw bytes — and each is saved with its own `save_to_path`
    // call, because a single `Tag` only ever writes its own container's
    // region of the file (this is also why the primary tag above needed
    // its own separate call). A container that never had a language value
    // is deliberately left alone: this fixes a stale value, it does not go
    // looking for new places to put one.
    //
    // Clearing was missed in the first round of this fix — found by an
    // independent review that actually set up a WAV with a language value
    // in two containers, cleared it, and read both back: `remove_tag`
    // (the general "delete this key" path) already loops over every
    // container the file has, so a language clear silently doing less
    // than an ordinary field's removal was the surprising direction for
    // the two to have drifted apart in.
    if let Some(change) = &language_change {
        for tag_type in other_tag_types {
            let Some(other_tag) = tagged_file.tag_mut(tag_type) else {
                continue; // the type was listed a moment ago; still defensive
            };
            if other_tag.get_items(&ItemKey::Language).next().is_none() {
                continue; // never had one — not this fix's job to add or touch one
            }
            if tag_type == TagType::RiffInfo {
                // The same description the check before saving worked from.
                riff_change.apply_to(other_tag);
            } else {
                apply_language_change(other_tag, change);
            }
            other_tag.save_to_path(path, WriteOptions::default())?;
        }
    }

    // Finding 1, second half: what is really on disk now.
    if let Some(guard) = riff_guard {
        guard.after_saving(path)?;
    }

    Ok(())
}

/// Everything [`write_tags`] works out before it changes anything: the
/// keys are known, the file is read, whether `language` really changes
/// (COMPAT-030), and which other containers hold a language. Shared with
/// [`check_tag_write`], so a preview and the save it predicts work from the
/// same answers (the stand-in review of round 6, M2).
struct TagWritePlan {
    /// The file as read, to be changed in memory and saved.
    tagged_file: lofty::file::TaggedFile,
    /// What `language` is really asked to do; `None` leaves it alone.
    language_change: Option<LanguageChange>,
    /// The containers other than the primary that hold a language.
    other_tag_types: Vec<TagType>,
}

impl TagWritePlan {
    /// Validate `tags` and read `path` — nothing is written.
    fn new(path: &Path, tags: &TagMap) -> MmResult<Self> {
        // -- Validate every key BEFORE touching the file (issue #206) ----------
        //
        // A key with no `ItemKey` mapping cannot be persisted by any tag format
        // we support.  Previously such keys were dropped inside the write loop,
        // so `meedya edit --set bogus=1` reported "✓ Set" having changed nothing.
        // Rejecting up-front also makes the write all-or-nothing: on a bad key we
        // never open the file, so the caller's other keys are not half-applied.
        let mut unknown: Vec<&str> = tags
            .keys()
            .filter(|key| mm_key_to_item_key(key).is_none())
            .map(String::as_str)
            .collect();
        if !unknown.is_empty() {
            // TagMap is a HashMap, so its iteration order is randomised per
            // process — sort so the message is reproducible in logs and tests.
            unknown.sort_unstable();
            return Err(unknown_tag_key_error(&unknown));
        }

        // Open and read the existing file so we can preserve its tags. This is
        // read-only until `tag.save_to_path` at the very end — nothing on disk
        // changes if a check below returns an error first, same all-or-nothing
        // guarantee as the unknown-key check above, just necessarily reached a
        // little later because the COMPAT-030 comparison just below needs to
        // see what the file already holds before it can decide anything.
        let tagged_file = open_tagged_file(path)?;

        // -- COMPAT-030: is a supplied `language` value actually NEW? ----------
        //
        // `None` means "leave `language` alone" — covers both "the key was
        // absent from `tags`" and "the key was present but identical to what
        // is already stored". The comparison is against what extract_tags
        // would report TODAY, not against what this crate would canonicalise
        // the input to — the whole point is to leave an untouched value
        // exactly as it is, including one this crate could never have parsed,
        // not to decide "close enough".
        let language_change: Option<LanguageChange> = match tags.get(TAG_LANGUAGE) {
            None => None,
            Some(values) => {
                let joined = join_multi_value(values);
                if current_joined_value(&tagged_file, TAG_LANGUAGE).as_deref()
                    == Some(joined.as_str())
                {
                    None
                } else if joined.is_empty() {
                    // A genuine change TO empty (clearing the field): no
                    // language to parse, same as any other key.
                    Some(LanguageChange::Clear)
                } else {
                    let parsed = language::parse_language_input(&joined).map_err(|e| {
                        MmError::Metadata(format!("cannot set '{TAG_LANGUAGE}': {e}"))
                    })?;
                    Some(LanguageChange::Set(Box::new(parsed)))
                }
            }
        };

        // -- Item 5 groundwork: which OTHER tag containers does this file
        // already have a language value in, besides the primary one? Collected
        // now, while `tagged_file` is only borrowed immutably, because getting
        // the primary tag mutably (in `write_tags`) borrows it exclusively
        // until that borrow's last use. A file can genuinely carry more than one tag
        // container at once (a WAV with both a RIFF INFO chunk and an
        // embedded ID3v2 tag, say) — found and reproduced while this rule was
        // being reviewed: setting `language` only ever touched the primary
        // container, leaving a stale, contradicting value in any other one
        // that already had its own. See the loop after the primary tag is
        // saved, below.
        //
        // `language_write_targets` is the single source of truth for this set
        // — also used by `language::preview_conversion_note` so a caller
        // previewing `--set language=...` (including on `--dry-run`, before
        // any write happens) sees a note that describes EXACTLY what this
        // function is actually about to do, never a guess based on the
        // primary container alone (review item 4 of the second language-
        // policy review round: the note used to describe a plan, not what
        // `write_tags` would really do to a file with more than one
        // container).
        let primary_type = tagged_file.primary_tag_type();
        let other_tag_types: Vec<TagType> = language_write_targets(&tagged_file)
            .into_iter()
            .filter(|tt| *tt != primary_type)
            .collect();

        Ok(Self {
            tagged_file,
            language_change,
            other_tag_types,
        })
    }
}

/// What a [`write_tags`] save does to a WAV's RIFF INFO list: it rewrites it
/// only when the list holds a language (`other_tag_types` names it) and the
/// language is really changing.
fn riff_change_for_write<'a>(
    language_change: Option<&'a LanguageChange>,
    other_tag_types: &[TagType],
) -> RiffChange<'a> {
    match language_change {
        Some(change) if other_tag_types.contains(&TagType::RiffInfo) => {
            RiffChange::Language(change)
        }
        _ => RiffChange::Untouched,
    }
}

/// A read-only form of the check [`write_tags`] makes before it writes
/// anything.
///
/// Reads `path` and answers `Err` with the same refusal the save would give
/// — an unknown key, a language nothing recognises, or a WAV whose RIFF
/// INFO list the save would damage (`RiffInfoGuard`). Writes nothing.
///
/// Why it exists (the stand-in review of round 6, M2): `meedya edit
/// --dry-run` never saves, so it never ran the RIFF INFO check, and said a
/// change would succeed that the real run refused. Reproduced with the
/// `meedya` binary built from `49cec29` on a WAV with a Latin-1 title:
/// `--set language=en --dry-run` printed "✓ Set language = en" and exited 0,
/// the real run refused it and exited 2.
///
/// What it cannot do: predict a failure that only the save itself meets — a
/// full disk, a file another program changes in between, or what the check
/// AFTER saving finds. It answers "would the save refuse, as things stand
/// now", nothing more.
///
/// # Errors
/// The refusal [`write_tags`] would return before saving.
pub fn check_tag_write(path: &Path, tags: &TagMap) -> MmResult<()> {
    let plan = TagWritePlan::new(path, tags)?;
    let riff_change = riff_change_for_write(plan.language_change.as_ref(), &plan.other_tag_types);
    RiffInfoGuard::before_saving(path, &plan.tagged_file, &riff_change)?;
    Ok(())
}

/// Put one language change into one tag container, in that container's own
/// TRACK-070 form: remove what it held, then (when setting) add the new
/// value. The ONE place this is done — for the primary container, for every
/// other container that already held a language, and (through
/// [`RiffChange::apply_to`]) for the copy [`RiffInfoGuard::before_saving`]
/// checks before anything is written — so what is checked can never drift
/// from what is saved.
fn apply_language_change(tag: &mut Tag, change: &LanguageChange) {
    tag.remove_key(&ItemKey::Language);
    if let LanguageChange::Set(parsed) = change {
        let value = language::language_value_for_tag_type(parsed, tag.tag_type());
        if !value.is_empty() {
            tag.push(TagItem::new(ItemKey::Language, ItemValue::Text(value)));
        }
    }
}

/// What one save does to a WAV's RIFF INFO list — the ONE description both
/// the save and [`RiffInfoGuard::before_saving`] work from, so what is
/// checked can never drift from what is saved.
enum RiffChange<'a> {
    /// The save does not rewrite the RIFF INFO list at all: it writes only
    /// the WAV's ID3 tag (any field other than a language the list already
    /// holds, and cover art), or nothing.
    Untouched,
    /// The list's language entry changes (`write_tags`, when the list
    /// already holds a language and the language is changing).
    Language(&'a LanguageChange),
    /// Every entry for this field is removed (`remove_tag`, when the list
    /// holds the field).
    Remove(&'a ItemKey),
}

impl RiffChange<'_> {
    /// Make this change to `tag` — used for the real save's RIFF INFO
    /// container and for the copy the check before saving dumps, alike.
    fn apply_to(&self, tag: &mut Tag) {
        match self {
            Self::Untouched => {}
            Self::Language(change) => apply_language_change(tag, change),
            Self::Remove(item_key) => tag.remove_key(item_key),
        }
    }
}

/// Whether one raw RIFF INFO entry is one `lofty` files under `item_key` —
/// so `IPRT` and `ITRK` both count for the track number, as `lofty` reads
/// both as one.
fn riff_entry_is(entry: &riff_info::InfoEntry, item_key: &ItemKey) -> bool {
    std::str::from_utf8(&entry.id)
        .is_ok_and(|id| ItemKey::from_key(TagType::RiffInfo, id) == *item_key)
}

/// Codex's catch-up review, finding 1, and the stand-in review of round 6
/// (carry-over 1 and M1): proof that a save of a WAV loses nothing from its
/// RIFF INFO list.
///
/// Why it is needed: the `lofty` tag library writes a RIFF INFO list back
/// whole, from what it read — and it cannot read an entry whose text is not
/// UTF-8 (RIFF INFO names no encoding, and older Windows tools write their
/// own code page), so in its forgiving reading mode it leaves such an entry
/// out. Reproduced with the `meedya` binary built from `a150926`: a WAV
/// whose title was "Café" in Latin-1 lost its title when its language was
/// set, cleared or removed, and the save reported success. Round 6 guarded
/// those language saves only; with the binary built from `49cec29`,
/// `--remove artist` (on a file with no artist) and `--remove-cover` still
/// deleted that title, so every save of a WAV is guarded now.
///
/// How: the list is read RAW (id and bytes, nothing decoded, so nothing can
/// be skipped — see `riff_info`) before anything is saved. Then twice:
///
/// 1. [`before_saving`](Self::before_saving), when the save will rewrite the
///    list, refuses a file with more than one INFO list (#259: `lofty`
///    rewrites only the first, merging the second into it — a difference
///    the entry comparison cannot see, because `lofty`'s own prediction
///    already holds the merged entries), then asks `lofty` for the exact
///    bytes it WOULD write for the list (`TagExt::dump_to`, which uses the
///    same code as its file writer) and compares them — so a save that
///    would lose something is refused before a single byte changes. A save
///    that does not rewrite the list predicts it unchanged.
/// 2. [`after_saving`](Self::after_saving) reads the list RAW again from the
///    file and compares that — the proof from what is really on disk.
///
/// Both compare with `riff_info::check_entries_kept`: every entry the save
/// was not asked to change byte for byte the same and in the same order,
/// and every asked-for entry holding exactly what was asked (or gone, when
/// clearing or removing). Any difference refuses the save with a message
/// naming what would be lost. Where a save is written and what happens to
/// that file on a refusal is decided by `integrity::mutate_file_safe`,
/// unchanged: the person's own file is never replaced by one that failed.
///
/// What it cannot do: a refusal by the check AFTER saving comes after the
/// save. Outside Test Mode that save went to a temporary file, which is
/// thrown away; in Test Mode, once an earlier edit has made the copy, the
/// save went into that copy. That is why the check before saving must catch
/// everything it can predict; the check after is the proof, not the plan.
struct RiffInfoGuard {
    /// The list's entries before anything was saved.
    before: Vec<riff_info::InfoEntry>,
    /// The entries the save was asked to change, and what each must hold.
    asked: riff_info::AskedChanges,
}

impl RiffInfoGuard {
    /// Read the list as it is now and refuse, before anything is written,
    /// if the save would lose or change anything it was not asked to.
    /// `Ok(None)` for a file that is not a WAV: there is nothing to guard.
    fn before_saving(
        path: &Path,
        tagged_file: &lofty::file::TaggedFile,
        change: &RiffChange<'_>,
    ) -> MmResult<Option<Self>> {
        let Some(lists) = riff_info::read_info_lists(path).map_err(MmError::Metadata)? else {
            return Ok(None);
        };
        let riff_tag = tagged_file.tag(TagType::RiffInfo);
        let asked: riff_info::AskedChanges = match change {
            RiffChange::Untouched => Vec::new(),
            RiffChange::Language(LanguageChange::Clear) => {
                vec![(riff_info::LANGUAGE_ID, riff_info::Expectation::Absent)]
            }
            RiffChange::Language(LanguageChange::Set(parsed)) => {
                let value = language::language_value_for_tag_type(parsed, TagType::RiffInfo);
                let expected = if value.is_empty() {
                    riff_info::Expectation::Absent
                } else {
                    riff_info::Expectation::Holds(value)
                };
                vec![(riff_info::LANGUAGE_ID, expected)]
            }
            RiffChange::Remove(item_key) => {
                let mut ids: riff_info::AskedChanges = Vec::new();
                for entry in &lists.entries {
                    if riff_entry_is(entry, item_key) && !ids.iter().any(|(id, _)| *id == entry.id)
                    {
                        ids.push((entry.id, riff_info::Expectation::Absent));
                    }
                }
                ids
            }
        };

        if !matches!(change, RiffChange::Untouched) {
            if lists.list_count > 1 {
                return Err(MmError::Metadata(riff_info::two_lists_refusal(
                    lists.list_count,
                )));
            }
            // What `lofty` would write: its own copy of the list, given the
            // same change the save will make, written into memory instead of
            // a file. No copy at all means `lofty` read nothing it could use
            // from the list — every entry is one it cannot read — so it has
            // nothing to rewrite the list from, and the save cannot do what
            // was asked; the comparison against an empty prediction names
            // what would be lost.
            let predicted = match riff_tag {
                Some(riff_tag) => {
                    let mut would_write = riff_tag.clone();
                    change.apply_to(&mut would_write);
                    let mut bytes = Vec::new();
                    would_write.dump_to(&mut bytes, WriteOptions::default())?;
                    riff_info::entries_from_list_chunk(&bytes).map_err(MmError::Metadata)?
                }
                None => Vec::new(),
            };
            riff_info::check_entries_kept(&lists.entries, &predicted, &asked)
                .map_err(MmError::Metadata)?;
            if riff_tag.is_none() && !lists.entries.is_empty() {
                // Only reached when every entry is asked to go: the save
                // still cannot rewrite a list `lofty` never read.
                return Err(MmError::Metadata(
                    "MeedyaManager's tag library cannot read the file's RIFF INFO list as it \
                     is stored, so the change was refused"
                        .to_string(),
                ));
            }
        }

        Ok(Some(Self {
            before: lists.entries,
            asked,
        }))
    }

    /// Read the list again, from the file as saved, and refuse if anything
    /// the save was not asked to change differs.
    fn after_saving(self, path: &Path) -> MmResult<()> {
        let after = riff_info::read_info_lists(path)
            .map_err(MmError::Metadata)?
            .map(|lists| lists.entries)
            .unwrap_or_default();
        riff_info::check_entries_kept(&self.before, &after, &self.asked).map_err(MmError::Metadata)
    }
}

/// Which tag containers [`write_tags`] will actually put a NEW `language`
/// value into (or remove one from) for this file: the primary tag type
/// always — creating a fresh container if the file has none — plus every
/// OTHER container the file already carries that already has its own
/// language value. A container that has never had one is never a target
/// (see `write_tags`'s own "Item 5" doc comment: this keeps existing
/// values in sync, it does not go looking for new places to put one).
///
/// `pub(crate)` rather than private: [`language::preview_conversion_note`]
/// calls this too, so a caller previewing what `--set language=...` will
/// do (including on `--dry-run`, before `write_tags` itself ever runs) is
/// guaranteed to see the same set of containers `write_tags` will actually
/// touch — there is no second copy of "which containers count" to drift
/// out of step with this one (review item 4 of the second language-policy
/// review round found the note computed this from the primary container
/// alone, which was wrong for any file carrying a language value in more
/// than one place).
pub(crate) fn language_write_targets(tagged_file: &lofty::file::TaggedFile) -> Vec<TagType> {
    let primary_type = tagged_file.primary_tag_type();
    let mut targets = vec![primary_type];
    for tag in tagged_file.tags() {
        let tt = tag.tag_type();
        if tt != primary_type
            && !targets.contains(&tt)
            && tag.get_items(&ItemKey::Language).next().is_some()
        {
            targets.push(tt);
        }
    }
    targets
}

/// What a caller supplied for `language` actually means to do, once
/// compared against what the file already has (COMPAT-030) — see
/// [`write_tags`]'s own doc comment. There is deliberately no variant for
/// "unchanged"; that case is represented by the absence of this type
/// (`Option::None`) one level up, precisely so it is handled identically
/// to "the key was never supplied" rather than as a third case that could
/// quietly diverge from it.
enum LanguageChange {
    /// The supplied value is empty: the field is being cleared outright.
    Clear,
    /// The supplied value is non-empty and genuinely different from what
    /// the file already has, and LANG-002 could make sense of it. Boxed
    /// because `LanguageTag` is a good deal larger than the `Clear` variant
    /// (it carries a script, region, every variant and extension subtag,
    /// and a list of notes) — without this, EVERY `LanguageChange` would
    /// pay `LanguageTag`'s size even in the `Clear` case, which never uses
    /// one at all.
    Set(Box<LanguageTag>),
}

/// What [`extract_tags`] would currently report for `key`, if this file
/// already has one, joined the way every multi-value write in this crate
/// does ([`join_multi_value`]). `None` means the file has nothing at all
/// for this key today (not even an empty value) — genuinely different from
/// "an empty string", which is what a deliberate clear looks like.
///
/// Used by [`write_tags`] to tell "the caller resent an unchanged value"
/// apart from "the caller is genuinely setting a new one" for `language`
/// (COMPAT-030) — this is `pub(crate)`, not private, for exactly the same
/// reason as [`language_write_targets`]: [`language::preview_conversion_note`]
/// needs to make the identical "unchanged?" judgement `write_tags` itself
/// will make, so a `--dry-run` preview and the real write can never
/// disagree about whether anything is actually changing.
///
/// For `key == TAG_LANGUAGE` this goes through [`read_language_values`] —
/// the same TRACK-070-aware, cross-container, standard-form-deduplicated
/// logic [`extract_tags`] uses — rather than the generic per-tag loop,
/// which would otherwise disagree with what a caller reading the file via
/// `extract_tags` actually sees whenever a file carries a language value
/// in more than one container. Every other key still uses the generic
/// loop: this crate does not yet special-case reading anything but
/// `language`, and the only caller today only ever asks about `language`.
pub(crate) fn current_joined_value(
    tagged_file: &lofty::file::TaggedFile,
    key: &str,
) -> Option<String> {
    if key == TAG_LANGUAGE {
        let values = read_language_values(tagged_file);
        return if values.is_empty() {
            None
        } else {
            Some(join_multi_value(&values))
        };
    }
    let mut map: TagMap = HashMap::new();
    for tag in tagged_file.tags() {
        read_tag_into_map(tag, &mut map);
    }
    map.get(key).map(|values| join_multi_value(values))
}

/// Everything [`remove_tag`] works out before it changes anything: the key
/// is known, the file is read, and which containers hold the field. Shared
/// with [`check_tag_removal`] (the stand-in review of round 6, M2).
struct TagRemovalPlan {
    /// The field being removed.
    item_key: ItemKey,
    /// The file as read, to be changed in memory and saved.
    tagged_file: lofty::file::TaggedFile,
    /// The containers that hold the field — the only ones rewritten.
    targets: Vec<TagType>,
    /// Whether the save rewrites a WAV's RIFF INFO list (or must, and
    /// cannot — see below).
    rewrites_riff_info: bool,
}

impl TagRemovalPlan {
    /// Validate `key` and read `path` — nothing is written.
    fn new(path: &Path, key: &str) -> MmResult<Self> {
        // Reject an unmapped key before any I/O — see `remove_tag`'s doc
        // comment.
        let item_key = mm_key_to_item_key(key).ok_or_else(|| unknown_tag_key_error(&[key]))?;

        // Open and read the existing file
        let tagged_file = open_tagged_file(path)?;

        // Which containers hold the field — only those are rewritten. Codex's
        // catch-up review, finding 1, made this the rule for `language`; the
        // stand-in review of round 6 (carry-over 1) found every other field
        // still rewrote EVERY container, whether or not it held the field — for
        // a WAV, its whole RIFF INFO list. Reproduced with the `meedya` binary
        // built from `49cec29`: `--remove artist` on a WAV with no artist at
        // all, whose RIFF INFO title was Latin-1 "Café", deleted the title and
        // reported success. Rewriting a container that holds nothing to remove
        // can only lose something, never gain anything.
        //
        // The RIFF INFO list is asked RAW as well as through `lofty`: an entry
        // `lofty` cannot read (that Latin-1 title, when the title is what is
        // being removed) is still in the file, so the list must still be
        // rewritten for the removal to happen.
        let raw_riff_holds = riff_info::read_info_lists(path)
            .map_err(MmError::Metadata)?
            .is_some_and(|lists| lists.entries.iter().any(|e| riff_entry_is(e, &item_key)));
        let targets: Vec<TagType> = tagged_file
            .tags()
            .iter()
            .filter(|tag| {
                tag.get_items(&item_key).next().is_some()
                    || (tag.tag_type() == TagType::RiffInfo && raw_riff_holds)
            })
            .map(lofty::tag::Tag::tag_type)
            .collect();

        // Every save of a WAV must lose nothing else from its RIFF INFO list
        // (`RiffInfoGuard`). When the list holds the field but `lofty` read
        // nothing at all from it, it is not among `targets` — and the check
        // before saving refuses, because the removal cannot be done.
        let rewrites_riff_info = targets.contains(&TagType::RiffInfo)
            || (raw_riff_holds && tagged_file.tag(TagType::RiffInfo).is_none());
        Ok(Self {
            item_key,
            tagged_file,
            targets,
            rewrites_riff_info,
        })
    }
}

/// A read-only form of the check [`remove_tag`] makes before it writes
/// anything — see [`check_tag_write`], which is the same for a write.
///
/// # Errors
/// The refusal [`remove_tag`] would return before saving.
pub fn check_tag_removal(path: &Path, key: &str) -> MmResult<()> {
    let plan = TagRemovalPlan::new(path, key)?;
    let riff_change = if plan.rewrites_riff_info {
        RiffChange::Remove(&plan.item_key)
    } else {
        RiffChange::Untouched
    };
    RiffInfoGuard::before_saving(path, &plan.tagged_file, &riff_change)?;
    Ok(())
}

/// Remove a specific tag field from the file at `path`.
///
/// The `key` must be one of the `TAG_*` constants (see [`known_tag_keys`]).
/// Removing a key the file does not carry is a successful no-op; supplying a
/// key that is not in our mapping at all is an **error** (issue #206) — it is
/// always a caller mistake, and returning `Ok(())` for it used to make
/// `meedya edit --remove bogus` report success having done nothing.
///
/// # Errors
/// Returns `MmError::Metadata` if `key` has no `ItemKey` mapping, or if the
/// file cannot be opened, read, or saved.
pub fn remove_tag(path: &Path, key: &str) -> MmResult<()> {
    let TagRemovalPlan {
        item_key,
        mut tagged_file,
        targets,
        rewrites_riff_info,
    } = TagRemovalPlan::new(path, key)?;

    // Every save of a WAV must lose nothing else from its RIFF INFO list
    // (`RiffInfoGuard`). [`check_tag_removal`] runs exactly this check, for a
    // preview.
    let riff_change = if rewrites_riff_info {
        RiffChange::Remove(&item_key)
    } else {
        RiffChange::Untouched
    };
    let riff_guard = RiffInfoGuard::before_saving(path, &tagged_file, &riff_change)?;

    // Remove the key from each container that holds it, then save that one.
    for tt in &targets {
        if let Some(tag) = tagged_file.tag_mut(*tt) {
            if *tt == TagType::RiffInfo {
                // The same description the check before saving worked from.
                riff_change.apply_to(tag);
            } else {
                tag.remove_key(&item_key);
            }
            // Save this tag back to disk
            tag.save_to_path(path, WriteOptions::default())?;
        }
    }

    if let Some(guard) = riff_guard {
        guard.after_saving(path)?;
    }

    Ok(())
}

/// Embed front-cover art into the file at `path`.
///
/// `data` is the raw image bytes (JPEG, PNG, etc.) and `mime` is the MIME
/// type string (e.g. "image/jpeg").
///
/// If the file already has a front-cover picture, it is **replaced**.
///
/// # Errors
/// Returns an error if the file cannot be opened, read, or saved.
pub fn embed_cover_art(path: &Path, data: &[u8], mime: &str) -> MmResult<()> {
    // Open and read the existing file
    let mut tagged_file = open_tagged_file(path)?;

    // A picture goes into the primary tag only (for a WAV, its ID3 tag), so
    // a WAV's RIFF INFO list must come through byte for byte
    // (`RiffInfoGuard`: every save of a WAV is checked).
    let riff_guard = RiffInfoGuard::before_saving(path, &tagged_file, &RiffChange::Untouched)?;

    // Build the Picture struct with front-cover type
    let picture = Picture::new_unchecked(
        PictureType::CoverFront,         // picture type: front cover
        Some(string_to_mime_type(mime)), // MIME type
        None,                            // no description
        data.to_vec(),                   // raw image data
    );

    // Get or create the primary tag
    let tag = get_or_create_primary_tag(&mut tagged_file);

    // Remove any existing front-cover pictures before adding the new one
    tag.remove_picture_type(PictureType::CoverFront);

    // Push the new picture into the tag
    tag.push_picture(picture);

    // Save to disk
    tag.save_to_path(path, WriteOptions::default())?;

    if let Some(guard) = riff_guard {
        guard.after_saving(path)?;
    }

    Ok(())
}

/// Remove ALL embedded cover art from the file at `path`.
///
/// This strips pictures from every tag container in the file.
///
/// # Errors
/// Returns an error if the file cannot be opened, read, or saved.
pub fn remove_cover_art(path: &Path) -> MmResult<()> {
    // Open and read the existing file
    let mut tagged_file = open_tagged_file(path)?;

    // Only the containers that hold a picture are rewritten. Every
    // container used to be, which for a WAV meant its whole RIFF INFO list —
    // a list that cannot even hold a picture. Reproduced with the `meedya`
    // binary built from `49cec29` (the stand-in review of round 6,
    // carry-over 1): `--remove-cover` on a WAV whose RIFF INFO title was
    // Latin-1 "Café" deleted the title, because `lofty` cannot read that
    // entry and writes the list back without it.
    let tag_types: Vec<TagType> = tagged_file
        .tags()
        .iter()
        .filter(|tag| !tag.pictures().is_empty())
        .map(lofty::tag::Tag::tag_type)
        .collect();

    // So a WAV's RIFF INFO list must come through byte for byte
    // (`RiffInfoGuard`: every save of a WAV is checked).
    let riff_guard = RiffInfoGuard::before_saving(path, &tagged_file, &RiffChange::Untouched)?;

    // All known picture types to remove
    let picture_types = [
        PictureType::CoverFront,
        PictureType::CoverBack,
        PictureType::Other,
        PictureType::Icon,
        PictureType::OtherIcon,
        PictureType::Leaflet,
        PictureType::Media,
        PictureType::LeadArtist,
        PictureType::Artist,
        PictureType::Conductor,
        PictureType::Band,
        PictureType::Composer,
        PictureType::Lyricist,
        PictureType::RecordingLocation,
        PictureType::DuringRecording,
        PictureType::DuringPerformance,
        PictureType::ScreenCapture,
        PictureType::BrightFish,
        PictureType::Illustration,
        PictureType::BandLogo,
        PictureType::PublisherLogo,
    ];

    // Strip pictures from every tag container
    for tt in &tag_types {
        if let Some(tag) = tagged_file.tag_mut(*tt) {
            // Remove every known picture type
            for pt in &picture_types {
                tag.remove_picture_type(*pt);
            }
            // Save this tag back to disk
            tag.save_to_path(path, WriteOptions::default())?;
        }
    }

    if let Some(guard) = riff_guard {
        guard.after_saving(path)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Multi-value parsing tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_multi_value_basic() {
        // Standard "; "-separated string splits into three values
        let result = parse_multi_value("Rock; Pop; Electronic");
        assert_eq!(result, vec!["Rock", "Pop", "Electronic"]);
    }

    #[test]
    fn parse_multi_value_single_value() {
        // A single value with no delimiter returns a one-element vec
        let result = parse_multi_value("Rock");
        assert_eq!(result, vec!["Rock"]);
    }

    #[test]
    fn parse_multi_value_empty_string() {
        // Empty input should produce an empty vec
        let result = parse_multi_value("");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_multi_value_whitespace_only() {
        // "  ;  " contains "; " after the semicolon, so it splits into
        // two segments: "  " and " " — both trim to empty, filtered out
        let result = parse_multi_value("  ;  ");
        assert!(result.is_empty());
    }

    #[test]
    fn parse_multi_value_trailing_delimiter() {
        // Trailing "; " produces an empty last segment that gets filtered out
        let result = parse_multi_value("Rock; Pop; ");
        assert_eq!(result, vec!["Rock", "Pop"]);
    }

    #[test]
    fn parse_multi_value_leading_delimiter() {
        // Leading "; " produces an empty first segment that gets filtered out
        let result = parse_multi_value("; Rock; Pop");
        assert_eq!(result, vec!["Rock", "Pop"]);
    }

    #[test]
    fn parse_multi_value_extra_whitespace() {
        // "  Rock ;  Pop  ;  Jazz  " contains "; " after each semicolon,
        // so it splits and trims whitespace from each segment
        let result = parse_multi_value("  Rock ;  Pop  ;  Jazz  ");
        assert_eq!(result, vec!["Rock", "Pop", "Jazz"]);
    }

    #[test]
    fn parse_multi_value_semicolon_no_space() {
        // Semicolons without a trailing space are NOT delimiters
        let result = parse_multi_value("Rock;Pop;Jazz");
        assert_eq!(result, vec!["Rock;Pop;Jazz"]);
    }

    #[test]
    fn parse_multi_value_unicode() {
        // Unicode values are handled correctly
        let result = parse_multi_value("Bjork; Sigur Ros; mum");
        assert_eq!(result, vec!["Bjork", "Sigur Ros", "mum"]);
    }

    #[test]
    fn join_multi_value_basic() {
        // Standard join of multiple values
        let values = vec![
            "Rock".to_string(),
            "Pop".to_string(),
            "Electronic".to_string(),
        ];
        assert_eq!(join_multi_value(&values), "Rock; Pop; Electronic");
    }

    #[test]
    fn join_multi_value_single() {
        // Single value — no delimiter inserted
        let values = vec!["Rock".to_string()];
        assert_eq!(join_multi_value(&values), "Rock");
    }

    #[test]
    fn join_multi_value_empty() {
        // Empty slice produces empty string
        let values: Vec<String> = vec![];
        assert_eq!(join_multi_value(&values), "");
    }

    #[test]
    fn roundtrip_multi_value() {
        // parse -> join should round-trip cleanly for well-formed input
        let original = "Artist A; Artist B; Artist C";
        let parsed = parse_multi_value(original);
        let joined = join_multi_value(&parsed);
        assert_eq!(joined, original);
    }

    // -----------------------------------------------------------------------
    // Tag key mapping tests
    // -----------------------------------------------------------------------

    #[test]
    fn mm_key_to_item_key_all_known_keys() {
        // Verify every standard MeedyaManager key maps to the correct ItemKey
        assert_eq!(mm_key_to_item_key(TAG_TITLE), Some(ItemKey::TrackTitle));
        assert_eq!(mm_key_to_item_key(TAG_ARTIST), Some(ItemKey::TrackArtist));
        assert_eq!(mm_key_to_item_key(TAG_ALBUM), Some(ItemKey::AlbumTitle));
        assert_eq!(
            mm_key_to_item_key(TAG_ALBUM_ARTIST),
            Some(ItemKey::AlbumArtist)
        );
        assert_eq!(mm_key_to_item_key(TAG_YEAR), Some(ItemKey::Year));
        assert_eq!(mm_key_to_item_key(TAG_GENRE), Some(ItemKey::Genre));
        assert_eq!(
            mm_key_to_item_key(TAG_TRACK_NUMBER),
            Some(ItemKey::TrackNumber)
        );
        assert_eq!(
            mm_key_to_item_key(TAG_TRACK_TOTAL),
            Some(ItemKey::TrackTotal)
        );
        assert_eq!(
            mm_key_to_item_key(TAG_DISC_NUMBER),
            Some(ItemKey::DiscNumber)
        );
        assert_eq!(mm_key_to_item_key(TAG_DISC_TOTAL), Some(ItemKey::DiscTotal));
        assert_eq!(mm_key_to_item_key(TAG_COMPOSER), Some(ItemKey::Composer));
        assert_eq!(mm_key_to_item_key(TAG_COMMENT), Some(ItemKey::Comment));
        assert_eq!(mm_key_to_item_key(TAG_LYRICS), Some(ItemKey::Lyrics));
        assert_eq!(mm_key_to_item_key(TAG_ISRC), Some(ItemKey::Isrc));
        assert_eq!(mm_key_to_item_key(TAG_BARCODE), Some(ItemKey::Barcode));
        assert_eq!(
            mm_key_to_item_key(TAG_CATALOG_NUMBER),
            Some(ItemKey::CatalogNumber)
        );
        assert_eq!(mm_key_to_item_key(TAG_LABEL), Some(ItemKey::Label));
        assert_eq!(
            mm_key_to_item_key(TAG_COMPILATION),
            Some(ItemKey::FlagCompilation)
        );
        assert_eq!(mm_key_to_item_key(TAG_BPM), Some(ItemKey::Bpm));
    }

    #[test]
    fn mm_key_to_item_key_unknown_returns_none() {
        // Unknown keys should return None
        assert_eq!(mm_key_to_item_key("nonexistent_tag"), None);
        assert_eq!(mm_key_to_item_key(""), None);
    }

    #[test]
    fn mm_key_to_item_key_case_sensitive() {
        // Our keys are strictly lowercase; "TITLE" must not match
        assert_eq!(mm_key_to_item_key("TITLE"), None);
        assert_eq!(mm_key_to_item_key("Title"), None);
        assert_eq!(mm_key_to_item_key("ARTIST"), None);
    }

    #[test]
    fn item_key_to_mm_key_known_keys() {
        // Reverse lookup: lofty ItemKey -> our string key
        assert_eq!(item_key_to_mm_key(&ItemKey::TrackTitle), Some(TAG_TITLE));
        assert_eq!(item_key_to_mm_key(&ItemKey::TrackArtist), Some(TAG_ARTIST));
        assert_eq!(item_key_to_mm_key(&ItemKey::AlbumTitle), Some(TAG_ALBUM));
        assert_eq!(
            item_key_to_mm_key(&ItemKey::AlbumArtist),
            Some(TAG_ALBUM_ARTIST)
        );
        assert_eq!(item_key_to_mm_key(&ItemKey::Year), Some(TAG_YEAR));
        assert_eq!(
            item_key_to_mm_key(&ItemKey::FlagCompilation),
            Some(TAG_COMPILATION)
        );
        assert_eq!(item_key_to_mm_key(&ItemKey::Bpm), Some(TAG_BPM));
    }

    #[test]
    fn item_key_to_mm_key_unknown_returns_none() {
        // An ItemKey we don't map should return None
        assert_eq!(item_key_to_mm_key(&ItemKey::EncoderSoftware), None);
    }

    #[test]
    fn tag_key_mappings_no_duplicate_mm_keys() {
        // Ensure no duplicate MeedyaManager keys in the mapping table
        let mappings = tag_key_mappings();
        let mut seen = std::collections::HashSet::new();
        for (mm_key, _) in &mappings {
            assert!(
                seen.insert(*mm_key),
                "Duplicate MeedyaManager key in tag mapping: {mm_key}",
            );
        }
    }

    #[test]
    fn tag_key_mappings_no_duplicate_item_keys() {
        // Ensure no duplicate lofty ItemKeys in the mapping table
        let mappings = tag_key_mappings();
        let mut seen = std::collections::HashSet::new();
        for (_, ik) in &mappings {
            let key_str = format!("{ik:?}");
            assert!(
                seen.insert(key_str.clone()),
                "Duplicate ItemKey in tag mapping: {key_str}",
            );
        }
    }

    #[test]
    fn tag_key_mappings_has_expected_count() {
        // Verify the mapping count is reasonable (extended beyond 19 in v1.1+)
        let count = tag_key_mappings().len();
        assert!(
            count >= 40,
            "Expected at least 40 tag mappings, got {count}"
        );
    }

    #[test]
    fn tag_key_roundtrip_mm_to_lofty_and_back() {
        // For every mapping, mm -> lofty -> mm should give the original key
        for (mm_key, item_key) in tag_key_mappings() {
            let resolved = item_key_to_mm_key(&item_key);
            assert_eq!(
                resolved,
                Some(mm_key),
                "Round-trip failed for key: {mm_key}",
            );
        }
    }

    // -----------------------------------------------------------------------
    // AudioProperties struct tests
    // -----------------------------------------------------------------------

    #[test]
    fn audio_properties_full_construction() {
        // Verify AudioProperties can be constructed with all fields populated
        let props = AudioProperties {
            duration_secs: 245.5,
            bitrate_kbps: Some(320),
            sample_rate_hz: Some(44100),
            channels: Some(2),
            bits_per_sample: Some(16),
        };
        assert!((props.duration_secs - 245.5).abs() < f64::EPSILON);
        assert_eq!(props.bitrate_kbps, Some(320));
        assert_eq!(props.sample_rate_hz, Some(44100));
        assert_eq!(props.channels, Some(2));
        assert_eq!(props.bits_per_sample, Some(16));
    }

    #[test]
    fn audio_properties_optional_fields_none() {
        // Lossy codecs may have None for bit depth and other optional fields
        let props = AudioProperties {
            duration_secs: 180.0,
            bitrate_kbps: None,
            sample_rate_hz: None,
            channels: None,
            bits_per_sample: None,
        };
        assert_eq!(props.bitrate_kbps, None);
        assert_eq!(props.sample_rate_hz, None);
        assert_eq!(props.channels, None);
        assert_eq!(props.bits_per_sample, None);
    }

    #[test]
    fn audio_properties_clone_and_eq() {
        // Verify Clone and PartialEq derive implementations
        let props = AudioProperties {
            duration_secs: 100.0,
            bitrate_kbps: Some(256),
            sample_rate_hz: Some(96000),
            channels: Some(1),
            bits_per_sample: Some(24),
        };
        let cloned = props.clone();
        assert_eq!(props, cloned);
    }

    #[test]
    fn audio_properties_inequality() {
        // Two AudioProperties with different values should not be equal
        let a = AudioProperties {
            duration_secs: 100.0,
            bitrate_kbps: Some(256),
            sample_rate_hz: Some(44100),
            channels: Some(2),
            bits_per_sample: Some(16),
        };
        let b = AudioProperties {
            duration_secs: 200.0, // different
            bitrate_kbps: Some(256),
            sample_rate_hz: Some(44100),
            channels: Some(2),
            bits_per_sample: Some(16),
        };
        assert_ne!(a, b);
    }

    #[test]
    fn audio_properties_debug_format() {
        // Verify Debug formatting produces useful output
        let props = AudioProperties {
            duration_secs: 60.0,
            bitrate_kbps: None,
            sample_rate_hz: None,
            channels: None,
            bits_per_sample: None,
        };
        let debug = format!("{props:?}");
        assert!(debug.contains("duration_secs"));
        assert!(debug.contains("60.0"));
    }

    #[test]
    fn audio_properties_serde_roundtrip() {
        // Verify JSON serialization / deserialization round-trips cleanly
        let props = AudioProperties {
            duration_secs: 312.75,
            bitrate_kbps: Some(320),
            sample_rate_hz: Some(44100),
            channels: Some(2),
            bits_per_sample: Some(16),
        };
        let json = serde_json::to_string(&props).expect("serialize");
        let deserialized: AudioProperties = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(props, deserialized);
    }

    #[test]
    fn audio_properties_serde_with_nulls() {
        // Verify None fields serialize as JSON null and deserialize back
        let props = AudioProperties {
            duration_secs: 0.0,
            bitrate_kbps: None,
            sample_rate_hz: None,
            channels: None,
            bits_per_sample: None,
        };
        let json = serde_json::to_string(&props).expect("serialize");
        assert!(json.contains("null"));
        let deserialized: AudioProperties = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(props, deserialized);
    }

    // -----------------------------------------------------------------------
    // CoverArt struct tests
    // -----------------------------------------------------------------------

    #[test]
    fn cover_art_construction() {
        // Verify CoverArt holds data and MIME type
        let art = CoverArt {
            data: vec![0xFF, 0xD8, 0xFF, 0xE0], // JPEG magic bytes (partial)
            mime: "image/jpeg".to_string(),
        };
        assert_eq!(art.data.len(), 4);
        assert_eq!(art.mime, "image/jpeg");
    }

    #[test]
    fn cover_art_clone_and_eq() {
        // Verify Clone and PartialEq
        let art = CoverArt {
            data: vec![0x89, 0x50, 0x4E, 0x47], // PNG magic bytes (partial)
            mime: "image/png".to_string(),
        };
        let cloned = art.clone();
        assert_eq!(art, cloned);
    }

    #[test]
    fn cover_art_serde_roundtrip() {
        // Verify JSON round-trip for CoverArt
        let art = CoverArt {
            data: vec![1, 2, 3, 4, 5],
            mime: "image/jpeg".to_string(),
        };
        let json = serde_json::to_string(&art).expect("serialize");
        let deserialized: CoverArt = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(art, deserialized);
    }

    #[test]
    fn cover_art_empty_data() {
        // An empty CoverArt should still be constructable (edge case)
        let art = CoverArt {
            data: vec![],
            mime: "image/jpeg".to_string(),
        };
        assert!(art.data.is_empty());
    }

    // -----------------------------------------------------------------------
    // MIME type conversion tests
    // -----------------------------------------------------------------------

    #[test]
    fn mime_type_to_string_known_types() {
        // All known lofty MimeType variants should map to correct strings
        assert_eq!(mime_type_to_string(Some(&MimeType::Jpeg)), "image/jpeg");
        assert_eq!(mime_type_to_string(Some(&MimeType::Png)), "image/png");
        assert_eq!(mime_type_to_string(Some(&MimeType::Bmp)), "image/bmp");
        assert_eq!(mime_type_to_string(Some(&MimeType::Gif)), "image/gif");
        assert_eq!(mime_type_to_string(Some(&MimeType::Tiff)), "image/tiff");
    }

    #[test]
    fn mime_type_to_string_unknown_fallback() {
        // Unknown MIME types fall back to application/octet-stream
        let unknown = MimeType::Unknown("image/webp".to_string());
        assert_eq!(
            mime_type_to_string(Some(&unknown)),
            "application/octet-stream"
        );
    }

    #[test]
    fn string_to_mime_type_known_types() {
        // All supported MIME strings should parse correctly
        assert!(matches!(string_to_mime_type("image/jpeg"), MimeType::Jpeg));
        assert!(matches!(string_to_mime_type("image/jpg"), MimeType::Jpeg));
        assert!(matches!(string_to_mime_type("image/png"), MimeType::Png));
        assert!(matches!(string_to_mime_type("image/bmp"), MimeType::Bmp));
        assert!(matches!(string_to_mime_type("image/gif"), MimeType::Gif));
        assert!(matches!(string_to_mime_type("image/tiff"), MimeType::Tiff));
    }

    #[test]
    fn string_to_mime_type_case_insensitive() {
        // MIME parsing should be case-insensitive
        assert!(matches!(string_to_mime_type("IMAGE/JPEG"), MimeType::Jpeg));
        assert!(matches!(string_to_mime_type("Image/Png"), MimeType::Png));
        assert!(matches!(string_to_mime_type("IMAGE/BMP"), MimeType::Bmp));
    }

    #[test]
    fn string_to_mime_type_unknown_produces_unknown() {
        // Unknown strings produce MimeType::Unknown with the original string
        let result = string_to_mime_type("image/webp");
        match result {
            MimeType::Unknown(s) => assert_eq!(s, "image/webp"),
            other => panic!("Expected MimeType::Unknown, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Tag constant value tests
    // -----------------------------------------------------------------------

    #[test]
    fn tag_constants_are_all_lowercase() {
        // All our tag key constants should be strictly lowercase
        let keys = [
            TAG_TITLE,
            TAG_ARTIST,
            TAG_ALBUM,
            TAG_ALBUM_ARTIST,
            TAG_YEAR,
            TAG_GENRE,
            TAG_TRACK_NUMBER,
            TAG_TRACK_TOTAL,
            TAG_DISC_NUMBER,
            TAG_DISC_TOTAL,
            TAG_COMPOSER,
            TAG_COMMENT,
            TAG_LYRICS,
            TAG_ISRC,
            TAG_BARCODE,
            TAG_CATALOG_NUMBER,
            TAG_LABEL,
            TAG_COMPILATION,
            TAG_BPM,
        ];
        for key in &keys {
            assert_eq!(*key, key.to_lowercase(), "Key should be lowercase: {key}");
        }
    }

    #[test]
    fn tag_constants_contain_no_whitespace() {
        // Tag key constants must not contain spaces or other whitespace
        let keys = [
            TAG_TITLE,
            TAG_ARTIST,
            TAG_ALBUM,
            TAG_ALBUM_ARTIST,
            TAG_YEAR,
            TAG_GENRE,
            TAG_TRACK_NUMBER,
            TAG_TRACK_TOTAL,
            TAG_DISC_NUMBER,
            TAG_DISC_TOTAL,
            TAG_COMPOSER,
            TAG_COMMENT,
            TAG_LYRICS,
            TAG_ISRC,
            TAG_BARCODE,
            TAG_CATALOG_NUMBER,
            TAG_LABEL,
            TAG_COMPILATION,
            TAG_BPM,
        ];
        for key in &keys {
            assert!(
                !key.contains(char::is_whitespace),
                "Key must not contain whitespace: {key}",
            );
        }
    }

    #[test]
    fn tag_constants_are_non_empty() {
        // No tag key constant should be an empty string
        let keys = [
            TAG_TITLE,
            TAG_ARTIST,
            TAG_ALBUM,
            TAG_ALBUM_ARTIST,
            TAG_YEAR,
            TAG_GENRE,
            TAG_TRACK_NUMBER,
            TAG_TRACK_TOTAL,
            TAG_DISC_NUMBER,
            TAG_DISC_TOTAL,
            TAG_COMPOSER,
            TAG_COMMENT,
            TAG_LYRICS,
            TAG_ISRC,
            TAG_BARCODE,
            TAG_CATALOG_NUMBER,
            TAG_LABEL,
            TAG_COMPILATION,
            TAG_BPM,
        ];
        for key in &keys {
            assert!(!key.is_empty(), "Tag key constant must not be empty");
        }
    }

    // -----------------------------------------------------------------------
    // read_tag_into_map unit tests (isolated from file I/O)
    // -----------------------------------------------------------------------

    #[test]
    fn read_tag_into_map_empty_tag() {
        // An empty tag should produce an empty map
        let tag = Tag::new(TagType::Id3v2);
        let mut map = TagMap::new();
        read_tag_into_map(&tag, &mut map);
        assert!(map.is_empty());
    }

    #[test]
    fn read_tag_into_map_with_values() {
        // Build a tag with some items and verify they appear in the map
        let mut tag = Tag::new(TagType::Id3v2);
        tag.push(TagItem::new(
            ItemKey::TrackTitle,
            ItemValue::Text("Test Song".to_string()),
        ));
        tag.push(TagItem::new(
            ItemKey::TrackArtist,
            ItemValue::Text("Test Artist".to_string()),
        ));
        tag.push(TagItem::new(
            ItemKey::Genre,
            ItemValue::Text("Rock".to_string()),
        ));

        let mut map = TagMap::new();
        read_tag_into_map(&tag, &mut map);

        assert_eq!(map.get(TAG_TITLE), Some(&vec!["Test Song".to_string()]));
        assert_eq!(map.get(TAG_ARTIST), Some(&vec!["Test Artist".to_string()]));
        assert_eq!(map.get(TAG_GENRE), Some(&vec!["Rock".to_string()]));
    }

    #[test]
    fn read_tag_into_map_deduplicates_identical_values() {
        // Duplicate values for the same key should be deduplicated
        let mut tag = Tag::new(TagType::Id3v2);
        tag.push(TagItem::new(
            ItemKey::TrackArtist,
            ItemValue::Text("Artist A".to_string()),
        ));
        tag.push(TagItem::new(
            ItemKey::TrackArtist,
            ItemValue::Text("Artist A".to_string()),
        ));

        let mut map = TagMap::new();
        read_tag_into_map(&tag, &mut map);

        // Should have only one entry, not two
        assert_eq!(map.get(TAG_ARTIST), Some(&vec!["Artist A".to_string()]),);
    }

    #[test]
    fn read_tag_into_map_preserves_distinct_multi_values() {
        // Multiple distinct values for the same key should all appear
        let mut tag = Tag::new(TagType::Id3v2);
        tag.push(TagItem::new(
            ItemKey::TrackArtist,
            ItemValue::Text("Artist A".to_string()),
        ));
        tag.push(TagItem::new(
            ItemKey::TrackArtist,
            ItemValue::Text("Artist B".to_string()),
        ));

        let mut map = TagMap::new();
        read_tag_into_map(&tag, &mut map);

        let artists = map.get(TAG_ARTIST).expect("artist key should exist");
        assert_eq!(artists.len(), 2);
        assert!(artists.contains(&"Artist A".to_string()));
        assert!(artists.contains(&"Artist B".to_string()));
    }

    #[test]
    fn read_tag_into_map_ignores_empty_and_whitespace_values() {
        // Empty or whitespace-only values should be discarded
        let mut tag = Tag::new(TagType::Id3v2);
        tag.push(TagItem::new(
            ItemKey::TrackTitle,
            ItemValue::Text(String::new()),
        ));
        tag.push(TagItem::new(
            ItemKey::TrackArtist,
            ItemValue::Text("   ".to_string()),
        ));

        let mut map = TagMap::new();
        read_tag_into_map(&tag, &mut map);

        // Neither key should appear
        assert!(!map.contains_key(TAG_TITLE));
        assert!(!map.contains_key(TAG_ARTIST));
    }

    #[test]
    fn read_tag_into_map_ignores_unmapped_keys() {
        // lofty ItemKeys not in our mapping should not appear in the map
        let mut tag = Tag::new(TagType::Id3v2);
        tag.push(TagItem::new(
            ItemKey::EncoderSoftware,
            ItemValue::Text("LAME".to_string()),
        ));

        let mut map = TagMap::new();
        read_tag_into_map(&tag, &mut map);

        assert!(map.is_empty());
    }

    #[test]
    fn read_tag_into_map_trims_whitespace_from_values() {
        // Values with leading/trailing whitespace should be trimmed
        let mut tag = Tag::new(TagType::Id3v2);
        tag.push(TagItem::new(
            ItemKey::TrackTitle,
            ItemValue::Text("  My Song  ".to_string()),
        ));

        let mut map = TagMap::new();
        read_tag_into_map(&tag, &mut map);

        assert_eq!(map.get(TAG_TITLE), Some(&vec!["My Song".to_string()]));
    }

    #[test]
    fn read_tag_into_map_merges_across_calls() {
        // Calling read_tag_into_map twice (simulating two tag containers)
        // should merge and deduplicate values
        let mut tag1 = Tag::new(TagType::Id3v2);
        tag1.push(TagItem::new(
            ItemKey::TrackArtist,
            ItemValue::Text("Artist A".to_string()),
        ));

        let mut tag2 = Tag::new(TagType::Id3v2);
        tag2.push(TagItem::new(
            ItemKey::TrackArtist,
            ItemValue::Text("Artist B".to_string()),
        ));

        let mut map = TagMap::new();
        read_tag_into_map(&tag1, &mut map);
        read_tag_into_map(&tag2, &mut map);

        let artists = map.get(TAG_ARTIST).expect("artist key should exist");
        assert_eq!(artists.len(), 2);
        assert!(artists.contains(&"Artist A".to_string()));
        assert!(artists.contains(&"Artist B".to_string()));
    }

    #[test]
    fn read_tag_into_map_merge_deduplicates_across_calls() {
        // Same value in two tags should appear only once
        let mut tag1 = Tag::new(TagType::Id3v2);
        tag1.push(TagItem::new(
            ItemKey::TrackTitle,
            ItemValue::Text("Same Title".to_string()),
        ));

        let mut tag2 = Tag::new(TagType::Id3v2);
        tag2.push(TagItem::new(
            ItemKey::TrackTitle,
            ItemValue::Text("Same Title".to_string()),
        ));

        let mut map = TagMap::new();
        read_tag_into_map(&tag1, &mut map);
        read_tag_into_map(&tag2, &mut map);

        assert_eq!(map.get(TAG_TITLE), Some(&vec!["Same Title".to_string()]),);
    }

    // -----------------------------------------------------------------------
    // Upstream CommonTag <-> MM string-key bridge tests
    // -----------------------------------------------------------------------

    #[test]
    fn mm_key_to_common_tag_known_keys() {
        // Each of the 22 MM keys with an upstream CommonTag equivalent
        assert_eq!(mm_key_to_common_tag(TAG_TITLE), Some(CommonTag::Title));
        assert_eq!(mm_key_to_common_tag(TAG_ARTIST), Some(CommonTag::Artist));
        assert_eq!(mm_key_to_common_tag(TAG_ALBUM), Some(CommonTag::Album));
        assert_eq!(
            mm_key_to_common_tag(TAG_ALBUM_ARTIST),
            Some(CommonTag::AlbumArtist)
        );
        assert_eq!(mm_key_to_common_tag(TAG_ISRC), Some(CommonTag::Isrc));
        assert_eq!(mm_key_to_common_tag(TAG_BARCODE), Some(CommonTag::Upc));
        assert_eq!(
            mm_key_to_common_tag(TAG_REPLAYGAIN_TRACK_GAIN),
            Some(CommonTag::ReplayGainTrackGain)
        );
    }

    #[test]
    fn mm_key_to_common_tag_mm_only_keys_return_none() {
        // MM-only keys (no upstream equivalent) should return None.
        // This documents which fields are "lost" if a caller goes through
        // the upstream surface.
        assert_eq!(mm_key_to_common_tag(TAG_TITLE_SORT), None);
        assert_eq!(mm_key_to_common_tag(TAG_ARTIST_SORT), None);
        assert_eq!(mm_key_to_common_tag(TAG_ALBUM_SORT), None);
        assert_eq!(mm_key_to_common_tag(TAG_CONDUCTOR), None);
        assert_eq!(mm_key_to_common_tag(TAG_REMIXER), None);
        assert_eq!(mm_key_to_common_tag(TAG_LYRICIST), None);
        assert_eq!(mm_key_to_common_tag(TAG_LANGUAGE), None);
        assert_eq!(mm_key_to_common_tag(TAG_MOOD), None);
        assert_eq!(mm_key_to_common_tag(TAG_GROUPING), None);
        assert_eq!(mm_key_to_common_tag(TAG_WORK), None);
        assert_eq!(mm_key_to_common_tag(TAG_MOVEMENT), None);
        assert_eq!(mm_key_to_common_tag(TAG_MOVEMENT_INDEX), None);
        assert_eq!(mm_key_to_common_tag(TAG_MOVEMENT_TOTAL), None);
        assert_eq!(mm_key_to_common_tag(TAG_CATALOG_NUMBER), None);
        assert_eq!(mm_key_to_common_tag(TAG_BPM), None);
        assert_eq!(mm_key_to_common_tag(TAG_ORIGINAL_YEAR), None);
        assert_eq!(mm_key_to_common_tag(TAG_ORIGINAL_ALBUM), None);
        assert_eq!(mm_key_to_common_tag(TAG_ORIGINAL_ARTIST), None);
        assert_eq!(mm_key_to_common_tag(TAG_PODCAST_TITLE), None);
        assert_eq!(mm_key_to_common_tag(TAG_PODCAST_ID), None);
        assert_eq!(mm_key_to_common_tag(TAG_PODCAST_URL), None);
        assert_eq!(mm_key_to_common_tag(TAG_PODCAST_CATEGORY), None);
        assert_eq!(mm_key_to_common_tag(TAG_PODCAST_DESCRIPTION), None);
        assert_eq!(mm_key_to_common_tag(TAG_ENCODER_SETTINGS), None);
    }

    #[test]
    fn mm_key_to_common_tag_unknown_returns_none() {
        // Garbage input
        assert_eq!(mm_key_to_common_tag("nonsense"), None);
        assert_eq!(mm_key_to_common_tag(""), None);
    }

    #[test]
    fn common_tag_to_mm_key_roundtrip() {
        // For every CommonTag variant, common_tag_to_mm_key is total.
        // We additionally verify that the returned key, if it's a MM TAG_*
        // constant, round-trips back through mm_key_to_common_tag.
        let variants = [
            CommonTag::Title,
            CommonTag::Artist,
            CommonTag::Album,
            CommonTag::AlbumArtist,
            CommonTag::Genre,
            CommonTag::Year,
            CommonTag::TrackNumber,
            CommonTag::TotalTracks,
            CommonTag::DiscNumber,
            CommonTag::TotalDiscs,
            CommonTag::Composer,
            CommonTag::Comment,
            CommonTag::Lyrics,
            CommonTag::Isrc,
            CommonTag::Upc,
            CommonTag::Label,
            CommonTag::Compilation,
            CommonTag::Encoder,
            CommonTag::ReplayGainTrackGain,
            CommonTag::ReplayGainTrackPeak,
            CommonTag::ReplayGainAlbumGain,
            CommonTag::ReplayGainAlbumPeak,
        ];
        for v in variants {
            let mm_key = common_tag_to_mm_key(v);
            assert!(!mm_key.is_empty(), "MM key for {v:?} must be non-empty");
            assert_eq!(
                mm_key_to_common_tag(mm_key),
                Some(v),
                "CommonTag::{v:?} -> mm_key -> CommonTag round-trip failed",
            );
        }
    }

    #[test]
    fn common_tag_to_mm_key_extended_variants_are_total() {
        // The variants that don't map to a TAG_* constant still produce
        // a well-defined non-empty string.
        assert!(!common_tag_to_mm_key(CommonTag::Copyright).is_empty());
        assert!(!common_tag_to_mm_key(CommonTag::MusicBrainzRecordingId).is_empty());
        assert!(!common_tag_to_mm_key(CommonTag::MusicBrainzReleaseId).is_empty());
        assert!(!common_tag_to_mm_key(CommonTag::AcoustId).is_empty());
        assert!(!common_tag_to_mm_key(CommonTag::ReleaseDate).is_empty());
        assert!(!common_tag_to_mm_key(CommonTag::Description).is_empty());
        assert!(!common_tag_to_mm_key(CommonTag::ReplayGainReferenceLoudness).is_empty());
    }

    #[test]
    fn upstream_reexports_resolve() {
        // Sanity: the re-exported upstream symbols actually resolve to types.
        // STANDARD_NAMESPACES is a slice constant — verify it includes the
        // canonical entries.
        let names: Vec<&str> = STANDARD_NAMESPACES.iter().map(|(k, _)| *k).collect();
        assert!(names.contains(&"itunes"));
        assert!(names.contains(&"meedya"));
    }

    #[test]
    fn upstream_tag_io_module_resolves() {
        // Touching the type forces the re-export to be exercised at compile
        // time.  TagMap is HashMap<CommonTag, Vec<String>>.
        let m: upstream::TagMap = upstream::TagMap::new();
        assert!(m.is_empty());
    }

    // -----------------------------------------------------------------------
    // Strict tag-key validation — issue #206
    //
    // A key with no `ItemKey` mapping used to be dropped on the floor by
    // `write_tags` (and turned into a silent `Ok(())` by `remove_tag`), so a
    // typo in a CLI `--set` or an FFI caller's key reported success having
    // changed nothing.  Both entry points must now refuse the write.
    // -----------------------------------------------------------------------

    /// Build a minimal but *real* WAV file on disk.
    ///
    /// lofty refuses a bare 44-byte header with no `data` payload, so the
    /// fixture carries 0.1 s of 8 kHz 16-bit mono silence (1,600 bytes of
    /// samples, 1,644 bytes total).  That is the smallest file lofty will
    /// both parse and write RIFF INFO tags back into.
    fn write_wav_fixture(path: &std::path::Path) {
        // 0.1 s x 8,000 frames/s x 2 bytes/frame = 1,600 bytes of samples.
        const DATA_LEN: u32 = 1600;

        let mut bytes: Vec<u8> = Vec::with_capacity(44 + DATA_LEN as usize);
        bytes.extend_from_slice(b"RIFF"); // RIFF container magic
        bytes.extend_from_slice(&(36 + DATA_LEN).to_le_bytes()); // size after this field
        bytes.extend_from_slice(b"WAVE"); // RIFF form type
        bytes.extend_from_slice(b"fmt "); // format chunk id (note trailing space)
        bytes.extend_from_slice(&16u32.to_le_bytes()); // PCM format chunk is 16 bytes
        bytes.extend_from_slice(&1u16.to_le_bytes()); // audio format: 1 = PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // channels: mono
        bytes.extend_from_slice(&8000u32.to_le_bytes()); // sample rate: 8 kHz
        bytes.extend_from_slice(&16000u32.to_le_bytes()); // byte rate = rate x align
        bytes.extend_from_slice(&2u16.to_le_bytes()); // block align: 1ch x 16-bit
        bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        bytes.extend_from_slice(b"data"); // sample data chunk id
        bytes.extend_from_slice(&DATA_LEN.to_le_bytes()); // sample data length
        bytes.extend_from_slice(&vec![0u8; DATA_LEN as usize]); // silence

        std::fs::write(path, &bytes).expect("WAV fixture must be writable");
    }

    #[test]
    fn write_tags_rejects_unknown_key() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);

        // `bogus_key` has no ItemKey mapping, so there is no way to persist it.
        let mut tags = TagMap::new();
        tags.insert("bogus_key".to_string(), vec!["1".to_string()]);

        let err = write_tags(&p, &tags)
            .expect_err("an unmapped tag key must be rejected, never silently dropped");
        let msg = err.to_string();
        assert!(
            msg.contains("bogus_key"),
            "the error must name the offending key, got: {msg}"
        );
        assert!(
            msg.contains(TAG_TITLE),
            "the error must list the valid keys, got: {msg}"
        );
    }

    /// Review item 12 of issue #251's independent review: `mm-ffi`'s
    /// `write_metadata_rejects_gibberish_language` already proves this
    /// through the FFI boundary the native UIs actually call, but that
    /// exercises a whole extra layer (`MmFfiError`, the FFI's own argument
    /// shape) on top of what actually does the refusing. This test calls
    /// `write_tags` directly — the one place in this crate that decides
    /// whether a `language` value is refused at all — so a future change
    /// that broke the refusal but happened to leave the FFI wrapper's own
    /// error mapping looking correct would still be caught here.
    #[test]
    fn write_tags_rejects_gibberish_language() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);
        let before = std::fs::read(&p).unwrap();

        let mut tags = TagMap::new();
        tags.insert(TAG_LANGUAGE.to_string(), vec!["not a language".to_string()]);

        let err = write_tags(&p, &tags)
            .expect_err("a language nothing recognises must not report success");
        let msg = err.to_string();
        assert!(
            msg.contains("not a language"),
            "the error must name the rejected value, got: {msg}"
        );
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "a rejected write must not touch the file — `write_tags` validates the language \
             value before it ever calls `tag.save_to_path`, so nothing on disk should change"
        );
    }

    #[test]
    fn remove_tag_rejects_unknown_key() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);

        let err = remove_tag(&p, "bogus_key")
            .expect_err("removing an unmapped tag key must be an error, not a silent no-op");
        let msg = err.to_string();
        assert!(
            msg.contains("bogus_key"),
            "the error must name the offending key, got: {msg}"
        );
        assert!(
            msg.contains(TAG_TITLE),
            "the error must list the valid keys, got: {msg}"
        );
    }
}
