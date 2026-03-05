// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Cloud Storage Monitoring (mm-cloud)
//
// Milestone 7 — Cloud Storage Monitoring.
//
// This crate implements cloud storage monitoring for MeedyaManager.
// It watches media libraries stored in cloud providers, detects new/changed
// files using incremental delta APIs, and surfaces events to the core engine
// for rule-based processing.
//
// Supported providers:
//   - OneDrive    (Microsoft Graph API)      — delta + webhooks + thumbnails
//   - Google Drive (Google Drive API v3)     — delta, polling webhooks
//   - Dropbox     (Dropbox API v2)           — cursor-based delta
//   - MEGA        (stub — no official API)   — coming soon
//   - iCloud      (macOS FileProvider only)  — native SwiftUI layer

// --- Module declarations ---

/// Shared traits, types, and error definitions used by all cloud backends.
pub mod traits;

/// Sync manager — orchestrates polling, conflict resolution, and state tracking.
pub mod sync_manager;

/// OneDrive provider implementation (Microsoft Graph API).
pub mod onedrive;

/// Google Drive provider implementation (Drive API v3).
pub mod google_drive;

/// Dropbox provider implementation (Dropbox API v2).
pub mod dropbox;

/// MEGA provider stub (no official Rust API available yet).
pub mod mega;

/// iCloud Drive provider stub (handled natively by the macOS SwiftUI layer).
pub mod icloud;

// --- Re-exports for ergonomic use by downstream crates ---

/// Core trait every cloud backend must implement.
pub use traits::CloudProvider;

/// All cloud-related error types.
pub use traits::CloudError;

/// File/folder metadata returned by provider list calls.
pub use traits::CloudFile;

/// Incremental delta returned by `watch_changes`.
pub use traits::ChangeSet;

/// Per-provider feature flags.
pub use traits::CloudCapabilities;

/// High-level connection status.
pub use traits::SyncStatus;

/// Runtime sync state for a single provider.
pub use traits::SyncState;

/// Conflict resolution strategy.
pub use traits::ConflictResolution;

/// User-configurable sync parameters.
pub use traits::SyncConfig;

/// Sync orchestrator and event log.
pub use sync_manager::SyncManager;
pub use sync_manager::SyncEvent;

/// Concrete provider implementations.
pub use onedrive::OneDriveProvider;
pub use google_drive::GoogleDriveProvider;
pub use dropbox::DropboxProvider;
pub use mega::MegaProvider;
pub use icloud::ICloudProvider;

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── Crate smoke tests ─────────────────────────────────────────────────────

    #[test]
    fn cloud_crate_loads() {
        // Confirms the module tree compiles and re-exports resolve correctly.
        let _ = SyncConfig::default();
        let _ = ConflictResolution::default();
        let _ = SyncStatus::NotConnected;
    }

    // ── OneDrive integration ──────────────────────────────────────────────────

    #[test]
    fn onedrive_name_accessible_via_reexport() {
        assert_eq!(OneDriveProvider::new("id").name(), "OneDrive");
    }

    #[test]
    fn onedrive_not_authenticated_by_default() {
        assert!(!OneDriveProvider::new("id").is_authenticated());
    }

    #[test]
    fn onedrive_caps_support_delta() {
        assert!(OneDriveProvider::new("id").capabilities().supports_delta);
    }

    // ── Google Drive integration ──────────────────────────────────────────────

    #[test]
    fn google_drive_name_accessible_via_reexport() {
        assert_eq!(GoogleDriveProvider::new("id").name(), "Google Drive");
    }

    #[test]
    fn google_drive_not_authenticated_by_default() {
        assert!(!GoogleDriveProvider::new("id").is_authenticated());
    }

    // ── Dropbox integration ───────────────────────────────────────────────────

    #[test]
    fn dropbox_name_accessible_via_reexport() {
        assert_eq!(DropboxProvider::new("key").name(), "Dropbox");
    }

    #[test]
    fn dropbox_not_authenticated_by_default() {
        assert!(!DropboxProvider::new("key").is_authenticated());
    }

    // ── MEGA stub integration ─────────────────────────────────────────────────

    #[test]
    fn mega_name_accessible_via_reexport() {
        assert_eq!(MegaProvider::new().name(), "MEGA");
    }

    #[tokio::test]
    async fn mega_authenticate_unsupported() {
        let mut p = MegaProvider::new();
        assert!(matches!(p.authenticate().await, Err(CloudError::Unsupported(_))));
    }

    // ── iCloud stub integration ───────────────────────────────────────────────

    #[test]
    fn icloud_name_accessible_via_reexport() {
        assert_eq!(ICloudProvider::new().name(), "iCloud Drive");
    }

    #[tokio::test]
    async fn icloud_authenticate_unsupported() {
        let mut p = ICloudProvider::new();
        assert!(matches!(p.authenticate().await, Err(CloudError::Unsupported(_))));
    }

    // ── SyncManager integration ───────────────────────────────────────────────

    #[test]
    fn sync_manager_registers_all_providers() {
        let mut mgr = SyncManager::new(SyncConfig::default());
        assert!(mgr.register_provider("OneDrive",   "/Music"));
        assert!(mgr.register_provider("GoogleDrive", "/Media"));
        assert!(mgr.register_provider("Dropbox",    "/Photos"));
        assert!(mgr.register_provider("MEGA",       "/Videos"));
        assert!(mgr.register_provider("iCloud",     "/iCloud"));
        assert_eq!(mgr.all_states().len(), 5);
    }

    #[test]
    fn sync_manager_all_states_start_not_connected() {
        let mut mgr = SyncManager::new(SyncConfig::default());
        mgr.register_provider("OneDrive", "/Music");
        for state in mgr.all_states() {
            assert_eq!(state.status, SyncStatus::NotConnected);
        }
    }

    #[test]
    fn change_event_round_trip_through_manager() {
        let mut mgr = SyncManager::new(SyncConfig::default());
        mgr.register_provider("Dropbox", "/");
        let cs = ChangeSet {
            added:   vec![CloudFile {
                id:           "f1".into(),
                name:         "track.mp3".into(),
                path:         "/track.mp3".into(),
                size:         Some(4096),
                modified:     None,
                is_folder:    false,
                mime_type:    Some("audio/mpeg".into()),
                hash:         None,
                download_url: None,
            }],
            modified: vec![],
            deleted:  vec![],
            cursor:   "delta_1".into(),
        };
        let events = mgr.process_changes("Dropbox", cs);
        assert!(events.iter().any(|e| matches!(e, SyncEvent::FileAdded { .. })));
        assert_eq!(
            mgr.state("Dropbox").unwrap().cursor.as_deref(),
            Some("delta_1")
        );
    }

    // ── SyncConfig + CloudCapabilities cross-checks ───────────────────────────

    #[test]
    fn default_sync_config_covers_all_audio_formats() {
        let cfg = SyncConfig::default();
        for ext in &["mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "wma"] {
            assert!(cfg.is_media_extension(ext), "missing extension: {ext}");
        }
    }

    #[test]
    fn default_sync_config_covers_common_video_formats() {
        let cfg = SyncConfig::default();
        for ext in &["mp4", "mkv", "avi", "mov"] {
            assert!(cfg.is_media_extension(ext), "missing extension: {ext}");
        }
    }

    #[test]
    fn cloud_error_retryability() {
        assert!(CloudError::Network("timeout".into()).is_retryable());
        assert!(CloudError::RateLimited { retry_after: 5 }.is_retryable());
        assert!(!CloudError::Auth("expired".into()).is_retryable());
        assert!(!CloudError::QuotaExceeded.is_retryable());
    }
}
