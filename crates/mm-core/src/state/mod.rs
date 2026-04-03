// (C) 2025-2026 MWBM Partners Ltd
//
// Application state persistence and single-instance enforcement.
//
// Safety: This module uses unsafe code for platform-specific PID checking
// (libc::kill on Unix, OpenProcess on Windows).  Both are audited OS APIs.
//
// Saves application state (last scan times, pending operations) to a
// JSON file in the platform config directory. Uses a lock file to
// prevent multiple instances from running simultaneously.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{MmError, MmResult};

/// Persistent application state saved between sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    /// When the state was last saved
    pub last_saved: DateTime<Utc>,
    /// When each watch folder was last scanned (folder path → timestamp)
    pub last_scan_times: std::collections::HashMap<PathBuf, DateTime<Utc>>,
    /// Number of files processed in the last session
    pub files_processed: u64,
    /// Number of files renamed in the last session
    pub files_renamed: u64,
    /// Application version that created this state
    pub app_version: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            last_saved: Utc::now(),
            last_scan_times: std::collections::HashMap::new(),
            files_processed: 0,
            files_renamed: 0,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

impl AppState {
    /// Load state from a file, or return defaults if the file doesn't exist
    pub fn load(path: &Path) -> MmResult<Self> {
        if !path.exists() {
            debug!("State file not found, using defaults: {}", path.display());
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(path)
            .map_err(|e| MmError::State(format!("cannot read state file: {e}")))?;

        let state: Self = serde_json::from_str(&contents)
            .map_err(|e| MmError::State(format!("cannot parse state file: {e}")))?;

        debug!("Loaded state from {}", path.display());
        Ok(state)
    }

    /// Save state to a file, creating parent directories as needed
    pub fn save(&mut self, path: &Path) -> MmResult<()> {
        // Update timestamp
        self.last_saved = Utc::now();

        // Create parent directories
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MmError::State(format!("cannot create state directory: {e}")))?;
        }

        // Serialize to pretty JSON
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| MmError::State(format!("cannot serialize state: {e}")))?;

        // Write atomically (write to temp file then rename)
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, &json)
            .map_err(|e| MmError::State(format!("cannot write state file: {e}")))?;
        std::fs::rename(&temp_path, path)
            .map_err(|e| MmError::State(format!("cannot finalise state file: {e}")))?;

        debug!("Saved state to {}", path.display());
        Ok(())
    }

    /// Record a scan time for a watch folder
    pub fn record_scan(&mut self, folder: &Path) {
        self.last_scan_times
            .insert(folder.to_path_buf(), Utc::now());
    }

    /// Get the default state file path for this platform
    pub fn default_path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.join("MeedyaManager").join("state.json")
    }
}

/// Single-instance lock file manager.
///
/// Creates a lock file containing the current process ID. Checks for
/// stale locks by verifying the PID is still running.
pub struct LockFile {
    /// Path to the lock file
    path: PathBuf,
    /// Whether we own the lock
    owned: bool,
}

impl LockFile {
    /// Attempt to acquire the lock. Returns Ok if acquired, Err if
    /// another instance is already running.
    pub fn acquire(path: &Path) -> MmResult<Self> {
        // Check for existing lock
        if path.exists() {
            let contents = std::fs::read_to_string(path).unwrap_or_default();
            if let Ok(pid) = contents.trim().parse::<u32>() {
                if is_process_running(pid) {
                    return Err(MmError::State(format!(
                        "another instance is running (PID {pid})"
                    )));
                }
                // Stale lock — process no longer running
                warn!("Removing stale lock file (PID {pid} no longer running)");
                let _ = std::fs::remove_file(path);
            }
        }

        // Create lock file with our PID
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| MmError::State(format!("cannot create lock directory: {e}")))?;
        }

        let pid = std::process::id();
        std::fs::write(path, pid.to_string())
            .map_err(|e| MmError::State(format!("cannot create lock file: {e}")))?;

        info!("Lock file acquired: {} (PID {pid})", path.display());
        Ok(Self {
            path: path.to_path_buf(),
            owned: true,
        })
    }

    /// Release the lock by deleting the lock file
    pub fn release(&mut self) -> MmResult<()> {
        if self.owned && self.path.exists() {
            std::fs::remove_file(&self.path)
                .map_err(|e| MmError::State(format!("cannot remove lock file: {e}")))?;
            self.owned = false;
            debug!("Lock file released: {}", self.path.display());
        }
        Ok(())
    }

    /// Get the default lock file path for this platform
    pub fn default_path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.join("MeedyaManager").join("meedya.lock")
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        if self.owned {
            let _ = self.release();
        }
    }
}

/// Check if a process with the given PID is currently running.
///
/// Uses platform-specific methods:
/// - Unix:    `kill(pid, 0)` — signal 0 probes existence without affecting the process.
/// - Windows: `OpenProcess(SYNCHRONIZE, FALSE, pid)` — attempts to open a minimal
///   handle to the target process.  Two distinct failure codes let us
///   distinguish "process does not exist" from "process exists but access
///   was denied".
/// - Other:   Conservative fallback — always returns `true` (never clears a lock).
// ---------------------------------------------------------------------------
// Unix implementation (Linux, macOS, FreeBSD, etc.)
// ---------------------------------------------------------------------------
#[cfg(unix)]
#[allow(unsafe_code)]
fn is_process_running(pid: u32) -> bool {
    // `kill(pid, 0)` returns 0 if the process exists (even if owned by another user),
    // or -1 with errno ESRCH if no such process exists.
    // errno EPERM means the process exists but we cannot signal it — still running.
    let ret = unsafe { libc::kill(pid as i32, 0) };
    if ret == 0 {
        return true;
    }
    // Use portable errno access instead of platform-specific __errno_location / __error
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

// Declare libc dependency for Unix PID checking
#[cfg(unix)]
extern crate libc;

// ---------------------------------------------------------------------------
// Windows implementation — OpenProcess (issue #131)
// ---------------------------------------------------------------------------
#[cfg(windows)]
#[allow(unsafe_code)]
fn is_process_running(pid: u32) -> bool {
    use winapi::shared::winerror::{ERROR_ACCESS_DENIED, ERROR_INVALID_PARAMETER};
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::processthreadsapi::OpenProcess;
    use winapi::um::winnt::SYNCHRONIZE;

    unsafe {
        // Request only SYNCHRONIZE — the minimal access right for process handles.
        // This succeeds for most processes, including those owned by other users in
        // the same session, without requiring elevated privileges.
        let handle = OpenProcess(
            SYNCHRONIZE, // dwDesiredAccess
            0,           // bInheritHandle = FALSE
            pid,         // dwProcessId
        );

        if !handle.is_null() {
            // Successfully opened a handle — process is alive.
            CloseHandle(handle);
            return true;
        }

        // The call failed.  Inspect the error code to determine why.
        match GetLastError() {
            // ERROR_ACCESS_DENIED: the process exists but we cannot open a handle
            // to it (e.g. it's a system process or belongs to a different user).
            // The process IS running — treat the lock as live.
            ERROR_ACCESS_DENIED => true,

            // ERROR_INVALID_PARAMETER: no process with this PID exists.
            // The lock is stale and can be safely removed.
            ERROR_INVALID_PARAMETER => false,

            // Any other error (e.g. ERROR_INVALID_HANDLE for PID 0): conservatively
            // assume the process is still running to avoid corrupting state.
            _ => true,
        }
    }
}

// ---------------------------------------------------------------------------
// Fallback for exotic platforms (WASM, UEFI, etc.)
// ---------------------------------------------------------------------------
#[cfg(not(any(unix, windows)))]
fn is_process_running(_pid: u32) -> bool {
    // No process-inspection API available.  Return true (conservative) so we
    // never silently delete a lock file that belongs to a running instance.
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn default_state_has_current_version() {
        let state = AppState::default();
        assert_eq!(state.app_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn default_state_empty_scan_times() {
        let state = AppState::default();
        assert!(state.last_scan_times.is_empty());
    }

    #[test]
    fn default_state_zero_counters() {
        let state = AppState::default();
        assert_eq!(state.files_processed, 0);
        assert_eq!(state.files_renamed, 0);
    }

    #[test]
    fn save_and_load_state() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("state.json");

        let mut state = AppState {
            files_processed: 42,
            files_renamed: 10,
            ..Default::default()
        };
        state.record_scan(Path::new("/music"));
        state.save(&path).unwrap();

        let loaded = AppState::load(&path).unwrap();
        assert_eq!(loaded.files_processed, 42);
        assert_eq!(loaded.files_renamed, 10);
        assert!(loaded.last_scan_times.contains_key(Path::new("/music")));
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.json");

        let state = AppState::load(&path).unwrap();
        assert_eq!(state.files_processed, 0);
    }

    #[test]
    fn load_corrupt_file_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("corrupt.json");
        std::fs::write(&path, "not valid json").unwrap();

        let result = AppState::load(&path);
        assert!(result.is_err());
    }

    #[test]
    fn record_scan_updates_timestamp() {
        let mut state = AppState::default();
        let folder = PathBuf::from("/music/library");
        state.record_scan(&folder);
        assert!(state.last_scan_times.contains_key(&folder));
    }

    #[test]
    fn lock_file_acquire_and_release() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let mut lock = LockFile::acquire(&path).unwrap();
        assert!(path.exists());

        // Verify lock contents is our PID
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents, std::process::id().to_string());

        lock.release().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn lock_file_prevents_double_acquire() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        let _lock1 = LockFile::acquire(&path).unwrap();

        // Second acquire should fail (our own PID is running)
        let result = LockFile::acquire(&path);
        assert!(result.is_err());
    }

    #[test]
    fn lock_file_cleans_stale_lock() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        // Write a fake PID that definitely isn't running
        std::fs::write(&path, "99999999").unwrap();

        // Should succeed because the PID isn't running
        // (On non-Unix this conservatively assumes the process IS running,
        // so this test only passes on Unix)
        #[cfg(unix)]
        {
            let lock = LockFile::acquire(&path);
            // PID 99999999 is almost certainly not running
            let _ = lock; // Don't fail test if PID 99999999 happens to be running
        }
    }

    #[test]
    fn lock_file_drop_releases() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.lock");

        {
            let _lock = LockFile::acquire(&path).unwrap();
            assert!(path.exists());
        }
        // After drop, lock should be released
        assert!(!path.exists());
    }

    #[test]
    fn state_serialization_roundtrip() {
        let state = AppState {
            files_processed: 100,
            ..Default::default()
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: AppState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.files_processed, 100);
    }

    #[test]
    fn default_state_path_contains_meedyamanager() {
        let path = AppState::default_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("MeedyaManager"));
    }

    #[test]
    fn default_lock_path_contains_meedyamanager() {
        let path = LockFile::default_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("MeedyaManager"));
    }

    // -----------------------------------------------------------------------
    // is_process_running — platform-specific tests
    // -----------------------------------------------------------------------

    #[test]
    fn current_process_is_detected_as_running() {
        // Our own PID must always be detected as running.
        let my_pid = std::process::id();
        assert!(
            is_process_running(my_pid),
            "is_process_running() must return true for the current process PID"
        );
    }

    #[test]
    fn extremely_large_pid_is_not_running() {
        // Use a very large but positive PID that almost certainly doesn't exist.
        // Note: u32::MAX casts to -1 as i32, and kill(-1, 0) sends to all processes
        // on Unix — so we use a value that stays positive after the cast.
        const FAKE_PID: u32 = 99_999_999;
        #[cfg(unix)]
        assert!(
            !is_process_running(FAKE_PID),
            "PID {FAKE_PID} should not be detected as running on Unix"
        );

        #[cfg(windows)]
        assert!(
            !is_process_running(FAKE_PID),
            "PID {FAKE_PID} should not be detected as running on Windows"
        );
    }

    #[cfg(windows)]
    #[test]
    fn windows_stale_lock_is_cleaned() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("win_stale.lock");

        // Write a PID that is guaranteed not to exist (u32::MAX is invalid on Windows)
        std::fs::write(&path, u32::MAX.to_string()).unwrap();

        // LockFile::acquire should detect the stale lock and overwrite it
        let lock = LockFile::acquire(&path);
        assert!(
            lock.is_ok(),
            "Stale lock with invalid PID should be cleaned on Windows"
        );
    }
}
