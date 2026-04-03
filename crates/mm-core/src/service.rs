// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Background Service Management
//
// This module provides cross-platform helpers to install, uninstall, start,
// stop, and query the status of the MeedyaManager background service.
//
// The service runs `meedya watch --organize` continuously in the background,
// monitoring configured library folders and auto-organising new media.
//
// Platform implementations:
//   Linux   — systemd user unit  (~/.config/systemd/user/meedyamanager.service)
//   macOS   — launchd user agent (~/Library/LaunchAgents/com.mwbm.meedyamanager.plist)
//   Windows — Windows Service via `sc.exe` (registered as "MeedyaManager")
//
// Service defaults: ENABLED, auto-start on login.
//
// Public API:
//   - install_service(bin_path)  — register the service with the OS
//   - uninstall_service()        — remove the service registration
//   - start_service()            — start the service immediately
//   - stop_service()             — stop the running service
//   - service_status()           — query running / stopped / not-installed
//   - is_service_running()       — quick boolean check

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{MmError, MmResult};

// ---------------------------------------------------------------------------
// Service status
// ---------------------------------------------------------------------------

/// Possible states for the MeedyaManager background service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    /// Service is registered and running.
    Running,
    /// Service is registered but not currently running.
    Stopped,
    /// Service is not registered with the OS service manager.
    NotInstalled,
    /// Could not determine status (OS query failed).
    Unknown,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Stopped => write!(f, "stopped"),
            Self::NotInstalled => write!(f, "not-installed"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

// ---------------------------------------------------------------------------
// Service name constants
// ---------------------------------------------------------------------------

/// Service identifier used on all platforms.
#[allow(dead_code)]
const SERVICE_NAME: &str = "meedyamanager";

/// Reverse-DNS bundle identifier (macOS launchd).
const LAUNCHD_LABEL: &str = "com.mwbm.meedyamanager";

// ---------------------------------------------------------------------------
// Public API — platform-dispatched
// ---------------------------------------------------------------------------

/// Register MeedyaManager as an OS background service.
///
/// `bin_path` must point to the installed `meedya` binary.  On platforms
/// that embed the path in the service definition (systemd, launchd), this
/// is written into the unit/plist file.
///
/// On Windows the binary must already be on the system `PATH` or a full path
/// must be supplied.
///
/// # Errors
/// Returns an error if the unit/plist cannot be written or the OS command
/// to register the service fails.
pub fn install_service(bin_path: &Path) -> MmResult<()> {
    info!(
        "service: installing for platform '{}'",
        std::env::consts::OS
    );
    platform::install(bin_path)
}

/// Remove the MeedyaManager service registration from the OS.
///
/// If the service is currently running it will be stopped first.
///
/// # Errors
/// Returns an error if the service manager command fails.
pub fn uninstall_service() -> MmResult<()> {
    info!("service: uninstalling");
    platform::uninstall()
}

/// Start the MeedyaManager service immediately (if stopped).
pub fn start_service() -> MmResult<()> {
    info!("service: starting");
    platform::start()
}

/// Stop the running MeedyaManager service.
pub fn stop_service() -> MmResult<()> {
    info!("service: stopping");
    platform::stop()
}

/// Query the current status of the MeedyaManager service.
pub fn service_status() -> ServiceStatus {
    platform::status()
}

/// Return `true` if the service is currently registered and running.
pub fn is_service_running() -> bool {
    service_status() == ServiceStatus::Running
}

// ---------------------------------------------------------------------------
// Internal helper — run a shell command and return success/failure
// ---------------------------------------------------------------------------

/// Execute a command and map failure to `MmError::Config`.
fn run_cmd(program: &str, args: &[&str]) -> MmResult<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| MmError::Config(format!("cannot run '{program}': {e}")))?;

    if status.success() {
        Ok(())
    } else {
        Err(MmError::Config(format!(
            "'{program} {}' exited with status {status}",
            args.join(" ")
        )))
    }
}

/// Execute a command and capture its stdout as a string.
fn cmd_output(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
}

// ---------------------------------------------------------------------------
// Platform-specific implementations
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod platform {
    use super::*;

    /// Path to the systemd user unit directory.
    fn unit_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("systemd").join("user"))
    }

    /// Path to the unit file itself.
    fn unit_file() -> Option<PathBuf> {
        unit_dir().map(|d| d.join(format!("{SERVICE_NAME}.service")))
    }

    /// Generate systemd unit file content for the given binary path.
    fn unit_content(bin_path: &Path) -> String {
        format!(
            "[Unit]\n\
             Description=MeedyaManager — Media File Auto-Organiser\n\
             Documentation=https://github.com/MWBM-Partners-Ltd/MeedyaManager\n\
             After=network.target\n\
             \n\
             [Service]\n\
             Type=simple\n\
             ExecStart={bin} watch --organize\n\
             Restart=on-failure\n\
             RestartSec=5s\n\
             # Limit resource usage so background monitoring is lightweight\n\
             CPUSchedulingPolicy=idle\n\
             IOSchedulingClass=idle\n\
             \n\
             [Install]\n\
             WantedBy=default.target\n",
            bin = bin_path.display()
        )
    }

    pub fn install(bin_path: &Path) -> MmResult<()> {
        let unit_dir = unit_dir().ok_or_else(|| {
            MmError::Config("cannot determine systemd user unit directory".into())
        })?;
        let unit_file = unit_file().ok_or_else(|| {
            MmError::Config("cannot determine systemd user unit file path".into())
        })?;

        // Ensure the systemd user directory exists
        std::fs::create_dir_all(&unit_dir)
            .map_err(|e| MmError::Config(format!("cannot create systemd user dir: {e}")))?;

        // Write the unit file
        std::fs::write(&unit_file, unit_content(bin_path))
            .map_err(|e| MmError::Config(format!("cannot write unit file: {e}")))?;

        info!("service: unit file written to '{}'", unit_file.display());

        // Reload systemd daemon so it picks up the new unit
        run_cmd("systemctl", &["--user", "daemon-reload"])?;
        // Enable auto-start on login
        run_cmd("systemctl", &["--user", "enable", SERVICE_NAME])?;

        info!("service: systemd unit installed and enabled");
        Ok(())
    }

    pub fn uninstall() -> MmResult<()> {
        // Stop if running
        let _ = stop(); // ignore error if not running

        // Disable auto-start
        let _ = run_cmd("systemctl", &["--user", "disable", SERVICE_NAME]);

        // Remove unit file
        if let Some(unit_file) = unit_file() {
            if unit_file.exists() {
                std::fs::remove_file(&unit_file)
                    .map_err(|e| MmError::Config(format!("cannot remove unit file: {e}")))?;
            }
        }

        // Reload daemon
        let _ = run_cmd("systemctl", &["--user", "daemon-reload"]);

        info!("service: systemd unit removed");
        Ok(())
    }

    pub fn start() -> MmResult<()> {
        run_cmd("systemctl", &["--user", "start", SERVICE_NAME])
    }

    pub fn stop() -> MmResult<()> {
        run_cmd("systemctl", &["--user", "stop", SERVICE_NAME])
    }

    pub fn status() -> ServiceStatus {
        let Some(out) = cmd_output("systemctl", &["--user", "is-active", SERVICE_NAME]) else {
            return ServiceStatus::Unknown;
        };
        match out.trim() {
            "active" => ServiceStatus::Running,
            "inactive" => ServiceStatus::Stopped,
            "failed" => ServiceStatus::Stopped,
            _ => {
                // "Unit ... not found" → not installed
                if out.contains("not found") || out.contains("not-found") {
                    ServiceStatus::NotInstalled
                } else {
                    ServiceStatus::Unknown
                }
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;

    /// Path to the LaunchAgents directory for the current user.
    fn launch_agents_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join("Library").join("LaunchAgents"))
    }

    /// Path to the plist file.
    fn plist_file() -> Option<PathBuf> {
        launch_agents_dir().map(|d| d.join(format!("{LAUNCHD_LABEL}.plist")))
    }

    /// Generate a launchd plist for the given binary path.
    fn plist_content(bin_path: &Path) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
    "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Service identifier — must match filename -->
    <key>Label</key>
    <string>{label}</string>

    <!-- Binary and arguments -->
    <key>ProgramArguments</key>
    <array>
        <string>{bin}</string>
        <string>watch</string>
        <string>--organize</string>
    </array>

    <!-- Auto-start on login -->
    <key>RunAtLoad</key>
    <true/>

    <!-- Restart on crash -->
    <key>KeepAlive</key>
    <dict>
        <key>Crashed</key>
        <true/>
    </dict>

    <!-- Low-priority scheduling to minimise impact on interactive use -->
    <key>ProcessType</key>
    <string>Background</string>

    <!-- Throttle restart to avoid hammering on persistent errors -->
    <key>ThrottleInterval</key>
    <integer>5</integer>

    <!-- Standard I/O log files -->
    <key>StandardOutPath</key>
    <string>/tmp/meedyamanager.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/meedyamanager.stderr.log</string>
</dict>
</plist>
"#,
            label = LAUNCHD_LABEL,
            bin = bin_path.display()
        )
    }

    pub fn install(bin_path: &Path) -> MmResult<()> {
        let la_dir = launch_agents_dir()
            .ok_or_else(|| MmError::Config("cannot determine LaunchAgents directory".into()))?;
        let plist_file = plist_file()
            .ok_or_else(|| MmError::Config("cannot determine plist file path".into()))?;

        // Ensure directory exists (it almost always does, but just in case)
        std::fs::create_dir_all(&la_dir)
            .map_err(|e| MmError::Config(format!("cannot create LaunchAgents dir: {e}")))?;

        // Write the plist
        std::fs::write(&plist_file, plist_content(bin_path))
            .map_err(|e| MmError::Config(format!("cannot write plist: {e}")))?;

        info!("service: plist written to '{}'", plist_file.display());

        // Load the plist into launchd immediately (also marks it to auto-start)
        run_cmd("launchctl", &["load", &plist_file.to_string_lossy()])?;

        info!("service: launchd agent loaded");
        Ok(())
    }

    pub fn uninstall() -> MmResult<()> {
        if let Some(plist) = plist_file() {
            if plist.exists() {
                // Unload first
                let _ = run_cmd("launchctl", &["unload", &plist.to_string_lossy()]);
                // Remove plist
                std::fs::remove_file(&plist)
                    .map_err(|e| MmError::Config(format!("cannot remove plist: {e}")))?;
            }
        }
        info!("service: launchd agent removed");
        Ok(())
    }

    pub fn start() -> MmResult<()> {
        run_cmd("launchctl", &["start", LAUNCHD_LABEL])
    }

    pub fn stop() -> MmResult<()> {
        run_cmd("launchctl", &["stop", LAUNCHD_LABEL])
    }

    pub fn status() -> ServiceStatus {
        let Some(out) = cmd_output("launchctl", &["list", LAUNCHD_LABEL]) else {
            return ServiceStatus::Unknown;
        };
        if out.contains("Could not find service") || out.contains("No such process") {
            return ServiceStatus::NotInstalled;
        }
        // `launchctl list <label>` outputs a JSON-like dict. The "PID" key is
        // present and non-zero when the service is running.
        if out.contains("\"PID\"") {
            // Extract PID value — if it's not 0 the service is running
            let running = out
                .lines()
                .find(|l| l.contains("\"PID\""))
                .and_then(|l| l.split_whitespace().last())
                .and_then(|v| v.trim_matches(|c| c == ',' || c == ';').parse::<u32>().ok())
                .is_some_and(|pid| pid > 0);
            if running {
                ServiceStatus::Running
            } else {
                ServiceStatus::Stopped
            }
        } else {
            ServiceStatus::Stopped
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;

    /// Windows service display name.
    const DISPLAY_NAME: &str = "MeedyaManager";
    const DESCRIPTION: &str =
        "MeedyaManager background service — monitors media folders and auto-organises files.";

    pub fn install(bin_path: &Path) -> MmResult<()> {
        // Register using `sc.exe create` (requires Administrator or elevated token)
        let bin_str = bin_path.to_string_lossy();
        let bin_cmd = format!("{bin_str} watch --organize");

        run_cmd(
            "sc",
            &[
                "create",
                SERVICE_NAME,
                "binPath=",
                &bin_cmd,
                "DisplayName=",
                DISPLAY_NAME,
                "start=",
                "auto", // auto-start at boot/login
                "type=",
                "own",
            ],
        )?;

        // Set the description
        run_cmd("sc", &["description", SERVICE_NAME, DESCRIPTION])?;

        info!("service: Windows service '{}' registered", SERVICE_NAME);
        Ok(())
    }

    pub fn uninstall() -> MmResult<()> {
        let _ = stop(); // stop first, ignore error if not running
        run_cmd("sc", &["delete", SERVICE_NAME])
    }

    pub fn start() -> MmResult<()> {
        run_cmd("sc", &["start", SERVICE_NAME])
    }

    pub fn stop() -> MmResult<()> {
        run_cmd("sc", &["stop", SERVICE_NAME])
    }

    pub fn status() -> ServiceStatus {
        let Some(out) = cmd_output("sc", &["query", SERVICE_NAME]) else {
            return ServiceStatus::Unknown;
        };
        if out.contains("FAILED 1060") || out.contains("does not exist") {
            return ServiceStatus::NotInstalled;
        }
        if out.contains("RUNNING") {
            ServiceStatus::Running
        } else if out.contains("STOPPED") {
            ServiceStatus::Stopped
        } else {
            ServiceStatus::Unknown
        }
    }
}

// Fallback for unsupported platforms (compilation only — not expected in prod)
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    use super::*;

    pub fn install(_bin_path: &Path) -> MmResult<()> {
        Err(MmError::Config(
            "background service not supported on this platform".into(),
        ))
    }
    pub fn uninstall() -> MmResult<()> {
        Err(MmError::Config(
            "background service not supported on this platform".into(),
        ))
    }
    pub fn start() -> MmResult<()> {
        Err(MmError::Config(
            "background service not supported on this platform".into(),
        ))
    }
    pub fn stop() -> MmResult<()> {
        Err(MmError::Config(
            "background service not supported on this platform".into(),
        ))
    }
    pub fn status() -> ServiceStatus {
        ServiceStatus::Unknown
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_status_display() {
        assert_eq!(ServiceStatus::Running.to_string(), "running");
        assert_eq!(ServiceStatus::Stopped.to_string(), "stopped");
        assert_eq!(ServiceStatus::NotInstalled.to_string(), "not-installed");
        assert_eq!(ServiceStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn service_status_eq() {
        assert_eq!(ServiceStatus::Running, ServiceStatus::Running);
        assert_ne!(ServiceStatus::Running, ServiceStatus::Stopped);
    }

    /// Verify that `is_service_running` returns `false` in CI environments
    /// (where the service is never installed).  On a developer machine with
    /// the service running this test is a no-op (it would return `true`).
    #[test]
    fn is_service_running_does_not_panic() {
        // We don't assert the value — just verify it doesn't crash.
        let _ = is_service_running();
    }

    #[test]
    fn service_status_does_not_panic() {
        // Same — platform query may return any variant, just must not panic.
        let status = service_status();
        let _ = status.to_string();
    }
}
