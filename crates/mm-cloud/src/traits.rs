// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Cloud Storage Core Traits (mm-cloud)
//
// Defines the `CloudProvider` trait, shared types (`CloudFile`, `ChangeSet`,
// `SyncState`, `SyncConfig`, etc.) and error types used across all cloud
// storage integrations (OneDrive, Google Drive, Dropbox, MEGA stub, iCloud stub).

use std::fmt;
use std::future::Future;
use std::path::Path;
use std::time::SystemTime;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// All errors that can arise from a cloud storage operation.
#[derive(Debug, thiserror::Error)]
pub enum CloudError {
    /// OAuth / token authentication failed.
    #[error("authentication failed: {0}")]
    Auth(String),

    /// A general network / transport failure.
    #[error("network error: {0}")]
    Network(String),

    /// The provider's rate limiter rejected the request.
    /// `retry_after` is the number of seconds the caller should wait.
    #[error("rate limited — retry after {retry_after}s")]
    RateLimited { retry_after: u64 },

    /// The requested file or folder was not found in the provider.
    #[error("not found: {0}")]
    NotFound(String),

    /// A conflict was detected (e.g. simultaneous edits on both sides).
    #[error("conflict: {0}")]
    Conflict(String),

    /// The user's storage quota has been exceeded.
    #[error("quota exceeded")]
    QuotaExceeded,

    /// The operation is not supported by this provider.
    #[error("unsupported: {0}")]
    Unsupported(String),

    /// A JSON / API response parse failure.
    #[error("parse error: {0}")]
    Parse(String),

    /// The file is too large for the provider's single-request upload limit.
    #[error("file too large: {size} bytes")]
    FileTooLarge { size: u64 },
}

impl CloudError {
    /// Returns `true` if the error is transient and the operation should be retried.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            CloudError::Network(_) | CloudError::RateLimited { .. }
        )
    }

    /// Returns the number of seconds to wait before retrying, if known.
    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            CloudError::RateLimited { retry_after } => Some(*retry_after),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// CloudFile
// ---------------------------------------------------------------------------

/// Metadata for a single file or folder returned by a cloud provider.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudFile {
    /// Provider-specific unique file/folder identifier.
    pub id: String,
    /// Display name (filename, without path).
    pub name: String,
    /// Full path within the cloud root, using forward slashes.
    pub path: String,
    /// File size in bytes (`None` for folders).
    pub size: Option<u64>,
    /// Last-modified timestamp (`None` if unavailable).
    pub modified: Option<SystemTime>,
    /// `true` if this entry represents a directory / folder.
    pub is_folder: bool,
    /// MIME type as reported by the provider (`None` for folders).
    pub mime_type: Option<String>,
    /// Provider-supplied content hash for change detection (`None` if unavailable).
    pub hash: Option<String>,
    /// Pre-signed or direct download URL (`None` for folders or if not provided).
    pub download_url: Option<String>,
}

impl CloudFile {
    /// Returns the lowercase file extension (e.g. `"mp3"`), or an empty string
    /// for folders or files without an extension.
    pub fn extension(&self) -> String {
        if self.is_folder {
            return String::new();
        }
        std::path::Path::new(&self.name)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default()
    }

    /// Returns `true` when the file extension matches a known media type.
    pub fn is_media_file(&self) -> bool {
        // Audio, video, image, and e-book extensions we care about.
        matches!(
            self.extension().as_str(),
            "mp3" | "flac" | "m4a" | "aac" | "ogg" | "opus" | "wav" | "wma"
                | "mp4" | "mkv" | "avi" | "mov" | "wmv" | "m4v" | "webm"
                | "jpg" | "jpeg" | "png" | "gif" | "webp"
                | "epub" | "pdf" | "mobi"
        )
    }
}

// ---------------------------------------------------------------------------
// ChangeSet
// ---------------------------------------------------------------------------

/// The set of incremental changes detected since the last sync cursor.
/// Providers that support delta APIs (OneDrive, Google Drive, Dropbox) fill
/// this from their native delta/changes endpoint; polling-only providers
/// re-compute it by diffing full directory listings.
#[derive(Debug, Clone, Default)]
pub struct ChangeSet {
    /// Files that were added or newly discovered.
    pub added: Vec<CloudFile>,
    /// Files that were modified (content or metadata changed).
    pub modified: Vec<CloudFile>,
    /// Provider IDs of files that were deleted.
    pub deleted: Vec<String>,
    /// Opaque cursor / page token for the *next* `watch_changes` call.
    pub cursor: String,
}

impl ChangeSet {
    /// Returns `true` when there are no changes to process.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    /// Total number of individual changes (added + modified + deleted).
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.modified.len() + self.deleted.len()
    }
}

// ---------------------------------------------------------------------------
// CloudCapabilities
// ---------------------------------------------------------------------------

/// Feature flags that describe what a provider implementation supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloudCapabilities {
    /// Provider exposes an incremental-change / delta API.
    pub supports_delta: bool,
    /// Provider can push real-time notifications via webhooks.
    pub supports_webhooks: bool,
    /// Provider can return image thumbnails for media files.
    pub supports_thumbnails: bool,
    /// Maximum file size (bytes) for a single-request upload.
    pub max_upload_bytes: u64,
}

impl CloudCapabilities {
    /// Capabilities for providers with full delta, webhook, and thumbnail support.
    pub fn full() -> Self {
        Self {
            supports_delta: true,
            supports_webhooks: true,
            supports_thumbnails: true,
            max_upload_bytes: 150 * 1024 * 1024, // 150 MiB
        }
    }

    /// Capabilities for providers with delta but no webhook support.
    pub fn delta_only() -> Self {
        Self {
            supports_delta: true,
            supports_webhooks: false,
            supports_thumbnails: false,
            max_upload_bytes: 150 * 1024 * 1024,
        }
    }

    /// Capabilities for stub / polling-only providers.
    pub fn polling_only() -> Self {
        Self {
            supports_delta: false,
            supports_webhooks: false,
            supports_thumbnails: false,
            max_upload_bytes: 150 * 1024 * 1024,
        }
    }
}

// ---------------------------------------------------------------------------
// SyncStatus
// ---------------------------------------------------------------------------

/// High-level state of a cloud provider connection.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    /// Not yet connected or authenticated.
    NotConnected,
    /// Everything is up to date.
    Synced,
    /// A sync pass is currently in progress.
    Syncing,
    /// One or more files have unresolved conflicts.
    Conflict,
    /// The last sync attempt failed with an error message.
    Error(String),
}

impl fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncStatus::NotConnected => write!(f, "Not Connected"),
            SyncStatus::Synced      => write!(f, "Synced"),
            SyncStatus::Syncing     => write!(f, "Syncing…"),
            SyncStatus::Conflict    => write!(f, "Conflict"),
            SyncStatus::Error(msg)  => write!(f, "Error: {msg}"),
        }
    }
}

// ---------------------------------------------------------------------------
// SyncState
// ---------------------------------------------------------------------------

/// Runtime state for one active cloud provider connection.
#[derive(Debug, Clone)]
pub struct SyncState {
    /// Human-readable name of the provider (e.g. `"OneDrive"`).
    pub provider: String,
    /// Current connection / sync status.
    pub status: SyncStatus,
    /// Timestamp of the most recent successful sync pass.
    pub last_sync: Option<SystemTime>,
    /// Number of media files successfully synced in the last pass.
    pub files_synced: u64,
    /// Number of media files queued but not yet processed.
    pub files_pending: u64,
    /// Opaque delta cursor used for incremental change detection.
    pub cursor: Option<String>,
    /// Root folder path being monitored within the cloud storage.
    pub root_path: String,
}

impl SyncState {
    /// Creates a fresh `SyncState` with `NotConnected` status for the given provider.
    pub fn new(provider: impl Into<String>, root_path: impl Into<String>) -> Self {
        Self {
            provider:      provider.into(),
            status:        SyncStatus::NotConnected,
            last_sync:     None,
            files_synced:  0,
            files_pending: 0,
            cursor:        None,
            root_path:     root_path.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// ConflictResolution
// ---------------------------------------------------------------------------

/// Strategy to apply when a local and remote file have both been modified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Always keep the local version; overwrite the remote copy.
    LocalWins,
    /// Always keep the remote version; overwrite the local copy.
    RemoteWins,
    /// Keep both versions by suffixing the local copy with a timestamp.
    KeepBoth,
    /// Pause and surface the conflict to the user for manual resolution.
    Ask,
}

impl Default for ConflictResolution {
    fn default() -> Self {
        Self::Ask
    }
}

impl fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictResolution::LocalWins  => write!(f, "Local wins"),
            ConflictResolution::RemoteWins => write!(f, "Remote wins"),
            ConflictResolution::KeepBoth   => write!(f, "Keep both"),
            ConflictResolution::Ask        => write!(f, "Ask me"),
        }
    }
}

// ---------------------------------------------------------------------------
// SyncConfig
// ---------------------------------------------------------------------------

/// User-configurable parameters for the cloud sync manager.
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Seconds between polling passes for providers that don't support webhooks.
    pub poll_interval_secs: u64,
    /// Maximum number of concurrent provider syncs allowed at once.
    pub max_concurrent_syncs: usize,
    /// Strategy to use when conflicts are detected.
    pub conflict_resolution: ConflictResolution,
    /// File extensions (lowercase, no dot) that should be synced.
    pub media_extensions: Vec<String>,
    /// Maximum files to process per polling cycle (prevents runaway scans).
    pub max_files_per_cycle: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs:  60,
            max_concurrent_syncs: 2,
            conflict_resolution: ConflictResolution::Ask,
            media_extensions: vec![
                // Audio
                "mp3".into(), "flac".into(), "m4a".into(), "aac".into(),
                "ogg".into(), "opus".into(), "wav".into(), "wma".into(),
                // Video
                "mp4".into(), "mkv".into(), "avi".into(), "mov".into(),
                "wmv".into(), "m4v".into(), "webm".into(),
                // E-books / documents
                "epub".into(), "pdf".into(), "mobi".into(),
            ],
            max_files_per_cycle: 500,
        }
    }
}

impl SyncConfig {
    /// Returns `true` if the given lowercase extension is in the media list.
    pub fn is_media_extension(&self, ext: &str) -> bool {
        self.media_extensions.iter().any(|e| e == ext)
    }
}

// ---------------------------------------------------------------------------
// CloudProvider trait
// ---------------------------------------------------------------------------

/// Common interface implemented by every cloud storage backend.
///
/// All async methods use RPITIT (return-position `impl Trait`) — no
/// `async_trait` macro is needed with the 2024 edition `+ Send` bounds.
pub trait CloudProvider: Send + Sync {
    /// Short display name for this provider (e.g. `"OneDrive"`).
    fn name(&self) -> &str;

    /// Returns `true` when the provider has a valid, non-expired access token.
    fn is_authenticated(&self) -> bool;

    /// Returns the feature flags for this provider implementation.
    fn capabilities(&self) -> CloudCapabilities;

    /// Performs the OAuth / API-key authentication flow and stores the token.
    fn authenticate(&mut self)
        -> impl Future<Output = Result<(), CloudError>> + Send;

    /// Refreshes an expired access token using the stored refresh token.
    fn refresh_token(&mut self)
        -> impl Future<Output = Result<(), CloudError>> + Send;

    /// Lists all files (recursively) under the given cloud path.
    fn list_files(&self, path: &str)
        -> impl Future<Output = Result<Vec<CloudFile>, CloudError>> + Send;

    /// Fetches metadata for a single file by its provider-specific ID.
    fn get_file(&self, id: &str)
        -> impl Future<Output = Result<CloudFile, CloudError>> + Send;

    /// Downloads the content of `file` and writes it to `dest` on disk.
    fn download_file(&self, file: &CloudFile, dest: &Path)
        -> impl Future<Output = Result<(), CloudError>> + Send;

    /// Uploads the file at `src` to `dest_path` in the cloud storage.
    fn upload_file(&self, src: &Path, dest_path: &str)
        -> impl Future<Output = Result<CloudFile, CloudError>> + Send;

    /// Returns changes since the given `cursor` (or the full listing if `None`).
    fn watch_changes(&self, path: &str, cursor: Option<&str>)
        -> impl Future<Output = Result<ChangeSet, CloudError>> + Send;

    /// Revokes stored tokens and resets the provider to an unauthenticated state.
    fn disconnect(&mut self)
        -> impl Future<Output = Result<(), CloudError>> + Send;
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── CloudError ───────────────────────────────────────────────────────────

    #[test]
    fn auth_error_is_not_retryable() {
        let e = CloudError::Auth("token expired".into());
        assert!(!e.is_retryable());
    }

    #[test]
    fn network_error_is_retryable() {
        let e = CloudError::Network("timeout".into());
        assert!(e.is_retryable());
    }

    #[test]
    fn rate_limited_is_retryable_and_has_delay() {
        let e = CloudError::RateLimited { retry_after: 30 };
        assert!(e.is_retryable());
        assert_eq!(e.retry_after_secs(), Some(30));
    }

    #[test]
    fn not_found_has_no_retry_delay() {
        let e = CloudError::NotFound("track.mp3".into());
        assert_eq!(e.retry_after_secs(), None);
    }

    #[test]
    fn quota_exceeded_is_not_retryable() {
        let e = CloudError::QuotaExceeded;
        assert!(!e.is_retryable());
    }

    #[test]
    fn file_too_large_display_includes_size() {
        let e = CloudError::FileTooLarge { size: 1_000_000 };
        assert!(e.to_string().contains("1000000"));
    }

    // ── CloudFile ────────────────────────────────────────────────────────────

    fn make_file(name: &str) -> CloudFile {
        CloudFile {
            id:           name.to_string(),
            name:         name.to_string(),
            path:         format!("/Music/{name}"),
            size:         Some(4_096),
            modified:     None,
            is_folder:    false,
            mime_type:    Some("audio/mpeg".into()),
            hash:         None,
            download_url: None,
        }
    }

    #[test]
    fn extension_returns_lowercase_ext() {
        let f = make_file("Track01.MP3");
        assert_eq!(f.extension(), "mp3");
    }

    #[test]
    fn extension_returns_empty_for_folder() {
        let f = CloudFile {
            id: "dir1".into(), name: "Music".into(), path: "/Music".into(),
            size: None, modified: None, is_folder: true,
            mime_type: None, hash: None, download_url: None,
        };
        assert_eq!(f.extension(), "");
    }

    #[test]
    fn is_media_file_true_for_mp3() {
        assert!(make_file("song.mp3").is_media_file());
    }

    #[test]
    fn is_media_file_true_for_flac() {
        assert!(make_file("album.flac").is_media_file());
    }

    #[test]
    fn is_media_file_false_for_txt() {
        assert!(!make_file("readme.txt").is_media_file());
    }

    #[test]
    fn is_media_file_false_for_exe() {
        assert!(!make_file("app.exe").is_media_file());
    }

    // ── ChangeSet ────────────────────────────────────────────────────────────

    #[test]
    fn empty_changeset_is_empty() {
        let cs = ChangeSet::default();
        assert!(cs.is_empty());
        assert_eq!(cs.total_changes(), 0);
    }

    #[test]
    fn changeset_total_counts_all_variants() {
        let cs = ChangeSet {
            added:    vec![make_file("a.mp3"), make_file("b.mp3")],
            modified: vec![make_file("c.mp3")],
            deleted:  vec!["d".into()],
            cursor:   "tok123".into(),
        };
        assert!(!cs.is_empty());
        assert_eq!(cs.total_changes(), 4);
    }

    #[test]
    fn changeset_with_only_deletions_is_not_empty() {
        let cs = ChangeSet {
            deleted: vec!["x".into()],
            ..Default::default()
        };
        assert!(!cs.is_empty());
    }

    // ── CloudCapabilities ────────────────────────────────────────────────────

    #[test]
    fn full_capabilities_supports_delta_and_webhooks() {
        let c = CloudCapabilities::full();
        assert!(c.supports_delta);
        assert!(c.supports_webhooks);
        assert!(c.supports_thumbnails);
    }

    #[test]
    fn delta_only_has_no_webhooks() {
        let c = CloudCapabilities::delta_only();
        assert!(c.supports_delta);
        assert!(!c.supports_webhooks);
    }

    #[test]
    fn polling_only_has_no_delta() {
        let c = CloudCapabilities::polling_only();
        assert!(!c.supports_delta);
        assert!(!c.supports_webhooks);
    }

    // ── SyncStatus ───────────────────────────────────────────────────────────

    #[test]
    fn sync_status_display_not_connected() {
        assert_eq!(SyncStatus::NotConnected.to_string(), "Not Connected");
    }

    #[test]
    fn sync_status_display_error_includes_message() {
        let s = SyncStatus::Error("auth failed".into());
        assert!(s.to_string().contains("auth failed"));
    }

    #[test]
    fn sync_status_syncing_contains_ellipsis() {
        assert!(SyncStatus::Syncing.to_string().contains('…'));
    }

    // ── SyncState ────────────────────────────────────────────────────────────

    #[test]
    fn new_sync_state_starts_not_connected() {
        let s = SyncState::new("OneDrive", "/Music");
        assert_eq!(s.status, SyncStatus::NotConnected);
        assert_eq!(s.provider, "OneDrive");
        assert_eq!(s.root_path, "/Music");
        assert!(s.last_sync.is_none());
        assert!(s.cursor.is_none());
    }

    // ── ConflictResolution ───────────────────────────────────────────────────

    #[test]
    fn conflict_resolution_default_is_ask() {
        assert_eq!(ConflictResolution::default(), ConflictResolution::Ask);
    }

    #[test]
    fn conflict_resolution_display() {
        assert_eq!(ConflictResolution::LocalWins.to_string(), "Local wins");
        assert_eq!(ConflictResolution::RemoteWins.to_string(), "Remote wins");
        assert_eq!(ConflictResolution::KeepBoth.to_string(), "Keep both");
        assert_eq!(ConflictResolution::Ask.to_string(), "Ask me");
    }

    // ── SyncConfig ───────────────────────────────────────────────────────────

    #[test]
    fn default_sync_config_has_60s_interval() {
        let cfg = SyncConfig::default();
        assert_eq!(cfg.poll_interval_secs, 60);
    }

    #[test]
    fn default_sync_config_has_18_extensions() {
        let cfg = SyncConfig::default();
        // 8 audio + 7 video + 3 ebook = 18
        assert_eq!(cfg.media_extensions.len(), 18);
    }

    #[test]
    fn is_media_extension_true_for_flac() {
        let cfg = SyncConfig::default();
        assert!(cfg.is_media_extension("flac"));
    }

    #[test]
    fn is_media_extension_false_for_docx() {
        let cfg = SyncConfig::default();
        assert!(!cfg.is_media_extension("docx"));
    }

    #[test]
    fn default_conflict_resolution_is_ask() {
        let cfg = SyncConfig::default();
        assert_eq!(cfg.conflict_resolution, ConflictResolution::Ask);
    }

    #[test]
    fn max_files_per_cycle_default_is_500() {
        let cfg = SyncConfig::default();
        assert_eq!(cfg.max_files_per_cycle, 500);
    }
}
