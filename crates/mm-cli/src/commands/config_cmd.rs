// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — `meedya config` Command
//
// Configuration management: show current settings, print config file path,
// initialise a new config file, export settings to a portable `.mmprofile`
// bundle, or import from one.

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya config` command.
#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// Config subcommand to execute
    #[command(subcommand)]
    pub action: ConfigAction,
}

/// Available config subcommands.
#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Display the current configuration (loaded or defaults)
    Show,

    /// Print the path to the configuration file
    Path,

    /// Create a new default configuration file
    Init {
        /// Directory to create the config file in (defaults to platform config dir)
        path: Option<PathBuf>,
    },

    /// Export current configuration to a portable .mmprofile bundle
    Export {
        /// Output file path for the .mmprofile bundle
        output: PathBuf,
    },

    /// Import configuration from a .mmprofile bundle
    Import {
        /// Path to the .mmprofile bundle to import
        profile: PathBuf,
    },
}

// ─── JSON output structures ─────────────────────────────────────────────────

/// Config path result for JSON output.
#[derive(Serialize)]
struct PathOutput {
    config_path: String,
    exists: bool,
}

/// Config init result for JSON output.
#[derive(Serialize)]
struct InitOutput {
    path: String,
    created: bool,
}

/// Export/import result for JSON output.
#[derive(Serialize)]
struct ProfileOutput {
    path: String,
    action: String,
    success: bool,
}

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya config` command.
pub fn run(ctx: &CliContext, args: &ConfigArgs) -> anyhow::Result<i32> {
    match &args.action {
        ConfigAction::Show => run_show(ctx),
        ConfigAction::Path => run_path(ctx),
        ConfigAction::Init { path } => run_init(ctx, path.as_deref()),
        ConfigAction::Export { output } => run_export(ctx, output),
        ConfigAction::Import { profile } => run_import(ctx, profile),
    }
}

// ─── Subcommand: show ───────────────────────────────────────────────────────

/// Display the current loaded configuration.
fn run_show(ctx: &CliContext) -> anyhow::Result<i32> {
    match ctx.output {
        OutputFormat::Json => {
            // Serialize the config struct as pretty-printed JSON
            output::print_json(&ctx.config);
        }
        OutputFormat::Human => {
            output::print_header("Current Configuration");
            // Serialize to JSON for readable display (JSON5 serialization is limited)
            match serde_json::to_string_pretty(&ctx.config) {
                Ok(json) => println!("{json}"),
                Err(e) => {
                    output::print_error(&format!("Failed to serialize config: {e}"));
                    return Ok(ExitCode::ERROR);
                }
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

// ─── Subcommand: path ───────────────────────────────────────────────────────

/// Print the path to the configuration file.
fn run_path(ctx: &CliContext) -> anyhow::Result<i32> {
    // Get the platform default settings path
    let path = mm_core::config::AppConfig::default_settings_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "(unable to determine)".to_string());

    let exists = std::path::Path::new(&path).exists();

    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&PathOutput {
                config_path: path,
                exists,
            });
        }
        OutputFormat::Human => {
            output::print_key_value("Config path", &path);
            output::print_key_value("Status", if exists { "exists" } else { "not found" });
        }
    }
    Ok(ExitCode::SUCCESS)
}

// ─── Subcommand: init ───────────────────────────────────────────────────────

/// Create a new default configuration file.
fn run_init(ctx: &CliContext, custom_path: Option<&std::path::Path>) -> anyhow::Result<i32> {
    // Determine where to write the config file
    let target_path = if let Some(p) = custom_path {
        p.join("settings.json5")
    } else {
        mm_core::config::AppConfig::default_settings_path()
            .map_err(|e| anyhow::anyhow!("Cannot determine config path: {e}"))?
    };

    // Check if file already exists
    if target_path.exists() {
        output::print_warning(&format!(
            "Config file already exists: {}",
            target_path.display()
        ));
        return Ok(ExitCode::ERROR);
    }

    // Ensure the parent directory exists
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            anyhow::anyhow!("Failed to create directory '{}': {e}", parent.display())
        })?;
    }

    // Serialize default config as pretty JSON
    let default_config = mm_core::config::AppConfig::default();
    let json = serde_json::to_string_pretty(&default_config)?;
    std::fs::write(&target_path, &json)
        .map_err(|e| anyhow::anyhow!("Failed to write config: {e}"))?;

    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&InitOutput {
                path: target_path.display().to_string(),
                created: true,
            });
        }
        OutputFormat::Human => {
            output::print_success(&format!("Config file created: {}", target_path.display()));
        }
    }
    Ok(ExitCode::SUCCESS)
}

// ─── Subcommand: export ─────────────────────────────────────────────────────

/// Export current configuration to a .mmprofile bundle (JSON).
fn run_export(ctx: &CliContext, output_path: &PathBuf) -> anyhow::Result<i32> {
    let profile = serde_json::to_string_pretty(&ctx.config)?;
    std::fs::write(output_path, &profile)
        .map_err(|e| anyhow::anyhow!("Failed to write profile: {e}"))?;

    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&ProfileOutput {
                path: output_path.display().to_string(),
                action: "export".to_string(),
                success: true,
            });
        }
        OutputFormat::Human => {
            output::print_success(&format!(
                "Configuration exported to {}",
                output_path.display()
            ));
        }
    }
    Ok(ExitCode::SUCCESS)
}

// ─── Subcommand: import ─────────────────────────────────────────────────────

/// Import configuration from a .mmprofile bundle.
fn run_import(ctx: &CliContext, profile_path: &PathBuf) -> anyhow::Result<i32> {
    if !profile_path.exists() {
        output::print_error(&format!("Profile not found: {}", profile_path.display()));
        return Ok(ExitCode::ERROR);
    }

    // Read and validate the profile
    let contents = std::fs::read_to_string(profile_path)
        .map_err(|e| anyhow::anyhow!("Failed to read profile: {e}"))?;
    let _config: mm_core::config::AppConfig = serde_json::from_str(&contents)
        .map_err(|e| anyhow::anyhow!("Invalid profile format: {e}"))?;

    // Write to platform config location
    let target_path = mm_core::config::AppConfig::default_settings_path()
        .map_err(|e| anyhow::anyhow!("Cannot determine config path: {e}"))?;
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target_path, &contents)?;

    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&ProfileOutput {
                path: target_path.display().to_string(),
                action: "import".to_string(),
                success: true,
            });
        }
        OutputFormat::Human => {
            output::print_success(&format!(
                "Configuration imported to {}",
                target_path.display()
            ));
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

    #[test]
    fn config_show() {
        let ctx = test_ctx(false);
        let args = ConfigArgs {
            action: ConfigAction::Show,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn config_show_json() {
        let ctx = test_ctx(true);
        let args = ConfigArgs {
            action: ConfigAction::Show,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn config_path() {
        let ctx = test_ctx(false);
        let args = ConfigArgs {
            action: ConfigAction::Path,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn config_init_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(false);
        let args = ConfigArgs {
            action: ConfigAction::Init {
                path: Some(tmp.path().to_path_buf()),
            },
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
        assert!(tmp.path().join("settings.json5").exists());
    }

    #[test]
    fn config_export_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let profile_path = tmp.path().join("test.mmprofile");
        let ctx = test_ctx(false);
        let args = ConfigArgs {
            action: ConfigAction::Export {
                output: profile_path.clone(),
            },
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
        assert!(profile_path.exists());
        // Validate exported file is parseable
        let contents = std::fs::read_to_string(&profile_path).unwrap();
        let _: mm_core::config::AppConfig = serde_json::from_str(&contents).unwrap();
    }
}
