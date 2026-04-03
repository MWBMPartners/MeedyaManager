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

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya edit` command.
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

    let mut actions: Vec<EditAction> = Vec::new();

    // ── 1. Parse and apply --set operations ─────────────────────────────
    for set_arg in &args.set {
        // Parse "key=value" format
        let (key, value) = if let Some((k, v)) = set_arg.split_once('=') {
            (k.to_string(), v.to_string())
        } else {
            let action = EditAction {
                action: "set".to_string(),
                key: Some(set_arg.clone()),
                value: None,
                success: false,
                error: Some("Invalid format — expected key=value".to_string()),
            };
            actions.push(action);
            continue;
        };

        if dry_run {
            actions.push(EditAction {
                action: "set".to_string(),
                key: Some(key),
                value: Some(value),
                success: true,
                error: None,
            });
        } else {
            // Build a TagMap with the single key-value pair
            let mut tags = mm_core::metadata::TagMap::new();
            tags.insert(key.clone(), vec![value.clone()]);

            match mm_core::metadata::write_tags(&args.path, &tags) {
                Ok(()) => {
                    actions.push(EditAction {
                        action: "set".to_string(),
                        key: Some(key),
                        value: Some(value),
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    actions.push(EditAction {
                        action: "set".to_string(),
                        key: Some(key),
                        value: Some(value),
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
    }

    // ── 2. Apply --remove operations ────────────────────────────────────
    for key in &args.remove {
        if dry_run {
            actions.push(EditAction {
                action: "remove".to_string(),
                key: Some(key.clone()),
                value: None,
                success: true,
                error: None,
            });
        } else {
            match mm_core::metadata::remove_tag(&args.path, key) {
                Ok(()) => {
                    actions.push(EditAction {
                        action: "remove".to_string(),
                        key: Some(key.clone()),
                        value: None,
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    actions.push(EditAction {
                        action: "remove".to_string(),
                        key: Some(key.clone()),
                        value: None,
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
    }

    // ── 3. Handle --cover (embed cover art) ─────────────────────────────
    if let Some(ref cover_path) = args.cover {
        if !cover_path.exists() {
            actions.push(EditAction {
                action: "embed_cover".to_string(),
                key: None,
                value: Some(cover_path.display().to_string()),
                success: false,
                error: Some("Image file not found".to_string()),
            });
        } else if dry_run {
            actions.push(EditAction {
                action: "embed_cover".to_string(),
                key: None,
                value: Some(cover_path.display().to_string()),
                success: true,
                error: None,
            });
        } else {
            // Read the image file
            let data = std::fs::read(cover_path)?;
            // Guess MIME type from extension
            let mime = match cover_path.extension().and_then(|e| e.to_str()) {
                Some("png") => "image/png",
                Some("jpg" | "jpeg") => "image/jpeg",
                Some("gif") => "image/gif",
                Some("webp") => "image/webp",
                _ => "image/jpeg", // Default fallback
            };

            match mm_core::metadata::embed_cover_art(&args.path, &data, mime) {
                Ok(()) => {
                    actions.push(EditAction {
                        action: "embed_cover".to_string(),
                        key: None,
                        value: Some(cover_path.display().to_string()),
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    actions.push(EditAction {
                        action: "embed_cover".to_string(),
                        key: None,
                        value: Some(cover_path.display().to_string()),
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
    }

    // ── 4. Handle --remove-cover ────────────────────────────────────────
    if args.remove_cover {
        if dry_run {
            actions.push(EditAction {
                action: "remove_cover".to_string(),
                key: None,
                value: None,
                success: true,
                error: None,
            });
        } else {
            match mm_core::metadata::remove_cover_art(&args.path) {
                Ok(()) => {
                    actions.push(EditAction {
                        action: "remove_cover".to_string(),
                        key: None,
                        value: None,
                        success: true,
                        error: None,
                    });
                }
                Err(e) => {
                    actions.push(EditAction {
                        action: "remove_cover".to_string(),
                        key: None,
                        value: None,
                        success: false,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
    }

    // ── 5. Render output ────────────────────────────────────────────────
    let any_failed = actions.iter().any(|a| !a.success);

    match ctx.output {
        OutputFormat::Json => {
            output::print_json(&EditOutput {
                file: args.path.display().to_string(),
                actions,
                dry_run,
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
        }
    }

    Ok(if any_failed {
        ExitCode::PARTIAL
    } else {
        ExitCode::SUCCESS
    })
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
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
}
