// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — File Type Registry
//
// Single source of truth for all file types MeedyaManager recognises.
// Each entry captures the file extension, MIME type, human-readable name,
// and classification category.
//
// To add a new file type:
//   1. Add an entry to the appropriate static array below (AUDIO_FORMATS,
//      VIDEO_FORMATS, SUBTITLE_FORMATS, or COMPANION_FORMATS).
//   2. Re-run `cargo test -p mm-core` to verify the registry invariants.
//
// The companion classify_companion() and classify_by_extension() helpers in
// companion/mod.rs and classify/mod.rs delegate here for canonical lookups.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Sub-type enums
// ---------------------------------------------------------------------------

/// Scope of a companion file — how broadly it applies within a library.
///
/// Used to decide whether a companion file moves with a single track,
/// with the whole album folder, or with all albums for an artist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
// ---------------------------------------------------------------------------

/// Description of a single audio file format.
#[derive(Debug, Clone, Copy)]
pub struct AudioFormat {
    /// File extension without leading dot, lower-case (e.g. `"mp3"`).
    pub extension: &'static str,
    /// IANA MIME type string (e.g. `"audio/mpeg"`).
    pub mime_type: &'static str,
    /// Human-readable format name (e.g. `"MP3"`).
    pub display_name: &'static str,
    /// `true` for lossless formats (FLAC, ALAC, WAV, AIFF, …).
    pub lossless: bool,
}

/// Description of a single video file format.
#[derive(Debug, Clone, Copy)]
pub struct VideoFormat {
    /// File extension without leading dot, lower-case (e.g. `"mkv"`).
    pub extension: &'static str,
    /// IANA MIME type string (e.g. `"video/x-matroska"`).
    pub mime_type: &'static str,
    /// Human-readable format name (e.g. `"Matroska"`).
    pub display_name: &'static str,
}

/// Description of a subtitle / caption / lyrics file format.
#[derive(Debug, Clone, Copy)]
pub struct SubtitleFormat {
    /// File extension without leading dot, lower-case.
    pub extension: &'static str,
    /// IANA MIME type, if standardised (many subtitle formats lack one).
    pub mime_type: Option<&'static str>,
    /// Human-readable format name.
    pub display_name: &'static str,
    /// Whether this is a subtitle, caption, lyrics, or transcript file.
    pub kind: SubtitleKind,
}

/// Description of a companion file format.
///
/// Companion files travel alongside media files during rename/move operations.
#[derive(Debug, Clone, Copy)]
pub struct CompanionFormat {
    /// File extension without leading dot, lower-case.
    pub extension: &'static str,
    /// IANA MIME type, if standardised.
    pub mime_type: Option<&'static str>,
    /// Human-readable description.
    pub display_name: &'static str,
    /// How broadly this companion type applies.
    pub scope: CompanionScope,
}

// ---------------------------------------------------------------------------
// Audio formats
// ---------------------------------------------------------------------------

/// All audio file extensions recognised by MeedyaManager.
///
/// Edit this array to add / remove audio formats. The order is not significant.
pub static AUDIO_FORMATS: &[AudioFormat] = &[
    // ── Lossy compressed ────────────────────────────────────────────────────
    AudioFormat { extension: "mp3",  mime_type: "audio/mpeg",               display_name: "MP3",                   lossless: false },
    AudioFormat { extension: "aac",  mime_type: "audio/aac",                display_name: "AAC",                   lossless: false },
    AudioFormat { extension: "m4a",  mime_type: "audio/mp4",                display_name: "M4A (AAC/ALAC)",        lossless: false }, // container; may be lossless if ALAC
    AudioFormat { extension: "m4b",  mime_type: "audio/mp4",                display_name: "M4B (Audiobook)",       lossless: false },
    AudioFormat { extension: "m4r",  mime_type: "audio/mp4",                display_name: "M4R (iPhone Ringtone)", lossless: false },
    AudioFormat { extension: "ogg",  mime_type: "audio/ogg",                display_name: "Ogg Vorbis",            lossless: false },
    AudioFormat { extension: "oga",  mime_type: "audio/ogg",                display_name: "Ogg Audio",             lossless: false },
    AudioFormat { extension: "opus", mime_type: "audio/ogg; codecs=opus",   display_name: "Opus",                  lossless: false },
    AudioFormat { extension: "wma",  mime_type: "audio/x-ms-wma",           display_name: "WMA",                   lossless: false },
    AudioFormat { extension: "amr",  mime_type: "audio/amr",                display_name: "AMR",                   lossless: false },
    AudioFormat { extension: "3gp",  mime_type: "audio/3gpp",               display_name: "3GPP Audio",            lossless: false },
    AudioFormat { extension: "mp2",  mime_type: "audio/mpeg",               display_name: "MP2",                   lossless: false },
    AudioFormat { extension: "ra",   mime_type: "audio/x-realaudio",        display_name: "RealAudio",             lossless: false },
    AudioFormat { extension: "ape",  mime_type: "audio/x-ape",              display_name: "Monkey's Audio (APE)",  lossless: true  }, // technically lossless
    AudioFormat { extension: "mpc",  mime_type: "audio/x-musepack",         display_name: "Musepack",              lossless: false },
    AudioFormat { extension: "spx",  mime_type: "audio/ogg; codecs=speex",  display_name: "Speex",                 lossless: false },
    AudioFormat { extension: "snd",  mime_type: "audio/basic",              display_name: "NeXT/Sun Audio",        lossless: false },

    // ── Lossless compressed ─────────────────────────────────────────────────
    AudioFormat { extension: "flac", mime_type: "audio/flac",               display_name: "FLAC",                  lossless: true },
    AudioFormat { extension: "alac", mime_type: "audio/mp4",                display_name: "ALAC",                  lossless: true },
    AudioFormat { extension: "wv",   mime_type: "audio/x-wavpack",          display_name: "WavPack",               lossless: true },
    AudioFormat { extension: "tta",  mime_type: "audio/x-tta",              display_name: "True Audio (TTA)",      lossless: true },

    // ── Uncompressed / PCM ──────────────────────────────────────────────────
    AudioFormat { extension: "wav",  mime_type: "audio/wav",                display_name: "WAV (PCM)",             lossless: true },
    AudioFormat { extension: "aiff", mime_type: "audio/aiff",               display_name: "AIFF",                  lossless: true },
    AudioFormat { extension: "aif",  mime_type: "audio/aiff",               display_name: "AIFF",                  lossless: true },
    AudioFormat { extension: "aifc", mime_type: "audio/aiff",               display_name: "AIFF-C",                lossless: true },
    AudioFormat { extension: "au",   mime_type: "audio/basic",              display_name: "Sun AU",                lossless: true },
    AudioFormat { extension: "caf",  mime_type: "audio/x-caf",              display_name: "Core Audio Format (CAF)", lossless: true },

    // ── Tracker / chiptune ──────────────────────────────────────────────────
    AudioFormat { extension: "mod",  mime_type: "audio/x-mod",              display_name: "MOD Tracker",           lossless: true },
    AudioFormat { extension: "xm",   mime_type: "audio/x-xm",               display_name: "XM Tracker",            lossless: true },
    AudioFormat { extension: "it",   mime_type: "audio/x-it",               display_name: "Impulse Tracker",       lossless: true },
    AudioFormat { extension: "s3m",  mime_type: "audio/x-s3m",              display_name: "ScreamTracker 3",       lossless: true },
    AudioFormat { extension: "mid",  mime_type: "audio/midi",               display_name: "MIDI",                  lossless: true },
    AudioFormat { extension: "midi", mime_type: "audio/midi",               display_name: "MIDI",                  lossless: true },
];

// ---------------------------------------------------------------------------
// Video formats
// ---------------------------------------------------------------------------

/// All video file extensions recognised by MeedyaManager.
pub static VIDEO_FORMATS: &[VideoFormat] = &[
    VideoFormat { extension: "mp4",  mime_type: "video/mp4",               display_name: "MPEG-4 Video" },
    VideoFormat { extension: "m4v",  mime_type: "video/x-m4v",             display_name: "M4V (iTunes Video)" },
    VideoFormat { extension: "mkv",  mime_type: "video/x-matroska",        display_name: "Matroska" },
    VideoFormat { extension: "webm", mime_type: "video/webm",              display_name: "WebM" },
    VideoFormat { extension: "avi",  mime_type: "video/x-msvideo",         display_name: "AVI" },
    VideoFormat { extension: "mov",  mime_type: "video/quicktime",         display_name: "QuickTime MOV" },
    VideoFormat { extension: "wmv",  mime_type: "video/x-ms-wmv",          display_name: "Windows Media Video" },
    VideoFormat { extension: "flv",  mime_type: "video/x-flv",             display_name: "Flash Video" },
    VideoFormat { extension: "f4v",  mime_type: "video/mp4",               display_name: "Flash MP4 Video" },
    VideoFormat { extension: "mpg",  mime_type: "video/mpeg",              display_name: "MPEG Video" },
    VideoFormat { extension: "mpeg", mime_type: "video/mpeg",              display_name: "MPEG Video" },
    VideoFormat { extension: "ts",   mime_type: "video/mp2t",              display_name: "MPEG Transport Stream" },
    VideoFormat { extension: "m2ts", mime_type: "video/mp2t",              display_name: "MPEG-2 Transport Stream" },
    VideoFormat { extension: "mts",  mime_type: "video/mp2t",              display_name: "AVCHD Transport Stream" },
    VideoFormat { extension: "vob",  mime_type: "video/mpeg",              display_name: "DVD Video Object" },
    VideoFormat { extension: "rmvb", mime_type: "application/vnd.rn-realmedia-vbr", display_name: "RealMedia Variable Bitrate" },
    VideoFormat { extension: "rm",   mime_type: "application/vnd.rn-realmedia", display_name: "RealMedia" },
    VideoFormat { extension: "3gp",  mime_type: "video/3gpp",              display_name: "3GPP Video" },
    VideoFormat { extension: "ogv",  mime_type: "video/ogg",               display_name: "Ogg Video" },
    VideoFormat { extension: "divx", mime_type: "video/x-msvideo",         display_name: "DivX Video" },
    VideoFormat { extension: "xvid", mime_type: "video/x-msvideo",         display_name: "Xvid Video" },
    VideoFormat { extension: "dv",   mime_type: "video/x-dv",              display_name: "Digital Video (DV)" },
    VideoFormat { extension: "hevc", mime_type: "video/hevc",              display_name: "HEVC / H.265" },
    VideoFormat { extension: "heic", mime_type: "image/heic",              display_name: "HEIC (Apple Video Still)" },
    VideoFormat { extension: "avif", mime_type: "image/avif",              display_name: "AVIF" },
];

// ---------------------------------------------------------------------------
// Subtitle / caption / lyrics formats
// ---------------------------------------------------------------------------

/// All subtitle, caption, and lyrics file extensions recognised by MeedyaManager.
///
/// These are considered per-track companions with special handling in the
/// rename engine (file is renamed alongside its paired media file).
pub static SUBTITLE_FORMATS: &[SubtitleFormat] = &[
    // ── Subtitles ────────────────────────────────────────────────────────────
    SubtitleFormat { extension: "srt",  mime_type: Some("application/x-subrip"),        display_name: "SubRip (.srt)",              kind: SubtitleKind::Subtitle },
    SubtitleFormat { extension: "sub",  mime_type: None,                                 display_name: "MicroDVD / SubViewer",       kind: SubtitleKind::Subtitle },
    SubtitleFormat { extension: "ass",  mime_type: Some("text/x-ass"),                   display_name: "Advanced SSA (.ass)",        kind: SubtitleKind::Subtitle },
    SubtitleFormat { extension: "ssa",  mime_type: Some("text/x-ssa"),                   display_name: "SubStation Alpha (.ssa)",    kind: SubtitleKind::Subtitle },
    SubtitleFormat { extension: "vtt",  mime_type: Some("text/vtt"),                     display_name: "WebVTT (.vtt)",              kind: SubtitleKind::Subtitle },
    SubtitleFormat { extension: "idx",  mime_type: None,                                 display_name: "VobSub Index (.idx)",        kind: SubtitleKind::Subtitle },
    SubtitleFormat { extension: "smi",  mime_type: Some("application/smil+xml"),         display_name: "SAMI (.smi)",                kind: SubtitleKind::Subtitle },
    SubtitleFormat { extension: "ttml", mime_type: Some("application/ttml+xml"),         display_name: "Timed Text (.ttml)",         kind: SubtitleKind::Subtitle },
    SubtitleFormat { extension: "dfxp", mime_type: Some("application/ttml+xml"),         display_name: "DFXP (.dfxp)",               kind: SubtitleKind::Subtitle },

    // ── Captions ─────────────────────────────────────────────────────────────
    SubtitleFormat { extension: "sbv",  mime_type: None,                                 display_name: "YouTube SBV (.sbv)",         kind: SubtitleKind::Caption },
    SubtitleFormat { extension: "srv1", mime_type: None,                                 display_name: "YouTube SRV1 (.srv1)",       kind: SubtitleKind::Caption },
    SubtitleFormat { extension: "srv2", mime_type: None,                                 display_name: "YouTube SRV2 (.srv2)",       kind: SubtitleKind::Caption },
    SubtitleFormat { extension: "srv3", mime_type: None,                                 display_name: "YouTube SRV3 (.srv3)",       kind: SubtitleKind::Caption },
    SubtitleFormat { extension: "cap",  mime_type: None,                                 display_name: "Caption (.cap)",             kind: SubtitleKind::Caption },

    // ── Lyrics ───────────────────────────────────────────────────────────────
    SubtitleFormat { extension: "lrc",  mime_type: Some("application/x-lrc"),            display_name: "Timed Lyrics (.lrc)",        kind: SubtitleKind::Lyrics },
    SubtitleFormat { extension: "elrc", mime_type: None,                                 display_name: "Enhanced LRC (.elrc)",       kind: SubtitleKind::Lyrics },

    // ── Transcripts ──────────────────────────────────────────────────────────
    SubtitleFormat { extension: "txt",  mime_type: Some("text/plain"),                   display_name: "Plain Text Transcript",      kind: SubtitleKind::Transcript },
];

// ---------------------------------------------------------------------------
// Companion file formats
// ---------------------------------------------------------------------------

/// All companion file formats recognised by MeedyaManager.
///
/// Companion files are non-media files that belong alongside media files
/// (e.g. cover art, cue sheets, rip logs, disc images, archives).
/// They are kept together with their parent media file during rename/move.
///
/// The `scope` field indicates how broadly each companion type applies:
/// - `Track` — travels with a single track (rename by shared filename stem)
/// - `Album` — applies to the whole album directory
/// - `Artist` — applies to all albums for an artist
pub static COMPANION_FORMATS: &[CompanionFormat] = &[
    // ── Cue sheets (Track/Album scope) ──────────────────────────────────────
    CompanionFormat { extension: "cue",     mime_type: Some("application/x-cue"),             display_name: "Cue Sheet",                    scope: CompanionScope::Album  },

    // ── Disc / optical images (Album scope) ─────────────────────────────────
    CompanionFormat { extension: "iso",     mime_type: Some("application/x-iso9660-image"),   display_name: "ISO Disc Image",               scope: CompanionScope::Album  },
    CompanionFormat { extension: "bin",     mime_type: Some("application/x-cd-image"),        display_name: "BIN Disc Image",               scope: CompanionScope::Album  },
    CompanionFormat { extension: "img",     mime_type: Some("application/x-raw-disk-image"),  display_name: "IMG Disc Image",               scope: CompanionScope::Album  },
    CompanionFormat { extension: "nrg",     mime_type: Some("application/x-nrg"),             display_name: "Nero Image (.nrg)",            scope: CompanionScope::Album  },
    CompanionFormat { extension: "mdf",     mime_type: None,                                   display_name: "Media Descriptor Image (.mdf)", scope: CompanionScope::Album },
    CompanionFormat { extension: "mds",     mime_type: None,                                   display_name: "MDS Sidecar (.mds)",           scope: CompanionScope::Album  },
    CompanionFormat { extension: "daa",     mime_type: None,                                   display_name: "PowerISO Image (.daa)",        scope: CompanionScope::Album  },
    CompanionFormat { extension: "udf",     mime_type: None,                                   display_name: "UDF Disc Image (.udf)",        scope: CompanionScope::Album  },

    // ── Archives (Album scope) ───────────────────────────────────────────────
    // ZIP and RAR archives often contain the full album release package
    // (liner notes, booklet PDFs, bonus tracks). Keep them alongside the album.
    CompanionFormat { extension: "zip",     mime_type: Some("application/zip"),               display_name: "ZIP Archive",                  scope: CompanionScope::Album  },
    CompanionFormat { extension: "rar",     mime_type: Some("application/x-rar-compressed"),  display_name: "RAR Archive",                  scope: CompanionScope::Album  },
    CompanionFormat { extension: "7z",      mime_type: Some("application/x-7z-compressed"),   display_name: "7-Zip Archive",                scope: CompanionScope::Album  },
    CompanionFormat { extension: "tar",     mime_type: Some("application/x-tar"),             display_name: "TAR Archive",                  scope: CompanionScope::Album  },
    CompanionFormat { extension: "gz",      mime_type: Some("application/gzip"),              display_name: "Gzip Archive",                 scope: CompanionScope::Album  },

    // ── Apple music packages (Artist scope) ─────────────────────────────────
    // iTunes LP / iTunes Artist packages are Apple-proprietary album/artist
    // bundles. They apply at the artist level and should travel with all
    // albums for that artist.
    CompanionFormat { extension: "itlp",    mime_type: None,                                   display_name: "iTunes LP Package (.itlp)",   scope: CompanionScope::Artist },
    CompanionFormat { extension: "itmsp",   mime_type: None,                                   display_name: "iTunes Music Store Package",   scope: CompanionScope::Artist },
    CompanionFormat { extension: "itms",    mime_type: None,                                   display_name: "iTunes Music Store Link",      scope: CompanionScope::Artist },

    // ── Info files (Album scope) ─────────────────────────────────────────────
    CompanionFormat { extension: "nfo",     mime_type: Some("text/plain"),                    display_name: "Release Info (.nfo)",          scope: CompanionScope::Album  },
    CompanionFormat { extension: "sfv",     mime_type: Some("text/plain"),                    display_name: "SFV Checksum (.sfv)",          scope: CompanionScope::Album  },
    CompanionFormat { extension: "md5",     mime_type: Some("text/plain"),                    display_name: "MD5 Checksum (.md5)",          scope: CompanionScope::Album  },

    // ── Rip logs (Album scope) ───────────────────────────────────────────────
    CompanionFormat { extension: "log",     mime_type: Some("text/plain"),                    display_name: "Rip Log (.log)",               scope: CompanionScope::Album  },

    // ── Accuracy checks (Album scope) ────────────────────────────────────────
    CompanionFormat { extension: "accurip", mime_type: None,                                   display_name: "AccurateRip Result (.accurip)", scope: CompanionScope::Album },
    CompanionFormat { extension: "crc",     mime_type: None,                                   display_name: "CRC Checksum (.crc)",          scope: CompanionScope::Album  },

    // ── Playlists (Album scope) ──────────────────────────────────────────────
    CompanionFormat { extension: "m3u",     mime_type: Some("audio/x-mpegurl"),               display_name: "M3U Playlist",                 scope: CompanionScope::Album  },
    CompanionFormat { extension: "m3u8",    mime_type: Some("audio/x-mpegurl"),               display_name: "M3U8 Playlist (UTF-8)",        scope: CompanionScope::Album  },
    CompanionFormat { extension: "pls",     mime_type: Some("audio/x-scpls"),                 display_name: "PLS Playlist",                 scope: CompanionScope::Album  },
    CompanionFormat { extension: "xspf",    mime_type: Some("application/xspf+xml"),          display_name: "XSPF Playlist",                scope: CompanionScope::Album  },
    CompanionFormat { extension: "wpl",     mime_type: Some("application/vnd.ms-wpl"),        display_name: "Windows Media Playlist",       scope: CompanionScope::Album  },
    CompanionFormat { extension: "asx",     mime_type: Some("video/x-ms-asf"),                display_name: "ASX Playlist",                 scope: CompanionScope::Album  },
];

// ---------------------------------------------------------------------------
// Lookup helpers
// ---------------------------------------------------------------------------

/// Return the `AudioFormat` for the given extension (case-insensitive),
/// or `None` if not recognised as an audio format.
pub fn audio_format(ext: &str) -> Option<&'static AudioFormat> {
    let lower = ext.to_ascii_lowercase();
    AUDIO_FORMATS.iter().find(|f| f.extension == lower)
}

/// Return the `VideoFormat` for the given extension (case-insensitive),
/// or `None` if not recognised as a video format.
pub fn video_format(ext: &str) -> Option<&'static VideoFormat> {
    let lower = ext.to_ascii_lowercase();
    VIDEO_FORMATS.iter().find(|f| f.extension == lower)
}

/// Return the `SubtitleFormat` for the given extension (case-insensitive),
/// or `None` if not a subtitle / lyrics / caption format.
pub fn subtitle_format(ext: &str) -> Option<&'static SubtitleFormat> {
    let lower = ext.to_ascii_lowercase();
    SUBTITLE_FORMATS.iter().find(|f| f.extension == lower)
}

/// Return the `CompanionFormat` for the given extension (case-insensitive),
/// or `None` if not a recognised companion format.
pub fn companion_format(ext: &str) -> Option<&'static CompanionFormat> {
    let lower = ext.to_ascii_lowercase();
    COMPANION_FORMATS.iter().find(|f| f.extension == lower)
}

/// Return `true` if the extension belongs to an audio format.
pub fn is_audio(ext: &str) -> bool {
    audio_format(ext).is_some()
}

/// Return `true` if the extension belongs to a video format.
pub fn is_video(ext: &str) -> bool {
    video_format(ext).is_some()
}

/// Return `true` if the extension is an audio or video media format.
pub fn is_media(ext: &str) -> bool {
    is_audio(ext) || is_video(ext)
}

/// Return the MIME type for any recognised extension, or `None`.
pub fn mime_for_extension(ext: &str) -> Option<&'static str> {
    let lower = ext.to_ascii_lowercase();
    if let Some(af) = AUDIO_FORMATS.iter().find(|f| f.extension == lower.as_str()) {
        return Some(af.mime_type);
    }
    if let Some(vf) = VIDEO_FORMATS.iter().find(|f| f.extension == lower.as_str()) {
        return Some(vf.mime_type);
    }
    if let Some(sf) = SUBTITLE_FORMATS.iter().find(|f| f.extension == lower.as_str()) {
        return sf.mime_type;
    }
    if let Some(cf) = COMPANION_FORMATS.iter().find(|f| f.extension == lower.as_str()) {
        return cf.mime_type;
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
        assert!(!AUDIO_FORMATS.is_empty());
    }

    #[test]
    fn video_formats_is_non_empty() {
        assert!(!VIDEO_FORMATS.is_empty());
    }

    #[test]
    fn subtitle_formats_is_non_empty() {
        assert!(!SUBTITLE_FORMATS.is_empty());
    }

    #[test]
    fn companion_formats_is_non_empty() {
        assert!(!COMPANION_FORMATS.is_empty());
    }

    // ── Extension uniqueness within each registry ────────────────────────────

    #[test]
    fn audio_extensions_are_unique() {
        let mut exts: Vec<_> = AUDIO_FORMATS.iter().map(|f| f.extension).collect();
        exts.sort_unstable();
        let original_len = exts.len();
        exts.dedup();
        assert_eq!(exts.len(), original_len, "Duplicate audio extension detected");
    }

    #[test]
    fn video_extensions_are_unique() {
        let mut exts: Vec<_> = VIDEO_FORMATS.iter().map(|f| f.extension).collect();
        exts.sort_unstable();
        let original_len = exts.len();
        exts.dedup();
        assert_eq!(exts.len(), original_len, "Duplicate video extension detected");
    }

    #[test]
    fn subtitle_extensions_are_unique() {
        let mut exts: Vec<_> = SUBTITLE_FORMATS.iter().map(|f| f.extension).collect();
        exts.sort_unstable();
        let original_len = exts.len();
        exts.dedup();
        assert_eq!(exts.len(), original_len, "Duplicate subtitle extension detected");
    }

    #[test]
    fn companion_extensions_are_unique() {
        let mut exts: Vec<_> = COMPANION_FORMATS.iter().map(|f| f.extension).collect();
        exts.sort_unstable();
        let original_len = exts.len();
        exts.dedup();
        assert_eq!(exts.len(), original_len, "Duplicate companion extension detected");
    }

    // ── All extensions are lower-case ────────────────────────────────────────

    #[test]
    fn audio_extensions_are_lowercase() {
        for f in AUDIO_FORMATS {
            assert_eq!(f.extension, f.extension.to_ascii_lowercase(),
                "Audio extension '{}' is not lower-case", f.extension);
        }
    }

    #[test]
    fn video_extensions_are_lowercase() {
        for f in VIDEO_FORMATS {
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

    // ── New companion formats ─────────────────────────────────────────────────

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
}
