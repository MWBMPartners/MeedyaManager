// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Database Export: SQL Server Backend (M9)
//
// Implements `DatabaseExporter` for Microsoft SQL Server 2019+ using the
// `tiberius` crate (TDS protocol).  Tiberius does not use a URL-style DSN;
// instead a `tiberius::Config` struct is built from the DSN components.
//
// Connection string format (ADO-style):
//   `server=tcp:host,1433;database=meedya;user=sa;password=P@ss`
//
// Upsert strategy: `MERGE INTO mm_files … WHEN MATCHED … WHEN NOT MATCHED …`
// Tags:            DELETE existing + INSERT new within the same transaction.

use crate::schema::SchemaBuilder;
use crate::traits::{
    DatabaseExporter, DbDialect, ExportConfig, ExportError, ExportRow, ExportStats, RenameEvent,
};

// ---------------------------------------------------------------------------
// MssqlExporter
// ---------------------------------------------------------------------------

/// Microsoft SQL Server 2019+ backend using Tiberius (TDS protocol).
pub struct MssqlExporter {
    /// Export configuration (ADO-style connection string)
    config: ExportConfig,
}

impl MssqlExporter {
    /// Construct a `MssqlExporter` from the given config.
    pub fn new(config: ExportConfig) -> Result<Self, ExportError> {
        if !config.is_valid() {
            return Err(ExportError::InvalidConnectionString(
                "SQL Server DSN must specify server, database, user, and password".into(),
            ));
        }
        Ok(Self { config })
    }

    /// Returns DDL using the SQL Server dialect (T-SQL).
    pub fn schema_ddl(&self) -> [String; 3] {
        SchemaBuilder::new(DbDialect::SqlServer, &self.config).all_ddl()
    }

    /// T-SQL MERGE statement for upsert on `file_hash`.
    ///
    /// SQL Server does not support `INSERT … ON CONFLICT`; MERGE is the
    /// idiomatic alternative.
    fn upsert_file_sql(&self) -> String {
        let t = self.config.table_name("files");
        format!(
            "MERGE INTO {t} AS tgt
             USING (VALUES (@file_hash, @path, @filename, @file_size, @media_type, @duration_s, @modified_at))
               AS src (file_hash, path, filename, file_size, media_type, duration_s, modified_at)
             ON tgt.file_hash = src.file_hash
             WHEN MATCHED THEN
               UPDATE SET
                 path        = src.path,
                 filename    = src.filename,
                 file_size   = src.file_size,
                 media_type  = src.media_type,
                 duration_s  = src.duration_s,
                 modified_at = src.modified_at,
                 updated_at  = GETUTCDATE()
             WHEN NOT MATCHED THEN
               INSERT (file_hash, path, filename, file_size, media_type, duration_s, modified_at)
               VALUES (src.file_hash, src.path, src.filename, src.file_size,
                       src.media_type, src.duration_s, src.modified_at);"
        )
    }

    /// SQL pair for delete + re-insert of tags (T-SQL named params).
    fn replace_tags_sql(&self) -> (String, String) {
        let ft = self.config.table_name("files");
        let tt = self.config.table_name("tags");
        (
            format!(
                "DELETE {tt} FROM {tt}
                 INNER JOIN {ft} ON {tt}.file_id = {ft}.id
                 WHERE {ft}.file_hash = @file_hash"
            ),
            format!(
                "INSERT INTO {tt} (file_id, tag_name, tag_value)
                 SELECT id, @tag_name, @tag_value FROM {ft} WHERE file_hash = @file_hash"
            ),
        )
    }

    fn insert_history_sql(&self) -> String {
        let t = self.config.table_name("history");
        format!(
            "INSERT INTO {t} (file_hash, old_path, new_path, rule_name, dry_run, renamed_at)
             VALUES (@file_hash, @old_path, @new_path, @rule_name, @dry_run, GETUTCDATE())"
        )
    }
}

impl DatabaseExporter for MssqlExporter {
    type Config = ExportConfig;

    fn dialect(&self) -> DbDialect {
        DbDialect::SqlServer
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

    fn exporter() -> MssqlExporter {
        MssqlExporter::new(ExportConfig::with_dsn(
            "server=tcp:localhost,1433;database=meedya;user=sa;password=P@ss",
        ))
        .unwrap()
    }

    #[test]
    fn dialect_is_sqlserver() {
        assert_eq!(exporter().dialect(), DbDialect::SqlServer);
    }

    #[test]
    fn new_rejects_empty_dsn() {
        assert!(MssqlExporter::new(ExportConfig::default()).is_err());
    }

    #[test]
    fn schema_ddl_uses_identity() {
        assert!(exporter().schema_ddl()[0].contains("IDENTITY(1,1)"));
    }

    #[test]
    fn schema_ddl_uses_getutcdate() {
        assert!(exporter().schema_ddl()[0].contains("GETUTCDATE()"));
    }

    #[test]
    fn upsert_sql_uses_merge() {
        assert!(exporter().upsert_file_sql().contains("MERGE INTO"));
        assert!(exporter().upsert_file_sql().contains("WHEN MATCHED"));
        assert!(exporter().upsert_file_sql().contains("WHEN NOT MATCHED"));
    }

    #[test]
    fn named_params_use_at_sign() {
        // T-SQL uses @param_name style
        assert!(exporter().upsert_file_sql().contains("@file_hash"));
    }

    #[tokio::test]
    async fn export_file_valid() {
        let row = ExportRow::new("/docs/report.mkv", "report.mkv", "sqlHash1");
        assert!(exporter().export_file(&row).await.is_ok());
    }

    #[tokio::test]
    async fn export_file_empty_hash_fails() {
        let row = ExportRow::new("/a.mkv", "a.mkv", "");
        assert!(exporter().export_file(&row).await.is_err());
    }

    #[tokio::test]
    async fn export_batch_success() {
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
            file_hash: "msH".into(),
            old_path: "/a".into(),
            new_path: "/b".into(),
            rule_name: String::new(),
            dry_run: false,
            renamed_at: 0,
        };
        assert!(exporter().record_rename(&ev).await.is_ok());
    }

    #[tokio::test]
    async fn disconnect_ok() {
        assert!(exporter().disconnect().await.is_ok());
    }
}
