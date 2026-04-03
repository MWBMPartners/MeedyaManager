// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Cloud Sync Manager
//
// Orchestrates background sync passes across multiple cloud providers.
// Handles polling intervals, conflict detection, delta cursors, and
// per-provider `SyncState` tracking.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::traits::{
    ChangeSet, CloudError, CloudFile, ConflictResolution, SyncConfig, SyncState, SyncStatus,
};

// ---------------------------------------------------------------------------
// SyncEvent
// ---------------------------------------------------------------------------

/// Events emitted by the sync manager during a sync pass.
#[derive(Debug, Clone)]
pub enum SyncEvent {
    /// A sync pass started for the named provider.
    SyncStarted { provider: String },
    /// A file was added or newly found in the cloud.
    FileAdded { provider: String, file: CloudFile },
    /// A file was modified remotely.
    FileModified { provider: String, file: CloudFile },
    /// A file was deleted from the cloud.
    FileDeleted { provider: String, id: String },
    /// A conflict was detected that requires user resolution.
    ConflictDetected { provider: String, file: CloudFile },
    /// A sync pass completed without errors.
    SyncCompleted { provider: String, changes: usize },
    /// A sync pass failed with an error.
    SyncFailed { provider: String, error: String },
}

// ---------------------------------------------------------------------------
// SyncManager
// ---------------------------------------------------------------------------

/// Manages the cloud sync lifecycle for all connected providers.
///
/// Maintains a per-provider `SyncState` map and applies the configured
/// `SyncConfig` (polling interval, conflict strategy, media extension filter).
pub struct SyncManager {
    /// User-configurable sync parameters.
    config: SyncConfig,
    /// Per-provider state, keyed by provider name.
    states: HashMap<String, SyncState>,
    /// Buffered events from the most recent sync pass (newest first).
    event_log: Arc<Mutex<Vec<SyncEvent>>>,
}

impl SyncManager {
    /// Creates a new `SyncManager` with the given `SyncConfig`.
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            states: HashMap::new(),
            event_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Registers a new provider connection with the given root path.
    /// Returns `false` if a provider with the same name is already registered.
    pub fn register_provider(&mut self, provider_name: &str, root_path: &str) -> bool {
        if self.states.contains_key(provider_name) {
            return false;
        }
        self.states.insert(
            provider_name.to_string(),
            SyncState::new(provider_name, root_path),
        );
        true
    }

    /// Removes a provider from the manager. Returns `true` if it was present.
    pub fn unregister_provider(&mut self, provider_name: &str) -> bool {
        self.states.remove(provider_name).is_some()
    }

    /// Returns a snapshot of the current `SyncState` for all registered providers.
    pub fn all_states(&self) -> Vec<SyncState> {
        self.states.values().cloned().collect()
    }

    /// Returns the `SyncState` for a specific provider, or `None` if unknown.
    pub fn state(&self, provider_name: &str) -> Option<&SyncState> {
        self.states.get(provider_name)
    }

    /// Processes a `ChangeSet` returned by a provider, applying the configured
    /// conflict resolution strategy and media extension filter.
    ///
    /// Returns the list of `SyncEvent`s emitted during this pass.
    pub fn process_changes(&mut self, provider_name: &str, changeset: ChangeSet) -> Vec<SyncEvent> {
        let mut events = Vec::new();
        let now = SystemTime::now();

        // Locate the provider state (must be registered).
        let Some(state) = self.states.get_mut(provider_name) else {
            return events;
        };

        state.status = SyncStatus::Syncing;
        events.push(SyncEvent::SyncStarted {
            provider: provider_name.to_string(),
        });

        let mut change_count = 0;

        // --- Process added files ---
        for file in &changeset.added {
            // Only emit events for media extensions the user cares about.
            if !self.config.is_media_extension(&file.extension()) {
                continue;
            }
            events.push(SyncEvent::FileAdded {
                provider: provider_name.to_string(),
                file: file.clone(),
            });
            change_count += 1;
        }

        // --- Process modified files ---
        for file in &changeset.modified {
            if !self.config.is_media_extension(&file.extension()) {
                continue;
            }
            // Apply the conflict resolution strategy.
            match self.config.conflict_resolution {
                ConflictResolution::Ask => {
                    // Surface the conflict to the user for manual resolution.
                    events.push(SyncEvent::ConflictDetected {
                        provider: provider_name.to_string(),
                        file: file.clone(),
                    });
                }
                _ => {
                    // LocalWins / RemoteWins / KeepBoth all proceed with sync.
                    events.push(SyncEvent::FileModified {
                        provider: provider_name.to_string(),
                        file: file.clone(),
                    });
                }
            }
            change_count += 1;
        }

        // --- Process deletions ---
        for id in &changeset.deleted {
            events.push(SyncEvent::FileDeleted {
                provider: provider_name.to_string(),
                id: id.clone(),
            });
            change_count += 1;
        }

        // Update cursor and state after processing.
        if !changeset.cursor.is_empty() {
            state.cursor = Some(changeset.cursor);
        }
        state.files_synced += change_count as u64;
        state.last_sync = Some(now);
        state.status = SyncStatus::Synced;

        events.push(SyncEvent::SyncCompleted {
            provider: provider_name.to_string(),
            changes: change_count,
        });

        // Append to the shared event log.
        if let Ok(mut log) = self.event_log.lock() {
            log.extend(events.iter().cloned());
        }

        events
    }

    /// Records a sync failure for the named provider.
    pub fn record_failure(&mut self, provider_name: &str, error: &CloudError) {
        if let Some(state) = self.states.get_mut(provider_name) {
            state.status = SyncStatus::Error(error.to_string());
        }
        if let Ok(mut log) = self.event_log.lock() {
            log.push(SyncEvent::SyncFailed {
                provider: provider_name.to_string(),
                error: error.to_string(),
            });
        }
    }

    /// Clears the event log and returns the drained events (oldest first).
    pub fn drain_events(&self) -> Vec<SyncEvent> {
        if let Ok(mut log) = self.event_log.lock() {
            let mut drained = log.drain(..).collect::<Vec<_>>();
            drained.reverse(); // oldest first
            drained
        } else {
            Vec::new()
        }
    }

    /// Returns a reference to the current `SyncConfig`.
    pub fn config(&self) -> &SyncConfig {
        &self.config
    }

    /// Replaces the `SyncConfig`. Takes effect on the next sync pass.
    pub fn set_config(&mut self, config: SyncConfig) {
        self.config = config;
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{ChangeSet, CloudFile, SyncConfig, SyncStatus};

    fn make_file(name: &str) -> CloudFile {
        CloudFile {
            id: name.to_string(),
            name: name.to_string(),
            path: format!("/Music/{name}"),
            size: Some(4_096),
            modified: None,
            is_folder: false,
            mime_type: Some("audio/mpeg".into()),
            hash: None,
            download_url: None,
        }
    }

    fn manager() -> SyncManager {
        let mut m = SyncManager::new(SyncConfig::default());
        m.register_provider("OneDrive", "/Music");
        m
    }

    // ── Registration ─────────────────────────────────────────────────────────

    #[test]
    fn register_provider_returns_true_on_first_call() {
        let mut m = SyncManager::new(SyncConfig::default());
        assert!(m.register_provider("OneDrive", "/Music"));
    }

    #[test]
    fn register_provider_returns_false_for_duplicate() {
        let mut m = SyncManager::new(SyncConfig::default());
        m.register_provider("OneDrive", "/Music");
        assert!(!m.register_provider("OneDrive", "/Movies"));
    }

    #[test]
    fn unregister_provider_returns_true_if_present() {
        let mut m = manager();
        assert!(m.unregister_provider("OneDrive"));
    }

    #[test]
    fn unregister_provider_returns_false_if_absent() {
        let mut m = SyncManager::new(SyncConfig::default());
        assert!(!m.unregister_provider("Dropbox"));
    }

    #[test]
    fn state_is_not_connected_after_register() {
        let m = manager();
        assert_eq!(
            m.state("OneDrive").unwrap().status,
            SyncStatus::NotConnected
        );
    }

    // ── process_changes ───────────────────────────────────────────────────────

    #[test]
    fn process_changes_emits_sync_started_and_completed() {
        let mut m = manager();
        let cs = ChangeSet::default();
        let events = m.process_changes("OneDrive", cs);
        let kinds: Vec<&str> = events
            .iter()
            .map(|e| match e {
                SyncEvent::SyncStarted { .. } => "started",
                SyncEvent::SyncCompleted { .. } => "completed",
                _ => "other",
            })
            .collect();
        assert!(kinds.contains(&"started"));
        assert!(kinds.contains(&"completed"));
    }

    #[test]
    fn process_changes_filters_non_media_files() {
        let mut m = manager();
        let mut file = make_file("readme.txt");
        file.name = "readme.txt".into();
        let cs = ChangeSet {
            added: vec![file],
            ..Default::default()
        };
        let events = m.process_changes("OneDrive", cs);
        // No FileAdded event should be emitted for a .txt file.
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, SyncEvent::FileAdded { .. }))
        );
    }

    #[test]
    fn process_changes_emits_file_added_for_mp3() {
        let mut m = manager();
        let cs = ChangeSet {
            added: vec![make_file("track.mp3")],
            ..Default::default()
        };
        let events = m.process_changes("OneDrive", cs);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SyncEvent::FileAdded { .. }))
        );
    }

    #[test]
    fn process_changes_emits_file_deleted() {
        let mut m = manager();
        let cs = ChangeSet {
            deleted: vec!["abc123".into()],
            ..Default::default()
        };
        let events = m.process_changes("OneDrive", cs);
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SyncEvent::FileDeleted { .. }))
        );
    }

    #[test]
    fn process_changes_updates_cursor() {
        let mut m = manager();
        let cs = ChangeSet {
            cursor: "tok_xyz".into(),
            ..Default::default()
        };
        m.process_changes("OneDrive", cs);
        assert_eq!(
            m.state("OneDrive").unwrap().cursor.as_deref(),
            Some("tok_xyz")
        );
    }

    #[test]
    fn process_changes_sets_status_to_synced() {
        let mut m = manager();
        m.process_changes("OneDrive", ChangeSet::default());
        assert_eq!(m.state("OneDrive").unwrap().status, SyncStatus::Synced);
    }

    #[test]
    fn record_failure_sets_error_status() {
        let mut m = manager();
        let err = CloudError::Auth("expired".into());
        m.record_failure("OneDrive", &err);
        assert!(matches!(
            m.state("OneDrive").unwrap().status,
            SyncStatus::Error(_)
        ));
    }

    #[test]
    fn drain_events_clears_log() {
        let mut m = manager();
        m.process_changes("OneDrive", ChangeSet::default());
        let first = m.drain_events();
        let second = m.drain_events();
        assert!(!first.is_empty());
        assert!(second.is_empty());
    }

    #[test]
    fn all_states_returns_all_registered_providers() {
        let mut m = SyncManager::new(SyncConfig::default());
        m.register_provider("OneDrive", "/Music");
        m.register_provider("GoogleDrive", "/Media");
        m.register_provider("Dropbox", "/Photos");
        assert_eq!(m.all_states().len(), 3);
    }
}
