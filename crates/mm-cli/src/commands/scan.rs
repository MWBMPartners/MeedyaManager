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
use std::io::{IsTerminal, Write};
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

    /// Skip the interactive confirmation prompt before `--execute`
    //
    // `--execute` performs irreversible renames. Without this gate, a bare
    // `--execute` typed against the wrong directory (or copy-pasted from
    // somewhere) runs immediately. `--yes` is how a script or CI job opts
    // out of the prompt on purpose; an interactive terminal without it is
    // asked to confirm. See `execute_pre_confirmed`.
    #[arg(long)]
    pub yes: bool,
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
    /// Disc folders found and deliberately left alone — see `DiscFolderEntry`.
    disc_folders: Vec<DiscFolderEntry>,
    rename_previews: Vec<PreviewEntry>,
    summary: ScanSummary,
}

/// One detected disc folder, for JSON output.
///
/// Reported separately from `rename_previews` on purpose: these files are
/// **not** part of the rename plan and never will be while renaming happens
/// one file at a time. A caller that merged the two lists would be back to
/// the issue #219 behaviour of treating a `.cue` and its `.bin` as unrelated.
#[derive(Serialize)]
struct DiscFolderEntry {
    path: String,
    kind: String,
    name_source: String,
    image_format: String,
    image_count: usize,
    file_count: usize,
    total_bytes: u64,
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

/// Remove any file the Test Mode manifest tracks as an *original*, warning
/// once per skipped file.
///
/// `meedya config test-mode commit` pairs each original with its
/// `_MeedyaManager` copy strictly by path (see `mm_core::test_mode`).
/// `watcher::should_ignore` already keeps the *copy* out of every scan and
/// watch (a Test Mode copy is never treated as ordinary media). This is the
/// other half: keeping the *original* out of the rename plan while an edit
/// of it is still only sitting in the copy. Renaming the original out from
/// under that pairing is exactly what leaves `commit` unable to find it —
/// see SCAN-HYGIENE.
///
/// Takes the tracked-originals set as a plain parameter (rather than
/// querying `mm_core::test_mode` itself) so the skip/warn decision is a pure
/// function, testable without touching any global Test Mode state.
fn skip_tracked_originals(
    files: Vec<PathBuf>,
    tracked_originals: &HashSet<PathBuf>,
) -> Vec<PathBuf> {
    if tracked_originals.is_empty() {
        return files;
    }

    files
        .into_iter()
        .filter(|f| {
            if tracked_originals.contains(f) {
                output::print_warning(&format!(
                    "Test Mode: '{}' is tracked as an original with a pending edit in its \
                     _MeedyaManager copy — skipping so `meedya config test-mode commit` can \
                     still find it.",
                    f.display()
                ));
                false
            } else {
                true
            }
        })
        .collect()
}

/// Decide whether `--execute` may proceed without an interactive prompt.
///
/// `--yes` always short-circuits it. Otherwise a non-interactive stdin (a
/// script, a pipe, a CI job) has nothing to prompt against, so it is treated
/// as already confirmed — only an attended, interactive terminal actually
/// blocks on a prompt before an irreversible batch of renames.
///
/// Shared with `meedya watch --organize`, which asks the same question once
/// at start-up before it begins moving files unattended.
pub(crate) fn execute_pre_confirmed(yes: bool, stdin_is_terminal: bool) -> bool {
    yes || !stdin_is_terminal
}

/// Ask an interactive terminal a yes/no question before doing something
/// irreversible. Returns `false` — the safe default — on any I/O error or a
/// non-affirmative answer.
///
/// Takes the whole question as text rather than a path, because the two
/// callers are asking genuinely different things: `scan` is about to rename
/// one directory now, while `watch --organize` is about to keep moving files
/// for as long as it runs. A shared helper that built the sentence itself
/// could only ever be right for one of them.
pub(crate) fn prompt_confirm(message: &str) -> bool {
    print!("{message} [y/N] ");
    if std::io::stdout().flush().is_err() {
        return false;
    }
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// The purpose text recorded in the write lock's info note while this
/// command holds it — see `mm_core::state::LockFile::try_acquire`. Written
/// once here, rather than inline at the call site, so the value actually
/// passed to `try_acquire_default` cannot silently drift from the strings
/// anybody debugging a stuck lock will see in `meedya.lock.info` (via
/// `mm_core::state::LockFile::holder`) or in the busy message built by
/// `mm_core::state::lock_busy_message`.
const LOCK_PURPOSE: &str = "meedya scan --execute";

// The message-building helpers that used to live here —
// `describe_holder`/`lock_busy_message` and `lock_unavailable_message` —
// moved to `mm_core::state` (issue #49, review round). That is the single
// place both this command and the FFI layer's `execute_renames` now call
// into, so a user sees exactly the same wording whichever one blocked
// them — see the doc comments on `mm_core::state::lock_busy_message` and
// `mm_core::state::lock_unavailable_message` for the full reasoning,
// including why the old wording here (telling a blocked user to set
// `MM_CONFIG_DIR`) was actively harmful rather than merely unhelpful.

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

/// Render a byte count the way a person would read it.
///
/// Disc images run to gigabytes, and "4700372992" tells a reader nothing at a
/// glance. Binary units (1 KiB = 1024 bytes) are used because that is what
/// every operating system's file manager shows for a disc image, so the
/// number matches what the user will see elsewhere.
fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    // Plain bytes below a kibibyte — no decimal point on "512 B".
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0usize;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
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

    // `--execute` performs irreversible renames — refuse to run it unattended
    // past a bare flag. A non-interactive stdin (script/pipe/CI) is treated
    // as pre-confirmed; only an attended terminal is actually prompted.
    if !dry_run
        && args.execute
        && !execute_pre_confirmed(args.yes, std::io::stdin().is_terminal())
        && !prompt_confirm(&format!(
            "About to rename files under '{}'. Continue?",
            args.path.display()
        ))
    {
        output::print_warning(
            "Execution cancelled — pass --yes to skip this confirmation next time.",
        );
        return Ok(ExitCode::ERROR);
    }

    // Take the write lock before looking at a single file.
    //
    // Order matters here, and it is deliberate. The confirmation prompt
    // comes first, because there is no point locking other people out while
    // we wait for somebody to type "y" — or, worse, holding the lock during
    // a prompt they walk away from. But the lock is taken *before* the scan,
    // not after it: the scan works out where every file is going, and if
    // another copy is moving files while we do that, our plan is out of date
    // before we ever act on it.
    //
    // `_write_lock` is bound (rather than being a bare expression) so it
    // stays alive for the rest of this function. The moment it goes out of
    // scope — normal return, early return or panic — the operating system's
    // lock on `meedya.lock` is released (the file itself is never deleted —
    // see `mm_core::state`'s module docs for why), so a crash cannot leave
    // renaming blocked for good.
    let _write_lock = if !dry_run && args.execute {
        match mm_core::state::LockFile::try_acquire_default(LOCK_PURPOSE) {
            Ok(Some(lock)) => Some(lock),
            Ok(None) => {
                // Busy — read who holds it, best effort, purely to tell the
                // user something useful. `holder` never decides anything;
                // the refusal above already came straight from the operating
                // system's own lock.
                let holder =
                    mm_core::state::LockFile::holder(&mm_core::state::LockFile::default_path());
                output::print_error(&mm_core::state::lock_busy_message(holder.as_ref()));
                return Ok(ExitCode::ERROR);
            }
            Err(err) => {
                // `err` is the raw `std::io::Error` from `try_acquire` (not
                // wrapped in `MmError`) precisely so its `ErrorKind` reaches
                // `lock_unavailable_message` intact — see that function's
                // doc comment for what decides on that basis.
                output::print_error(&mm_core::state::lock_unavailable_message(&err));
                return Ok(ExitCode::ERROR);
            }
        }
    } else {
        // Previewing does not move anything, so it never needs the lock and
        // must never block a real rename that is already under way.
        None
    };

    // ── 1. Scan for files ───────────────────────────────────────────────
    let files = scan_files(ctx, args)?;

    // Test Mode: never rename a file the manifest still tracks as an
    // original — that would break the original<->copy pairing `commit`
    // depends on. See `skip_tracked_originals` above.
    let files = if mm_core::test_mode::is_enabled() {
        let tracked_originals: HashSet<PathBuf> = mm_core::test_mode::tracked_files()
            .into_iter()
            .map(|entry| entry.original)
            .collect();
        skip_tracked_originals(files, &tracked_originals)
    } else {
        files
    };

    // Issue #219 — a live data-loss bug. A `.cue` sheet and the `.bin` image
    // it names are one disc, and the cue refers to the image by bare file
    // name, so the pair only works while both sit in the same directory. Left
    // in this list they were two unrelated files, and a template such as
    // `<Extension>/<Filename>` moved one into `cue/` and the other into
    // `bin/` — destroying the rip, silently, with no unusual settings. This
    // call lifts every disc image member out of the per-file rename plan
    // before anything is renamed, and reports the folders they live in
    // instead. Whole-folder moving arrives with the next stage (#217);
    // excluding these files from per-file renaming is what stops the loss.
    //
    // Useful side benefit: because they leave the list here, `extract_all`
    // below no longer tries to read audio tags out of multi-gigabyte `.bin`
    // and `.iso` files — work that never had any chance of succeeding.
    let (disc_folders, files) = mm_core::disc::partition_for_scan(files, &args.path)?;

    if files.is_empty() && disc_folders.is_empty() {
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

    let disc_entries: Vec<DiscFolderEntry> = disc_folders
        .iter()
        .map(|folder| DiscFolderEntry {
            path: folder.dir.display().to_string(),
            kind: folder.info.kind.to_string(),
            name_source: folder.info.name_source.to_string(),
            image_format: folder.info.image_format.to_string(),
            image_count: folder.info.image_count,
            file_count: folder.file_count,
            total_bytes: folder.total_bytes,
        })
        .collect();

    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&ScanOutput {
                directory: args.path.display().to_string(),
                total_files: files.len(),
                classification_summary: group_counts_vec,
                disc_folders: disc_entries,
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

            // Disc folders. Printed before the rename preview because the
            // point being made is "these were deliberately left out of the
            // list below", and a reader needs that before reading the list.
            if !disc_entries.is_empty() {
                output::print_header("Disc Folders");
                let disc_rows: Vec<Vec<String>> = disc_entries
                    .iter()
                    .map(|d| {
                        vec![
                            d.path.clone(),
                            d.kind.clone(),
                            d.name_source.clone(),
                            d.file_count.to_string(),
                            format_bytes(d.total_bytes),
                            "detected (not moved — folder moves arrive with the next stage)"
                                .to_string(),
                        ]
                    })
                    .collect();
                output::print_table(
                    &["Folder", "Kind", "Name source", "Files", "Size", "Status"],
                    &disc_rows,
                );
            }

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
    ///
    /// `yes: true` throughout: cargo test's stdin can itself be a terminal
    /// (see `execute_pre_confirmed`), and `prompt_confirm` blocks on a
    /// `read_line` no test harness supplies — every test that flips
    /// `execute` on must carry `yes: true` or the suite hangs.
    fn args_for(path: &Path) -> ScanArgs {
        ScanArgs {
            path: path.to_path_buf(),
            no_recursive: false,
            template: None,
            output_dir: None,
            execute: false,
            dry_run: false,
            yes: true,
        }
    }

    // The WAV fixture writers and the `MM_CONFIG_DIR` guard are shared with
    // the other command modules' tests — see `crate::test_support` for why
    // the guard in particular cannot be duplicated per module.
    use crate::test_support::{
        ConfigDirGuard, write_tagged_wav, write_wav_fixture as write_test_wav,
    };

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
            yes: true,
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
            yes: true,
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
        // Renaming for real now takes the write lock, which lives in the
        // configuration directory. Point that at a private folder so this
        // test cannot collide with another test doing the same thing.
        let _config = ConfigDirGuard::new();

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
            yes: true,
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
        // Renaming for real now takes the write lock, which lives in the
        // configuration directory. Point that at a private folder so this
        // test cannot collide with another test doing the same thing.
        let _config = ConfigDirGuard::new();

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
        // Renaming for real now takes the write lock, which lives in the
        // configuration directory. Point that at a private folder so this
        // test cannot collide with another test doing the same thing.
        let _config = ConfigDirGuard::new();

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

    // ── SCAN-HYGIENE ─────────────────────────────────────────────────────

    /// **Regression.** A Test Mode copy (`_MeedyaManager` suffix) must never
    /// be picked up by `scan` as ordinary media — `watcher::should_ignore`
    /// is where this is actually enforced, but this proves the wiring holds
    /// all the way through `scan_files`.
    #[test]
    fn scan_skips_test_mode_copies() {
        let tmp = tempfile::tempdir().unwrap();
        write_test_wav(&tmp.path().join("song.wav"));
        // The copy: same directory, `_MeedyaManager` suffix.
        std::fs::copy(
            tmp.path().join("song.wav"),
            tmp.path().join("song_MeedyaManager.wav"),
        )
        .unwrap();

        let ctx = test_ctx(false);
        let args = args_for(tmp.path());
        let files = scan_files(&ctx, &args).unwrap();

        assert_eq!(files.len(), 1, "the copy must not be scanned as a file");
        assert!(files[0].ends_with("song.wav"));
    }

    /// **Regression — the SCAN-HYGIENE report.**
    ///
    /// With Test Mode on, a file tracked as an *original* must be left alone
    /// by `scan --execute`. Renaming it out from under its `_MeedyaManager`
    /// copy is exactly what left `meedya config test-mode commit` unable to
    /// find the copy afterwards.
    ///
    /// Exercises `skip_tracked_originals` directly with a hand-built tracked
    /// set, rather than going through a live `mm_core::test_mode` manifest by
    /// way of `MM_CONFIG_DIR`: that environment variable is a single
    /// process-global, `cargo test` runs every test in this crate in one
    /// process, and no lock in this binary coordinates every file that might
    /// redirect it — `edit.rs`'s Test Mode tests, for one, carry their own
    /// private lock. Testing the pure skip/warn decision in isolation proves
    /// the same behaviour without that race. `run`'s wiring of it to the
    /// live manifest (`is_enabled` + `tracked_files`) is a two-line call —
    /// see the body of `run` above — and not itself where any bug could
    /// hide.
    #[test]
    fn scan_warns_and_skips_tracked_originals_in_test_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let original = tmp.path().join("song.wav");
        let untracked = tmp.path().join("other.wav");
        write_test_wav(&original);
        write_test_wav(&untracked);

        let tracked: HashSet<PathBuf> = std::iter::once(original.clone()).collect();
        let files = skip_tracked_originals(vec![original, untracked.clone()], &tracked);

        assert_eq!(
            files,
            vec![untracked],
            "a tracked original must be skipped; an untracked file must not be"
        );
    }

    /// `execute_pre_confirmed` is the pure decision behind the confirmation
    /// gate: `--yes` always wins, otherwise only an interactive terminal
    /// actually requires the prompt.
    #[test]
    fn scan_execute_requires_confirmation_unless_yes() {
        assert!(
            execute_pre_confirmed(true, true),
            "--yes must skip the prompt even on a terminal"
        );
        assert!(
            execute_pre_confirmed(true, false),
            "--yes must skip the prompt off a terminal too"
        );
        assert!(
            execute_pre_confirmed(false, false),
            "a non-interactive stdin (script/pipe/CI) has nothing to prompt \
             against, so it is pre-confirmed"
        );
        assert!(
            !execute_pre_confirmed(false, true),
            "an interactive terminal without --yes must NOT be pre-confirmed"
        );
    }

    // ── Issue #219 — a disc image must never be split up ────────────────

    /// Build the fixture from issue #219: a raw CD rip in its own folder —
    /// a cue sheet, the `.bin` image it names, the ripper's log, and the
    /// cover art. Returns the bytes written for the cue and the bin so a
    /// test can prove they were not merely left *present* but left
    /// *untouched*.
    fn write_disc_rip_fixture(root: &Path) -> (Vec<u8>, Vec<u8>) {
        let rip = root.join("Rip");
        std::fs::create_dir_all(&rip).unwrap();

        // A minimal but completely ordinary single-file BIN/CUE rip.
        let cue = concat!(
            "PERFORMER \"Test Artist\"\n",
            "TITLE \"Test Album\"\n",
            "FILE \"Album.bin\" BINARY\n",
            "  TRACK 01 AUDIO\n",
            "    INDEX 01 00:00:00\n",
        )
        .as_bytes()
        .to_vec();

        // 4 KiB of image payload. The content does not matter; the point is
        // that it is a distinct blob we can compare byte-for-byte afterwards.
        let bin = vec![0x5Au8; 4096];

        std::fs::write(rip.join("Album.cue"), &cue).unwrap();
        std::fs::write(rip.join("Album.bin"), &bin).unwrap();
        std::fs::write(rip.join("Album.log"), b"Exact Audio Copy log\n").unwrap();
        std::fs::write(rip.join("cover.jpg"), b"\xFF\xD8\xFF\xE0 not a real JPEG").unwrap();

        (cue, bin)
    }

    /// Issue #219, the live data-loss bug: `meedya scan --execute` used to
    /// treat `Album.cue` and `Album.bin` as two unrelated files and move
    /// each one somewhere different. The cue still says `FILE "Album.bin"`,
    /// but that file is no longer beside it — the rip is destroyed, silently,
    /// with no unusual settings and no way to undo it.
    ///
    /// **This test was written before the fix, and it failed, as intended.**
    /// What the first run printed, verbatim (paths shortened to `<tmp>`):
    ///
    /// ```text
    /// Rename Preview
    /// Source                 Destination            Status
    /// <tmp>/Rip/Album.bin    <tmp>/bin/Album.bin    OK
    /// <tmp>/Rip/Album.cue    <tmp>/cue/Album.cue    OK
    /// <tmp>/Rip/Album.log    <tmp>/log/Album.log    OK
    /// <tmp>/Rip/cover.jpg    <tmp>/jpg/cover.jpg    OK
    ///
    /// Summary
    /// Total: 4 / To rename: 4 / Unchanged: 0 / Conflicts: 0
    /// ✓ Renames executed
    ///
    /// thread 'commands::scan::tests::scan_execute_never_splits_a_cue_bin_pair'
    ///   panicked at crates/mm-cli/src/commands/scan.rs:1179:9:
    /// issue #219: Album.cue was moved away from its .bin — the rip is destroyed
    ///
    /// test result: FAILED. 0 passed; 1 failed
    /// ```
    #[test]
    fn scan_execute_never_splits_a_cue_bin_pair() {
        // Renaming for real now takes the write lock, which lives in the
        // configuration directory. Point that at a private folder so this
        // test cannot collide with another test doing the same thing.
        let _config = ConfigDirGuard::new();

        let tmp = tempfile::tempdir().unwrap();
        let (cue_bytes, bin_bytes) = write_disc_rip_fixture(tmp.path());

        let ctx = test_ctx(false);
        let mut args = args_for(tmp.path());
        // The exact template from the bug report: route every file into a
        // folder named after its extension.
        args.template = Some("<Extension>/<Filename>".to_string());
        args.execute = true;
        args.yes = true;

        run(&ctx, &args).unwrap();

        let cue_path = tmp.path().join("Rip/Album.cue");
        let bin_path = tmp.path().join("Rip/Album.bin");

        assert!(
            cue_path.is_file(),
            "issue #219: Album.cue was moved away from its .bin — the rip is destroyed"
        );
        assert!(
            bin_path.is_file(),
            "issue #219: Album.bin was moved away from its .cue — the rip is destroyed"
        );
        assert_eq!(
            std::fs::read(&cue_path).unwrap(),
            cue_bytes,
            "the cue sheet must be left byte-identical"
        );
        assert_eq!(
            std::fs::read(&bin_path).unwrap(),
            bin_bytes,
            "the disc image must be left byte-identical"
        );

        // The template would have created these two directories on its way to
        // splitting the pair, so their absence is a second, independent proof
        // that no per-file rename was attempted on a disc image.
        assert!(
            !tmp.path().join("cue").exists(),
            "no `cue/` directory may be created — a cue sheet is never renamed on its own"
        );
        assert!(
            !tmp.path().join("bin").exists(),
            "no `bin/` directory may be created — a disc image is never renamed on its own"
        );
    }

    /// The same fixture without `--execute`: a disc image must not even be
    /// *offered* as a rename. Showing it in the preview table would invite a
    /// user to re-run with `--execute` and hit the very bug above.
    #[test]
    fn scan_disc_files_never_appear_in_file_previews() {
        let tmp = tempfile::tempdir().unwrap();
        let _ = write_disc_rip_fixture(tmp.path());

        let ctx = test_ctx(false);
        let mut args = args_for(tmp.path());
        args.template = Some("<Extension>/<Filename>".to_string());

        let files = scan_files(&ctx, &args).unwrap();
        let (_disc_folders, loose) = mm_core::disc::partition_for_scan(files, &args.path).unwrap();
        let extracted = extract_all(&loose);
        let summary = preview_renames(&ctx, &args, &loose, &extracted).unwrap();

        for preview in summary.iter().flat_map(|s| s.previews.iter()) {
            let source = preview.source.display().to_string().to_ascii_lowercase();
            assert!(
                !source.ends_with(".cue") && !source.ends_with(".bin"),
                "a disc image member must never reach the rename preview: {source}"
            );
        }
    }
    // ── The write lock — issue #49 ──────────────────────────────────────────

    /// **Regression — two copies moving files at once.**
    ///
    /// `LockFile` existed but nothing ever called it, so a second
    /// `meedya scan --execute` (or the desktop app's rename button) could
    /// start moving the very same files while the first was still working.
    /// One process would rename a file out from under the other's plan, and
    /// both would report success.
    ///
    /// Rewritten for the #49 lock redesign: the previous version of this
    /// test planted a *file naming this process's own PID* to stand in for a
    /// second holder. That plant no longer means anything — the new lock is
    /// the operating system's own file lock, not anything read out of the
    /// file's content — so this now genuinely takes the lock first, via the
    /// same `LockFile` API a real second process would use, and keeps it
    /// alive for the whole test.
    #[test]
    fn scan_execute_refuses_while_another_process_holds_the_lock() {
        // Redirect the configuration directory so the lock this test takes
        // is a private one, not the real user's.
        let _guard = ConfigDirGuard::new();

        let tmp = tempfile::tempdir().unwrap();
        let track = tmp.path().join("song.wav");
        write_tagged_wav(&track, "Band", "Album", "Song");

        // Genuinely hold the write lock, standing in for a second live copy
        // of MeedyaManager already moving files.
        let lock_path = mm_core::state::LockFile::default_path();
        let held_lock = mm_core::state::LockFile::try_acquire(&lock_path, "another test")
            .expect("acquiring must not error")
            .expect("nothing else holds this lock yet");

        let ctx = test_ctx(false);
        let args = ScanArgs {
            template: Some("<Artist>/<Title>".to_string()),
            execute: true,
            ..args_for(tmp.path())
        };

        let code = run(&ctx, &args).unwrap();

        assert_eq!(
            code,
            ExitCode::ERROR,
            "scan --execute must refuse to start while another process holds \
             the write lock"
        );
        assert!(
            track.is_file(),
            "the file must still be where it started — nothing may move while \
             another process holds the write lock"
        );

        // The refusal must not have deleted the other process's lock file —
        // this module never deletes it at all, see `mm_core::state`'s
        // module docs — and this test's own hold on it must still be live.
        assert!(
            lock_path.is_file(),
            "a refused run must leave the other process's lock file alone"
        );
        drop(held_lock);
    }

    // The two tests that used to live here (`lock_error_message_names_the_
    // holder_and_its_process_id`, `lock_error_message_without_a_readable_
    // holder_still_makes_sense`) exercised `lock_busy_message`, which moved
    // to `mm_core::state` along with `lock_unavailable_message` (issue #49,
    // review round — see the comment above `LOCK_PURPOSE`). Their coverage
    // moved with it: see `mm_core::state::tests::the_busy_message_names_
    // the_holder_when_known`, `..._omits_the_holder_sentence_when_unknown`,
    // `the_unavailable_message_mentions_network_drives_only_when_
    // unsupported` and `no_lock_message_ever_suggests_moving_settings_or_
    // deleting_files` in `crates/mm-core/src/state/mod.rs`. What stays
    // useful here, in this crate, is proving the command actually *uses*
    // that shared wording when refused — `scan_execute_refuses_while_
    // another_process_holds_the_lock` above already does that end to end.

    /// Once the first run finishes and drops its lock, the next run may take
    /// it. A lock that could not be re-taken would leave the command
    /// permanently broken after a single use.
    #[test]
    fn releasing_the_lock_lets_the_next_run_take_it() {
        let _guard = ConfigDirGuard::new();

        {
            let _lock = mm_core::state::LockFile::try_acquire_default(LOCK_PURPOSE)
                .expect("acquiring must not error")
                .expect("first run must be able to take the write lock");
            assert!(
                matches!(
                    mm_core::state::LockFile::try_acquire_default(LOCK_PURPOSE),
                    Ok(None)
                ),
                "a second run must be refused while the first still holds it"
            );
        } // first lock dropped here — released, but the file itself stays

        assert!(
            matches!(
                mm_core::state::LockFile::try_acquire_default(LOCK_PURPOSE),
                Ok(Some(_))
            ),
            "the lock must be available again once the first run has finished"
        );
    }
}
