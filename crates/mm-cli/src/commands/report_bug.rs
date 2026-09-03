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
use std::path::{Path, PathBuf};

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
        // Issue #50: this used to scan the platform log directory for
        // `meedya-*.log` and, on finding nothing, tell the user to "enable
        // file output in settings.json5" — advice that did nothing, because
        // nothing ever called `init_logging`. The lookup is now driven by the
        // same `[logging]` config the CLI actually installs at startup, so
        // the path we read and the advice we print are both true.
        match locate_log_file(&ctx.config.logging, &config_path) {
            Ok(log_file) => match std::fs::read_to_string(&log_file) {
                Ok(contents) => {
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
                }
                Err(e) => {
                    output::print_warning(&format!(
                        "Could not read log file {}: {e}",
                        log_file.display()
                    ));
                    None
                }
            },
            Err(explanation) => {
                output::print_warning(&explanation);
                None
            }
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

// ─── Log file discovery ────────────────────────────────────────────────────

/// Find the log file to attach to a bug report.
///
/// Returns the path on success, or a user-facing explanation of why there is
/// nothing to attach. The explanation is the whole point of this function:
/// before issue #50 the command pointed users at a settings toggle that had
/// no effect anywhere in the codebase, so a "no logs found" message was
/// actively misleading. Each branch below now describes the real state.
fn locate_log_file(
    logging: &mm_core::config::LoggingConfig,
    config_path: &str,
) -> Result<PathBuf, String> {
    let log_config = mm_core::logging::LogConfig::from_settings(logging);

    // Case 1 — file logging is simply off. `logging.file` being unset IS the
    // off switch, so name it and show what turning it on looks like.
    let Some(resolved) = log_config.resolved_log_path() else {
        let suggestion = mm_core::logging::default_log_dir().join("meedya.log");
        return Err(format!(
            "File logging is not enabled, so there are no logs to include. \
             Add `logging: {{ file: \"{}\" }}` to {config_path} and re-run the \
             command you want to capture, then run `meedya report-bug \
             --include-logs` again.",
            suggestion.display()
        ));
    };

    // Case 2 — enabled and the configured file is there. The common path.
    if resolved.is_file() {
        return Ok(resolved);
    }

    // Case 3 — enabled with the dated-filename scheme (no explicit path), and
    // today's file does not exist yet. An earlier day's log is still useful,
    // so fall back to the most recently modified `meedya-*.log` in the
    // directory before giving up.
    if log_config.file_path.is_none()
        && let Some(previous) = newest_dated_log(&log_config.log_dir)
    {
        return Ok(previous);
    }

    // Case 4 — enabled, but nothing has been written there yet.
    Err(format!(
        "File logging is enabled ({}) but that file does not exist yet. It is \
         created the next time a `meedya` command runs — reproduce the problem \
         first, then re-run `meedya report-bug --include-logs`.",
        resolved.display()
    ))
}

/// The most recently modified `meedya-<date>.log` in `dir`, if any.
///
/// Only used for the dated-filename scheme; an explicitly configured path is
/// never guessed at.
fn newest_dated_log(dir: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<(PathBuf, Option<std::time::SystemTime>)> = Vec::new();

    // A missing log directory is normal (nothing has run yet), not an error.
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with("meedya-") && name.ends_with(".log") {
            let modified = entry.metadata().ok().and_then(|m| m.modified().ok());
            candidates.push((path, modified));
        }
    }

    // Most recent first — a stale file from last week is worse than nothing
    // only if it hides a newer one, so ordering matters here.
    use std::cmp::Reverse;
    candidates.sort_by_key(|(_, modified)| Reverse(*modified));
    candidates.into_iter().next().map(|(path, _)| path)
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

    // ── Log discovery (issue #50) ───────────────────────────────────────

    /// With no `logging.file` configured, the warning must say file logging
    /// is off and show how to turn it on — not send the user to a toggle
    /// that does nothing.
    #[test]
    fn locate_log_file_reports_disabled_logging() {
        let logging = mm_core::config::LoggingConfig::default();
        assert!(logging.file.is_none(), "precondition: default has no file");

        let err = locate_log_file(&logging, "/etc/meedya/settings.json5").unwrap_err();
        assert!(err.contains("not enabled"), "got: {err}");
        assert!(err.contains("logging:"), "no example given: {err}");
        assert!(
            err.contains("/etc/meedya/settings.json5"),
            "config path not named: {err}"
        );
    }

    /// Configured but not yet written: say so, rather than claiming logging
    /// is disabled.
    #[test]
    fn locate_log_file_reports_configured_but_missing_file() {
        let tmp = tempfile::tempdir().unwrap();
        let log_path = tmp.path().join("meedya.log");
        let logging = mm_core::config::LoggingConfig {
            file: Some(log_path.clone()),
            ..mm_core::config::LoggingConfig::default()
        };

        let err = locate_log_file(&logging, "settings.json5").unwrap_err();
        assert!(err.contains("enabled"), "got: {err}");
        assert!(err.contains(&log_path.display().to_string()), "got: {err}");
    }

    /// The happy path: the configured file exists, so it is the one used.
    #[test]
    fn locate_log_file_finds_the_configured_file() {
        let tmp = tempfile::tempdir().unwrap();
        let log_path = tmp.path().join("meedya.log");
        std::fs::write(&log_path, "hello\n").unwrap();
        let logging = mm_core::config::LoggingConfig {
            file: Some(log_path.clone()),
            ..mm_core::config::LoggingConfig::default()
        };

        assert_eq!(
            locate_log_file(&logging, "settings.json5").unwrap(),
            log_path
        );
    }

    /// `newest_dated_log` picks the most recently modified `meedya-*.log`
    /// and ignores unrelated files.
    #[test]
    fn newest_dated_log_picks_the_most_recent() {
        let tmp = tempfile::tempdir().unwrap();
        let older = tmp.path().join("meedya-2026-01-01.log");
        let newer = tmp.path().join("meedya-2026-01-02.log");
        std::fs::write(&older, "old").unwrap();
        std::fs::write(tmp.path().join("notes.txt"), "ignore me").unwrap();
        std::fs::write(&newer, "new").unwrap();
        // Back-date the older file explicitly. Relying on the two writes
        // above landing on different mtimes would be a coin flip on any
        // filesystem with coarse timestamp resolution.
        let past = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        let times = std::fs::FileTimes::new()
            .set_accessed(past)
            .set_modified(past);
        std::fs::File::options()
            .write(true)
            .open(&older)
            .unwrap()
            .set_times(times)
            .unwrap();

        assert_eq!(newest_dated_log(tmp.path()), Some(newer));
    }

    /// A missing log directory is a normal state, not an error.
    #[test]
    fn newest_dated_log_handles_missing_directory() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(newest_dated_log(&tmp.path().join("nope")), None);
    }

    /// End-to-end: `report-bug --include-logs` with a real log file must put
    /// that file's contents into the report.
    #[test]
    fn report_bug_include_logs_picks_up_an_existing_log() {
        let tmp = tempfile::tempdir().unwrap();
        let log_path = tmp.path().join("meedya.log");
        std::fs::write(
            &log_path,
            "{\"level\":\"INFO\",\"msg\":\"marker-abc123\"}\n",
        )
        .unwrap();

        let mut config = mm_core::config::AppConfig::default();
        config.logging.file = Some(log_path);

        let ctx = CliContext {
            config,
            output: OutputFormat::Human,
            verbosity: 0,
            dry_run: false,
        };
        let report_path = tmp.path().join("report.md");
        let args = ReportBugArgs {
            output: Some(report_path.clone()),
            include_logs: true,
        };

        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
        let report = std::fs::read_to_string(&report_path).unwrap();
        assert!(
            report.contains("## Recent Logs"),
            "no log section: {report}"
        );
        assert!(
            report.contains("marker-abc123"),
            "log line missing: {report}"
        );
    }

    /// ...and with logging disabled the report simply has no log section —
    /// the command still succeeds.
    #[test]
    fn report_bug_include_logs_without_logging_configured() {
        let tmp = tempfile::tempdir().unwrap();
        let report_path = tmp.path().join("report.md");
        let ctx = test_ctx(false);
        assert!(ctx.config.logging.file.is_none());

        let args = ReportBugArgs {
            output: Some(report_path.clone()),
            include_logs: true,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
        let report = std::fs::read_to_string(&report_path).unwrap();
        assert!(report.contains("MeedyaManager Bug Report"));
    }
}
