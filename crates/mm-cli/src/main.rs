// (C) 2025-2026 MWBM Partners Ltd
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
//   meedya lookup <query>     — Search metadata providers
//   meedya config <action>    — Manage configuration
//   meedya report-bug         — Generate a bug report
//   meedya export             — Export library to database (M9)
//   meedya serve              — Start HTTPS media server (M10)
//   meedya service <action>   — Manage background service (install/start/stop/status)

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
/// A powerful CLI for scanning, organising, and enriching your media library.
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

    /// Search metadata providers for a query (not available in this alpha)
    Lookup(commands::lookup::LookupArgs),

    /// Manage MeedyaManager configuration
    Config(commands::config_cmd::ConfigArgs),

    /// Generate a bug report with system info and health checks
    #[command(name = "report-bug")]
    ReportBug(commands::report_bug::ReportBugArgs),

    /// Export media library metadata to a database (preview — not functional in this alpha)
    Export(commands::export::ExportArgs),

    /// Start the HTTPS media server with JWT authentication (preview — not functional in this alpha)
    Serve(commands::serve::ServeArgs),

    /// Manage the MeedyaManager background service (install/start/stop/status)
    Service(commands::service_cmd::ServiceArgs),
}

/// Application entry point.
///
/// Parses CLI arguments, builds the shared context, initialises logging,
/// and dispatches to the appropriate subcommand handler. The exit code
/// from the command handler is propagated to the process.
#[tokio::main]
async fn main() {
    // Initialise i18n — must run before any user-visible strings are produced
    mm_core::i18n::init();

    // Parse command-line arguments using clap derive
    let cli = Cli::parse();

    // Console verbosity comes from the -v count, never from settings.json5.
    // The CLI is user-facing, so its terminal output stays quiet by default
    // even when the config asks for a chatty *file* log.
    let console_level = match cli.verbose {
        0 => "warn", // Default: warnings only (CLI is user-facing)
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    // ── Bootstrap subscriber ────────────────────────────────────────────
    //
    // Chicken-and-egg: the real subscriber needs `logging.file` and
    // `logging.level` from the config, but loading the config itself emits
    // tracing events (which file it read, why it fell back to defaults) that
    // are exactly what a user debugging a bad settings.json5 needs to see.
    //
    // Solved with a THREAD-LOCAL default that covers only the config load.
    // `with_default` does not claim the process-global subscriber slot, so
    // the real subscriber installed below is still the first and only global
    // one — installing a global here would permanently block the file sink.
    let bootstrap = tracing_subscriber::fmt()
        .with_env_filter(console_level)
        .finish();

    // Build the shared CLI context (loads config, sets output format)
    let ctx_result = tracing::subscriber::with_default(bootstrap, || {
        CliContext::build(cli.config.as_deref(), cli.verbose, cli.json, cli.dry_run)
    });
    let ctx = match ctx_result {
        Ok(ctx) => ctx,
        Err(e) => {
            output::print_error(&format!("Failed to initialise: {e}"));
            std::process::exit(ExitCode::ERROR);
        }
    };

    // ── Real subscriber ─────────────────────────────────────────────────
    //
    // Issue #50: `init_logging` had zero callers, so the whole `[logging]`
    // section of settings.json5 — file path, level, PII redaction — did
    // nothing. Wire it up now that the config is in hand.
    let mut log_config = mm_core::logging::LogConfig::from_settings(&ctx.config.logging);
    // The -v flags own the console; settings.json5 owns the file.
    log_config.console_level = console_level.to_string();
    match mm_core::logging::init_logging(&log_config) {
        Ok(Some(path)) => {
            tracing::debug!(log_file = %path.display(), "File logging active");
        }
        Ok(None) => {
            // File logging is off — console only. Nothing to announce.
        }
        Err(e) => {
            // A logging failure (unwritable log directory, say) must never
            // stop the command the user actually asked for. Fall back to a
            // console-only subscriber so the run is not silent, and tell them
            // plainly why their configured log file is not being written.
            let _ = tracing_subscriber::fmt()
                .with_env_filter(console_level)
                .try_init();
            output::print_warning(&format!(
                "Could not initialise file logging: {e} — continuing with console output only"
            ));
        }
    }

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
        Some(Commands::Serve(ref args)) => commands::serve::run(&ctx, args),
        Some(Commands::Service(ref args)) => commands::service_cmd::run(&ctx, args),
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

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// `clap`'s own structural sanity check for the derived `Cli` definition
    /// — catches duplicate/conflicting short or long flags, malformed
    /// argument groups, and similar mistakes across every subcommand. These
    /// bugs would otherwise only surface at runtime (and only for whichever
    /// subcommand happens to get exercised), so this test panics loudly at
    /// `cargo test` time instead of waiting for a user to trip over it.
    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
