// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya edit` Command
//
// Metadata editor: set/remove tags, embed/remove cover art on media files.
// Supports `--dry-run` to preview changes without modifying files.

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::Args;
use serde::Serialize;
use std::path::PathBuf;

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya edit` command.
#[derive(Args, Debug)]
pub struct EditArgs {
    /// Path to the media file to edit
    pub path: PathBuf,

    /// Set a metadata tag (format: key=value, can be repeated)
    #[arg(long, value_name = "KEY=VALUE")]
    pub set: Vec<String>,

    /// Remove a metadata tag by key (can be repeated)
    #[arg(long, value_name = "KEY")]
    pub remove: Vec<String>,

    /// Embed cover art from an image file
    #[arg(long, value_name = "IMAGE_PATH")]
    pub cover: Option<PathBuf>,

    /// Remove all embedded cover art
    #[arg(long)]
    pub remove_cover: bool,

    /// Show proposed changes without modifying the file
    #[arg(long)]
    pub dry_run: bool,
}

// ─── JSON output structures ─────────────────────────────────────────────────

/// Edit result for JSON output.
#[derive(Serialize)]
struct EditOutput {
    file: String,
    actions: Vec<EditAction>,
    dry_run: bool,
    /// Path the edits actually landed on when Test Mode diverted them to a
    /// `_MeedyaManager` copy.  Omitted entirely outside Test Mode so existing
    /// JSON consumers see an unchanged document shape.
    #[serde(skip_serializing_if = "Option::is_none")]
    written_to: Option<String>,
}

/// Individual edit action for JSON output.
#[derive(Debug, Serialize)]
struct EditAction {
    action: String,
    key: Option<String>,
    value: Option<String>,
    success: bool,
    error: Option<String>,
    /// Set only for a successful `--set language=...` that a person needs
    /// to be told something about: its stored form differs from what was
    /// typed (item 6 of the language-policy review — e.g. an ID3 tag can
    /// only hold the three-letter code, so a region can be silently lost
    /// unless something says so), or the file's tags disagree about the
    /// language and one of them will be left as it is (third review round,
    /// item 3). Omitted entirely from JSON when there is nothing to say, so
    /// existing consumers see an unchanged shape for every other action.
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

impl EditAction {
    /// A successful action with nothing extra to say about it.
    fn ok(action: &str, key: Option<String>, value: Option<String>) -> Self {
        Self {
            action: action.to_string(),
            key,
            value,
            success: true,
            error: None,
            note: None,
        }
    }

    /// A successful action, with a plain-English note attached — used only
    /// for `language`, when what will actually be stored differs from what
    /// was typed.
    fn ok_with_note(
        action: &str,
        key: Option<String>,
        value: Option<String>,
        note: String,
    ) -> Self {
        Self {
            action: action.to_string(),
            key,
            value,
            success: true,
            error: None,
            note: Some(note),
        }
    }

    /// A failed action carrying a user-facing reason.
    fn failed(action: &str, key: Option<String>, value: Option<String>, error: String) -> Self {
        Self {
            action: action.to_string(),
            key,
            value,
            success: false,
            error: Some(error),
            note: None,
        }
    }
}

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya edit` command.
///
/// ## Why this is a two-phase command
///
/// Phase 1 validates *every* requested operation without touching the disk;
/// phase 2 performs the writes, and only runs if phase 1 found nothing wrong.
///
/// The split is what makes an edit batch atomic.  Previously each `--set` was
/// written the moment it was parsed, so `--set title=X --set bogus=1` left the
/// title applied and then reported a partial failure — the user had to work
/// out which half had landed.  Validating first means the file is either fully
/// updated or not opened at all (issue #206).
///
/// All writes go through `mm_core::integrity`, never `mm_core::metadata`
/// directly, because the integrity layer is the only place Test Mode is
/// enforced (issue #128).
pub fn run(ctx: &CliContext, args: &EditArgs) -> anyhow::Result<i32> {
    // Verify the file exists
    if !args.path.exists() {
        output::print_error(&format!("File not found: {}", args.path.display()));
        return Ok(ExitCode::ERROR);
    }

    // Check that at least one edit operation was requested
    if args.set.is_empty() && args.remove.is_empty() && args.cover.is_none() && !args.remove_cover {
        output::print_warning(
            "No edit operations specified. Use --set, --remove, --cover, or --remove-cover.",
        );
        return Ok(ExitCode::ERROR);
    }

    // Determine effective dry-run state
    let dry_run = ctx.dry_run || args.dry_run;

    // ── Phase 1: validate everything before any I/O ─────────────────────
    let plan = match build_plan(args) {
        Ok(plan) => plan,
        // At least one operation is invalid.  Report the failures and stop —
        // deliberately performing none of the *valid* operations either, so
        // the user never has to guess how far the batch got.
        Err(actions) => {
            render(ctx, args, actions, dry_run, None);
            return Ok(ExitCode::PARTIAL);
        }
    };

    // ── Phase 2: apply (or, in dry-run, just describe) ──────────────────
    let (actions, written_to) = if dry_run {
        (describe_plan(&plan), None)
    } else {
        apply_plan(args, &plan)
    };

    let any_failed = actions.iter().any(|a| !a.success);
    render(ctx, args, actions, dry_run, written_to);

    Ok(if any_failed {
        ExitCode::PARTIAL
    } else {
        ExitCode::SUCCESS
    })
}

// ─── Phase 1: planning & validation ────────────────────────────────────────

/// A fully validated edit batch: every key is known, every input file exists.
struct EditPlan {
    /// All `--set` pairs merged into ONE `TagMap`.
    ///
    /// One map means one `write_tags_safe` call, hence one integrity cycle and
    /// — in Test Mode — one copy, instead of N sequential rewrites of the file.
    tags: mm_core::metadata::TagMap,
    /// The `--set` pairs again, in argument order, purely so the output can
    /// report one line per requested operation.
    set_pairs: Vec<(String, String)>,
    /// Validated `--remove` keys.
    remove_keys: Vec<String>,
    /// Validated `--cover` image: (source path, raw bytes, MIME type).
    cover: Option<(PathBuf, Vec<u8>, &'static str)>,
    /// Whether `--remove-cover` was requested.
    remove_cover: bool,
    /// Item 6 of the language-policy review: when a `--set language=...`
    /// value would be stored differently from what was typed (TRACK-070's
    /// per-format conversion can lose a region, say), a plain-English
    /// explanation of what will actually be stored and why — or, since the
    /// third review round (item 3), when the value is already the file's
    /// language but one of the file's tags disagrees with it and will be
    /// left alone. `None` when there is nothing to say — no
    /// `--set language=...`, a clearing `--set language=`, or nothing worth
    /// saying. Computed once in `build_plan` (Phase 1, before any write)
    /// so it shows up on `--dry-run` too, which never calls `write_tags`.
    language_note: Option<String>,
}

/// Validate every requested operation.
///
/// Returns `Ok(plan)` when the whole batch is sound, or `Err(actions)` holding
/// one failed `EditAction` per problem — the caller renders those and performs
/// no writes at all.
fn build_plan(args: &EditArgs) -> Result<EditPlan, Vec<EditAction>> {
    let mut failures: Vec<EditAction> = Vec::new();

    // The set of keys the metadata layer can actually persist.  Derived from
    // the lofty ItemKey mapping, so it cannot drift from what a write accepts.
    let known = mm_core::metadata::known_tag_keys();
    let valid_list = known.join(", ");

    // -- --set: parse `key=value`, then check the key ----------------------
    let mut tags = mm_core::metadata::TagMap::new();
    let mut set_pairs: Vec<(String, String)> = Vec::new();
    let mut language_note: Option<String> = None;

    // Codex's catch-up review of the language-policy branch, finding 3: the
    // same field set twice in one command is refused, before anything is
    // written. It used to be "the last one wins" — but every `--set` was
    // still reported as done, each with the LAST value's note. Reproduced
    // with the binary built from `a150926`: `--set language=en --set
    // language=pt-BR` on an MP3 printed two successful rows, both saying the
    // region would be lost and "por" stored, the file stored `por`, and `en`
    // was never written at all; `--dry-run` said the same. Refusing is the
    // only answer that cannot mislead: which value a person meant is not
    // something this command can know.
    //
    // The stand-in review of round 6 (M3) found the same misleading report
    // for a field given to both `--set` and `--remove`, or to `--remove`
    // twice: reproduced with the binary built from `49cec29`, `--set
    // language=pt-BR --remove language` on an MP3 printed two successful
    // rows — the first saying "por" would be stored — and the file was left
    // with no tag at all; `--set title=X --remove title` and `--remove title
    // --remove title` each reported every row as done. So every time a field
    // is named, by either flag, counts. Each is collected first, in order, as
    // it was typed (with any invisible character written out), so the message
    // can name them all.
    let mut given_by_key: Vec<(&str, Vec<String>)> = Vec::new();
    let named = args
        .set
        .iter()
        .filter_map(|set_arg| {
            set_arg.split_once('=').map(|(key, value)| {
                let shown = mm_core::metadata::language::show_invisible_characters(value);
                (key, format!("--set {key}={shown}"))
            })
        })
        .chain(
            args.remove
                .iter()
                .map(|key| (key.as_str(), format!("--remove {key}"))),
        );
    for (key, as_typed) in named {
        match given_by_key.iter_mut().find(|(k, _)| *k == key) {
            Some((_, given)) => given.push(as_typed),
            None => given_by_key.push((key, vec![as_typed])),
        }
    }
    // The refusal for a field named more than once, or `None`.
    let given_more_than_once = |key: &str| -> Option<String> {
        given_by_key
            .iter()
            .find(|(k, given)| *k == key && given.len() > 1)
            .map(|(_, given)| {
                format!(
                    "'{key}' is given more than once in this command ({}) — give each field \
                     once, so it is clear what to save",
                    given.join(", then ")
                )
            })
    };

    for set_arg in &args.set {
        let Some((key, value)) = set_arg.split_once('=') else {
            failures.push(EditAction::failed(
                "set",
                Some(set_arg.clone()),
                None,
                "Invalid format — expected key=value".to_string(),
            ));
            continue;
        };

        if !known.contains(&key) {
            failures.push(EditAction::failed(
                "set",
                Some(key.to_string()),
                Some(value.to_string()),
                format!("unknown key '{key}' — valid: {valid_list}"),
            ));
            continue;
        }

        // Finding 3 and M3 (see above): one failed row for each time the
        // field was given, so every `--set` and `--remove` the person typed
        // is answered, and none of them is reported as done.
        if let Some(refusal) = given_more_than_once(key) {
            failures.push(EditAction::failed(
                "set",
                Some(key.to_string()),
                Some(value.to_string()),
                refusal,
            ));
            continue;
        }

        // `language` gets one extra check here, on top of the generic
        // unknown-key check above: policy MWBM-MEDIA-LANG 1.0.0 says a
        // person setting a language on purpose is refused with a plain
        // message when the value is not recognised, rather than being
        // silently accepted and written as `und`. `write_tags` in mm-core
        // enforces this too (so the FFI and the Linux UI cannot bypass it),
        // but checking it here as well means the failure is reported
        // against this exact `--set` pair, in Phase 1, before any file is
        // opened at all — the same all-or-nothing guarantee an unknown key
        // already gets. A non-empty value is what fails; an empty one
        // (`--set language=`) clears the field, the same as any other key,
        // and has no note. (A clearing `--set language=` after an earlier
        // `--set language=...` used to need its note dropped here; giving
        // a field twice is now refused above, so that cannot happen.)
        if key == mm_core::metadata::TAG_LANGUAGE && !value.is_empty() {
            if let Err(e) = mm_core::metadata::language::parse_language_input(value) {
                failures.push(EditAction::failed(
                    "set",
                    Some(key.to_string()),
                    Some(value.to_string()),
                    e.to_string(),
                ));
                continue;
            }
            // Item 6 of the language-policy review: the value IS a real
            // language (the check above just confirmed that), but the
            // form this file's own tag container can actually hold may
            // not be the exact text typed — an ID3 tag (an MP3's, or one
            // embedded in a WAV) can only keep the three-letter code, so a
            // region such as "-BR" is silently gone unless this says so.
            // Since the third review round (item 3) this also says when
            // the value is already the file's language but one of its tags
            // disagrees and will be left alone — never a silent "✓ Set".
            // Worked out once: a field given twice is refused above.
            //
            // Read from the file the save will really change, which is not
            // always `args.path`: with Test Mode on and a copy already made
            // by an earlier edit, the save goes to that copy, so the note
            // must describe the copy (fourth review round, item S1 — it
            // used to read the untouched original, and could say a tag
            // would be "left alone" that the save then rewrote).
            let save_starts_from = mm_core::integrity::where_a_save_starts(&args.path);
            language_note =
                mm_core::metadata::language::preview_conversion_note(&save_starts_from, value);
        }

        // Each field reaches here at most once: giving one twice is refused
        // above (finding 3), so nothing is overwritten.
        tags.insert(key.to_string(), vec![value.to_string()]);
        set_pairs.push((key.to_string(), value.to_string()));
    }

    // -- --remove: check the key -------------------------------------------
    let mut remove_keys: Vec<String> = Vec::new();
    for key in &args.remove {
        if !known.contains(&key.as_str()) {
            failures.push(EditAction::failed(
                "remove",
                Some(key.clone()),
                None,
                format!("unknown key '{key}' — valid: {valid_list}"),
            ));
        } else if let Some(refusal) = given_more_than_once(key) {
            // M3 (see above).
            failures.push(EditAction::failed(
                "remove",
                Some(key.clone()),
                None,
                refusal,
            ));
        } else {
            remove_keys.push(key.clone());
        }
    }

    // -- --cover: the image must exist and be readable ---------------------
    let mut cover = None;
    if let Some(cover_path) = args.cover.as_ref() {
        if cover_path.exists() {
            match std::fs::read(cover_path) {
                Ok(data) => {
                    // Guess MIME type from extension
                    let mime = match cover_path.extension().and_then(|e| e.to_str()) {
                        Some("png") => "image/png",
                        Some("gif") => "image/gif",
                        Some("webp") => "image/webp",
                        // Default fallback, and the explicit jpg/jpeg case
                        _ => "image/jpeg",
                    };
                    cover = Some((cover_path.clone(), data, mime));
                }
                Err(e) => failures.push(EditAction::failed(
                    "embed_cover",
                    None,
                    Some(cover_path.display().to_string()),
                    format!("Cannot read image file: {e}"),
                )),
            }
        } else {
            failures.push(EditAction::failed(
                "embed_cover",
                None,
                Some(cover_path.display().to_string()),
                "Image file not found".to_string(),
            ));
        }
    }

    // -- The checks a save makes before writing anything, run now ---------
    //
    // The stand-in review of round 6, M2: a save can refuse from inside —
    // a WAV whose RIFF INFO list it would damage — and `--dry-run` never
    // saves, so it said "✓" where the real run refused. Reproduced with the
    // binary built from `49cec29` on a WAV with a Latin-1 title: `--set
    // language=en --dry-run` exited 0, the real run exited 2. So the same
    // check runs here, read-only, for a dry run and a real run alike, and a
    // refusal is answered exactly as the save would answer it. Only once
    // everything else is valid: an unknown key is answered on its own.
    //
    // What this cannot do: the saves of a real run happen one after
    // another (the `--set` batch, then each `--remove`), and each check
    // here reads the file as it is now, before any of them. A save only ever
    // writes entries for the fields it was asked to set (as UTF-8 text the
    // tag library can read back) or removes entries, and keeps every other
    // entry byte for byte, so what one save leaves cannot make a later one
    // lose an entry this check passed — and each save still runs its own
    // check as well.
    if failures.is_empty() {
        if !tags.is_empty()
            && let Err(reason) = mm_core::integrity::check_save(&args.path, |target| {
                mm_core::metadata::check_tag_write(target, &tags)
            })
        {
            for (key, value) in &set_pairs {
                failures.push(EditAction::failed(
                    "set",
                    Some(key.clone()),
                    Some(value.clone()),
                    reason.clone(),
                ));
            }
        }
        for key in &remove_keys {
            if let Err(reason) = mm_core::integrity::check_save(&args.path, |target| {
                mm_core::metadata::check_tag_removal(target, key)
            }) {
                failures.push(EditAction::failed(
                    "remove",
                    Some(key.clone()),
                    None,
                    reason,
                ));
            }
        }
    }

    if failures.is_empty() {
        Ok(EditPlan {
            tags,
            set_pairs,
            remove_keys,
            cover,
            remove_cover: args.remove_cover,
            language_note,
        })
    } else {
        Err(failures)
    }
}

/// Describe a validated plan without performing it — the `--dry-run` path.
/// Build the `EditAction` for one SUCCESSFUL `--set key=value` pair,
/// attaching `plan.language_note` (item 6 of the language-policy review)
/// when `key` is `language` and there is something to say. Shared between
/// `describe_plan` (`--dry-run`, which never calls `write_tags` at all —
/// this is the only place that note would ever be shown) and `apply_plan`
/// (a real write), so the two paths can never show a different answer for
/// the same batch.
fn set_action_for(key: &str, value: &str, plan: &EditPlan) -> EditAction {
    if key == mm_core::metadata::TAG_LANGUAGE {
        if let Some(note) = &plan.language_note {
            return EditAction::ok_with_note(
                "set",
                Some(key.to_string()),
                Some(value.to_string()),
                note.clone(),
            );
        }
    }
    EditAction::ok("set", Some(key.to_string()), Some(value.to_string()))
}

fn describe_plan(plan: &EditPlan) -> Vec<EditAction> {
    let mut actions = Vec::new();

    for (key, value) in &plan.set_pairs {
        actions.push(set_action_for(key, value, plan));
    }
    for key in &plan.remove_keys {
        actions.push(EditAction::ok("remove", Some(key.clone()), None));
    }
    if let Some((path, _, _)) = &plan.cover {
        actions.push(EditAction::ok(
            "embed_cover",
            None,
            Some(path.display().to_string()),
        ));
    }
    if plan.remove_cover {
        actions.push(EditAction::ok("remove_cover", None, None));
    }

    actions
}

// ─── Phase 2: application ──────────────────────────────────────────────────

/// Perform a validated plan against `args.path`.
///
/// Returns the per-operation actions plus, when Test Mode diverted the writes,
/// the path of the `_MeedyaManager` copy they landed on.
fn apply_plan(args: &EditArgs, plan: &EditPlan) -> (Vec<EditAction>, Option<String>) {
    use mm_core::integrity;

    let mut actions: Vec<EditAction> = Vec::new();
    // Set by the first successful write; every later write in the same batch
    // accumulates onto the same copy, so one value describes the whole run.
    let mut written_to: Option<String> = None;

    /// Fold one `IntegrityWriteResult` into the running `written_to`.
    ///
    /// The integrity layer reports the file it actually wrote.  If that is not
    /// the path we asked it to edit, Test Mode redirected us to a copy.
    fn note_target(
        result: &integrity::IntegrityWriteResult,
        requested: &std::path::Path,
        written_to: &mut Option<String>,
    ) {
        if result.success && result.path != requested {
            *written_to = Some(result.path.display().to_string());
        }
    }

    // -- 1. All --set pairs in ONE guarded write ---------------------------
    if !plan.tags.is_empty() {
        let result = integrity::write_tags_safe(&args.path, &plan.tags);
        note_target(&result, &args.path, &mut written_to);

        for (key, value) in &plan.set_pairs {
            actions.push(if result.success {
                set_action_for(key, value, plan)
            } else {
                EditAction::failed(
                    "set",
                    Some(key.clone()),
                    Some(value.clone()),
                    result
                        .error
                        .clone()
                        .unwrap_or_else(|| "unknown write error".to_string()),
                )
            });
        }
    }

    // -- 2. --remove, one guarded call per key -----------------------------
    for key in &plan.remove_keys {
        let result = integrity::remove_tag_safe(&args.path, key);
        note_target(&result, &args.path, &mut written_to);

        actions.push(if result.success {
            EditAction::ok("remove", Some(key.clone()), None)
        } else {
            EditAction::failed(
                "remove",
                Some(key.clone()),
                None,
                result
                    .error
                    .clone()
                    .unwrap_or_else(|| "unknown write error".to_string()),
            )
        });
    }

    // -- 3. --cover ---------------------------------------------------------
    if let Some((cover_path, data, mime)) = &plan.cover {
        let result = integrity::embed_cover_art_safe(&args.path, data, mime);
        note_target(&result, &args.path, &mut written_to);

        let label = Some(cover_path.display().to_string());
        actions.push(if result.success {
            EditAction::ok("embed_cover", None, label)
        } else {
            EditAction::failed(
                "embed_cover",
                None,
                label,
                result
                    .error
                    .clone()
                    .unwrap_or_else(|| "unknown write error".to_string()),
            )
        });
    }

    // -- 4. --remove-cover --------------------------------------------------
    if plan.remove_cover {
        let result = integrity::remove_cover_art_safe(&args.path);
        note_target(&result, &args.path, &mut written_to);

        actions.push(if result.success {
            EditAction::ok("remove_cover", None, None)
        } else {
            EditAction::failed(
                "remove_cover",
                None,
                None,
                result
                    .error
                    .clone()
                    .unwrap_or_else(|| "unknown write error".to_string()),
            )
        });
    }

    (actions, written_to)
}

// ─── Output rendering ──────────────────────────────────────────────────────

/// One line `render`'s Human branch will print, tagged with which of
/// `output`'s three status styles it belongs to. Kept separate from the
/// actual `println!`/`eprintln!` calls inside those `print_*` functions
/// only so [`build_human_lines`] is a plain, pure function a test can call
/// directly — see that function's own doc comment for why this exists
/// (review item 1 of the second language-policy review round).
#[derive(Debug, PartialEq, Eq)]
enum HumanLine {
    Success(String),
    Warning(String),
    Error(String),
}

/// What each action's own description line reads, before success/failure
/// or a note is folded in.
fn action_description(action: &EditAction) -> String {
    match action.action.as_str() {
        "set" => format!(
            "Set {} = {}",
            action.key.as_deref().unwrap_or("?"),
            action.value.as_deref().unwrap_or("?"),
        ),
        "remove" => format!("Remove {}", action.key.as_deref().unwrap_or("?")),
        "embed_cover" => format!(
            "Embed cover from {}",
            action.value.as_deref().unwrap_or("?"),
        ),
        "remove_cover" => "Remove cover art".to_string(),
        _ => action.action.clone(),
    }
}

/// Every line `render`'s Human branch will print for `actions`, in order —
/// everything except the header and the Test Mode "written to" line, which
/// belong to the whole command rather than to one action.
///
/// Pulled out of `render` itself (review item 1 of the second review
/// round) because the bug that round found — `action.note` (item 6 of the
/// FIRST round: what actually got stored, when it differs from what was
/// typed) reaching `EditOutput` for `--json` callers, and then never once
/// being printed for anyone running `meedya edit` from a terminal without
/// `--json` — lived entirely inside `render`, one step past where the
/// existing tests looked (they checked `EditPlan`/`EditAction` directly,
/// never what `render` actually did with them). A function `render` itself
/// calls, rather than a second copy of the same logic, is what makes a
/// test of this actually prove the real output matches — there is no way
/// for the two to drift apart, because they are the same code.
fn build_human_lines(actions: &[EditAction]) -> Vec<HumanLine> {
    let mut lines = Vec::new();
    for action in actions {
        let desc = action_description(action);
        if action.success {
            lines.push(HumanLine::Success(desc));
            // The note belongs right under the success line it explains,
            // whether or not this is `--dry-run` — the whole point of
            // computing it at Phase-1 validation time is that it is true
            // before anything is written.
            if let Some(note) = &action.note {
                lines.push(HumanLine::Warning(note.clone()));
            }
        } else {
            lines.push(HumanLine::Error(format!(
                "{desc}: {}",
                action.error.as_deref().unwrap_or("unknown error"),
            )));
        }
    }
    lines
}

/// Render the outcome in whichever format the user asked for.
fn render(
    ctx: &CliContext,
    args: &EditArgs,
    actions: Vec<EditAction>,
    dry_run: bool,
    written_to: Option<String>,
) {
    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&EditOutput {
                file: args.path.display().to_string(),
                actions,
                dry_run,
                written_to,
            });
        }
        OutputFormat::Human => {
            if dry_run {
                output::print_header("Dry Run — Proposed Changes");
            } else {
                output::print_header("Edit Results");
            }

            for line in build_human_lines(&actions) {
                match line {
                    HumanLine::Success(s) => output::print_success(&s),
                    HumanLine::Warning(s) => output::print_warning(&s),
                    HumanLine::Error(s) => output::print_error(&s),
                }
            }

            // Test Mode diverted the write — say so, and say where, otherwise
            // the user sees "✓ Set title" and reasonably assumes their own
            // file changed.
            if let Some(copy) = written_to {
                output::print_warning(&format!(
                    "written to {copy} (Test Mode — original untouched)"
                ));
            }
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
// No `unsafe` here any more: the environment-variable juggling these tests
// need now lives in `crate::test_support`, which carries the lock that makes
// it safe and the explanation of why.
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    fn test_ctx() -> CliContext {
        CliContext {
            config: mm_core::config::AppConfig::default(),
            output: OutputFormat::Human,
            verbosity: 0,
            dry_run: false,
        }
    }

    /// Edit returns error for missing file
    #[test]
    fn edit_missing_file() {
        let ctx = test_ctx();
        let args = EditArgs {
            path: PathBuf::from("/nonexistent/file.mp3"),
            set: vec!["artist=Test".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::ERROR);
    }

    /// Edit returns error when no operations specified
    #[test]
    fn edit_no_operations() {
        let ctx = test_ctx();
        let args = EditArgs {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            set: vec![],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::ERROR);
    }

    /// Dry-run mode succeeds without modifying files — and, since the
    /// stand-in review of round 6 (M2), answers as the real run would. This
    /// used to dry-run a `Cargo.toml` and expect success; the real run on
    /// that file fails ("Cannot read tags"), so the dry run must too. A
    /// real (WAV) file is used for the success.
    #[test]
    fn edit_dry_run() {
        let _guard = ConfigDirGuard::new();
        let ctx = test_ctx();
        let dir = tempfile::tempdir().unwrap();
        let wav = dir.path().join("track.wav");
        write_wav_fixture(&wav);
        let before = std::fs::read(&wav).unwrap();
        let args = |path: PathBuf, dry_run: bool| EditArgs {
            path,
            set: vec!["artist=Test".to_string()],
            remove: vec!["genre".to_string()],
            cover: None,
            remove_cover: true,
            dry_run,
        };
        assert_eq!(
            run(&ctx, &args(wav.clone(), true)).unwrap(),
            ExitCode::SUCCESS
        );
        assert_eq!(
            std::fs::read(&wav).unwrap(),
            before,
            "a dry run writes nothing"
        );

        let text = dir.path().join("notes.txt");
        std::fs::write(&text, "not a media file").unwrap();
        for dry_run in [true, false] {
            assert_eq!(
                run(&ctx, &args(text.clone(), dry_run)).unwrap(),
                ExitCode::PARTIAL,
                "dry_run={dry_run}: a file with no tags to read is refused either way"
            );
        }
        assert_eq!(std::fs::read(&text).unwrap(), b"not a media file");
    }

    /// Invalid set format is handled gracefully
    #[test]
    fn edit_invalid_set_format() {
        let ctx = test_ctx();
        let args = EditArgs {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            set: vec!["no_equals_sign".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        // Should report partial (the invalid set fails)
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::PARTIAL);
    }

    /// Cover art from nonexistent image file
    #[test]
    fn edit_cover_missing_image() {
        let ctx = test_ctx();
        let args = EditArgs {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            set: vec![],
            remove: vec![],
            cover: Some(PathBuf::from("/nonexistent/cover.jpg")),
            remove_cover: false,
            dry_run: false,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::PARTIAL);
    }

    /// EditArgs construction
    #[test]
    fn edit_args_construction() {
        let args = EditArgs {
            path: PathBuf::from("/test/file.flac"),
            set: vec!["title=Song".to_string(), "artist=Band".to_string()],
            remove: vec!["comment".to_string()],
            cover: Some(PathBuf::from("/cover.jpg")),
            remove_cover: false,
            dry_run: true,
        };
        assert_eq!(args.set.len(), 2);
        assert_eq!(args.remove.len(), 1);
        assert!(args.dry_run);
    }

    // ── Test Mode enforcement (#128) & strict keys (#206) ───────────────────

    // The helpers these tests rely on — the `MM_CONFIG_DIR` lock, the guard
    // that redirects the configuration directory, and the WAV fixture writer
    // — live in `crate::test_support`. They used to be private to this file,
    // but `scan.rs` needs the very same lock: two private locks would each be
    // held happily while both tests fought over one environment variable.
    use crate::test_support::{ConfigDirGuard, write_wav_fixture};

    /// Test Mode must divert the write to a `_MeedyaManager` copy and leave
    /// the user's original file byte-for-byte identical.
    #[test]
    fn edit_in_test_mode_leaves_original_untouched() {
        let _guard = ConfigDirGuard::new();

        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("track.wav");
        write_wav_fixture(&original);
        let before = mm_core::integrity::file_sha256(&original).unwrap();

        mm_core::test_mode::enable().expect("test mode must enable under the isolated config dir");

        let args = EditArgs {
            path: original.clone(),
            set: vec!["title=X".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        assert_eq!(run(&test_ctx(), &args).unwrap(), ExitCode::SUCCESS);

        assert_eq!(
            mm_core::integrity::file_sha256(&original).unwrap(),
            before,
            "Test Mode must not modify the original file"
        );

        let copy = dir.path().join("track_MeedyaManager.wav");
        assert!(copy.exists(), "Test Mode copy {} missing", copy.display());

        let tags = mm_core::metadata::extract_tags(&copy).unwrap();
        assert_eq!(
            tags.get("title").map(Vec::as_slice),
            Some(&["X".to_string()][..]),
            "the copy must carry the new title"
        );

        assert!(
            mm_core::test_mode::tracked_files()
                .iter()
                .any(|e| e.original == original && e.copy == copy),
            "the copy must be recorded in the Test Mode manifest"
        );
    }

    /// An unmapped `--set` key must be reported as a failed action and must
    /// not touch the file at all.
    #[test]
    fn edit_set_unknown_key_reports_error_and_file_unchanged() {
        // Guard even though Test Mode stays off: the isolated config dir
        // stops a developer's real manifest (which may have Test Mode on)
        // from changing what this test observes.
        let _guard = ConfigDirGuard::new();

        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);
        let before = std::fs::read(&p).unwrap();

        let args = EditArgs {
            path: p.clone(),
            set: vec!["bogus=1".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        assert_eq!(
            run(&test_ctx(), &args).unwrap(),
            ExitCode::PARTIAL,
            "an unknown --set key must not report success"
        );
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "a rejected --set must leave the file byte-for-byte identical"
        );
    }

    /// Validation runs over the whole batch *before* any I/O, so one bad key
    /// aborts the good ones too — an edit batch is all-or-nothing.
    #[test]
    fn edit_mixed_batch_writes_nothing_on_invalid_key() {
        let _guard = ConfigDirGuard::new();

        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);
        let before = std::fs::read(&p).unwrap();

        let args = EditArgs {
            path: p.clone(),
            set: vec!["title=X".to_string(), "bogus=1".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        assert_eq!(run(&test_ctx(), &args).unwrap(), ExitCode::PARTIAL);
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "the valid --set must not be applied when a sibling key is invalid"
        );
    }

    /// Policy MWBM-MEDIA-LANG 1.0.0: a person typing a language MeedyaManager
    /// cannot make sense of at all is refused with a plain, non-zero-exit
    /// message, the same way an unknown key already is — never silently
    /// accepted and written as `und`.
    #[test]
    fn edit_set_language_gibberish_reports_error_and_file_unchanged() {
        let _guard = ConfigDirGuard::new();

        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);
        let before = std::fs::read(&p).unwrap();

        let args = EditArgs {
            path: p.clone(),
            set: vec!["language=not a language".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        assert_eq!(
            run(&test_ctx(), &args).unwrap(),
            ExitCode::PARTIAL,
            "a language nothing recognises must not report success"
        );
        assert_eq!(
            std::fs::read(&p).unwrap(),
            before,
            "a rejected --set must leave the file byte-for-byte identical"
        );
    }

    /// The message a refused language gets must actually help — not just say
    /// no. This is checked separately from the exit-code test above so a
    /// future change that keeps the exit code right but drops the guidance
    /// still fails a test.
    #[test]
    fn edit_set_language_gibberish_message_gives_an_example() {
        let _guard = ConfigDirGuard::new();

        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);

        // `EditPlan` (the `Ok` side) does not implement `Debug`, so this is
        // matched by hand rather than `.expect_err(...)`, which would need it to.
        let plan_err = match build_plan(&EditArgs {
            path: p,
            set: vec!["language=not a language".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        }) {
            Err(actions) => actions,
            Ok(_) => panic!("gibberish language must fail Phase 1 validation"),
        };

        let action = plan_err
            .iter()
            .find(|a| a.key.as_deref() == Some("language"))
            .expect("a 'set' failure for the 'language' key");
        let message = action.error.as_deref().unwrap_or_default();
        // Checked against the exact example text, not the bare letters
        // "en" — a message that dropped every example would still contain
        // "en" as a substring of other ordinary words, so that check could
        // never fail even if the examples were deleted entirely.
        assert!(
            message.contains("\"pt-BR\""),
            "the refusal message must show the working example \"pt-BR\", got: {message:?}"
        );
    }

    /// Codex's catch-up review, finding 2, through the command line: a
    /// language value holding a zero character (several values) or any other
    /// control character is refused in Phase 1 with its own plain message,
    /// and nothing is written, on `--dry-run` or not. A real command line
    /// cannot carry a zero character (the operating system ends an argument
    /// there), but `EditArgs` is also built by code, so it is tested here
    /// directly, beside a control character a command line CAN carry.
    #[test]
    fn edit_set_language_refuses_several_values_or_a_control_character() {
        let _guard = ConfigDirGuard::new();

        for (value, says) in [
            ("en\u{0}fr", "more than one value"),
            ("en\u{1}fr", "control character"),
            ("en\nfr", "control character"),
        ] {
            for dry_run in [false, true] {
                let dir = tempfile::tempdir().unwrap();
                let path = crate::test_support::copy_core_fixture("silence.flac", dir.path());
                let before = std::fs::read(&path).unwrap();
                let args = EditArgs {
                    path: path.clone(),
                    set: vec![format!("language={value}")],
                    remove: vec![],
                    cover: None,
                    remove_cover: false,
                    dry_run,
                };

                let Err(actions) = build_plan(&args) else {
                    panic!("{value:?}: must fail Phase 1");
                };
                let message = actions[0].error.as_deref().unwrap_or_default();
                assert!(message.contains(says), "{value:?}: {message:?}");
                assert!(
                    !message.contains('\u{0}'),
                    "{value:?}: raw zero in {message:?}"
                );

                assert_eq!(run(&test_ctx(), &args).unwrap(), ExitCode::PARTIAL);
                assert_eq!(
                    std::fs::read(&path).unwrap(),
                    before,
                    "{value:?} dry_run={dry_run}: the file must be untouched"
                );
            }
        }
    }

    /// Both accepted shapes LANG-002 defines — a full BCP 47 tag and an old
    /// three-letter code — must be accepted by `--set language=...`, and an
    /// EMPTY value must still be allowed (it clears the field, same as any
    /// other `--set key=` with nothing after the `=`).
    #[test]
    fn edit_set_language_accepts_both_shapes_and_a_clearing_empty_value() {
        let _guard = ConfigDirGuard::new();

        for value in ["en-GB", "fre", ""] {
            let dir = tempfile::tempdir().unwrap();
            let p = dir.path().join("track.wav");
            write_wav_fixture(&p);

            let args = EditArgs {
                path: p.clone(),
                set: vec![format!("language={value}")],
                remove: vec![],
                cover: None,
                remove_cover: false,
                dry_run: false,
            };
            assert_eq!(
                run(&test_ctx(), &args).unwrap(),
                ExitCode::SUCCESS,
                "language={value:?} should have been accepted"
            );
        }
    }

    /// Item 6 of the language-policy review: a value that IS a real
    /// language, but will genuinely be stored differently on this file's
    /// own container, gets a plain-English note explaining what and why —
    /// computed at Phase 1 (`build_plan`), so it is there even under
    /// `--dry-run`, which never calls `write_tags` at all.
    #[test]
    fn edit_set_language_notes_a_lost_region_even_on_dry_run() {
        let _guard = ConfigDirGuard::new();

        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);

        for dry_run in [false, true] {
            let plan = build_plan(&EditArgs {
                path: p.clone(),
                set: vec!["language=pt-BR".to_string()],
                remove: vec![],
                cover: None,
                remove_cover: false,
                dry_run,
            })
            .unwrap_or_else(|_| panic!("pt-BR is a real language, must not fail Phase 1"));

            let note = plan
                .language_note
                .as_deref()
                .unwrap_or_else(|| panic!("dry_run={dry_run}: expected a note, got None"));
            // Exact wording changed under review item 4/9 of the second
            // review round (per-container, named tag format, no bare
            // "und") — checked structurally rather than byte-for-byte so
            // this test does not need updating again for the next wording
            // tweak; `describe_conversion_reports_a_lost_region_on_id3` in
            // `language.rs` is the test that pins the exact words.
            assert!(note.contains("\"por\""), "dry_run={dry_run}: {note:?}");
            assert!(
                note.contains("an ID3 tag"),
                "dry_run={dry_run}: must name the tag format, not \"an MP3\": {note:?}"
            );
            assert!(
                note.to_lowercase().contains("region"),
                "dry_run={dry_run}: must say a region was lost: {note:?}"
            );
        }
    }

    /// The other side of item 6: nothing is lost, so there is nothing to
    /// say — an ordinary `--set language=en` must not carry a note.
    #[test]
    fn edit_set_language_has_no_note_when_nothing_is_lost() {
        let _guard = ConfigDirGuard::new();

        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);

        // `EditPlan` (via `EditAction`) does not implement `Debug`, so this
        // is matched by hand rather than `.expect(...)`, which would need it to.
        let plan = match build_plan(&EditArgs {
            path: p,
            set: vec!["language=en".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        }) {
            Ok(plan) => plan,
            Err(actions) => panic!("en is a real language, must not fail Phase 1: {actions:?}"),
        };
        assert_eq!(plan.language_note, None);
    }

    /// Review item 1 of the second language-policy review round: the
    /// existing two tests above only ever checked `EditPlan.language_note`
    /// — the value that reaches `EditOutput` for `--json` callers — never
    /// what `render`'s Human branch actually prints from it. That branch
    /// simply never read `action.note` at all, so every person running
    /// `meedya edit` from a terminal (the overwhelmingly more common case
    /// than `--json`) never saw the note however real the loss was. This
    /// test calls `build_human_lines` — the exact function `render` itself
    /// calls, not a re-implementation of it — with a synthetic action
    /// carrying a note, and checks the note appears as its own
    /// [`HumanLine::Warning`] right after the success line, which is
    /// exactly what the bug meant did not happen.
    #[test]
    fn build_human_lines_shows_the_language_note_after_the_success_line() {
        let actions = vec![EditAction::ok_with_note(
            "set",
            Some("language".to_string()),
            Some("pt-BR".to_string()),
            "an ID3 tag can only hold the three-letter language code, so it will lose the \
             region you typed — it will be stored there as \"por\""
                .to_string(),
        )];

        let lines = build_human_lines(&actions);

        assert_eq!(
            lines,
            vec![
                HumanLine::Success("Set language = pt-BR".to_string()),
                HumanLine::Warning(
                    "an ID3 tag can only hold the three-letter language code, so it will lose \
                     the region you typed — it will be stored there as \"por\""
                        .to_string()
                ),
            ],
            "the note must be its own Warning line, immediately after the Success line it \
             explains"
        );
    }

    /// Third review round, item 2: the tests above check each link of the
    /// chain on its own — the plan holds a note, and `build_human_lines`
    /// prints a note it is HANDED — but nothing checked the link between
    /// them, `set_action_for`, which copies the plan's note onto the action
    /// that gets printed. The reviewer broke exactly that link (the note
    /// was never attached) and every test still passed. This walks the
    /// whole chain on a real MP3, the way `run` does: `build_plan`, then
    /// both `describe_plan` (`--dry-run`) and `apply_plan` (a real write),
    /// then `build_human_lines` — and checks the warning is really among
    /// the printed lines both times, and that the file really holds what
    /// the warning says.
    #[test]
    fn a_language_note_reaches_the_printed_lines_on_dry_run_and_real_write() {
        let _guard = ConfigDirGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let path = crate::test_support::copy_core_fixture("silence.mp3", dir.path());

        let args = EditArgs {
            path: path.clone(),
            set: vec!["language=pt-BR".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        let plan = match build_plan(&args) {
            Ok(plan) => plan,
            Err(actions) => panic!("pt-BR is a real language: {actions:?}"),
        };
        let expected_note = "an ID3 tag can only hold the three-letter language code, so it \
                             will lose the region you typed — it will be stored there as \
                             \"por\"";

        let (applied, _written_to) = apply_plan(&args, &plan);
        for (which, actions) in [("dry run", describe_plan(&plan)), ("real write", applied)] {
            let action = actions
                .iter()
                .find(|a| a.key.as_deref() == Some("language"))
                .unwrap_or_else(|| panic!("{which}: no action for the language key"));
            assert!(action.success, "{which}: {action:?}");
            assert_eq!(
                action.note.as_deref(),
                Some(expected_note),
                "{which}: the plan's note must be attached to the action that is printed"
            );
            assert_eq!(
                build_human_lines(&actions),
                vec![
                    HumanLine::Success("Set language = pt-BR".to_string()),
                    HumanLine::Warning(expected_note.to_string()),
                ],
                "{which}: the warning must be among the lines a person actually sees"
            );
        }

        // The warning says "por" is what is stored — check that it is.
        let tags = mm_core::metadata::extract_tags(&path).unwrap();
        assert_eq!(
            tags.get("language").map(Vec::as_slice),
            Some(&["por".to_string()][..])
        );
    }

    /// Third review round, item 3: on a WAV whose RIFF INFO chunk says
    /// "fre" and whose ID3 tag says "ger", "fre" is what the file shows
    /// (a tag holding the full code is read first — TRACK-070). Sending
    /// "fre" is therefore a no-change (COMPAT-030 — and it must stay one,
    /// because every editing screen resends every field on save), so the
    /// ID3 tag goes on saying German. Before this round `meedya edit`
    /// printed a bare "✓ Set language = fre" — reproduced with the binary
    /// built from e18fb18. It must say the ID3 tag disagrees and is left
    /// alone, on `--dry-run` and on a real write alike.
    #[test]
    fn resending_the_shown_language_when_the_tags_disagree_is_never_a_silent_set() {
        let _guard = ConfigDirGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let path = crate::test_support::copy_core_fixture("lang_riff_fre_id3_ger.wav", dir.path());

        let args = EditArgs {
            path: path.clone(),
            set: vec!["language=fre".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        let plan = match build_plan(&args) {
            Ok(plan) => plan,
            Err(actions) => panic!("fre is a real language: {actions:?}"),
        };
        let expected_note = "this file's ID3 tag says \"ger\", which disagrees with \"fre\" and \
                             will be left alone, because \"fre\" is already the file's language";

        let (applied, _written_to) = apply_plan(&args, &plan);
        for (which, actions) in [("dry run", describe_plan(&plan)), ("real write", applied)] {
            assert_eq!(
                build_human_lines(&actions),
                vec![
                    HumanLine::Success("Set language = fre".to_string()),
                    HumanLine::Warning(expected_note.to_string()),
                ],
                "{which}: a person must be told the ID3 tag still says something else"
            );
        }

        // "Left alone" really means left alone: the file still shows "fre"
        // and its ID3 tag still disagrees.
        let tags = mm_core::metadata::extract_tags(&path).unwrap();
        assert_eq!(
            tags.get("language").map(Vec::as_slice),
            Some(&["fre".to_string()][..])
        );
        assert!(
            mm_core::metadata::language::disagreement_note(&path).is_some(),
            "a resend must not quietly rewrite the ID3 tag (COMPAT-030)"
        );
    }

    /// The same disagreement in a FLAC that starts with an ID3 tag (Vorbis
    /// "eng", ID3 "ger"). MeedyaManager can read such a file but cannot yet
    /// SAVE one (a separate, older fault with its own issue), so only the
    /// `--dry-run` preview is checked here.
    #[test]
    fn a_flac_whose_leading_id3_tag_disagrees_gets_the_note_on_dry_run() {
        let _guard = ConfigDirGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let path =
            crate::test_support::copy_core_fixture("lang_vorbis_eng_id3_ger.flac", dir.path());

        let plan = match build_plan(&EditArgs {
            path,
            set: vec!["language=eng".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: true,
        }) {
            Ok(plan) => plan,
            Err(actions) => panic!("eng is a real language: {actions:?}"),
        };
        let lines = build_human_lines(&describe_plan(&plan));
        assert_eq!(
            lines,
            vec![
                HumanLine::Success("Set language = eng".to_string()),
                HumanLine::Warning(
                    "this file's ID3 tag says \"ger\", which disagrees with \"eng\" and will be \
                     left alone, because \"eng\" is already the file's language"
                        .to_string()
                ),
            ]
        );
    }

    /// Setting a genuinely NEW language on that WAV updates every tag that
    /// already holds one (the first review round's fix), so the tags agree
    /// afterwards and there is nothing to warn about, before or after.
    #[test]
    fn setting_a_new_language_brings_disagreeing_tags_into_line_without_a_note() {
        let _guard = ConfigDirGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let path = crate::test_support::copy_core_fixture("lang_riff_fre_id3_ger.wav", dir.path());

        let args = EditArgs {
            path: path.clone(),
            set: vec!["language=es".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        let plan = match build_plan(&args) {
            Ok(plan) => plan,
            Err(actions) => panic!("es is a real language: {actions:?}"),
        };
        assert_eq!(plan.language_note, None);

        let (applied, _written_to) = apply_plan(&args, &plan);
        assert!(applied.iter().all(|a| a.success), "{applied:?}");
        assert_eq!(mm_core::metadata::language::disagreement_note(&path), None);
        let tags = mm_core::metadata::extract_tags(&path).unwrap();
        assert_eq!(
            tags.get("language").map(Vec::as_slice),
            Some(&["es".to_string()][..])
        );
    }

    /// Plan and apply one `--set` batch the way `run` does, and return the
    /// plan's language note, the printed actions and where the write landed.
    fn plan_and_apply(
        path: &std::path::Path,
        set: &str,
    ) -> (Option<String>, Vec<EditAction>, Option<String>) {
        let args = EditArgs {
            path: path.to_path_buf(),
            set: vec![set.to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        let plan = match build_plan(&args) {
            Ok(plan) => plan,
            Err(actions) => panic!("`--set {set}` is acceptable: {actions:?}"),
        };
        let (actions, written_to) = apply_plan(&args, &plan);
        assert!(actions.iter().all(|a| a.success), "{set}: {actions:?}");
        (plan.language_note, actions, written_to)
    }

    /// Fourth review round, S1: with Test Mode on, the SECOND edit of a file
    /// is saved to the Test Mode copy the first edit made, not to the file
    /// named on the command line — so the note must describe that copy.
    ///
    /// The reviewer's two steps, on a WAV whose RIFF INFO chunk says "fre"
    /// and whose ID3 tag says "ger": set "es" (makes the copy, both of its
    /// tags now Spanish), then set "fre". Reproduced with the binary built
    /// from `aa7a30d`: the second step printed "this file's ID3 tag says
    /// \"ger\" ... and will be left alone", read from the untouched
    /// original, while mutagen then read `fra` from the copy's ID3 tag —
    /// the tag the note said would keep saying German had just been
    /// rewritten. The copy agreed with itself (Spanish), so the true answer
    /// is no note at all.
    #[test]
    fn in_test_mode_the_note_describes_the_copy_the_change_is_saved_to() {
        let _guard = ConfigDirGuard::new();
        mm_core::test_mode::enable().expect("Test Mode must switch on in the private config");
        let dir = tempfile::tempdir().unwrap();
        let path = crate::test_support::copy_core_fixture("lang_riff_fre_id3_ger.wav", dir.path());
        let copy = mm_core::test_mode::test_mode_path(&path);

        let (_note, _actions, written_to) = plan_and_apply(&path, "language=es");
        assert_eq!(
            written_to.as_deref(),
            Some(copy.display().to_string().as_str()),
            "step 1 must make the Test Mode copy"
        );

        let (note, actions, written_to) = plan_and_apply(&path, "language=fre");
        assert_eq!(
            written_to.as_deref(),
            Some(copy.display().to_string().as_str()),
            "step 2 must land on the same copy"
        );
        assert_eq!(
            note, None,
            "the copy's tags both said Spanish, so nothing disagrees and nothing is lost"
        );
        assert_eq!(
            build_human_lines(&actions),
            vec![HumanLine::Success("Set language = fre".to_string())]
        );

        // What the copy really holds matches "nothing to say": one French
        // value shown, and an ID3 tag that agrees with it.
        let copy_tags = mm_core::metadata::extract_tags(&copy).unwrap();
        assert_eq!(
            copy_tags.get("language").map(Vec::as_slice),
            Some(&["fr".to_string()][..])
        );
        assert_eq!(mm_core::metadata::language::disagreement_note(&copy), None);

        // The original was never touched, and still disagrees with itself —
        // which is exactly what the old note wrongly described.
        let original_tags = mm_core::metadata::extract_tags(&path).unwrap();
        assert_eq!(
            original_tags.get("language").map(Vec::as_slice),
            Some(&["fre".to_string()][..])
        );
        assert!(mm_core::metadata::language::disagreement_note(&path).is_some());
    }

    /// The other direction of S1: a note the copy calls for, which the
    /// original would not. The original holds "en-JJ" (a region code nobody
    /// has registered), so resending "en-JJ" to the ORIGINAL is a no-change
    /// with nothing to say. But after a first Test Mode edit set "en", the
    /// change is saved to the copy, which holds "en" — so "en-JJ" is a real
    /// change there, and the person must be told "JJ" is not on the
    /// official list (and is kept as typed). Checked with the real binary
    /// too: before the fix it printed no note at all for this change.
    #[test]
    fn in_test_mode_a_note_the_copy_needs_is_not_lost() {
        let _guard = ConfigDirGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let path = crate::test_support::copy_core_fixture("riff_language.wav", dir.path());
        // Before Test Mode: set "en-JJ" on the original. Its RIFF INFO chunk
        // keeps it whole; the ID3 tag `write_tags` gives every WAV keeps
        // "eng".
        mm_core::metadata::write_tags(&path, &{
            let mut tags = mm_core::metadata::TagMap::new();
            tags.insert("language".to_string(), vec!["en-JJ".to_string()]);
            tags
        })
        .unwrap();
        assert_eq!(
            mm_core::metadata::language::preview_conversion_note(&path, "en-JJ"),
            None,
            "setup: on the original, en-JJ is a resend with nothing to say"
        );

        mm_core::test_mode::enable().expect("Test Mode must switch on in the private config");
        let copy = mm_core::test_mode::test_mode_path(&path);
        let (_note, _actions, _written_to) = plan_and_apply(&path, "language=en");
        let (note, _actions, _written_to) = plan_and_apply(&path, "language=en-JJ");
        let note = note.expect("en-JJ is a real change on the copy, which held en");
        assert!(
            note.contains("\"JJ\" is not on the official list"),
            "{note}"
        );
        assert!(note.contains("kept exactly as typed"), "{note}");

        // And it was kept exactly as typed, on the copy.
        assert_eq!(
            mm_core::metadata::extract_tags(&copy)
                .unwrap()
                .get("language")
                .map(Vec::as_slice),
            Some(&["en-JJ".to_string()][..])
        );
    }

    /// Codex's catch-up review, finding 3: the same field set twice in one
    /// command is refused before anything is written — in a real run and on
    /// `--dry-run` — with every `--set` answered by a failed row that names
    /// the field and every time it was given. Since the stand-in review of
    /// round 6 (M3), the same for a field given to `--set` and `--remove`,
    /// or to `--remove` twice. It used to be "the last one wins",
    /// with every row reported as done and carrying the last value's note.
    /// Covers `language` (with the case that once needed its note dropped: a
    /// clearing `--set language=` after `pt-BR`) and an ordinary field.
    #[test]
    fn the_same_field_set_twice_is_refused_before_anything_is_written() {
        let _guard = ConfigDirGuard::new();

        // (the `--set` values, the `--remove` keys, the field, what the
        // message must name). The stand-in review of round 6 (M3) added the
        // last three: a field in both `--set` and `--remove`, or in
        // `--remove` twice, was reported as done every time.
        type Case<'a> = (&'a [&'a str], &'a [&'a str], &'a str, &'a str);
        let cases: [Case; 6] = [
            (
                &["language=en", "language=pt-BR"],
                &[],
                "language",
                "(--set language=en, then --set language=pt-BR)",
            ),
            (
                &["language=pt-BR", "language="],
                &[],
                "language",
                "(--set language=pt-BR, then --set language=)",
            ),
            (
                &["title=One", "artist=Someone", "title=Two"],
                &[],
                "title",
                "(--set title=One, then --set title=Two)",
            ),
            (
                &["language=pt-BR"],
                &["language"],
                "language",
                "(--set language=pt-BR, then --remove language)",
            ),
            (
                &["title=X"],
                &["title"],
                "title",
                "(--set title=X, then --remove title)",
            ),
            (
                &[],
                &["title", "title"],
                "title",
                "(--remove title, then --remove title)",
            ),
        ];
        for (sets, removes, field, values) in cases {
            for dry_run in [false, true] {
                let dir = tempfile::tempdir().unwrap();
                let path = crate::test_support::copy_core_fixture("silence.mp3", dir.path());
                let before = std::fs::read(&path).unwrap();
                let args = EditArgs {
                    path: path.clone(),
                    set: sets.iter().map(ToString::to_string).collect(),
                    remove: removes.iter().map(ToString::to_string).collect(),
                    cover: None,
                    remove_cover: false,
                    dry_run,
                };

                let Err(actions) = build_plan(&args) else {
                    panic!("{sets:?} {removes:?}: '{field}' given twice must be refused");
                };
                let refused: Vec<&EditAction> = actions
                    .iter()
                    .filter(|a| a.key.as_deref() == Some(field))
                    .collect();
                assert_eq!(
                    refused.len(),
                    2,
                    "{sets:?} {removes:?}: one failed row each time '{field}' was given"
                );
                for action in refused {
                    assert!(!action.success && action.note.is_none(), "{action:?}");
                    let message = action.error.as_deref().unwrap_or_default();
                    assert!(
                        message.contains(&format!("'{field}' is given more than once"))
                            && message.contains(values),
                        "{sets:?} {removes:?}: {message:?}"
                    );
                }

                assert_eq!(
                    run(&test_ctx(), &args).unwrap(),
                    ExitCode::PARTIAL,
                    "{sets:?} {removes:?} dry_run={dry_run}: must not report success"
                );
                assert_eq!(
                    std::fs::read(&path).unwrap(),
                    before,
                    "{sets:?} {removes:?} dry_run={dry_run}: nothing may be written"
                );
            }
        }
    }

    /// Carry-over 1 of the stand-in review of round 6, through the command a
    /// person types. Reproduced with the `meedya` binary built from
    /// `49cec29`: `meedya edit <file> --remove artist` on a WAV with no
    /// artist at all, whose RIFF INFO title was Latin-1 "Café", printed
    /// "✓ Remove artist" and deleted the title. Nothing holds an artist, so
    /// nothing may be rewritten: the file must be byte for byte as it was.
    #[test]
    fn remove_artist_on_a_wav_keeps_its_latin1_title() {
        let _guard = ConfigDirGuard::new();
        let dir = tempfile::tempdir().unwrap();
        let path =
            crate::test_support::copy_core_fixture("lang_riff_fre_title_latin1.wav", dir.path());
        let before = std::fs::read(&path).unwrap();
        let args = EditArgs {
            path: path.clone(),
            set: vec![],
            remove: vec!["artist".to_string()],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        assert_eq!(run(&test_ctx(), &args).unwrap(), ExitCode::SUCCESS);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            before,
            "the title must survive: nothing held an artist"
        );
    }

    /// The stand-in review of round 7, M1, through the command a person
    /// types. Reproduced with the `meedya` binary built from `e4db8f8` on a
    /// WAV whose RIFF INFO list holds the title "Café" (all UTF-8): `--set
    /// title=New` printed "✓" and left "Café" in the list beside the new ID3
    /// title, so `meedya debug` showed `["Café", "New"]`; `--set title=`
    /// printed "✓" and changed nothing. The list now holds exactly what was
    /// asked, so the file reads back as one title, or none.
    #[test]
    fn setting_or_clearing_a_title_a_wav_list_holds_changes_it_there_too() {
        let _guard = ConfigDirGuard::new();
        for (value, expected) in [("New", Some(vec!["New".to_string()])), ("", None)] {
            let dir = tempfile::tempdir().unwrap();
            let path =
                crate::test_support::copy_core_fixture("lang_riff_fre_all_utf8.wav", dir.path());
            let args = EditArgs {
                path: path.clone(),
                set: vec![format!("title={value}")],
                remove: vec![],
                cover: None,
                remove_cover: false,
                dry_run: false,
            };
            assert_eq!(run(&test_ctx(), &args).unwrap(), ExitCode::SUCCESS);
            let tags = mm_core::metadata::extract_tags(&path).unwrap();
            assert_eq!(
                tags.get("title"),
                expected.as_ref(),
                "--set title={value}: the title the file holds afterwards"
            );
        }
    }

    /// The stand-in review of round 6, M2: `--dry-run` gives the same answer
    /// and exit code as the real run when the save would refuse from inside.
    /// Reproduced with the binary built from `49cec29` on a WAV with a
    /// Latin-1 title: `--set language=en --dry-run` printed "✓ Set language
    /// = en" and exited 0; the real run refused and exited 2. Both now
    /// refuse, exit 2, leave the file as it was, and say exactly what the
    /// save itself says when it refuses — for a set and for a removal of
    /// the language.
    #[test]
    fn dry_run_predicts_a_wav_refusal_like_the_real_run() {
        let _guard = ConfigDirGuard::new();
        let fixture = "lang_riff_fre_title_latin1.wav";
        // The words after the file's path, which differs from copy to copy.
        let after_path = |message: &str| message.split_once("': ").map(|(_, r)| r.to_string());

        for removing in [false, true] {
            // What the save itself answers, called directly.
            let dir = tempfile::tempdir().unwrap();
            let path = crate::test_support::copy_core_fixture(fixture, dir.path());
            let saved = if removing {
                mm_core::integrity::remove_tag_safe(&path, "language")
            } else {
                let mut tags = mm_core::metadata::TagMap::new();
                tags.insert("language".to_string(), vec!["en".to_string()]);
                mm_core::integrity::write_tags_safe(&path, &tags)
            };
            assert!(!saved.success, "removing={removing}: the save refuses");
            let save_says = after_path(&saved.error.unwrap_or_default());
            assert!(
                save_says
                    .as_deref()
                    .is_some_and(|s| s.contains("the title (INAM)")),
                "{save_says:?}"
            );

            for dry_run in [true, false] {
                let dir = tempfile::tempdir().unwrap();
                let path = crate::test_support::copy_core_fixture(fixture, dir.path());
                let before = std::fs::read(&path).unwrap();
                let args = EditArgs {
                    path: path.clone(),
                    set: if removing {
                        vec![]
                    } else {
                        vec!["language=en".to_string()]
                    },
                    remove: if removing {
                        vec!["language".to_string()]
                    } else {
                        vec![]
                    },
                    cover: None,
                    remove_cover: false,
                    dry_run,
                };
                let Err(actions) = build_plan(&args) else {
                    panic!("removing={removing} dry_run={dry_run}: must be refused");
                };
                assert_eq!(actions.len(), 1, "{actions:?}");
                assert_eq!(
                    after_path(actions[0].error.as_deref().unwrap_or_default()),
                    save_says,
                    "removing={removing} dry_run={dry_run}: the save's own words"
                );
                assert_eq!(
                    run(&test_ctx(), &args).unwrap(),
                    ExitCode::PARTIAL,
                    "removing={removing} dry_run={dry_run}: exit 2"
                );
                assert_eq!(std::fs::read(&path).unwrap(), before, "file untouched");
            }
        }
    }

    /// The companion case: an action with no note produces exactly one
    /// line, not a spurious empty warning.
    #[test]
    fn build_human_lines_has_no_extra_line_when_there_is_no_note() {
        let actions = vec![EditAction::ok(
            "set",
            Some("language".to_string()),
            Some("en".to_string()),
        )];
        assert_eq!(
            build_human_lines(&actions),
            vec![HumanLine::Success("Set language = en".to_string())]
        );
    }

    /// The `--remove` side of #206: an unmapped key used to be a silent
    /// `Ok(())` in the raw metadata layer, so the CLI reported "✓ Remove".
    #[test]
    fn edit_remove_unknown_key_reports_error_and_file_unchanged() {
        let _guard = ConfigDirGuard::new();

        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);
        let before = std::fs::read(&p).unwrap();

        let args = EditArgs {
            path: p.clone(),
            set: vec![],
            remove: vec!["bogus".to_string()],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        assert_eq!(run(&test_ctx(), &args).unwrap(), ExitCode::PARTIAL);
        assert_eq!(std::fs::read(&p).unwrap(), before);
    }

    /// Outside Test Mode the edit must land on the user's own file.
    #[test]
    fn edit_outside_test_mode_writes_the_original() {
        let _guard = ConfigDirGuard::new();

        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("track.wav");
        write_wav_fixture(&p);

        let args = EditArgs {
            path: p.clone(),
            set: vec!["title=Direct".to_string()],
            remove: vec![],
            cover: None,
            remove_cover: false,
            dry_run: false,
        };
        assert_eq!(run(&test_ctx(), &args).unwrap(), ExitCode::SUCCESS);

        let tags = mm_core::metadata::extract_tags(&p).unwrap();
        assert_eq!(
            tags.get("title").map(Vec::as_slice),
            Some(&["Direct".to_string()][..])
        );
        assert!(
            !dir.path().join("track_MeedyaManager.wav").exists(),
            "no Test Mode copy may appear with Test Mode off"
        );
    }
}
