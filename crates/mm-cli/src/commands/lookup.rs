// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya lookup` Command (Stub)
//
// Provider-based metadata search. This is a stub for M3 — full provider
// implementation comes in M5 (Metadata Lookup Providers).

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::Args;
use serde::Serialize;

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya lookup` command.
#[derive(Args, Debug)]
pub struct LookupArgs {
    /// Search query (artist name, album title, or filename)
    pub query: String,

    /// Specific provider to search (e.g. musicbrainz, discogs)
    #[arg(long)]
    pub provider: Option<String>,

    /// Auto-match from file metadata tags
    #[arg(long)]
    pub auto: bool,

    /// Apply matched metadata back to the file
    #[arg(long)]
    pub apply: bool,

    /// Process an entire directory
    #[arg(long)]
    pub batch: bool,
}

// ─── JSON output structures ─────────────────────────────────────────────────

/// Stub lookup result for JSON output.
#[derive(Serialize)]
struct LookupStubOutput {
    status: String,
    message: String,
    planned_providers: Vec<String>,
}

// ─── Planned providers list ─────────────────────────────────────────────────

/// List of providers planned for M5.
const PLANNED_PROVIDERS: &[&str] = &[
    // Music (10)
    "MusicBrainz",
    "Discogs",
    "Last.fm",
    "Spotify",
    "Apple Music",
    "Deezer",
    "Tidal",
    "Bandcamp",
    "Genius (lyrics)",
    "AcoustID (fingerprint)",
    // Video (5)
    "TMDb (movies)",
    "OMDb (movies)",
    "TheTVDB (TV shows)",
    "TVMaze (TV shows)",
    "OpenSubtitles",
    // Podcasts (1)
    "Apple Podcasts",
    // Identifiers (3)
    "ISRC",
    "UPC/EAN",
    "ISBN",
];

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya lookup` command (stub).
///
/// Displays a notice that providers are not yet implemented and lists the
/// planned providers coming in M5.
pub fn run(ctx: &CliContext, args: &LookupArgs) -> anyhow::Result<i32> {
    let message = format!(
        "Metadata lookup for '{}' is not yet available. Provider support is coming in M5.",
        args.query
    );

    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&LookupStubOutput {
                status: "stub".to_string(),
                message,
                planned_providers: PLANNED_PROVIDERS.iter().map(|s| s.to_string()).collect(),
            });
        }
        OutputFormat::Human => {
            output::print_warning(&message);
            output::print_header("Planned Providers (M5)");
            let rows: Vec<Vec<String>> = PLANNED_PROVIDERS
                .iter()
                .enumerate()
                .map(|(i, name)| vec![(i + 1).to_string(), name.to_string()])
                .collect();
            output::print_table(&["#", "Provider"], &rows);
        }
    }

    Ok(ExitCode::SUCCESS)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    fn test_ctx(json: bool) -> CliContext {
        CliContext {
            config: mm_core::config::AppConfig::default(),
            output: if json {
                OutputFormat::Json
            } else {
                OutputFormat::Human
            },
            verbosity: 0,
            dry_run: false,
        }
    }

    /// Lookup stub displays message and succeeds
    #[test]
    fn lookup_stub_message() {
        let ctx = test_ctx(false);
        let args = LookupArgs {
            query: "Beatles Abbey Road".to_string(),
            provider: None,
            auto: false,
            apply: false,
            batch: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    /// Lookup stub in JSON mode
    #[test]
    fn lookup_stub_json() {
        let ctx = test_ctx(true);
        let args = LookupArgs {
            query: "test query".to_string(),
            provider: Some("musicbrainz".to_string()),
            auto: true,
            apply: false,
            batch: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }
}
