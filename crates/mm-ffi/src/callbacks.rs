// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — FFI callback interfaces
//
// Defines the traits that Swift/Kotlin implementations must satisfy to receive
// asynchronous events from the Rust core (file watcher, scan progress, etc.).
//
// UniFFI callback interfaces allow Rust to call into Swift/Kotlin at any time
// from any thread.  Implementors must be `Send + Sync` as callbacks may fire
// from the notify watcher thread.
//
// Architecture:
//   Rust core (watcher thread)
//      │
//      ▼  on_event(WatchEventFfi)
//   WatchCallback trait
//      │
//      ▼  (UniFFI bridges to Swift/Kotlin)
//   SwiftUI / Compose view model

use crate::types::WatchEventFfi;

// ---------------------------------------------------------------------------
// WatchCallback — real-time file system event delivery
// ---------------------------------------------------------------------------

/// Callback interface for receiving file system events from the watcher.
///
/// Implement this trait in Swift (macOS) or Kotlin (future Android) to receive
/// live events when files are created, modified, deleted, or renamed inside
/// a watched directory.
///
/// # Thread safety
/// Callbacks are delivered from the notify watcher thread.  Implementations
/// must be `Send + Sync` and must not block the calling thread.  Dispatch
/// all UI updates to the main thread inside the implementation.
///
/// # UniFFI callback interface
/// UniFFI generates a Swift protocol and a concrete C-backed implementation
/// from this trait definition, so Swift code can pass a class instance to
/// `start_watch()` and receive events as method calls.
#[uniffi::export(callback_interface)]
pub trait WatchCallback: Send + Sync {
    /// Called when a file system event occurs in the watched directory.
    ///
    /// - `event.kind` — "created" | "modified" | "deleted" | "renamed" | "error"
    /// - `event.path` — absolute path of the affected file
    /// - `event.new_path` — only populated for "renamed" events
    fn on_event(&self, event: WatchEventFfi);

    /// Called when the watcher encounters an unrecoverable error.
    ///
    /// After this is called the watcher has stopped; call `start_watch` again
    /// to resume monitoring.
    fn on_error(&self, message: String);
}

// ---------------------------------------------------------------------------
// ScanProgressCallback — scan operation progress reporting
// ---------------------------------------------------------------------------

/// Callback interface for scan operation progress.
///
/// Implement this in Swift/Kotlin to drive a progress bar or status text
/// during a directory scan.
#[uniffi::export(callback_interface)]
pub trait ScanProgressCallback: Send + Sync {
    /// Called once for each file processed during a scan.
    ///
    /// - `current` — number of files processed so far
    /// - `total`   — total number of media files found (0 if still counting)
    /// - `path`    — path of the file just processed
    fn on_progress(&self, current: u32, total: u32, path: String);

    /// Called when the scan completes (successfully or due to an error).
    ///
    /// - `error_message` — empty string on success; human-readable on failure
    fn on_complete(&self, error_message: String);
}
