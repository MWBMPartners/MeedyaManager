// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — `meedya service` Command
//
// Manages the MeedyaManager background service (systemd / launchd / Windows
// Service).  Delegates to `mm_core::service` for OS-level operations.
//
// Subcommands:
//   meedya service install  — register the background service with the OS
//   meedya service uninstall — remove the background service registration
//   meedya service start    — start the service
//   meedya service stop     — stop the service
//   meedya service status   — query whether the service is running

use crate::context::CliContext;
use crate::output::{self, ExitCode, OutputFormat};
use clap::{Args, Subcommand};
use mm_core::service::{self, ServiceStatus};
use std::path::PathBuf;

// ─── Command arguments ─────────────────────────────────────────────────────

/// Arguments for the `meedya service` command.
#[derive(Args, Debug)]
pub struct ServiceArgs {
    /// Service action to perform
    #[command(subcommand)]
    pub action: ServiceAction,
}

/// Available service actions.
#[derive(Subcommand, Debug)]
pub enum ServiceAction {
    /// Register MeedyaManager as a background service (systemd / launchd / Windows Service)
    Install {
        /// Path to the meedya binary.
        /// Defaults to the currently running executable.
        #[arg(long)]
        bin_path: Option<PathBuf>,
    },
    /// Remove the MeedyaManager background service registration
    Uninstall,
    /// Start the background service immediately
    Start,
    /// Stop the background service
    Stop,
    /// Display the current background service status
    Status,
}

// ─── Command execution ──────────────────────────────────────────────────────

/// Execute the `meedya service` command.
pub fn run(ctx: &CliContext, args: &ServiceArgs) -> anyhow::Result<i32> {
    match &args.action {
        ServiceAction::Install { bin_path } => install(ctx, bin_path.as_deref()),
        ServiceAction::Uninstall => uninstall(ctx),
        ServiceAction::Start => start(ctx),
        ServiceAction::Stop => stop(ctx),
        ServiceAction::Status => status(ctx),
    }
}

// ─── Subcommand handlers ────────────────────────────────────────────────────

/// Install the background service with the OS service manager.
fn install(ctx: &CliContext, bin_path: Option<&std::path::Path>) -> anyhow::Result<i32> {
    // Resolve the binary path: use the supplied path, or the current executable
    let resolved_bin = match bin_path {
        Some(p) => p.to_path_buf(),
        None => std::env::current_exe()
            .map_err(|e| anyhow::anyhow!("cannot determine current executable: {e}"))?,
    };

    if ctx.dry_run {
        println!(
            "Dry-run: would install MeedyaManager service (binary: {})",
            resolved_bin.display()
        );
        return Ok(ExitCode::SUCCESS);
    }

    match service::install_service(&resolved_bin) {
        Ok(()) => {
            output::print_success("MeedyaManager background service installed and enabled.");
            println!("  The service will start automatically at next login.");
            println!("  To start immediately: meedya service start");
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            output::print_error(&format!("Service install failed: {e}"));
            Ok(ExitCode::ERROR)
        }
    }
}

/// Uninstall the background service.
fn uninstall(ctx: &CliContext) -> anyhow::Result<i32> {
    if ctx.dry_run {
        println!("Dry-run: would uninstall MeedyaManager background service.");
        return Ok(ExitCode::SUCCESS);
    }

    match service::uninstall_service() {
        Ok(()) => {
            output::print_success("MeedyaManager background service removed.");
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            output::print_error(&format!("Service uninstall failed: {e}"));
            Ok(ExitCode::ERROR)
        }
    }
}

/// Start the background service.
fn start(_ctx: &CliContext) -> anyhow::Result<i32> {
    match service::start_service() {
        Ok(()) => {
            output::print_success("MeedyaManager background service started.");
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            output::print_error(&format!("Service start failed: {e}"));
            Ok(ExitCode::ERROR)
        }
    }
}

/// Stop the background service.
fn stop(_ctx: &CliContext) -> anyhow::Result<i32> {
    match service::stop_service() {
        Ok(()) => {
            output::print_success("MeedyaManager background service stopped.");
            Ok(ExitCode::SUCCESS)
        }
        Err(e) => {
            output::print_error(&format!("Service stop failed: {e}"));
            Ok(ExitCode::ERROR)
        }
    }
}

/// Query and display the service status.
fn status(ctx: &CliContext) -> anyhow::Result<i32> {
    let s = service::service_status();

    if ctx.output == OutputFormat::Json {
        // Machine-readable JSON output
        let json = serde_json::json!({
            "service": "meedyamanager",
            "status": s.to_string(),
            "running": s == ServiceStatus::Running,
        });
        println!("{json}");
    } else {
        // Human-readable output with colour coding
        match s {
            ServiceStatus::Running => {
                output::print_success("MeedyaManager background service: RUNNING");
            }
            ServiceStatus::Stopped => {
                output::print_warning("MeedyaManager background service: STOPPED");
                println!("  To start: meedya service start");
            }
            ServiceStatus::NotInstalled => {
                output::print_warning("MeedyaManager background service: NOT INSTALLED");
                println!("  To install: meedya service install");
            }
            ServiceStatus::Unknown => {
                output::print_warning("MeedyaManager background service: UNKNOWN");
                println!("  Could not query the OS service manager.");
            }
        }
    }

    // Exit code: 0 if running, 1 if not running / not installed / unknown
    if s == ServiceStatus::Running {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::ERROR)
    }
}
