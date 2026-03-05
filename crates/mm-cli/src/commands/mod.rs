// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — CLI Command Modules
//
// Each subcommand is implemented in its own module. All modules expose a
// `run()` function that takes a `CliContext` reference and the command's
// parsed arguments, returning an `anyhow::Result<i32>` (exit code).

/// `meedya debug <file>` — Single-file metadata inspector
pub mod debug;

/// `meedya rule <action>` — Template validation, tag listing, test with file
pub mod rule;

/// `meedya config <action>` — Configuration management (show/path/init/export/import)
pub mod config_cmd;

/// `meedya scan <path>` — Directory scan with rename preview
pub mod scan;

/// `meedya edit <file>` — Metadata editor (set/remove tags, cover art)
pub mod edit;

/// `meedya watch [paths]` — Foreground file system watcher
pub mod watch;

/// `meedya lookup <query>` — Provider search (stub — providers come in M5)
pub mod lookup;

/// `meedya report-bug` — System info + log collection for bug reports
pub mod report_bug;

/// `meedya export` — Export media library to a relational database (M9)
pub mod export;

/// `meedya serve` — Start the HTTPS media server with JWT auth (M10)
pub mod serve;
