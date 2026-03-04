// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// Unified error types for the mm-core crate.
// Uses thiserror for ergonomic error definition.

use thiserror::Error;

/// Top-level error type for mm-core operations
#[derive(Error, Debug)]
pub enum MmError {
    /// Configuration loading or parsing failed
    #[error("Configuration error: {0}")]
    Config(String),

    /// File system watcher error
    #[error("Watcher error: {0}")]
    Watcher(String),

    /// Rule engine parsing or evaluation error
    #[error("Rule engine error: {0}")]
    RuleEngine(String),

    /// Metadata extraction or writing error
    #[error("Metadata error: {0}")]
    Metadata(String),

    /// File I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// Convenience type alias for mm-core Results
pub type MmResult<T> = Result<T, MmError>;
