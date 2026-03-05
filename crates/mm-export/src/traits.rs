// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Database Export: Shared Traits & Types (M9)
//
// Defines:
//   - `DatabaseExporter`  — async trait implemented by every backend
//   - `ExportRow`         — a single media file row ready for insert/upsert
//   - `RenameEvent`       — a single rename history record
//   - `ExportConfig`      — connection + behaviour settings
//   - `ExportStats`       — counters returned after an export run
//   - `ExportError`       — typed error enum for all failure modes
//   - `DbDialect`         — discriminant identifying the target database engine

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

// ---------------------------------------------------------------------------
// DbDialect — identifies the target database engine
// ---------------------------------------------------------------------------

/// Discriminates between the supported database engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DbDialect {
    /// MySQL 8.x (or compatible forks)
    MySql,
    /// MariaDB 10.x / 11.x
    MariaDb,
    /// PostgreSQL 14+
    Postgres,
    /// SQLite 3.x (local file or `:memory:`)
    Sqlite,
    /// Microsoft SQL Server 2019+ (TDS protocol via Tiberius)
    SqlServer,
}

impl fmt::Display for DbDialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbDialect::MySql      => write!(f, "MySQL"),
            DbDialect::MariaDb    => write!(f, "MariaDB"),
            DbDialect::Postgres   => write!(f, "PostgreSQL"),
            DbDialect::Sqlite     => write!(f, "SQLite"),
            DbDialect::SqlServer  => write!(f, "SQL Server"),
        }
    }
}

// ---------------------------------------------------------------------------
// ExportRow — one media file ready for upsert
// ---------------------------------------------------------------------------

/// A single media file record to be upserted into the `mm_files` +
/// `mm_tags` tables.
///
/// `file_hash` is a SHA-256 hex string used as the natural key for upserts so
/// renaming a file does not duplicate its row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportRow {
    /// Absolute path to the media file at the time of export
    pub path:        String,
    /// Bare filename (without directory)
    pub filename:    String,
    /// File size in bytes
    pub file_size:   u64,
    /// SHA-256 hex digest of the file contents (natural upsert key)
    pub file_hash:   String,
    /// Key–value map of audio/video metadata tags (title, artist, album, etc.)
    pub tags:        HashMap<String, String>,
    /// MIME / media type string (e.g. `"audio/flac"`, `"video/mp4"`)
    pub media_type:  String,
    /// Duration in seconds (0 if unknown)
    pub duration_s:  u32,
    /// UTC Unix timestamp when the file was last modified on disk
    pub modified_at: i64,
}

impl ExportRow {
    /// Create a minimal `ExportRow` with only the required fields set; all
    /// optional fields are set to their zero values.
    pub fn new(
        path:      impl Into<String>,
        filename:  impl Into<String>,
        file_hash: impl Into<String>,
    ) -> Self {
        Self {
            path:        path.into(),
            filename:    filename.into(),
            file_size:   0,
            file_hash:   file_hash.into(),
            tags:        HashMap::new(),
            media_type:  String::new(),
            duration_s:  0,
            modified_at: 0,
        }
    }

    /// Returns `true` if the row has at least one metadata tag.
    pub fn has_tags(&self) -> bool {
        !self.tags.is_empty()
    }

    /// Returns the value of a tag, if present.
    pub fn tag(&self, key: &str) -> Option<&str> {
        self.tags.get(key).map(|s| s.as_str())
    }
}

// ---------------------------------------------------------------------------
// RenameEvent — one rename history record
// ---------------------------------------------------------------------------

/// Records that a file was renamed (or would be renamed in dry-run mode).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameEvent {
    /// SHA-256 hex digest of the file (links back to `mm_files.file_hash`)
    pub file_hash:  String,
    /// Absolute path before the rename
    pub old_path:   String,
    /// Absolute path after the rename
    pub new_path:   String,
    /// Name of the rule that triggered this rename (empty if manual)
    pub rule_name:  String,
    /// `true` if this was a dry-run preview only; `false` if the rename executed
    pub dry_run:    bool,
    /// UTC Unix timestamp when the rename event was recorded
    pub renamed_at: i64,
}

// ---------------------------------------------------------------------------
// ExportConfig — connection + behaviour parameters
// ---------------------------------------------------------------------------

/// Configuration passed to every `DatabaseExporter` implementation.
///
/// For SQLite the `connection_string` is the file path (or `":memory:"`).
/// For all other backends it is a standard DSN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Database DSN / connection string (format varies by backend)
    pub connection_string:  String,
    /// Maximum number of rows to upsert in a single batch transaction (default 500)
    pub batch_size:         usize,
    /// Schema / database name to use when creating tables (empty = use DSN default)
    pub schema_name:        String,
    /// Table name prefix (default `"mm_"`)
    pub table_prefix:       String,
    /// When `true`, skip the `ensure_schema()` call — tables must already exist
    pub skip_schema_init:   bool,
    /// Connection timeout in seconds (default 30)
    pub connect_timeout_s:  u64,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            connection_string: String::new(),
            batch_size:        500,
            schema_name:       String::new(),
            table_prefix:      "mm_".into(),
            skip_schema_init:  false,
            connect_timeout_s: 30,
        }
    }
}

impl ExportConfig {
    /// Construct a config with just a connection string, using all other defaults.
    pub fn with_dsn(dsn: impl Into<String>) -> Self {
        Self { connection_string: dsn.into(), ..Default::default() }
    }

    /// Returns the fully-qualified table name for `suffix` (e.g. `"mm_files"`).
    pub fn table_name(&self, suffix: &str) -> String {
        format!("{}{}", self.table_prefix, suffix)
    }

    /// Returns `true` if the config has a non-empty connection string.
    pub fn is_valid(&self) -> bool {
        !self.connection_string.is_empty()
    }
}

// ---------------------------------------------------------------------------
// ExportStats — counters returned after an export run
// ---------------------------------------------------------------------------

/// Aggregated counters produced by an export operation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExportStats {
    /// Number of rows successfully inserted (new files)
    pub inserted:   u64,
    /// Number of rows successfully updated (existing files with changed metadata)
    pub updated:    u64,
    /// Number of rows skipped because the hash matched and no fields changed
    pub skipped:    u64,
    /// Number of rows that produced an error
    pub errors:     u64,
    /// Total elapsed time in milliseconds
    pub elapsed_ms: u64,
}

impl ExportStats {
    /// Total number of rows processed (inserted + updated + skipped + errors).
    pub fn total(&self) -> u64 {
        self.inserted + self.updated + self.skipped + self.errors
    }

    /// Total number of rows successfully persisted (inserted + updated).
    pub fn persisted(&self) -> u64 {
        self.inserted + self.updated
    }

    /// Returns `true` if no errors occurred during the export.
    pub fn is_clean(&self) -> bool {
        self.errors == 0
    }

    /// Merge `other` into `self` (accumulate multiple batch results).
    pub fn merge(&mut self, other: &ExportStats) {
        self.inserted   += other.inserted;
        self.updated    += other.updated;
        self.skipped    += other.skipped;
        self.errors     += other.errors;
        self.elapsed_ms += other.elapsed_ms;
    }
}

// ---------------------------------------------------------------------------
// ExportError — typed failure modes
// ---------------------------------------------------------------------------

/// All failure modes that can arise during a database export operation.
#[derive(Debug, Error)]
pub enum ExportError {
    /// The connection DSN is empty, missing required fields, or malformed
    #[error("Invalid connection string: {0}")]
    InvalidConnectionString(String),

    /// The underlying database driver returned a connection error
    #[error("Connection failed ({dialect}): {message}")]
    ConnectionFailed { dialect: DbDialect, message: String },

    /// Schema initialisation (`CREATE TABLE IF NOT EXISTS`) failed
    #[error("Schema init failed: {0}")]
    SchemaInitFailed(String),

    /// A single-row upsert failed
    #[error("Row export failed for {path}: {message}")]
    RowFailed { path: String, message: String },

    /// A batch transaction was rolled back
    #[error("Batch transaction failed after {count} rows: {message}")]
    BatchFailed { count: usize, message: String },

    /// A rename event insert failed
    #[error("Rename event insert failed: {0}")]
    RenameEventFailed(String),

    /// The operation was cancelled by the caller
    #[error("Export cancelled")]
    Cancelled,

    /// Serialisation / deserialisation error
    #[error("Serialisation error: {0}")]
    Serialisation(String),

    /// The requested feature is not supported by this backend
    #[error("Unsupported by {dialect}: {feature}")]
    Unsupported { dialect: DbDialect, feature: String },
}

impl ExportError {
    /// Returns `true` if the error is transient and the operation can be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ExportError::ConnectionFailed { .. } | ExportError::BatchFailed { .. }
        )
    }
}

// ---------------------------------------------------------------------------
// DatabaseExporter — async trait (RPITIT, Rust 2024)
// ---------------------------------------------------------------------------

/// Shared interface implemented by all five database export backends.
///
/// Because the trait uses RPITIT it is not object-safe; callers use generic
/// bounds: `T: DatabaseExporter`.
pub trait DatabaseExporter {
    /// The concrete config type (same `ExportConfig` for all current backends).
    type Config: Clone;

    /// Returns the `DbDialect` of this backend.
    fn dialect(&self) -> DbDialect;

    /// Create table schema if it does not already exist (idempotent).
    fn ensure_schema(&self) -> impl std::future::Future<Output = Result<(), ExportError>> + Send;

    /// Insert or update a single `ExportRow` (upsert on `file_hash`).
    fn export_file(&self, row: &ExportRow) -> impl std::future::Future<Output = Result<(), ExportError>> + Send;

    /// Upsert a slice of rows in a single transaction.
    fn export_batch(&self, rows: &[ExportRow]) -> impl std::future::Future<Output = Result<ExportStats, ExportError>> + Send;

    /// Append a rename event to `mm_history`.
    fn record_rename(&self, event: &RenameEvent) -> impl std::future::Future<Output = Result<(), ExportError>> + Send;

    /// Gracefully close the connection pool.
    fn disconnect(self) -> impl std::future::Future<Output = Result<(), ExportError>> + Send;
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- DbDialect ---

    #[test]
    fn dialect_display_mysql()     { assert_eq!(DbDialect::MySql.to_string(),     "MySQL"); }
    #[test]
    fn dialect_display_mariadb()   { assert_eq!(DbDialect::MariaDb.to_string(),   "MariaDB"); }
    #[test]
    fn dialect_display_postgres()  { assert_eq!(DbDialect::Postgres.to_string(),  "PostgreSQL"); }
    #[test]
    fn dialect_display_sqlite()    { assert_eq!(DbDialect::Sqlite.to_string(),    "SQLite"); }
    #[test]
    fn dialect_display_sqlserver() { assert_eq!(DbDialect::SqlServer.to_string(), "SQL Server"); }

    #[test]
    fn dialect_equality() {
        assert_eq!(DbDialect::MySql, DbDialect::MySql);
        assert_ne!(DbDialect::MySql, DbDialect::MariaDb);
    }

    // --- ExportRow ---

    #[test]
    fn export_row_new_minimal() {
        let row = ExportRow::new("/music/song.flac", "song.flac", "abc123");
        assert_eq!(row.path,      "/music/song.flac");
        assert_eq!(row.filename,  "song.flac");
        assert_eq!(row.file_hash, "abc123");
        assert_eq!(row.file_size, 0);
        assert!(!row.has_tags());
    }

    #[test]
    fn export_row_has_tags_when_populated() {
        let mut row = ExportRow::new("/a.mp3", "a.mp3", "hash1");
        row.tags.insert("title".into(), "Test".into());
        assert!(row.has_tags());
        assert_eq!(row.tag("title"),  Some("Test"));
        assert_eq!(row.tag("artist"), None);
    }

    #[test]
    fn export_row_tag_accessor() {
        let mut row = ExportRow::new("/b.mp3", "b.mp3", "hash2");
        row.tags.insert("album".into(), "Greatest Hits".into());
        assert_eq!(row.tag("album"), Some("Greatest Hits"));
    }

    // --- ExportConfig ---

    #[test]
    fn export_config_default_values() {
        let cfg = ExportConfig::default();
        assert_eq!(cfg.batch_size, 500);
        assert_eq!(cfg.table_prefix, "mm_");
        assert!(!cfg.is_valid());
        assert!(!cfg.skip_schema_init);
        assert_eq!(cfg.connect_timeout_s, 30);
    }

    #[test]
    fn export_config_with_dsn_is_valid() {
        let cfg = ExportConfig::with_dsn("sqlite://:memory:");
        assert!(cfg.is_valid());
    }

    #[test]
    fn export_config_table_name() {
        let cfg = ExportConfig::with_dsn("sqlite://:memory:");
        assert_eq!(cfg.table_name("files"),   "mm_files");
        assert_eq!(cfg.table_name("tags"),    "mm_tags");
        assert_eq!(cfg.table_name("history"), "mm_history");
    }

    #[test]
    fn export_config_custom_prefix() {
        let mut cfg = ExportConfig::with_dsn("postgres://localhost/test");
        cfg.table_prefix = "meedya_".into();
        assert_eq!(cfg.table_name("files"), "meedya_files");
    }

    // --- ExportStats ---

    #[test]
    fn export_stats_total_and_persisted() {
        let s = ExportStats { inserted: 10, updated: 5, skipped: 3, errors: 2, elapsed_ms: 100 };
        assert_eq!(s.total(),     20);
        assert_eq!(s.persisted(), 15);
        assert!(!s.is_clean());
    }

    #[test]
    fn export_stats_clean_run() {
        let s = ExportStats { inserted: 7, updated: 2, skipped: 0, errors: 0, elapsed_ms: 50 };
        assert!(s.is_clean());
        assert_eq!(s.total(), 9);
    }

    #[test]
    fn export_stats_default_zeros() {
        let s = ExportStats::default();
        assert_eq!(s.total(), 0);
        assert!(s.is_clean());
    }

    #[test]
    fn export_stats_merge() {
        let mut a = ExportStats { inserted: 5, updated: 2, skipped: 1, errors: 0, elapsed_ms: 30 };
        let     b = ExportStats { inserted: 3, updated: 1, skipped: 0, errors: 1, elapsed_ms: 20 };
        a.merge(&b);
        assert_eq!(a.inserted,   8);
        assert_eq!(a.updated,    3);
        assert_eq!(a.errors,     1);
        assert_eq!(a.elapsed_ms, 50);
        assert!(!a.is_clean());
    }

    // --- ExportError ---

    #[test]
    fn export_error_display_invalid_dsn() {
        let e = ExportError::InvalidConnectionString("empty".into());
        assert!(e.to_string().contains("Invalid connection string"));
    }

    #[test]
    fn export_error_display_connection_failed() {
        let e = ExportError::ConnectionFailed { dialect: DbDialect::MySql, message: "refused".into() };
        assert!(e.to_string().contains("MySQL"));
        assert!(e.to_string().contains("refused"));
    }

    #[test]
    fn export_error_retryable() {
        assert!(ExportError::ConnectionFailed { dialect: DbDialect::Sqlite, message: String::new() }.is_retryable());
        assert!(ExportError::BatchFailed { count: 5, message: String::new() }.is_retryable());
        assert!(!ExportError::Cancelled.is_retryable());
        assert!(!ExportError::SchemaInitFailed("x".into()).is_retryable());
    }

    #[test]
    fn export_error_unsupported() {
        let e = ExportError::Unsupported { dialect: DbDialect::Sqlite, feature: "full-text search".into() };
        assert!(e.to_string().contains("SQLite"));
        assert!(e.to_string().contains("full-text search"));
    }

    // --- RenameEvent ---

    #[test]
    fn rename_event_fields() {
        let ev = RenameEvent {
            file_hash:  "abc".into(),
            old_path:   "/old/file.mp3".into(),
            new_path:   "/new/file.mp3".into(),
            rule_name:  "Music Rule".into(),
            dry_run:    false,
            renamed_at: 1_700_000_000,
        };
        assert!(!ev.dry_run);
        assert_eq!(ev.rule_name, "Music Rule");
        assert_ne!(ev.old_path, ev.new_path);
    }

    #[test]
    fn rename_event_dry_run_flag() {
        let ev = RenameEvent {
            file_hash: String::new(), old_path: String::new(), new_path: String::new(),
            rule_name: String::new(), dry_run: true, renamed_at: 0,
        };
        assert!(ev.dry_run);
    }
}
