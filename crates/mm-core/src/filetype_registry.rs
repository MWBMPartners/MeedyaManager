// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — File Type Registry
//
// Single source of truth for all file types MeedyaManager recognises.
//
// Data is stored in `config/filetypes.json5` (workspace root), embedded into
// the binary at compile time via `include_str!`.  Users can override the
// compiled defaults by placing a modified copy at:
//
//   Linux/macOS:  ~/.config/meedyamanager/filetypes.json5
//   Windows:      %APPDATA%\MeedyaManager\filetypes.json5
//
// At first access the registry is lazily parsed from JSON5 and cached for the
// lifetime of the process.  If the user's override file exists but is
// malformed, a warning is logged and the compiled defaults are used instead.
//
// To add a new file type:
//   1. Edit `config/filetypes.json5`.
//   2. Run `cargo test -p mm-core -- filetype` to verify invariants.
//
// To disable a format without removing it, add `"enabled": false` to its
// JSON5 entry.

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Embedded default registry — compiled into every binary build
// ---------------------------------------------------------------------------

/// The built-in `filetypes.json5`, baked into the binary at compile time.
/// This is used when no user-override file is present.
static DEFAULT_JSON5: &str = include_str!("../../../config/filetypes.json5");

// ---------------------------------------------------------------------------
// Sub-type enums
// ---------------------------------------------------------------------------

/// Scope of a companion file — how broadly it applies within a library.
///
/// Used to decide whether a companion file moves with a single track,
/// with the whole album folder, or with all albums for an artist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompanionScope {
    /// Relates to a single media track (e.g. `.lrc`, `.srt`, track-named `.cue`).
    Track,
    /// Relates to the whole album / directory (e.g. `cover.jpg`, `.nfo`, `.zip`).
    Album,
    /// Relates to an artist across albums (e.g. artist photo, `.itlp` package).
    Artist,
}

/// Fine-grained kind for subtitle / caption / lyric companion files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleKind {
    /// Timed subtitle track (for video)
    Subtitle,
    /// Closed caption track
    Caption,
    /// Song lyrics (timed or plain)
    Lyrics,
    /// Spoken-word transcript
    Transcript,
}

// ---------------------------------------------------------------------------
// Registry entry types
//
// These structs hold owned Strings so they can be deserialized from JSON5.
// They are stored inside the process-global LazyLock, so any reference into
// them has an implicit 'static lifetime.
// ---------------------------------------------------------------------------

/// Returns `true`; used as `#[serde(default = "default_true")]`.
fn default_true() -> bool { true }

/// Description of a single audio file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFormat {
    /// File extension without leading dot, lower-case (e.g. `"mp3"`).
    #[serde(rename = "ext")]
    pub extension: String,

    /// IANA MIME type string (e.g. `"audio/mpeg"`).
    #[serde(rename = "mime", default)]
    pub mime_type: String,

    /// Human-readable format name (e.g. `"MP3"`).
    #[serde(rename = "name")]
    pub display_name: String,

    /// `true` for lossless formats (FLAC, ALAC, WAV, AIFF, …).
    pub lossless: bool,

    /// When `false` the format is ignored by all registry lookups.
    /// Defaults to `true` when the field is absent from JSON5.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Description of a single video file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFormat {
    /// File extension without leading dot, lower-case (e.g. `"mkv"`).
    #[serde(rename = "ext")]
    pub extension: String,

    /// IANA MIME type string (e.g. `"video/x-matroska"`).
    #[serde(rename = "mime", default)]
    pub mime_type: String,

    /// Human-readable format name (e.g. `"Matroska"`).
    #[serde(rename = "name")]
    pub display_name: String,

    /// When `false` the format is ignored by all registry lookups.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Description of a subtitle / caption / lyrics file format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleFormat {
    /// File extension without leading dot, lower-case.
    #[serde(rename = "ext")]
    pub extension: String,

    /// IANA MIME type, if standardised (many subtitle formats lack one).
    /// Absent in JSON5 means `None` (thanks to `default`).
    #[serde(rename = "mime", default)]
    pub mime_type: Option<String>,

    /// Human-readable format name.
    #[serde(rename = "name")]
    pub display_name: String,

    /// Whether this is a subtitle, caption, lyrics, or transcript file.
    pub kind: SubtitleKind,

    /// When `false` the format is ignored by all registry lookups.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Description of a companion file format.
///
/// Companion files travel alongside media files during rename/move operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanionFormat {
    /// File extension without leading dot, lower-case.
    #[serde(rename = "ext")]
    pub extension: String,

    /// IANA MIME type, if standardised.
    /// Absent in JSON5 means `None` (thanks to `default`).
    #[serde(rename = "mime", default)]
    pub mime_type: Option<String>,

    /// Human-readable description.
    #[serde(rename = "name")]
    pub display_name: String,

    /// How broadly this companion type applies.
    pub scope: CompanionScope,

    /// When `false` the format is ignored by all registry lookups.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Top-level deserialization wrapper
// ---------------------------------------------------------------------------

/// Internal struct that maps the top-level keys of `filetypes.json5`.
#[derive(Debug, Deserialize)]
struct FiletypeRegistryData {
    /// All audio format definitions.
    audio: Vec<AudioFormat>,
    /// All video format definitions.
    video: Vec<VideoFormat>,
    /// All subtitle / caption / lyrics format definitions.
    subtitle: Vec<SubtitleFormat>,
    /// All companion file format definitions.
    companion: Vec<CompanionFormat>,
}

// ---------------------------------------------------------------------------
// Lazy-initialised process-global registry
// ---------------------------------------------------------------------------

/// The singleton registry, initialised exactly once on first access.
///
/// All public API functions delegate to this instance, so lookups are
/// effectively `O(n)` linear scans over a Vec stored in static memory.
/// For the expected registry sizes (tens of entries) this is faster than a
/// HashMap in practice due to cache locality.
static REGISTRY: LazyLock<FiletypeRegistryData> = LazyLock::new(|| {
    // Attempt to load a user-provided override file.
    if let Some(user_json5) = load_user_override() {
        match json5::from_str::<FiletypeRegistryData>(&user_json5) {
            Ok(data) => {
                tracing::info!("filetype registry: loaded from user override file");
                return data;
            }
            Err(err) => {
                // Malformed override → warn and fall through to built-in defaults.
                tracing::warn!(
                    "filetype registry: user override file is malformed ({err}) \
                     — falling back to built-in defaults"
                );
            }
        }
    }

    // Parse the built-in defaults embedded at compile time.
    json5::from_str(DEFAULT_JSON5)
        .expect("built-in config/filetypes.json5 is malformed — this is a compile-time defect")
});

/// Try to read the user's custom `filetypes.json5` from the OS config directory.
///
/// Returns `None` if the file does not exist or cannot be read.
fn load_user_override() -> Option<String> {
    // `dirs::config_dir()` resolves to:
    //   Linux/macOS: ~/.config
    //   Windows:     %APPDATA%  (e.g. C:\Users\User\AppData\Roaming)
    let config_root = dirs::config_dir()?;
    let path = config_root
        .join("meedyamanager")
        .join("filetypes.json5");

    std::fs::read_to_string(&path).ok()
}

// ---------------------------------------------------------------------------
// Public slice accessors
// ---------------------------------------------------------------------------
//
// These functions expose the full lists (including disabled entries) for UI
// configuration screens.  The lookup helpers below filter out disabled entries.

/// All audio format definitions (including disabled ones).
pub fn audio_formats() -> &'static [AudioFormat] {
    &REGISTRY.audio
}

/// All video format definitions (including disabled ones).
pub fn video_formats() -> &'static [VideoFormat] {
    &REGISTRY.video
}

/// All subtitle / caption / lyrics format definitions (including disabled ones).
pub fn subtitle_formats() -> &'static [SubtitleFormat] {
    &REGISTRY.subtitle
}

/// All companion file format definitions (including disabled ones).
pub fn companion_formats() -> &'static [CompanionFormat] {
    &REGISTRY.companion
}

// ---------------------------------------------------------------------------
// Lookup helpers (only consider enabled entries)
// ---------------------------------------------------------------------------

/// Return the `AudioFormat` for the given extension (case-insensitive),
/// or `None` if not recognised as an audio format (or if disabled).
pub fn audio_format(ext: &str) -> Option<&'static AudioFormat> {
    let lower = ext.to_ascii_lowercase();
    REGISTRY.audio.iter().find(|f| f.enabled && f.extension == lower)
}

/// Return the `VideoFormat` for the given extension (case-insensitive),
/// or `None` if not recognised as a video format (or if disabled).
pub fn video_format(ext: &str) -> Option<&'static VideoFormat> {
    let lower = ext.to_ascii_lowercase();
    REGISTRY.video.iter().find(|f| f.enabled && f.extension == lower)
}

/// Return the `SubtitleFormat` for the given extension (case-insensitive),
/// or `None` if not a subtitle / lyrics / caption format (or if disabled).
pub fn subtitle_format(ext: &str) -> Option<&'static SubtitleFormat> {
    let lower = ext.to_ascii_lowercase();
    REGISTRY.subtitle.iter().find(|f| f.enabled && f.extension == lower)
}

/// Return the `CompanionFormat` for the given extension (case-insensitive),
/// or `None` if not a recognised companion format (or if disabled).
pub fn companion_format(ext: &str) -> Option<&'static CompanionFormat> {
    let lower = ext.to_ascii_lowercase();
    REGISTRY.companion.iter().find(|f| f.enabled && f.extension == lower)
}

/// Return `true` if the extension belongs to an enabled audio format.
pub fn is_audio(ext: &str) -> bool {
    audio_format(ext).is_some()
}

/// Return `true` if the extension belongs to an enabled video format.
pub fn is_video(ext: &str) -> bool {
    video_format(ext).is_some()
}

/// Return `true` if the extension is an audio or video media format.
pub fn is_media(ext: &str) -> bool {
    is_audio(ext) || is_video(ext)
}

/// Return the MIME type for any recognised enabled extension, or `None`.
pub fn mime_for_extension(ext: &str) -> Option<&'static str> {
    let lower = ext.to_ascii_lowercase();

    if let Some(af) = REGISTRY.audio.iter().find(|f| f.enabled && f.extension == lower) {
        return Some(af.mime_type.as_str());
    }
    if let Some(vf) = REGISTRY.video.iter().find(|f| f.enabled && f.extension == lower) {
        return Some(vf.mime_type.as_str());
    }
    if let Some(sf) = REGISTRY.subtitle.iter().find(|f| f.enabled && f.extension == lower) {
        return sf.mime_type.as_deref();
    }
    if let Some(cf) = REGISTRY.companion.iter().find(|f| f.enabled && f.extension == lower) {
        return cf.mime_type.as_deref();
    }
    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Registry completeness ────────────────────────────────────────────────

    #[test]
    fn audio_formats_is_non_empty() {
        assert!(!audio_formats().is_empty());
    }

    #[test]
    fn video_formats_is_non_empty() {
        assert!(!video_formats().is_empty());
    }

    #[test]
    fn subtitle_formats_is_non_empty() {
        assert!(!subtitle_formats().is_empty());
    }

    #[test]
    fn companion_formats_is_non_empty() {
        assert!(!companion_formats().is_empty());
    }

    // ── Extension uniqueness within each registry ────────────────────────────

    #[test]
    fn audio_extensions_are_unique() {
        let mut exts: Vec<_> = audio_formats().iter().map(|f| f.extension.as_str()).collect();
        exts.sort_unstable();
        let original_len = exts.len();
        exts.dedup();
        assert_eq!(exts.len(), original_len, "Duplicate audio extension detected");
    }

    #[test]
    fn video_extensions_are_unique() {
        let mut exts: Vec<_> = video_formats().iter().map(|f| f.extension.as_str()).collect();
        exts.sort_unstable();
        let original_len = exts.len();
        exts.dedup();
        assert_eq!(exts.len(), original_len, "Duplicate video extension detected");
    }

    #[test]
    fn subtitle_extensions_are_unique() {
        let mut exts: Vec<_> = subtitle_formats().iter().map(|f| f.extension.as_str()).collect();
        exts.sort_unstable();
        let original_len = exts.len();
        exts.dedup();
        assert_eq!(exts.len(), original_len, "Duplicate subtitle extension detected");
    }

    #[test]
    fn companion_extensions_are_unique() {
        let mut exts: Vec<_> = companion_formats().iter().map(|f| f.extension.as_str()).collect();
        exts.sort_unstable();
        let original_len = exts.len();
        exts.dedup();
        assert_eq!(exts.len(), original_len, "Duplicate companion extension detected");
    }

    // ── All extensions are lower-case ────────────────────────────────────────

    #[test]
    fn audio_extensions_are_lowercase() {
        for f in audio_formats() {
            assert_eq!(f.extension, f.extension.to_ascii_lowercase(),
                "Audio extension '{}' is not lower-case", f.extension);
        }
    }

    #[test]
    fn video_extensions_are_lowercase() {
        for f in video_formats() {
            assert_eq!(f.extension, f.extension.to_ascii_lowercase(),
                "Video extension '{}' is not lower-case", f.extension);
        }
    }

    // ── Key format lookups ────────────────────────────────────────────────────

    #[test]
    fn mp3_is_audio() {
        let af = audio_format("mp3").expect("mp3 must be in registry");
        assert!(!af.lossless);
        assert_eq!(af.mime_type, "audio/mpeg");
    }

    #[test]
    fn flac_is_lossless_audio() {
        let af = audio_format("flac").expect("flac must be in registry");
        assert!(af.lossless);
    }

    #[test]
    fn mkv_is_video() {
        assert!(video_format("mkv").is_some());
    }

    #[test]
    fn lrc_is_lyrics() {
        let sf = subtitle_format("lrc").expect("lrc must be in registry");
        assert_eq!(sf.kind, SubtitleKind::Lyrics);
    }

    #[test]
    fn srt_is_subtitle() {
        let sf = subtitle_format("srt").expect("srt must be in registry");
        assert_eq!(sf.kind, SubtitleKind::Subtitle);
    }

    // ── Companion format lookups ──────────────────────────────────────────────

    #[test]
    fn zip_is_album_scoped_companion() {
        let cf = companion_format("zip").expect("zip must be in companion registry");
        assert_eq!(cf.scope, CompanionScope::Album);
    }

    #[test]
    fn rar_is_album_scoped_companion() {
        let cf = companion_format("rar").expect("rar must be in companion registry");
        assert_eq!(cf.scope, CompanionScope::Album);
    }

    #[test]
    fn itlp_is_artist_scoped_companion() {
        let cf = companion_format("itlp").expect("itlp must be in companion registry");
        assert_eq!(cf.scope, CompanionScope::Artist);
    }

    #[test]
    fn nrg_is_album_scoped_companion() {
        let cf = companion_format("nrg").expect("nrg must be in companion registry");
        assert_eq!(cf.scope, CompanionScope::Album);
    }

    #[test]
    fn iso_is_album_scoped_companion() {
        let cf = companion_format("iso").expect("iso must be in companion registry");
        assert_eq!(cf.scope, CompanionScope::Album);
    }

    // ── Case-insensitive lookup ───────────────────────────────────────────────

    #[test]
    fn lookup_is_case_insensitive() {
        assert!(is_audio("MP3"), "Upper-case lookup should work for audio");
        assert!(is_video("MKV"), "Upper-case lookup should work for video");
        assert!(companion_format("ZIP").is_some(), "Upper-case lookup should work for companions");
    }

    // ── is_media helper ──────────────────────────────────────────────────────

    #[test]
    fn is_media_true_for_audio_and_video() {
        assert!(is_media("mp3"));
        assert!(is_media("mkv"));
        assert!(!is_media("srt"));
        assert!(!is_media("zip"));
        assert!(!is_media("nfo"));
    }

    // ── MIME type lookup ─────────────────────────────────────────────────────

    #[test]
    fn mime_for_mp3_is_audio_mpeg() {
        assert_eq!(mime_for_extension("mp3"), Some("audio/mpeg"));
    }

    #[test]
    fn mime_for_zip_is_application_zip() {
        assert_eq!(mime_for_extension("zip"), Some("application/zip"));
    }

    #[test]
    fn mime_for_unknown_is_none() {
        assert_eq!(mime_for_extension("xyz123"), None);
    }

    // ── JSON5 round-trip: built-in defaults parse without panic ──────────────

    #[test]
    fn default_json5_parses_successfully() {
        // Verifies the embedded JSON5 is well-formed by triggering the LazyLock.
        assert!(!audio_formats().is_empty());
        assert!(!video_formats().is_empty());
    }
}
