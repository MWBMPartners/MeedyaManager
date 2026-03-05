// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya debug` Command
//
// Single-file metadata inspector. Shows classification, all tags, audio
// properties, cover art info, and companion files for a given media file.
// Supports both human-readable (coloured table) and JSON output.

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::Args;
use serde::Serialize;
use std::path::PathBuf;

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya debug` command.
#[derive(Args, Debug)]
pub struct DebugArgs {
    /// Path to the media file to inspect
    pub path: PathBuf,

    /// Show raw lofty tag names alongside canonical MeedyaManager names
    #[arg(long)]
    pub raw: bool,

    /// Extract and save embedded cover art to the specified file path
    #[arg(long, value_name = "OUTPUT_PATH")]
    pub cover: Option<PathBuf>,
}

// ─── JSON output structure ──────────────────────────────────────────────────

/// Complete debug output for JSON serialization.
#[derive(Serialize)]
struct DebugOutput {
    /// Path to the inspected file
    file: String,
    /// Media classification details
    classification: ClassificationInfo,
    /// All extracted metadata tags
    tags: std::collections::HashMap<String, Vec<String>>,
    /// Audio properties (if available)
    audio_properties: Option<AudioPropsInfo>,
    /// Cover art info (if embedded)
    cover_art: Option<CoverArtInfo>,
    /// Companion files found alongside this media file
    companions: Vec<CompanionInfo>,
}

/// Classification details for JSON output.
#[derive(Serialize)]
struct ClassificationInfo {
    group: String,
    format: String,
    class: String,
    quality: String,
}

/// Audio properties for JSON output.
#[derive(Serialize)]
struct AudioPropsInfo {
    duration_secs: f64,
    bitrate_kbps: Option<u32>,
    sample_rate_hz: Option<u32>,
    channels: Option<u8>,
    bits_per_sample: Option<u8>,
}

/// Cover art info for JSON output.
#[derive(Serialize)]
struct CoverArtInfo {
    size_bytes: usize,
    mime: String,
}

/// Companion file info for JSON output.
#[derive(Serialize)]
struct CompanionInfo {
    path: String,
    companion_type: String,
}

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya debug` command.
///
/// Inspects a single media file and displays all available metadata,
/// classification, audio properties, cover art info, and companion files.
///
/// # Returns
/// Exit code: 0 on success, 1 on error.
pub fn run(ctx: &CliContext, args: &DebugArgs) -> anyhow::Result<i32> {
    // Verify the file exists before attempting any operations
    if !args.path.exists() {
        output::print_error(&format!("File not found: {}", args.path.display()));
        return Ok(ExitCode::ERROR);
    }

    // ── 1. Classify the file ────────────────────────────────────────────
    let classification = mm_core::classify::classify_by_path(&args.path)
        .unwrap_or_else(|_| mm_core::classify::MediaClassification::unknown());

    // ── 2. Extract metadata tags ────────────────────────────────────────
    let tags = mm_core::metadata::extract_tags(&args.path).unwrap_or_default();

    // ── 3. Extract audio properties ─────────────────────────────────────
    let audio_props = mm_core::metadata::extract_audio_properties(&args.path).ok();

    // ── 4. Check for cover art ──────────────────────────────────────────
    let cover_art = mm_core::metadata::extract_cover_art(&args.path)
        .ok()
        .flatten();

    // ── 5. Find companion files ─────────────────────────────────────────
    let companions = mm_core::companion::find_companions(&args.path).unwrap_or_default();

    // ── 6. Handle --cover flag (save cover art to file) ─────────────────
    if let Some(ref output_path) = args.cover {
        if let Some(ref art) = cover_art {
            // Write the raw cover art bytes to the specified file
            std::fs::write(output_path, &art.data).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to write cover art to '{}': {}",
                    output_path.display(),
                    e
                )
            })?;
            output::print_success(&format!(
                "Cover art saved to {} ({} bytes)",
                output_path.display(),
                art.data.len()
            ));
        } else {
            output::print_warning("No embedded cover art found in this file");
        }
    }

    // ── 7. Render output ────────────────────────────────────────────────
    match ctx.output {
        OutputFormat::Json => render_json(
            &args.path,
            &classification,
            &tags,
            &audio_props,
            &cover_art,
            &companions,
        ),
        OutputFormat::Human => render_human(
            &args.path,
            &classification,
            &tags,
            &audio_props,
            &cover_art,
            &companions,
            args.raw,
        ),
    }

    Ok(ExitCode::SUCCESS)
}

// ─── JSON renderer ──────────────────────────────────────────────────────────

/// Render all debug info as a single JSON object.
fn render_json(
    path: &std::path::Path,
    classification: &mm_core::classify::MediaClassification,
    tags: &mm_core::metadata::TagMap,
    audio_props: &Option<mm_core::metadata::AudioProperties>,
    cover_art: &Option<mm_core::metadata::CoverArt>,
    companions: &[mm_core::companion::CompanionFile],
) {
    // Build the JSON output structure
    let output = DebugOutput {
        file: path.display().to_string(),
        classification: ClassificationInfo {
            group: format!("{:?}", classification.group),
            format: format!("{:?}", classification.format),
            class: format!("{:?}", classification.class),
            quality: format!("{:?}", classification.quality),
        },
        tags: tags.clone(),
        audio_properties: audio_props.as_ref().map(|p| AudioPropsInfo {
            duration_secs: p.duration_secs,
            bitrate_kbps: p.bitrate_kbps,
            sample_rate_hz: p.sample_rate_hz,
            channels: p.channels,
            bits_per_sample: p.bits_per_sample,
        }),
        cover_art: cover_art.as_ref().map(|a| CoverArtInfo {
            size_bytes: a.data.len(),
            mime: a.mime.clone(),
        }),
        companions: companions
            .iter()
            .map(|c| CompanionInfo {
                path: c.path.display().to_string(),
                companion_type: format!("{:?}", c.companion_type),
            })
            .collect(),
    };

    output::print_json(&output);
}

// ─── Human renderer ─────────────────────────────────────────────────────────

/// Render all debug info as coloured, human-readable tables.
#[allow(clippy::too_many_arguments)]
fn render_human(
    path: &std::path::Path,
    classification: &mm_core::classify::MediaClassification,
    tags: &mm_core::metadata::TagMap,
    audio_props: &Option<mm_core::metadata::AudioProperties>,
    cover_art: &Option<mm_core::metadata::CoverArt>,
    companions: &[mm_core::companion::CompanionFile],
    _raw: bool,
) {
    // File path header
    output::print_header(&format!("File: {}", path.display()));

    // ── Classification ──────────────────────────────────────────────────
    output::print_header("Classification");
    output::print_key_value("Group", &format!("{:?}", classification.group));
    output::print_key_value("Format", &format!("{:?}", classification.format));
    output::print_key_value("Class", &format!("{:?}", classification.class));
    output::print_key_value("Quality", &format!("{:?}", classification.quality));

    // ── Tags ────────────────────────────────────────────────────────────
    output::print_header("Metadata Tags");
    if tags.is_empty() {
        println!("  (no tags found)");
    } else {
        // Sort tags alphabetically for consistent output
        let mut sorted_tags: Vec<_> = tags.iter().collect();
        sorted_tags.sort_by_key(|(k, _)| (*k).clone());

        let rows: Vec<Vec<String>> = sorted_tags
            .iter()
            .map(|(key, values)| vec![key.to_string(), mm_core::metadata::join_multi_value(values)])
            .collect();
        output::print_table(&["Tag", "Value"], &rows);
    }

    // ── Audio Properties ────────────────────────────────────────────────
    if let Some(props) = audio_props {
        output::print_header("Audio Properties");
        // Format duration as M:SS or H:MM:SS
        let duration = if props.duration_secs >= 3600.0 {
            let h = (props.duration_secs / 3600.0) as u32;
            let m = ((props.duration_secs % 3600.0) / 60.0) as u32;
            let s = (props.duration_secs % 60.0) as u32;
            format!("{h}:{m:02}:{s:02}")
        } else {
            let m = (props.duration_secs / 60.0) as u32;
            let s = (props.duration_secs % 60.0) as u32;
            format!("{m}:{s:02}")
        };
        output::print_key_value("Duration", &duration);
        if let Some(bitrate) = props.bitrate_kbps {
            output::print_key_value("Bitrate", &format!("{bitrate} kbps"));
        }
        if let Some(sample_rate) = props.sample_rate_hz {
            output::print_key_value("Sample Rate", &format!("{sample_rate} Hz"));
        }
        if let Some(channels) = props.channels {
            output::print_key_value("Channels", &format!("{channels}"));
        }
        if let Some(bps) = props.bits_per_sample {
            output::print_key_value("Bit Depth", &format!("{bps}-bit"));
        }
    }

    // ── Cover Art ───────────────────────────────────────────────────────
    output::print_header("Cover Art");
    if let Some(art) = cover_art {
        output::print_key_value("Embedded", "Yes");
        output::print_key_value("Size", &format!("{} bytes", art.data.len()));
        output::print_key_value("MIME Type", &art.mime);
    } else {
        output::print_key_value("Embedded", "No");
    }

    // ── Companion Files ─────────────────────────────────────────────────
    output::print_header("Companion Files");
    if companions.is_empty() {
        println!("  (none found)");
    } else {
        let rows: Vec<Vec<String>> = companions
            .iter()
            .map(|c| {
                vec![
                    c.path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    format!("{:?}", c.companion_type),
                ]
            })
            .collect();
        output::print_table(&["File", "Type"], &rows);
    }
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

    /// Debug command returns error exit code for missing files
    #[test]
    fn missing_file_returns_error() {
        let ctx = test_ctx(false);
        let args = DebugArgs {
            path: PathBuf::from("/nonexistent/file.mp3"),
            raw: false,
            cover: None,
        };
        let code = run(&ctx, &args).unwrap();
        assert_eq!(code, ExitCode::ERROR);
    }

    /// Debug command succeeds for an existing non-media file (shows Unknown classification)
    #[test]
    fn existing_non_media_file() {
        // Use Cargo.toml as a test file — it exists but isn't media
        let ctx = test_ctx(false);
        let args = DebugArgs {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            raw: false,
            cover: None,
        };
        let code = run(&ctx, &args).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    /// Debug command works in JSON mode without panicking
    #[test]
    fn json_output_mode() {
        let ctx = test_ctx(true);
        let args = DebugArgs {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            raw: false,
            cover: None,
        };
        let code = run(&ctx, &args).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    /// Debug command handles --cover flag when no cover art exists
    #[test]
    fn cover_extraction_no_art() {
        let ctx = test_ctx(false);
        let args = DebugArgs {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            raw: false,
            cover: Some(PathBuf::from("/tmp/test_cover.jpg")),
        };
        // Should succeed but print a warning about no cover art
        let code = run(&ctx, &args).unwrap();
        assert_eq!(code, ExitCode::SUCCESS);
    }

    /// DebugArgs can be constructed programmatically for testing
    #[test]
    fn debug_args_construction() {
        let args = DebugArgs {
            path: PathBuf::from("/test/file.flac"),
            raw: true,
            cover: Some(PathBuf::from("/tmp/art.png")),
        };
        assert!(args.raw);
        assert!(args.cover.is_some());
    }
}
