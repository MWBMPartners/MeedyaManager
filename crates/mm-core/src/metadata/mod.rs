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
// License: GPL-2.0-or-later

/// Tag definition registry — loads from config/tags.json5 at startup.
/// Provides known tag lists for template validation and UI pickers.
pub mod tag_registry;

use std::collections::HashMap;
use std::path::Path;

// lofty 0.22 re-exports — organised by submodule
use lofty::config::WriteOptions; // write-time options (padding, etc.)
use lofty::file::{AudioFile, TaggedFileExt}; // traits: properties(), tags(), etc.
use lofty::picture::{MimeType, Picture, PictureType}; // embedded artwork types
use lofty::probe::Probe; // file format auto-detection
use lofty::tag::{ItemKey, ItemValue, Tag, TagExt, TagItem, TagType};

use serde::{Deserialize, Serialize};

use crate::error::{MmError, MmResult};

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
/// Language of the lyrics (ISO 639-1, e.g. "en", "fr", "ja")
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

    Ok(tag_map)
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
/// # Errors
/// Returns an error if the file cannot be opened, read, or saved.
pub fn write_tags(path: &Path, tags: &TagMap) -> MmResult<()> {
    // Open and read the existing file so we can preserve its tags
    let mut tagged_file = open_tagged_file(path)?;

    // Get (or create) the primary tag for this file format
    let tag = get_or_create_primary_tag(&mut tagged_file);

    // Write each entry from the supplied map into the tag
    for (key, values) in tags {
        // Look up the lofty ItemKey for this MeedyaManager key
        if let Some(item_key) = mm_key_to_item_key(key) {
            // Join multi-value into a single "; "-delimited string
            let joined = join_multi_value(values);

            // Remove existing items for this key to avoid duplicates
            tag.remove_key(&item_key);

            // Insert the new value (skip if the joined string is empty)
            if !joined.is_empty() {
                let item = TagItem::new(item_key, ItemValue::Text(joined));
                tag.push(item);
            }
        }
        // Keys not in our mapping are silently ignored — callers should
        // use the TAG_* constants for reliable round-tripping.
    }

    // Persist to disk using default write options (preserves format quirks)
    tag.save_to_path(path, WriteOptions::default())?;

    Ok(())
}

/// Remove a specific tag field from the file at `path`.
///
/// The `key` should be one of the `TAG_*` constants.  If the key is not
/// recognised or the file has no such tag, this is a no-op (not an error).
///
/// # Errors
/// Returns an error if the file cannot be opened, read, or saved.
pub fn remove_tag(path: &Path, key: &str) -> MmResult<()> {
    // Only proceed if the key maps to a known ItemKey
    let item_key = match mm_key_to_item_key(key) {
        Some(ik) => ik,
        None => return Ok(()), // unknown key -> silent no-op
    };

    // Open and read the existing file
    let mut tagged_file = open_tagged_file(path)?;

    // Collect the tag types present in this file so we can iterate mutably
    let tag_types: Vec<TagType> = tagged_file
        .tags()
        .iter()
        .map(lofty::tag::Tag::tag_type)
        .collect();

    // Remove the key from each tag container, then save
    for tt in &tag_types {
        if let Some(tag) = tagged_file.tag_mut(*tt) {
            // Remove all items matching this key
            tag.remove_key(&item_key);
            // Save this tag back to disk
            tag.save_to_path(path, WriteOptions::default())?;
        }
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

    // Collect all tag types present so we can iterate mutably
    let tag_types: Vec<TagType> = tagged_file
        .tags()
        .iter()
        .map(lofty::tag::Tag::tag_type)
        .collect();

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
}
