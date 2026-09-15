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
#[derive(Serialize)]
struct EditAction {
    action: String,
    key: Option<String>,
    value: Option<String>,
    success: bool,
    error: Option<String>,
}

impl EditAction {
    /// A successful action.
    fn ok(action: &str, key: Option<String>, value: Option<String>) -> Self {
        Self {
            action: action.to_string(),
            key,
            value,
            success: true,
            error: None,
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

        // Later `--set` occurrences of the same key win, matching the
        // last-flag-wins convention users expect from a CLI.
        tags.insert(key.to_string(), vec![value.to_string()]);
        set_pairs.push((key.to_string(), value.to_string()));
    }

    // -- --remove: check the key -------------------------------------------
    let mut remove_keys: Vec<String> = Vec::new();
    for key in &args.remove {
        if known.contains(&key.as_str()) {
            remove_keys.push(key.clone());
        } else {
            failures.push(EditAction::failed(
                "remove",
                Some(key.clone()),
                None,
                format!("unknown key '{key}' — valid: {valid_list}"),
            ));
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

    if failures.is_empty() {
        Ok(EditPlan {
            tags,
            set_pairs,
            remove_keys,
            cover,
            remove_cover: args.remove_cover,
        })
    } else {
        Err(failures)
    }
}

/// Describe a validated plan without performing it — the `--dry-run` path.
fn describe_plan(plan: &EditPlan) -> Vec<EditAction> {
    let mut actions = Vec::new();

    for (key, value) in &plan.set_pairs {
        actions.push(EditAction::ok(
            "set",
            Some(key.clone()),
            Some(value.clone()),
        ));
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
                EditAction::ok("set", Some(key.clone()), Some(value.clone()))
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

            for action in &actions {
                let desc = match action.action.as_str() {
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
                };

                if action.success {
                    output::print_success(&desc);
                } else {
                    output::print_error(&format!(
                        "{}: {}",
                        desc,
                        action.error.as_deref().unwrap_or("unknown error"),
                    ));
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

    /// Dry-run mode succeeds without modifying files
    #[test]
    fn edit_dry_run() {
        let ctx = test_ctx();
        let args = EditArgs {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            set: vec!["artist=Test".to_string()],
            remove: vec!["genre".to_string()],
            cover: None,
            remove_cover: true,
            dry_run: true,
        };
        assert_eq!(run(&ctx, &args).unwrap(), ExitCode::SUCCESS);
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
