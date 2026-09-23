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
///
/// On Linux this writes a systemd **user** unit; on macOS a launchd
/// **LaunchAgent**. Both run as the person who installed them, which matters:
/// they read that person's `settings.json5` and can see that person's files.
/// Both run `meedya watch --organize --yes`.
///
/// On Windows this deliberately refuses — see the explanation printed below.
fn install(ctx: &CliContext, bin_path: Option<&std::path::Path>) -> anyhow::Result<i32> {
    // ── Windows: honestly out of scope, and say why ─────────────────────
    //
    // `sc create` registers a *Windows Service*, and a Windows Service is not
    // just "a program Windows starts". It has to talk back to the Service
    // Control Manager within about thirty seconds of starting, to report that
    // it is running and to accept stop requests. `meedya` does not speak that
    // protocol, so Windows would decide it had hung and kill it.
    //
    // There is a second problem underneath the first. A service registered
    // this way runs as **LocalSystem**, which is a different account with a
    // different home directory — so it would load a different settings file
    // from the one the person configuring it can see, and would very likely
    // have no access to their media folders at all.
    //
    // Both are real engineering jobs, not one-line fixes, so rather than
    // installing something that quietly does not work, this says so.
    #[cfg(target_os = "windows")]
    {
        // Silence the unused-parameter warnings on this platform only.
        let _ = (ctx, bin_path);
        output::print_error(
            "Installing a background service is not available on Windows yet.\n\n\
             A program registered with `sc create` has to report back to the Windows \
             Service Control Manager within about thirty seconds of starting. \
             MeedyaManager does not speak that protocol, so Windows would stop it \
             again almost immediately. It would also run as the LocalSystem account, \
             which reads a different settings file from yours and may not be able to \
             reach your media folders at all.\n\n\
             Automatic organising itself is also switched off in this build while known \
             problems are fixed, so there is nothing to run in the background yet. Once it \
             is switched back on, the way to do this will be Task Scheduler, running\n\
             \x20   meedya watch --organize --yes\n\
             at logon. That runs as you and reads your settings.",
        );
        return Ok(ExitCode::NOT_IMPLEMENTED);
    }

    // ── Linux and macOS: actually install it ────────────────────────────
    #[cfg(not(target_os = "windows"))]
    {
        // Work out which binary the service should run. Defaulting to the
        // currently running executable is what makes `meedya service install`
        // work with no arguments — but the path is resolved *now* and baked
        // into the unit file, so moving or reinstalling `meedya` afterwards
        // means installing the service again.
        let resolved: std::path::PathBuf = match bin_path {
            Some(path) => path.to_path_buf(),
            None => match std::env::current_exe() {
                Ok(path) => path,
                Err(e) => {
                    output::print_error(&format!(
                        "Cannot work out where the meedya binary is: {e}. \
                         Pass --bin-path with the full path to it."
                    ));
                    return Ok(ExitCode::ERROR);
                }
            },
        };

        if ctx.dry_run {
            println!(
                "Dry-run: would register the MeedyaManager background service to run \n  \
                 '{} watch --organize --yes' automatically at login.",
                resolved.display()
            );
            if !super::watch::ORGANISING_SWITCHED_ON {
                println!(
                    "Note: a real install is switched off in this build, because automatic \
                     organising is switched off while known problems are fixed."
                );
            }
            return Ok(ExitCode::SUCCESS);
        }

        // ── Safety catch: switched off with organising (#180) ───────────
        //
        // The service runs `meedya watch --organize --yes`, and real
        // organising is switched off in this build (owner decision,
        // 2026-09-23 — see the matching block in `watch.rs`). Installing it
        // would register a service that refuses at every start, which
        // systemd and launchd would then restart over and over. So refuse
        // here too, before anything is written or registered. The dry-run
        // preview above is still allowed. Stage (g) of the fix plan turns
        // the shared switch back on.
        if !super::watch::ORGANISING_SWITCHED_ON {
            super::watch::print_switched_off(
                ctx.output,
                "Installing the background service is switched off in this build, because \
                 automatic organising is switched off while known problems are fixed. \
                 Nothing has been installed.\n\n\
                 If you installed the service from an earlier build, remove it with:\n\
                 \x20   meedya service uninstall",
            );
            return Ok(ExitCode::NOT_IMPLEMENTED);
        }

        match service::install_service(&resolved) {
            Ok(()) => {
                output::print_success(
                    "MeedyaManager background service installed and set to start at login.",
                );
                println!("  It runs: {} watch --organize --yes", resolved.display());
                println!("  To start it now without logging out: meedya service start");
                Ok(ExitCode::SUCCESS)
            }
            Err(e) => {
                output::print_error(&format!("Service install failed: {e}"));
                Ok(ExitCode::ERROR)
            }
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
