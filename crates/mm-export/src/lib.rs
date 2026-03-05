// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Database Export (M9)
//
// Exports media library metadata to relational databases.
//
// Supported backends:
//   - MySQL 8.x         (via sqlx)
//   - MariaDB 10.x/11.x (via sqlx MySQL driver)
//   - PostgreSQL 14+    (via sqlx)
//   - SQLite 3.x        (via sqlx)
//   - SQL Server 2019+  (via tiberius / TDS)
//
// Public API surface:
//   - `traits::{DatabaseExporter, DbDialect, ExportConfig, ExportRow, ExportStats,
//               ExportError, RenameEvent}`
//   - `schema::SchemaBuilder`
//   - Backend structs: `SqliteExporter`, `MySqlExporter`, `MariaDbExporter`,
//     `PostgresExporter`, `MssqlExporter`

// --- Module declarations ---

/// Shared trait, types, and error enum for all database backends.
pub mod traits;

/// SQL DDL generation for all five dialects.
pub mod schema;

/// MySQL 8.x exporter.
pub mod mysql;

/// PostgreSQL 14+ exporter.
pub mod postgres;

/// SQLite 3.x exporter (default for local / offline use).
pub mod sqlite;

/// MariaDB 10.x / 11.x exporter (MySQL wire protocol).
pub mod mariadb;

/// Microsoft SQL Server 2019+ exporter (TDS protocol via Tiberius).
pub mod mssql;

// --- Convenience re-exports ---

/// All public types needed to use mm-export from other crates.
pub use traits::{
    DatabaseExporter, DbDialect, ExportConfig, ExportError, ExportRow, ExportStats, RenameEvent,
};
pub use schema::SchemaBuilder;
pub use mysql::MySqlExporter;
pub use mariadb::MariaDbExporter;
pub use postgres::PostgresExporter;
pub use sqlite::SqliteExporter;
pub use mssql::MssqlExporter;

// ---------------------------------------------------------------------------
// Integration / smoke tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Dialect coverage ----

    #[test]
    fn all_five_dialects_are_distinct() {
        let dialects = [
            DbDialect::MySql,
            DbDialect::MariaDb,
            DbDialect::Postgres,
            DbDialect::Sqlite,
            DbDialect::SqlServer,
        ];
        // Every dialect must have a unique Display string
        let labels: Vec<String> = dialects.iter().map(|d| d.to_string()).collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), 5, "dialect labels must all be unique");
    }

    #[test]
    fn sqlite_exporter_dialect() {
        let e = SqliteExporter::new(ExportConfig::with_dsn("sqlite://:memory:")).unwrap();
        assert_eq!(e.dialect(), DbDialect::Sqlite);
    }

    #[test]
    fn mysql_exporter_dialect() {
        let e = MySqlExporter::new(ExportConfig::with_dsn("mysql://u:p@h/db")).unwrap();
        assert_eq!(e.dialect(), DbDialect::MySql);
    }

    #[test]
    fn mariadb_exporter_dialect() {
        let e = MariaDbExporter::new(ExportConfig::with_dsn("mysql://u:p@h/db")).unwrap();
        assert_eq!(e.dialect(), DbDialect::MariaDb);
    }

    #[test]
    fn postgres_exporter_dialect() {
        let e = PostgresExporter::new(ExportConfig::with_dsn("postgres://u:p@h/db")).unwrap();
        assert_eq!(e.dialect(), DbDialect::Postgres);
    }

    #[test]
    fn mssql_exporter_dialect() {
        let e = MssqlExporter::new(ExportConfig::with_dsn("server=tcp:h,1433;database=d;user=u;password=P")).unwrap();
        assert_eq!(e.dialect(), DbDialect::SqlServer);
    }

    // ---- Schema DDL coverage ----

    #[test]
    fn all_backends_produce_three_ddl_statements() {
        let cfgs: Vec<(&str, DbDialect)> = vec![
            ("sqlite://:memory:",           DbDialect::Sqlite),
            ("mysql://u:p@h/db",            DbDialect::MySql),
            ("mysql://u:p@h/db",            DbDialect::MariaDb),
            ("postgres://u:p@h/db",         DbDialect::Postgres),
            ("server=tcp:h;database=d;user=u;password=P", DbDialect::SqlServer),
        ];
        for (dsn, dialect) in cfgs {
            let cfg = ExportConfig::with_dsn(dsn);
            let ddl = SchemaBuilder::new(dialect, &cfg).all_ddl();
            assert_eq!(ddl.len(), 3, "{dialect} should produce 3 DDL stmts");
            for stmt in &ddl {
                assert!(!stmt.is_empty(), "{dialect}: empty DDL stmt");
            }
        }
    }

    // ---- ExportRow round-trip ----

    #[test]
    fn export_row_serialise_deserialise() {
        let mut row = ExportRow::new("/music/song.flac", "song.flac", "aabbcc");
        row.tags.insert("title".into(), "My Song".into());
        row.tags.insert("artist".into(), "The Band".into());
        row.file_size  = 12_000_000;
        row.duration_s = 240;

        let json = serde_json::to_string(&row).expect("serialise");
        let back: ExportRow = serde_json::from_str(&json).expect("deserialise");

        assert_eq!(row, back);
        assert_eq!(back.tag("title"), Some("My Song"));
    }

    // ---- ExportStats accumulation ----

    #[test]
    fn export_stats_accumulate_across_batches() {
        let mut total = ExportStats::default();
        let batch1 = ExportStats { inserted: 100, updated: 20, skipped: 5, errors: 0, elapsed_ms: 50 };
        let batch2 = ExportStats { inserted: 80,  updated: 10, skipped: 3, errors: 2, elapsed_ms: 40 };
        total.merge(&batch1);
        total.merge(&batch2);

        assert_eq!(total.inserted,   180);
        assert_eq!(total.updated,     30);
        assert_eq!(total.skipped,      8);
        assert_eq!(total.errors,       2);
        assert_eq!(total.elapsed_ms,  90);
        assert_eq!(total.total(),    220);
        assert!(!total.is_clean());
    }

    // ---- ExportConfig table naming ----

    #[test]
    fn default_table_prefix_is_mm_underscore() {
        let cfg = ExportConfig::with_dsn("sqlite://:memory:");
        assert_eq!(cfg.table_name("files"),   "mm_files");
        assert_eq!(cfg.table_name("tags"),    "mm_tags");
        assert_eq!(cfg.table_name("history"), "mm_history");
    }

    // ---- ExportError retryability ----

    #[test]
    fn connection_errors_are_retryable() {
        let e = ExportError::ConnectionFailed { dialect: DbDialect::Postgres, message: "timeout".into() };
        assert!(e.is_retryable());
    }

    #[test]
    fn schema_errors_are_not_retryable() {
        let e = ExportError::SchemaInitFailed("permission denied".into());
        assert!(!e.is_retryable());
    }

    // ---- Backend construction ----

    #[test]
    fn all_backends_reject_empty_dsn() {
        let empty = ExportConfig::default();
        assert!(SqliteExporter::new(empty.clone()).is_err());
        assert!(MySqlExporter::new(empty.clone()).is_err());
        assert!(MariaDbExporter::new(empty.clone()).is_err());
        assert!(PostgresExporter::new(empty.clone()).is_err());
        assert!(MssqlExporter::new(empty).is_err());
    }
}
