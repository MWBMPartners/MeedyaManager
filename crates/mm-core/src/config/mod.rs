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
// Config directory resolution — the SINGLE source of truth
// ---------------------------------------------------------------------------
//
// Historically several modules each computed "the config directory" for
// themselves, and did so inconsistently: some joined the platform config dir
// with "MeedyaManager" (uppercase, matching the app's canonical name) while
// others joined it with "meedyamanager" (lowercase). On a case-INsensitive
// filesystem (macOS default, Windows) both paths happen to resolve to the
// same directory on disk, so the bug went unnoticed there — but on a
// case-sensitive filesystem (Linux) they are two entirely different
// directories, so state written by one module (e.g. the test-mode manifest)
// is invisible to another (e.g. the settings loader). `app_config_dir()` is
// now the ONLY place that decides where "the config directory" lives; every
// other module must call it rather than touching `dirs::config_dir()`
// directly.

/// Resolve MeedyaManager's application config directory.
///
/// Resolution order (checked every call — no caching, since the override is
/// only ever set for the lifetime of a test or a headless invocation):
///
///   1. If the `MM_CONFIG_DIR` environment variable is set to a non-empty
///      value, that value is used verbatim as the config directory. This is
///      the explicit override point used by the test suite (see
///      `mm_config_dir_env_override_wins` below) to get full isolation from
///      whatever the real platform config directory happens to be, and by
///      any future headless/portable run mode.
///   2. Otherwise, the platform default: `dirs::config_dir()/MeedyaManager`
///      (uppercase — the canonical spelling of the app name everywhere
///      else in the codebase).
///
/// Note: the `dirs` crate (v6) deliberately ignores `XDG_CONFIG_HOME` on
/// macOS — it always resolves to `~/Library/Application Support` there,
/// matching Apple platform conventions. `MM_CONFIG_DIR` is therefore the only
/// reliable way to redirect MeedyaManager's config directory on macOS, which
/// is exactly why it exists as an explicit override rather than relying on
/// XDG variables.
pub fn app_config_dir() -> MmResult<PathBuf> {
    // Check the override first — an empty string is treated as "not set" so
    // that `MM_CONFIG_DIR=` in an inherited environment does not silently
    // redirect state into the process's current working directory.
    if let Ok(dir) = std::env::var("MM_CONFIG_DIR") {
        if !dir.is_empty() {
            return Ok(PathBuf::from(dir));
        }
    }

    // Fall back to the platform-default location, joined with the
    // canonical (uppercase) application name.
    let base = dirs::config_dir().ok_or_else(|| {
        MmError::Config("unable to determine platform config directory".to_string())
    })?;
    Ok(base.join("MeedyaManager"))
}

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
//
// Issue #211 (P1-SETTINGS): `AppConfig` is `#[serde(default)]` at every
// level and nowhere uses `#[serde(deny_unknown_fields)]`, so historically a
// settings.json5 file written against a stale/legacy schema (e.g. the old
// `watch_paths` / `rename_format` / per-provider-map shape) would parse
// *successfully* straight through to an all-defaults `AppConfig` — every
// value the user actually set was silently discarded, with no warning at
// all. Rather than switching to `deny_unknown_fields` (which would turn a
// typo into a hard load failure — too harsh for a config file users hand-
// edit) the loader instead parses into a generic `serde_json::Value` first,
// diffs its key set against `AppConfig::default()`'s own serialised shape,
// and reports every key that doesn't correspond to a real field. This is
// deliberately non-fatal: unknown keys are still ignored (exactly as
// before), just no longer *silently*.

/// Recursively collect dotted key paths present in `value` that have no
/// corresponding field in `shape` (both are generic `serde_json::Value`s —
/// `shape` is expected to be `serde_json::to_value(AppConfig::default())`,
/// i.e. the authoritative "what fields actually exist" reference).
///
/// Only descends into a key when BOTH sides have an object at that path —
/// once a key is unrecognised there is no field-shape to compare its
/// children against, so the whole unrecognised subtree is reported as one
/// path (e.g. the legacy `providers: { apple_music: { enabled: true } }`
/// map reports `providers.apple_music`, not `providers.apple_music.enabled`
/// as well). Arrays are never descended into: `rename.rules` is a
/// `Vec<crate::rule_engine::Rule>` that defaults to empty, so there is no
/// per-element shape to diff against, and we'd rather under-report than
/// flag legitimate rule fields as unknown.
fn find_unknown_keys(
    value: &serde_json::Value,
    shape: &serde_json::Value,
    prefix: &str,
    out: &mut Vec<String>,
) {
    // Both sides must be JSON objects for a key-by-key comparison to make
    // sense; anything else (array, string, number, bool, null) is a leaf
    // as far as this walk is concerned.
    let (Some(obj), Some(shape_obj)) = (value.as_object(), shape.as_object()) else {
        return;
    };

    for (key, sub_value) in obj {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        match shape_obj.get(key) {
            // Recognised field — recurse in case it is itself a nested
            // section (e.g. "watch", "providers") that might also carry
            // stray keys of its own.
            Some(shape_sub) => find_unknown_keys(sub_value, shape_sub, &path, out),
            // Not a field on `AppConfig` at all — record it.
            None => out.push(path),
        }
    }
}

/// A suggested replacement for a legacy/unknown dotted config key path.
///
/// Distinguishes "here's the modern equivalent" from "there simply isn't
/// one yet" so the two cases can be phrased differently in the warning
/// message rather than both reading as "did you mean `no equivalent yet`?".
enum KeySuggestion {
    /// A direct modern replacement exists — e.g. `watch_paths` → `watch.folders`.
    Equivalent(String),
    /// The legacy key named a real feature that has no config surface yet
    /// (e.g. `filename_replacements` — see `SanitizeConfig` in
    /// `crate::renamer`, which is currently code-only).
    NoneYet,
}

/// Look up a suggestion for one unknown dotted key path.
///
/// Covers the eight legacy keys the shipped `config/settings.json5` used to
/// carry before it was regenerated from `AppConfig::default()` (issue
/// #211), plus a pattern match for the old per-provider map shape
/// (`providers.<name>: { enabled: ... }`), which every one of the 19
/// provider pages under `help/providers/` used to document and which now
/// maps onto the flat `providers.<name>_enabled` boolean field.
fn suggest_replacement(key: &str) -> Option<KeySuggestion> {
    if let Some(equivalent) = match key {
        "watch_paths" => Some("watch.folders"),
        "valid_extensions" => Some("watch.include_extensions"),
        "rename_format" => Some("rename.template"),
        "logging.max_log_size_mb" => Some("logging.max_file_size_bytes"),
        _ => None,
    } {
        return Some(KeySuggestion::Equivalent(equivalent.to_string()));
    }

    if matches!(
        key,
        "fallback_metadata" | "filename_replacements" | "cover_art"
    ) {
        return Some(KeySuggestion::NoneYet);
    }

    // `providers.<name>` (old per-provider map) → `providers.<name>_enabled`
    // (modern flat field). Guard against matching `providers.<name>_enabled`
    // itself or any other already-nested path — those are either
    // recognised fields (never reach this function) or genuinely unknown
    // nested keys we have no specific advice for.
    if let Some(name) = key.strip_prefix("providers.") {
        if !name.is_empty() && !name.contains('.') {
            return Some(KeySuggestion::Equivalent(format!(
                "providers.{name}_enabled"
            )));
        }
    }

    None
}

/// Render the `tracing::warn!` message for one unknown config key,
/// including a suggestion where `suggest_replacement` has one.
fn format_unknown_key_warning(key: &str) -> String {
    match suggest_replacement(key) {
        Some(KeySuggestion::Equivalent(s)) => {
            format!("unknown config key `{key}` — ignored (did you mean `{s}`?)")
        }
        Some(KeySuggestion::NoneYet) => {
            format!("unknown config key `{key}` — ignored (no equivalent yet)")
        }
        None => format!("unknown config key `{key}` — ignored"),
    }
}

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
        // Resolve the application config directory via the single resolver
        // (honours the `MM_CONFIG_DIR` override — see `app_config_dir`).
        let config_dir = app_config_dir()?;

        // Build the full path: <config_dir>/settings.json5
        let settings_path = config_dir.join("settings.json5");

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
    ///
    /// This is a thin wrapper around [`Self::load_from_with_report`] that
    /// logs each unknown config key (with a suggestion, where one is known
    /// — see `suggest_replacement`) via `tracing::warn!` and discards the
    /// report, returning only the config. Call `load_from_with_report`
    /// directly if you need the unknown-key list programmatically (e.g. to
    /// surface it in a CLI command, or — as here — to assert on it in
    /// tests without standing up a tracing subscriber).
    pub fn load_from(path: &Path) -> MmResult<Self> {
        let (config, unknown_keys) = Self::load_from_with_report(path)?;

        for message in &unknown_keys {
            warn!("{message}");
        }

        Ok(config)
    }

    /// Load configuration from a specific file path, returning both the
    /// config AND a report of any unrecognised keys found in the file.
    ///
    /// The report entries are the fully-rendered warning strings (see
    /// `format_unknown_key_warning`) rather than bare dotted key paths —
    /// this lets callers (and tests) check for both the offending key and
    /// its suggested replacement in one string, without needing to
    /// duplicate `suggest_replacement`'s lookup logic themselves.
    ///
    /// A missing file has no keys to report on, so the report is simply
    /// empty in that case (behaviour otherwise unchanged: falls back to
    /// `Self::default()` with the existing "file not found" warning).
    pub fn load_from_with_report(path: &Path) -> MmResult<(Self, Vec<String>)> {
        if !path.exists() {
            warn!(
                path = %path.display(),
                "Configuration file not found — using defaults"
            );
            let mut config = Self::default();
            Self::apply_env_overrides(&mut config);
            Self::validate(&config);
            debug!(?config, "Final configuration loaded");
            return Ok((config, Vec::new()));
        }

        info!(path = %path.display(), "Reading configuration file");

        // Read the raw file contents
        let contents = std::fs::read_to_string(path).map_err(|e| {
            MmError::Config(format!(
                "failed to read config file '{}': {}",
                path.display(),
                e
            ))
        })?;

        // Parse JSON5 into a generic value FIRST — this is what makes
        // unknown-key detection possible. JSON5 is a superset of JSON that
        // allows comments, trailing commas, unquoted keys, and other
        // conveniences; `serde_json::Value` is a perfectly valid target
        // type for `json5::from_str` like any other `Deserialize` type.
        let parsed: serde_json::Value = json5::from_str(&contents).map_err(|e| {
            MmError::Config(format!(
                "failed to parse config file '{}': {}",
                path.display(),
                e
            ))
        })?;

        // `AppConfig::default()`'s own serialised shape is the single
        // source of truth for "what fields actually exist" — this is the
        // same value the shipped `config/settings.json5` and
        // `config/schemas/settings.schema.json` are checked against in
        // this module's tests, so all three stay in lockstep.
        let default_shape = serde_json::to_value(Self::default())
            .expect("AppConfig::default() must always serialise to a JSON value");

        let mut unknown_key_paths = Vec::new();
        find_unknown_keys(&parsed, &default_shape, "", &mut unknown_key_paths);
        let report: Vec<String> = unknown_key_paths
            .iter()
            .map(|key| format_unknown_key_warning(key))
            .collect();

        // Deserialise strongly from the already-parsed value (not the raw
        // text again — `serde_json::Value` implements `Deserializer`, so
        // this is a second, cheap in-memory conversion rather than a
        // second parse). Every struct in this module is `#[serde(default)]`,
        // so unknown keys are simply skipped here exactly as before — we
        // have already captured them above.
        let mut config: Self = serde_json::from_value(parsed).map_err(|e| {
            MmError::Config(format!(
                "failed to parse config file '{}': {}",
                path.display(),
                e
            ))
        })?;

        // Apply .env overrides on top of the loaded config
        Self::apply_env_overrides(&mut config);

        // Run validation (warnings only for non-critical issues)
        Self::validate(&config);

        debug!(?config, "Final configuration loaded");

        Ok((config, report))
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

    /// Return the application's configuration directory path.
    ///
    /// This is a convenience method for other modules that need to locate
    /// files relative to the config directory. It is a thin wrapper around
    /// the module-level `app_config_dir()` resolver, kept here so callers
    /// that already hold an `AppConfig`-shaped mental model (or just prefer
    /// the associated-function spelling) don't need to import the free
    /// function separately.
    pub fn config_dir() -> MmResult<PathBuf> {
        app_config_dir()
    }

    /// Return the path to the settings file within the platform config dir.
    pub fn default_settings_path() -> MmResult<PathBuf> {
        Ok(Self::config_dir()?.join("settings.json5"))
    }
}

// ---------------------------------------------------------------------------
// Test-only synchronisation for the process-wide environment
// ---------------------------------------------------------------------------
//
// `MM_CONFIG_DIR` (like any environment variable) is process-global state.
// Rust runs `#[test]` functions concurrently on multiple threads by default,
// so two tests that set/read/clear this var at the same time can race and
// see each other's value. This lock is deliberately declared here — at the
// top of the `config` module rather than nested inside a private `mod
// tests` block — and marked `pub(crate)` so that every other module's test
// suite (test_mode, integrity, filetype_registry, tag_registry,
// settings_bundle, state) can take the SAME lock before touching
// `MM_CONFIG_DIR`. A lock private to each module's own test block would not
// help: two different `Mutex` instances do not exclude each other, so the
// race would still be possible across module boundaries.
#[cfg(test)]
pub(crate) static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        // `load_from` applies MM_* environment overrides, so this test READS the
        // process-wide environment even though it never writes to it. It must
        // therefore serialise against the env-mutating tests in this module —
        // otherwise a concurrent `test_env_override_*` can inject its value
        // here (e.g. MM_RENAME_TEMPLATE) and fail this assertion spuriously.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Loading from a non-existent path should succeed with defaults
        let fake_path = Path::new("/tmp/nonexistent_meedya_config_12345.json5");
        let config = AppConfig::load_from(fake_path).expect("should succeed with defaults");
        assert_eq!(config.app_name, "MeedyaManager");
        assert!(!config.dry_run);
    }

    #[test]
    fn test_load_from_empty_json5_object() {
        // `load_from` applies MM_* environment overrides, so this test READS the
        // process-wide environment even though it never writes to it. It must
        // therefore serialise against the env-mutating tests in this module —
        // otherwise a concurrent `test_env_override_*` can inject its value
        // here (e.g. MM_RENAME_TEMPLATE) and fail this assertion spuriously.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // An empty JSON5 object `{}` should deserialize to all defaults
        let file = write_temp_config("{}");
        let config =
            AppConfig::load_from(file.path()).expect("empty object should parse to defaults");
        assert_eq!(config.app_name, "MeedyaManager");
        assert_eq!(config.watch.poll_interval_secs, 5);
    }

    #[test]
    fn test_load_from_partial_json5() {
        // `load_from` applies MM_* environment overrides, so this test READS the
        // process-wide environment even though it never writes to it. It must
        // therefore serialise against the env-mutating tests in this module —
        // otherwise a concurrent `test_env_override_*` can inject its value
        // here (e.g. MM_RENAME_TEMPLATE) and fail this assertion spuriously.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        // `load_from` applies MM_* environment overrides, so this test READS the
        // process-wide environment even though it never writes to it. It must
        // therefore serialise against the env-mutating tests in this module —
        // otherwise a concurrent `test_env_override_*` can inject its value
        // here (e.g. MM_RENAME_TEMPLATE) and fail this assertion spuriously.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        // `load_from` applies MM_* environment overrides, so this test READS the
        // process-wide environment even though it never writes to it. It must
        // therefore serialise against the env-mutating tests in this module —
        // otherwise a concurrent `test_env_override_*` can inject its value
        // here (e.g. MM_RENAME_TEMPLATE) and fail this assertion spuriously.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        // Serialise against every other env-mutating test in this crate.
        // `set_var`/`remove_var` are process-wide, and cargo runs tests
        // concurrently, so without this a sibling test can observe this
        // test's variable (or clear one this test is relying on) mid-run.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        // Serialise against every other env-mutating test in this crate.
        // `set_var`/`remove_var` are process-wide, and cargo runs tests
        // concurrently, so without this a sibling test can observe this
        // test's variable (or clear one this test is relying on) mid-run.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        // Serialise against every other env-mutating test in this crate.
        // `set_var`/`remove_var` are process-wide, and cargo runs tests
        // concurrently, so without this a sibling test can observe this
        // test's variable (or clear one this test is relying on) mid-run.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        // Serialise against every other env-mutating test in this crate.
        // `set_var`/`remove_var` are process-wide, and cargo runs tests
        // concurrently, so without this a sibling test can observe this
        // test's variable (or clear one this test is relying on) mid-run.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        // Serialise against every other env-mutating test in this crate.
        // `set_var`/`remove_var` are process-wide, and cargo runs tests
        // concurrently, so without this a sibling test can observe this
        // test's variable (or clear one this test is relying on) mid-run.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        // `load_from` applies MM_* environment overrides, so this test READS the
        // process-wide environment even though it never writes to it. It must
        // therefore serialise against the env-mutating tests in this module —
        // otherwise a concurrent `test_env_override_*` can inject its value
        // here (e.g. MM_RENAME_TEMPLATE) and fail this assertion spuriously.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        // Take the ENV_LOCK even though this test doesn't itself set
        // MM_CONFIG_DIR: `AppConfig::config_dir()` now honours that variable,
        // so without the lock a concurrently-running test that points
        // MM_CONFIG_DIR at a tempdir could make this assertion flake.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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
        // Serialise against every other env-mutating test in this crate.
        // `set_var`/`remove_var` are process-wide, and cargo runs tests
        // concurrently, so without this a sibling test can observe this
        // test's variable (or clear one this test is relying on) mid-run.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

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

    // -----------------------------------------------------------------------
    // 7. `MM_CONFIG_DIR` resolution — issue #212 (P0-CONFIGDIR)
    // -----------------------------------------------------------------------

    #[test]
    fn mm_config_dir_env_override_wins() {
        // Take the process-environment lock before touching MM_CONFIG_DIR —
        // several other test modules set the same variable, and cargo runs
        // tests concurrently by default.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Point MM_CONFIG_DIR at a freshly-created tempdir so this test is
        // fully isolated from whatever the real platform config dir is.
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        unsafe {
            std::env::set_var("MM_CONFIG_DIR", tmp.path());
        }

        let resolved = app_config_dir().expect("app_config_dir should resolve under override");
        assert_eq!(
            resolved,
            tmp.path(),
            "MM_CONFIG_DIR should be used verbatim, not joined with 'MeedyaManager'"
        );

        // Clean up before releasing the lock so the next test starts clean.
        unsafe {
            std::env::remove_var("MM_CONFIG_DIR");
        }
    }

    #[test]
    fn mm_config_dir_env_override_ignores_empty_value() {
        // An MM_CONFIG_DIR set to the empty string must be treated as unset
        // (otherwise an inherited `MM_CONFIG_DIR=` would silently redirect
        // config/state into the current working directory).
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        unsafe {
            std::env::set_var("MM_CONFIG_DIR", "");
        }

        let resolved = app_config_dir();
        // Falls through to the platform default, which — on this test host —
        // resolves to Some(..); we only assert it did NOT become an empty path.
        if let Ok(path) = resolved {
            assert!(
                path.ends_with("MeedyaManager"),
                "empty MM_CONFIG_DIR should fall back to the platform default, got: {}",
                path.display()
            );
        }

        unsafe {
            std::env::remove_var("MM_CONFIG_DIR");
        }
    }

    #[test]
    fn all_core_paths_share_one_directory() {
        // REGRESSION TEST for issue #212: before the fix, test_mode,
        // integrity, filetype_registry, tag_registry and settings_bundle each
        // resolved their own file paths under a *lowercase*
        // "meedyamanager" directory, while config/state/health used the
        // canonical *uppercase* "MeedyaManager" — two different directories
        // on a case-sensitive filesystem (Linux). This test proves every
        // module now agrees on exactly one directory.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        unsafe {
            std::env::set_var("MM_CONFIG_DIR", tmp.path());
        }

        let expected = app_config_dir().expect("app_config_dir should resolve under override");

        // test_mode — persists `testmode_manifest.json`
        let test_mode_path =
            crate::test_mode::manifest_path().expect("test_mode manifest path should resolve");
        assert!(
            test_mode_path.starts_with(&expected),
            "test_mode manifest path {} does not share the config dir {}",
            test_mode_path.display(),
            expected.display()
        );

        // integrity — appends to `corruption.log`
        let integrity_path = crate::integrity::corruption_log_path()
            .expect("integrity corruption log path should resolve");
        assert!(
            integrity_path.starts_with(&expected),
            "integrity corruption log path {} does not share the config dir {}",
            integrity_path.display(),
            expected.display()
        );

        // filetype_registry — reads a `filetypes.json5` override
        let filetype_path = crate::filetype_registry::user_override_path()
            .expect("filetype_registry override path should resolve");
        assert!(
            filetype_path.starts_with(&expected),
            "filetype_registry override path {} does not share the config dir {}",
            filetype_path.display(),
            expected.display()
        );

        // metadata::tag_registry — reads a `tags.json5` override
        let tag_path = crate::metadata::tag_registry::user_override_path()
            .expect("tag_registry override path should resolve");
        assert!(
            tag_path.starts_with(&expected),
            "tag_registry override path {} does not share the config dir {}",
            tag_path.display(),
            expected.display()
        );

        // settings_bundle — reads/writes override files by name
        let bundle_path = crate::settings_bundle::user_override_path("filetypes.json5")
            .expect("settings_bundle override path should resolve");
        assert!(
            bundle_path.starts_with(&expected),
            "settings_bundle override path {} does not share the config dir {}",
            bundle_path.display(),
            expected.display()
        );

        // state — persists `state.json` and the single-instance lock file
        let state_path = crate::state::AppState::default_path();
        assert!(
            state_path.starts_with(&expected),
            "state path {} does not share the config dir {}",
            state_path.display(),
            expected.display()
        );

        unsafe {
            std::env::remove_var("MM_CONFIG_DIR");
        }
    }

    // -----------------------------------------------------------------------
    // 8. Shipped `config/settings.json5` / `config/schemas/settings.schema.json`
    //    stay in sync with `AppConfig` — issue #211 (P1-SETTINGS)
    // -----------------------------------------------------------------------
    //
    // These `include_str!` the actual files shipped in the repository (not a
    // copy embedded in the test), so a future edit that lets either file
    // drift from the real `AppConfig` struct fails CI immediately instead of
    // shipping a config example users can't actually use.

    /// Recursively assert that `schema_node`'s `"properties"` key set matches
    /// `shape_node`'s object key set at every level where `shape_node` (a
    /// slice of `serde_json::to_value(AppConfig::default())`) is itself a
    /// JSON object. Mismatches are collected as human-readable strings
    /// rather than asserted immediately, so a single test run reports every
    /// offending path at once instead of stopping at the first one.
    fn schema_mismatches(
        schema_node: &serde_json::Value,
        shape_node: &serde_json::Value,
        prefix: &str,
        out: &mut Vec<String>,
    ) {
        // A non-object leaf in the struct's default shape (string, bool,
        // number, array, null) has nothing further to check — arrays in
        // particular (e.g. `rename.rules`) are deliberately not modelled
        // field-by-field in the schema, matching `find_unknown_keys`'s same
        // choice not to walk into them above.
        let Some(shape_obj) = shape_node.as_object() else {
            return;
        };

        let Some(schema_properties) = schema_node.get("properties").and_then(|p| p.as_object())
        else {
            out.push(format!(
                "{}: schema node has no \"properties\" but AppConfig has fields {:?}",
                if prefix.is_empty() { "<root>" } else { prefix },
                shape_obj.keys().collect::<Vec<_>>()
            ));
            return;
        };

        let shape_keys: std::collections::BTreeSet<&str> =
            shape_obj.keys().map(String::as_str).collect();
        let schema_keys: std::collections::BTreeSet<&str> =
            schema_properties.keys().map(String::as_str).collect();

        if shape_keys != schema_keys {
            out.push(format!(
                "{}: AppConfig fields {:?} != schema properties {:?}",
                if prefix.is_empty() { "<root>" } else { prefix },
                shape_keys,
                schema_keys
            ));
        }

        // Recurse only into keys both sides agree exist — a key missing on
        // one side was already reported above, and there's no matching node
        // to recurse into on the side that lacks it.
        for key in shape_keys.intersection(&schema_keys) {
            let child_path = if prefix.is_empty() {
                (*key).to_string()
            } else {
                format!("{prefix}.{key}")
            };
            if let (Some(shape_child), Some(schema_child)) =
                (shape_obj.get(*key), schema_properties.get(*key))
            {
                schema_mismatches(schema_child, shape_child, &child_path, out);
            }
        }
    }

    #[test]
    fn shipped_settings_json5_has_only_known_keys() {
        // MUST FAIL FIRST (before the issue #211 fix): the shipped file used
        // to ship `watch_paths`, `rename_format`, `fallback_metadata`,
        // `filename_replacements`, `cover_art`, and a per-provider
        // `providers.<name>: { enabled, ... }` map — none of which exist on
        // `AppConfig`. Parsing that file against `AppConfig::default()`'s own
        // shape must report an EMPTY unknown-key list.
        let shipped: serde_json::Value =
            json5::from_str(include_str!("../../../../config/settings.json5"))
                .expect("shipped config/settings.json5 must be valid JSON5");
        let default_shape = serde_json::to_value(AppConfig::default())
            .expect("AppConfig::default() must serialise");

        let mut unknown = Vec::new();
        find_unknown_keys(&shipped, &default_shape, "", &mut unknown);

        assert!(
            unknown.is_empty(),
            "shipped config/settings.json5 has keys AppConfig does not recognise: {unknown:#?}"
        );
    }

    #[test]
    fn shipped_settings_json5_deserialises_to_non_default() {
        // MUST FAIL FIRST (before the issue #211 fix): because the shipped
        // file used a schema `AppConfig` does not implement, every key a
        // user might edit was silently discarded and the file deserialised
        // to (almost) pure defaults — i.e. editing the "example config" had
        // zero effect. At least one field must now differ from
        // `AppConfig::default()` after loading the real shipped file.
        //
        // NOTE on why this asserts a *specific* field rather than only the
        // blanket `assert_ne!` below: the OLD shipped file's
        // `logging: { level: "INFO", ... }` happens to share a field NAME
        // with the real `LoggingConfig::level` field, so it deserialises to
        // a non-default (wrongly-cased) value *by accident*, even though
        // the fields that actually demonstrate the bug (`watch_paths`,
        // `rename_format`, ...) are still being silently discarded. A plain
        // `assert_ne!` alone would therefore already pass against the old,
        // broken file — this targets the defect precisely instead: a
        // watch-folder list the user configured under the OLD key name
        // (`watch_paths`) must survive under the NEW key name
        // (`watch.folders`) once the shipped file is regenerated.
        let shipped: AppConfig = json5::from_str(include_str!("../../../../config/settings.json5"))
            .expect("shipped config/settings.json5 must deserialise to AppConfig");

        assert!(
            !shipped.watch.folders.is_empty(),
            "shipped config/settings.json5's watch folders came back empty — the file's \
             example watch-folder list is being silently discarded instead of actually \
             populating `watch.folders`"
        );
        assert_ne!(
            shipped,
            AppConfig::default(),
            "shipped config/settings.json5 deserialises to pure defaults — every example \
             value in the file is being silently ignored"
        );
    }

    #[test]
    fn settings_schema_properties_match_appconfig() {
        // MUST FAIL FIRST (before the issue #211 fix): the shipped schema
        // described `watch_paths`, `rename_format`, `providers.<name>`,
        // `cover_art`, etc. — a completely different property set from the
        // real `AppConfig` struct's `watch`, `rename`, `providers.*_enabled`,
        // and so on. Recursive property-name set equality must hold at
        // every nested object level.
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../config/schemas/settings.schema.json"
        ))
        .expect("config/schemas/settings.schema.json must be valid JSON");
        let default_shape = serde_json::to_value(AppConfig::default())
            .expect("AppConfig::default() must serialise");

        let mut mismatches = Vec::new();
        schema_mismatches(&schema, &default_shape, "", &mut mismatches);

        assert!(
            mismatches.is_empty(),
            "settings.schema.json properties do not match AppConfig: {mismatches:#?}"
        );
    }

    #[test]
    fn load_from_reports_unknown_keys_with_suggestion() {
        // This test calls `load_from_with_report`, which — via
        // `apply_env_overrides` — reads several `MM_*` process environment
        // variables even though it never sets any itself. It must therefore
        // take `ENV_LOCK` like every other test in this module that touches
        // `load_from`/`load_from_with_report`, or a concurrently-running
        // env-mutating test could make an unrelated field assertion flake.
        let _guard = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // A file using the single most common legacy key (`watch_paths`,
        // the old name for `watch.folders`) should be reported by dotted
        // path AND come with its modern replacement suggested — returning
        // the list (rather than only logging via `tracing::warn!`) is what
        // makes this assertable without standing up a tracing subscriber.
        let file = write_temp_config("{watch_paths:[]}");
        let (_config, report) = AppConfig::load_from_with_report(file.path())
            .expect("a file with one unknown key should still load successfully");

        assert!(
            report
                .iter()
                .any(|message| message.contains("watch_paths") && message.contains("watch.folders")),
            "expected a report entry mentioning both `watch_paths` and `watch.folders`, got: {report:#?}"
        );
    }
}
