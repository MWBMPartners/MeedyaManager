// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Settings Export/Import Bundle
//
// A `SettingsBundle` is a portable, self-contained JSON5 archive of an
// entire MeedyaManager installation's configuration.  It is designed for:
//
//   - Migrating settings to a new device or OS installation
//   - Backing up configuration before a major upgrade
//   - Sharing a curated configuration with other users
//
// Bundle contents:
//   - `version`          — MeedyaManager version that produced the bundle
//   - `exported_at`      — ISO 8601 UTC timestamp
//   - `settings`         — Full AppConfig (watch folders, rename rules, etc.)
//   - `custom_filetypes` — User's filetypes.json5 override (if any)
//   - `custom_tags`      — User's tags.json5 override (if any)
//
// API keys in `settings.providers` ARE included in the bundle by default
// (they are part of `AppConfig`).  Users must treat `.mmprofile` bundles
// as sensitive files and avoid sharing them publicly.
//
// Public API:
//   - SettingsBundle::capture()      — build a bundle from the current install
//   - SettingsBundle::export(path)   — write a bundle to a .mmprofile file
//   - SettingsBundle::import(path)   — read a bundle from a .mmprofile file
//   - SettingsBundle::apply()        — write bundle contents to config locations

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::error::{MmError, MmResult};

// ---------------------------------------------------------------------------
// Bundle format
// ---------------------------------------------------------------------------

/// A portable snapshot of an entire MeedyaManager configuration.
///
/// Serialised as JSON (the `.mmprofile` file format).  The `settings` field
/// is always present; the `custom_filetypes` and `custom_tags` fields are
/// `None` if the user has no override files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsBundle {
    /// The MeedyaManager version that created this bundle (from `CARGO_PKG_VERSION`).
    pub version: String,

    /// ISO 8601 UTC timestamp of when the bundle was created.
    pub exported_at: String,

    /// Full application configuration (replaces `settings.json5` on import).
    pub settings: AppConfig,

    /// Contents of the user's `filetypes.json5` override file, if it exists.
    /// `None` means the bundle was created without a user override (the
    /// compiled defaults will be used after import).
    pub custom_filetypes: Option<String>,

    /// Contents of the user's `tags.json5` override file, if it exists.
    /// `None` means no custom tag definitions (built-in defaults used).
    pub custom_tags: Option<String>,
}

// ---------------------------------------------------------------------------
// Bundle construction
// ---------------------------------------------------------------------------

impl SettingsBundle {
    /// Capture the current installation's configuration into a bundle.
    ///
    /// Reads the active `AppConfig`, and optionally the user's custom
    /// `filetypes.json5` / `tags.json5` override files if they exist.
    ///
    /// # Arguments
    /// * `config` — the currently loaded `AppConfig` (from `AppConfig::load()`).
    pub fn capture(config: AppConfig) -> Self {
        // Read custom filetypes override if present
        let custom_filetypes = read_user_override("filetypes.json5");

        // Read custom tags override if present
        let custom_tags = read_user_override("tags.json5");

        Self {
            // Embed the crate version at capture time (set by Cargo at compile time)
            version: env!("CARGO_PKG_VERSION").to_string(),
            // UTC timestamp in RFC 3339 format
            exported_at: Utc::now().to_rfc3339(),
            settings: config,
            custom_filetypes,
            custom_tags,
        }
    }

    // ── Serialisation ────────────────────────────────────────────────────────

    /// Serialise the bundle to a pretty-printed JSON string.
    ///
    /// The `.mmprofile` format uses standard JSON (not JSON5) so it can be
    /// read by any standard JSON parser, making it easy to inspect or process
    /// with external tools (jq, Python, etc.).
    pub fn to_json(&self) -> MmResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            MmError::Config(format!("cannot serialise settings bundle: {e}"))
        })
    }

    /// Parse a bundle from a JSON string.
    pub fn from_json(s: &str) -> MmResult<Self> {
        // Try standard JSON first, then JSON5 (for hand-edited files)
        serde_json::from_str(s)
            .or_else(|_| json5::from_str(s))
            .map_err(|e| {
                MmError::Config(format!("cannot parse settings bundle: {e}"))
            })
    }

    // ── File I/O ─────────────────────────────────────────────────────────────

    /// Write the bundle to the file at `path` (creates parent directories if
    /// needed).
    ///
    /// The file is written atomically: first to `<path>.meedya_tmp`, then
    /// renamed over `path`.  This prevents a corrupt or incomplete bundle on
    /// disk if the process is interrupted.
    ///
    /// # Errors
    /// Returns `MmError::Io` if the file cannot be written.
    pub fn export(&self, path: &Path) -> MmResult<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                MmError::Io(format!("cannot create directory '{}': {e}", parent.display()))
            })?;
        }

        // Serialise to JSON
        let json = self.to_json()?;

        // Write atomically: temp file → rename
        let tmp_path = {
            let mut t = path.to_path_buf();
            let mut name = t.file_name()
                .map(|n| n.to_os_string())
                .unwrap_or_else(|| "bundle".into());
            name.push(".meedya_tmp");
            t.set_file_name(name);
            t
        };

        std::fs::write(&tmp_path, json.as_bytes()).map_err(|e| {
            MmError::Io(format!("cannot write temp bundle '{}': {e}", tmp_path.display()))
        })?;

        std::fs::rename(&tmp_path, path).map_err(|e| {
            // Clean up temp on failure
            let _ = std::fs::remove_file(&tmp_path);
            MmError::Io(format!("cannot rename bundle into place: {e}"))
        })?;

        Ok(())
    }

    /// Read and parse a bundle from the file at `path`.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or is not a valid bundle.
    pub fn import(path: &Path) -> MmResult<Self> {
        if !path.exists() {
            return Err(MmError::Io(format!(
                "bundle file not found: '{}'", path.display()
            )));
        }

        let contents = std::fs::read_to_string(path).map_err(|e| {
            MmError::Io(format!("cannot read bundle '{}': {e}", path.display()))
        })?;

        Self::from_json(&contents)
    }

    // ── Apply ────────────────────────────────────────────────────────────────

    /// Apply the bundle to the current installation.
    ///
    /// Writes:
    /// - `settings.json5`   → platform settings directory
    /// - `filetypes.json5`  → platform config directory (if bundle contains one)
    /// - `tags.json5`       → platform config directory (if bundle contains one)
    ///
    /// Existing files are overwritten.  All writes are atomic.
    ///
    /// Returns a list of paths that were written.
    ///
    /// # Errors
    /// Returns the first write error encountered.
    pub fn apply(&self) -> MmResult<Vec<PathBuf>> {
        let mut written = Vec::new();

        // ── settings.json5 ──────────────────────────────────────────────────
        let settings_path = AppConfig::default_settings_path()?;
        if let Some(parent) = settings_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                MmError::Io(format!("cannot create config directory: {e}"))
            })?;
        }
        let settings_json = serde_json::to_string_pretty(&self.settings).map_err(|e| {
            MmError::Config(format!("cannot serialise settings: {e}"))
        })?;
        atomic_write(&settings_path, settings_json.as_bytes())?;
        written.push(settings_path);

        // ── custom filetypes.json5 ──────────────────────────────────────────
        if let Some(ref ft) = self.custom_filetypes {
            let ft_path = user_override_path("filetypes.json5")?;
            if let Some(parent) = ft_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    MmError::Io(format!("cannot create config directory: {e}"))
                })?;
            }
            atomic_write(&ft_path, ft.as_bytes())?;
            written.push(ft_path);
        }

        // ── custom tags.json5 ───────────────────────────────────────────────
        if let Some(ref tags) = self.custom_tags {
            let tags_path = user_override_path("tags.json5")?;
            if let Some(parent) = tags_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    MmError::Io(format!("cannot create config directory: {e}"))
                })?;
            }
            atomic_write(&tags_path, tags.as_bytes())?;
            written.push(tags_path);
        }

        Ok(written)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Try to read the user override file at `<config_dir>/meedyamanager/<name>`.
/// Returns `None` if the file does not exist or cannot be read.
fn read_user_override(filename: &str) -> Option<String> {
    let path = user_override_path(filename).ok()?;
    std::fs::read_to_string(&path).ok()
}

/// Resolve `<config_dir>/meedyamanager/<filename>`.
fn user_override_path(filename: &str) -> MmResult<PathBuf> {
    let config_root = dirs::config_dir()
        .ok_or_else(|| MmError::Config("cannot determine OS config directory".into()))?;
    Ok(config_root.join("meedyamanager").join(filename))
}

/// Write `data` to `path` atomically (via a temp file + rename).
fn atomic_write(path: &Path, data: &[u8]) -> MmResult<()> {
    // Build tmp path next to the target
    let tmp_path = {
        let mut t = path.to_path_buf();
        let mut name = t.file_name()
            .map(|n| n.to_os_string())
            .unwrap_or_else(|| "out".into());
        name.push(".meedya_tmp");
        t.set_file_name(name);
        t
    };

    std::fs::write(&tmp_path, data).map_err(|e| {
        MmError::Io(format!("cannot write '{}': {e}", tmp_path.display()))
    })?;

    std::fs::rename(&tmp_path, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        MmError::Io(format!("cannot rename '{}' to '{}': {e}", tmp_path.display(), path.display()))
    })?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn default_bundle() -> SettingsBundle {
        SettingsBundle::capture(AppConfig::default())
    }

    // ── Construction ─────────────────────────────────────────────────────────

    #[test]
    fn capture_has_version() {
        let bundle = default_bundle();
        assert!(!bundle.version.is_empty(), "version must not be empty");
        assert!(bundle.version.contains('.'), "version should be semver");
    }

    #[test]
    fn capture_has_timestamp() {
        let bundle = default_bundle();
        assert!(!bundle.exported_at.is_empty());
        // Should parse as a valid RFC 3339 timestamp
        chrono::DateTime::parse_from_rfc3339(&bundle.exported_at)
            .expect("exported_at must be a valid RFC 3339 timestamp");
    }

    // ── Serialisation round-trip ──────────────────────────────────────────────

    #[test]
    fn json_roundtrip() {
        let original = default_bundle();
        let json = original.to_json().expect("to_json must succeed");
        let parsed = SettingsBundle::from_json(&json).expect("from_json must succeed");
        assert_eq!(parsed.version, original.version);
        assert_eq!(parsed.settings.app_name, original.settings.app_name);
    }

    #[test]
    fn from_json_invalid_returns_error() {
        let result = SettingsBundle::from_json("not valid json at all !!!");
        assert!(result.is_err());
    }

    // ── File export / import ─────────────────────────────────────────────────

    #[test]
    fn export_creates_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mmprofile");
        let bundle = default_bundle();
        bundle.export(&path).expect("export must succeed");
        assert!(path.exists(), "exported file must exist");
    }

    #[test]
    fn import_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mmprofile");
        let original = default_bundle();
        original.export(&path).unwrap();
        let imported = SettingsBundle::import(&path).expect("import must succeed");
        assert_eq!(imported.version, original.version);
        assert_eq!(imported.settings.dry_run, original.settings.dry_run);
    }

    #[test]
    fn import_nonexistent_file_returns_error() {
        let result = SettingsBundle::import(Path::new("/tmp/meedya_no_such_bundle.mmprofile"));
        assert!(result.is_err());
    }

    #[test]
    fn no_tmp_file_left_after_export() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.mmprofile");
        default_bundle().export(&path).unwrap();
        // No temp files should remain
        let tmp_path = dir.path().join("test.mmprofile.meedya_tmp");
        assert!(!tmp_path.exists(), "no temp file should remain after export");
    }

    // ── Optional fields ───────────────────────────────────────────────────────

    #[test]
    fn bundle_without_custom_overrides() {
        // When no user override files exist, these fields should be None
        // (or Some if the test runs on a machine that has them — we just check it doesn't panic)
        let bundle = default_bundle();
        // These are Option<String> — either is valid in a test environment
        let _ = bundle.custom_filetypes;
        let _ = bundle.custom_tags;
    }

    #[test]
    fn bundle_with_custom_filetypes() {
        let bundle = SettingsBundle {
            version: "1.0.0".into(),
            exported_at: "2026-01-01T00:00:00+00:00".into(),
            settings: AppConfig::default(),
            custom_filetypes: Some("{ \"audio\": [] }".into()),
            custom_tags: None,
        };
        let json = bundle.to_json().unwrap();
        let parsed = SettingsBundle::from_json(&json).unwrap();
        assert_eq!(parsed.custom_filetypes, Some("{ \"audio\": [] }".into()));
        assert!(parsed.custom_tags.is_none());
    }
}
