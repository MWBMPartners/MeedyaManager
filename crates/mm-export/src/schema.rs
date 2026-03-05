// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Database Export: Schema DDL (M9)
//
// Provides dialect-appropriate `CREATE TABLE IF NOT EXISTS` DDL for the three
// core tables:
//
//   mm_files   — one row per unique media file (keyed by SHA-256 hash)
//   mm_tags    — key–value metadata tags associated with each file
//   mm_history — rename history (old path → new path, with rule name)
//
// The `SchemaBuilder` struct generates DDL strings for a given `DbDialect`
// and table-name prefix.  Actual execution is delegated to each backend's
// `ensure_schema()` method.

use crate::traits::{DbDialect, ExportConfig};

/// Generates `CREATE TABLE IF NOT EXISTS` DDL for all three export tables.
pub struct SchemaBuilder<'a> {
    /// Target database engine
    pub dialect: DbDialect,
    /// Export configuration (provides table prefix)
    pub config:  &'a ExportConfig,
}

impl<'a> SchemaBuilder<'a> {
    /// Create a new builder for the given dialect and config.
    pub fn new(dialect: DbDialect, config: &'a ExportConfig) -> Self {
        Self { dialect, config }
    }

    // -----------------------------------------------------------------------
    // mm_files
    // -----------------------------------------------------------------------

    /// DDL for the `mm_files` table.
    ///
    /// Primary key: `id` (auto-increment integer).
    /// Natural key: `file_hash` (SHA-256 hex, UNIQUE) — used for upserts.
    pub fn files_ddl(&self) -> String {
        let t = self.config.table_name("files");
        match self.dialect {
            DbDialect::SqlServer => format!(
                "IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = '{t}')
CREATE TABLE {t} (
    id          BIGINT IDENTITY(1,1) PRIMARY KEY,
    file_hash   VARCHAR(64)   NOT NULL UNIQUE,
    path        NVARCHAR(MAX) NOT NULL,
    filename    NVARCHAR(512) NOT NULL,
    file_size   BIGINT        NOT NULL DEFAULT 0,
    media_type  VARCHAR(128)  NOT NULL DEFAULT '',
    duration_s  INT           NOT NULL DEFAULT 0,
    modified_at BIGINT        NOT NULL DEFAULT 0,
    created_at  DATETIME2     NOT NULL DEFAULT GETUTCDATE(),
    updated_at  DATETIME2     NOT NULL DEFAULT GETUTCDATE()
);"
            ),
            DbDialect::Postgres => format!(
                "CREATE TABLE IF NOT EXISTS {t} (
    id          BIGSERIAL PRIMARY KEY,
    file_hash   VARCHAR(64)  NOT NULL UNIQUE,
    path        TEXT         NOT NULL,
    filename    TEXT         NOT NULL,
    file_size   BIGINT       NOT NULL DEFAULT 0,
    media_type  VARCHAR(128) NOT NULL DEFAULT '',
    duration_s  INTEGER      NOT NULL DEFAULT 0,
    modified_at BIGINT       NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);"
            ),
            // MySQL / MariaDB share DDL; MariaDB accepts all MySQL 8 syntax
            DbDialect::MySql | DbDialect::MariaDb => format!(
                "CREATE TABLE IF NOT EXISTS {t} (
    id          BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    file_hash   VARCHAR(64)     NOT NULL UNIQUE,
    path        TEXT            NOT NULL,
    filename    VARCHAR(512)    NOT NULL,
    file_size   BIGINT UNSIGNED NOT NULL DEFAULT 0,
    media_type  VARCHAR(128)    NOT NULL DEFAULT '',
    duration_s  INT UNSIGNED    NOT NULL DEFAULT 0,
    modified_at BIGINT          NOT NULL DEFAULT 0,
    created_at  DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;"
            ),
            DbDialect::Sqlite => format!(
                "CREATE TABLE IF NOT EXISTS {t} (
    id          INTEGER  PRIMARY KEY AUTOINCREMENT,
    file_hash   TEXT     NOT NULL UNIQUE,
    path        TEXT     NOT NULL,
    filename    TEXT     NOT NULL,
    file_size   INTEGER  NOT NULL DEFAULT 0,
    media_type  TEXT     NOT NULL DEFAULT '',
    duration_s  INTEGER  NOT NULL DEFAULT 0,
    modified_at INTEGER  NOT NULL DEFAULT 0,
    created_at  INTEGER  NOT NULL DEFAULT (unixepoch()),
    updated_at  INTEGER  NOT NULL DEFAULT (unixepoch())
);"
            ),
        }
    }

    // -----------------------------------------------------------------------
    // mm_tags
    // -----------------------------------------------------------------------

    /// DDL for the `mm_tags` table.
    ///
    /// Each row is one key–value metadata tag for a file.
    /// Foreign key `file_id` references `mm_files(id)`.
    pub fn tags_ddl(&self) -> String {
        let t  = self.config.table_name("tags");
        let ft = self.config.table_name("files");
        match self.dialect {
            DbDialect::SqlServer => format!(
                "IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = '{t}')
CREATE TABLE {t} (
    id        BIGINT IDENTITY(1,1) PRIMARY KEY,
    file_id   BIGINT        NOT NULL REFERENCES {ft}(id) ON DELETE CASCADE,
    tag_name  VARCHAR(128)  NOT NULL,
    tag_value NVARCHAR(MAX) NOT NULL DEFAULT '',
    UNIQUE (file_id, tag_name)
);"
            ),
            DbDialect::Postgres => format!(
                "CREATE TABLE IF NOT EXISTS {t} (
    id        BIGSERIAL PRIMARY KEY,
    file_id   BIGINT      NOT NULL REFERENCES {ft}(id) ON DELETE CASCADE,
    tag_name  VARCHAR(128) NOT NULL,
    tag_value TEXT         NOT NULL DEFAULT '',
    UNIQUE (file_id, tag_name)
);"
            ),
            DbDialect::MySql | DbDialect::MariaDb => format!(
                "CREATE TABLE IF NOT EXISTS {t} (
    id        BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    file_id   BIGINT UNSIGNED NOT NULL,
    tag_name  VARCHAR(128)    NOT NULL,
    tag_value TEXT            NOT NULL DEFAULT '',
    UNIQUE KEY uk_file_tag (file_id, tag_name),
    CONSTRAINT fk_tags_file FOREIGN KEY (file_id) REFERENCES {ft}(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;"
            ),
            DbDialect::Sqlite => format!(
                "CREATE TABLE IF NOT EXISTS {t} (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id   INTEGER NOT NULL REFERENCES {ft}(id) ON DELETE CASCADE,
    tag_name  TEXT    NOT NULL,
    tag_value TEXT    NOT NULL DEFAULT '',
    UNIQUE (file_id, tag_name)
);"
            ),
        }
    }

    // -----------------------------------------------------------------------
    // mm_history
    // -----------------------------------------------------------------------

    /// DDL for the `mm_history` table.
    ///
    /// Each row records one rename event (old path → new path).
    pub fn history_ddl(&self) -> String {
        let t = self.config.table_name("history");
        match self.dialect {
            DbDialect::SqlServer => format!(
                "IF NOT EXISTS (SELECT * FROM sys.tables WHERE name = '{t}')
CREATE TABLE {t} (
    id          BIGINT IDENTITY(1,1) PRIMARY KEY,
    file_hash   VARCHAR(64)   NOT NULL,
    old_path    NVARCHAR(MAX) NOT NULL,
    new_path    NVARCHAR(MAX) NOT NULL,
    rule_name   NVARCHAR(256) NOT NULL DEFAULT '',
    dry_run     BIT           NOT NULL DEFAULT 0,
    renamed_at  DATETIME2     NOT NULL DEFAULT GETUTCDATE()
);"
            ),
            DbDialect::Postgres => format!(
                "CREATE TABLE IF NOT EXISTS {t} (
    id         BIGSERIAL    PRIMARY KEY,
    file_hash  VARCHAR(64)  NOT NULL,
    old_path   TEXT         NOT NULL,
    new_path   TEXT         NOT NULL,
    rule_name  TEXT         NOT NULL DEFAULT '',
    dry_run    BOOLEAN      NOT NULL DEFAULT FALSE,
    renamed_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);"
            ),
            DbDialect::MySql | DbDialect::MariaDb => format!(
                "CREATE TABLE IF NOT EXISTS {t} (
    id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
    file_hash  VARCHAR(64)     NOT NULL,
    old_path   TEXT            NOT NULL,
    new_path   TEXT            NOT NULL,
    rule_name  VARCHAR(256)    NOT NULL DEFAULT '',
    dry_run    TINYINT(1)      NOT NULL DEFAULT 0,
    renamed_at DATETIME        NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;"
            ),
            DbDialect::Sqlite => format!(
                "CREATE TABLE IF NOT EXISTS {t} (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    file_hash  TEXT    NOT NULL,
    old_path   TEXT    NOT NULL,
    new_path   TEXT    NOT NULL,
    rule_name  TEXT    NOT NULL DEFAULT '',
    dry_run    INTEGER NOT NULL DEFAULT 0,
    renamed_at INTEGER NOT NULL DEFAULT (unixepoch())
);"
            ),
        }
    }

    /// Returns an ordered list of all three DDL statements for this dialect.
    /// The order (files → tags → history) respects foreign-key dependencies.
    pub fn all_ddl(&self) -> [String; 3] {
        [self.files_ddl(), self.tags_ddl(), self.history_ddl()]
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

    // ---- files DDL ----

    #[test]
    fn files_ddl_sqlite_contains_expected_columns() {
        let ddl = SchemaBuilder::new(DbDialect::Sqlite, &cfg()).files_ddl();
        assert!(ddl.contains("mm_files"));
        assert!(ddl.contains("file_hash"));
        assert!(ddl.contains("filename"));
        assert!(ddl.contains("AUTOINCREMENT"));
    }

    #[test]
    fn files_ddl_mysql_utf8mb4() {
        let ddl = SchemaBuilder::new(DbDialect::MySql, &cfg()).files_ddl();
        assert!(ddl.contains("utf8mb4"));
        assert!(ddl.contains("AUTO_INCREMENT"));
    }

    #[test]
    fn files_ddl_mariadb_same_as_mysql() {
        let mysql_ddl   = SchemaBuilder::new(DbDialect::MySql,   &cfg()).files_ddl();
        let mariadb_ddl = SchemaBuilder::new(DbDialect::MariaDb, &cfg()).files_ddl();
        assert_eq!(mysql_ddl, mariadb_ddl);
    }

    #[test]
    fn files_ddl_postgres_bigserial() {
        let ddl = SchemaBuilder::new(DbDialect::Postgres, &cfg()).files_ddl();
        assert!(ddl.contains("BIGSERIAL"));
        assert!(ddl.contains("TIMESTAMPTZ"));
    }

    #[test]
    fn files_ddl_sqlserver_identity() {
        let ddl = SchemaBuilder::new(DbDialect::SqlServer, &cfg()).files_ddl();
        assert!(ddl.contains("IDENTITY(1,1)"));
        assert!(ddl.contains("GETUTCDATE()"));
    }

    // ---- tags DDL ----

    #[test]
    fn tags_ddl_sqlite_references_files() {
        let ddl = SchemaBuilder::new(DbDialect::Sqlite, &cfg()).tags_ddl();
        assert!(ddl.contains("mm_tags"));
        assert!(ddl.contains("REFERENCES mm_files(id)"));
        assert!(ddl.contains("ON DELETE CASCADE"));
    }

    #[test]
    fn tags_ddl_postgres_unique_constraint() {
        let ddl = SchemaBuilder::new(DbDialect::Postgres, &cfg()).tags_ddl();
        assert!(ddl.contains("UNIQUE (file_id, tag_name)"));
    }

    #[test]
    fn tags_ddl_mysql_foreign_key() {
        let ddl = SchemaBuilder::new(DbDialect::MySql, &cfg()).tags_ddl();
        assert!(ddl.contains("FOREIGN KEY"));
        assert!(ddl.contains("ON DELETE CASCADE"));
    }

    // ---- history DDL ----

    #[test]
    fn history_ddl_all_dialects_contain_dry_run() {
        for dialect in [DbDialect::Sqlite, DbDialect::MySql, DbDialect::MariaDb,
                        DbDialect::Postgres, DbDialect::SqlServer]
        {
            let ddl = SchemaBuilder::new(dialect, &cfg()).history_ddl();
            assert!(ddl.contains("dry_run"), "missing dry_run in {dialect}");
            assert!(ddl.contains("renamed_at"), "missing renamed_at in {dialect}");
        }
    }

    #[test]
    fn history_ddl_sqlite_no_foreign_key() {
        // mm_history references file_hash (string), not a FK to mm_files
        let ddl = SchemaBuilder::new(DbDialect::Sqlite, &cfg()).history_ddl();
        assert!(!ddl.contains("REFERENCES"), "SQLite history should not have FK");
    }

    // ---- all_ddl ----

    #[test]
    fn all_ddl_returns_three_statements() {
        let ddl = SchemaBuilder::new(DbDialect::Sqlite, &cfg()).all_ddl();
        assert_eq!(ddl.len(), 3);
        // files first, then tags (FK dependency), then history
        assert!(ddl[0].contains("mm_files"));
        assert!(ddl[1].contains("mm_tags"));
        assert!(ddl[2].contains("mm_history"));
    }

    #[test]
    fn all_ddl_none_empty() {
        for dialect in [DbDialect::Sqlite, DbDialect::MySql, DbDialect::MariaDb,
                        DbDialect::Postgres, DbDialect::SqlServer]
        {
            let ddl = SchemaBuilder::new(dialect, &cfg()).all_ddl();
            for stmt in &ddl {
                assert!(!stmt.is_empty(), "empty DDL for {dialect}");
            }
        }
    }

    // ---- custom prefix ----

    #[test]
    fn custom_prefix_reflected_in_ddl() {
        let mut c = ExportConfig::with_dsn("sqlite://:memory:");
        c.table_prefix = "exp_".into();
        let ddl = SchemaBuilder::new(DbDialect::Sqlite, &c).files_ddl();
        assert!(ddl.contains("exp_files"));
        assert!(!ddl.contains("mm_files"));
    }
}
