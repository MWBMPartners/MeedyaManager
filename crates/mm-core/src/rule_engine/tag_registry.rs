// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rule Engine Tag Registry
//
// Bidirectional mapping between template display names (what users write
// between `< >` brackets) and internal tag identifiers.  The registry
// supports three kinds of tags:
//
//   - **Metadata tags** — read from embedded file metadata via lofty
//   - **Virtual tags**  — computed at evaluation time from file properties,
//     classification, or the filesystem
//   - **Custom tags**   — user-defined tags (Custom1–Custom16, MeedyaMeta.*)
//
// All lookups are case-insensitive (normalised to lowercase).  The registry
// is initialised once via `OnceLock` and shared across all evaluations.
//
// License: GPL-2.0-or-later

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::metadata::{
    TAG_ALBUM, TAG_ALBUM_ARTIST, TAG_ARTIST, TAG_BARCODE, TAG_BPM, TAG_CATALOG_NUMBER,
    TAG_COMMENT, TAG_COMPILATION, TAG_COMPOSER, TAG_DISC_NUMBER, TAG_DISC_TOTAL, TAG_GENRE,
    TAG_ISRC, TAG_LABEL, TAG_LYRICS, TAG_TITLE, TAG_TRACK_NUMBER, TAG_TRACK_TOTAL, TAG_YEAR,
};

// ───────────────────────────────────────────────────────────────────────────
// Tag kind enum
// ───────────────────────────────────────────────────────────────────────────

/// Describes what kind of tag a template name resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagKind {
    /// A standard metadata tag — value comes from the file's embedded tags.
    /// The inner string is the canonical mm-core key (e.g. "artist", "album").
    Metadata(&'static str),

    /// A virtual/computed tag — value computed from file path, audio properties,
    /// or media classification at evaluation time.
    Virtual(VirtualTag),

    /// A custom user-defined tag.  Matches `Custom1`–`Custom16` or any tag
    /// starting with `meedyameta.`.  The inner string is the canonical key.
    Custom(String),
}

/// Virtual tags computed at evaluation time rather than read from metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VirtualTag {
    /// Base filename without extension (e.g. "01 - Song Title")
    Filename,
    /// File extension without dot (e.g. "mp3", "flac")
    Extension,
    /// Immediate parent directory name
    Folder,
    /// Full absolute file path
    FullPath,
    /// Human-readable duration (e.g. "3:42")
    Duration,
    /// Duration in integer seconds as a string
    DurationSecs,
    /// Bitrate in kbps as a string (e.g. "320")
    BitrateKbps,
    /// Sample rate in Hz as a string (e.g. "44100")
    SampleRateHz,
    /// Channel count as a string (e.g. "2")
    Channels,
    /// Bit depth per sample as a string (e.g. "16", "24")
    BitDepth,
    /// Media class display string (e.g. "Music", "TV Show")
    MediaClass,
    /// Media group display string (e.g. "Audio", "Video")
    MediaGroup,
    /// Media format display string (e.g. "MP3", "FLAC")
    MediaFormat,
    /// Media quality tier display string (e.g. "Lossless", "320 kbps")
    MediaQuality,
}

// ───────────────────────────────────────────────────────────────────────────
// Registry initialisation
// ───────────────────────────────────────────────────────────────────────────

/// Global tag registry — initialised once, shared across all evaluations.
fn registry() -> &'static HashMap<String, TagKind> {
    static REG: OnceLock<HashMap<String, TagKind>> = OnceLock::new();
    REG.get_or_init(|| {
        let mut map = HashMap::new();

        // ── Standard metadata tags (19 from metadata module) ──────────
        // Each entry maps a lowercase display name to its canonical key.
        // Some tags have aliases (e.g. "track#" and "tracknumber").
        map.insert("title".into(), TagKind::Metadata(TAG_TITLE));
        map.insert("artist".into(), TagKind::Metadata(TAG_ARTIST));
        map.insert("album".into(), TagKind::Metadata(TAG_ALBUM));
        map.insert("album artist".into(), TagKind::Metadata(TAG_ALBUM_ARTIST));
        map.insert("albumartist".into(), TagKind::Metadata(TAG_ALBUM_ARTIST));
        map.insert("year".into(), TagKind::Metadata(TAG_YEAR));
        map.insert("date".into(), TagKind::Metadata(TAG_YEAR));
        map.insert("genre".into(), TagKind::Metadata(TAG_GENRE));
        map.insert("track#".into(), TagKind::Metadata(TAG_TRACK_NUMBER));
        map.insert("tracknumber".into(), TagKind::Metadata(TAG_TRACK_NUMBER));
        map.insert("track number".into(), TagKind::Metadata(TAG_TRACK_NUMBER));
        map.insert("track count".into(), TagKind::Metadata(TAG_TRACK_TOTAL));
        map.insert("tracktotal".into(), TagKind::Metadata(TAG_TRACK_TOTAL));
        map.insert("disc#".into(), TagKind::Metadata(TAG_DISC_NUMBER));
        map.insert("discnumber".into(), TagKind::Metadata(TAG_DISC_NUMBER));
        map.insert("disc number".into(), TagKind::Metadata(TAG_DISC_NUMBER));
        map.insert("disc count".into(), TagKind::Metadata(TAG_DISC_TOTAL));
        map.insert("disctotal".into(), TagKind::Metadata(TAG_DISC_TOTAL));
        map.insert("composer".into(), TagKind::Metadata(TAG_COMPOSER));
        map.insert("comment".into(), TagKind::Metadata(TAG_COMMENT));
        map.insert("lyrics".into(), TagKind::Metadata(TAG_LYRICS));
        map.insert("isrc".into(), TagKind::Metadata(TAG_ISRC));
        map.insert("barcode".into(), TagKind::Metadata(TAG_BARCODE));
        map.insert("catalog#".into(), TagKind::Metadata(TAG_CATALOG_NUMBER));
        map.insert("catalognumber".into(), TagKind::Metadata(TAG_CATALOG_NUMBER));
        map.insert("catalog number".into(), TagKind::Metadata(TAG_CATALOG_NUMBER));
        map.insert("label".into(), TagKind::Metadata(TAG_LABEL));
        map.insert("compilation".into(), TagKind::Metadata(TAG_COMPILATION));
        map.insert("bpm".into(), TagKind::Metadata(TAG_BPM));

        // ── Extended metadata tags (12) ───────────────────────────────
        // These map to lofty ItemKey variants or custom frame identifiers.
        // The canonical keys match common tag frame names.
        map.insert("sort title".into(), TagKind::Metadata("sort_title"));
        map.insert("sorttitle".into(), TagKind::Metadata("sort_title"));
        map.insert("sort artist".into(), TagKind::Metadata("sort_artist"));
        map.insert("sortartist".into(), TagKind::Metadata("sort_artist"));
        map.insert("sort album".into(), TagKind::Metadata("sort_album"));
        map.insert("sortalbum".into(), TagKind::Metadata("sort_album"));
        map.insert("sort album artist".into(), TagKind::Metadata("sort_album_artist"));
        map.insert("sortalbumartist".into(), TagKind::Metadata("sort_album_artist"));
        map.insert("sort composer".into(), TagKind::Metadata("sort_composer"));
        map.insert("sortcomposer".into(), TagKind::Metadata("sort_composer"));
        map.insert("grouping".into(), TagKind::Metadata("grouping"));
        map.insert("conductor".into(), TagKind::Metadata("conductor"));
        map.insert("remixer".into(), TagKind::Metadata("remixer"));
        map.insert("producer".into(), TagKind::Metadata("producer"));
        map.insert("lyricist".into(), TagKind::Metadata("lyricist"));
        map.insert("mood".into(), TagKind::Metadata("mood"));
        map.insert("initial key".into(), TagKind::Metadata("initial_key"));
        map.insert("initialkey".into(), TagKind::Metadata("initial_key"));
        map.insert("key".into(), TagKind::Metadata("initial_key"));
        map.insert("encoder".into(), TagKind::Metadata("encoder"));
        map.insert("copyright".into(), TagKind::Metadata("copyright"));
        map.insert("publisher".into(), TagKind::Metadata("publisher"));
        map.insert("language".into(), TagKind::Metadata("language"));
        map.insert("rating".into(), TagKind::Metadata("rating"));
        map.insert("subtitle".into(), TagKind::Metadata("subtitle"));

        // ── Virtual tags (15) ────────────────────────────────────────
        map.insert("filename".into(), TagKind::Virtual(VirtualTag::Filename));
        map.insert("extension".into(), TagKind::Virtual(VirtualTag::Extension));
        map.insert("folder".into(), TagKind::Virtual(VirtualTag::Folder));
        map.insert("full path".into(), TagKind::Virtual(VirtualTag::FullPath));
        map.insert("fullpath".into(), TagKind::Virtual(VirtualTag::FullPath));
        map.insert("file path".into(), TagKind::Virtual(VirtualTag::FullPath));
        map.insert("filepath".into(), TagKind::Virtual(VirtualTag::FullPath));
        map.insert("duration".into(), TagKind::Virtual(VirtualTag::Duration));
        map.insert("duration secs".into(), TagKind::Virtual(VirtualTag::DurationSecs));
        map.insert("durationsecs".into(), TagKind::Virtual(VirtualTag::DurationSecs));
        map.insert("bitrate".into(), TagKind::Virtual(VirtualTag::BitrateKbps));
        map.insert("sample rate".into(), TagKind::Virtual(VirtualTag::SampleRateHz));
        map.insert("samplerate".into(), TagKind::Virtual(VirtualTag::SampleRateHz));
        map.insert("channels".into(), TagKind::Virtual(VirtualTag::Channels));
        map.insert("bit depth".into(), TagKind::Virtual(VirtualTag::BitDepth));
        map.insert("bitdepth".into(), TagKind::Virtual(VirtualTag::BitDepth));
        map.insert("media class".into(), TagKind::Virtual(VirtualTag::MediaClass));
        map.insert("mediaclass".into(), TagKind::Virtual(VirtualTag::MediaClass));
        map.insert("media group".into(), TagKind::Virtual(VirtualTag::MediaGroup));
        map.insert("mediagroup".into(), TagKind::Virtual(VirtualTag::MediaGroup));
        map.insert("media format".into(), TagKind::Virtual(VirtualTag::MediaFormat));
        map.insert("mediaformat".into(), TagKind::Virtual(VirtualTag::MediaFormat));
        map.insert("media quality".into(), TagKind::Virtual(VirtualTag::MediaQuality));
        map.insert("mediaquality".into(), TagKind::Virtual(VirtualTag::MediaQuality));

        map
    })
}

// ───────────────────────────────────────────────────────────────────────────
// Public API
// ───────────────────────────────────────────────────────────────────────────

/// Look up a tag by its template display name (case-insensitive).
///
/// Returns `Some(TagKind)` if the name is a known standard, extended,
/// virtual, or custom tag.  Returns `None` if the name is not recognised.
///
/// Custom tags are detected by pattern:
/// - `custom1` through `custom16` → `TagKind::Custom("custom_N")`
/// - Names starting with `meedyameta.` → `TagKind::Custom("meedyameta.rest")`
pub fn lookup_tag(display_name: &str) -> Option<TagKind> {
    // Normalise to lowercase for case-insensitive lookup
    let key = display_name.to_lowercase();

    // Check the static registry first
    if let Some(kind) = registry().get(&key) {
        return Some(kind.clone());
    }

    // Check for custom tag patterns: custom1–custom16
    if let Some(rest) = key.strip_prefix("custom") {
        if let Ok(n) = rest.parse::<u32>() {
            if (1..=16).contains(&n) {
                return Some(TagKind::Custom(format!("custom_{n}")));
            }
        }
    }

    // Check for MeedyaMeta namespace: meedyameta.*
    if key.starts_with("meedyameta.") {
        return Some(TagKind::Custom(key));
    }

    // Not a known tag
    None
}

/// Return the total number of entries in the static registry
/// (excludes dynamic custom tag patterns).
pub fn registry_size() -> usize {
    registry().len()
}

/// Return all entries in the static registry as `(display_name, TagKind)` pairs.
///
/// The results are sorted alphabetically by display name for consistent output.
/// Dynamic custom tags (Custom1–16, MeedyaMeta.*) are not included — only the
/// statically registered tags and their aliases.
pub fn all_tags() -> Vec<(String, TagKind)> {
    let reg = registry();
    let mut entries: Vec<(String, TagKind)> = reg
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    // Sort alphabetically by display name for deterministic output
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

// ───────────────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Standard tag: title resolves to Metadata
    #[test]
    fn standard_tag_title() {
        let kind = lookup_tag("Title").unwrap();
        assert_eq!(kind, TagKind::Metadata(TAG_TITLE));
    }

    /// Standard tag: artist resolves to Metadata
    #[test]
    fn standard_tag_artist() {
        let kind = lookup_tag("Artist").unwrap();
        assert_eq!(kind, TagKind::Metadata(TAG_ARTIST));
    }

    /// Standard tag: album resolves to Metadata
    #[test]
    fn standard_tag_album() {
        let kind = lookup_tag("Album").unwrap();
        assert_eq!(kind, TagKind::Metadata(TAG_ALBUM));
    }

    /// Album Artist (with space) resolves correctly
    #[test]
    fn standard_tag_album_artist_spaced() {
        let kind = lookup_tag("Album Artist").unwrap();
        assert_eq!(kind, TagKind::Metadata(TAG_ALBUM_ARTIST));
    }

    /// AlbumArtist (no space) also resolves
    #[test]
    fn standard_tag_album_artist_nospace() {
        let kind = lookup_tag("AlbumArtist").unwrap();
        assert_eq!(kind, TagKind::Metadata(TAG_ALBUM_ARTIST));
    }

    /// Case-insensitive: ARTIST, artist, Artist all resolve the same
    #[test]
    fn case_insensitive_lookup() {
        let upper = lookup_tag("ARTIST").unwrap();
        let lower = lookup_tag("artist").unwrap();
        let mixed = lookup_tag("Artist").unwrap();
        assert_eq!(upper, lower);
        assert_eq!(lower, mixed);
    }

    /// Track# alias resolves to track_number
    #[test]
    fn track_number_alias() {
        let hash = lookup_tag("Track#").unwrap();
        let full = lookup_tag("TrackNumber").unwrap();
        assert_eq!(hash, full);
        assert_eq!(hash, TagKind::Metadata(TAG_TRACK_NUMBER));
    }

    /// Disc# alias resolves to disc_number
    #[test]
    fn disc_number_alias() {
        let hash = lookup_tag("Disc#").unwrap();
        let full = lookup_tag("DiscNumber").unwrap();
        assert_eq!(hash, full);
        assert_eq!(hash, TagKind::Metadata(TAG_DISC_NUMBER));
    }

    /// Year and Date both resolve to the year tag
    #[test]
    fn year_date_alias() {
        let year = lookup_tag("Year").unwrap();
        let date = lookup_tag("Date").unwrap();
        assert_eq!(year, date);
        assert_eq!(year, TagKind::Metadata(TAG_YEAR));
    }

    /// Virtual tag: Filename
    #[test]
    fn virtual_tag_filename() {
        let kind = lookup_tag("Filename").unwrap();
        assert_eq!(kind, TagKind::Virtual(VirtualTag::Filename));
    }

    /// Virtual tag: Extension
    #[test]
    fn virtual_tag_extension() {
        let kind = lookup_tag("Extension").unwrap();
        assert_eq!(kind, TagKind::Virtual(VirtualTag::Extension));
    }

    /// Virtual tag: MediaClass (with space)
    #[test]
    fn virtual_tag_media_class() {
        let spaced = lookup_tag("Media Class").unwrap();
        let joined = lookup_tag("MediaClass").unwrap();
        assert_eq!(spaced, joined);
        assert_eq!(spaced, TagKind::Virtual(VirtualTag::MediaClass));
    }

    /// Virtual tag: Duration
    #[test]
    fn virtual_tag_duration() {
        let kind = lookup_tag("Duration").unwrap();
        assert_eq!(kind, TagKind::Virtual(VirtualTag::Duration));
    }

    /// Virtual tag: Bitrate
    #[test]
    fn virtual_tag_bitrate() {
        let kind = lookup_tag("Bitrate").unwrap();
        assert_eq!(kind, TagKind::Virtual(VirtualTag::BitrateKbps));
    }

    /// Extended tag: SortArtist
    #[test]
    fn extended_tag_sort_artist() {
        let kind = lookup_tag("Sort Artist").unwrap();
        assert_eq!(kind, TagKind::Metadata("sort_artist"));
    }

    /// Extended tag: Conductor
    #[test]
    fn extended_tag_conductor() {
        let kind = lookup_tag("Conductor").unwrap();
        assert_eq!(kind, TagKind::Metadata("conductor"));
    }

    /// Extended tag: Mood
    #[test]
    fn extended_tag_mood() {
        let kind = lookup_tag("Mood").unwrap();
        assert_eq!(kind, TagKind::Metadata("mood"));
    }

    /// Custom tag: Custom1 resolves to Custom
    #[test]
    fn custom_tag_custom1() {
        let kind = lookup_tag("Custom1").unwrap();
        assert_eq!(kind, TagKind::Custom("custom_1".into()));
    }

    /// Custom tag: Custom16 resolves to Custom
    #[test]
    fn custom_tag_custom16() {
        let kind = lookup_tag("Custom16").unwrap();
        assert_eq!(kind, TagKind::Custom("custom_16".into()));
    }

    /// Custom tag: Custom17 is out of range — returns None
    #[test]
    fn custom_tag_out_of_range() {
        assert!(lookup_tag("Custom17").is_none());
    }

    /// Custom tag: Custom0 is out of range — returns None
    #[test]
    fn custom_tag_zero() {
        assert!(lookup_tag("Custom0").is_none());
    }

    /// MeedyaMeta namespace tag
    #[test]
    fn meedyameta_tag() {
        let kind = lookup_tag("MeedyaMeta.SpotifyId").unwrap();
        assert_eq!(kind, TagKind::Custom("meedyameta.spotifyid".into()));
    }

    /// Unknown tag returns None
    #[test]
    fn unknown_tag() {
        assert!(lookup_tag("NonExistentTag").is_none());
    }

    /// Registry has at least 40 entries (the 40+ mapping target)
    #[test]
    fn registry_has_40_plus_entries() {
        assert!(
            registry_size() >= 40,
            "expected 40+ registry entries, got {}",
            registry_size()
        );
    }

    /// All 15 virtual tag variants are registered
    #[test]
    fn all_virtual_tags_registered() {
        let virtual_tags = [
            "Filename", "Extension", "Folder", "FullPath", "Duration",
            "DurationSecs", "Bitrate", "SampleRate", "Channels", "BitDepth",
            "MediaClass", "MediaGroup", "MediaFormat", "MediaQuality",
        ];
        for name in virtual_tags {
            assert!(
                lookup_tag(name).is_some(),
                "virtual tag '{name}' not found in registry"
            );
        }
    }
}
