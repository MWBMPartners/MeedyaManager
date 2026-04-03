// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya report-bug` Command
//
// Generates a bug report with system information, health check results,
// and optionally recent log file contents. Output can be saved to a file
// or printed to stdout.

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::Args;
use serde::Serialize;
use std::path::PathBuf;

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya report-bug` command.
#[derive(Args, Debug)]
pub struct ReportBugArgs {
    /// Write the report to a file instead of stdout
    #[arg(long, value_name = "PATH")]
    pub output: Option<PathBuf>,

    /// Include the last 200 lines of the log file
    #[arg(long)]
    pub include_logs: bool,
}

// ─── JSON output structures ─────────────────────────────────────────────────

/// Complete bug report for JSON output.
#[derive(Serialize)]
struct BugReport {
    system: SystemInfo,
    health: Vec<HealthCheckEntry>,
    config_path: String,
    watch_folders: Vec<String>,
    log_tail: Option<Vec<String>>,
}

/// System information for the report.
#[derive(Serialize)]
struct SystemInfo {
    os: String,
    arch: String,
    meedya_version: String,
}

/// Health check entry for the report.
#[derive(Serialize)]
struct HealthCheckEntry {
    name: String,
    status: String,
    message: String,
}

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya report-bug` command.
pub fn run(ctx: &CliContext, args: &ReportBugArgs) -> anyhow::Result<i32> {
    // ── 1. Collect system information ───────────────────────────────────
    let system = SystemInfo {
        os: format!("{} {}", std::env::consts::OS, std::env::consts::FAMILY),
        arch: std::env::consts::ARCH.to_string(),
        meedya_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    // ── 2. Get config file path ─────────────────────────────────────────
    let config_path = mm_core::config::AppConfig::default_settings_path().map_or_else(
        |_| "(unable to determine)".to_string(),
        |p| p.display().to_string(),
    );

    // ── 3. Get watch folders from config ────────────────────────────────
    let watch_folders: Vec<String> = ctx
        .config
        .watch
        .folders
        .iter()
        .map(|f| f.display().to_string())
        .collect();

    // ── 4. Run health checks ────────────────────────────────────────────
    let config_path_buf = mm_core::config::AppConfig::default_settings_path()
        .unwrap_or_else(|_| PathBuf::from("settings.json5"));
    let health_report =
        mm_core::health::run_health_checks(&config_path_buf, &ctx.config.watch.folders);

    let health_entries: Vec<HealthCheckEntry> = health_report
        .checks
        .iter()
        .map(|check| HealthCheckEntry {
            name: check.name.clone(),
            status: format!("{:?}", check.status),
            message: check.message.clone(),
        })
        .collect();

    // ── 5. Read log tail (if requested) ─────────────────────────────────
    let log_tail = if args.include_logs {
        // Try to find the log file from config
        let log_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("MeedyaManager")
            .join("logs");
        let log_file = log_dir.join("meedya.log");

        if log_file.exists() {
            if let Ok(contents) = std::fs::read_to_string(&log_file) {
                // Take last 200 lines
                let lines: Vec<String> = contents
                    .lines()
                    .rev()
                    .take(200)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(std::string::ToString::to_string)
                    .collect();
                Some(lines)
            } else {
                output::print_warning(&format!("Could not read log file: {}", log_file.display()));
                None
            }
        } else {
            output::print_warning("No log file found");
            None
        }
    } else {
        None
    };

    // ── 6. Build complete report ────────────────────────────────────────
    let report = BugReport {
        system,
        health: health_entries,
        config_path,
        watch_folders,
        log_tail,
    };

    // ── 7. Render output ────────────────────────────────────────────────
    let output_text = match ctx.output {
        OutputFormat::Json => {
            // JSON output
            serde_json::to_string_pretty(&report)?
        }
        OutputFormat::Human => {
            // Markdown-formatted report
            let mut lines = Vec::new();
            lines.push("# MeedyaManager Bug Report".to_string());
            lines.push(String::new());
            lines.push("## System Information".to_string());
            lines.push(format!("- **OS:** {}", report.system.os));
            lines.push(format!("- **Architecture:** {}", report.system.arch));
            lines.push(format!(
                "- **MeedyaManager Version:** {}",
                report.system.meedya_version
            ));
            lines.push(format!("- **Config Path:** {}", report.config_path));
            lines.push(String::new());

            lines.push("## Watch Folders".to_string());
            if report.watch_folders.is_empty() {
                lines.push("- (none configured)".to_string());
            } else {
                for folder in &report.watch_folders {
                    lines.push(format!("- `{folder}`"));
                }
            }
            lines.push(String::new());

            lines.push("## Health Checks".to_string());
            for check in &report.health {
                let icon = match check.status.as_str() {
                    "Pass" => "✅",
                    "Warn" => "⚠️",
                    "Fail" => "❌",
                    _ => "❓",
                };
                lines.push(format!("- {icon} **{}**: {}", check.name, check.message));
            }
            lines.push(String::new());

            if let Some(ref tail) = report.log_tail {
                lines.push("## Recent Logs".to_string());
                lines.push("```".to_string());
                for line in tail {
                    lines.push(line.clone());
                }
                lines.push("```".to_string());
            }

            lines.join("\n")
        }
    };

    // ── 8. Write or print ───────────────────────────────────────────────
    if let Some(ref output_path) = args.output {
        std::fs::write(output_path, &output_text)
            .map_err(|e| anyhow::anyhow!("Failed to write report: {e}"))?;
        output::print_success(&format!("Bug report saved to {}", output_path.display()));
    } else {
        println!("{output_text}");
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

    /// Report generation succeeds in human mode
    #[test]
    fn report_bug_human() {
        let ctx = test_ctx(false);
        let args = ReportBugArgs {
            output: None,
            include_logs: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    /// Report generation succeeds in JSON mode
    #[test]
    fn report_bug_json() {
        let ctx = test_ctx(true);
        let args = ReportBugArgs {
            output: None,
            include_logs: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    /// Report can be written to a file
    #[test]
    fn report_bug_to_file() {
        let tmp = tempfile::tempdir().unwrap();
        let output_path = tmp.path().join("report.md");
        let ctx = test_ctx(false);
        let args = ReportBugArgs {
            output: Some(output_path.clone()),
            include_logs: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
        assert!(output_path.exists());
        let contents = std::fs::read_to_string(&output_path).unwrap();
        assert!(contents.contains("MeedyaManager Bug Report"));
    }
}
