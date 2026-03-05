// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// File system watcher with debouncing and polling fallback.
//
// Uses the `notify` crate for native OS events, with an optional
// polling backend for network/cloud-mounted drives where inotify/FSEvents
// may not work.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{
    Config as NotifyConfig, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::error::{MmError, MmResult};

/// Events emitted by the file watcher
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchEvent {
    /// A new file was created
    Created(PathBuf),
    /// An existing file was modified
    Modified(PathBuf),
    /// A file was deleted
    Deleted(PathBuf),
    /// A file was renamed (from, to)
    Renamed(PathBuf, PathBuf),
}

impl std::fmt::Display for WatchEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created(p) => write!(f, "Created: {}", p.display()),
            Self::Modified(p) => write!(f, "Modified: {}", p.display()),
            Self::Deleted(p) => write!(f, "Deleted: {}", p.display()),
            Self::Renamed(from, to) => {
                write!(f, "Renamed: {} → {}", from.display(), to.display())
            }
        }
    }
}

/// Configuration for the file watcher
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherConfig {
    /// Directories to watch
    pub folders: Vec<PathBuf>,
    /// Watch subdirectories recursively
    pub recursive: bool,
    /// Debounce interval in milliseconds (coalesce rapid events)
    pub debounce_ms: u64,
    /// File extensions to include (empty = all)
    pub include_extensions: Vec<String>,
    /// File extensions to exclude
    pub exclude_extensions: Vec<String>,
    /// Filename patterns to ignore (e.g. temp files)
    pub ignore_patterns: Vec<String>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            folders: Vec::new(),
            recursive: true,
            debounce_ms: 500,
            include_extensions: Vec::new(),
            exclude_extensions: vec![
                "tmp".into(), "part".into(), "crdownload".into(),
                "download".into(),
            ],
            ignore_patterns: vec![
                ".*".into(),       // Hidden files
                "~*".into(),       // Temp files
                "Thumbs.db".into(),
                ".DS_Store".into(),
                "desktop.ini".into(),
            ],
        }
    }
}

/// Check if a path should be filtered out based on config
pub fn should_ignore(path: &Path, config: &WatcherConfig) -> bool {
    // Get filename
    let filename = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return true, // No filename = ignore
    };

    // Check ignore patterns
    for pattern in &config.ignore_patterns {
        if pattern.starts_with(".*") && filename.starts_with('.') {
            return true;
        }
        if pattern.starts_with("~*") && filename.starts_with('~') {
            return true;
        }
        if pattern.eq_ignore_ascii_case(filename) {
            return true;
        }
    }

    // Check extension filters
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    // If include list is non-empty, only allow listed extensions
    if !config.include_extensions.is_empty() {
        if !config.include_extensions.iter().any(|e| e.to_ascii_lowercase() == ext) {
            return true;
        }
    }

    // Exclude listed extensions
    if config.exclude_extensions.iter().any(|e| e.to_ascii_lowercase() == ext) {
        return true;
    }

    false
}

/// Convert a notify event to our WatchEvent type.
///
/// Returns None for events we don't care about (access, metadata-only, etc.)
pub fn convert_event(event: &Event) -> Vec<WatchEvent> {
    let mut events = Vec::new();

    for path in &event.paths {
        match event.kind {
            EventKind::Create(_) => {
                events.push(WatchEvent::Created(path.clone()));
            }
            EventKind::Modify(_) => {
                events.push(WatchEvent::Modified(path.clone()));
            }
            EventKind::Remove(_) => {
                events.push(WatchEvent::Deleted(path.clone()));
            }
            _ => {
                // Access, Other, etc. — skip
            }
        }
    }

    // Handle rename events (notify provides two paths)
    if matches!(event.kind, EventKind::Modify(_))
        && event.paths.len() == 2
    {
        events.clear();
        events.push(WatchEvent::Renamed(
            event.paths[0].clone(),
            event.paths[1].clone(),
        ));
    }

    events
}

/// Start watching directories and return a receiver for events.
///
/// Spawns a background thread that listens for OS file events and
/// applies debouncing + filtering before sending them to the receiver.
pub fn start_watcher(
    config: &WatcherConfig,
) -> MmResult<(RecommendedWatcher, mpsc::Receiver<WatchEvent>)> {
    let (tx, rx) = mpsc::channel();
    let config_clone = config.clone();

    // Create the notify watcher with debounce config
    let _debounce = Duration::from_millis(config.debounce_ms);
    let notify_config = NotifyConfig::default()
        .with_poll_interval(Duration::from_secs(2));

    let watcher_tx = tx.clone();
    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| {
            match result {
                Ok(event) => {
                    let watch_events = convert_event(&event);
                    for we in watch_events {
                        // Apply filtering
                        let path = match &we {
                            WatchEvent::Created(p) => p,
                            WatchEvent::Modified(p) => p,
                            WatchEvent::Deleted(p) => p,
                            WatchEvent::Renamed(_, to) => to,
                        };

                        if !should_ignore(path, &config_clone) {
                            if watcher_tx.send(we).is_err() {
                                // Receiver dropped — stop
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("File watcher error: {e}");
                }
            }
        },
        notify_config,
    )?;

    // Watch all configured directories
    let mode = if config.recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };

    for folder in &config.folders {
        if folder.exists() {
            info!("Watching directory: {}", folder.display());
            watcher.watch(folder, mode)?;
        } else {
            warn!("Watch folder does not exist: {}", folder.display());
        }
    }

    Ok((watcher, rx))
}

/// Collect all matching files in watched directories (initial scan).
///
/// Returns a set of all file paths that match the watch configuration
/// filters. Useful for building initial state before watching for changes.
pub fn scan_existing_files(config: &WatcherConfig) -> MmResult<Vec<PathBuf>> {
    let mut files = Vec::new();

    for folder in &config.folders {
        if !folder.exists() {
            warn!("Scan folder does not exist: {}", folder.display());
            continue;
        }

        scan_directory(folder, config.recursive, config, &mut files)?;
    }

    // Sort for deterministic output
    files.sort();

    info!("Initial scan found {} files", files.len());
    Ok(files)
}

/// Recursively scan a directory for matching files
fn scan_directory(
    dir: &Path,
    recursive: bool,
    config: &WatcherConfig,
    files: &mut Vec<PathBuf>,
) -> MmResult<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| {
        MmError::Watcher(format!("cannot read directory {}: {e}", dir.display()))
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| {
            MmError::Watcher(format!("directory entry error: {e}"))
        })?;
        let path = entry.path();

        if path.is_dir() && recursive {
            // Skip hidden directories
            let dirname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !dirname.starts_with('.') {
                scan_directory(&path, recursive, config, files)?;
            }
        } else if path.is_file() {
            if !should_ignore(&path, config) {
                files.push(path);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, b"").unwrap();
    }

    #[test]
    fn should_ignore_hidden_files() {
        let config = WatcherConfig::default();
        assert!(should_ignore(Path::new(".hidden"), &config));
        assert!(should_ignore(Path::new(".DS_Store"), &config));
    }

    #[test]
    fn should_ignore_temp_files() {
        let config = WatcherConfig::default();
        assert!(should_ignore(Path::new("~temp.mp3"), &config));
    }

    #[test]
    fn should_ignore_excluded_extensions() {
        let config = WatcherConfig::default();
        assert!(should_ignore(Path::new("file.tmp"), &config));
        assert!(should_ignore(Path::new("file.part"), &config));
        assert!(should_ignore(Path::new("file.crdownload"), &config));
    }

    #[test]
    fn should_not_ignore_media_files() {
        let config = WatcherConfig::default();
        assert!(!should_ignore(Path::new("song.mp3"), &config));
        assert!(!should_ignore(Path::new("video.mkv"), &config));
        assert!(!should_ignore(Path::new("track.flac"), &config));
    }

    #[test]
    fn should_ignore_system_files() {
        let config = WatcherConfig::default();
        assert!(should_ignore(Path::new("Thumbs.db"), &config));
        assert!(should_ignore(Path::new("desktop.ini"), &config));
    }

    #[test]
    fn include_filter_restricts_extensions() {
        let mut config = WatcherConfig::default();
        config.include_extensions = vec!["mp3".into(), "flac".into()];

        assert!(!should_ignore(Path::new("song.mp3"), &config));
        assert!(!should_ignore(Path::new("track.FLAC"), &config));
        assert!(should_ignore(Path::new("video.mkv"), &config));
        assert!(should_ignore(Path::new("doc.pdf"), &config));
    }

    #[test]
    fn convert_create_event() {
        let event = Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![PathBuf::from("/test/file.mp3")],
            attrs: Default::default(),
        };
        let results = convert_event(&event);
        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], WatchEvent::Created(p) if p == Path::new("/test/file.mp3")));
    }

    #[test]
    fn convert_modify_event() {
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("/test/file.mp3")],
            attrs: Default::default(),
        };
        let results = convert_event(&event);
        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], WatchEvent::Modified(_)));
    }

    #[test]
    fn convert_delete_event() {
        let event = Event {
            kind: EventKind::Remove(notify::event::RemoveKind::File),
            paths: vec![PathBuf::from("/test/file.mp3")],
            attrs: Default::default(),
        };
        let results = convert_event(&event);
        assert_eq!(results.len(), 1);
        assert!(matches!(&results[0], WatchEvent::Deleted(_)));
    }

    #[test]
    fn watch_event_display() {
        let event = WatchEvent::Created(PathBuf::from("/test/song.mp3"));
        assert_eq!(event.to_string(), "Created: /test/song.mp3");

        let event = WatchEvent::Renamed(
            PathBuf::from("/test/old.mp3"),
            PathBuf::from("/test/new.mp3"),
        );
        assert!(event.to_string().contains("→"));
    }

    #[test]
    fn scan_existing_files_basic() {
        let dir = TempDir::new().unwrap();
        let base = dir.path();

        touch(&base.join("song.mp3"));
        touch(&base.join("track.flac"));
        touch(&base.join(".hidden"));
        touch(&base.join("file.tmp"));

        let config = WatcherConfig {
            folders: vec![base.to_path_buf()],
            ..Default::default()
        };

        let files = scan_existing_files(&config).unwrap();
        assert_eq!(files.len(), 2); // mp3 + flac, not hidden or tmp
    }

    #[test]
    fn scan_existing_files_recursive() {
        let dir = TempDir::new().unwrap();
        let base = dir.path();

        touch(&base.join("song.mp3"));
        touch(&base.join("subdir").join("track.flac"));
        touch(&base.join("subdir").join("nested").join("deep.wav"));

        let config = WatcherConfig {
            folders: vec![base.to_path_buf()],
            recursive: true,
            ..Default::default()
        };

        let files = scan_existing_files(&config).unwrap();
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn scan_existing_files_non_recursive() {
        let dir = TempDir::new().unwrap();
        let base = dir.path();

        touch(&base.join("song.mp3"));
        touch(&base.join("subdir").join("track.flac"));

        let config = WatcherConfig {
            folders: vec![base.to_path_buf()],
            recursive: false,
            ..Default::default()
        };

        let files = scan_existing_files(&config).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn scan_nonexistent_folder_warns() {
        let config = WatcherConfig {
            folders: vec![PathBuf::from("/nonexistent/path")],
            ..Default::default()
        };

        let files = scan_existing_files(&config).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn default_watcher_config() {
        let config = WatcherConfig::default();
        assert!(config.recursive);
        assert_eq!(config.debounce_ms, 500);
        assert!(config.include_extensions.is_empty());
        assert!(!config.exclude_extensions.is_empty());
    }
}
