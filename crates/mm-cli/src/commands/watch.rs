// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya watch` Command
//
// Foreground file system watcher. Monitors directories for media file changes
// and logs events with timestamps. Optionally auto-organises files on change.

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use chrono::Local;
use clap::Args;
use serde::Serialize;
use std::path::PathBuf;

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
}

// ─── Command execution ─────────────────────────────────────────────────────

/// Execute the `meedya watch` command.
///
/// Starts a file system watcher and runs until Ctrl+C is pressed.
/// Events are printed to stdout with timestamps.
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
            });
        }
        OutputFormat::Human => {
            output::print_header("MeedyaManager — File Watcher");
            for folder in &folders {
                output::print_key_value("Watching", &folder.display().to_string());
            }
            output::print_key_value("Recursive", &(!args.no_recursive).to_string());
            output::print_key_value("Auto-organise", &args.organize.to_string());
            println!();
            output::print_success("Watcher started — press Ctrl+C to stop");
            println!();
        }
    }

    // Start the file system watcher
    let (_watcher, rx) = mm_core::watcher::start_watcher(&watcher_config)?;

    // Bridge the std::sync::mpsc receiver into the async world
    let output_format = ctx.output;
    let event_handle = tokio::task::spawn_blocking(move || {
        // Process events until the channel is closed
        while let Ok(event) = rx.recv() {
            let timestamp = Local::now().format("%H:%M:%S").to_string();
            let (event_type, path) = match &event {
                mm_core::watcher::WatchEvent::Created(p) => ("Created", p.display().to_string()),
                mm_core::watcher::WatchEvent::Modified(p) => ("Modified", p.display().to_string()),
                mm_core::watcher::WatchEvent::Deleted(p) => ("Deleted", p.display().to_string()),
                mm_core::watcher::WatchEvent::Renamed(from, to) => {
                    ("Renamed", format!("{} → {}", from.display(), to.display()))
                }
            };

            match output_format {
                OutputFormat::Json => {
                    let event_out = WatchEventOutput {
                        timestamp: timestamp.clone(),
                        event_type: event_type.to_string(),
                        path: path.clone(),
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
    });

    // Wait for Ctrl+C signal
    tokio::signal::ctrl_c().await?;

    if ctx.output == OutputFormat::Human {
        println!();
        output::print_success("Watcher stopped");
    }

    // The watcher is dropped here, which closes the channel and
    // allows the event_handle task to finish
    drop(_watcher);
    let _ = event_handle.await;

    Ok(ExitCode::SUCCESS)
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

    /// Watch returns error when no folders specified
    #[tokio::test]
    async fn watch_no_folders() {
        let mut ctx = test_ctx();
        ctx.config.watch.folders.clear();
        let args = WatchArgs {
            paths: vec![],
            no_recursive: false,
            organize: false,
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
        };
        assert_eq!(run(&ctx, &args).await.unwrap(), ExitCode::ERROR);
    }

    /// WatchArgs construction
    #[test]
    fn watch_args_construction() {
        let args = WatchArgs {
            paths: vec![PathBuf::from("/music"), PathBuf::from("/videos")],
            no_recursive: true,
            organize: true,
        };
        assert_eq!(args.paths.len(), 2);
        assert!(args.no_recursive);
        assert!(args.organize);
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
}
