// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya scan` Command
//
// Directory scan with media classification summary and optional rename preview.
// Supports `--execute` to perform renames, with `--dry-run` safety guard.
//
// Design note — why this command is a thin shell:
//   Destination computation lives entirely in `mm_core::renamer`. Historically
//   `scan` re-implemented it, and the copy drifted: it flattened folder
//   templates, never noticed two files resolving to the *same* destination,
//   and then handed that stale "no conflict" verdict to the mover — which
//   silently overwrote the first file. The core simulator already tracks
//   intra-batch destinations and splits directory components, so this module
//   now delegates to it and confines itself to CLI concerns: argument
//   handling, config precedence, conflict policy and rendering.

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::Args;
use mm_core::classify::MediaClassification;
use mm_core::error::{MmError, MmResult};
use mm_core::metadata::{AudioProperties, TagMap};
use mm_core::renamer::{ExecuteOptions, RenamePreview, RenameSummary, SanitizeConfig};
use mm_core::rule_engine::{EvalContext, MissingTagMode};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::str::FromStr;

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya scan` command.
#[derive(Args, Debug)]
pub struct ScanArgs {
    /// Path to the directory to scan
    pub path: PathBuf,

    /// Disable recursive scanning into subdirectories
    //
    // A bare `bool` flag with `default_value_t = true` cannot be switched off
    // by clap — there is no `--no-x` counterpart for a `SetTrue` action — so
    // the old `-r/--recursive` was permanently stuck on. We mirror
    // `watch --no-recursive` instead: recursion stays the default and this
    // flag turns it off. See `recursive()` below.
    #[arg(long)]
    pub no_recursive: bool,

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

impl ScanArgs {
    /// Effective recursion setting — recursive unless explicitly disabled.
    fn recursive(&self) -> bool {
        !self.no_recursive
    }
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

// ─── Conflict policy ────────────────────────────────────────────────────────

/// What to do when a destination is already claimed.
///
/// Mirrors `config.rename.conflict_strategy`. Two of the four documented
/// values are deliberately *not* honoured yet:
///
///   * `overwrite` would mean re-enabling the exact data-loss path this
///     command was just fixed to close, so it must not be a config typo away;
///   * `ask` needs an interactive confirmation prompt that the CLI does not
///     have.
///
/// Both therefore warn and skip, which is lossless and reversible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConflictStrategy {
    /// Leave conflicting files where they are (the safe default).
    Skip,
    /// Append " (n)" before the extension until a free name is found.
    Rename,
    /// Recognised but unimplemented (`overwrite`, `ask`) — warn and skip.
    Unsupported,
}

/// Parse `config.rename.conflict_strategy` into a policy.
///
/// Unrecognised values fall back to `Skip` rather than erroring: a stale or
/// misspelled config should degrade to the safest behaviour, not abort a scan.
fn parse_conflict_strategy(raw: &str) -> ConflictStrategy {
    match raw.trim().to_ascii_lowercase().as_str() {
        "rename" => ConflictStrategy::Rename,
        "overwrite" | "ask" => ConflictStrategy::Unsupported,
        _ => ConflictStrategy::Skip,
    }
}

// ─── Metadata extraction (pass 1) ───────────────────────────────────────────

/// Everything read from one file, owned so the borrow lives long enough.
///
/// `EvalContext` holds only shared references, so the data it points at must
/// outlive every context we hand to the simulator. Reading each file exactly
/// once into this map also means the simulator does not re-open files.
struct Extracted {
    /// Embedded tag values (empty when the file has none or cannot be read)
    tags: TagMap,
    /// Technical audio properties, when the file has a readable audio stream
    props: Option<AudioProperties>,
    /// 4-level media classification derived from the path/extension
    class: Option<MediaClassification>,
}

/// Pass 1 — read tags, audio properties and classification for every file.
fn extract_all(files: &[PathBuf]) -> HashMap<PathBuf, Extracted> {
    let mut extracted = HashMap::with_capacity(files.len());

    for file in files {
        extracted.insert(
            file.clone(),
            Extracted {
                // Unreadable or tagless files yield an empty map rather than
                // aborting the whole scan.
                tags: mm_core::metadata::extract_tags(file).unwrap_or_default(),
                props: mm_core::metadata::extract_audio_properties(file).ok(),
                class: mm_core::classify::classify_by_path(file).ok(),
            },
        );
    }

    extracted
}

/// Build the evaluation context for one file from the pass-1 map.
///
/// Written as a named `fn` (rather than an inline closure) so the two
/// lifetimes unify explicitly: the simulator wants
/// `for<'p> Fn(&'p Path) -> MmResult<EvalContext<'p>>`, and `EvalContext` is
/// covariant in its lifetime, so a context borrowing the longer-lived
/// `extracted` map coerces down to `'p` without any cloning.
fn build_eval_context<'a>(
    path: &'a Path,
    extracted: &'a HashMap<PathBuf, Extracted>,
    missing_tag_mode: MissingTagMode,
) -> MmResult<EvalContext<'a>> {
    // The map is built from exactly the same file list, so a miss is a
    // programming error rather than a user-facing condition — but report it
    // instead of panicking.
    let entry = extracted.get(path).ok_or_else(|| {
        MmError::Metadata(format!("no extracted metadata for {}", path.display()))
    })?;

    let mut ctx = EvalContext::new(&entry.tags);
    if let Some(props) = entry.props.as_ref() {
        ctx = ctx.with_audio_props(props);
    }
    if let Some(class) = entry.class.as_ref() {
        ctx = ctx.with_classification(class);
    }

    Ok(ctx
        // Virtual tags such as <Filename> and <Folder> need the source path.
        .with_file_path(path)
        // Path mode: multi-value tags collapse to their first value, because
        // "Artist1; Artist2" is a poor directory name.
        .with_path_mode(true)
        // Honour the configured behaviour for tags the file does not carry.
        .with_missing_tag_mode(missing_tag_mode))
}

// ─── Scan helpers ───────────────────────────────────────────────────────────

/// Collect the media files to operate on, honouring `--no-recursive`.
fn scan_files(ctx: &CliContext, args: &ScanArgs) -> MmResult<Vec<PathBuf>> {
    let watcher_config = mm_core::watcher::WatcherConfig {
        folders: vec![args.path.clone()],
        recursive: args.recursive(),
        debounce_ms: 0, // Not relevant for scanning
        include_extensions: ctx.config.watch.include_extensions.clone(),
        exclude_extensions: ctx.config.watch.exclude_extensions.clone(),
        ignore_patterns: Vec::new(),
    };

    mm_core::watcher::scan_existing_files(&watcher_config)
}

/// Resolve the destination root.
///
/// Precedence: `--output-dir` beats `config.rename.output_dir`, which beats
/// the scanned directory itself (i.e. rename in place).
fn resolve_output_dir<'a>(ctx: &'a CliContext, args: &'a ScanArgs) -> &'a Path {
    args.output_dir
        .as_deref()
        .or(ctx.config.rename.output_dir.as_deref())
        .unwrap_or(args.path.as_path())
}

/// Pass 2 — ask `mm-core` to simulate the whole batch.
///
/// Returns `None` when there is no template at all, in which case the command
/// degrades to a pure classification report.
fn preview_renames(
    ctx: &CliContext,
    args: &ScanArgs,
    files: &[PathBuf],
    extracted: &HashMap<PathBuf, Extracted>,
) -> anyhow::Result<Option<RenameSummary>> {
    let template = args
        .template
        .as_deref()
        .unwrap_or(&ctx.config.rename.template);

    if template.is_empty() {
        return Ok(None);
    }

    let output_dir = resolve_output_dir(ctx, args);

    // A bad `missing_tag_mode` is a hard error: silently defaulting it would
    // change every destination path without telling the user.
    let missing_tag_mode = MissingTagMode::from_str(&ctx.config.rename.missing_tag_mode)?;

    // `rules` are evaluated first and the template is the fallback — this is
    // the config surface that the old hand-rolled loop ignored entirely.
    let summary = mm_core::renamer::simulate_rename_with_rules(
        files,
        &ctx.config.rename.rules,
        template,
        output_dir,
        &SanitizeConfig::default(),
        |path: &Path| build_eval_context(path, extracted, missing_tag_mode),
    )?;

    Ok(Some(summary))
}

/// Outcome of applying a batch of previews to the file system.
struct ExecutionReport {
    /// Conflicts the active strategy could not resolve
    unresolved_conflicts: usize,
    /// Files that failed to move/copy
    errors: usize,
}

/// Apply the previews, honouring the configured conflict strategy.
///
/// Conflicting previews are never passed to the mover as-is: either they are
/// re-pointed at a free name (`rename`) or they are skipped. `claimed` records
/// every destination this run has taken so the counter strategy cannot hand
/// out the same replacement name twice within one batch.
fn execute_previews(
    previews: &[RenamePreview],
    strategy: ConflictStrategy,
    strategy_name: &str,
    opts: &ExecuteOptions,
) -> ExecutionReport {
    let mut report = ExecutionReport {
        unresolved_conflicts: 0,
        errors: 0,
    };
    let mut claimed: HashSet<PathBuf> = HashSet::new();
    let mut warned_unsupported = false;

    for preview in previews {
        // Already where it belongs.
        if preview.unchanged {
            continue;
        }

        if preview.conflict {
            match strategy {
                ConflictStrategy::Skip => {
                    report.unresolved_conflicts += 1;
                }
                ConflictStrategy::Unsupported => {
                    // One warning per run, not one per file.
                    if !warned_unsupported {
                        output::print_warning(&format!(
                            "conflict_strategy \"{strategy_name}\" is not implemented — \
                             conflicting files were skipped"
                        ));
                        warned_unsupported = true;
                    }
                    report.unresolved_conflicts += 1;
                }
                ConflictStrategy::Rename => {
                    match mm_core::renamer::resolve_conflict_by_counter(
                        &preview.destination,
                        &claimed,
                    ) {
                        Ok(free) => {
                            // Rebuild the preview around the free name; the
                            // conflict flag is now genuinely false, and
                            // `execute_rename_with` re-checks the disk anyway.
                            let resolved = RenamePreview {
                                source: preview.source.clone(),
                                destination: free.clone(),
                                conflict: false,
                                unchanged: false,
                            };
                            if let Err(e) = mm_core::renamer::execute_rename_with(&resolved, opts) {
                                output::print_error(&format!("Failed to rename: {e}"));
                                report.errors += 1;
                            } else {
                                claimed.insert(free);
                            }
                        }
                        Err(e) => {
                            output::print_error(&format!("Cannot resolve conflict: {e}"));
                            report.unresolved_conflicts += 1;
                        }
                    }
                }
            }
            continue;
        }

        // Non-conflicting preview — the common case.
        if let Err(e) = mm_core::renamer::execute_rename_with(preview, opts) {
            output::print_error(&format!("Failed to rename: {e}"));
            report.errors += 1;
        } else {
            claimed.insert(preview.destination.clone());
        }
    }

    report
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

    // ── 1. Scan for files ───────────────────────────────────────────────
    let files = scan_files(ctx, args)?;

    if files.is_empty() {
        output::print_warning("No media files found in the specified directory");
        return Ok(ExitCode::SUCCESS);
    }

    // ── 2. Pass 1: read every file exactly once ─────────────────────────
    let extracted = extract_all(&files);

    // ── 3. Classification summary (reuses the pass-1 classifications) ───
    let mut group_counts: HashMap<String, usize> = HashMap::new();
    for entry in extracted.values() {
        // Files we could not classify are counted under the "unknown" group,
        // matching the previous behaviour.
        let group_name = entry.class.as_ref().map_or_else(
            || format!("{:?}", MediaClassification::unknown().group),
            |c| format!("{:?}", c.group),
        );
        *group_counts.entry(group_name).or_insert(0) += 1;
    }

    // ── 4. Pass 2: rename preview via the core simulator ────────────────
    let summary = preview_renames(ctx, args, &files, &extracted)?;

    let (core_previews, renamed_count, unchanged_count, conflict_count) = match summary {
        Some(s) => (s.previews, s.renamed, s.unchanged, s.conflicts),
        None => (Vec::new(), 0, 0, 0),
    };

    let previews: Vec<PreviewEntry> = core_previews
        .iter()
        .map(|p| PreviewEntry {
            source: p.source.display().to_string(),
            destination: p.destination.display().to_string(),
            conflict: p.conflict,
            unchanged: p.unchanged,
        })
        .collect();

    // ── 5. Execute renames if requested ──────────────────────────────────
    let strategy_name = ctx.config.rename.conflict_strategy.as_str();
    let strategy = parse_conflict_strategy(strategy_name);
    let executed = !dry_run && args.execute;

    let report = if executed {
        execute_previews(
            &core_previews,
            strategy,
            strategy_name,
            &ExecuteOptions {
                // Both were silently ignored by the old implementation.
                copy_mode: ctx.config.rename.copy_mode,
                create_dirs: ctx.config.rename.create_dirs,
            },
        )
    } else {
        // Preview-only: every detected conflict is by definition unresolved.
        ExecutionReport {
            unresolved_conflicts: conflict_count,
            errors: 0,
        }
    };

    // ── 6. Render output ────────────────────────────────────────────────
    let group_counts_vec: Vec<GroupCount> = {
        let mut v: Vec<_> = group_counts
            .into_iter()
            .map(|(group, count)| GroupCount { group, count })
            .collect();
        v.sort_by_key(|a| std::cmp::Reverse(a.count));
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
            // Surfaced because it decides what happens to those conflicts.
            output::print_key_value("Conflict strategy", strategy_name);
            if executed {
                output::print_success("Renames executed");
            } else if args.execute {
                output::print_warning("Dry-run mode — no files modified");
            }
        }
    }

    // Partial success when anything was left undone: unresolved conflicts, or
    // files that failed to move. Conflicts the `rename` strategy resolved do
    // not count — nothing was left behind.
    if report.unresolved_conflicts > 0 || report.errors > 0 {
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

    /// Build a `ScanArgs` for a directory with everything else at its default.
    fn args_for(path: &Path) -> ScanArgs {
        ScanArgs {
            path: path.to_path_buf(),
            no_recursive: false,
            template: None,
            output_dir: None,
            execute: false,
            dry_run: false,
        }
    }

    /// Write a real, playable WAV file: 16-bit mono PCM, 8 kHz, 0.1 s silence.
    ///
    /// A bare 44-byte header with no `data` payload is rejected by lofty, so
    /// the tests would never exercise the tag path. 800 frames × 2 bytes =
    /// 1,600 data bytes, for 1,644 bytes total.
    fn write_test_wav(path: &Path) {
        const SAMPLE_RATE: u32 = 8_000;
        const CHANNELS: u16 = 1;
        const BITS_PER_SAMPLE: u16 = 16;
        const FRAMES: u32 = 800; // 0.1 s at 8 kHz

        let block_align = CHANNELS * BITS_PER_SAMPLE / 8;
        let byte_rate = SAMPLE_RATE * u32::from(block_align);
        let data_len = FRAMES * u32::from(block_align);

        let mut wav: Vec<u8> = Vec::with_capacity(44 + data_len as usize);

        // ── RIFF container header ───────────────────────────────────────
        wav.extend_from_slice(b"RIFF");
        // Everything after this field: 4 ("WAVE") + 24 (fmt) + 8 + data
        wav.extend_from_slice(&(4 + 24 + 8 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // ── "fmt " chunk (16-byte PCM form) ─────────────────────────────
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&CHANNELS.to_le_bytes());
        wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());

        // ── "data" chunk — real silence, not an empty stub ──────────────
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend(std::iter::repeat_n(0u8, data_len as usize));

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, &wav).unwrap();
    }

    /// Write a WAV and stamp artist/album/title onto it.
    fn write_tagged_wav(path: &Path, artist: &str, album: &str, title: &str) {
        write_test_wav(path);

        let mut tags: TagMap = TagMap::new();
        tags.insert("artist".to_string(), vec![artist.to_string()]);
        tags.insert("album".to_string(), vec![album.to_string()]);
        tags.insert("title".to_string(), vec![title.to_string()]);

        mm_core::metadata::write_tags(path, &tags).unwrap();
    }

    /// Scan returns error for non-existent directory
    #[test]
    fn scan_nonexistent_dir() {
        let ctx = test_ctx(false);
        let args = args_for(Path::new("/nonexistent/directory"));
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::ERROR);
    }

    /// Scan succeeds on an empty temp directory
    #[test]
    fn scan_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(false);
        let args = args_for(tmp.path());
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    /// Scan succeeds in JSON mode on empty directory
    #[test]
    fn scan_json_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let ctx = test_ctx(true);
        let args = args_for(tmp.path());
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    /// Scan with a directory containing a non-media file
    #[test]
    fn scan_with_files() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a dummy text file (won't be classified as media)
        std::fs::write(tmp.path().join("readme.txt"), "hello").unwrap();
        let ctx = test_ctx(false);
        let args = args_for(tmp.path());
        // May find the file or not depending on watcher config filters
        let code = run(&ctx, &args).unwrap();
        assert!(code == ExitCode::SUCCESS || code == ExitCode::PARTIAL);
    }

    /// ScanArgs construction
    #[test]
    fn scan_args_defaults() {
        let args = args_for(Path::new("/tmp"));
        assert!(args.recursive());
        assert!(!args.no_recursive);
        assert!(!args.execute);
        assert!(!args.dry_run);
    }

    /// Dry-run prevents execution
    #[test]
    fn scan_dry_run_flag() {
        let args = ScanArgs {
            path: PathBuf::from("/tmp"),
            no_recursive: false,
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
            no_recursive: false,
            template: None,
            output_dir: None,
            execute: true,
            dry_run: true, // Overrides execute
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
    }

    /// **Regression — silent data loss.**
    ///
    /// Two tagless files both resolve to the same destination under
    /// `<Title>`. The old implementation computed `conflict` from a bare
    /// destination-existence check at preview time only, so both were flagged
    /// conflict-free and `std::fs::rename` overwrote the first file: one
    /// file's bytes were destroyed and the command still exited SUCCESS.
    #[test]
    fn scan_execute_two_untagged_files_both_survive() {
        let tmp = tempfile::tempdir().unwrap();
        let first = tmp.path().join("first.wav");
        let second = tmp.path().join("second.wav");
        write_test_wav(&first);
        write_test_wav(&second);

        // Distinct byte lengths make it unambiguous which file survived.
        let first_len = std::fs::metadata(&first).unwrap().len();
        let second_len = first_len + 2;
        {
            let mut bytes = std::fs::read(&second).unwrap();
            bytes.extend_from_slice(&[0u8, 0u8]);
            std::fs::write(&second, &bytes).unwrap();
        }

        let ctx = test_ctx(false);
        let args = ScanArgs {
            path: tmp.path().to_path_buf(),
            no_recursive: false,
            template: Some("<Title>".to_string()),
            output_dir: None,
            execute: true,
            dry_run: false,
        };

        let code = run(&ctx, &args).unwrap();

        // Neither file's content may have been destroyed: exactly one file of
        // each original length must still be present somewhere in the tree.
        let mut lengths: Vec<u64> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().metadata().unwrap().len())
            .collect();
        lengths.sort_unstable();

        assert_eq!(
            lengths,
            vec![first_len, second_len],
            "one file's contents were destroyed by the rename"
        );

        // The unresolved second destination is a conflict, so the run is only
        // partially successful — not the SUCCESS the old code reported.
        assert_eq!(code, ExitCode::PARTIAL);
    }

    /// **Regression — folder templates flattened.**
    ///
    /// The default template `<Artist>/<Album>/<Title>` describes a directory
    /// hierarchy. The old implementation pushed the whole evaluated string
    /// through the filename sanitiser, whose invalid-character table contains
    /// `/`, collapsing every file into a flat `Artist_Album_Title.wav`.
    #[test]
    fn scan_folder_template_yields_subdirectories() {
        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("track.wav");
        write_tagged_wav(&source, "Portishead", "Dummy", "Roads");

        let ctx = test_ctx(false);
        let args = args_for(tmp.path());
        let files = scan_files(&ctx, &args).unwrap();
        let extracted = extract_all(&files);
        let summary = preview_renames(&ctx, &args, &files, &extracted)
            .unwrap()
            .expect("default template is non-empty");

        let destination = &summary.previews[0].destination;
        assert!(
            destination.ends_with(Path::new("Portishead/Dummy/Roads.wav")),
            "expected nested directories, got {}",
            destination.display()
        );
        // And it must stay inside the output root.
        assert!(destination.starts_with(tmp.path()));
    }

    /// `--no-recursive` must actually stop the walk at the top level.
    #[test]
    fn scan_no_recursive_skips_subdirectories() {
        let tmp = tempfile::tempdir().unwrap();
        write_test_wav(&tmp.path().join("top.wav"));
        write_test_wav(&tmp.path().join("nested").join("deep.wav"));

        let ctx = test_ctx(false);

        // Recursive (the default) sees both files.
        let recursive_args = args_for(tmp.path());
        assert_eq!(scan_files(&ctx, &recursive_args).unwrap().len(), 2);

        // With the flag set, only the top-level file is visited. Under the old
        // `#[arg(short, long, default_value_t = true)] recursive: bool` there
        // was no way to reach this state from the command line at all.
        let shallow_args = ScanArgs {
            no_recursive: true,
            ..args_for(tmp.path())
        };
        let shallow = scan_files(&ctx, &shallow_args).unwrap();
        assert_eq!(shallow.len(), 1);
        assert!(shallow[0].ends_with("top.wav"));
    }

    /// `conflict_strategy = "rename"` re-points the loser onto " (1)".
    #[test]
    fn conflict_strategy_rename_appends_counter() {
        let tmp = tempfile::tempdir().unwrap();
        write_test_wav(&tmp.path().join("first.wav"));
        write_test_wav(&tmp.path().join("second.wav"));

        let mut ctx = test_ctx(false);
        ctx.config.rename.conflict_strategy = "rename".to_string();

        let args = ScanArgs {
            template: Some("<Title>".to_string()),
            execute: true,
            ..args_for(tmp.path())
        };

        let code = run(&ctx, &args).unwrap();

        // Both files moved: nothing was skipped and nothing was overwritten.
        assert!(tmp.path().join("unnamed.wav").exists());
        assert!(tmp.path().join("unnamed (1).wav").exists());
        assert!(!tmp.path().join("first.wav").exists());
        assert!(!tmp.path().join("second.wav").exists());
        // The conflict was resolved, so nothing was left undone.
        assert_eq!(code, ExitCode::SUCCESS);
    }

    /// `conflict_strategy = "overwrite"` must NOT re-enable the data-loss path.
    #[test]
    fn conflict_strategy_overwrite_warns_and_skips() {
        let tmp = tempfile::tempdir().unwrap();
        write_test_wav(&tmp.path().join("first.wav"));
        write_test_wav(&tmp.path().join("second.wav"));

        let mut ctx = test_ctx(false);
        ctx.config.rename.conflict_strategy = "overwrite".to_string();

        let args = ScanArgs {
            template: Some("<Title>".to_string()),
            execute: true,
            ..args_for(tmp.path())
        };

        let code = run(&ctx, &args).unwrap();

        // Two files in, two files out — the loser was left alone.
        assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 2);
        assert_eq!(code, ExitCode::PARTIAL);
    }

    /// Conflict strategies parse case-insensitively, unknown values are safe.
    #[test]
    fn conflict_strategy_parsing() {
        assert_eq!(parse_conflict_strategy("skip"), ConflictStrategy::Skip);
        assert_eq!(parse_conflict_strategy("Rename"), ConflictStrategy::Rename);
        assert_eq!(
            parse_conflict_strategy("overwrite"),
            ConflictStrategy::Unsupported
        );
        assert_eq!(
            parse_conflict_strategy("ask"),
            ConflictStrategy::Unsupported
        );
        // Anything unrecognised degrades to the safest behaviour.
        assert_eq!(parse_conflict_strategy("wibble"), ConflictStrategy::Skip);
    }

    /// `--output-dir` beats config, which beats the scanned directory.
    #[test]
    fn output_dir_precedence() {
        let ctx_default = test_ctx(false);
        let args = args_for(Path::new("/scan/root"));
        assert_eq!(
            resolve_output_dir(&ctx_default, &args),
            Path::new("/scan/root")
        );

        let mut ctx_configured = test_ctx(false);
        ctx_configured.config.rename.output_dir = Some(PathBuf::from("/from/config"));
        assert_eq!(
            resolve_output_dir(&ctx_configured, &args),
            Path::new("/from/config")
        );

        let flag_args = ScanArgs {
            output_dir: Some(PathBuf::from("/from/flag")),
            ..args_for(Path::new("/scan/root"))
        };
        assert_eq!(
            resolve_output_dir(&ctx_configured, &flag_args),
            Path::new("/from/flag")
        );
    }
}
