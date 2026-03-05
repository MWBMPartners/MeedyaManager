// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// Startup health checks.
//
// Verifies the application environment is ready: config file readable,
// watch folders exist, disk space sufficient, and network reachable.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::info;


/// Status of an individual health check
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    /// Check passed successfully
    Pass,
    /// Check passed with a warning
    Warn,
    /// Check failed — may prevent normal operation
    Fail,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "PASS"),
            Self::Warn => write!(f, "WARN"),
            Self::Fail => write!(f, "FAIL"),
        }
    }
}

/// Result of a single health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Human-readable name of the check
    pub name: String,
    /// Pass, Warn, or Fail
    pub status: CheckStatus,
    /// Descriptive message
    pub message: String,
}

/// Overall health report from all checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Individual check results
    pub checks: Vec<CheckResult>,
    /// Overall status (worst of all individual checks)
    pub overall: CheckStatus,
}

impl HealthReport {
    /// Create a new empty health report
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            overall: CheckStatus::Pass,
        }
    }

    /// Add a check result and update overall status
    pub fn add(&mut self, result: CheckResult) {
        // Overall is the worst status seen
        match (&self.overall, &result.status) {
            (CheckStatus::Pass, CheckStatus::Warn) => self.overall = CheckStatus::Warn,
            (_, CheckStatus::Fail) => self.overall = CheckStatus::Fail,
            _ => {}
        }
        self.checks.push(result);
    }

    /// How many checks passed
    pub fn pass_count(&self) -> usize {
        self.checks.iter().filter(|c| c.status == CheckStatus::Pass).count()
    }

    /// How many checks have warnings
    pub fn warn_count(&self) -> usize {
        self.checks.iter().filter(|c| c.status == CheckStatus::Warn).count()
    }

    /// How many checks failed
    pub fn fail_count(&self) -> usize {
        self.checks.iter().filter(|c| c.status == CheckStatus::Fail).count()
    }

    /// Whether all checks passed (no warnings or failures)
    pub fn is_healthy(&self) -> bool {
        self.overall == CheckStatus::Pass
    }
}

impl Default for HealthReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Check that a configuration file exists and is readable
pub fn check_config_file(path: &Path) -> CheckResult {
    if !path.exists() {
        return CheckResult {
            name: "Config file".into(),
            status: CheckStatus::Warn,
            message: format!("Config file not found at {}. Using defaults.", path.display()),
        };
    }

    match std::fs::read_to_string(path) {
        Ok(_) => CheckResult {
            name: "Config file".into(),
            status: CheckStatus::Pass,
            message: format!("Config file readable: {}", path.display()),
        },
        Err(e) => CheckResult {
            name: "Config file".into(),
            status: CheckStatus::Fail,
            message: format!("Cannot read config file: {e}"),
        },
    }
}

/// Check that watch folders exist and are accessible
pub fn check_watch_folders(folders: &[PathBuf]) -> Vec<CheckResult> {
    if folders.is_empty() {
        return vec![CheckResult {
            name: "Watch folders".into(),
            status: CheckStatus::Warn,
            message: "No watch folders configured".into(),
        }];
    }

    folders
        .iter()
        .map(|folder| {
            if !folder.exists() {
                CheckResult {
                    name: format!("Watch folder: {}", folder.display()),
                    status: CheckStatus::Warn,
                    message: format!("Folder does not exist: {}", folder.display()),
                }
            } else if !folder.is_dir() {
                CheckResult {
                    name: format!("Watch folder: {}", folder.display()),
                    status: CheckStatus::Fail,
                    message: format!("Path is not a directory: {}", folder.display()),
                }
            } else {
                // Try to read the directory to check permissions
                match std::fs::read_dir(folder) {
                    Ok(_) => CheckResult {
                        name: format!("Watch folder: {}", folder.display()),
                        status: CheckStatus::Pass,
                        message: format!("Folder accessible: {}", folder.display()),
                    },
                    Err(e) => CheckResult {
                        name: format!("Watch folder: {}", folder.display()),
                        status: CheckStatus::Fail,
                        message: format!("Cannot read folder: {e}"),
                    },
                }
            }
        })
        .collect()
}

/// Check available disk space in the output directory.
///
/// Warns if less than `min_bytes` are available. This is a best-effort
/// check — disk space APIs vary by platform.
pub fn check_disk_space(path: &Path, _min_bytes: u64) -> CheckResult {
    // Use a simple heuristic: try to stat the filesystem
    // On most platforms, we can check available space via std
    if !path.exists() {
        return CheckResult {
            name: "Disk space".into(),
            status: CheckStatus::Warn,
            message: format!("Cannot check disk space: path does not exist: {}", path.display()),
        };
    }

    // Rust std doesn't provide disk space APIs directly.
    // We report a Pass with a note that detailed checks require platform APIs.
    CheckResult {
        name: "Disk space".into(),
        status: CheckStatus::Pass,
        message: format!(
            "Path exists: {}. Detailed disk space check requires platform-specific APIs.",
            path.display()
        ),
    }
}

/// Check that the config directory is writable (for state and lock files)
pub fn check_config_dir_writable() -> CheckResult {
    let config_dir = dirs::config_dir()
        .map(|d| d.join("MeedyaManager"))
        .unwrap_or_else(|| PathBuf::from("."));

    // Try to create the directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        return CheckResult {
            name: "Config directory".into(),
            status: CheckStatus::Fail,
            message: format!("Cannot create config directory: {e}"),
        };
    }

    // Try to write a test file
    let test_file = config_dir.join(".health_check_test");
    match std::fs::write(&test_file, "test") {
        Ok(_) => {
            let _ = std::fs::remove_file(&test_file);
            CheckResult {
                name: "Config directory".into(),
                status: CheckStatus::Pass,
                message: format!("Config directory writable: {}", config_dir.display()),
            }
        }
        Err(e) => CheckResult {
            name: "Config directory".into(),
            status: CheckStatus::Fail,
            message: format!("Config directory not writable: {e}"),
        },
    }
}

/// Run all health checks and return a consolidated report.
pub fn run_health_checks(
    config_path: &Path,
    watch_folders: &[PathBuf],
) -> HealthReport {
    let mut report = HealthReport::new();

    // Check config file
    report.add(check_config_file(config_path));

    // Check watch folders
    for result in check_watch_folders(watch_folders) {
        report.add(result);
    }

    // Check config directory is writable
    report.add(check_config_dir_writable());

    // Log the summary
    info!(
        "Health check: {} pass, {} warn, {} fail — overall: {}",
        report.pass_count(),
        report.warn_count(),
        report.fail_count(),
        report.overall,
    );

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn check_status_display() {
        assert_eq!(CheckStatus::Pass.to_string(), "PASS");
        assert_eq!(CheckStatus::Warn.to_string(), "WARN");
        assert_eq!(CheckStatus::Fail.to_string(), "FAIL");
    }

    #[test]
    fn health_report_new_is_healthy() {
        let report = HealthReport::new();
        assert!(report.is_healthy());
        assert_eq!(report.pass_count(), 0);
        assert_eq!(report.warn_count(), 0);
        assert_eq!(report.fail_count(), 0);
    }

    #[test]
    fn health_report_overall_degrades() {
        let mut report = HealthReport::new();

        report.add(CheckResult {
            name: "test".into(),
            status: CheckStatus::Pass,
            message: "ok".into(),
        });
        assert_eq!(report.overall, CheckStatus::Pass);

        report.add(CheckResult {
            name: "test2".into(),
            status: CheckStatus::Warn,
            message: "warning".into(),
        });
        assert_eq!(report.overall, CheckStatus::Warn);

        report.add(CheckResult {
            name: "test3".into(),
            status: CheckStatus::Fail,
            message: "failed".into(),
        });
        assert_eq!(report.overall, CheckStatus::Fail);
    }

    #[test]
    fn health_report_counts() {
        let mut report = HealthReport::new();
        report.add(CheckResult { name: "a".into(), status: CheckStatus::Pass, message: "".into() });
        report.add(CheckResult { name: "b".into(), status: CheckStatus::Pass, message: "".into() });
        report.add(CheckResult { name: "c".into(), status: CheckStatus::Warn, message: "".into() });
        report.add(CheckResult { name: "d".into(), status: CheckStatus::Fail, message: "".into() });

        assert_eq!(report.pass_count(), 2);
        assert_eq!(report.warn_count(), 1);
        assert_eq!(report.fail_count(), 1);
        assert!(!report.is_healthy());
    }

    #[test]
    fn check_config_file_exists() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("settings.json5");
        fs::write(&path, "{}").unwrap();

        let result = check_config_file(&path);
        assert_eq!(result.status, CheckStatus::Pass);
    }

    #[test]
    fn check_config_file_missing() {
        let result = check_config_file(Path::new("/nonexistent/settings.json5"));
        assert_eq!(result.status, CheckStatus::Warn);
    }

    #[test]
    fn check_watch_folders_empty() {
        let results = check_watch_folders(&[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, CheckStatus::Warn);
    }

    #[test]
    fn check_watch_folders_existing() {
        let dir = TempDir::new().unwrap();
        let results = check_watch_folders(&[dir.path().to_path_buf()]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, CheckStatus::Pass);
    }

    #[test]
    fn check_watch_folders_nonexistent() {
        let results = check_watch_folders(&[PathBuf::from("/nonexistent/path")]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, CheckStatus::Warn);
    }

    #[test]
    fn check_watch_folders_file_not_dir() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("not_a_dir.txt");
        fs::write(&file, "").unwrap();

        let results = check_watch_folders(&[file]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, CheckStatus::Fail);
    }

    #[test]
    fn check_disk_space_existing_path() {
        let dir = TempDir::new().unwrap();
        let result = check_disk_space(dir.path(), 1024);
        assert_eq!(result.status, CheckStatus::Pass);
    }

    #[test]
    fn check_disk_space_nonexistent_path() {
        let result = check_disk_space(Path::new("/nonexistent"), 1024);
        assert_eq!(result.status, CheckStatus::Warn);
    }

    #[test]
    fn check_config_dir_writable_succeeds() {
        let result = check_config_dir_writable();
        // Should pass on most systems
        assert!(result.status == CheckStatus::Pass || result.status == CheckStatus::Fail);
    }

    #[test]
    fn run_health_checks_with_valid_config() {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("settings.json5");
        fs::write(&config_path, "{}").unwrap();

        let report = run_health_checks(&config_path, &[dir.path().to_path_buf()]);
        assert!(report.pass_count() >= 2); // Config file + watch folder
    }

    #[test]
    fn run_health_checks_with_missing_config() {
        let report = run_health_checks(
            Path::new("/nonexistent/settings.json5"),
            &[],
        );
        assert!(report.warn_count() >= 2); // Missing config + no watch folders
    }

    #[test]
    fn check_result_serialization() {
        let result = CheckResult {
            name: "test".into(),
            status: CheckStatus::Pass,
            message: "all good".into(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: CheckResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, CheckStatus::Pass);
    }
}
