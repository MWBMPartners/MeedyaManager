// (C) 2025-2026 MWBM Partners Ltd
//
// Unified error types for the mm-core crate.
// Uses thiserror for ergonomic error definition with automatic
// Display and From implementations.

use thiserror::Error;

/// Top-level error type for mm-core operations.
///
/// Each variant maps to a specific subsystem so callers can match
/// on the error source without inspecting messages.
#[derive(Error, Debug)]
pub enum MmError {
    /// Configuration loading, parsing, or validation failed
    #[error("Configuration error: {0}")]
    Config(String),

    /// File system watcher initialisation or event error
    #[error("Watcher error: {0}")]
    Watcher(String),

    /// Rule engine parsing or evaluation error
    #[error("Rule engine error: {0}")]
    RuleEngine(String),

    /// Metadata extraction or writing error
    #[error("Metadata error: {0}")]
    Metadata(String),

    /// Media classification error
    #[error("Classification error: {0}")]
    Classify(String),

    /// Rename operation error (conflict, permission, path length)
    #[error("Rename error: {0}")]
    Rename(String),

    /// Companion file detection error
    #[error("Companion error: {0}")]
    Companion(String),

    /// Application state persistence error
    #[error("State error: {0}")]
    State(String),

    /// Logging initialisation error
    #[error("Logging error: {0}")]
    Logging(String),

    /// Health check failure
    #[error("Health check failed: {0}")]
    Health(String),

    /// File I/O error (auto-converted from std::io::Error)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Notify (file watcher) crate error
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),

    /// Lofty (metadata) crate error
    #[error("Lofty error: {0}")]
    Lofty(#[from] lofty::error::LoftyError),
}

/// Convenience type alias for mm-core Results
pub type MmResult<T> = Result<T, MmError>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify error Display formatting works for all variants
    #[test]
    fn error_display_formatting() {
        let err = MmError::Config("missing file".into());
        assert_eq!(err.to_string(), "Configuration error: missing file");

        let err = MmError::Watcher("permission denied".into());
        assert_eq!(err.to_string(), "Watcher error: permission denied");

        let err = MmError::RuleEngine("unexpected token".into());
        assert_eq!(err.to_string(), "Rule engine error: unexpected token");

        let err = MmError::Metadata("unsupported format".into());
        assert_eq!(err.to_string(), "Metadata error: unsupported format");

        let err = MmError::Classify("unknown extension".into());
        assert_eq!(err.to_string(), "Classification error: unknown extension");

        let err = MmError::Rename("destination exists".into());
        assert_eq!(err.to_string(), "Rename error: destination exists");

        let err = MmError::Health("disk full".into());
        assert_eq!(err.to_string(), "Health check failed: disk full");
    }

    /// Verify From<std::io::Error> conversion
    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let mm_err: MmError = io_err.into();
        assert!(matches!(mm_err, MmError::Io(_)));
        assert!(mm_err.to_string().contains("file not found"));
    }

    /// Verify From<serde_json::Error> conversion
    #[test]
    fn from_serde_error() {
        let result: Result<serde_json::Value, _> = serde_json::from_str("{invalid");
        let serde_err = result.unwrap_err();
        let mm_err: MmError = serde_err.into();
        assert!(matches!(mm_err, MmError::Serde(_)));
    }

    /// Verify MmResult type alias works
    #[test]
    fn result_type_alias() {
        let ok: MmResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: MmResult<i32> = Err(MmError::Config("bad".into()));
        assert!(err.is_err());
    }

    /// Verify Debug formatting is available
    #[test]
    fn error_debug_format() {
        let err = MmError::Config("test".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Config"));
    }
}
