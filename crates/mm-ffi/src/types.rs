// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — FFI-safe shared data types
//
// All types in this module must be compatible with:
//   - UniFFI record/enum constraints (Clone, Debug, no references)
//   - cbindgen C header generation (repr(C) where needed)
//   - serde serialization for the C API JSON returns
//
// Design rules:
//   - No HashMap (not UniFFI-compatible) — use Vec<TagEntry> instead
//   - All string fields use owned String (not &str)
//   - Numeric fields use platform-stable sizes (u32, u8, etc.)
//   - Error types implement Display for human-readable messages

use mm_core::error::MmError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Metadata tag pair
// ---------------------------------------------------------------------------

/// A single metadata tag represented as a UTF-8 key/value pair.
///
/// UniFFI does not support HashMap across the FFI boundary, so all tag
/// collections are passed as `Vec<TagEntry>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct TagEntry {
    /// Canonical lowercase tag key (e.g. "title", "artist", "album")
    pub key: String,
    /// Tag value encoded as a UTF-8 string (numeric tags are string-encoded)
    pub value: String,
    /// Something a person should be told about this value, in plain
    /// English, for an app to show beside it — or `None` (almost always).
    ///
    /// Filled in by `get_metadata` for the `language` entry only, when the
    /// file's tags disagree about the language: the value shown comes from
    /// the tag that can hold the full language code, and the file's ID3 tag
    /// says something different (third review round of the language-policy
    /// work, item 3 — before, the other answer was hidden from the apps
    /// entirely). Ignored by `write_metadata`: it is a report about the
    /// file, never something to write into it.
    ///
    /// Optional in every direction, so nothing that already builds or reads
    /// a `TagEntry` has to change: UniFFI gives the Swift initialiser a
    /// default of `nil`; the C API leaves the field out of the JSON when it
    /// is empty, and accepts JSON that does not mention it.
    #[uniffi(default = None)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

// ---------------------------------------------------------------------------
// What a successful metadata write reports
// ---------------------------------------------------------------------------

/// What `write_metadata` reports when the save succeeded.
///
/// Codex's catch-up review of the language-policy branch, finding 6: an app
/// writing `language` = `pt-BR` to an MP3 got `{"ok":true}` back while the
/// file stored only `por` — an ID3 tag can hold just the three-letter code,
/// so the region is lost — and nothing told it. The command line has said so
/// since the first review round; the apps had no way to. This carries the
/// same note the command line shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct WriteMetadataResult {
    /// One `TagEntry` for each field written that a person should be told
    /// something about: the key and the value as given, and `note` — the
    /// same plain-English text `meedya edit --set` prints, worked out by
    /// the same code (`mm_core::metadata::language::preview_conversion_note`),
    /// before the save, from the file the save starts from.
    ///
    /// Only the `language` field has notes today: when what is stored loses
    /// something that was asked for (an ID3 tag keeping only `por` for
    /// `pt-BR`), when the shared code has something to say about the value
    /// itself, or when the value is already the file's and one of its tags
    /// disagrees and is left alone. `None` — not an empty list — when there
    /// is nothing to say, so the C API's JSON stays exactly `{"ok":true}`.
    ///
    /// A write holding the same key more than once is refused, so there is
    /// at most one note per key (the stand-in review of round 6,
    /// carry-over 2).
    ///
    /// The apps do not show these notes yet (#256).
    #[uniffi(default = None)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<TagEntry>>,
}

// ---------------------------------------------------------------------------
// Rename preview
// ---------------------------------------------------------------------------

/// The result of simulating a single file rename operation.
///
/// Passed from Rust to Swift/C# so the UI can display a diff of pending
/// renames before the user confirms execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct RenamePreviewFfi {
    /// Absolute path of the source file
    pub source: String,
    /// Computed absolute path of the destination file
    pub destination: String,
    /// True if a file already exists at the destination (rename would conflict)
    pub conflict: bool,
    /// True if source and destination are the same path (no rename needed)
    pub unchanged: bool,
}

impl RenamePreviewFfi {
    /// Convert from the mm-core RenamePreview type.
    ///
    /// Converts PathBuf fields to Strings, using lossy UTF-8 conversion
    /// to ensure the FFI boundary always receives valid strings.
    pub fn from_core(core: mm_core::renamer::RenamePreview) -> Self {
        Self {
            // Convert PathBuf to String using lossy UTF-8 (handles non-UTF paths gracefully)
            source: core.source.to_string_lossy().into_owned(),
            destination: core.destination.to_string_lossy().into_owned(),
            conflict: core.conflict,
            unchanged: core.unchanged,
        }
    }
}

// ---------------------------------------------------------------------------
// Audio technical properties
// ---------------------------------------------------------------------------

/// Technical properties of an audio file (codec-independent).
///
/// All fields default to 0 / false / empty string when not applicable
/// so consumers do not need to handle Option types across the FFI.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct AudioPropertiesFfi {
    /// Duration in whole seconds (0 if unknown)
    pub duration_secs: u32,
    /// Bitrate in kilobits per second (0 if unknown)
    pub bitrate_kbps: u32,
    /// Sample rate in Hz (e.g. 44100, 48000) — 0 if unknown
    pub sample_rate_hz: u32,
    /// Number of audio channels (1 = mono, 2 = stereo) — 0 if unknown
    pub channels: u8,
    /// Bit depth (e.g. 16, 24) — 0 if not applicable (lossy formats)
    pub bit_depth: u8,
    /// True for lossless formats (FLAC, ALAC, WAV, AIFF)
    pub is_lossless: bool,
    /// Human-readable codec name (e.g. "FLAC", "MP3", "AAC", "Opus")
    pub codec: String,
}

// ---------------------------------------------------------------------------
// Template validation result
// ---------------------------------------------------------------------------

/// Result of validating a rename template string.
///
/// Returned by `validate_template()` so the UI can display inline
/// errors and warnings in the rule builder without calling into Rust repeatedly.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct ValidationResult {
    /// True if the template is syntactically valid and can be applied
    pub is_valid: bool,
    /// Human-readable error message (empty string when `is_valid` is true)
    pub error_message: String,
    /// Non-fatal warnings, e.g. unknown tag names that will always be empty
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// File watcher event
// ---------------------------------------------------------------------------

/// A file system event delivered via the WatchCallback interface.
///
/// The `kind` field is a string enum to remain easily serializable
/// across the FFI boundary without a separate UniFFI enum.
#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct WatchEventFfi {
    /// Event kind: "created" | "modified" | "deleted" | "renamed" | "error"
    pub kind: String,
    /// Absolute path of the affected file
    pub path: String,
    /// New absolute path — only populated for "renamed" events, empty otherwise
    pub new_path: String,
}

// ---------------------------------------------------------------------------
// FFI error type
// ---------------------------------------------------------------------------

/// Error variants returned by mm-ffi functions across the FFI boundary.
///
/// UniFFI maps these to Swift/Kotlin error types automatically.
/// The C API serializes these as JSON with an `"error"` field.
#[derive(Debug, Error, uniffi::Error)]
pub enum MmFfiError {
    /// Configuration loading or validation failed
    #[error("Configuration error: {0}")]
    Config(String),

    /// File I/O operation failed
    #[error("I/O error: {0}")]
    Io(String),

    /// Metadata extraction or writing failed
    #[error("Metadata error: {0}")]
    Metadata(String),

    /// File rename operation failed (conflict, permission denied, etc.)
    #[error("Rename error: {0}")]
    Rename(String),

    /// Rule engine template parsing or evaluation failed
    #[error("Rule engine error: {0}")]
    RuleEngine(String),

    /// File system watcher initialisation or event error
    #[error("Watcher error: {0}")]
    Watcher(String),

    /// Unexpected or unclassified error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<MmError> for MmFfiError {
    /// Convert from mm-core's MmError to the FFI-safe MmFfiError.
    ///
    /// This conversion runs on every mm-core API call that crosses the FFI
    /// boundary, mapping internal errors to their closest FFI variant.
    fn from(err: MmError) -> Self {
        match err {
            MmError::Config(msg) => Self::Config(msg),
            MmError::Watcher(msg) => Self::Watcher(msg),
            MmError::RuleEngine(msg) => Self::RuleEngine(msg),
            MmError::Metadata(msg) => Self::Metadata(msg),
            MmError::Rename(msg) => Self::Rename(msg),
            MmError::Io(e) => Self::Io(e.to_string()),
            // Lofty errors surfaced as Metadata errors across the boundary
            MmError::Lofty(e) => Self::Metadata(e.to_string()),
            // Serde errors surfaced as Config errors (typically JSON5 parse failures)
            MmError::Serde(e) => Self::Config(e.to_string()),
            // Notify errors surfaced as Watcher errors
            MmError::Notify(e) => Self::Watcher(e.to_string()),
            // All other variants fall into Unknown
            other => Self::Unknown(other.to_string()),
        }
    }
}
