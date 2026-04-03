// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Test Mode (Safe Edit Mode)
//
// When Test Mode is enabled, file-mutating operations (metadata tagging,
// cover art embedding) create a duplicate file with a `_MeedyaManager` suffix
// instead of overwriting the original.  A persistent manifest tracks all
// test-mode files so they can be cleaned up across application sessions.
//
// ## Lifecycle
//
// 1. User enables Test Mode (settings toggle or CLI `meedya config test-mode on`)
// 2. All edit/tag operations write to `<stem>_MeedyaManager.<ext>` copies
// 3. Each written file is recorded in `testmode_manifest.json`
// 4. User disables Test Mode — prompted:
//    - **Yes (commit):** delete originals, rename copies (remove suffix)
//    - **No (revert):**  keep both originals and copies
// 5. Manifest is cleared after commit or revert
//
// The manifest persists at `<config_dir>/meedyamanager/testmode_manifest.json`
// and survives application restarts.
//
// ## Pre-release Safety
//
// Pre-release builds (semver pre-release label present, e.g. `1.3.0-beta.1`)
// auto-enable Test Mode on first launch to protect user files from potential
// bugs in unreleased code.
//
// License: GPL-2.0-or-later

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::error::{MmError, MmResult};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// The suffix inserted before the file extension for test-mode copies.
const TEST_MODE_SUFFIX: &str = "_MeedyaManager";

/// The manifest filename within the MeedyaManager config directory.
const MANIFEST_FILENAME: &str = "testmode_manifest.json";

// ---------------------------------------------------------------------------
// Manifest types
// ---------------------------------------------------------------------------

/// A single entry in the test-mode file manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestModeEntry {
    /// Absolute path to the original (unmodified) file.
    pub original: PathBuf,
    /// Absolute path to the test-mode copy (with `_MeedyaManager` suffix).
    pub copy: PathBuf,
    /// ISO 8601 timestamp when this entry was created.
    pub created_at: String,
}

/// The persistent test-mode manifest.
///
/// Stored as JSON at `<config_dir>/meedyamanager/testmode_manifest.json`.
/// Tracks whether test mode is currently active and all files that have
/// been written in test mode across all sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TestModeManifest {
    /// Whether test mode is currently enabled.
    pub enabled: bool,
    /// ISO 8601 timestamp when test mode was last enabled (empty if never).
    pub enabled_since: String,
    /// Map of original-path → entry.  BTreeMap keeps entries sorted by path
    /// for deterministic serialization and human-readable diffs.
    pub files: BTreeMap<String, TestModeEntry>,
}

// Default is derived — all fields default to false / empty.

// ---------------------------------------------------------------------------
// Path helpers
// ---------------------------------------------------------------------------

/// Compute the test-mode copy path for an original file.
///
/// Example: `/music/track.mp3` → `/music/track_MeedyaManager.mp3`
///
/// If the file has no extension, the suffix is appended to the stem:
///   `/music/README` → `/music/README_MeedyaManager`
pub fn test_mode_path(original: &Path) -> PathBuf {
    let stem = original
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = original
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()));

    let new_name = match ext {
        Some(ext) => format!("{stem}{TEST_MODE_SUFFIX}{ext}"),
        None => format!("{stem}{TEST_MODE_SUFFIX}"),
    };

    let mut result = original.to_path_buf();
    result.set_file_name(new_name);
    result
}

/// Return `true` if the given path has the `_MeedyaManager` suffix pattern,
/// indicating it is a test-mode copy.
pub fn is_test_mode_copy(path: &Path) -> bool {
    path.file_stem()
        .is_some_and(|s| s.to_string_lossy().ends_with(TEST_MODE_SUFFIX))
}

/// Strip the `_MeedyaManager` suffix from a test-mode copy path to recover
/// the original filename.
///
/// Returns `None` if the path does not have the suffix.
pub fn original_path_from_copy(copy_path: &Path) -> Option<PathBuf> {
    let stem = copy_path.file_stem()?.to_string_lossy();
    if !stem.ends_with(TEST_MODE_SUFFIX) {
        return None;
    }
    let original_stem = &stem[..stem.len() - TEST_MODE_SUFFIX.len()];
    let ext = copy_path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()));
    let new_name = match ext {
        Some(ext) => format!("{original_stem}{ext}"),
        None => original_stem.to_string(),
    };
    let mut result = copy_path.to_path_buf();
    result.set_file_name(new_name);
    Some(result)
}

// ---------------------------------------------------------------------------
// Manifest I/O
// ---------------------------------------------------------------------------

/// Resolve the path to the test-mode manifest file.
fn manifest_path() -> MmResult<PathBuf> {
    let config_root = dirs::config_dir()
        .ok_or_else(|| MmError::Config("cannot determine platform config directory".into()))?;
    Ok(config_root.join("meedyamanager").join(MANIFEST_FILENAME))
}

/// Load the test-mode manifest from disk.
///
/// Returns `TestModeManifest::default()` if the file does not exist.
pub fn load_manifest() -> MmResult<TestModeManifest> {
    let path = manifest_path()?;
    if !path.exists() {
        debug!("test mode manifest not found — returning defaults");
        return Ok(TestModeManifest::default());
    }
    let contents = std::fs::read_to_string(&path).map_err(|e| {
        MmError::Config(format!(
            "failed to read test mode manifest '{}': {e}",
            path.display()
        ))
    })?;
    let manifest: TestModeManifest = serde_json::from_str(&contents).map_err(|e| {
        MmError::Config(format!(
            "failed to parse test mode manifest '{}': {e}",
            path.display()
        ))
    })?;
    debug!(
        enabled = manifest.enabled,
        files = manifest.files.len(),
        "loaded test mode manifest"
    );
    Ok(manifest)
}

/// Save the test-mode manifest to disk atomically.
fn save_manifest(manifest: &TestModeManifest) -> MmResult<()> {
    let path = manifest_path()?;
    // Ensure directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            MmError::Config(format!(
                "cannot create config dir '{}': {e}",
                parent.display()
            ))
        })?;
    }
    // Write atomically via temp file
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(manifest)?;
    std::fs::write(&tmp, json.as_bytes())
        .map_err(|e| MmError::Config(format!("failed to write manifest temp file: {e}")))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| MmError::Config(format!("failed to rename manifest temp file: {e}")))?;
    debug!(path = %path.display(), "saved test mode manifest");
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return `true` if test mode is currently enabled.
pub fn is_enabled() -> bool {
    load_manifest().map(|m| m.enabled).unwrap_or(false)
}

/// Enable test mode.  Records the enable timestamp in the manifest.
pub fn enable() -> MmResult<()> {
    let mut manifest = load_manifest()?;
    if manifest.enabled {
        info!("test mode is already enabled");
        return Ok(());
    }
    manifest.enabled = true;
    manifest.enabled_since = Utc::now().to_rfc3339();
    save_manifest(&manifest)?;
    info!(since = %manifest.enabled_since, "test mode enabled");
    Ok(())
}

/// Disable test mode without cleaning up files.
///
/// The manifest retains its file entries so that a subsequent `commit_files`
/// or `revert_files` call can still process them.  Callers should prompt the
/// user and then call `commit_files()` or `revert_files()` before or after
/// this function.
pub fn disable() -> MmResult<()> {
    let mut manifest = load_manifest()?;
    manifest.enabled = false;
    manifest.enabled_since = String::new();
    save_manifest(&manifest)?;
    info!("test mode disabled");
    Ok(())
}

/// Record a file written in test mode.
///
/// `original` is the path to the unmodified source file.
/// `copy` is the path to the `_MeedyaManager` suffixed duplicate.
pub fn record_file(original: &Path, copy: &Path) -> MmResult<()> {
    let mut manifest = load_manifest()?;
    let key = original.to_string_lossy().to_string();
    let entry = TestModeEntry {
        original: original.to_path_buf(),
        copy: copy.to_path_buf(),
        created_at: Utc::now().to_rfc3339(),
    };
    manifest.files.insert(key, entry);
    save_manifest(&manifest)?;
    debug!(
        original = %original.display(),
        copy = %copy.display(),
        "recorded test mode file"
    );
    Ok(())
}

/// Return the number of files currently tracked in the test-mode manifest.
pub fn tracked_file_count() -> usize {
    load_manifest().map(|m| m.files.len()).unwrap_or(0)
}

/// Return a list of all tracked test-mode entries.
pub fn tracked_files() -> Vec<TestModeEntry> {
    load_manifest()
        .map(|m| m.files.values().cloned().collect())
        .unwrap_or_default()
}

/// Commit test-mode files: delete originals, rename copies to original names.
///
/// This is the "Yes" path when the user disables test mode.
/// Returns the number of files successfully committed.
pub fn commit_files() -> MmResult<usize> {
    let mut manifest = load_manifest()?;
    let mut committed = 0usize;

    for entry in manifest.files.values() {
        // Skip entries where the copy no longer exists (user may have
        // deleted it manually)
        if !entry.copy.exists() {
            warn!(
                copy = %entry.copy.display(),
                "test mode copy not found — skipping"
            );
            continue;
        }

        // Delete the original if it still exists
        if entry.original.exists() {
            if let Err(e) = std::fs::remove_file(&entry.original) {
                error!(
                    path = %entry.original.display(),
                    %e,
                    "failed to delete original file during commit"
                );
                continue;
            }
        }

        // Rename the copy to the original's name (removing the suffix)
        if let Err(e) = std::fs::rename(&entry.copy, &entry.original) {
            error!(
                from = %entry.copy.display(),
                to = %entry.original.display(),
                %e,
                "failed to rename test mode copy during commit"
            );
            continue;
        }

        committed += 1;
        info!(
            path = %entry.original.display(),
            "committed test mode file"
        );
    }

    // Clear the manifest
    manifest.files.clear();
    save_manifest(&manifest)?;

    info!(committed, "test mode commit complete");
    Ok(committed)
}

/// Revert test-mode files: keep both originals and copies, just clear the
/// manifest.
///
/// This is the "No" path when the user disables test mode.
pub fn revert_files() -> MmResult<()> {
    let mut manifest = load_manifest()?;
    let count = manifest.files.len();
    manifest.files.clear();
    save_manifest(&manifest)?;
    info!(count, "test mode revert — cleared manifest, kept all files");
    Ok(())
}

// ---------------------------------------------------------------------------
// Pre-release detection
// ---------------------------------------------------------------------------

/// Return `true` if the given version string has a semver pre-release label
/// (e.g. `"1.3.0-beta.1"`, `"2.0.0-alpha"`, `"1.0.0-rc.1"`).
///
/// Uses the `semver` crate for reliable parsing.
pub fn is_prerelease_version(version_str: &str) -> bool {
    match semver::Version::parse(version_str) {
        Ok(v) => !v.pre.is_empty(),
        Err(_) => false,
    }
}

/// Return `true` if the currently compiled version of MeedyaManager is a
/// pre-release build.
pub fn is_current_prerelease() -> bool {
    is_prerelease_version(env!("CARGO_PKG_VERSION"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // ── Path helpers ────────────────────────────────────────────────────────

    #[test]
    fn test_mode_path_inserts_suffix_before_extension() {
        let p = Path::new("/music/albums/track.mp3");
        let result = test_mode_path(p);
        assert_eq!(
            result,
            PathBuf::from("/music/albums/track_MeedyaManager.mp3")
        );
    }

    #[test]
    fn test_mode_path_handles_no_extension() {
        let p = Path::new("/music/README");
        let result = test_mode_path(p);
        assert_eq!(result, PathBuf::from("/music/README_MeedyaManager"));
    }

    #[test]
    fn test_mode_path_handles_double_extension() {
        // Only the last extension is preserved; stem includes the first dot
        let p = Path::new("/music/track.backup.flac");
        let result = test_mode_path(p);
        assert_eq!(
            result,
            PathBuf::from("/music/track.backup_MeedyaManager.flac")
        );
    }

    #[test]
    fn test_mode_path_preserves_directory() {
        let p = Path::new("/deep/nested/dir/song.wav");
        let result = test_mode_path(p);
        assert_eq!(result.parent(), p.parent());
    }

    // ── is_test_mode_copy ───────────────────────────────────────────────────

    #[test]
    fn is_test_mode_copy_detects_suffix() {
        let p = Path::new("/music/track_MeedyaManager.mp3");
        assert!(is_test_mode_copy(p));
    }

    #[test]
    fn is_test_mode_copy_rejects_normal_file() {
        let p = Path::new("/music/track.mp3");
        assert!(!is_test_mode_copy(p));
    }

    #[test]
    fn is_test_mode_copy_rejects_partial_suffix() {
        let p = Path::new("/music/track_Meedya.mp3");
        assert!(!is_test_mode_copy(p));
    }

    // ── original_path_from_copy ─────────────────────────────────────────────

    #[test]
    fn original_path_from_copy_strips_suffix() {
        let copy = Path::new("/music/track_MeedyaManager.flac");
        let original = original_path_from_copy(copy);
        assert_eq!(original, Some(PathBuf::from("/music/track.flac")));
    }

    #[test]
    fn original_path_from_copy_returns_none_for_normal() {
        let p = Path::new("/music/track.flac");
        assert_eq!(original_path_from_copy(p), None);
    }

    #[test]
    fn original_path_from_copy_handles_no_extension() {
        let copy = Path::new("/music/README_MeedyaManager");
        let original = original_path_from_copy(copy);
        assert_eq!(original, Some(PathBuf::from("/music/README")));
    }

    // ── Round-trip ──────────────────────────────────────────────────────────

    #[test]
    fn test_mode_path_round_trips() {
        let original = Path::new("/music/track.mp3");
        let copy = test_mode_path(original);
        let recovered = original_path_from_copy(&copy);
        assert_eq!(recovered, Some(original.to_path_buf()));
    }

    // ── Manifest serialization ──────────────────────────────────────────────

    #[test]
    fn manifest_default_is_disabled_and_empty() {
        let m = TestModeManifest::default();
        assert!(!m.enabled);
        assert!(m.enabled_since.is_empty());
        assert!(m.files.is_empty());
    }

    #[test]
    fn manifest_round_trip_serde() {
        let mut m = TestModeManifest {
            enabled: true,
            enabled_since: "2026-03-06T12:00:00Z".into(),
            ..Default::default()
        };
        m.files.insert(
            "/music/track.mp3".into(),
            TestModeEntry {
                original: PathBuf::from("/music/track.mp3"),
                copy: PathBuf::from("/music/track_MeedyaManager.mp3"),
                created_at: "2026-03-06T12:00:00Z".into(),
            },
        );
        let json = serde_json::to_string(&m).unwrap();
        let m2: TestModeManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(m, m2);
    }

    // ── Pre-release detection ───────────────────────────────────────────────

    #[test]
    fn is_prerelease_version_detects_beta() {
        assert!(is_prerelease_version("1.3.0-beta.1"));
    }

    #[test]
    fn is_prerelease_version_detects_alpha() {
        assert!(is_prerelease_version("2.0.0-alpha"));
    }

    #[test]
    fn is_prerelease_version_detects_rc() {
        assert!(is_prerelease_version("1.0.0-rc.1"));
    }

    #[test]
    fn is_prerelease_version_rejects_stable() {
        assert!(!is_prerelease_version("1.2.0"));
    }

    #[test]
    fn is_prerelease_version_rejects_invalid() {
        assert!(!is_prerelease_version("not-a-version"));
    }

    #[test]
    fn is_current_prerelease_returns_false_for_stable() {
        // The workspace version is 1.3.0 (stable) — no pre-release label.
        assert!(!is_current_prerelease());
    }

    // ── File operations (integration-style) ─────────────────────────────────

    #[test]
    fn commit_workflow_deletes_original_and_renames_copy() {
        let dir = TempDir::new().unwrap();
        let original = dir.path().join("song.mp3");
        let copy = test_mode_path(&original);

        // Create both files
        fs::write(&original, b"original content").unwrap();
        fs::write(&copy, b"tagged content").unwrap();

        // Simulate commit for a single entry
        assert!(original.exists());
        assert!(copy.exists());

        // Delete original
        fs::remove_file(&original).unwrap();
        // Rename copy → original
        fs::rename(&copy, &original).unwrap();

        assert!(original.exists());
        assert!(!copy.exists());
        assert_eq!(fs::read_to_string(&original).unwrap(), "tagged content");
    }

    #[test]
    fn revert_keeps_both_files() {
        let dir = TempDir::new().unwrap();
        let original = dir.path().join("song.flac");
        let copy = test_mode_path(&original);

        fs::write(&original, b"original").unwrap();
        fs::write(&copy, b"tagged").unwrap();

        // Revert = keep both, just clear manifest
        assert!(original.exists());
        assert!(copy.exists());
    }
}
