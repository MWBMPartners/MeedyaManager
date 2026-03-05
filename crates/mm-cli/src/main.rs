// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Command-Line Interface
//
// Entry point for the `meedya` CLI binary. Provides terminal-based access
// to MeedyaManager's core functionality: scanning, organising, metadata
// editing, rule testing, file watching, and bug reporting.
//
// Usage:
//   meedya scan <path>        — Scan a directory for media files
//   meedya debug <file>       — Inspect a single file's metadata
//   meedya edit <file>        — Edit metadata tags and cover art
//   meedya rule <action>      — Validate templates and list tags
//   meedya watch [paths]      — Watch directories for changes
//   meedya lookup <query>     — Search metadata providers (stub — M5)
//   meedya config <action>    — Manage configuration
//   meedya report-bug         — Generate a bug report
//   meedya export             — Export library to database (M9)
//   meedya serve              — Start HTTPS media server (M10)

// Subcommand modules
mod commands;
// Shared CLI context (config + output format + flags)
mod context;
// Output formatting helpers (tables, JSON, colours)
mod output;

use clap::Parser;
use context::CliContext;
use output::ExitCode;

/// MeedyaManager — Cross-platform media file manager and auto-organizer.
///
/// A powerful CLI for scanning, organising, and enriching your media library
/// with metadata from 19+ providers.
#[derive(Parser, Debug)]
#[command(
    name = "meedya",
    version,
    author,
    about = "MeedyaManager — Cross-platform media file manager and auto-organizer"
)]
struct Cli {
    /// Enable verbose logging output (repeat for more detail: -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Path to configuration file (defaults to platform config directory)
    #[arg(short, long, global = true)]
    config: Option<String>,

    /// Emit machine-parseable JSON output instead of coloured tables
    #[arg(long, global = true)]
    json: bool,

    /// Dry-run mode — preview changes without modifying any files
    #[arg(long, global = true)]
    dry_run: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Top-level CLI subcommands.
#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Scan a directory for media files and preview renames
    Scan(commands::scan::ScanArgs),

    /// Inspect a single file's metadata, classification, and properties
    Debug(commands::debug::DebugArgs),

    /// Edit metadata tags and cover art on a media file
    Edit(commands::edit::EditArgs),

    /// Validate templates, list tags, and test rules against files
    Rule(commands::rule::RuleArgs),

    /// Watch directories for media file changes
    Watch(commands::watch::WatchArgs),

    /// Search metadata providers for a query (coming in M5)
    Lookup(commands::lookup::LookupArgs),

    /// Manage MeedyaManager configuration
    Config(commands::config_cmd::ConfigArgs),

    /// Generate a bug report with system info and health checks
    #[command(name = "report-bug")]
    ReportBug(commands::report_bug::ReportBugArgs),

    /// Export media library metadata to a database (M9)
    Export(commands::export::ExportArgs),

    /// Start the HTTPS media server with JWT authentication (M10)
    Serve(commands::serve::ServeArgs),
}

/// Application entry point.
///
/// Parses CLI arguments, builds the shared context, initialises logging,
/// and dispatches to the appropriate subcommand handler. The exit code
/// from the command handler is propagated to the process.
#[tokio::main]
async fn main() {
    // Parse command-line arguments using clap derive
    let cli = Cli::parse();

    // Initialise tracing subscriber with verbosity level
    let filter_level = match cli.verbose {
        0 => "warn", // Default: warnings only (CLI is user-facing)
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    // Ignore subscriber init errors (e.g. if already initialised in tests)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter_level)
        .try_init();

    // Build the shared CLI context (loads config, sets output format)
    let ctx = match CliContext::build(cli.config.as_deref(), cli.verbose, cli.json, cli.dry_run) {
        Ok(ctx) => ctx,
        Err(e) => {
            output::print_error(&format!("Failed to initialise: {e}"));
            std::process::exit(ExitCode::ERROR);
        }
    };

    // Dispatch to the appropriate subcommand handler
    let exit_code = match cli.command {
        Some(Commands::Scan(ref args)) => commands::scan::run(&ctx, args),
        Some(Commands::Debug(ref args)) => commands::debug::run(&ctx, args),
        Some(Commands::Edit(ref args)) => commands::edit::run(&ctx, args),
        Some(Commands::Rule(ref args)) => commands::rule::run(&ctx, args),
        Some(Commands::Watch(ref args)) => commands::watch::run(&ctx, args).await,
        Some(Commands::Lookup(ref args)) => commands::lookup::run(&ctx, args),
        Some(Commands::Config(ref args)) => commands::config_cmd::run(&ctx, args),
        Some(Commands::ReportBug(ref args)) => commands::report_bug::run(&ctx, args),
        Some(Commands::Export(ref args)) => commands::export::run(&ctx, args),
        Some(Commands::Serve(ref args))  => commands::serve::run(&ctx, args),
        None => {
            // No subcommand provided — print help
            use clap::CommandFactory;
            Cli::command().print_help().ok();
            println!(); // Trailing newline after help output
            Ok(ExitCode::SUCCESS)
        }
    };

    // Map the result to a process exit code
    match exit_code {
        Ok(code) => {
            if code != ExitCode::SUCCESS {
                std::process::exit(code);
            }
        }
        Err(e) => {
            output::print_error(&format!("{e}"));
            std::process::exit(ExitCode::ERROR);
        }
    }
}
