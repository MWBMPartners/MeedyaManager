// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya export` Command (M9)
//
// Exports the scanned media library to a relational database.
// Supports SQLite, MySQL, MariaDB, PostgreSQL, and SQL Server.
//
// Usage:
//   meedya export --db sqlite:///path/to/library.db
//   meedya export --db mysql://user:pass@host/meedya --path /music
//   meedya export --db postgres://user:pass@host/meedya --dry-run
//   meedya export --db "server=tcp:host,1433;database=meedya;user=sa;password=P" \
//                 --path /media --backend mssql

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::Args;
use serde::Serialize;

// ─── Supported backends ─────────────────────────────────────────────────────

/// Database backend options available for export.
#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum BackendChoice {
    /// SQLite (local file or :memory:) — default, no server required
    Sqlite,
    /// MySQL 8.x
    Mysql,
    /// MariaDB 10.x / 11.x (MySQL-compatible wire protocol)
    Mariadb,
    /// PostgreSQL 14+
    Postgres,
    /// Microsoft SQL Server 2019+ (TDS protocol via Tiberius)
    Mssql,
}

impl std::fmt::Display for BackendChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite => write!(f, "SQLite"),
            Self::Mysql => write!(f, "MySQL"),
            Self::Mariadb => write!(f, "MariaDB"),
            Self::Postgres => write!(f, "PostgreSQL"),
            Self::Mssql => write!(f, "SQL Server"),
        }
    }
}

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya export` command.
#[derive(Args, Debug)]
pub struct ExportArgs {
    /// Database connection string / DSN
    ///
    /// Examples:
    ///   sqlite:///home/user/library.db
    ///   mysql://user:pass@localhost/meedya
    ///   postgres://user:pass@localhost/meedya
    ///   server=tcp:host,1433;database=meedya;user=sa;password=P
    #[arg(long, short = 'd', value_name = "DSN")]
    pub db: String,

    /// Path to scan and export (defaults to config's watch_paths)
    #[arg(long, short = 'p', value_name = "PATH")]
    pub path: Option<String>,

    /// Database backend to use (auto-detected from DSN prefix if omitted)
    #[arg(long, short = 'b', value_enum, default_value = "sqlite")]
    pub backend: BackendChoice,

    /// Custom table name prefix (default: "mm_")
    #[arg(long, default_value = "mm_")]
    pub prefix: String,

    /// Batch size for database transactions (default: 500 rows)
    #[arg(long, default_value_t = 500)]
    pub batch_size: usize,

    /// Skip schema initialisation — tables must already exist
    #[arg(long)]
    pub skip_schema: bool,

    /// Show the DDL that would be executed without running the export
    #[arg(long)]
    pub show_schema: bool,
}

// ─── JSON output structures ─────────────────────────────────────────────────

/// JSON-serialisable export result.
///
/// The export pipeline itself (scan → `mm-export` backend I/O) is not wired
/// up yet — see issues #113 and #118 — so this always reports
/// `"not_implemented"` rather than fabricating row counts nothing produced.
#[derive(Serialize)]
struct ExportOutput {
    /// Always `"not_implemented"` until the export pipeline lands.
    status: String,
    /// Backend that would be used, once implemented
    backend: String,
    /// Connection string (redacted — shows scheme + host only)
    connection: String,
    /// Explanation of why nothing was scanned or written
    message: String,
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Redact credentials from a DSN, keeping only the scheme + host for display.
fn redact_dsn(dsn: &str) -> String {
    // For SQLite file paths and SQL Server ADO strings, return a trimmed version
    if dsn.starts_with("sqlite") || dsn.starts_with("server=") {
        // Keep just the first 40 chars to avoid leaking a full ADO string
        let truncated = &dsn[..dsn.len().min(40)];
        return format!("{truncated}…");
    }
    // For URL-style DSNs strip user:pass
    if let Some(at_pos) = dsn.find('@') {
        if let Some(scheme_end) = dsn.find("://") {
            let scheme = &dsn[..scheme_end + 3];
            let host_onward = &dsn[at_pos + 1..];
            return format!("{scheme}***@{host_onward}");
        }
    }
    // Fallback: just truncate
    format!("{}…", &dsn[..dsn.len().min(30)])
}

/// Auto-detect the backend from the DSN scheme.
///
/// Deliberately matches on the DSN's *scheme* (its `xxx://` prefix, or the
/// ADO-style `server=` prefix used by SQL Server connection strings) rather
/// than scanning the whole string for a magic substring. The previous
/// implementation matched any DSN containing the literal text `"1433"` (the
/// SQL Server default port) as SQL Server — which misrouted a perfectly
/// ordinary SQLite path such as `sqlite:///data/1433.db` (a file merely
/// *named* after the port) to the wrong backend entirely.
pub fn detect_backend(dsn: &str) -> BackendChoice {
    let trimmed = dsn.trim();

    // SQL Server: either an ADO-style `server=...` connection string, or an
    // explicit `sqlserver://` / `mssql://` URI scheme.
    if trimmed.starts_with("server=")
        || trimmed.starts_with("sqlserver://")
        || trimmed.starts_with("mssql://")
    {
        return BackendChoice::Mssql;
    }

    if trimmed.starts_with("postgres://") || trimmed.starts_with("postgresql://") {
        return BackendChoice::Postgres;
    }
    if trimmed.starts_with("mysql://") {
        return BackendChoice::Mysql;
    }
    if trimmed.starts_with("mariadb://") {
        return BackendChoice::Mariadb;
    }

    // Everything else — an explicit `sqlite:`/`sqlite://` scheme, or a bare
    // filesystem path with no scheme at all — defaults to SQLite.
    BackendChoice::Sqlite
}

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya export` command.
///
/// `--show-schema` is fully functional — it renders the real DDL for the
/// selected backend via `mm-export::SchemaBuilder`. Everything else (the
/// actual scan-then-write pipeline) is not implemented yet: this reports
/// `NOT_IMPLEMENTED` rather than pretending an export ran. See issues #113
/// and #118.
pub fn run(ctx: &CliContext, args: &ExportArgs) -> anyhow::Result<i32> {
    // Validate the DSN is non-empty
    if args.db.trim().is_empty() {
        output::print_error("--db <DSN> is required. Use --help for examples.");
        return Ok(ExitCode::ERROR);
    }

    // Auto-detect backend from DSN if the default SQLite was passed but DSN
    // looks like a different backend
    let backend = if args.backend == BackendChoice::Sqlite {
        detect_backend(&args.db)
    } else {
        args.backend.clone()
    };

    let redacted = redact_dsn(&args.db);

    // --show-schema: print DDL and exit without running the export
    if args.show_schema {
        use mm_export::DbDialect;
        use mm_export::{ExportConfig, SchemaBuilder};

        let dialect = match backend {
            BackendChoice::Sqlite => DbDialect::Sqlite,
            BackendChoice::Mysql => DbDialect::MySql,
            BackendChoice::Mariadb => DbDialect::MariaDb,
            BackendChoice::Postgres => DbDialect::Postgres,
            BackendChoice::Mssql => DbDialect::SqlServer,
        };
        let mut cfg = ExportConfig::with_dsn(&args.db);
        args.prefix.clone_into(&mut cfg.table_prefix);
        cfg.batch_size = args.batch_size;

        let builder = SchemaBuilder::new(dialect, &cfg);
        for (i, stmt) in builder.all_ddl().iter().enumerate() {
            output::print_header(&format!("DDL statement {}", i + 1));
            println!("{stmt}");
        }
        return Ok(ExitCode::SUCCESS);
    }

    // Determine the scan path — reported in the settings table below so the
    // user can see what *would* be scanned, even though nothing is scanned.
    let scan_path = args.path.clone().unwrap_or_else(|| {
        ctx.config
            .watch
            .folders
            .first()
            .map_or_else(|| ".".to_string(), |p| p.to_string_lossy().into_owned())
    });

    // The export pipeline is not wired up in this release: no scan is run,
    // and `mm-export` never opens a database connection (SQLite, MySQL,
    // MariaDB, PostgreSQL, and SQL Server backends all exist as crates, but
    // nothing here calls into them). Rather than fabricate row counts for
    // work that never happened, report the resolved settings and say plainly
    // that nothing ran. A `--dry-run` request gets the identical answer — a
    // dry run of nothing is still nothing. Tracked by issues #113 (scan →
    // export wiring) and #118 (backend connection).
    const NOT_IMPLEMENTED_MESSAGE: &str = "`meedya export` is not implemented in this release — \
        nothing was scanned or written (see issues #113 and #118).";

    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&ExportOutput {
                status: "not_implemented".into(),
                backend: backend.to_string(),
                connection: redacted,
                message: NOT_IMPLEMENTED_MESSAGE.into(),
            });
        }
        OutputFormat::Human => {
            output::print_header(&format!("Export — {scan_path} → {backend}"));

            let rows = vec![
                vec!["Backend".into(), backend.to_string()],
                vec!["Connection".into(), redacted],
                vec!["Scan path".into(), scan_path],
                vec!["Table prefix".into(), args.prefix.clone()],
                vec!["Batch size".into(), args.batch_size.to_string()],
                vec!["Dry run".into(), ctx.dry_run.to_string()],
            ];
            output::print_table(&["Setting", "Value"], &rows);

            output::print_error(NOT_IMPLEMENTED_MESSAGE);
        }
    }

    Ok(ExitCode::NOT_IMPLEMENTED)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    fn test_ctx(json: bool, dry_run: bool) -> CliContext {
        CliContext {
            config: mm_core::config::AppConfig::default(),
            output: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Human
            },
            verbosity: 0,
            dry_run,
        }
    }

    fn sqlite_args() -> ExportArgs {
        ExportArgs {
            db: "sqlite://:memory:".into(),
            path: Some("/music".into()),
            backend: BackendChoice::Sqlite,
            prefix: "mm_".into(),
            batch_size: 500,
            skip_schema: false,
            show_schema: false,
        }
    }

    // --- detect_backend ---

    #[test]
    fn detect_sqlite_default() {
        assert_eq!(detect_backend("/home/user/lib.db"), BackendChoice::Sqlite);
        assert_eq!(
            detect_backend("sqlite:///home/user/lib.db"),
            BackendChoice::Sqlite
        );
    }

    #[test]
    fn detect_postgres() {
        assert_eq!(
            detect_backend("postgres://u:p@host/db"),
            BackendChoice::Postgres
        );
        assert_eq!(
            detect_backend("postgresql://u:p@host/db"),
            BackendChoice::Postgres
        );
    }

    #[test]
    fn detect_mysql() {
        assert_eq!(detect_backend("mysql://u:p@host/db"), BackendChoice::Mysql);
    }

    #[test]
    fn detect_mssql() {
        assert_eq!(
            detect_backend("server=tcp:host,1433;database=d"),
            BackendChoice::Mssql
        );
    }

    #[test]
    fn detect_mssql_scheme_prefix() {
        // Explicit URI-style schemes, not just the ADO `server=` form, must
        // also route to SQL Server.
        assert_eq!(
            detect_backend("sqlserver://sa:pass@host/meedya"),
            BackendChoice::Mssql
        );
        assert_eq!(
            detect_backend("mssql://sa:pass@host/meedya"),
            BackendChoice::Mssql
        );
    }

    #[test]
    fn detect_sqlite_path_containing_1433() {
        // Regression: the old detector matched on `dsn.contains("1433")`, so a
        // perfectly ordinary SQLite path that happens to contain the SQL
        // Server default port digits was misrouted to SQL Server.
        assert_eq!(
            detect_backend("sqlite:///data/1433.db"),
            BackendChoice::Sqlite
        );
    }

    // --- redact_dsn ---

    #[test]
    fn redact_hides_password() {
        let out = redact_dsn("postgres://admin:secret123@db.host/mydb");
        assert!(!out.contains("secret123"));
        assert!(out.contains("db.host"));
    }

    #[test]
    fn redact_sqlite_truncates() {
        let out = redact_dsn("sqlite:///very/long/path/to/my/library.db");
        assert!(out.ends_with('…'));
    }

    // --- BackendChoice display ---

    #[test]
    fn backend_display_names() {
        assert_eq!(BackendChoice::Sqlite.to_string(), "SQLite");
        assert_eq!(BackendChoice::Mysql.to_string(), "MySQL");
        assert_eq!(BackendChoice::Mariadb.to_string(), "MariaDB");
        assert_eq!(BackendChoice::Postgres.to_string(), "PostgreSQL");
        assert_eq!(BackendChoice::Mssql.to_string(), "SQL Server");
    }

    // --- run() ---

    #[test]
    fn run_empty_dsn_returns_error() {
        let ctx = test_ctx(false, false);
        let args = ExportArgs {
            db: "  ".into(),
            ..sqlite_args()
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::ERROR);
    }

    #[test]
    fn run_sqlite_human_reports_not_implemented() {
        let ctx = test_ctx(false, false);
        assert_eq!(
            run(&ctx, &sqlite_args()).unwrap(),
            ExitCode::NOT_IMPLEMENTED
        );
    }

    #[test]
    fn run_sqlite_json_reports_not_implemented() {
        let ctx = test_ctx(true, false);
        assert_eq!(
            run(&ctx, &sqlite_args()).unwrap(),
            ExitCode::NOT_IMPLEMENTED
        );
    }

    #[test]
    fn run_dry_run_reports_not_implemented() {
        // A dry run of nothing is still nothing — same outcome as a real run.
        let ctx = test_ctx(false, true);
        assert_eq!(
            run(&ctx, &sqlite_args()).unwrap(),
            ExitCode::NOT_IMPLEMENTED
        );
    }

    #[test]
    fn run_show_schema_exits_cleanly() {
        let ctx = test_ctx(false, false);
        let mut args = sqlite_args();
        args.show_schema = true;
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn run_postgres_backend_reports_not_implemented() {
        let ctx = test_ctx(false, false);
        let args = ExportArgs {
            db: "postgres://admin:pass@localhost/meedya".into(),
            backend: BackendChoice::Postgres,
            ..sqlite_args()
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::NOT_IMPLEMENTED);
    }
}
