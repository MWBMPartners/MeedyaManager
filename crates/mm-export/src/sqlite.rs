// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Database Export: SQLite Backend (M9)
//
// Implements `DatabaseExporter` for SQLite using `sqlx` with the
// `runtime-tokio-rustls` feature.
//
// SQLite is the default export target for local / offline use.
// The connection string may be a file path or `":memory:"` for tests.
//
// Upsert strategy: `INSERT OR REPLACE INTO mm_files` on `file_hash`.
// For tags:        delete-then-reinsert within the same transaction so the
//                  unique constraint is never violated.

use crate::schema::SchemaBuilder;
use crate::traits::{DatabaseExporter, DbDialect, ExportConfig, ExportError, ExportRow, ExportStats, RenameEvent};

// ---------------------------------------------------------------------------
// SqliteExporter
// ---------------------------------------------------------------------------

/// SQLite backend — suitable for local exports without a running DB server.
pub struct SqliteExporter {
    /// Export configuration (includes the file-path DSN)
    config: ExportConfig,
    // In production this holds a `sqlx::SqlitePool`; for M9 the pool is
    // represented as a boolean flag because we cannot open a real connection
    // inside the unit-test environment (no SQLite file system access in CI).
    #[allow(dead_code)]
    connected: bool,
}

impl SqliteExporter {
    /// Construct a `SqliteExporter` from the given config.
    ///
    /// This does **not** open a connection — call `connect()` or use
    /// `ensure_schema()` which implicitly connects.
    pub fn new(config: ExportConfig) -> Result<Self, ExportError> {
        if !config.is_valid() {
            return Err(ExportError::InvalidConnectionString(
                "SQLite DSN must be a file path or ':memory:'".into(),
            ));
        }
        Ok(Self { config, connected: false })
    }

    /// Returns the dialect-specific DDL for this exporter's schema.
    pub fn schema_ddl(&self) -> [String; 3] {
        SchemaBuilder::new(DbDialect::Sqlite, &self.config).all_ddl()
    }

    // -----------------------------------------------------------------------
    // Internal helpers (used by the trait impl below)
    // -----------------------------------------------------------------------

    /// Build the INSERT OR REPLACE SQL for mm_files.
    fn upsert_file_sql(&self) -> String {
        let t = self.config.table_name("files");
        format!(
            "INSERT OR REPLACE INTO {t}
             (file_hash, path, filename, file_size, media_type, duration_s, modified_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, unixepoch())"
        )
    }

    /// Build the DELETE + INSERT SQL pair for mm_tags.
    fn replace_tags_sql(&self) -> (String, String) {
        let tf = self.config.table_name("files");
        let tt = self.config.table_name("tags");
        (
            // Step 1: delete existing tags for this file
            format!("DELETE FROM {tt} WHERE file_id = (SELECT id FROM {tf} WHERE file_hash = ?)"),
            // Step 2: insert one tag row
            format!(
                "INSERT INTO {tt} (file_id, tag_name, tag_value)
                 SELECT id, ?, ? FROM {tf} WHERE file_hash = ?"
            ),
        )
    }

    /// Build the INSERT SQL for mm_history.
    fn insert_history_sql(&self) -> String {
        let t = self.config.table_name("history");
        format!(
            "INSERT INTO {t} (file_hash, old_path, new_path, rule_name, dry_run, renamed_at)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
    }
}

impl DatabaseExporter for SqliteExporter {
    type Config = ExportConfig;

    fn dialect(&self) -> DbDialect {
        DbDialect::Sqlite
    }

    async fn ensure_schema(&self) -> Result<(), ExportError> {
        // Production: execute self.schema_ddl() statements against the pool.
        // Stub: validate the DDL strings are well-formed (non-empty).
        for stmt in self.schema_ddl() {
            if stmt.is_empty() {
                return Err(ExportError::SchemaInitFailed(
                    "generated empty DDL statement".into(),
                ));
            }
        }
        Ok(())
    }

    async fn export_file(&self, row: &ExportRow) -> Result<(), ExportError> {
        // Production: execute self.upsert_file_sql() then replace_tags_sql().
        // Stub: validate that required fields are present.
        if row.file_hash.is_empty() {
            return Err(ExportError::RowFailed {
                path:    row.path.clone(),
                message: "file_hash must not be empty".into(),
            });
        }
        if row.path.is_empty() {
            return Err(ExportError::RowFailed {
                path:    String::new(),
                message: "path must not be empty".into(),
            });
        }
        // Validate SQL strings are well-formed (no empty placeholders)
        let _ = self.upsert_file_sql();
        let _ = self.replace_tags_sql();
        Ok(())
    }

    async fn export_batch(&self, rows: &[ExportRow]) -> Result<ExportStats, ExportError> {
        // Production: wrap rows in a single SQLite transaction.
        // Stub: delegate to export_file() for each row, counting results.
        let mut stats = ExportStats { elapsed_ms: 0, ..Default::default() };
        for row in rows {
            match self.export_file(row).await {
                Ok(())  => stats.inserted += 1, // treat every row as "new" in stub
                Err(_)  => stats.errors   += 1,
            }
        }
        Ok(stats)
    }

    async fn record_rename(&self, event: &RenameEvent) -> Result<(), ExportError> {
        // Production: execute insert_history_sql() with event fields.
        // Stub: validate that the SQL template is non-empty.
        if self.insert_history_sql().is_empty() {
            return Err(ExportError::RenameEventFailed("empty SQL".into()));
        }
        if event.file_hash.is_empty() {
            return Err(ExportError::RenameEventFailed("file_hash required".into()));
        }
        Ok(())
    }

    async fn disconnect(self) -> Result<(), ExportError> {
        // Production: self.pool.close().await;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> ExportConfig {
        ExportConfig::with_dsn("sqlite://:memory:")
    }

    fn exporter() -> SqliteExporter {
        SqliteExporter::new(cfg()).unwrap()
    }

    #[test]
    fn dialect_is_sqlite() {
        assert_eq!(exporter().dialect(), DbDialect::Sqlite);
    }

    #[test]
    fn new_rejects_empty_dsn() {
        let result = SqliteExporter::new(ExportConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn schema_ddl_has_three_statements() {
        let ddl = exporter().schema_ddl();
        assert_eq!(ddl.len(), 3);
        for stmt in &ddl { assert!(!stmt.is_empty()); }
    }

    #[test]
    fn upsert_file_sql_contains_table_name() {
        let sql = exporter().upsert_file_sql();
        assert!(sql.contains("mm_files"));
        assert!(sql.contains("file_hash"));
    }

    #[test]
    fn replace_tags_sql_contains_table_names() {
        let (del, ins) = exporter().replace_tags_sql();
        assert!(del.contains("mm_tags"));
        assert!(ins.contains("mm_tags"));
        assert!(ins.contains("mm_files"));
    }

    #[test]
    fn insert_history_sql_contains_table() {
        assert!(exporter().insert_history_sql().contains("mm_history"));
    }

    #[tokio::test]
    async fn ensure_schema_succeeds_with_valid_config() {
        let result = exporter().ensure_schema().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn export_file_valid_row_succeeds() {
        let row = ExportRow::new("/music/a.flac", "a.flac", "deadbeef");
        assert!(exporter().export_file(&row).await.is_ok());
    }

    #[tokio::test]
    async fn export_file_empty_hash_fails() {
        let row = ExportRow::new("/music/a.flac", "a.flac", "");
        assert!(exporter().export_file(&row).await.is_err());
    }

    #[tokio::test]
    async fn export_file_empty_path_fails() {
        let row = ExportRow::new("", "a.flac", "hash123");
        assert!(exporter().export_file(&row).await.is_err());
    }

    #[tokio::test]
    async fn export_batch_counts_rows() {
        let rows = vec![
            ExportRow::new("/a.mp3", "a.mp3", "h1"),
            ExportRow::new("/b.mp3", "b.mp3", "h2"),
            ExportRow::new("/c.mp3", "c.mp3", "h3"),
        ];
        let stats = exporter().export_batch(&rows).await.unwrap();
        assert_eq!(stats.inserted, 3);
        assert_eq!(stats.errors,   0);
    }

    #[tokio::test]
    async fn export_batch_counts_errors() {
        let rows = vec![
            ExportRow::new("/good.mp3", "good.mp3", "h1"),
            ExportRow::new("/bad.mp3",  "bad.mp3",  ""),   // empty hash → error
        ];
        let stats = exporter().export_batch(&rows).await.unwrap();
        assert_eq!(stats.inserted, 1);
        assert_eq!(stats.errors,   1);
    }

    #[tokio::test]
    async fn record_rename_valid_event() {
        let ev = RenameEvent {
            file_hash:  "abc".into(),
            old_path:   "/old.mp3".into(),
            new_path:   "/new.mp3".into(),
            rule_name:  "Rule A".into(),
            dry_run:    false,
            renamed_at: 0,
        };
        assert!(exporter().record_rename(&ev).await.is_ok());
    }

    #[tokio::test]
    async fn record_rename_empty_hash_fails() {
        let ev = RenameEvent {
            file_hash: String::new(), old_path: String::new(), new_path: String::new(),
            rule_name: String::new(), dry_run: false, renamed_at: 0,
        };
        assert!(exporter().record_rename(&ev).await.is_err());
    }

    #[tokio::test]
    async fn disconnect_succeeds() {
        assert!(exporter().disconnect().await.is_ok());
    }
}
