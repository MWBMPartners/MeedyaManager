// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Command-Line Interface
//
// Entry point for the `meedya` CLI binary. This provides terminal-based access
// to MeedyaManager's core functionality: scanning, organising, metadata lookup,
// and library management.
//
// Usage:
//   meedya scan <path>       — Scan a directory for media files
//   meedya organise <path>   — Organise media files using rules
//   meedya lookup <query>    — Look up metadata from providers
//   meedya config            — Manage configuration
//   meedya export            — Export library to a database

// Subcommand modules
mod commands;

use clap::Parser;

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
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Path to configuration file (defaults to settings.json5 in working directory)
    #[arg(short, long, global = true)]
    config: Option<String>,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Top-level CLI subcommands.
#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Scan a directory for media files and display results
    Scan {
        /// Path to the directory to scan
        path: String,
    },

    /// Organise media files according to configured rules
    Organise {
        /// Path to the directory to organise
        path: String,

        /// Perform a dry run (show what would change without modifying files)
        #[arg(long)]
        dry_run: bool,
    },

    /// Look up metadata for a media file or search query
    Lookup {
        /// Search query or file path
        query: String,
    },

    /// Manage MeedyaManager configuration
    Config,

    /// Export media library metadata to a database
    Export,
}

/// Application entry point.
///
/// Parses CLI arguments, initialises logging, and dispatches to the
/// appropriate subcommand handler.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command-line arguments using clap derive
    let cli = Cli::parse();

    // Initialise tracing subscriber with verbosity level
    let filter_level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter_level)
        .init();

    // Dispatch to the appropriate subcommand
    match cli.command {
        Some(Commands::Scan { path }) => {
            tracing::info!("Scanning directory: {}", path);
            // TODO: Implement scan logic via mm-core
            println!("Scanning: {path}");
        }
        Some(Commands::Organise { path, dry_run }) => {
            tracing::info!("Organising directory: {} (dry_run: {})", path, dry_run);
            // TODO: Implement organise logic via mm-core
            println!("Organising: {path} (dry_run: {dry_run})");
        }
        Some(Commands::Lookup { query }) => {
            tracing::info!("Looking up metadata: {}", query);
            // TODO: Implement lookup logic via mm-providers
            println!("Looking up: {query}");
        }
        Some(Commands::Config) => {
            tracing::info!("Managing configuration");
            // TODO: Implement config management
            println!("Config management (not yet implemented)");
        }
        Some(Commands::Export) => {
            tracing::info!("Exporting library");
            // TODO: Implement export logic via mm-export
            println!("Export (not yet implemented)");
        }
        None => {
            // No subcommand provided — print help
            use clap::CommandFactory;
            Cli::command().print_help()?;
            println!(); // Trailing newline after help output
        }
    }

    Ok(())
}
