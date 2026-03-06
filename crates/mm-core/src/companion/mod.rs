// (C) 2025-2026 MWBM Partners Ltd
//
// Companion file detection and grouping.
//
// Detects files that belong alongside a media file (subtitles, lyrics,
// cue sheets, cover art, disc images, NFO files, archives) and groups
// them so they move together during rename operations.
//
// The canonical list of recognised extensions and their MIME types lives in
// `crate::filetype_registry`. This module uses that registry for lookups
// and adds file-system-level grouping logic on top.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{MmError, MmResult};
// Re-export CompanionScope from the filetype registry for consumer convenience
pub use crate::filetype_registry::CompanionScope;

/// Types of companion files we recognise.
///
/// When a new companion type is needed, add an entry here AND add the
/// relevant extension entries to `crate::filetype_registry::COMPANION_FORMATS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompanionType {
    /// Subtitle file (.srt, .sub, .ass, .ssa, .vtt, .idx, .smi, .ttml)
    Subtitle,
    /// Lyrics file (.lrc, .elrc)
    Lyrics,
    /// Cue sheet (.cue) — describes disc track layout
    CueSheet,
    /// Cover art image (cover.jpg, folder.png, album.jpg, etc.)
    CoverArt,
    /// Disc / optical image (.iso, .bin, .img, .nrg, .mdf, .mds, .daa)
    DiscImage,
    /// Compressed archive containing album release files (.zip, .rar, .7z)
    Archive,
    /// Apple iTunes LP / music store package (.itlp, .itmsp, .itms)
    ItunesPackage,
    /// Information / release notes file (.nfo)
    InfoFile,
    /// Log file from CD ripping (.log)
    RipLog,
    /// Playlist file (.m3u, .m3u8, .pls, .xspf, .wpl, .asx)
    Playlist,
    /// Accuracy check / checksum file (.accurip, .crc, .sfv, .md5)
    AccuracyCheck,
}

impl std::fmt::Display for CompanionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Subtitle      => write!(f, "Subtitle"),
            Self::Lyrics        => write!(f, "Lyrics"),
            Self::CueSheet      => write!(f, "Cue Sheet"),
            Self::CoverArt      => write!(f, "Cover Art"),
            Self::DiscImage     => write!(f, "Disc Image"),
            Self::Archive       => write!(f, "Archive"),
            Self::ItunesPackage => write!(f, "iTunes Package"),
            Self::InfoFile      => write!(f, "Info File"),
            Self::RipLog        => write!(f, "Rip Log"),
            Self::Playlist      => write!(f, "Playlist"),
            Self::AccuracyCheck => write!(f, "Accuracy Check"),
        }
    }
}

/// A detected companion file with its type and path
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompanionFile {
    /// Full path to the companion file
    pub path: PathBuf,
    /// What kind of companion this is
    pub companion_type: CompanionType,
}

/// A media file grouped with its companion files
#[derive(Debug, Clone)]
pub struct MediaGroup {
    /// The primary media file
    pub media_file: PathBuf,
    /// All detected companion files
    pub companions: Vec<CompanionFile>,
}

/// Well-known cover art filenames (case-insensitive matching)
const COVER_ART_STEMS: &[&str] = &[
    "cover", "folder", "album", "albumart", "albumartsmall",
    "front", "back", "disc", "cd", "inlay", "booklet",
    "artwork", "thumb", "thumbnail", "poster",
];

/// Image extensions that qualify as cover art
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff", "tif"];

/// Classify a file extension as a companion type, if applicable.
///
/// Returns the `CompanionType` for the given extension, or `None` if the
/// extension is not a recognised companion format.
///
/// The canonical source of truth for companion extensions is
/// `crate::filetype_registry::COMPANION_FORMATS`. This function mirrors
/// that registry and additionally handles subtitles (from SUBTITLE_FORMATS).
pub fn classify_companion(extension: &str) -> Option<CompanionType> {
    match extension.to_ascii_lowercase().as_str() {
        // ── Subtitles ────────────────────────────────────────────────────────
        "srt" | "sub" | "ass" | "ssa" | "vtt" | "idx" | "smi" | "ttml" | "dfxp" => {
            Some(CompanionType::Subtitle)
        }
        // ── Captions ─────────────────────────────────────────────────────────
        "sbv" | "srv1" | "srv2" | "srv3" | "cap" => Some(CompanionType::Subtitle),
        // ── Lyrics ───────────────────────────────────────────────────────────
        "lrc" | "elrc" => Some(CompanionType::Lyrics),
        // ── Cue sheets ───────────────────────────────────────────────────────
        "cue" => Some(CompanionType::CueSheet),
        // ── Disc / optical images ─────────────────────────────────────────────
        "iso" | "bin" | "img" | "nrg" | "mdf" | "mds" | "daa" | "udf" => {
            Some(CompanionType::DiscImage)
        }
        // ── Archives (ZIP, RAR and friends travel with album releases) ────────
        "zip" | "rar" | "7z" | "tar" | "gz" => Some(CompanionType::Archive),
        // ── Apple iTunes LP / music store packages ────────────────────────────
        "itlp" | "itmsp" | "itms" => Some(CompanionType::ItunesPackage),
        // ── Info / release notes ──────────────────────────────────────────────
        "nfo" => Some(CompanionType::InfoFile),
        // ── Rip logs ─────────────────────────────────────────────────────────
        "log" => Some(CompanionType::RipLog),
        // ── Playlists ────────────────────────────────────────────────────────
        "m3u" | "m3u8" | "pls" | "xspf" | "wpl" | "asx" => Some(CompanionType::Playlist),
        // ── Accuracy checks / checksums ───────────────────────────────────────
        "accurip" | "crc" | "sfv" | "md5" => Some(CompanionType::AccuracyCheck),
        // ── Not a companion extension ─────────────────────────────────────────
        _ => None,
    }
}

/// Check if a filename is a well-known cover art name
pub fn is_cover_art(path: &Path) -> bool {
    // Get the file stem (name without extension)
    let stem = match path.file_stem().and_then(OsStr::to_str) {
        Some(s) => s.to_ascii_lowercase(),
        None => return false,
    };

    // Get the extension
    let ext = match path.extension().and_then(OsStr::to_str) {
        Some(e) => e.to_ascii_lowercase(),
        None => return false,
    };

    // Must be an image file with a known cover art stem
    IMAGE_EXTENSIONS.contains(&ext.as_str()) && COVER_ART_STEMS.contains(&stem.as_str())
}

/// Detect all companion files for media files in a directory.
///
/// Groups companions with their parent media file by shared filename stem.
/// Also detects standalone cover art files (cover.jpg, folder.png, etc.)
/// which apply to all media files in the same directory.
pub fn detect_companions(
    media_files: &[PathBuf],
    directory_files: &[PathBuf],
) -> MmResult<Vec<MediaGroup>> {
    // Build a map from filename stem → media file path
    let mut stem_to_media: HashMap<String, &PathBuf> = HashMap::new();
    for media in media_files {
        if let Some(stem) = media.file_stem().and_then(OsStr::to_str) {
            stem_to_media.insert(stem.to_string(), media);
        }
    }

    // Initialise groups for all media files
    let mut groups: HashMap<PathBuf, Vec<CompanionFile>> = HashMap::new();
    for media in media_files {
        groups.entry(media.clone()).or_default();
    }

    // Directory-level cover art files (apply to all media in the directory)
    let mut dir_cover_art: Vec<CompanionFile> = Vec::new();

    for file in directory_files {
        // Skip if this file is one of the media files
        if media_files.contains(file) {
            continue;
        }

        // Check for directory-level cover art (cover.jpg, folder.png, etc.)
        if is_cover_art(file) {
            dir_cover_art.push(CompanionFile {
                path: file.clone(),
                companion_type: CompanionType::CoverArt,
            });
            continue;
        }

        // Check if this is a companion by extension
        let ext = match file.extension().and_then(OsStr::to_str) {
            Some(e) => e,
            None => continue,
        };

        if let Some(companion_type) = classify_companion(ext) {
            // Try to match by shared filename stem
            let stem = match file.file_stem().and_then(OsStr::to_str) {
                Some(s) => s.to_string(),
                None => continue,
            };

            if let Some(media) = stem_to_media.get(&stem) {
                groups.entry((*media).clone()).or_default().push(CompanionFile {
                    path: file.clone(),
                    companion_type,
                });
            }
        }
    }

    // Build result: attach directory-level cover art to all media files
    let result = groups
        .into_iter()
        .map(|(media_file, mut companions)| {
            companions.extend(dir_cover_art.iter().cloned());
            MediaGroup {
                media_file,
                companions,
            }
        })
        .collect();

    Ok(result)
}

/// Get companion files for a single media file by scanning its parent directory.
pub fn find_companions(media_path: &Path) -> MmResult<Vec<CompanionFile>> {
    let parent = media_path.parent().ok_or_else(|| {
        MmError::Companion("media file has no parent directory".into())
    })?;

    // Read directory contents
    let entries: Vec<PathBuf> = std::fs::read_dir(parent)
        .map_err(|e| MmError::Companion(format!("cannot read directory: {e}")))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| p.is_file())
        .collect();

    let media_stem = media_path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("");

    let mut companions = Vec::new();

    for entry in &entries {
        // Skip the media file itself
        if entry == media_path {
            continue;
        }

        // Check for directory-level cover art
        if is_cover_art(entry) {
            companions.push(CompanionFile {
                path: entry.clone(),
                companion_type: CompanionType::CoverArt,
            });
            continue;
        }

        // Check extension-based companions with matching stem
        let ext = match entry.extension().and_then(OsStr::to_str) {
            Some(e) => e,
            None => continue,
        };

        let entry_stem = entry
            .file_stem()
            .and_then(OsStr::to_str)
            .unwrap_or("");

        if entry_stem == media_stem {
            if let Some(companion_type) = classify_companion(ext) {
                companions.push(CompanionFile {
                    path: entry.clone(),
                    companion_type,
                });
            }
        }
    }

    Ok(companions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create an empty file at the given path
    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"").unwrap();
    }

    #[test]
    fn classify_subtitle_extensions() {
        assert_eq!(classify_companion("srt"), Some(CompanionType::Subtitle));
        assert_eq!(classify_companion("SRT"), Some(CompanionType::Subtitle));
        assert_eq!(classify_companion("sub"), Some(CompanionType::Subtitle));
        assert_eq!(classify_companion("ass"), Some(CompanionType::Subtitle));
        assert_eq!(classify_companion("vtt"), Some(CompanionType::Subtitle));
    }

    #[test]
    fn classify_lyrics_extension() {
        assert_eq!(classify_companion("lrc"), Some(CompanionType::Lyrics));
    }

    #[test]
    fn classify_cue_extension() {
        assert_eq!(classify_companion("cue"), Some(CompanionType::CueSheet));
    }

    #[test]
    fn classify_disc_image_extensions() {
        assert_eq!(classify_companion("iso"), Some(CompanionType::DiscImage));
        assert_eq!(classify_companion("bin"), Some(CompanionType::DiscImage));
    }

    #[test]
    fn classify_info_extension() {
        assert_eq!(classify_companion("nfo"), Some(CompanionType::InfoFile));
    }

    #[test]
    fn classify_playlist_extensions() {
        assert_eq!(classify_companion("m3u"), Some(CompanionType::Playlist));
        assert_eq!(classify_companion("m3u8"), Some(CompanionType::Playlist));
        assert_eq!(classify_companion("pls"), Some(CompanionType::Playlist));
    }

    #[test]
    fn classify_rip_log() {
        assert_eq!(classify_companion("log"), Some(CompanionType::RipLog));
    }

    #[test]
    fn classify_accuracy_check() {
        assert_eq!(classify_companion("accurip"), Some(CompanionType::AccuracyCheck));
        assert_eq!(classify_companion("crc"), Some(CompanionType::AccuracyCheck));
    }

    #[test]
    fn classify_unknown_extension() {
        assert_eq!(classify_companion("mp3"), None);
        assert_eq!(classify_companion("flac"), None);
        assert_eq!(classify_companion("xyz"), None);
    }

    #[test]
    fn classify_archive_extensions() {
        assert_eq!(classify_companion("zip"), Some(CompanionType::Archive));
        assert_eq!(classify_companion("ZIP"), Some(CompanionType::Archive));
        assert_eq!(classify_companion("rar"), Some(CompanionType::Archive));
        assert_eq!(classify_companion("7z"),  Some(CompanionType::Archive));
    }

    #[test]
    fn classify_itunes_package_extensions() {
        assert_eq!(classify_companion("itlp"),  Some(CompanionType::ItunesPackage));
        assert_eq!(classify_companion("itmsp"), Some(CompanionType::ItunesPackage));
        assert_eq!(classify_companion("itms"),  Some(CompanionType::ItunesPackage));
    }

    #[test]
    fn classify_checksum_extensions() {
        assert_eq!(classify_companion("sfv"), Some(CompanionType::AccuracyCheck));
        assert_eq!(classify_companion("md5"), Some(CompanionType::AccuracyCheck));
    }

    #[test]
    fn classify_ttml_subtitle() {
        assert_eq!(classify_companion("ttml"), Some(CompanionType::Subtitle));
        assert_eq!(classify_companion("dfxp"), Some(CompanionType::Subtitle));
    }

    #[test]
    fn companion_type_display_new_types() {
        assert_eq!(CompanionType::Archive.to_string(),       "Archive");
        assert_eq!(CompanionType::ItunesPackage.to_string(), "iTunes Package");
        assert_eq!(CompanionType::AccuracyCheck.to_string(), "Accuracy Check");
    }

    #[test]
    fn cover_art_detection_common_names() {
        assert!(is_cover_art(Path::new("cover.jpg")));
        assert!(is_cover_art(Path::new("Cover.JPG")));
        assert!(is_cover_art(Path::new("folder.png")));
        assert!(is_cover_art(Path::new("album.jpg")));
        assert!(is_cover_art(Path::new("front.png")));
        assert!(is_cover_art(Path::new("FRONT.JPEG")));
        assert!(is_cover_art(Path::new("back.webp")));
    }

    #[test]
    fn cover_art_rejects_non_image() {
        assert!(!is_cover_art(Path::new("cover.txt")));
        assert!(!is_cover_art(Path::new("folder.mp3")));
    }

    #[test]
    fn cover_art_rejects_non_standard_name() {
        assert!(!is_cover_art(Path::new("vacation.jpg")));
        assert!(!is_cover_art(Path::new("photo.png")));
    }

    #[test]
    fn detect_companions_by_stem() {
        let dir = TempDir::new().unwrap();
        let base = dir.path();

        let media = base.join("song.mp3");
        let srt = base.join("song.srt");
        let lrc = base.join("song.lrc");
        let unrelated = base.join("other.txt");

        touch(&media);
        touch(&srt);
        touch(&lrc);
        touch(&unrelated);

        let groups = detect_companions(
            &[media.clone()],
            &[media.clone(), srt, lrc, unrelated],
        )
        .unwrap();

        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        assert_eq!(group.media_file, media);
        assert_eq!(group.companions.len(), 2);
        let types: Vec<_> = group.companions.iter().map(|c| c.companion_type).collect();
        assert!(types.contains(&CompanionType::Subtitle));
        assert!(types.contains(&CompanionType::Lyrics));
    }

    #[test]
    fn detect_directory_cover_art() {
        let dir = TempDir::new().unwrap();
        let base = dir.path();

        let media = base.join("track.flac");
        let cover = base.join("cover.jpg");

        touch(&media);
        touch(&cover);

        let groups = detect_companions(
            &[media.clone()],
            &[media.clone(), cover],
        )
        .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].companions.len(), 1);
        assert_eq!(groups[0].companions[0].companion_type, CompanionType::CoverArt);
    }

    #[test]
    fn find_companions_from_disk() {
        let dir = TempDir::new().unwrap();
        let base = dir.path();

        let media = base.join("video.mkv");
        let sub = base.join("video.srt");
        let cover = base.join("folder.jpg");

        touch(&media);
        touch(&sub);
        touch(&cover);

        let companions = find_companions(&media).unwrap();
        assert_eq!(companions.len(), 2);
    }

    #[test]
    fn companion_type_display() {
        assert_eq!(CompanionType::Subtitle.to_string(), "Subtitle");
        assert_eq!(CompanionType::CoverArt.to_string(), "Cover Art");
        assert_eq!(CompanionType::CueSheet.to_string(), "Cue Sheet");
        assert_eq!(CompanionType::RipLog.to_string(), "Rip Log");
    }

    #[test]
    fn empty_directory_no_companions() {
        let dir = TempDir::new().unwrap();
        let media = dir.path().join("song.mp3");
        touch(&media);

        let companions = find_companions(&media).unwrap();
        assert!(companions.is_empty());
    }
}
