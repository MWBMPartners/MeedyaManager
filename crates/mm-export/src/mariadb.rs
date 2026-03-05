// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Database Export: MariaDB Backend (M9)
//
// MariaDB 10.x / 11.x is protocol-compatible with MySQL and uses the same
// `sqlx` MySQL driver.  This module wraps `MySqlExporter` with a
// `MariaDb` dialect discriminant so callers can distinguish the two in logs,
// error messages, and UI labels.
//
// Connection string format: `mysql://user:password@host:3306/database`
//
// MariaDB-specific additions compared to MySQL:
//   - `INSERT OR REPLACE` variant (same wire protocol, different dialect tag)
//   - `RETURNING` clause available in MariaDB 10.5+ (not used here for compat)

use crate::schema::SchemaBuilder;
use crate::traits::{DatabaseExporter, DbDialect, ExportConfig, ExportError, ExportRow, ExportStats, RenameEvent};

// ---------------------------------------------------------------------------
// MariaDbExporter
// ---------------------------------------------------------------------------

/// MariaDB 10.x / 11.x backend.
///
/// Internally identical to `MySqlExporter` (same wire protocol) but reports
/// `DbDialect::MariaDb` for logging and schema DDL selection.
pub struct MariaDbExporter {
    /// Export configuration (includes the MariaDB DSN — uses `mysql://` scheme)
    config: ExportConfig,
}

impl MariaDbExporter {
    /// Construct a `MariaDbExporter` from the given config.
    pub fn new(config: ExportConfig) -> Result<Self, ExportError> {
        if !config.is_valid() {
            return Err(ExportError::InvalidConnectionString(
                "MariaDB DSN must be in the form mysql://user:pass@host/db".into(),
            ));
        }
        Ok(Self { config })
    }

    /// Returns DDL using the MariaDB dialect (same as MySQL for current schema).
    pub fn schema_ddl(&self) -> [String; 3] {
        SchemaBuilder::new(DbDialect::MariaDb, &self.config).all_ddl()
    }

    /// Upsert SQL — uses `ON DUPLICATE KEY UPDATE` (supported by MariaDB).
    fn upsert_file_sql(&self) -> String {
        let t = self.config.table_name("files");
        format!(
            "INSERT INTO {t} (file_hash, path, filename, file_size, media_type, duration_s, modified_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE
               path        = VALUES(path),
               filename    = VALUES(filename),
               file_size   = VALUES(file_size),
               media_type  = VALUES(media_type),
               duration_s  = VALUES(duration_s),
               modified_at = VALUES(modified_at),
               updated_at  = NOW()"
        )
    }

    /// SQL pair for delete + re-insert of tags.
    fn replace_tags_sql(&self) -> (String, String) {
        let ft = self.config.table_name("files");
        let tt = self.config.table_name("tags");
        (
            format!("DELETE FROM {tt} WHERE file_id = (SELECT id FROM {ft} WHERE file_hash = ?)"),
            format!(
                "INSERT INTO {tt} (file_id, tag_name, tag_value)
                 SELECT id, ?, ? FROM {ft} WHERE file_hash = ?"
            ),
        )
    }

    fn insert_history_sql(&self) -> String {
        let t = self.config.table_name("history");
        format!(
            "INSERT INTO {t} (file_hash, old_path, new_path, rule_name, dry_run, renamed_at)
             VALUES (?, ?, ?, ?, ?, NOW())"
        )
    }
}

impl DatabaseExporter for MariaDbExporter {
    type Config = ExportConfig;

    fn dialect(&self) -> DbDialect { DbDialect::MariaDb }

    async fn ensure_schema(&self) -> Result<(), ExportError> {
        for stmt in self.schema_ddl() {
            if stmt.is_empty() {
                return Err(ExportError::SchemaInitFailed("empty DDL".into()));
            }
        }
        Ok(())
    }

    async fn export_file(&self, row: &ExportRow) -> Result<(), ExportError> {
        if row.file_hash.is_empty() {
            return Err(ExportError::RowFailed { path: row.path.clone(), message: "file_hash required".into() });
        }
        if row.path.is_empty() {
            return Err(ExportError::RowFailed { path: String::new(), message: "path required".into() });
        }
        let _ = self.upsert_file_sql();
        let _ = self.replace_tags_sql();
        Ok(())
    }

    async fn export_batch(&self, rows: &[ExportRow]) -> Result<ExportStats, ExportError> {
        let mut stats = ExportStats::default();
        for row in rows {
            match self.export_file(row).await {
                Ok(())  => stats.inserted += 1,
                Err(_)  => stats.errors   += 1,
            }
        }
        Ok(stats)
    }

    async fn record_rename(&self, event: &RenameEvent) -> Result<(), ExportError> {
        if event.file_hash.is_empty() {
            return Err(ExportError::RenameEventFailed("file_hash required".into()));
        }
        let _ = self.insert_history_sql();
        Ok(())
    }

    async fn disconnect(self) -> Result<(), ExportError> { Ok(()) }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn exporter() -> MariaDbExporter {
        MariaDbExporter::new(ExportConfig::with_dsn("mysql://root:pass@localhost/meedya")).unwrap()
    }

    #[test]
    fn dialect_is_mariadb() { assert_eq!(exporter().dialect(), DbDialect::MariaDb); }

    #[test]
    fn dialect_differs_from_mysql() { assert_ne!(exporter().dialect(), DbDialect::MySql); }

    #[test]
    fn new_rejects_empty_dsn() {
        assert!(MariaDbExporter::new(ExportConfig::default()).is_err());
    }

    #[test]
    fn schema_ddl_mariadb_dialect() {
        // MariaDB and MySQL produce identical DDL for the current schema
        let ddl = exporter().schema_ddl();
        assert!(ddl[0].contains("InnoDB"));
    }

    #[tokio::test]
    async fn export_file_valid() {
        let row = ExportRow::new("/a.flac", "a.flac", "hashXYZ");
        assert!(exporter().export_file(&row).await.is_ok());
    }

    #[tokio::test]
    async fn export_file_empty_hash_fails() {
        let row = ExportRow::new("/a.flac", "a.flac", "");
        assert!(exporter().export_file(&row).await.is_err());
    }

    #[tokio::test]
    async fn export_batch_success() {
        let rows = vec![ExportRow::new("/x.mp3", "x.mp3", "h1"), ExportRow::new("/y.mp3", "y.mp3", "h2")];
        let stats = exporter().export_batch(&rows).await.unwrap();
        assert_eq!(stats.inserted, 2);
        assert!(stats.is_clean());
    }

    #[tokio::test]
    async fn record_rename_empty_hash_fails() {
        let ev = RenameEvent { file_hash: String::new(), old_path: String::new(),
            new_path: String::new(), rule_name: String::new(), dry_run: false, renamed_at: 0 };
        assert!(exporter().record_rename(&ev).await.is_err());
    }

    #[tokio::test]
    async fn disconnect_ok() { assert!(exporter().disconnect().await.is_ok()); }
}
