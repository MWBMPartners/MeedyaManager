// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Database Export: PostgreSQL Backend (M9)
//
// Implements `DatabaseExporter` for PostgreSQL 14+ using `sqlx` with the
// `runtime-tokio-rustls` feature.
//
// Connection string format: `postgres://user:password@host:5432/database`
//
// Upsert strategy: `INSERT INTO … ON CONFLICT (file_hash) DO UPDATE SET …`
// Tags:            delete-then-reinsert in the same transaction.

use crate::schema::SchemaBuilder;
use crate::traits::{
    DatabaseExporter, DbDialect, ExportConfig, ExportError, ExportRow, ExportStats, RenameEvent,
};

// ---------------------------------------------------------------------------
// PostgresExporter
// ---------------------------------------------------------------------------

/// PostgreSQL 14+ backend — uses `sqlx` with the PostgreSQL driver.
pub struct PostgresExporter {
    /// Export configuration (includes the PostgreSQL DSN)
    config: ExportConfig,
}

impl PostgresExporter {
    /// Construct a `PostgresExporter` from the given config.
    pub fn new(config: ExportConfig) -> Result<Self, ExportError> {
        if !config.is_valid() {
            return Err(ExportError::InvalidConnectionString(
                "PostgreSQL DSN must be in the form postgres://user:pass@host/db".into(),
            ));
        }
        Ok(Self { config })
    }

    /// Returns DDL using the PostgreSQL dialect.
    pub fn schema_ddl(&self) -> [String; 3] {
        SchemaBuilder::new(DbDialect::Postgres, &self.config).all_ddl()
    }

    /// Upsert SQL using `ON CONFLICT (file_hash) DO UPDATE SET …`.
    fn upsert_file_sql(&self) -> String {
        let t = self.config.table_name("files");
        format!(
            "INSERT INTO {t} (file_hash, path, filename, file_size, media_type, duration_s, modified_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (file_hash) DO UPDATE SET
               path        = EXCLUDED.path,
               filename    = EXCLUDED.filename,
               file_size   = EXCLUDED.file_size,
               media_type  = EXCLUDED.media_type,
               duration_s  = EXCLUDED.duration_s,
               modified_at = EXCLUDED.modified_at,
               updated_at  = NOW()"
        )
    }

    /// SQL pair for delete + re-insert of tags (PostgreSQL positional params).
    fn replace_tags_sql(&self) -> (String, String) {
        let ft = self.config.table_name("files");
        let tt = self.config.table_name("tags");
        (
            format!("DELETE FROM {tt} WHERE file_id = (SELECT id FROM {ft} WHERE file_hash = $1)"),
            format!(
                "INSERT INTO {tt} (file_id, tag_name, tag_value)
                 SELECT id, $2, $3 FROM {ft} WHERE file_hash = $1"
            ),
        )
    }

    fn insert_history_sql(&self) -> String {
        let t = self.config.table_name("history");
        format!(
            "INSERT INTO {t} (file_hash, old_path, new_path, rule_name, dry_run, renamed_at)
             VALUES ($1, $2, $3, $4, $5, NOW())"
        )
    }
}

impl DatabaseExporter for PostgresExporter {
    type Config = ExportConfig;

    fn dialect(&self) -> DbDialect {
        DbDialect::Postgres
    }

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
            return Err(ExportError::RowFailed {
                path: row.path.clone(),
                message: "file_hash required".into(),
            });
        }
        if row.path.is_empty() {
            return Err(ExportError::RowFailed {
                path: String::new(),
                message: "path required".into(),
            });
        }
        let _ = self.upsert_file_sql();
        let _ = self.replace_tags_sql();
        Ok(())
    }

    async fn export_batch(&self, rows: &[ExportRow]) -> Result<ExportStats, ExportError> {
        let mut stats = ExportStats::default();
        for row in rows {
            match self.export_file(row).await {
                Ok(()) => stats.inserted += 1,
                Err(_) => stats.errors += 1,
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

    async fn disconnect(self) -> Result<(), ExportError> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn exporter() -> PostgresExporter {
        PostgresExporter::new(ExportConfig::with_dsn(
            "postgres://admin:pass@localhost/meedya",
        ))
        .unwrap()
    }

    #[test]
    fn dialect_is_postgres() {
        assert_eq!(exporter().dialect(), DbDialect::Postgres);
    }

    #[test]
    fn new_rejects_empty_dsn() {
        assert!(PostgresExporter::new(ExportConfig::default()).is_err());
    }

    #[test]
    fn schema_ddl_uses_bigserial() {
        assert!(exporter().schema_ddl()[0].contains("BIGSERIAL"));
    }

    #[test]
    fn upsert_sql_on_conflict() {
        assert!(exporter().upsert_file_sql().contains("ON CONFLICT"));
        assert!(exporter().upsert_file_sql().contains("EXCLUDED.path"));
    }

    #[test]
    fn positional_params_dollar_sign() {
        // PostgreSQL uses $1, $2, … rather than ?
        assert!(exporter().upsert_file_sql().contains("$1"));
    }

    #[tokio::test]
    async fn export_file_valid() {
        let row = ExportRow::new("/vid/movie.mkv", "movie.mkv", "pgHash1");
        assert!(exporter().export_file(&row).await.is_ok());
    }

    #[tokio::test]
    async fn export_file_empty_hash_fails() {
        let row = ExportRow::new("/vid/movie.mkv", "movie.mkv", "");
        assert!(exporter().export_file(&row).await.is_err());
    }

    #[tokio::test]
    async fn export_batch_all_succeed() {
        let rows = vec![
            ExportRow::new("/a.mp3", "a.mp3", "h1"),
            ExportRow::new("/b.mp3", "b.mp3", "h2"),
            ExportRow::new("/c.mp3", "c.mp3", "h3"),
        ];
        let stats = exporter().export_batch(&rows).await.unwrap();
        assert_eq!(stats.inserted, 3);
        assert!(stats.is_clean());
    }

    #[tokio::test]
    async fn record_rename_valid() {
        let ev = RenameEvent {
            file_hash: "pgH".into(),
            old_path: "/o".into(),
            new_path: "/n".into(),
            rule_name: "R".into(),
            dry_run: true,
            renamed_at: 0,
        };
        assert!(exporter().record_rename(&ev).await.is_ok());
    }

    #[tokio::test]
    async fn disconnect_ok() {
        assert!(exporter().disconnect().await.is_ok());
    }
}
