// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Cloud Storage Monitoring
//
// This crate implements cloud storage monitoring for MeedyaManager (Milestone 7).
// It watches media libraries stored in cloud providers and synchronises file
// metadata and organisation rules with the local engine.
//
// Supported providers:
//   - OneDrive    (Microsoft Graph API)
//   - Google Drive (Google Drive API v3)
//   - Dropbox     (Dropbox API v2)
//   - MEGA        (MEGA SDK / API)
//   - iCloud      (CloudKit / iCloud Drive)

// --- Module declarations ---

/// Shared traits defining the `CloudProvider` interface for all backends.
pub mod traits;

/// Sync manager — orchestrates background sync, conflict resolution, and state tracking.
pub mod sync_manager;

/// OneDrive provider implementation (Microsoft Graph API).
pub mod onedrive;

/// Google Drive provider implementation (Drive API v3).
pub mod google_drive;

/// Dropbox provider implementation (Dropbox API v2).
pub mod dropbox;

/// MEGA provider implementation.
pub mod mega;

/// iCloud Drive provider implementation (CloudKit).
pub mod icloud;

// --- Unit tests ---

#[cfg(test)]
mod tests {
    /// Smoke test to verify the crate compiles and the module tree is valid.
    #[test]
    fn cloud_crate_loads() {
        // Confirms that the mm-cloud crate links correctly.
        // Provider-specific tests live in each submodule.
        assert!(true, "mm-cloud crate loaded successfully");
    }
}
