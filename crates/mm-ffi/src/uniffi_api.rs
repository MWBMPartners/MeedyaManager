// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — UniFFI-exported API functions
//
// All public functions are annotated with `#[uniffi::export]` and exposed
// to Swift (macOS) via the UniFFI proc-macro scaffolding registered in lib.rs.
//
// Design rules:
//   - All parameters and return types must be UniFFI-compatible
//   - TagMap (HashMap<String, Vec<String>>) is flattened to Vec<TagEntry>
//     because UniFFI does not support nested generic types
//   - Errors are converted from MmError → MmFfiError at every boundary
//   - The file watcher uses a background thread to forward channel events
//     to the UniFFI callback interface

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

use mm_core::classify;
use mm_core::config::AppConfig;
use mm_core::metadata::{self, TagMap};
use mm_core::renamer::{self, SanitizeConfig};
use mm_core::rule_engine::{self, evaluator::{EvalContext, evaluate_template}};
use mm_core::watcher::{self, WatchEvent, WatcherConfig};

use crate::callbacks::WatchCallback;
use crate::types::{AudioPropertiesFfi, MmFfiError, RenamePreviewFfi, TagEntry, ValidationResult, WatchEventFfi};

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// Return the MeedyaManager core version string (e.g. "0.5.0").
#[uniffi::export]
pub fn mm_version() -> String {
    // Injected at compile time from Cargo.toml [package].version
    env!("CARGO_PKG_VERSION").to_string()
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Return the platform-specific path to `settings.json5`.
///
/// macOS:   `~/Library/Application Support/MeedyaManager/settings.json5`
/// Linux:   `~/.config/MeedyaManager/settings.json5`
/// Windows: `%APPDATA%\MeedyaManager\settings.json5`
#[uniffi::export]
pub fn config_path() -> String {
    dirs::config_dir()
        .map(|d| d.join("MeedyaManager").join("settings.json5"))
        .unwrap_or_else(|| PathBuf::from("settings.json5"))
        .to_string_lossy()
        .into_owned()
}

/// Load the configuration from the platform-default location.
///
/// Returns the config serialized as a JSON string for the Settings panel.
/// If no config file exists, returns the default configuration JSON.
#[uniffi::export]
pub fn config_load() -> Result<String, MmFfiError> {
    // AppConfig::load() reads from the platform config dir (no arguments)
    let config = AppConfig::load()
        .map_err(MmFfiError::from)?;

    serde_json::to_string_pretty(&config)
        .map_err(|e| MmFfiError::Config(e.to_string()))
}

// ---------------------------------------------------------------------------
// Media scanning & rename preview
// ---------------------------------------------------------------------------

/// Scan a directory and compute rename previews for all media files.
///
/// - `directory` — absolute path to scan
/// - `template`  — MusicBee-style rename template (e.g. `"<Artist> - <Title>"`)
/// - `recursive` — if true, descend into sub-directories
///
/// Returns previews sorted by source path. The UI shows this list before the
/// user confirms execution via `execute_renames`.
#[uniffi::export]
pub fn scan_directory(
    directory: String,
    template: String,
    recursive: bool,
) -> Result<Vec<RenamePreviewFfi>, MmFfiError> {
    let dir_path = PathBuf::from(&directory);

    // Collect paths of all recognised media files in the directory
    let media_files = collect_media_files(&dir_path, recursive)
        .map_err(|e| MmFfiError::Io(e.to_string()))?;

    // For each file: read metadata, flatten to HashMap<String, String>, collect
    let files_with_tags: Vec<(PathBuf, HashMap<String, String>)> = media_files
        .into_iter()
        .map(|path| {
            // Read tags; use empty map for files that cannot be read
            let flat = metadata::extract_tags(&path)
                .map(|tag_map| flatten_tag_map(tag_map))
                .unwrap_or_default();
            (path, flat)
        })
        .collect();

    if files_with_tags.is_empty() {
        return Ok(vec![]);
    }

    // Use the source directory itself as the output directory
    // (renamer computes relative names, UI confirms full paths)
    let sanitize_cfg = SanitizeConfig::default();

    // Simulate renames using the renamer module
    let summary = renamer::simulate_rename(
        &files_with_tags,
        &template,
        &dir_path,
        &sanitize_cfg,
    )
    .map_err(MmFfiError::from)?;

    // Convert mm-core RenamePreview to FFI-safe RenamePreviewFfi
    let mut previews: Vec<RenamePreviewFfi> = summary
        .previews
        .into_iter()
        .map(RenamePreviewFfi::from_core)
        .collect();

    // Sort by source path for deterministic UI display order
    previews.sort_by(|a, b| a.source.cmp(&b.source));

    Ok(previews)
}

/// Execute a set of renames (non-conflicting, non-unchanged only).
///
/// Returns the count of files successfully renamed.
#[uniffi::export]
pub fn execute_renames(previews: Vec<RenamePreviewFfi>) -> Result<u32, MmFfiError> {
    let mut count = 0u32;

    for preview in previews {
        // Skip unchanged files and conflicting destinations
        if preview.unchanged || preview.conflict {
            continue;
        }

        let src = PathBuf::from(&preview.source);
        let dst = PathBuf::from(&preview.destination);

        // Ensure destination directory exists (handles sub-directory templates)
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MmFfiError::Io(e.to_string()))?;
        }

        // Perform the rename
        std::fs::rename(&src, &dst)
            .map_err(|e| MmFfiError::Rename(format!("{}: {e}", preview.source)))?;

        count += 1;
    }

    Ok(count)
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/// Read all metadata tags from a single media file.
///
/// Returns a list of `TagEntry` pairs sorted by key for stable UI display.
/// Multi-value tags (e.g. multiple artists) are joined with "; ".
#[uniffi::export]
pub fn get_metadata(path: String) -> Result<Vec<TagEntry>, MmFfiError> {
    let file_path = PathBuf::from(&path);

    // Extract the multi-value TagMap from the file
    let tag_map = metadata::extract_tags(&file_path)
        .map_err(MmFfiError::from)?;

    // Flatten: join Vec<String> values with "; " and build sorted TagEntry list
    let mut entries: Vec<TagEntry> = tag_map
        .into_iter()
        .map(|(key, values)| TagEntry {
            key,
            // Join multi-values with the canonical MeedyaManager delimiter
            value: values.join("; "),
        })
        .collect();

    // Sort by key for deterministic display order
    entries.sort_by(|a, b| a.key.cmp(&b.key));

    Ok(entries)
}

/// Write/update metadata tags on a media file.
///
/// Only the tags in `tags` are written; existing tags not in the list are
/// preserved. Multi-value tags can be passed with "; " as the delimiter.
#[uniffi::export]
pub fn write_metadata(path: String, tags: Vec<TagEntry>) -> Result<(), MmFfiError> {
    let file_path = PathBuf::from(&path);

    // Convert Vec<TagEntry> → TagMap (HashMap<String, Vec<String>>)
    // Split "; "-delimited values back into separate entries
    let tag_map: TagMap = tags
        .into_iter()
        .map(|e| {
            // Split on "; " to reconstruct multi-value vectors
            let values = e.value
                .split("; ")
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned())
                .collect::<Vec<_>>();
            (e.key, if values.is_empty() { vec![e.value] } else { values })
        })
        .collect();

    metadata::write_tags(&file_path, &tag_map)
        .map_err(MmFfiError::from)
}

/// Remove a single tag field from a media file.
///
/// Uses the canonical lowercase key (e.g. "title", "artist", "album").
/// This is a no-op if the key is not recognised or not present.
#[uniffi::export]
pub fn remove_tag(path: String, tag_key: String) -> Result<(), MmFfiError> {
    let file_path = PathBuf::from(&path);
    metadata::remove_tag(&file_path, &tag_key)
        .map_err(MmFfiError::from)
}

/// Read audio technical properties from a media file.
///
/// Returns duration, bitrate, sample rate, channels, and bit depth.
/// Fields that cannot be determined are set to 0.
#[uniffi::export]
pub fn get_audio_properties(path: String) -> Result<AudioPropertiesFfi, MmFfiError> {
    let file_path = PathBuf::from(&path);
    let props = metadata::extract_audio_properties(&file_path)
        .map_err(MmFfiError::from)?;

    // Determine codec and lossless flag from the file extension via classify
    let (codec, is_lossless) = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|ext| {
            let classification = classify::classify_by_extension(ext);
            let codec_str = classification.format.extension().to_ascii_uppercase();
            // Lossless formats: bit depth is typically present (Some), lossy formats return None
            let lossless = props.bits_per_sample.is_some();
            (codec_str, lossless)
        })
        .unwrap_or_else(|| ("Unknown".to_string(), false));

    Ok(AudioPropertiesFfi {
        // Convert fractional seconds to whole seconds (u32)
        duration_secs: props.duration_secs as u32,
        // Bitrate in kbps; 0 if unknown
        bitrate_kbps: props.bitrate_kbps.unwrap_or(0),
        // Sample rate in Hz; 0 if unknown
        sample_rate_hz: props.sample_rate_hz.unwrap_or(0),
        // Channel count; 0 if unknown
        channels: props.channels.unwrap_or(0),
        // Bit depth; 0 for lossy formats
        bit_depth: props.bits_per_sample.unwrap_or(0),
        is_lossless,
        codec,
    })
}

// ---------------------------------------------------------------------------
// Rule / template engine
// ---------------------------------------------------------------------------

/// Validate a rename template string.
///
/// Safe to call on every keystroke from the rule builder UI.
/// Returns a `ValidationResult` with `is_valid`, error message, and warnings.
#[uniffi::export]
pub fn validate_template(template: String) -> ValidationResult {
    // Empty / whitespace-only templates are immediately invalid
    if template.trim().is_empty() {
        return ValidationResult {
            is_valid: false,
            error_message: "Template must not be empty".into(),
            warnings: vec![],
        };
    }

    // Parse the template through the rule engine lexer + parser
    // A successful parse means the syntax is valid
    match rule_engine::parse_template(&template) {
        Ok(_ast) => ValidationResult {
            is_valid: true,
            error_message: String::new(),
            // Future: add warnings for unknown tag names here
            warnings: vec![],
        },
        Err(e) => ValidationResult {
            is_valid: false,
            error_message: e.to_string(),
            warnings: vec![],
        },
    }
}

/// Apply a rename template to a set of tags and return the computed filename.
///
/// Used by the rule builder live-preview to show the template result
/// against a sample file's metadata without touching any files.
#[uniffi::export]
pub fn apply_template(template: String, tags: Vec<TagEntry>) -> Result<String, MmFfiError> {
    // Convert Vec<TagEntry> → TagMap for EvalContext
    let tag_map: TagMap = tags
        .into_iter()
        .map(|e| (e.key, vec![e.value]))
        .collect();

    // Build an evaluation context from the tag map
    let ctx = EvalContext::new(&tag_map);

    // Parse + evaluate the template
    evaluate_template(&template, &ctx)
        .map_err(MmFfiError::from)
}

/// List all tag display names that MeedyaManager recognises.
///
/// Returns names as used in templates (e.g. "Artist", "Title", "Album").
/// Used to populate the tag picker in the rule builder UI.
#[uniffi::export]
pub fn list_known_tags() -> Vec<String> {
    // These are the canonical template tag names from the tag registry.
    // Kept as a static list here so Swift/C# don't need to call into the
    // lexer; the rule engine accepts these case-insensitively at evaluation time.
    vec![
        "Title".into(), "Artist".into(), "Album".into(), "AlbumArtist".into(),
        "Year".into(), "Genre".into(), "TrackNumber".into(), "TrackTotal".into(),
        "DiscNumber".into(), "DiscTotal".into(), "Composer".into(), "Comment".into(),
        "Lyrics".into(), "ISRC".into(), "Barcode".into(), "CatalogNumber".into(),
        "Label".into(), "Compilation".into(), "BPM".into(),
        // Virtual / computed tags
        "Filename".into(), "Extension".into(), "Folder".into(), "FullPath".into(),
        "Duration".into(), "DurationSecs".into(), "BitrateKbps".into(),
        "SampleRateHz".into(), "Channels".into(), "BitDepth".into(),
        "MediaClass".into(), "MediaGroup".into(), "MediaFormat".into(), "MediaQuality".into(),
    ]
}

// ---------------------------------------------------------------------------
// File watcher
// ---------------------------------------------------------------------------

/// Internal handle keeping a watcher alive and its reader thread running.
struct ActiveWatcher {
    /// The notify watcher — dropping this closes the channel sender and
    /// causes the reader thread to exit its receive loop naturally.
    _watcher: notify::RecommendedWatcher,
    /// Background thread that reads WatchEvents and forwards to the callback.
    /// Joined (cleaned up) when the handle is dropped.
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for ActiveWatcher {
    fn drop(&mut self) {
        // The _watcher field drops first, closing the channel.
        // We then join the thread to ensure the callback is not called
        // after the handle is removed from WATCHERS.
        if let Some(handle) = self.thread.take() {
            // Thread will exit shortly after the channel closes;
            // best-effort join (ignore errors from panicking threads)
            let _ = handle.join();
        }
    }
}

/// Map of active watcher handles keyed by their handle ID.
static WATCHERS: std::sync::LazyLock<Mutex<HashMap<u64, ActiveWatcher>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Atomic counter for generating unique watcher handle IDs.
static NEXT_HANDLE_ID: AtomicU64 = AtomicU64::new(1);

/// Start watching a directory for file system events.
///
/// Events are delivered to `callback` from a background thread.
/// Returns a handle ID to pass to `stop_watch` when done.
#[uniffi::export]
pub fn start_watch(
    directory: String,
    callback: Arc<dyn WatchCallback>,
) -> Result<u64, MmFfiError> {
    let dir_path = PathBuf::from(&directory);

    // Build a WatcherConfig for the target directory
    let config = WatcherConfig {
        folders: vec![dir_path],
        recursive: true,
        ..WatcherConfig::default()
    };

    // Start the channel-based watcher from mm-core
    let (watcher, receiver) = watcher::start_watcher(&config)
        .map_err(MmFfiError::from)?;

    // Assign a unique ID for this watcher instance
    let handle_id = NEXT_HANDLE_ID.fetch_add(1, Ordering::SeqCst);

    // Spawn a background thread that reads WatchEvents from the channel
    // and forwards them to the UniFFI callback implementation
    let thread = std::thread::spawn(move || {
        // Block until an event arrives or the channel is closed (watcher dropped)
        while let Ok(event) = receiver.recv() {
            // Convert mm-core WatchEvent to FFI-safe WatchEventFfi
            let ffi_event = watch_event_to_ffi(event);
            // Deliver to the callback implementation (Swift / Kotlin / test)
            callback.on_event(ffi_event);
        }
        // Channel closed — watcher was stopped; thread exits cleanly
    });

    // Store the handle so stop_watch can find and drop it
    WATCHERS.lock().unwrap().insert(handle_id, ActiveWatcher {
        _watcher: watcher,
        thread: Some(thread),
    });

    Ok(handle_id)
}

/// Stop a previously started directory watcher.
///
/// Removing the handle drops the watcher, which closes the event channel
/// and causes the reader thread to exit. This is a no-op for unknown IDs.
#[uniffi::export]
pub fn stop_watch(handle_id: u64) {
    // Removing from the map drops ActiveWatcher, which drops _watcher
    // (closing the channel) and joins the thread via the Drop impl.
    WATCHERS.lock().unwrap().remove(&handle_id);
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Collect all media (Audio + Video) file paths from a directory.
///
/// Uses the classify module to determine if each file is a recognised
/// media format. Other file types (documents, archives, etc.) are skipped.
pub(crate) fn collect_media_files(
    dir: &PathBuf,
    recursive: bool,
) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    collect_media_files_inner(dir, recursive, &mut paths)?;
    Ok(paths)
}

/// Recursive inner helper for `collect_media_files`.
fn collect_media_files_inner(
    dir: &PathBuf,
    recursive: bool,
    out: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    use mm_core::classify::MediaGroup;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && recursive {
            // Recurse into sub-directory
            collect_media_files_inner(&path, recursive, out)?;
        } else if path.is_file() {
            // Check the file extension against the classify module
            let is_media = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| {
                    let c = classify::classify_by_extension(ext);
                    // Include Audio and Video files only; skip Image/Document/Archive
                    matches!(c.group, MediaGroup::Audio | MediaGroup::Video)
                })
                .unwrap_or(false);

            if is_media {
                out.push(path);
            }
        }
    }

    Ok(())
}

/// Flatten a `TagMap` (HashMap<String, Vec<String>>) to a flat
/// `HashMap<String, String>` by joining multi-values with "; ".
///
/// This is required because `renamer::simulate_rename` works with the
/// flat map while `metadata::extract_tags` returns the multi-value form.
fn flatten_tag_map(tag_map: TagMap) -> HashMap<String, String> {
    tag_map
        .into_iter()
        .map(|(key, values)| (key, values.join("; ")))
        .collect()
}

/// Convert a mm-core `WatchEvent` to the FFI-safe `WatchEventFfi`.
fn watch_event_to_ffi(event: WatchEvent) -> WatchEventFfi {
    match event {
        WatchEvent::Created(path) => WatchEventFfi {
            kind: "created".into(),
            path: path.to_string_lossy().into_owned(),
            new_path: String::new(),
        },
        WatchEvent::Modified(path) => WatchEventFfi {
            kind: "modified".into(),
            path: path.to_string_lossy().into_owned(),
            new_path: String::new(),
        },
        WatchEvent::Deleted(path) => WatchEventFfi {
            kind: "deleted".into(),
            path: path.to_string_lossy().into_owned(),
            new_path: String::new(),
        },
        WatchEvent::Renamed(from, to) => WatchEventFfi {
            kind: "renamed".into(),
            path: from.to_string_lossy().into_owned(),
            new_path: to.to_string_lossy().into_owned(),
        },
    }
}
