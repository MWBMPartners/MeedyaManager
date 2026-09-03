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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use mm_core::classify;
use mm_core::config::AppConfig;
// The integrity guard — the only sanctioned way to mutate a media file.
use mm_core::integrity;
use mm_core::metadata::{self, TagMap};
use mm_core::renamer::{self, SanitizeConfig};
use mm_core::rule_engine::{
    self,
    evaluator::{EvalContext, evaluate_template},
};
use mm_core::watcher::{self, WatchEvent, WatcherConfig};

use crate::callbacks::WatchCallback;
use crate::types::{
    AudioPropertiesFfi, MmFfiError, RenamePreviewFfi, TagEntry, ValidationResult, WatchEventFfi,
};

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
    // Routed through `mm_core::config::app_config_dir()` (the single
    // resolver — see issue #212, P0-CONFIGDIR) rather than calling
    // `dirs::config_dir()` directly, so this always matches where
    // `AppConfig::load()` actually reads from.
    mm_core::config::app_config_dir()
        .map_or_else(
            |_| PathBuf::from("settings.json5"),
            |d| d.join("settings.json5"),
        )
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
    let config = AppConfig::load().map_err(MmFfiError::from)?;

    serde_json::to_string_pretty(&config).map_err(|e| MmFfiError::Config(e.to_string()))
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
    let media_files =
        collect_media_files(&dir_path, recursive).map_err(|e| MmFfiError::Io(e.to_string()))?;

    // For each file: read metadata, flatten to HashMap<String, String>, collect
    let files_with_tags: Vec<(PathBuf, HashMap<String, String>)> = media_files
        .into_iter()
        .map(|path| {
            // Read tags; use empty map for files that cannot be read
            let flat = metadata::extract_tags(&path)
                .map(flatten_tag_map)
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
    let summary = renamer::simulate_rename(&files_with_tags, &template, &dir_path, &sanitize_cfg)
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
///
/// Routed through [`renamer::execute_rename`] rather than moving the file
/// directly via the standard library (issue #201 reached the CLI, mm-core and
/// mm-gtk but missed this FFI path). That gives the FFI surface the same
/// safety net every other caller gets: a re-check of the destination
/// immediately before the move (a preview's `conflict` flag can go stale
/// between scan and execute — another process, the user, or an earlier
/// entry in this very batch may have created it since), plus the
/// create-directories and copy-vs-move policy handling — see
/// [`renamer::ExecuteOptions`].
#[uniffi::export]
pub fn execute_renames(previews: Vec<RenamePreviewFfi>) -> Result<u32, MmFfiError> {
    let mut count = 0u32;

    for preview in previews {
        // Skip unchanged files and conflicting destinations
        if preview.unchanged || preview.conflict {
            continue;
        }

        // Rebuild the mm-core RenamePreview the FFI-safe type was flattened
        // from, so `execute_rename` gets exactly the facts it re-validates.
        let core_preview = renamer::RenamePreview {
            source: PathBuf::from(&preview.source),
            destination: PathBuf::from(&preview.destination),
            conflict: preview.conflict,
            unchanged: preview.unchanged,
        };

        renamer::execute_rename(&core_preview).map_err(MmFfiError::from)?;

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
    let tag_map = metadata::extract_tags(&file_path).map_err(MmFfiError::from)?;

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
///
/// ## Behaviour changes callers must know about
///
/// * **Test Mode is honoured.**  With Test Mode enabled the original file is
///   left byte-for-byte untouched and the tags land on a `_MeedyaManager`
///   copy beside it (issue #128).  A subsequent write accumulates onto that
///   same copy.
/// * **Unknown keys are rejected.**  A key with no `ItemKey` mapping used to
///   be dropped silently, so the call returned `Ok` having changed nothing.
///   It now returns `MmFfiError::Metadata` naming the key and listing the
///   valid ones (issue #206).  Use `mm_core::metadata::known_tag_keys` — or
///   simply write back keys obtained from `get_metadata`.
#[uniffi::export]
pub fn write_metadata(path: String, tags: Vec<TagEntry>) -> Result<(), MmFfiError> {
    let file_path = PathBuf::from(&path);

    // Convert Vec<TagEntry> → TagMap (HashMap<String, Vec<String>>)
    // Split "; "-delimited values back into separate entries
    let tag_map: TagMap = tags
        .into_iter()
        .map(|e| {
            // Split on "; " to reconstruct multi-value vectors
            let values = e
                .value
                .split("; ")
                .filter(|s| !s.is_empty())
                .map(std::borrow::ToOwned::to_owned)
                .collect::<Vec<_>>();
            (
                e.key,
                if values.is_empty() {
                    vec![e.value]
                } else {
                    values
                },
            )
        })
        .collect();

    // Route through the integrity guard rather than the raw metadata layer:
    // the guard is the only enforcement point for Test Mode (issue #128), so
    // a direct call would overwrite the user's original file even with Test
    // Mode on.  The guard also gives us the atomic-rename + hash-verify
    // behaviour the native UIs would otherwise have to reimplement.
    let result = integrity::write_tags_safe(&file_path, &tag_map);

    if result.success {
        Ok(())
    } else {
        Err(MmFfiError::Metadata(
            result
                .error
                .unwrap_or_else(|| "metadata write failed".to_string()),
        ))
    }
}

/// Remove a single tag field from a media file.
///
/// Uses the canonical lowercase key (e.g. "title", "artist", "album") — see
/// `mm_core::metadata::known_tag_keys` for the full list.  Removing a key the
/// file does not carry succeeds; passing a key that is not in the mapping at
/// all now returns `MmFfiError::Metadata` rather than silently succeeding
/// (issue #206) — see the note on `write_metadata`.
///
/// Honours Test Mode: with it enabled the original file is left untouched and
/// the removal is applied to the `_MeedyaManager` copy.
#[uniffi::export]
pub fn remove_tag(path: String, tag_key: String) -> Result<(), MmFfiError> {
    let file_path = PathBuf::from(&path);

    // Integrity guard, not the raw metadata layer — see `write_metadata`.
    let result = integrity::remove_tag_safe(&file_path, &tag_key);

    if result.success {
        Ok(())
    } else {
        Err(MmFfiError::Metadata(
            result
                .error
                .unwrap_or_else(|| "tag removal failed".to_string()),
        ))
    }
}

/// Read audio technical properties from a media file.
///
/// Returns duration, bitrate, sample rate, channels, and bit depth.
/// Fields that cannot be determined are set to 0.
#[uniffi::export]
pub fn get_audio_properties(path: String) -> Result<AudioPropertiesFfi, MmFfiError> {
    let file_path = PathBuf::from(&path);
    let props = metadata::extract_audio_properties(&file_path).map_err(MmFfiError::from)?;

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
    let tag_map: TagMap = tags.into_iter().map(|e| (e.key, vec![e.value])).collect();

    // Build an evaluation context from the tag map
    let ctx = EvalContext::new(&tag_map);

    // Parse + evaluate the template
    evaluate_template(&template, &ctx).map_err(MmFfiError::from)
}

/// List all tag display names that MeedyaManager recognises.
///
/// Returns names as used in templates (e.g. "Artist", "Title", "Album"),
/// sorted alphabetically.  Sourced dynamically from the TagRegistry so any
/// user-defined custom tags added to `tags.json5` are included automatically.
///
/// Used to populate the tag picker in the rule builder UI and by the FFI
/// layer so Swift / C# do not need to maintain their own lists.
#[uniffi::export]
pub fn list_known_tags() -> Vec<String> {
    // Delegate to the TagRegistry which loads from config/tags.json5
    // (embedded at compile time, user-overridable at runtime).
    mm_core::metadata::tag_registry::all_known_template_tags()
}

// ---------------------------------------------------------------------------
// Test Mode
// ---------------------------------------------------------------------------
//
// Mirrors `mm_core::test_mode`'s five entry points for the native UIs. Until
// these were exported, `macos/MeedyaManager/Bindings/MmCore.swift` referenced
// `testModeEnabled`, `setTestMode`, `testModeFileCount`, `commitTestModeFiles`
// and `revertTestModeFiles` with nothing on the Rust side to bind to, so the
// macOS app could not compile against the real generated bindings.

/// Return whether Test Mode is currently enabled.
///
/// Fails open (returns `false`) if the manifest cannot be read — see
/// `mm_core::test_mode::is_enabled` for why that is the safer default for a
/// UI toggle: the alternative traps the user in a Test Mode they cannot see
/// how to turn off.
#[uniffi::export]
pub fn test_mode_enabled() -> bool {
    mm_core::test_mode::is_enabled()
}

/// Enable or disable Test Mode.
///
/// Disabling does not itself commit or revert already-staged files — the UI
/// is expected to call `commit_test_mode_files` or `revert_test_mode_files`
/// around this, as `mm_core::test_mode::disable` documents.
#[uniffi::export]
pub fn set_test_mode(enabled: bool) -> Result<(), MmFfiError> {
    if enabled {
        mm_core::test_mode::enable().map_err(MmFfiError::from)
    } else {
        mm_core::test_mode::disable().map_err(MmFfiError::from)
    }
}

/// Return the number of files currently staged in Test Mode.
#[uniffi::export]
pub fn test_mode_file_count() -> u32 {
    // Manifest files are files-on-disk, never anywhere near u32::MAX.
    mm_core::test_mode::tracked_file_count() as u32
}

/// Commit all staged Test Mode files: originals are deleted and their
/// `_MeedyaManager` copies take the original names.
///
/// Returns the number of files successfully committed.
#[uniffi::export]
pub fn commit_test_mode_files() -> Result<u32, MmFfiError> {
    mm_core::test_mode::commit_files()
        .map(|committed| committed as u32)
        .map_err(MmFfiError::from)
}

/// Revert all staged Test Mode files, discarding the staged copies and
/// leaving the originals untouched.
#[uniffi::export]
pub fn revert_test_mode_files() -> Result<(), MmFfiError> {
    mm_core::test_mode::revert_files().map_err(MmFfiError::from)
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
pub fn start_watch(directory: String, callback: Arc<dyn WatchCallback>) -> Result<u64, MmFfiError> {
    let dir_path = PathBuf::from(&directory);

    // Build a WatcherConfig for the target directory
    let config = WatcherConfig {
        folders: vec![dir_path],
        recursive: true,
        ..WatcherConfig::default()
    };

    // Start the channel-based watcher from mm-core
    let (watcher, receiver) = watcher::start_watcher(&config).map_err(MmFfiError::from)?;

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
    WATCHERS.lock().unwrap().insert(
        handle_id,
        ActiveWatcher {
            _watcher: watcher,
            thread: Some(thread),
        },
    );

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
pub(crate) fn collect_media_files(dir: &PathBuf, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
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
            // Skip Test Mode copies (`_MeedyaManager` suffixed duplicates).
            // A scan that picked these up would offer to rename the staged
            // copy alongside — or instead of — the real original, which is
            // exactly the confusion Test Mode exists to prevent: the copy is
            // an internal implementation detail, not a file the user asked
            // MeedyaManager to organise.
            if mm_core::test_mode::is_test_mode_copy(&path) {
                continue;
            }

            // Check the file extension against the classify module
            let is_media = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| {
                    let c = classify::classify_by_extension(ext);
                    // Include Audio and Video files only; skip Image/Document/Archive
                    matches!(c.group, MediaGroup::Audio | MediaGroup::Video)
                });

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
// `std::env::set_var`/`remove_var` are `unsafe` in Edition 2024 because they
// race with concurrent readers.  The test below serialises its mutation behind
// ENV_LOCK and restores the variable from `Drop`, which is the discipline the
// marker asks for.  (The crate already has a blanket `#![allow(unsafe_code)]`
// for the FFI scaffolding; this attribute documents the intent locally.)
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Process-wide lock for `MM_CONFIG_DIR` within the mm-ffi test binary.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// RAII guard that points `MM_CONFIG_DIR` at a private directory for the
    /// lifetime of one test and cleans up on drop — including on an assertion
    /// panic, which a trailing `remove_var` would skip.
    ///
    /// Built by hand rather than with `tempfile` because mm-ffi has no
    /// dev-dependency on it and adding one would touch `Cargo.lock`.
    struct ConfigDirGuard {
        dir: PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl ConfigDirGuard {
        fn new(tag: &str) -> Self {
            let lock = ENV_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            // Nanosecond clock + a tag keeps concurrent runs from colliding.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("mm_ffi_{tag}_{nanos}"));
            std::fs::create_dir_all(&dir).unwrap();

            unsafe {
                std::env::set_var("MM_CONFIG_DIR", &dir);
            }
            Self { dir, _lock: lock }
        }

        fn path(&self) -> &Path {
            &self.dir
        }
    }

    impl Drop for ConfigDirGuard {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var("MM_CONFIG_DIR");
            }
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    /// Build a minimal but *real* WAV file on disk.
    ///
    /// lofty refuses a bare 44-byte header with no `data` payload, so the
    /// fixture carries 0.1 s of 8 kHz 16-bit mono silence (1,644 bytes total).
    fn write_wav_fixture(path: &Path) {
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

    /// The FFI write path must obey Test Mode exactly as the CLI does — the
    /// native UIs call straight into it, so an unguarded write here would let
    /// macOS/Windows clobber originals the user asked us not to touch.
    #[test]
    fn write_metadata_respects_test_mode() {
        let guard = ConfigDirGuard::new("testmode");

        let original = guard.path().join("track.wav");
        write_wav_fixture(&original);
        let before = std::fs::read(&original).unwrap();

        mm_core::test_mode::enable().expect("test mode must enable under the isolated config dir");

        write_metadata(
            original.display().to_string(),
            vec![TagEntry {
                key: "title".to_string(),
                value: "Diverted".to_string(),
            }],
        )
        .expect("write_metadata should succeed in Test Mode");

        assert_eq!(
            std::fs::read(&original).unwrap(),
            before,
            "Test Mode must leave the original byte-for-byte untouched"
        );

        let copy = guard.path().join("track_MeedyaManager.wav");
        assert!(copy.exists(), "Test Mode copy {} missing", copy.display());

        let tags = mm_core::metadata::extract_tags(&copy).unwrap();
        assert_eq!(
            tags.get("title").map(Vec::as_slice),
            Some(&["Diverted".to_string()][..]),
            "the copy must carry the new title"
        );
    }

    /// An unmapped key now reaches the caller as an error instead of a silent
    /// success — the behaviour change that matters most to the native UIs.
    #[test]
    fn write_metadata_rejects_unknown_key() {
        let guard = ConfigDirGuard::new("unknownkey");

        let p = guard.path().join("track.wav");
        write_wav_fixture(&p);
        let before = std::fs::read(&p).unwrap();

        let err = write_metadata(
            p.display().to_string(),
            vec![TagEntry {
                key: "bogus_key".to_string(),
                value: "1".to_string(),
            }],
        )
        .expect_err("an unmapped key must not report success");

        assert!(
            matches!(err, MmFfiError::Metadata(ref m) if m.contains("bogus_key")),
            "expected a Metadata error naming the key, got: {err:?}"
        );
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "a rejected write must not touch the file"
        );
    }
}
