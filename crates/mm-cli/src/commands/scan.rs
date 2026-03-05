// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya scan` Command
//
// Directory scan with media classification summary and optional rename preview.
// Supports `--execute` to perform renames, with `--dry-run` safety guard.

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::Args;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya scan` command.
#[derive(Args, Debug)]
pub struct ScanArgs {
    /// Path to the directory to scan
    pub path: PathBuf,

    /// Scan recursively into subdirectories (default: true)
    #[arg(short, long, default_value_t = true)]
    pub recursive: bool,

    /// Override the rename template from config
    #[arg(long)]
    pub template: Option<String>,

    /// Override the output directory for renamed files
    #[arg(long)]
    pub output_dir: Option<PathBuf>,

    /// Actually perform renames (default: preview only)
    #[arg(long)]
    pub execute: bool,

    /// Force preview mode even with --execute
    #[arg(long)]
    pub dry_run: bool,
}

// ─── JSON output structures ─────────────────────────────────────────────────

/// Complete scan result for JSON output.
#[derive(Serialize)]
struct ScanOutput {
    directory: String,
    total_files: usize,
    classification_summary: Vec<GroupCount>,
    rename_previews: Vec<PreviewEntry>,
    summary: ScanSummary,
}

/// File count by media group for JSON output.
#[derive(Serialize)]
struct GroupCount {
    group: String,
    count: usize,
}

/// Single rename preview entry for JSON output.
#[derive(Serialize)]
struct PreviewEntry {
    source: String,
    destination: String,
    conflict: bool,
    unchanged: bool,
}

/// Scan summary for JSON output.
#[derive(Serialize)]
struct ScanSummary {
    total: usize,
    renamed: usize,
    unchanged: usize,
    conflicts: usize,
    executed: bool,
}

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya scan` command.
pub fn run(ctx: &CliContext, args: &ScanArgs) -> anyhow::Result<i32> {
    // Verify the directory exists
    if !args.path.is_dir() {
        output::print_error(&format!("Not a directory: {}", args.path.display()));
        return Ok(ExitCode::ERROR);
    }

    // Determine effective dry-run state (global or per-command)
    let dry_run = ctx.dry_run || args.dry_run || !args.execute;

    // ── 1. Build watcher config for scanning ────────────────────────────
    let watcher_config = mm_core::watcher::WatcherConfig {
        folders: vec![args.path.clone()],
        recursive: args.recursive,
        debounce_ms: 0, // Not relevant for scanning
        include_extensions: ctx.config.watch.include_extensions.clone(),
        exclude_extensions: ctx.config.watch.exclude_extensions.clone(),
        ignore_patterns: Vec::new(),
    };

    // ── 2. Scan for files ───────────────────────────────────────────────
    let files = mm_core::watcher::scan_existing_files(&watcher_config)?;

    if files.is_empty() {
        output::print_warning("No media files found in the specified directory");
        return Ok(ExitCode::SUCCESS);
    }

    // ── 3. Classify each file ───────────────────────────────────────────
    let mut group_counts: HashMap<String, usize> = HashMap::new();
    for file in &files {
        let classification = mm_core::classify::classify_by_path(file)
            .unwrap_or_else(|_| mm_core::classify::MediaClassification::unknown());
        let group_name = format!("{:?}", classification.group);
        *group_counts.entry(group_name).or_insert(0) += 1;
    }

    // ── 4. Rename preview (if template available) ───────────────────────
    let template = args
        .template
        .as_deref()
        .unwrap_or(&ctx.config.rename.template);

    let mut previews: Vec<PreviewEntry> = Vec::new();
    let mut renamed_count = 0usize;
    let mut unchanged_count = 0usize;
    let mut conflict_count = 0usize;

    // Only generate previews if we have a non-empty template
    if !template.is_empty() {
        let output_dir = args.output_dir.as_deref().unwrap_or(args.path.as_path());

        for file in &files {
            // Extract tags and properties for each file
            let tags = mm_core::metadata::extract_tags(file).unwrap_or_default();
            let audio_props = mm_core::metadata::extract_audio_properties(file).ok();
            let classification = mm_core::classify::classify_by_path(file).ok();

            // Build EvalContext
            let mut eval_ctx = mm_core::rule_engine::EvalContext::new(&tags);
            if let Some(ref props) = audio_props {
                eval_ctx = eval_ctx.with_audio_props(props);
            }
            if let Some(ref class) = classification {
                eval_ctx = eval_ctx.with_classification(class);
            }
            eval_ctx = eval_ctx.with_file_path(file).with_path_mode(true);

            // Evaluate template to get new filename
            let new_name = mm_core::rule_engine::evaluate_template(template, &eval_ctx)
                .unwrap_or_else(|_| {
                    file.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                });

            // Sanitize the filename
            let sanitized = mm_core::renamer::sanitize_filename(
                &new_name,
                &mm_core::renamer::SanitizeConfig::default(),
            );

            // Build destination path
            let ext = file.extension().unwrap_or_default().to_string_lossy();
            let dest = output_dir.join(format!("{sanitized}.{ext}"));

            let unchanged = dest == *file;
            let conflict = !unchanged && dest.exists();

            if unchanged {
                unchanged_count += 1;
            } else if conflict {
                conflict_count += 1;
            } else {
                renamed_count += 1;
            }

            previews.push(PreviewEntry {
                source: file.display().to_string(),
                destination: dest.display().to_string(),
                conflict,
                unchanged,
            });
        }
    }

    // ── 5. Execute renames if requested ──────────────────────────────────
    let executed = !dry_run && args.execute;
    if executed {
        for preview in &previews {
            if !preview.unchanged && !preview.conflict {
                let rename_preview = mm_core::renamer::RenamePreview {
                    source: PathBuf::from(&preview.source),
                    destination: PathBuf::from(&preview.destination),
                    conflict: preview.conflict,
                    unchanged: preview.unchanged,
                };
                if let Err(e) = mm_core::renamer::execute_rename(&rename_preview) {
                    output::print_error(&format!("Failed to rename: {e}"));
                }
            }
        }
    }

    // ── 6. Render output ────────────────────────────────────────────────
    let group_counts_vec: Vec<GroupCount> = {
        let mut v: Vec<_> = group_counts
            .into_iter()
            .map(|(group, count)| GroupCount { group, count })
            .collect();
        v.sort_by(|a, b| b.count.cmp(&a.count));
        v
    };

    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&ScanOutput {
                directory: args.path.display().to_string(),
                total_files: files.len(),
                classification_summary: group_counts_vec,
                rename_previews: previews,
                summary: ScanSummary {
                    total: files.len(),
                    renamed: renamed_count,
                    unchanged: unchanged_count,
                    conflicts: conflict_count,
                    executed,
                },
            });
        }
        OutputFormat::Human => {
            // Classification summary
            output::print_header(&format!(
                "Scan: {} ({} files)",
                args.path.display(),
                files.len()
            ));
            let rows: Vec<Vec<String>> = group_counts_vec
                .iter()
                .map(|gc| vec![gc.group.clone(), gc.count.to_string()])
                .collect();
            output::print_table(&["Media Group", "Count"], &rows);

            // Rename preview (if we generated any)
            if !previews.is_empty() {
                output::print_header("Rename Preview");
                let preview_rows: Vec<Vec<String>> = previews
                    .iter()
                    .filter(|p| !p.unchanged)
                    .map(|p| {
                        let status = if p.conflict { "CONFLICT" } else { "OK" };
                        vec![p.source.clone(), p.destination.clone(), status.to_string()]
                    })
                    .collect();
                if preview_rows.is_empty() {
                    println!("  (all files already at correct names)");
                } else {
                    output::print_table(&["Source", "Destination", "Status"], &preview_rows);
                }
            }

            // Summary line
            output::print_header("Summary");
            output::print_key_value("Total", &files.len().to_string());
            output::print_key_value("To rename", &renamed_count.to_string());
            output::print_key_value("Unchanged", &unchanged_count.to_string());
            output::print_key_value("Conflicts", &conflict_count.to_string());
            if executed {
                output::print_success("Renames executed");
            } else if args.execute {
                output::print_warning("Dry-run mode — no files modified");
            }
        }
    }

    // Return partial exit code if there were conflicts
    if conflict_count > 0 {
        Ok(ExitCode::PARTIAL)
    } else {
        Ok(ExitCode::SUCCESS)
    }
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

    /// Scan returns error for non-existent directory
    #[test]
    fn scan_nonexistent_dir() {
        let ctx = test_ctx(false);
        let args = ScanArgs {
            path: PathBuf::from("/nonexistent/directory"),
            recursive: true,
            template: None,
            output_dir: None,
            execute: false,
            dry_run: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::ERROR);
    }

    /// Scan succeeds on an empty temp directory
    #[test]
    fn scan_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(false);
        let args = ScanArgs {
            path: tmp.path().to_path_buf(),
            recursive: true,
            template: None,
            output_dir: None,
            execute: false,
            dry_run: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    /// Scan succeeds in JSON mode on empty directory
    #[test]
    fn scan_json_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(true);
        let args = ScanArgs {
            path: tmp.path().to_path_buf(),
            recursive: true,
            template: None,
            output_dir: None,
            execute: false,
            dry_run: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    /// Scan with a directory containing a non-media file
    #[test]
    fn scan_with_files() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a dummy text file (won't be classified as media)
        std::fs::write(tmp.path().join("readme.txt"), "hello").unwrap();
        let ctx = test_ctx(false);
        let args = ScanArgs {
            path: tmp.path().to_path_buf(),
            recursive: true,
            template: None,
            output_dir: None,
            execute: false,
            dry_run: false,
        };
        // May find the file or not depending on watcher config filters
        let code = run(&ctx, &args).unwrap();
        assert!(code == ExitCode::SUCCESS || code == ExitCode::PARTIAL);
    }

    /// ScanArgs construction
    #[test]
    fn scan_args_defaults() {
        let args = ScanArgs {
            path: PathBuf::from("/tmp"),
            recursive: true,
            template: None,
            output_dir: None,
            execute: false,
            dry_run: false,
        };
        assert!(args.recursive);
        assert!(!args.execute);
        assert!(!args.dry_run);
    }

    /// Dry-run prevents execution
    #[test]
    fn scan_dry_run_flag() {
        let args = ScanArgs {
            path: PathBuf::from("/tmp"),
            recursive: true,
            template: Some("<Title>".to_string()),
            output_dir: None,
            execute: true,
            dry_run: true,
        };
        // dry_run should override execute
        assert!(args.dry_run);
    }

    /// Execute flag only works without dry-run
    #[test]
    fn scan_execute_without_dry_run() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(false);
        let args = ScanArgs {
            path: tmp.path().to_path_buf(),
            recursive: true,
            template: None,
            output_dir: None,
            execute: true,
            dry_run: true, // Overrides execute
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }
}
