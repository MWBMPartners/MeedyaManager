// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — CLI Context
//
// Shared context threaded through all CLI command handlers. Holds the loaded
// configuration, output format preference, verbosity level, and dry-run flag.
// Built once at startup in `main.rs` and passed by reference to each command.

use crate::output::OutputFormat;
use std::path::Path;

// ─── CLI context ────────────────────────────────────────────────────────────

/// Shared context available to all CLI command handlers.
///
/// Created once at startup from global CLI flags and the loaded configuration.
/// Each command receives an immutable reference to this struct.
#[derive(Debug)]
pub struct CliContext {
    /// Loaded application configuration (from file or defaults)
    pub config: mm_core::config::AppConfig,
    /// Output format — Human (coloured tables) or Json (machine-parseable)
    pub output: OutputFormat,
    /// Verbosity level (0 = info, 1 = debug, 2+ = trace)
    #[allow(dead_code)] // Used for future verbosity-dependent output
    pub verbosity: u8,
    /// Global dry-run override — when true, no files are modified
    pub dry_run: bool,
}

impl CliContext {
    /// Build a CLI context from the global flags parsed by clap.
    ///
    /// # Arguments
    /// * `config_path` — Optional path to a custom config file (from `--config`)
    /// * `verbose` — Verbosity count (0–3, from `-v` / `-vv` / `-vvv`)
    /// * `json` — Whether JSON output was requested (from `--json`)
    /// * `dry_run` — Whether dry-run mode was requested (from `--dry-run`)
    ///
    /// # Behaviour
    /// - If `config_path` is provided, loads from that file via `AppConfig::load_from()`
    /// - Otherwise, loads from the platform default via `AppConfig::load()`
    /// - If config loading fails, falls back to `AppConfig::default()` with a warning
    pub fn build(
        config_path: Option<&str>,
        verbose: u8,
        json: bool,
        dry_run: bool,
    ) -> anyhow::Result<Self> {
        // Load configuration — from custom path or platform default
        let config = match config_path {
            Some(path) => {
                // User specified a config file path via --config
                mm_core::config::AppConfig::load_from(Path::new(path)).unwrap_or_else(|e| {
                    // Log the error but don't abort — fall back to defaults
                    tracing::warn!(
                        "Failed to load config from '{}': {} — using defaults",
                        path,
                        e
                    );
                    mm_core::config::AppConfig::default()
                })
            }
            None => {
                // Use platform default config path
                mm_core::config::AppConfig::load().unwrap_or_else(|e| {
                    // Platform default may not exist yet — that's fine
                    tracing::warn!("Failed to load default config: {} — using defaults", e);
                    mm_core::config::AppConfig::default()
                })
            }
        };

        // Determine output format from the --json flag
        let output = if json {
            OutputFormat::Json
        } else {
            OutputFormat::Human
        };

        Ok(Self {
            config,
            output,
            verbosity: verbose,
            dry_run,
        })
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Building context with no config file falls back to defaults
    #[test]
    fn build_with_defaults() {
        let ctx = CliContext::build(None, 0, false, false).unwrap();
        assert_eq!(ctx.output, OutputFormat::Human);
        assert_eq!(ctx.verbosity, 0);
        assert!(!ctx.dry_run);
    }

    /// The --json flag sets JSON output mode
    #[test]
    fn build_with_json_flag() {
        let ctx = CliContext::build(None, 0, true, false).unwrap();
        assert_eq!(ctx.output, OutputFormat::Json);
    }

    /// The --dry-run flag is propagated to the context
    #[test]
    fn build_with_dry_run() {
        let ctx = CliContext::build(None, 2, false, true).unwrap();
        assert!(ctx.dry_run);
        assert_eq!(ctx.verbosity, 2);
    }
}
