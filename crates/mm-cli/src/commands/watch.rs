// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya watch` Command
//
// Foreground file system watcher. Monitors directories for media file changes
// and logs events with timestamps. With `--organize` it also renames and moves
// each new file into place, which is what the background service installed by
// `meedya service install` actually runs.
//
// Design note — why organising is a thin shell over `scan`
// --------------------------------------------------------
// Everything organising needs already exists in `meedya scan --execute`: the
// file walk, the Test Mode hygiene rules, the whole-folder protection that
// stops a `.cue` being separated from its `.bin` (issue #219), the conflict
// policy, copy mode, the output directory and the write lock that keeps two
// copies of MeedyaManager from moving the same files at once.
//
// Re-implementing any of that here would mean two copies of the same rules,
// and the last time this project kept two copies of the rename logic they
// drifted apart and silently destroyed files. So `watch --organize` builds a
// `ScanArgs` by hand and calls `scan::run` as an ordinary function. When a
// safety rule is added to `scan`, the watcher inherits it for free.

use crate::commands::scan::{self, ScanArgs};
use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use chrono::Local;
use clap::Args;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

// ─── Tuning constants ───────────────────────────────────────────────────────

/// How long the event loop waits for the next file-system event before it
/// stops listening and goes to look at the files it is already holding.
///
/// This is not the settle window — it is only how often the loop wakes up to
/// check whether the settle window has passed for anything. A quarter of a
/// second is short enough that a two-second settle is honoured to within a
/// blink, and long enough that an idle watcher costs nothing measurable.
const EVENT_POLL: Duration = Duration::from_millis(250);

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya watch` command.
#[derive(Args, Debug)]
pub struct WatchArgs {
    /// Directories to watch (uses config folders if empty)
    pub paths: Vec<PathBuf>,

    /// Disable recursive watching
    #[arg(long)]
    pub no_recursive: bool,

    /// Auto-organise files when changes are detected
    #[arg(long)]
    pub organize: bool,

    /// Skip the one-off confirmation prompt before `--organize` starts moving files
    //
    // `--organize` moves files without anybody watching, so an attended
    // terminal is asked to confirm once at start-up. A service, a script or a
    // cron job has nobody to answer that question, so it passes `--yes`.
    #[arg(long)]
    pub yes: bool,

    /// How many seconds a file must go untouched before it is organised
    //
    // Deliberately a command-line flag and *not* a settings file field. A new
    // settings field would have to be added to the shipped settings bundle as
    // well, or the drift guard from issue #211 would start failing — and this
    // value is a property of one watch run, not of the library.
    //
    // Why any wait at all: a file being copied in fires a stream of "modified"
    // events while it is still half-written. Organising it at the first event
    // would read the tags out of an incomplete file. Waiting until the file
    // has been quiet for a few seconds is the cheapest way to tell "finished"
    // from "still arriving" without asking the operating system for a lock.
    #[arg(long, default_value_t = 2)]
    pub settle_secs: u64,
}

// ─── JSON output structures ─────────────────────────────────────────────────

/// Watch event for JSON output.
#[derive(Serialize)]
struct WatchEventOutput {
    timestamp: String,
    event_type: String,
    path: String,
}

/// Watch startup info for JSON output.
#[derive(Serialize)]
struct WatchStartOutput {
    folders: Vec<String>,
    recursive: bool,
    organize: bool,
    settle_secs: u64,
}

// ─── Conflict policy for unattended organising ──────────────────────────────

/// Force the conflict strategy to "skip" for this watch run, returning the
/// setting it replaced when that setting was something else.
///
/// **Why this is not optional.** `mm_core::renamer::resolve_conflict_by_counter`
/// rejects any candidate name that already exists on disk — and when a file is
/// already sitting at its destination, the file *itself* is what exists. Under
/// the "rename" strategy that turns a settled file into `Title (1).mp3`. Moving
/// it generates a fresh event, the next pass sees `Title (1).mp3` in the way
/// and produces `Title (2).mp3`, and so on. Attended, somebody notices after
/// the second copy. Unattended, in a service that runs from login until
/// shutdown, it does not stop.
///
/// Skipping instead is what makes the watcher **idempotent**: a file that is
/// already where it belongs is left alone, its own move-event settles, the
/// folder is re-scanned once, nothing changes, and the churn ends there.
///
/// The underlying counter bug is real and is being tracked separately; forcing
/// "skip" here contains it rather than pretending it is fixed.
fn force_skip_conflicts(config: &mut mm_core::config::AppConfig) -> Option<String> {
    let previous = config.rename.conflict_strategy.clone();
    config.rename.conflict_strategy = "skip".to_string();

    if previous.trim().eq_ignore_ascii_case("skip") {
        // Already what we were going to use — nothing worth telling anybody.
        None
    } else {
        Some(previous)
    }
}

// ─── Settle logic ───────────────────────────────────────────────────────────

/// Files seen but not yet organised, each with the moment it was last touched.
type Pending = HashMap<PathBuf, Instant>;

/// Note what one file-system event means for the pending list.
///
/// A rename is two things at once: the old path has gone and the new path has
/// just appeared, so it is recorded as both. A deletion simply drops the file —
/// there is nothing left to organise, and leaving it in the list would only
/// send a pointless scan at the folder later.
fn record_event(pending: &mut Pending, event: &mm_core::watcher::WatchEvent, now: Instant) {
    use mm_core::watcher::WatchEvent;
    match event {
        WatchEvent::Created(path) | WatchEvent::Modified(path) => {
            pending.insert(path.clone(), now);
        }
        WatchEvent::Deleted(path) => {
            pending.remove(path);
        }
        WatchEvent::Renamed(from, to) => {
            pending.remove(from);
            pending.insert(to.clone(), now);
        }
    }
}

/// Take out every file that has now been quiet for at least `settle`.
///
/// Removing them here — rather than copying them and clearing the list later —
/// is deliberate: a file that is being organised must not also still be
/// counted as waiting, or a second event arriving mid-move would queue it
/// twice. Anything that fails to organise is deliberately put back by the
/// caller with a fresh timestamp.
///
/// The result is sorted so that grouping, and therefore the order folders are
/// organised in, is the same on every run. Tests depend on that; so does
/// anybody reading the log afterwards.
fn take_settled(pending: &mut Pending, now: Instant, settle: Duration) -> Vec<PathBuf> {
    let mut ready: Vec<PathBuf> = pending
        .iter()
        // `saturating_duration_since` rather than plain subtraction: a clock
        // reading taken before the entry was stamped must read as "no time has
        // passed", never as a panic or a huge number.
        .filter(|(_, last_touched)| now.saturating_duration_since(**last_touched) >= settle)
        .map(|(path, _)| path.clone())
        .collect();

    ready.sort();
    for path in &ready {
        pending.remove(path);
    }
    ready
}

/// Group settled files by the folder they sit in.
///
/// Organising works a folder at a time, not a file at a time, because that is
/// the unit `scan` understands — and because ten files dropped into one album
/// folder should cost one scan, not ten. `BTreeMap` keeps the folders in a
/// stable, predictable order.
///
/// A path with no parent at all (the root of a drive) is dropped: there is no
/// folder to scan for it.
fn group_by_parent(files: &[PathBuf]) -> BTreeMap<PathBuf, Vec<PathBuf>> {
    let mut grouped: BTreeMap<PathBuf, Vec<PathBuf>> = BTreeMap::new();
    for file in files {
        if let Some(parent) = file.parent() {
            grouped
                .entry(parent.to_path_buf())
                .or_default()
                .push(file.clone());
        }
    }
    grouped
}

/// Find which watched folder a path belongs to.
///
/// The **longest** match wins, and that matters: somebody can perfectly
/// reasonably watch both `~/Media` and `~/Media/Incoming`. A file dropped in
/// the second one belongs to the second one, because that is the root its
/// organised copy should be laid out under. Taking the first match instead
/// would file it under the wrong tree, depending purely on config order.
fn watched_root_for<'a>(path: &Path, roots: &'a [PathBuf]) -> Option<&'a Path> {
    roots
        .iter()
        .filter(|root| path.starts_with(root))
        .max_by_key(|root| root.components().count())
        .map(PathBuf::as_path)
}

// ─── Organising ─────────────────────────────────────────────────────────────

/// Organise one folder, laying the results out under `root`.
///
/// `dir` is the folder to look in — the watched root itself for the start-up
/// sweep, or just the folder an event happened in the rest of the time.
/// `root` is the watched folder the results belong under, and it is the whole
/// reason a file dropped into `<root>/incoming` ends up at
/// `<root>/Artist/Album/Title.wav` rather than buried at
/// `<root>/incoming/Artist/Album/Title.wav`.
///
/// Dry-run needs no special handling here: `scan::run` treats the context's
/// dry-run flag as an override, so `meedya --dry-run watch --organize` previews
/// and moves nothing.
fn organise_directory(
    ctx: &CliContext,
    dir: &Path,
    root: &Path,
    recursive: bool,
) -> anyhow::Result<i32> {
    let args = ScanArgs {
        path: dir.to_path_buf(),
        no_recursive: !recursive,
        // No override — the template (and any rules) come from the config,
        // which is the whole point of a service that organises to *your* rules.
        template: None,
        output_dir: Some(root.to_path_buf()),
        execute: true,
        dry_run: false,
        // The person running the watcher confirmed once at start-up; asking
        // again per folder would block a background service forever.
        yes: true,
    };

    scan::run(ctx, &args)
}

/// Everything the background thread needs in order to organise.
///
/// Holds its own `CliContext` — a clone of the command's, with the conflict
/// strategy forced to "skip" — because the thread outlives the borrow `run`
/// was handed, and because that one setting must be changed for the watcher
/// without changing it for anything else.
struct Organiser {
    ctx: CliContext,
    /// The watched folders, in the order they were given.
    roots: Vec<PathBuf>,
    /// Whether to look inside subfolders during the start-up sweep.
    recursive: bool,
    /// How long a file must be quiet before it is considered finished.
    settle: Duration,
}

impl Organiser {
    /// Organise each watched folder once, before the event loop starts.
    ///
    /// **Why this exists.** A watcher only ever hears about changes that
    /// happen while it is running. Files that arrived overnight, while the
    /// service was stopped, or during a reboot, produce no event at all and
    /// would sit there untouched forever. The sweep is what makes "install the
    /// service and forget about it" actually true.
    fn sweep_roots(&self) {
        if self.ctx.output == OutputFormat::Human {
            output::print_header("Start-up sweep");
            println!(
                "  Files that arrived while the watcher was not running never produce an\n  \
                 event, so each watched folder is organised once now."
            );
            println!();
        }

        for root in &self.roots {
            if let Err(err) = organise_directory(&self.ctx, root, root, self.recursive) {
                output::print_error(&format!(
                    "Start-up sweep of '{}' failed: {err}",
                    root.display()
                ));
            }
        }
    }

    /// Organise everything that has now settled, folder by folder.
    fn organise_settled(&self, pending: &mut Pending) {
        let settled = take_settled(pending, Instant::now(), self.settle);
        if settled.is_empty() {
            return;
        }

        for (folder, files) in group_by_parent(&settled) {
            // The folder may already be gone — the file could have been moved
            // out, or the whole folder deleted, between the event and now.
            if !folder.is_dir() {
                continue;
            }

            let Some(root) = watched_root_for(&folder, &self.roots) else {
                // Outside every watched folder. There is no tree to organise
                // it into, so leaving it alone is the only honest answer.
                continue;
            };

            // Only this folder, never its subfolders: an event in one album
            // folder must not drag the entire library through a fresh scan.
            let retry = match organise_directory(&self.ctx, &folder, root, false) {
                // `ERROR` here almost always means the write lock is held by a
                // `meedya scan --execute` somebody started by hand. That is a
                // temporary condition, not a bad file, so the work goes back
                // in the queue and is tried again after another settle window.
                Ok(code) => code == ExitCode::ERROR,
                Err(err) => {
                    // A hard failure — a broken template, say. Retrying would
                    // just reprint the same message every few seconds, so the
                    // files are dropped and the reason is stated once.
                    output::print_error(&format!(
                        "Could not organise '{}': {err}",
                        folder.display()
                    ));
                    false
                }
            };

            if retry {
                let now = Instant::now();
                for file in files {
                    pending.insert(file, now);
                }
            }
        }
    }
}

// ─── Event printing ─────────────────────────────────────────────────────────

/// Turn one event into the label and path text used by both output formats.
fn describe_event(event: &mm_core::watcher::WatchEvent) -> (&'static str, String) {
    use mm_core::watcher::WatchEvent;
    match event {
        WatchEvent::Created(p) => ("Created", p.display().to_string()),
        WatchEvent::Modified(p) => ("Modified", p.display().to_string()),
        WatchEvent::Deleted(p) => ("Deleted", p.display().to_string()),
        WatchEvent::Renamed(from, to) => {
            ("Renamed", format!("{} → {}", from.display(), to.display()))
        }
    }
}

/// Print one event in whichever format was asked for.
fn print_event(output_format: OutputFormat, event: &mm_core::watcher::WatchEvent) {
    let timestamp = Local::now().format("%H:%M:%S").to_string();
    let (event_type, path) = describe_event(event);

    match output_format {
        OutputFormat::Json => {
            let event_out = WatchEventOutput {
                timestamp,
                event_type: event_type.to_string(),
                path,
            };
            // Print each event as a JSON line
            if let Ok(json) = serde_json::to_string(&event_out) {
                println!("{json}");
            }
        }
        OutputFormat::Human => {
            use colored::Colorize;
            let coloured_type = match event_type {
                "Created" => event_type.green().to_string(),
                "Modified" => event_type.yellow().to_string(),
                "Deleted" => event_type.red().to_string(),
                "Renamed" => event_type.cyan().to_string(),
                _ => event_type.to_string(),
            };
            println!("[{timestamp}] {coloured_type:>10}  {path}");
        }
    }
}

/// Whether `watch --organize` may move files for real, and whether
/// `service install` may register the background service that runs it.
///
/// **Switched off** by owner decision on 2026-09-23 (#180), until the review's
/// data-loss findings are fixed and reviewed — see `.claude/HANDOFF.md` §0 and
/// §15. A `--dry-run` preview works either way, because it moves nothing.
/// Stage (g) of the fix plan sets this back to `true`.
pub(crate) const ORGANISING_SWITCHED_ON: bool = false;

/// What a refusal looks like to a script that asked for `--json`.
#[derive(Serialize)]
struct SwitchedOffOutput {
    /// Always `"switched_off"`, so a script can tell this apart from an
    /// ordinary error without parsing the message.
    status: &'static str,
    /// The same plain-English explanation a person would see.
    message: &'static str,
}

/// Print the "switched off in this build" refusal, as JSON when `--json` was
/// asked for and as an error message otherwise. Shared with `service install`
/// so the two refusals always look alike.
pub(crate) fn print_switched_off(format: OutputFormat, message: &'static str) {
    match format {
        OutputFormat::Json => output::print_json(&SwitchedOffOutput {
            status: "switched_off",
            message,
        }),
        OutputFormat::Human => output::print_error(message),
    }
}

// ─── Event loop ─────────────────────────────────────────────────────────────

/// Log events, and organise them once they have settled, until the watcher is
/// dropped and the channel closes.
///
/// This runs on a blocking thread because everything it calls is blocking:
/// the channel receive, the file walk, the tag reads and the moves themselves.
fn event_loop(
    rx: Receiver<mm_core::watcher::WatchEvent>,
    output_format: OutputFormat,
    organiser: Option<Organiser>,
) {
    // Catch up on anything that arrived while the watcher was not running,
    // before listening for anything new.
    if let Some(organiser) = organiser.as_ref() {
        organiser.sweep_roots();
    }

    let mut pending: Pending = Pending::new();
    // When the settle check last ran. A busy folder can deliver events faster
    // than `EVENT_POLL`, and without this the loop would never stop receiving
    // long enough to notice that something had settled.
    let mut last_settle_check = Instant::now();

    loop {
        // `recv_timeout` rather than `recv`: a plain receive blocks until the
        // next event, and a folder that goes quiet for an hour would leave the
        // last file that arrived unorganised for that whole hour.
        match rx.recv_timeout(EVENT_POLL) {
            Ok(event) => {
                print_event(output_format, &event);
                if organiser.is_some() {
                    record_event(&mut pending, &event, Instant::now());
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            // The watcher was dropped — `run` is shutting us down.
            Err(RecvTimeoutError::Disconnected) => break,
        }

        if let Some(organiser) = organiser.as_ref() {
            if last_settle_check.elapsed() >= EVENT_POLL {
                last_settle_check = Instant::now();
                organiser.organise_settled(&mut pending);
            }
        }
    }
}

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya watch` command.
///
/// Starts a file system watcher and runs until Ctrl+C is pressed. Events are
/// printed to stdout with timestamps.
///
/// With `--organize`, each watched folder is organised once at start-up and
/// then again whenever a file in it settles. The global `--dry-run` flag turns
/// the whole thing into a preview that moves nothing.
pub async fn run(ctx: &CliContext, args: &WatchArgs) -> anyhow::Result<i32> {
    // Determine which folders to watch
    let folders = if args.paths.is_empty() {
        // Use folders from config
        ctx.config.watch.folders.clone()
    } else {
        args.paths.clone()
    };

    // Validate that we have folders to watch
    if folders.is_empty() {
        output::print_error("No watch folders specified. Use arguments or configure in settings.");
        return Ok(ExitCode::ERROR);
    }

    // Verify all folders exist
    for folder in &folders {
        if !folder.is_dir() {
            output::print_error(&format!("Not a directory: {}", folder.display()));
            return Ok(ExitCode::ERROR);
        }
    }

    // ── Set up organising, if it was asked for ──────────────────────────
    let organiser = if args.organize {
        // ── Safety catch: organising for real is switched off (#180) ────
        //
        // The owner decided on 2026-09-23 to switch real organising off
        // until the review's known data-loss findings are fixed and reviewed
        // (`.claude/HANDOFF.md` §0 and §15). Two of them can damage a
        // library: a `.cue` can be moved away from a `.bin` that is still
        // downloading, and a `<Filename>` template can rename the same file
        // again and again. A `--dry-run` preview moves nothing, so it is
        // still allowed. `--yes` does not get past this — it is exactly what
        // an installed background service passes.
        //
        // Stage (g) of the fix plan turns `ORGANISING_SWITCHED_ON` back on.
        if !ORGANISING_SWITCHED_ON && !ctx.dry_run {
            print_switched_off(
                ctx.output,
                "Automatic organising is switched off in this build while known problems \
                 are fixed — it could separate a disc image from its cue sheet, or rename \
                 the same file over and over. Nothing has been moved.\n\n\
                 You can still preview what it would do:\n\
                 \x20   meedya --dry-run watch --organize <folder>\n\
                 or organise a folder yourself, checking the preview first:\n\
                 \x20   meedya scan <folder>",
            );
            return Ok(ExitCode::NOT_IMPLEMENTED);
        }

        // A clone, so the forced conflict strategy below belongs to this watch
        // run and to nothing else.
        let mut organise_ctx = ctx.clone();

        if let Some(previous) = force_skip_conflicts(&mut organise_ctx.config) {
            output::print_warning(&format!(
                "conflict_strategy \"{previous}\" is not used while watching — a file that is \
                 already at its destination would be renamed again on every pass, so the \
                 watcher always skips conflicts instead."
            ));
        }

        // Ask once, at the start, on an attended terminal. A service, a script
        // or a pipe has nobody to answer, and `--dry-run` moves nothing, so
        // neither is prompted.
        if !ctx.dry_run
            && !scan::execute_pre_confirmed(args.yes, std::io::stdin().is_terminal())
            && !scan::prompt_confirm(
                "About to watch these folders and move files into place as they arrive. \
                 Continue?",
            )
        {
            output::print_warning(
                "Watching cancelled — pass --yes to skip this confirmation next time.",
            );
            return Ok(ExitCode::ERROR);
        }

        Some(Organiser {
            ctx: organise_ctx,
            roots: folders.clone(),
            recursive: !args.no_recursive,
            settle: Duration::from_secs(args.settle_secs),
        })
    } else {
        None
    };

    // Build watcher config
    let watcher_config = mm_core::watcher::WatcherConfig {
        folders: folders.clone(),
        recursive: !args.no_recursive,
        debounce_ms: ctx.config.watch.debounce_ms,
        include_extensions: ctx.config.watch.include_extensions.clone(),
        exclude_extensions: ctx.config.watch.exclude_extensions.clone(),
        ignore_patterns: Vec::new(),
    };

    // Print startup info
    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&WatchStartOutput {
                folders: folders.iter().map(|f| f.display().to_string()).collect(),
                recursive: !args.no_recursive,
                organize: args.organize,
                settle_secs: args.settle_secs,
            });
        }
        OutputFormat::Human => {
            output::print_header("MeedyaManager — File Watcher");
            for folder in &folders {
                output::print_key_value("Watching", &folder.display().to_string());
            }
            output::print_key_value("Recursive", &(!args.no_recursive).to_string());
            output::print_key_value("Auto-organise", &args.organize.to_string());
            if args.organize {
                output::print_key_value("Settle window", &format!("{} seconds", args.settle_secs));
                if ctx.dry_run {
                    output::print_warning("Dry-run mode — nothing will actually be moved.");
                }
            }
            println!();
            output::print_success("Watcher started — press Ctrl+C to stop");
            println!();
        }
    }

    // Start the file system watcher.
    //
    // Deliberately started *before* the start-up sweep runs inside the loop
    // below: anything that lands while the sweep is still working is then
    // caught as an ordinary event rather than being missed entirely.
    let (watcher, rx) = mm_core::watcher::start_watcher(&watcher_config)?;

    // Bridge the std::sync::mpsc receiver into the async world
    let output_format = ctx.output;
    let event_handle =
        tokio::task::spawn_blocking(move || event_loop(rx, output_format, organiser));

    // Wait for Ctrl+C signal
    tokio::signal::ctrl_c().await?;

    if ctx.output == OutputFormat::Human {
        println!();
        output::print_success("Watcher stopped");
    }

    // The watcher is dropped here, which closes the channel and
    // allows the event_handle task to finish
    drop(watcher);
    let _ = event_handle.await;

    Ok(ExitCode::SUCCESS)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;
    use crate::test_support::{ConfigDirGuard, write_tagged_wav};

    fn test_ctx() -> CliContext {
        CliContext {
            config: mm_core::config::AppConfig::default(),
            output: OutputFormat::Human,
            verbosity: 0,
            dry_run: false,
        }
    }

    /// Watch returns error when no folders specified
    #[tokio::test]
    async fn watch_no_folders() {
        let mut ctx = test_ctx();
        ctx.config.watch.folders.clear();
        let args = WatchArgs {
            paths: vec![],
            no_recursive: false,
            organize: false,
            yes: true,
            settle_secs: 2,
        };
        assert_eq!(run(&ctx, &args).await.unwrap(), ExitCode::ERROR);
    }

    /// Watch returns error for nonexistent directory
    #[tokio::test]
    async fn watch_nonexistent_dir() {
        let ctx = test_ctx();
        let args = WatchArgs {
            paths: vec![PathBuf::from("/nonexistent/directory")],
            no_recursive: false,
            organize: false,
            yes: true,
            settle_secs: 2,
        };
        assert_eq!(run(&ctx, &args).await.unwrap(), ExitCode::ERROR);
    }

    /// `--organize` with nothing to watch is an ordinary "you have not told me
    /// where to look" error, exactly like a plain `watch` with no folders.
    ///
    /// **This test was written before the feature and it failed, as intended.**
    /// What the first run printed, verbatim:
    ///
    /// ```text
    /// ✗ File auto-organising is not yet implemented. The `--organize` flag
    ///   cannot be used in this release.
    ///
    /// thread 'commands::watch::tests::watch_organize_with_no_folders_is_a_plain_error'
    ///   panicked at crates/mm-cli/src/commands/watch.rs:265:9:
    /// assertion `left == right` failed
    ///   left: 3
    ///  right: 1
    ///
    /// test result: FAILED. 0 passed; 1 failed
    /// ```
    ///
    /// Exit code 3 is `NOT_IMPLEMENTED`. It was returned before the folder
    /// list was even looked at, which is the behaviour this stage removes.
    #[tokio::test]
    async fn watch_organize_with_no_folders_is_a_plain_error() {
        let mut ctx = test_ctx();
        ctx.config.watch.folders.clear();
        let args = WatchArgs {
            paths: vec![],
            no_recursive: false,
            organize: true,
            yes: true,
            settle_secs: 2,
        };
        assert_eq!(run(&ctx, &args).await.unwrap(), ExitCode::ERROR);
    }

    /// **Safety catch (#180, owner decision 2026-09-23).** Until the
    /// organiser's known data-loss findings are fixed and reviewed, a real
    /// `--organize` run must refuse with exit code 3 (`NOT_IMPLEMENTED`) and
    /// move nothing — even with `--yes`, which is what an installed service
    /// passes. Only a `--dry-run` preview is allowed through.
    #[tokio::test]
    async fn watch_organize_without_dry_run_refuses_and_moves_nothing() {
        let _config = ConfigDirGuard::new();

        let tmp = tempfile::tempdir().unwrap();
        let source = tmp.path().join("incoming").join("track.wav");
        write_tagged_wav(&source, "Portishead", "Dummy", "Roads");

        let ctx = test_ctx();
        let args = WatchArgs {
            paths: vec![tmp.path().to_path_buf()],
            no_recursive: false,
            organize: true,
            yes: true,
            settle_secs: 2,
        };

        // Bounded, because without the safety catch `run` starts the
        // watcher and then waits for Ctrl+C for ever. A timeout turns that
        // into a plain test failure instead of a hung test run.
        let code = tokio::time::timeout(Duration::from_secs(20), run(&ctx, &args))
            .await
            .expect("run should refuse at once, not start watching")
            .unwrap();
        assert_eq!(code, ExitCode::NOT_IMPLEMENTED);
        assert!(
            source.is_file(),
            "the refusal must leave the file exactly where it was"
        );
        assert!(
            !tmp.path().join("Portishead").exists(),
            "the refusal must not create any destination folders"
        );
    }

    /// Watch without --organize still validates folders
    #[tokio::test]
    async fn watch_without_organize_validates_folders() {
        let mut ctx = test_ctx();
        ctx.config.watch.folders.clear();
        let args = WatchArgs {
            paths: vec![],
            no_recursive: false,
            organize: false,
            yes: true,
            settle_secs: 2,
        };
        // Should proceed to folder validation and return ERROR
        assert_eq!(run(&ctx, &args).await.unwrap(), ExitCode::ERROR);
    }

    /// WatchArgs construction
    #[test]
    fn watch_args_construction() {
        let args = WatchArgs {
            paths: vec![PathBuf::from("/music"), PathBuf::from("/videos")],
            no_recursive: true,
            organize: true,
            yes: true,
            settle_secs: 5,
        };
        assert_eq!(args.paths.len(), 2);
        assert!(args.no_recursive);
        assert!(args.organize);
        assert!(args.yes);
        assert_eq!(args.settle_secs, 5);
    }

    /// WatchEventOutput serializes correctly
    #[test]
    fn watch_event_json_serialization() {
        let event = WatchEventOutput {
            timestamp: "12:34:56".to_string(),
            event_type: "Created".to_string(),
            path: "/music/song.mp3".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("Created"));
        assert!(json.contains("song.mp3"));
    }

    // ── Organising ──────────────────────────────────────────────────────
    //
    // There is deliberately **no test that starts a real watcher and waits
    // for an event**. `notify` delivers events on the operating system's own
    // schedule — coalesced, delayed, and different on macOS, Linux and
    // Windows — so any such test would pass or fail depending on how busy the
    // machine happened to be that minute. Everything below either calls the
    // organiser directly or tests the pure decision-making around it.

    /// A tagged file dropped into a subfolder of a watched root is laid out
    /// under the **root**, not under the subfolder it arrived in. That is the
    /// whole reason `organise_directory` passes the root as the output
    /// directory rather than letting `scan` rename in place.
    #[test]
    fn organise_directory_moves_a_tagged_file_under_the_root() {
        // Moving files for real takes the write lock, which lives in the
        // configuration directory — point that somewhere private.
        let _config = ConfigDirGuard::new();

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let source = root.join("incoming").join("track.wav");
        write_tagged_wav(&source, "Portishead", "Dummy", "Roads");

        let ctx = test_ctx(); // default template: <Artist>/<Album>/<Title>

        organise_directory(&ctx, &root.join("incoming"), root, false).unwrap();

        let expected = root.join("Portishead").join("Dummy").join("Roads.wav");
        assert!(
            expected.is_file(),
            "the file should have been organised to {}",
            expected.display()
        );
        assert!(
            !source.exists(),
            "the file should have moved, not been copied — {} is still there",
            source.display()
        );
    }

    /// The global `--dry-run` flag has to reach all the way through to the
    /// watcher, or a preview would quietly move files.
    #[test]
    fn organise_directory_dry_run_moves_nothing() {
        let _config = ConfigDirGuard::new();

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let source = root.join("incoming").join("track.wav");
        write_tagged_wav(&source, "Portishead", "Dummy", "Roads");

        let mut ctx = test_ctx();
        ctx.dry_run = true;

        organise_directory(&ctx, &root.join("incoming"), root, false).unwrap();

        assert!(
            source.is_file(),
            "dry-run must leave the original exactly where it was"
        );
        assert!(
            !root.join("Portishead").exists(),
            "dry-run must not even create the destination folders"
        );
    }

    /// **The most important test in this stage.**
    ///
    /// Issue #219: a `.cue` sheet names its `.bin` image by bare file name, so
    /// the pair only works while both sit in the same folder. `scan --execute`
    /// used to move each one somewhere different and destroy the rip. That is
    /// fixed in `scan`, and because the watcher calls `scan`, the watcher
    /// inherits the fix — but the watcher is the far more dangerous caller,
    /// because it runs unattended, from login until shutdown, with nobody
    /// reading the output. A regression here would eat a library overnight.
    ///
    /// The template is the one from the original bug report: route every file
    /// into a folder named after its extension.
    #[test]
    fn organise_directory_leaves_disc_folders_alone() {
        let _config = ConfigDirGuard::new();

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let rip = root.join("Rip");
        std::fs::create_dir_all(&rip).unwrap();

        // An ordinary single-file BIN/CUE rip, plus the ripper's log.
        let cue_bytes = concat!(
            "PERFORMER \"Test Artist\"\n",
            "TITLE \"Test Album\"\n",
            "FILE \"Album.bin\" BINARY\n",
            "  TRACK 01 AUDIO\n",
            "    INDEX 01 00:00:00\n",
        )
        .as_bytes()
        .to_vec();
        // 4 KiB of image payload — the content is irrelevant, being able to
        // compare it byte-for-byte afterwards is the point.
        let bin_bytes = vec![0x5Au8; 4096];

        std::fs::write(rip.join("Album.cue"), &cue_bytes).unwrap();
        std::fs::write(rip.join("Album.bin"), &bin_bytes).unwrap();
        std::fs::write(rip.join("Album.log"), b"Exact Audio Copy log\n").unwrap();

        let mut ctx = test_ctx();
        ctx.config.rename.template = "<Extension>/<Filename>".to_string();

        // Recursive, exactly like the start-up sweep of a watched root.
        organise_directory(&ctx, root, root, true).unwrap();

        let cue_path = rip.join("Album.cue");
        let bin_path = rip.join("Album.bin");

        assert!(
            cue_path.is_file(),
            "the watcher moved Album.cue away from its .bin — the rip is destroyed"
        );
        assert!(
            bin_path.is_file(),
            "the watcher moved Album.bin away from its .cue — the rip is destroyed"
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
        assert!(
            !root.join("cue").exists() && !root.join("bin").exists(),
            "no per-extension folder may be created for a disc image member"
        );
    }

    // ── Forced conflict strategy ────────────────────────────────────────

    /// "rename" is replaced by "skip", and the caller is told which setting
    /// was overridden so it can say so once.
    #[test]
    fn a_rename_conflict_strategy_is_forced_to_skip_and_reported() {
        let mut config = mm_core::config::AppConfig::default();
        config.rename.conflict_strategy = "rename".to_string();

        let previous = force_skip_conflicts(&mut config);

        assert_eq!(previous.as_deref(), Some("rename"));
        assert_eq!(config.rename.conflict_strategy, "skip");
    }

    /// A config that already says "skip" produces no warning — there is
    /// nothing for anybody to act on.
    #[test]
    fn an_already_skipping_config_is_not_reported() {
        let mut config = mm_core::config::AppConfig::default();
        config.rename.conflict_strategy = "  SKIP  ".to_string();

        assert_eq!(force_skip_conflicts(&mut config), None);
        assert_eq!(config.rename.conflict_strategy, "skip");
    }

    // ── Settle logic ────────────────────────────────────────────────────

    /// Only files that have been quiet for the full window come out, and the
    /// ones that come out are removed from the waiting list.
    #[test]
    fn only_files_quiet_for_the_whole_window_are_taken() {
        let settle = Duration::from_secs(2);
        let now = Instant::now();

        // `checked_sub` rather than plain `-`: on a machine that has only just
        // booted, "now" can be less than three seconds after the clock started,
        // and subtracting past the start of the clock would panic. `unwrap` is
        // still right here — if it ever did fail, the fixture would be
        // meaningless and the test should say so loudly.
        let ago = |d: Duration| now.checked_sub(d).unwrap();

        let mut pending = Pending::new();
        // Touched three seconds ago — finished.
        pending.insert(
            PathBuf::from("/media/quiet.wav"),
            ago(Duration::from_secs(3)),
        );
        // Touched exactly on the boundary — the window has passed.
        pending.insert(PathBuf::from("/media/exact.wav"), ago(settle));
        // Touched half a second ago — still arriving.
        pending.insert(
            PathBuf::from("/media/busy.wav"),
            ago(Duration::from_millis(500)),
        );

        let settled = take_settled(&mut pending, now, settle);

        assert_eq!(
            settled,
            vec![
                PathBuf::from("/media/exact.wav"),
                PathBuf::from("/media/quiet.wav"),
            ],
            "the settled list must be sorted and must exclude the busy file"
        );
        assert_eq!(
            pending.keys().collect::<Vec<_>>(),
            vec![&PathBuf::from("/media/busy.wav")],
            "a file that has been taken must no longer be waiting"
        );
    }

    /// Nothing settled means nothing taken and nothing lost.
    #[test]
    fn nothing_settles_before_the_window_passes() {
        let now = Instant::now();
        let mut pending = Pending::new();
        pending.insert(PathBuf::from("/media/new.wav"), now);

        assert!(take_settled(&mut pending, now, Duration::from_secs(2)).is_empty());
        assert_eq!(pending.len(), 1);
    }

    /// Creations and modifications start the clock; a deletion takes the file
    /// out entirely; a rename moves the clock from the old name to the new.
    #[test]
    fn events_start_stop_and_move_the_settle_clock() {
        use mm_core::watcher::WatchEvent;
        let now = Instant::now();
        let mut pending = Pending::new();

        record_event(
            &mut pending,
            &WatchEvent::Created(PathBuf::from("/media/a.wav")),
            now,
        );
        record_event(
            &mut pending,
            &WatchEvent::Modified(PathBuf::from("/media/b.wav")),
            now,
        );
        assert_eq!(pending.len(), 2);

        record_event(
            &mut pending,
            &WatchEvent::Deleted(PathBuf::from("/media/a.wav")),
            now,
        );
        assert!(!pending.contains_key(Path::new("/media/a.wav")));

        record_event(
            &mut pending,
            &WatchEvent::Renamed(PathBuf::from("/media/b.wav"), PathBuf::from("/media/c.wav")),
            now,
        );
        assert!(!pending.contains_key(Path::new("/media/b.wav")));
        assert!(pending.contains_key(Path::new("/media/c.wav")));
    }

    // ── Grouping and root matching ──────────────────────────────────────

    /// Files are organised a folder at a time, so ten files in one folder must
    /// collapse to one entry.
    #[test]
    fn files_are_grouped_by_the_folder_they_sit_in() {
        let files = vec![
            PathBuf::from("/media/Album/one.wav"),
            PathBuf::from("/media/Album/two.wav"),
            PathBuf::from("/media/Other/three.wav"),
        ];

        let grouped = group_by_parent(&files);

        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped[Path::new("/media/Album")].len(), 2);
        assert_eq!(grouped[Path::new("/media/Other")].len(), 1);
    }

    /// When watched folders are nested, the deepest one that contains the file
    /// is the one it belongs to — regardless of the order they were listed in.
    #[test]
    fn the_deepest_matching_watched_folder_wins() {
        let roots = vec![
            PathBuf::from("/media"),
            PathBuf::from("/media/Incoming/Singles"),
            PathBuf::from("/media/Incoming"),
        ];

        assert_eq!(
            watched_root_for(Path::new("/media/Incoming/Singles/a.wav"), &roots),
            Some(Path::new("/media/Incoming/Singles"))
        );
        assert_eq!(
            watched_root_for(Path::new("/media/Incoming/b.wav"), &roots),
            Some(Path::new("/media/Incoming"))
        );
        assert_eq!(
            watched_root_for(Path::new("/media/c.wav"), &roots),
            Some(Path::new("/media"))
        );
    }

    /// A path under no watched folder has no root, and the caller leaves it
    /// alone rather than guessing where it should go.
    #[test]
    fn a_path_outside_every_watched_folder_has_no_root() {
        let roots = vec![PathBuf::from("/media")];
        assert_eq!(
            watched_root_for(Path::new("/elsewhere/a.wav"), &roots),
            None
        );
        // A folder that merely *starts with the same letters* is not a match:
        // "/mediocre" is not inside "/media".
        assert_eq!(watched_root_for(Path::new("/mediocre/a.wav"), &roots), None);
    }
}
