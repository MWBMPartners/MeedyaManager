// (C) 2025-2026 MWBM Partners Ltd
//
// Structured logging with PII redaction and file rotation.
//
// Uses the `tracing` crate for structured, async-safe logging.
// Console output is human-readable with colours; file output is
// JSON-structured with daily rotation.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::error::{MmError, MmResult};

/// Configuration for the logging system
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Minimum log level (trace, debug, info, warn, error)
    pub level: String,
    /// Enable console (stdout) logging
    pub console: bool,
    /// Enable file logging
    pub file: bool,
    /// Directory for log files
    pub log_dir: PathBuf,
    /// Enable PII redaction in log output
    pub redact_pii: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        let log_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("MeedyaManager")
            .join("logs");

        Self {
            level: "info".to_string(),
            console: true,
            file: false,
            log_dir,
            redact_pii: true,
        }
    }
}

/// Initialise the global tracing subscriber with console and/or file output.
///
/// Call once at application startup. After this, all `tracing::info!()`,
/// `tracing::debug!()`, etc. macros will emit to the configured outputs.
pub fn init_logging(config: &LogConfig) -> MmResult<()> {
    // Build the env filter from config level
    let filter = EnvFilter::try_new(&config.level).unwrap_or_else(|_| {
        tracing::warn!("Invalid log level '{}', defaulting to 'info'", config.level);
        EnvFilter::new("info")
    });

    // Console layer: human-readable, coloured
    let console_layer = if config.console {
        Some(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .compact(),
        )
    } else {
        None
    };

    // File layer: JSON structured output
    let file_layer = if config.file {
        // Create log directory
        std::fs::create_dir_all(&config.log_dir).map_err(|e| {
            MmError::Logging(format!("cannot create log directory: {e}"))
        })?;

        // Create log file with date-based name
        let date = chrono::Utc::now().format("%Y-%m-%d");
        let log_path = config.log_dir.join(format!("meedya-{date}.log"));
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .map_err(|e| {
                MmError::Logging(format!("cannot open log file: {e}"))
            })?;

        Some(
            tracing_subscriber::fmt::layer()
                .with_writer(std::sync::Mutex::new(file))
                .json()
                .with_target(true)
                .with_span_events(FmtSpan::CLOSE),
        )
    } else {
        None
    };

    // Build and set the global subscriber
    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .try_init()
        .map_err(|e| MmError::Logging(format!("cannot initialise logging: {e}")))?;

    tracing::info!("Logging initialised (level: {})", config.level);
    Ok(())
}

/// Redact a file path for logging purposes.
///
/// Replaces the home directory portion with `~` and optionally hashes
/// the filename to prevent PII leakage in logs.
pub fn redact_path(path: &Path, hash_filename: bool) -> String {
    let path_str = path.to_string_lossy();

    // Replace home directory with ~
    let redacted = if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if path_str.starts_with(home_str.as_ref()) {
            path_str.replacen(home_str.as_ref(), "~", 1)
        } else {
            path_str.to_string()
        }
    } else {
        path_str.to_string()
    };

    if hash_filename {
        // Hash just the filename component
        let p = Path::new(&redacted);
        if let (Some(parent), Some(name)) = (p.parent(), p.file_name()) {
            let hash = hash_string(&name.to_string_lossy());
            format!("{}/{}", parent.display(), hash)
        } else {
            redacted
        }
    } else {
        redacted
    }
}

/// Redact a username or personal identifier.
///
/// Replaces all but the first character with asterisks.
pub fn redact_username(username: &str) -> String {
    if username.is_empty() {
        return String::new();
    }
    if username.len() <= 2 {
        return "*".repeat(username.len());
    }
    let first = &username[..1];
    let rest = "*".repeat(username.len() - 1);
    format!("{first}{rest}")
}

/// Generate a short hash of a string for redaction purposes.
///
/// Returns the first 8 hex characters of the SHA-256 hash.
pub fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..8].to_string()
}

/// Parse a log level string into a tracing Level.
pub fn parse_level(level: &str) -> Level {
    match level.to_ascii_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" | "warning" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_path_replaces_home() {
        if let Some(home) = dirs::home_dir() {
            let path = home.join("Music").join("song.mp3");
            let redacted = redact_path(&path, false);
            assert!(redacted.starts_with("~"));
            assert!(redacted.contains("Music"));
            assert!(redacted.contains("song.mp3"));
        }
    }

    #[test]
    fn redact_path_with_hash() {
        if let Some(home) = dirs::home_dir() {
            let path = home.join("Music").join("secret_song.mp3");
            let redacted = redact_path(&path, true);
            assert!(redacted.starts_with("~"));
            assert!(!redacted.contains("secret_song"));
        }
    }

    #[test]
    fn redact_path_no_home() {
        let path = Path::new("/opt/media/song.mp3");
        let redacted = redact_path(path, false);
        assert_eq!(redacted, "/opt/media/song.mp3");
    }

    #[test]
    fn redact_username_basic() {
        assert_eq!(redact_username("alice"), "a****");
        assert_eq!(redact_username("bob"), "b**");
    }

    #[test]
    fn redact_username_short() {
        assert_eq!(redact_username("a"), "*");
        assert_eq!(redact_username("ab"), "**");
    }

    #[test]
    fn redact_username_empty() {
        assert_eq!(redact_username(""), "");
    }

    #[test]
    fn hash_string_deterministic() {
        let hash1 = hash_string("test");
        let hash2 = hash_string("test");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 8);
    }

    #[test]
    fn hash_string_different_inputs() {
        let hash1 = hash_string("hello");
        let hash2 = hash_string("world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn parse_level_valid() {
        assert_eq!(parse_level("trace"), Level::TRACE);
        assert_eq!(parse_level("debug"), Level::DEBUG);
        assert_eq!(parse_level("info"), Level::INFO);
        assert_eq!(parse_level("warn"), Level::WARN);
        assert_eq!(parse_level("warning"), Level::WARN);
        assert_eq!(parse_level("error"), Level::ERROR);
    }

    #[test]
    fn parse_level_case_insensitive() {
        assert_eq!(parse_level("INFO"), Level::INFO);
        assert_eq!(parse_level("Debug"), Level::DEBUG);
        assert_eq!(parse_level("ERROR"), Level::ERROR);
    }

    #[test]
    fn parse_level_unknown_defaults_to_info() {
        assert_eq!(parse_level("invalid"), Level::INFO);
        assert_eq!(parse_level(""), Level::INFO);
    }

    #[test]
    fn default_log_config() {
        let config = LogConfig::default();
        assert_eq!(config.level, "info");
        assert!(config.console);
        assert!(!config.file);
        assert!(config.redact_pii);
    }

    #[test]
    fn log_dir_contains_meedyamanager() {
        let config = LogConfig::default();
        let path_str = config.log_dir.to_string_lossy();
        assert!(path_str.contains("MeedyaManager"));
    }
}
