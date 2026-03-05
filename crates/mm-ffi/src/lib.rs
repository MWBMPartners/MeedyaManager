// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Foreign Function Interface (FFI) Library
//
// This crate bridges the Rust core (`mm-core`) to external language runtimes:
//
// ┌─────────────────────────────────────────────────────────────┐
// │                     FFI Consumers                           │
// │  ┌──────────────────┐            ┌────────────────────────┐ │
// │  │  Swift (macOS)   │            │    C# (Windows)        │ │
// │  │  via UniFFI      │            │    via P/Invoke        │ │
// │  │  (proc-macro     │            │    (cbindgen header)   │ │
// │  │   generated)     │            │                        │ │
// │  └────────┬─────────┘            └───────────┬────────────┘ │
// │           │  uniffi_api.rs                    │  capi.rs    │
// └───────────┼───────────────────────────────────┼─────────────┘
//             │                                   │
//             └─────────────┬─────────────────────┘
//                           │
//                    ┌──────▼──────┐
//                    │   mm-core   │
//                    └─────────────┘
//
// Module layout:
//   types.rs      — FFI-safe data types shared by both bridges
//   callbacks.rs  — UniFFI callback interface traits (WatchCallback, etc.)
//   uniffi_api.rs — #[uniffi::export] functions (Swift/macOS via UniFFI)
//   capi.rs       — #[no_mangle] extern "C" functions (C#/Windows via cbindgen)
//
// UniFFI approach: proc-macro (`setup_scaffolding!`) — no UDL file required
// at compile time.  The `mm_ffi.udl` file in src/ serves as reference
// documentation and can be used to generate Swift bindings with `uniffi-bindgen`.

// UniFFI scaffolding registration — must be at crate root, not inside a module.
// This macro registers all #[uniffi::export] functions and #[derive(uniffi::*)]
// types with the UniFFI runtime, enabling Swift/Kotlin binding generation.
uniffi::setup_scaffolding!("mm_ffi");

// ---------------------------------------------------------------------------
// Sub-module declarations
// ---------------------------------------------------------------------------

/// Async callback interfaces for event delivery from Rust to native UIs
pub mod callbacks;

/// C API: #[no_mangle] extern "C" functions for Windows P/Invoke
pub mod capi;

/// FFI-safe shared data types (TagEntry, RenamePreviewFfi, MmFfiError, etc.)
pub mod types;

/// UniFFI-exported API functions (Swift/macOS bridge)
pub mod uniffi_api;

// ---------------------------------------------------------------------------
// Re-exports for ergonomic use in tests and by consumers of the lib target
// ---------------------------------------------------------------------------
pub use types::{
    AudioPropertiesFfi, MmFfiError, RenamePreviewFfi, TagEntry, ValidationResult, WatchEventFfi,
};

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uniffi_api;
    use crate::capi;
    use std::ffi::CString;

    // --- MmFfiError tests ---

    /// All error variants produce the expected Display string
    #[test]
    fn error_display_all_variants() {
        assert_eq!(MmFfiError::Config("bad".into()).to_string(), "Configuration error: bad");
        assert_eq!(MmFfiError::Io("not found".into()).to_string(), "I/O error: not found");
        assert_eq!(MmFfiError::Metadata("unsupported".into()).to_string(), "Metadata error: unsupported");
        assert_eq!(MmFfiError::Rename("conflict".into()).to_string(), "Rename error: conflict");
        assert_eq!(MmFfiError::RuleEngine("bad token".into()).to_string(), "Rule engine error: bad token");
        assert_eq!(MmFfiError::Watcher("permission".into()).to_string(), "Watcher error: permission");
        assert_eq!(MmFfiError::Unknown("oops".into()).to_string(), "Unknown error: oops");
    }

    /// MmFfiError converts from MmError correctly
    #[test]
    fn error_converts_from_mm_error() {
        use mm_core::error::MmError;
        let mm_err = MmError::Config("test config error".into());
        let ffi_err: MmFfiError = mm_err.into();
        assert!(matches!(ffi_err, MmFfiError::Config(_)));

        let mm_err = MmError::Metadata("unsupported format".into());
        let ffi_err: MmFfiError = mm_err.into();
        assert!(matches!(ffi_err, MmFfiError::Metadata(_)));
    }

    // --- TagEntry tests ---

    /// TagEntry stores key and value correctly
    #[test]
    fn tag_entry_fields() {
        let entry = TagEntry { key: "artist".into(), value: "Pink Floyd".into() };
        assert_eq!(entry.key, "artist");
        assert_eq!(entry.value, "Pink Floyd");
    }

    /// TagEntry is Clone
    #[test]
    fn tag_entry_clone() {
        let entry = TagEntry { key: "title".into(), value: "The Wall".into() };
        let cloned = entry.clone();
        assert_eq!(entry, cloned);
    }

    // --- RenamePreviewFfi tests ---

    /// RenamePreviewFfi stores all four fields correctly
    #[test]
    fn rename_preview_fields() {
        let preview = RenamePreviewFfi {
            source: "/music/track.mp3".into(),
            destination: "/music/Pink Floyd - Shine On.mp3".into(),
            conflict: false,
            unchanged: false,
        };
        assert!(!preview.conflict);
        assert!(!preview.unchanged);
        assert!(preview.destination.contains("Shine On"));
    }

    /// Unchanged preview has matching source and destination
    #[test]
    fn rename_preview_unchanged() {
        let preview = RenamePreviewFfi {
            source: "/music/track.mp3".into(),
            destination: "/music/track.mp3".into(),
            conflict: false,
            unchanged: true,
        };
        assert!(preview.unchanged);
        assert_eq!(preview.source, preview.destination);
    }

    // --- ValidationResult tests ---

    /// Valid template produces is_valid = true with empty error
    #[test]
    fn validation_result_valid() {
        let result = ValidationResult {
            is_valid: true,
            error_message: String::new(),
            warnings: vec![],
        };
        assert!(result.is_valid);
        assert!(result.error_message.is_empty());
    }

    /// Invalid template produces is_valid = false with non-empty error
    #[test]
    fn validation_result_invalid() {
        let result = ValidationResult {
            is_valid: false,
            error_message: "Unexpected token at position 5".into(),
            warnings: vec!["Unknown tag: <Foobar>".into()],
        };
        assert!(!result.is_valid);
        assert!(!result.error_message.is_empty());
        assert_eq!(result.warnings.len(), 1);
    }

    // --- AudioPropertiesFfi tests ---

    /// Lossless FLAC file has correct properties
    #[test]
    fn audio_properties_lossless() {
        let props = AudioPropertiesFfi {
            duration_secs: 300,
            bitrate_kbps: 1411,
            sample_rate_hz: 44100,
            channels: 2,
            bit_depth: 16,
            is_lossless: true,
            codec: "FLAC".into(),
        };
        assert!(props.is_lossless);
        assert_eq!(props.codec, "FLAC");
        assert_eq!(props.channels, 2);
        assert_eq!(props.bit_depth, 16);
    }

    // --- WatchEventFfi tests ---

    /// Created event has kind "created" and no new_path
    #[test]
    fn watch_event_created() {
        let event = WatchEventFfi {
            kind: "created".into(),
            path: "/music/new_track.mp3".into(),
            new_path: String::new(),
        };
        assert_eq!(event.kind, "created");
        assert!(event.new_path.is_empty());
    }

    /// Renamed event carries both old and new paths
    #[test]
    fn watch_event_renamed() {
        let event = WatchEventFfi {
            kind: "renamed".into(),
            path: "/music/old.mp3".into(),
            new_path: "/music/new.mp3".into(),
        };
        assert_eq!(event.kind, "renamed");
        assert!(!event.new_path.is_empty());
        assert_ne!(event.path, event.new_path);
    }

    // --- mm_version ---

    /// mm_version returns a non-empty string starting with "0."
    #[test]
    fn version_not_empty() {
        let v = uniffi_api::mm_version();
        assert!(!v.is_empty(), "version string must not be empty");
        assert!(v.starts_with("0."), "version should start with '0.' — got: {v}");
    }

    // --- validate_template ---

    /// Valid template passes validation with is_valid = true
    #[test]
    fn validate_template_valid() {
        let result = uniffi_api::validate_template("<Artist> - <Title>".into());
        assert!(result.is_valid, "simple template should be valid");
    }

    /// Empty template fails validation
    #[test]
    fn validate_template_empty() {
        let result = uniffi_api::validate_template(String::new());
        assert!(!result.is_valid, "empty template should be invalid");
        assert!(!result.error_message.is_empty());
    }

    /// Whitespace-only template fails validation
    #[test]
    fn validate_template_whitespace_only() {
        let result = uniffi_api::validate_template("   ".into());
        assert!(!result.is_valid, "whitespace-only template should be invalid");
    }

    // --- list_known_tags ---

    /// list_known_tags returns a non-empty list with standard tag names
    #[test]
    fn list_known_tags_non_empty() {
        let tags = uniffi_api::list_known_tags();
        assert!(!tags.is_empty(), "tag list must not be empty");
        assert!(tags.contains(&"Artist".to_string()), "must contain 'Artist'");
        assert!(tags.contains(&"Title".to_string()), "must contain 'Title'");
        assert!(tags.contains(&"Album".to_string()), "must contain 'Album'");
    }

    // --- C API tests ---

    /// mm_ffi_version returns a non-null pointer containing the version
    #[test]
    fn c_api_version_non_null() {
        let ptr = capi::mm_ffi_version();
        assert!(!ptr.is_null(), "mm_ffi_version must not return null");

        // Safety: the returned pointer is a valid C string allocated by CString::into_raw
        let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(s.starts_with("0."), "C API version should start with '0.' — got: {s}");

        // Free the allocated string to avoid memory leak in tests
        unsafe { capi::mm_ffi_free_string(ptr as *mut std::os::raw::c_char) };
    }

    /// mm_ffi_validate_template returns JSON with is_valid:true for a valid template
    #[test]
    fn c_api_validate_template_valid() {
        let template = CString::new("<Artist> - <Title>").unwrap();
        let ptr = unsafe { capi::mm_ffi_validate_template(template.as_ptr()) };
        assert!(!ptr.is_null());

        let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(s.contains("\"is_valid\":true"), "expected is_valid:true — got: {s}");

        unsafe { capi::mm_ffi_free_string(ptr as *mut std::os::raw::c_char) };
    }

    /// mm_ffi_validate_template returns is_valid:false for an empty template
    #[test]
    fn c_api_validate_template_empty() {
        let template = CString::new("").unwrap();
        let ptr = unsafe { capi::mm_ffi_validate_template(template.as_ptr()) };
        assert!(!ptr.is_null());

        let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(s.contains("\"is_valid\":false"), "expected is_valid:false — got: {s}");

        unsafe { capi::mm_ffi_free_string(ptr as *mut std::os::raw::c_char) };
    }

    /// mm_ffi_validate_template handles a null pointer safely
    #[test]
    fn c_api_validate_template_null() {
        let ptr = unsafe { capi::mm_ffi_validate_template(std::ptr::null()) };
        assert!(!ptr.is_null());

        let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(s.contains("error"), "null input should produce an error — got: {s}");

        unsafe { capi::mm_ffi_free_string(ptr as *mut std::os::raw::c_char) };
    }

    /// mm_ffi_free_string does not panic on null pointer
    #[test]
    fn c_api_free_string_null_safe() {
        // Should be a no-op, not a crash
        unsafe { capi::mm_ffi_free_string(std::ptr::null_mut()) };
    }

    /// mm_ffi_list_known_tags returns a JSON array with standard tags
    #[test]
    fn c_api_list_known_tags() {
        let ptr = capi::mm_ffi_list_known_tags();
        assert!(!ptr.is_null());

        let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(s.starts_with('['), "should be a JSON array — got: {s}");
        assert!(s.contains("Artist"), "should contain 'Artist' — got: {s}");

        unsafe { capi::mm_ffi_free_string(ptr as *mut std::os::raw::c_char) };
    }

    /// mm_ffi_config_path returns a non-null, non-empty path string
    #[test]
    fn c_api_config_path_non_empty() {
        let ptr = capi::mm_ffi_config_path();
        assert!(!ptr.is_null());

        let s = unsafe { std::ffi::CStr::from_ptr(ptr).to_str().unwrap() };
        assert!(!s.is_empty(), "config path must not be empty");

        unsafe { capi::mm_ffi_free_string(ptr as *mut std::os::raw::c_char) };
    }
}
