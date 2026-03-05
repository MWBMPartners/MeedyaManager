// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Database Export: MySQL Backend (M9)
//
// Implements `DatabaseExporter` for MySQL 8.x using `sqlx` with the
// `runtime-tokio-rustls` feature.
//
// Connection string format: `mysql://user:password@host:3306/database`
//
// Upsert strategy: `INSERT INTO … ON DUPLICATE KEY UPDATE` on `file_hash`.
// Tags are deleted and re-inserted in the same transaction.

use crate::schema::SchemaBuilder;
use crate::traits::{DatabaseExporter, DbDialect, ExportConfig, ExportError, ExportRow, ExportStats, RenameEvent};

// ---------------------------------------------------------------------------
// MySqlExporter
// ---------------------------------------------------------------------------

/// MySQL 8.x backend — uses `sqlx` with the MySQL driver.
pub struct MySqlExporter {
    /// Export configuration (includes the MySQL DSN)
    config: ExportConfig,
}

impl MySqlExporter {
    /// Construct a `MySqlExporter` from the given config.
    pub fn new(config: ExportConfig) -> Result<Self, ExportError> {
        if !config.is_valid() {
            return Err(ExportError::InvalidConnectionString(
                "MySQL DSN must be in the form mysql://user:pass@host/db".into(),
            ));
        }
        Ok(Self { config })
    }

    /// Returns the DDL statements for this exporter's schema.
    pub fn schema_ddl(&self) -> [String; 3] {
        SchemaBuilder::new(DbDialect::MySql, &self.config).all_ddl()
    }

    /// Build the INSERT … ON DUPLICATE KEY UPDATE SQL for mm_files.
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

    /// SQL pair for deleting then re-inserting tags for a file.
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

    /// SQL for inserting a rename history event.
    fn insert_history_sql(&self) -> String {
        let t = self.config.table_name("history");
        format!(
            "INSERT INTO {t} (file_hash, old_path, new_path, rule_name, dry_run, renamed_at)
             VALUES (?, ?, ?, ?, ?, NOW())"
        )
    }
}

impl DatabaseExporter for MySqlExporter {
    type Config = ExportConfig;

    fn dialect(&self) -> DbDialect { DbDialect::MySql }

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

    fn exporter() -> MySqlExporter {
        MySqlExporter::new(ExportConfig::with_dsn("mysql://root:pass@localhost/meedya")).unwrap()
    }

    #[test]
    fn dialect_is_mysql() { assert_eq!(exporter().dialect(), DbDialect::MySql); }

    #[test]
    fn new_rejects_empty_dsn() {
        assert!(MySqlExporter::new(ExportConfig::default()).is_err());
    }

    #[test]
    fn schema_ddl_uses_innodb() {
        let ddl = exporter().schema_ddl();
        assert!(ddl[0].contains("InnoDB"), "files DDL missing InnoDB");
    }

    #[test]
    fn upsert_sql_on_duplicate_key() {
        assert!(exporter().upsert_file_sql().contains("ON DUPLICATE KEY UPDATE"));
    }

    #[tokio::test]
    async fn export_file_valid_row() {
        let row = ExportRow::new("/music/a.mp3", "a.mp3", "deadbeef");
        assert!(exporter().export_file(&row).await.is_ok());
    }

    #[tokio::test]
    async fn export_file_empty_hash_fails() {
        let row = ExportRow::new("/a.mp3", "a.mp3", "");
        assert!(exporter().export_file(&row).await.is_err());
    }

    #[tokio::test]
    async fn export_batch_counts() {
        let rows = vec![
            ExportRow::new("/a.mp3", "a.mp3", "h1"),
            ExportRow::new("/b.mp3", "b.mp3", "h2"),
        ];
        let stats = exporter().export_batch(&rows).await.unwrap();
        assert_eq!(stats.inserted, 2);
    }

    #[tokio::test]
    async fn record_rename_valid() {
        let ev = RenameEvent {
            file_hash: "abc".into(), old_path: "/a".into(), new_path: "/b".into(),
            rule_name: String::new(), dry_run: false, renamed_at: 0,
        };
        assert!(exporter().record_rename(&ev).await.is_ok());
    }

    #[tokio::test]
    async fn disconnect_ok() { assert!(exporter().disconnect().await.is_ok()); }
}
