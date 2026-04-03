// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya rule` Command
//
// Template validation, tag listing, template testing with real files, and
// legacy MusicBee syntax detection. Provides CLI access to the rule engine
// without requiring a full scan or organise operation.

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya rule` command.
#[derive(Args, Debug)]
pub struct RuleArgs {
    /// Rule subcommand to execute
    #[command(subcommand)]
    pub action: RuleAction,
}

/// Available rule subcommands.
#[derive(Subcommand, Debug)]
pub enum RuleAction {
    /// Validate a template string (check syntax without evaluating)
    Validate {
        /// The template string to validate (e.g. "<Artist>\\<Album>\\<Title>")
        template: String,
    },

    /// List all known tag names and their types
    Tags,

    /// Test a template against a real media file
    Test {
        /// The template string to evaluate
        template: String,
        /// Path to the media file to use as context
        file: PathBuf,
    },

    /// Detect legacy MusicBee-style `{key}` syntax in a template
    Legacy {
        /// The template string to check for legacy syntax
        template: String,
    },
}

// ─── JSON output structures ─────────────────────────────────────────────────

/// Validation result for JSON output.
#[derive(Serialize)]
struct ValidateOutput {
    template: String,
    valid: bool,
    error: Option<String>,
    ast: Option<String>,
}

/// Tag listing entry for JSON output.
#[derive(Serialize)]
struct TagEntry {
    name: String,
    kind: String,
}

/// Template test result for JSON output.
#[derive(Serialize)]
struct TestOutput {
    template: String,
    file: String,
    result: Option<String>,
    error: Option<String>,
}

/// Legacy detection result for JSON output.
#[derive(Serialize)]
struct LegacyOutput {
    template: String,
    legacy_keys: Vec<String>,
}

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya rule` command.
///
/// Dispatches to the appropriate subcommand handler.
pub fn run(ctx: &CliContext, args: &RuleArgs) -> anyhow::Result<i32> {
    match &args.action {
        RuleAction::Validate { template } => run_validate(ctx, template),
        RuleAction::Tags => run_tags(ctx),
        RuleAction::Test { template, file } => run_test(ctx, template, file),
        RuleAction::Legacy { template } => run_legacy(ctx, template),
    }
}

// ─── Subcommand: validate ───────────────────────────────────────────────────

/// Validate a template string by parsing it into an AST.
fn run_validate(ctx: &CliContext, template: &str) -> anyhow::Result<i32> {
    // Attempt to parse the template into an AST
    let result = mm_core::rule_engine::parse_template(template);

    match ctx.output {
        OutputFormat::Json => {
            let output = match &result {
                Ok(ast) => ValidateOutput {
                    template: template.to_string(),
                    valid: true,
                    error: None,
                    ast: Some(format!("{ast:?}")),
                },
                Err(e) => ValidateOutput {
                    template: template.to_string(),
                    valid: false,
                    error: Some(e.to_string()),
                    ast: None,
                },
            };
            output::print_json(&output);
        }
        OutputFormat::Human => match &result {
            Ok(ast) => {
                output::print_success("Template is valid");
                output::print_header("AST");
                println!("{ast:#?}");
            }
            Err(e) => {
                output::print_error(&format!("Invalid template: {e}"));
            }
        },
    }

    // Return error exit code if template is invalid
    Ok(if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::ERROR
    })
}

// ─── Subcommand: tags ───────────────────────────────────────────────────────

/// List all known tag names and their types.
fn run_tags(ctx: &CliContext) -> anyhow::Result<i32> {
    // Get all registered tags from the tag registry
    let tags = mm_core::rule_engine::tag_registry::all_tags();

    match ctx.output {
        OutputFormat::Json => {
            let entries: Vec<TagEntry> = tags
                .iter()
                .map(|(name, kind)| TagEntry {
                    name: name.clone(),
                    kind: format!("{kind:?}"),
                })
                .collect();
            output::print_json(&entries);
        }
        OutputFormat::Human => {
            output::print_header(&format!(
                "Known Tags ({} entries)",
                mm_core::rule_engine::tag_registry::registry_size()
            ));
            let rows: Vec<Vec<String>> = tags
                .iter()
                .map(|(name, kind)| {
                    let kind_str = match kind {
                        mm_core::rule_engine::TagKind::Metadata(key) => {
                            format!("Metadata ({key})")
                        }
                        mm_core::rule_engine::TagKind::Virtual(vt) => {
                            format!("Virtual ({vt:?})")
                        }
                        mm_core::rule_engine::TagKind::Custom(key) => {
                            format!("Custom ({key})")
                        }
                    };
                    vec![name.clone(), kind_str]
                })
                .collect();
            output::print_table(&["Tag Name", "Kind"], &rows);
        }
    }

    Ok(ExitCode::SUCCESS)
}

// ─── Subcommand: test ───────────────────────────────────────────────────────

/// Test a template against a real media file.
fn run_test(ctx: &CliContext, template: &str, file: &std::path::Path) -> anyhow::Result<i32> {
    // Verify the file exists
    if !file.exists() {
        output::print_error(&format!("File not found: {}", file.display()));
        return Ok(ExitCode::ERROR);
    }

    // Extract metadata from the file to build an evaluation context
    let tags = mm_core::metadata::extract_tags(file).unwrap_or_default();
    let audio_props = mm_core::metadata::extract_audio_properties(file).ok();
    let classification = mm_core::classify::classify_by_path(file).ok();

    // Build the EvalContext with available data
    let mut eval_ctx = mm_core::rule_engine::EvalContext::new(&tags);
    if let Some(ref props) = audio_props {
        eval_ctx = eval_ctx.with_audio_props(props);
    }
    if let Some(ref class) = classification {
        eval_ctx = eval_ctx.with_classification(class);
    }
    eval_ctx = eval_ctx.with_file_path(file);

    // Evaluate the template
    let result = mm_core::rule_engine::evaluate_template(template, &eval_ctx);

    match ctx.output {
        OutputFormat::Json => {
            let output = match &result {
                Ok(value) => TestOutput {
                    template: template.to_string(),
                    file: file.display().to_string(),
                    result: Some(value.clone()),
                    error: None,
                },
                Err(e) => TestOutput {
                    template: template.to_string(),
                    file: file.display().to_string(),
                    result: None,
                    error: Some(e.to_string()),
                },
            };
            output::print_json(&output);
        }
        OutputFormat::Human => match &result {
            Ok(value) => {
                output::print_key_value("Template", template);
                output::print_key_value("File", &file.display().to_string());
                output::print_header("Result");
                println!("  {value}");
            }
            Err(e) => {
                output::print_error(&format!("Evaluation failed: {e}"));
            }
        },
    }

    Ok(if result.is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::ERROR
    })
}

// ─── Subcommand: legacy ─────────────────────────────────────────────────────

/// Detect legacy MusicBee-style `{key}` syntax in a template.
fn run_legacy(ctx: &CliContext, template: &str) -> anyhow::Result<i32> {
    // Detect any legacy {key} patterns in the template
    let legacy_keys = mm_core::rule_engine::detect_legacy_syntax(template);

    match ctx.output {
        OutputFormat::Json => {
            let output = LegacyOutput {
                template: template.to_string(),
                legacy_keys,
            };
            output::print_json(&output);
        }
        OutputFormat::Human => {
            if legacy_keys.is_empty() {
                output::print_success("No legacy syntax detected");
            } else {
                output::print_warning(&format!(
                    "Found {} legacy {{key}} pattern(s):",
                    legacy_keys.len()
                ));
                for key in &legacy_keys {
                    println!("  • {{{key}}} → try <{key}> instead");
                }
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;

    /// Helper to build a minimal CliContext for testing
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

    /// Validate a valid template returns success
    #[test]
    fn validate_valid_template() {
        let ctx = test_ctx(false);
        let args = RuleArgs {
            action: RuleAction::Validate {
                template: "<Artist>\\<Album>\\<Title>".to_string(),
            },
        };
        let code = run(&ctx, &args).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    /// Validate an invalid template returns error
    #[test]
    fn validate_invalid_template() {
        let ctx = test_ctx(false);
        let args = RuleArgs {
            action: RuleAction::Validate {
                // Unclosed tag
                template: "<Artist".to_string(),
            },
        };
        let code = run(&ctx, &args).unwrap();
        assert_eq!(code, ExitCode::ERROR);
    }

    /// Tags listing returns success
    #[test]
    fn tags_listing() {
        let ctx = test_ctx(false);
        let args = RuleArgs {
            action: RuleAction::Tags,
        };
        let code = run(&ctx, &args).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    /// Tags listing in JSON mode
    #[test]
    fn tags_listing_json() {
        let ctx = test_ctx(true);
        let args = RuleArgs {
            action: RuleAction::Tags,
        };
        let code = run(&ctx, &args).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    /// Legacy detection with no legacy syntax
    #[test]
    fn legacy_no_matches() {
        let ctx = test_ctx(false);
        let args = RuleArgs {
            action: RuleAction::Legacy {
                template: "<Artist>\\<Title>".to_string(),
            },
        };
        let code = run(&ctx, &args).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    /// Legacy detection with legacy syntax found
    #[test]
    fn legacy_with_matches() {
        let ctx = test_ctx(false);
        let args = RuleArgs {
            action: RuleAction::Legacy {
                template: "{artist}\\{title}".to_string(),
            },
        };
        let code = run(&ctx, &args).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }
}
