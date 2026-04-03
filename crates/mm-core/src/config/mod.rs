// (C) 2025-2026 MWBM Partners Ltd
//
// Configuration module for MeedyaManager.
//
// Loads application configuration from a JSON5 file (`settings.json5`) located
// in the platform-appropriate config directory, then applies overrides from a
// `.env` file (if present) via the `dotenvy` crate.
//
// The configuration is strongly typed through `AppConfig` and its nested
// section structs, all of which implement `Default` for sensible out-of-the-box
// behaviour. The module exposes two entry points:
//
//   - `AppConfig::load()`          — loads from the platform default location
//   - `AppConfig::load_from(path)` — loads from an explicit file path
//
// License: GPL-2.0-or-later

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{MmError, MmResult};

// ---------------------------------------------------------------------------
// Top-level application configuration
// ---------------------------------------------------------------------------

/// Root configuration struct for MeedyaManager.
///
/// Each nested section maps to a logical subsystem (watching, renaming,
/// logging, metadata providers). All fields carry defaults so a completely
/// empty JSON5 file (or no file at all) still yields a usable config.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    /// Human-readable application name (informational only)
    pub app_name: String,

    /// Whether to run the application in dry-run / preview mode globally
    pub dry_run: bool,

    /// Test Mode — when enabled, edit/tag operations create `_MeedyaManager`
    /// suffixed copies instead of modifying originals.  Managed via the
    /// `test_mode` module; this field reflects the persisted state.
    #[serde(default)]
    pub test_mode: bool,

    /// File-system watching settings
    pub watch: WatchConfig,

    /// Rename / organise settings
    pub rename: RenameConfig,

    /// Logging and diagnostics settings
    pub logging: LoggingConfig,

    /// Metadata provider settings (API keys, enabled providers, etc.)
    pub providers: ProviderConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: "MeedyaManager".to_string(),
            dry_run: false,
            test_mode: false,
            watch: WatchConfig::default(),
            rename: RenameConfig::default(),
            logging: LoggingConfig::default(),
            providers: ProviderConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Watch configuration
// ---------------------------------------------------------------------------

/// Configuration for the file-system watcher subsystem.
///
/// `folders` lists the directories to monitor. `poll_interval_secs` controls
/// the fallback polling frequency when native events are unavailable.
/// `recursive` determines whether subdirectories are included.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct WatchConfig {
    /// Directories to watch for new/changed media files
    pub folders: Vec<PathBuf>,

    /// Whether to watch subdirectories recursively
    pub recursive: bool,

    /// Polling interval in seconds (used when native FS events are unavailable)
    pub poll_interval_secs: u64,

    /// Debounce window in milliseconds — events within this window are merged
    pub debounce_ms: u64,

    /// File extensions to include (empty = all supported media types)
    pub include_extensions: Vec<String>,

    /// File extensions to explicitly exclude
    pub exclude_extensions: Vec<String>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            // No default watch folders — user must configure at least one
            folders: Vec::new(),
            // Recursive watching is on by default
            recursive: true,
            // 5-second poll interval as a reasonable default
            poll_interval_secs: 5,
            // 200 ms debounce window
            debounce_ms: 200,
            // Empty means "all supported extensions"
            include_extensions: Vec::new(),
            // Nothing excluded by default
            exclude_extensions: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Rename configuration
// ---------------------------------------------------------------------------

/// Configuration for the file renaming / organisation subsystem.
///
/// Controls the template pattern used to build destination paths, conflict
/// resolution strategy, and whether dry-run mode is active for renames.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct RenameConfig {
    /// MusicBee-style template for building the destination file path.
    /// Uses `<Tag>` placeholders and `$If()` / `$And()` / `$Or()` functions.
    pub template: String,

    /// Root output directory where organised files are placed
    pub output_dir: Option<PathBuf>,

    /// Strategy when a destination file already exists:
    /// "skip", "overwrite", "rename" (append counter), "ask"
    pub conflict_strategy: String,

    /// Whether to create missing directories in the output path
    pub create_dirs: bool,

    /// Whether to preserve the original file (copy) instead of moving
    pub copy_mode: bool,

    /// Named rules for conditional template selection.
    /// Rules are evaluated in priority order; the first match wins.
    /// If no rules match, the `template` field is used as fallback.
    #[serde(default)]
    pub rules: Vec<crate::rule_engine::Rule>,

    /// Behaviour when a tag is missing during template evaluation:
    /// "empty" (default), "literal" (show `<TagName>`), "error"
    #[serde(default = "default_missing_tag_mode")]
    pub missing_tag_mode: String,
}

/// Default value for `missing_tag_mode` — returns "empty"
fn default_missing_tag_mode() -> String {
    "empty".to_string()
}

impl Default for RenameConfig {
    fn default() -> Self {
        Self {
            // Sensible default template: Artist / Album / Track Title
            template: "<Artist>/<Album>/<Title>".to_string(),
            // No default output directory — user should configure
            output_dir: None,
            // Skip conflicts by default (safest choice)
            conflict_strategy: "skip".to_string(),
            // Automatically create missing directories
            create_dirs: true,
            // Move files by default (not copy)
            copy_mode: false,
            // No conditional rules by default — use the template field
            rules: Vec::new(),
            // Missing tags render as empty strings by default
            missing_tag_mode: default_missing_tag_mode(),
        }
    }
}

// ---------------------------------------------------------------------------
// Logging configuration
// ---------------------------------------------------------------------------

/// Configuration for structured logging and diagnostics.
///
/// Supports console output, file output, and configurable verbosity levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LoggingConfig {
    /// Minimum log level: "trace", "debug", "info", "warn", "error"
    pub level: String,

    /// Whether to emit logs to the console (stdout/stderr)
    pub console: bool,

    /// Optional path to a log file (None = no file logging)
    pub file: Option<PathBuf>,

    /// Maximum log file size in bytes before rotation (default 10 MB)
    pub max_file_size_bytes: u64,

    /// Number of rotated log files to keep
    pub max_rotated_files: u32,

    /// Whether to redact personally-identifiable information (PII) in logs
    pub redact_pii: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            // Info level by default — not too noisy, not too quiet
            level: "info".to_string(),
            // Console output is on by default
            console: true,
            // No log file by default
            file: None,
            // 10 MB default max file size
            max_file_size_bytes: 10 * 1024 * 1024,
            // Keep 3 rotated files
            max_rotated_files: 3,
            // PII redaction on by default for safety
            redact_pii: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Provider configuration
// ---------------------------------------------------------------------------

/// Configuration for metadata lookup providers.
///
/// Controls which providers are enabled and stores API keys. Keys can be
/// set in the JSON5 file or overridden via environment variables (preferred
/// for secrets).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ProviderConfig {
    /// Enable MusicBrainz lookups
    pub musicbrainz_enabled: bool,

    /// Enable Discogs lookups
    pub discogs_enabled: bool,

    /// Discogs personal access token (prefer MM_DISCOGS_TOKEN env var)
    pub discogs_token: Option<String>,

    /// Enable Spotify metadata lookups
    pub spotify_enabled: bool,

    /// Spotify client ID (prefer MM_SPOTIFY_CLIENT_ID env var)
    pub spotify_client_id: Option<String>,

    /// Spotify client secret (prefer MM_SPOTIFY_CLIENT_SECRET env var)
    pub spotify_client_secret: Option<String>,

    /// Enable TMDb (The Movie Database) lookups
    pub tmdb_enabled: bool,

    /// TMDb API key (prefer MM_TMDB_API_KEY env var)
    pub tmdb_api_key: Option<String>,

    /// Enable AcoustID fingerprint lookups
    pub acoustid_enabled: bool,

    /// AcoustID API key (prefer MM_ACOUSTID_API_KEY env var)
    pub acoustid_api_key: Option<String>,

    /// Global request timeout for provider API calls, in seconds
    pub request_timeout_secs: u64,

    /// Maximum concurrent provider requests
    pub max_concurrent_requests: usize,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            // MusicBrainz is free and open — enabled by default
            musicbrainz_enabled: true,
            // Discogs requires a token — disabled by default
            discogs_enabled: false,
            discogs_token: None,
            // Spotify requires OAuth credentials — disabled by default
            spotify_enabled: false,
            spotify_client_id: None,
            spotify_client_secret: None,
            // TMDb requires an API key — disabled by default
            tmdb_enabled: false,
            tmdb_api_key: None,
            // AcoustID requires an API key — disabled by default
            acoustid_enabled: false,
            acoustid_api_key: None,
            // 30-second default timeout for API calls
            request_timeout_secs: 30,
            // Up to 4 concurrent provider requests
            max_concurrent_requests: 4,
        }
    }
}

// ---------------------------------------------------------------------------
// Loading logic
// ---------------------------------------------------------------------------

impl AppConfig {
    /// Load configuration from the platform-default config directory.
    ///
    /// Resolves the path as: `<platform config dir>/MeedyaManager/settings.json5`
    ///
    /// Platform config directories (via the `dirs` crate):
    ///   - macOS:   `~/Library/Application Support/MeedyaManager/`
    ///   - Linux:   `~/.config/MeedyaManager/`
    ///   - Windows: `C:\Users\<user>\AppData\Roaming\MeedyaManager\`
    ///
    /// If the file does not exist, returns `AppConfig::default()` with a
    /// warning logged. After loading the JSON5 file, `.env` overrides are
    /// applied on top.
    pub fn load() -> MmResult<Self> {
        // Resolve the platform-specific config directory
        let config_dir = dirs::config_dir().ok_or_else(|| {
            MmError::Config("unable to determine platform config directory".to_string())
        })?;

        // Build the full path: <config_dir>/MeedyaManager/settings.json5
        let settings_path = config_dir.join("MeedyaManager").join("settings.json5");

        info!(
            path = %settings_path.display(),
            "Loading configuration from platform default location"
        );

        // Delegate to load_from, which handles missing-file fallback
        Self::load_from(&settings_path)
    }

    /// Load configuration from a specific file path.
    ///
    /// If the file does not exist, returns `AppConfig::default()` with a
    /// warning. If the file exists but is unparseable, returns an error.
    /// After parsing, `.env` overrides are applied.
    pub fn load_from(path: &Path) -> MmResult<Self> {
        // Start with the default configuration
        let mut config = if path.exists() {
            info!(path = %path.display(), "Reading configuration file");

            // Read the raw file contents
            let contents = std::fs::read_to_string(path).map_err(|e| {
                MmError::Config(format!(
                    "failed to read config file '{}': {}",
                    path.display(),
                    e
                ))
            })?;

            // Parse JSON5 into the strongly-typed AppConfig struct.
            // JSON5 is a superset of JSON that allows comments, trailing
            // commas, unquoted keys, and other conveniences.
            json5::from_str::<Self>(&contents).map_err(|e| {
                MmError::Config(format!(
                    "failed to parse config file '{}': {}",
                    path.display(),
                    e
                ))
            })?
        } else {
            warn!(
                path = %path.display(),
                "Configuration file not found — using defaults"
            );
            Self::default()
        };

        // Apply .env overrides on top of the loaded (or default) config
        Self::apply_env_overrides(&mut config);

        // Run validation (warnings only for non-critical issues)
        Self::validate(&config);

        debug!(?config, "Final configuration loaded");

        Ok(config)
    }

    /// Apply environment variable overrides from `.env` and the process
    /// environment.
    ///
    /// The `dotenvy` crate loads `.env` into the process environment.
    /// We then read specific `MM_*` variables and override the corresponding
    /// config fields. This is the recommended way to supply secrets (API keys)
    /// without putting them in the JSON5 file.
    fn apply_env_overrides(config: &mut Self) {
        // Attempt to load .env — it is perfectly fine if the file is missing
        match dotenvy::dotenv() {
            Ok(path) => {
                info!(path = %path.display(), "Loaded .env file");
            }
            Err(_) => {
                debug!("No .env file found — using process environment only");
            }
        }

        // --- Top-level overrides ---

        // MM_DRY_RUN: override the global dry-run flag
        if let Ok(val) = std::env::var("MM_DRY_RUN") {
            config.dry_run = val == "1" || val.eq_ignore_ascii_case("true");
            debug!(dry_run = config.dry_run, "MM_DRY_RUN override applied");
        }

        // MM_TEST_MODE: override the test-mode flag
        if let Ok(val) = std::env::var("MM_TEST_MODE") {
            config.test_mode = val == "1" || val.eq_ignore_ascii_case("true");
            debug!(
                test_mode = config.test_mode,
                "MM_TEST_MODE override applied"
            );
        }

        // --- Logging overrides ---

        // MM_LOG_LEVEL: override the log level
        if let Ok(val) = std::env::var("MM_LOG_LEVEL") {
            config.logging.level = val;
            debug!(level = %config.logging.level, "MM_LOG_LEVEL override applied");
        }

        // --- Watch overrides ---

        // MM_WATCH_RECURSIVE: override recursive watching
        if let Ok(val) = std::env::var("MM_WATCH_RECURSIVE") {
            config.watch.recursive = val == "1" || val.eq_ignore_ascii_case("true");
            debug!(
                recursive = config.watch.recursive,
                "MM_WATCH_RECURSIVE override applied"
            );
        }

        // MM_WATCH_POLL_INTERVAL: override poll interval
        if let Ok(val) = std::env::var("MM_WATCH_POLL_INTERVAL") {
            if let Ok(secs) = val.parse::<u64>() {
                config.watch.poll_interval_secs = secs;
                debug!(
                    poll_interval_secs = secs,
                    "MM_WATCH_POLL_INTERVAL override applied"
                );
            }
        }

        // --- Provider API key overrides (secrets) ---

        // MM_DISCOGS_TOKEN: Discogs personal access token
        if let Ok(val) = std::env::var("MM_DISCOGS_TOKEN") {
            config.providers.discogs_token = Some(val);
            debug!("MM_DISCOGS_TOKEN override applied");
        }

        // MM_SPOTIFY_CLIENT_ID: Spotify OAuth client ID
        if let Ok(val) = std::env::var("MM_SPOTIFY_CLIENT_ID") {
            config.providers.spotify_client_id = Some(val);
            debug!("MM_SPOTIFY_CLIENT_ID override applied");
        }

        // MM_SPOTIFY_CLIENT_SECRET: Spotify OAuth client secret
        if let Ok(val) = std::env::var("MM_SPOTIFY_CLIENT_SECRET") {
            config.providers.spotify_client_secret = Some(val);
            debug!("MM_SPOTIFY_CLIENT_SECRET override applied");
        }

        // MM_TMDB_API_KEY: TMDb API key
        if let Ok(val) = std::env::var("MM_TMDB_API_KEY") {
            config.providers.tmdb_api_key = Some(val);
            debug!("MM_TMDB_API_KEY override applied");
        }

        // MM_ACOUSTID_API_KEY: AcoustID API key
        if let Ok(val) = std::env::var("MM_ACOUSTID_API_KEY") {
            config.providers.acoustid_api_key = Some(val);
            debug!("MM_ACOUSTID_API_KEY override applied");
        }

        // --- Rename overrides ---

        // MM_RENAME_TEMPLATE: override the rename template
        if let Ok(val) = std::env::var("MM_RENAME_TEMPLATE") {
            config.rename.template = val;
            debug!(
                template = %config.rename.template,
                "MM_RENAME_TEMPLATE override applied"
            );
        }

        // MM_RENAME_CONFLICT: override the conflict strategy
        if let Ok(val) = std::env::var("MM_RENAME_CONFLICT") {
            config.rename.conflict_strategy = val;
            debug!(
                strategy = %config.rename.conflict_strategy,
                "MM_RENAME_CONFLICT override applied"
            );
        }
    }

    /// Validate the loaded configuration, emitting warnings for potential
    /// issues.
    ///
    /// Validation is intentionally lenient — we warn about problems but
    /// do not fail, because the user may be running in a mode that does
    /// not require the problematic settings (e.g. CLI --help).
    fn validate(config: &Self) {
        // Warn if no watch folders are configured
        if config.watch.folders.is_empty() {
            warn!("No watch folders configured — the watcher will have nothing to monitor");
        }

        // Warn about non-existent watch folders (but do not error)
        for folder in &config.watch.folders {
            if !folder.exists() {
                warn!(
                    path = %folder.display(),
                    "Watch folder does not exist — it will be skipped until created"
                );
            }
        }

        // Validate the conflict strategy is one of the known values
        let valid_strategies = ["skip", "overwrite", "rename", "ask"];
        if !valid_strategies.contains(&config.rename.conflict_strategy.as_str()) {
            warn!(
                strategy = %config.rename.conflict_strategy,
                "Unknown conflict strategy — falling back to 'skip' at runtime"
            );
        }

        // Validate the log level string
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&config.logging.level.as_str()) {
            warn!(
                level = %config.logging.level,
                "Unknown log level — falling back to 'info' at runtime"
            );
        }

        // Warn if poll interval is suspiciously low (could cause high CPU)
        if config.watch.poll_interval_secs == 0 {
            warn!("Watch poll interval is 0 — this may cause excessive CPU usage");
        }

        // Warn if a provider is enabled but its API key is missing
        if config.providers.discogs_enabled && config.providers.discogs_token.is_none() {
            warn!("Discogs is enabled but no token is configured (set MM_DISCOGS_TOKEN)");
        }
        if config.providers.spotify_enabled
            && (config.providers.spotify_client_id.is_none()
                || config.providers.spotify_client_secret.is_none())
        {
            warn!(
                "Spotify is enabled but credentials are incomplete (set MM_SPOTIFY_CLIENT_ID and MM_SPOTIFY_CLIENT_SECRET)"
            );
        }
        if config.providers.tmdb_enabled && config.providers.tmdb_api_key.is_none() {
            warn!("TMDb is enabled but no API key is configured (set MM_TMDB_API_KEY)");
        }
        if config.providers.acoustid_enabled && config.providers.acoustid_api_key.is_none() {
            warn!("AcoustID is enabled but no API key is configured (set MM_ACOUSTID_API_KEY)");
        }
    }

    /// Return the platform-default configuration directory path.
    ///
    /// This is a convenience method for other modules that need to locate
    /// files relative to the config directory.
    pub fn config_dir() -> MmResult<PathBuf> {
        let base = dirs::config_dir().ok_or_else(|| {
            MmError::Config("unable to determine platform config directory".to_string())
        })?;
        Ok(base.join("MeedyaManager"))
    }

    /// Return the path to the settings file within the platform config dir.
    pub fn default_settings_path() -> MmResult<PathBuf> {
        Ok(Self::config_dir()?.join("settings.json5"))
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
#[allow(unsafe_code)] // Tests use set_var/remove_var which require unsafe in Edition 2024
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Helper: write content to a temporary file and return the path handle.
    fn write_temp_config(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("failed to create temp file");
        file.write_all(content.as_bytes())
            .expect("failed to write temp config");
        file.flush().expect("failed to flush temp config");
        file
    }

    // -----------------------------------------------------------------------
    // 1. Default values
    // -----------------------------------------------------------------------

    #[test]
    fn test_default_app_config_has_correct_app_name() {
        // Verify the default app name is "MeedyaManager"
        let config = AppConfig::default();
        assert_eq!(config.app_name, "MeedyaManager");
    }

    #[test]
    fn test_default_dry_run_is_false() {
        // Dry-run should be off by default
        let config = AppConfig::default();
        assert!(!config.dry_run);
    }

    #[test]
    fn test_default_watch_config_values() {
        // Watch defaults: no folders, recursive on, 5s poll, 200ms debounce
        let watch = WatchConfig::default();
        assert!(watch.folders.is_empty());
        assert!(watch.recursive);
        assert_eq!(watch.poll_interval_secs, 5);
        assert_eq!(watch.debounce_ms, 200);
        assert!(watch.include_extensions.is_empty());
        assert!(watch.exclude_extensions.is_empty());
    }

    #[test]
    fn test_default_rename_config_values() {
        // Rename defaults: standard template, skip conflicts, create dirs, move mode
        let rename = RenameConfig::default();
        assert_eq!(rename.template, "<Artist>/<Album>/<Title>");
        assert!(rename.output_dir.is_none());
        assert_eq!(rename.conflict_strategy, "skip");
        assert!(rename.create_dirs);
        assert!(!rename.copy_mode);
    }

    #[test]
    fn test_default_logging_config_values() {
        // Logging defaults: info level, console on, no file, 10 MB max, PII redaction
        let logging = LoggingConfig::default();
        assert_eq!(logging.level, "info");
        assert!(logging.console);
        assert!(logging.file.is_none());
        assert_eq!(logging.max_file_size_bytes, 10 * 1024 * 1024);
        assert_eq!(logging.max_rotated_files, 3);
        assert!(logging.redact_pii);
    }

    #[test]
    fn test_default_provider_config_values() {
        // Only MusicBrainz is enabled by default (it's free and open)
        let providers = ProviderConfig::default();
        assert!(providers.musicbrainz_enabled);
        assert!(!providers.discogs_enabled);
        assert!(providers.discogs_token.is_none());
        assert!(!providers.spotify_enabled);
        assert!(providers.spotify_client_id.is_none());
        assert!(!providers.tmdb_enabled);
        assert!(providers.tmdb_api_key.is_none());
        assert!(!providers.acoustid_enabled);
        assert_eq!(providers.request_timeout_secs, 30);
        assert_eq!(providers.max_concurrent_requests, 4);
    }

    // -----------------------------------------------------------------------
    // 2. Loading from files
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_from_missing_file_returns_defaults() {
        // Loading from a non-existent path should succeed with defaults
        let fake_path = Path::new("/tmp/nonexistent_meedya_config_12345.json5");
        let config = AppConfig::load_from(fake_path).expect("should succeed with defaults");
        assert_eq!(config.app_name, "MeedyaManager");
        assert!(!config.dry_run);
    }

    #[test]
    fn test_load_from_empty_json5_object() {
        // An empty JSON5 object `{}` should deserialize to all defaults
        let file = write_temp_config("{}");
        let config =
            AppConfig::load_from(file.path()).expect("empty object should parse to defaults");
        assert_eq!(config.app_name, "MeedyaManager");
        assert_eq!(config.watch.poll_interval_secs, 5);
    }

    #[test]
    fn test_load_from_partial_json5() {
        // A JSON5 file with only some fields should merge with defaults
        let content = r#"{
            // Override the app name
            app_name: "MyCustomName",
            dry_run: true,
            watch: {
                recursive: false,
                poll_interval_secs: 10,
            },
        }"#;
        let file = write_temp_config(content);
        let config = AppConfig::load_from(file.path()).expect("partial config should parse");

        // Overridden fields
        assert_eq!(config.app_name, "MyCustomName");
        assert!(config.dry_run);
        assert!(!config.watch.recursive);
        assert_eq!(config.watch.poll_interval_secs, 10);

        // Default fields that were not specified
        assert_eq!(config.watch.debounce_ms, 200);
        assert_eq!(config.rename.template, "<Artist>/<Album>/<Title>");
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_load_from_full_json5() {
        // A fully specified JSON5 file
        let content = r#"{
            app_name: "FullConfig",
            dry_run: true,
            watch: {
                folders: ["/tmp/music", "/tmp/video"],
                recursive: false,
                poll_interval_secs: 15,
                debounce_ms: 500,
                include_extensions: ["mp3", "flac"],
                exclude_extensions: ["tmp"],
            },
            rename: {
                template: "<Genre>/<Artist> - <Title>",
                output_dir: "/tmp/organized",
                conflict_strategy: "overwrite",
                create_dirs: false,
                copy_mode: true,
            },
            logging: {
                level: "debug",
                console: false,
                file: "/tmp/meedya.log",
                max_file_size_bytes: 5242880,
                max_rotated_files: 5,
                redact_pii: false,
            },
            providers: {
                musicbrainz_enabled: false,
                discogs_enabled: true,
                discogs_token: "test-token",
                spotify_enabled: true,
                spotify_client_id: "client-id",
                spotify_client_secret: "client-secret",
                tmdb_enabled: true,
                tmdb_api_key: "tmdb-key",
                acoustid_enabled: true,
                acoustid_api_key: "acoustid-key",
                request_timeout_secs: 60,
                max_concurrent_requests: 8,
            },
        }"#;
        let file = write_temp_config(content);
        let config = AppConfig::load_from(file.path()).expect("full config should parse");

        assert_eq!(config.app_name, "FullConfig");
        assert!(config.dry_run);

        // Watch section
        assert_eq!(config.watch.folders.len(), 2);
        assert_eq!(config.watch.folders[0], PathBuf::from("/tmp/music"));
        assert!(!config.watch.recursive);
        assert_eq!(config.watch.poll_interval_secs, 15);
        assert_eq!(config.watch.debounce_ms, 500);
        assert_eq!(config.watch.include_extensions, vec!["mp3", "flac"]);
        assert_eq!(config.watch.exclude_extensions, vec!["tmp"]);

        // Rename section
        assert_eq!(config.rename.template, "<Genre>/<Artist> - <Title>");
        assert_eq!(
            config.rename.output_dir,
            Some(PathBuf::from("/tmp/organized"))
        );
        assert_eq!(config.rename.conflict_strategy, "overwrite");
        assert!(!config.rename.create_dirs);
        assert!(config.rename.copy_mode);

        // Logging section
        assert_eq!(config.logging.level, "debug");
        assert!(!config.logging.console);
        assert_eq!(config.logging.file, Some(PathBuf::from("/tmp/meedya.log")));
        assert_eq!(config.logging.max_file_size_bytes, 5_242_880);
        assert_eq!(config.logging.max_rotated_files, 5);
        assert!(!config.logging.redact_pii);

        // Provider section
        assert!(!config.providers.musicbrainz_enabled);
        assert!(config.providers.discogs_enabled);
        assert_eq!(
            config.providers.discogs_token,
            Some("test-token".to_string())
        );
        assert!(config.providers.spotify_enabled);
        assert_eq!(
            config.providers.spotify_client_id,
            Some("client-id".to_string())
        );
        assert!(config.providers.tmdb_enabled);
        assert_eq!(config.providers.tmdb_api_key, Some("tmdb-key".to_string()));
        assert!(config.providers.acoustid_enabled);
        assert_eq!(
            config.providers.acoustid_api_key,
            Some("acoustid-key".to_string())
        );
        assert_eq!(config.providers.request_timeout_secs, 60);
        assert_eq!(config.providers.max_concurrent_requests, 8);
    }

    #[test]
    fn test_load_from_invalid_json5_returns_error() {
        // Malformed JSON5 should produce a Config error
        let file = write_temp_config("{ this is not valid json5 at all !!!");
        let result = AppConfig::load_from(file.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, MmError::Config(_)),
            "expected Config error, got: {err}"
        );
    }

    // -----------------------------------------------------------------------
    // 3. Environment variable overrides
    // -----------------------------------------------------------------------

    #[test]
    fn test_env_override_dry_run() {
        // MM_DRY_RUN=true should set dry_run to true
        let mut config = AppConfig::default();
        assert!(!config.dry_run);

        // Temporarily set the env var
        unsafe {
            std::env::set_var("MM_DRY_RUN", "true");
        }
        AppConfig::apply_env_overrides(&mut config);
        assert!(config.dry_run);

        // Clean up
        unsafe {
            std::env::remove_var("MM_DRY_RUN");
        }
    }

    #[test]
    fn test_env_override_log_level() {
        // MM_LOG_LEVEL=debug should override the log level
        let mut config = AppConfig::default();
        assert_eq!(config.logging.level, "info");

        unsafe {
            std::env::set_var("MM_LOG_LEVEL", "debug");
        }
        AppConfig::apply_env_overrides(&mut config);
        assert_eq!(config.logging.level, "debug");

        // Clean up
        unsafe {
            std::env::remove_var("MM_LOG_LEVEL");
        }
    }

    #[test]
    fn test_env_override_provider_keys() {
        // Provider API keys should be overridable via env vars
        let mut config = AppConfig::default();
        assert!(config.providers.discogs_token.is_none());
        assert!(config.providers.tmdb_api_key.is_none());

        unsafe {
            std::env::set_var("MM_DISCOGS_TOKEN", "env-discogs-token");
        }
        unsafe {
            std::env::set_var("MM_TMDB_API_KEY", "env-tmdb-key");
        }
        AppConfig::apply_env_overrides(&mut config);

        assert_eq!(
            config.providers.discogs_token,
            Some("env-discogs-token".to_string())
        );
        assert_eq!(
            config.providers.tmdb_api_key,
            Some("env-tmdb-key".to_string())
        );

        // Clean up
        unsafe {
            std::env::remove_var("MM_DISCOGS_TOKEN");
        }
        unsafe {
            std::env::remove_var("MM_TMDB_API_KEY");
        }
    }

    #[test]
    fn test_env_override_rename_template() {
        // MM_RENAME_TEMPLATE should override the rename template
        let mut config = AppConfig::default();
        unsafe {
            std::env::set_var("MM_RENAME_TEMPLATE", "<Genre>/<Artist>");
        }
        AppConfig::apply_env_overrides(&mut config);
        assert_eq!(config.rename.template, "<Genre>/<Artist>");

        // Clean up
        unsafe {
            std::env::remove_var("MM_RENAME_TEMPLATE");
        }
    }

    #[test]
    fn test_env_override_watch_poll_interval() {
        // MM_WATCH_POLL_INTERVAL should override the poll interval
        let mut config = AppConfig::default();
        unsafe {
            std::env::set_var("MM_WATCH_POLL_INTERVAL", "30");
        }
        AppConfig::apply_env_overrides(&mut config);
        assert_eq!(config.watch.poll_interval_secs, 30);

        // Clean up
        unsafe {
            std::env::remove_var("MM_WATCH_POLL_INTERVAL");
        }
    }

    // -----------------------------------------------------------------------
    // 4. JSON5 features (comments, trailing commas, unquoted keys)
    // -----------------------------------------------------------------------

    #[test]
    fn test_json5_comments_and_trailing_commas() {
        // JSON5 supports single-line and multi-line comments, plus trailing commas
        let content = r#"{
            // This is a single-line comment
            app_name: "CommentTest",
            /* This is a
               multi-line comment */
            dry_run: true,  // trailing comma is OK
        }"#;
        let file = write_temp_config(content);
        let config = AppConfig::load_from(file.path()).expect("JSON5 with comments should parse");
        assert_eq!(config.app_name, "CommentTest");
        assert!(config.dry_run);
    }

    // -----------------------------------------------------------------------
    // 5. Helper methods
    // -----------------------------------------------------------------------

    #[test]
    fn test_config_dir_returns_path_with_meedyamanager() {
        // The config directory should end with "MeedyaManager"
        let dir = AppConfig::config_dir();
        // This test will only pass on systems where dirs::config_dir() returns Some
        if let Ok(path) = dir {
            assert!(
                path.ends_with("MeedyaManager"),
                "config dir should end with MeedyaManager, got: {}",
                path.display()
            );
        }
    }

    #[test]
    fn test_default_settings_path_ends_with_settings_json5() {
        // The default settings path should end with settings.json5
        if let Ok(path) = AppConfig::default_settings_path() {
            assert!(
                path.ends_with("settings.json5"),
                "path should end with settings.json5, got: {}",
                path.display()
            );
        }
    }

    // -----------------------------------------------------------------------
    // 6. Serde round-trip and Clone/PartialEq
    // -----------------------------------------------------------------------

    #[test]
    fn test_config_clone_equals_original() {
        // Clone should produce an identical config
        let config = AppConfig::default();
        let cloned = config.clone();
        assert_eq!(config, cloned);
    }

    #[test]
    fn test_config_debug_formatting() {
        // Debug formatting should not panic and should contain key field names
        let config = AppConfig::default();
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("app_name"));
        assert!(debug_str.contains("MeedyaManager"));
        assert!(debug_str.contains("watch"));
        assert!(debug_str.contains("rename"));
        assert!(debug_str.contains("logging"));
        assert!(debug_str.contains("providers"));
    }

    #[test]
    fn test_env_override_conflict_strategy() {
        // MM_RENAME_CONFLICT should override the conflict strategy
        let mut config = AppConfig::default();
        unsafe {
            std::env::set_var("MM_RENAME_CONFLICT", "overwrite");
        }
        AppConfig::apply_env_overrides(&mut config);
        assert_eq!(config.rename.conflict_strategy, "overwrite");

        // Clean up
        unsafe {
            std::env::remove_var("MM_RENAME_CONFLICT");
        }
    }
}
