// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — C API for Windows P/Invoke interop
//
// All functions in this module are exported with C linkage (`extern "C"`) and
// marked `#[no_mangle]` so cbindgen can generate the matching C header and
// C# can call them via P/Invoke.
//
// API conventions:
//   - All string parameters are `*const c_char` (UTF-8, null-terminated)
//   - All string return values are heap-allocated `*mut c_char` (caller frees)
//   - Call `mm_ffi_free_string` to free any returned string pointer
//   - Boolean returns use `bool` (C99 _Bool / stdbool.h)
//   - On error, string-returning functions return JSON: `{"error":"message"}`
//
// Memory safety:
//   - Input pointers are checked for null before dereferencing
//   - All returned strings are allocated with `CString::new().into_raw()`
//   - `mm_ffi_free_string` reclaims ownership and drops the allocation
//
// cbindgen will generate declarations for all `pub unsafe extern "C"` functions
// in this module into `include/mm_ffi.h` at build time.

use std::collections::HashMap;
use std::ffi::{CStr, CString, c_char};
use std::path::PathBuf;

use crate::types::{MmFfiError, RenamePreviewFfi, TagEntry, ValidationResult};
use crate::uniffi_api;

// ---------------------------------------------------------------------------
// Memory management
// ---------------------------------------------------------------------------

/// Free a string pointer previously returned by any `mm_ffi_*` function.
///
/// Passing a null pointer is safe (no-op).
/// Passing a non-mm_ffi-allocated pointer results in undefined behaviour.
///
/// # Safety
/// `ptr` must be null or a pointer returned by an `mm_ffi_*` function.
#[no_mangle]
pub unsafe extern "C" fn mm_ffi_free_string(ptr: *mut c_char) {
    // Null check — do nothing for null pointers
    if ptr.is_null() {
        return;
    }
    // Reconstruct the CString from the raw pointer, then drop it
    // This correctly deallocates the memory that was allocated by CString::into_raw()
    drop(unsafe { CString::from_raw(ptr) });
}

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------

/// Return the MeedyaManager version string as a C string.
///
/// Caller must free the returned pointer with `mm_ffi_free_string`.
///
/// Example return value: `"2.0.0-alpha.5"`
#[no_mangle]
pub extern "C" fn mm_ffi_version() -> *const c_char {
    // Allocate the version string and transfer ownership to the caller
    alloc_cstring(env!("CARGO_PKG_VERSION"))
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Return the platform-specific path to `settings.json5` as a C string.
///
/// Caller must free with `mm_ffi_free_string`.
#[no_mangle]
pub extern "C" fn mm_ffi_config_path() -> *const c_char {
    alloc_cstring(&uniffi_api::config_path())
}

/// Load the configuration and return it as a JSON-encoded C string.
///
/// On success: JSON object of the config.
/// On failure: `{"error":"<message>"}`.
/// Caller must free with `mm_ffi_free_string`.
#[no_mangle]
pub extern "C" fn mm_ffi_config_load() -> *const c_char {
    let result = uniffi_api::config_load();
    result_to_cstring(result)
}

// ---------------------------------------------------------------------------
// Media scanning
// ---------------------------------------------------------------------------

/// Scan a directory and return rename previews as a JSON array.
///
/// - `directory`  — UTF-8 null-terminated absolute path
/// - `template`   — UTF-8 null-terminated rename template string
/// - `recursive`  — 1 to descend into sub-directories, 0 for flat scan
///
/// On success: `[{"source":"...","destination":"...","conflict":false,...},...]`
/// On failure: `{"error":"<message>"}`
/// Caller must free with `mm_ffi_free_string`.
///
/// # Safety
/// `directory` and `template` must be non-null, valid UTF-8 C strings.
#[no_mangle]
pub unsafe extern "C" fn mm_ffi_scan_directory(
    directory: *const c_char,
    template: *const c_char,
    recursive: bool,
) -> *const c_char {
    // Safely convert C strings to Rust strings (return error JSON on null/invalid UTF-8)
    let dir = match cstr_to_string(directory) {
        Some(s) => s,
        None => return alloc_error_json("directory parameter is null or invalid UTF-8"),
    };
    let tmpl = match cstr_to_string(template) {
        Some(s) => s,
        None => return alloc_error_json("template parameter is null or invalid UTF-8"),
    };

    let result = uniffi_api::scan_directory(dir, tmpl, recursive);

    match result {
        Ok(previews) => {
            // Serialise the preview list to a JSON array
            match serde_json::to_string(&previews) {
                Ok(json) => alloc_cstring(&json),
                Err(e) => alloc_error_json(&e.to_string()),
            }
        }
        Err(e) => alloc_error_json(&e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/// Read all metadata tags from a file and return them as a JSON array.
///
/// On success: `[{"key":"title","value":"Track Name"},...]`
/// On failure: `{"error":"<message>"}`
/// Caller must free with `mm_ffi_free_string`.
///
/// # Safety
/// `path` must be a non-null, valid UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn mm_ffi_get_metadata(path: *const c_char) -> *const c_char {
    let p = match cstr_to_string(path) {
        Some(s) => s,
        None => return alloc_error_json("path parameter is null or invalid UTF-8"),
    };

    let result = uniffi_api::get_metadata(p);
    match result {
        Ok(tags) => match serde_json::to_string(&tags) {
            Ok(json) => alloc_cstring(&json),
            Err(e) => alloc_error_json(&e.to_string()),
        },
        Err(e) => alloc_error_json(&e.to_string()),
    }
}

/// Write metadata tags to a file from a JSON array of `{key, value}` objects.
///
/// On success: `{"ok":true}`
/// On failure: `{"error":"<message>"}`
/// Caller must free with `mm_ffi_free_string`.
///
/// # Safety
/// `path` and `tags_json` must be non-null, valid UTF-8 C strings.
#[no_mangle]
pub unsafe extern "C" fn mm_ffi_write_metadata(
    path: *const c_char,
    tags_json: *const c_char,
) -> *const c_char {
    let p = match cstr_to_string(path) {
        Some(s) => s,
        None => return alloc_error_json("path parameter is null or invalid UTF-8"),
    };
    let json = match cstr_to_string(tags_json) {
        Some(s) => s,
        None => return alloc_error_json("tags_json parameter is null or invalid UTF-8"),
    };

    // Parse the JSON array into Vec<TagEntry>
    let tags: Vec<TagEntry> = match serde_json::from_str(&json) {
        Ok(t) => t,
        Err(e) => return alloc_error_json(&format!("Failed to parse tags_json: {e}")),
    };

    match uniffi_api::write_metadata(p, tags) {
        Ok(()) => alloc_cstring(r#"{"ok":true}"#),
        Err(e) => alloc_error_json(&e.to_string()),
    }
}

/// Remove a single tag field from a file.
///
/// On success: `{"ok":true}`
/// On failure: `{"error":"<message>"}`
/// Caller must free with `mm_ffi_free_string`.
///
/// # Safety
/// `path` and `tag_key` must be non-null, valid UTF-8 C strings.
#[no_mangle]
pub unsafe extern "C" fn mm_ffi_remove_tag(
    path: *const c_char,
    tag_key: *const c_char,
) -> *const c_char {
    let p = match cstr_to_string(path) {
        Some(s) => s,
        None => return alloc_error_json("path is null or invalid UTF-8"),
    };
    let key = match cstr_to_string(tag_key) {
        Some(s) => s,
        None => return alloc_error_json("tag_key is null or invalid UTF-8"),
    };

    match uniffi_api::remove_tag(p, key) {
        Ok(()) => alloc_cstring(r#"{"ok":true}"#),
        Err(e) => alloc_error_json(&e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Template validation
// ---------------------------------------------------------------------------

/// Validate a rename template and return the result as JSON.
///
/// On success: `{"is_valid":true,"error_message":"","warnings":[]}`
/// On failure: `{"is_valid":false,"error_message":"...","warnings":[]}`
/// Caller must free with `mm_ffi_free_string`.
///
/// # Safety
/// `template` must be a non-null, valid UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn mm_ffi_validate_template(template: *const c_char) -> *const c_char {
    let tmpl = match cstr_to_string(template) {
        Some(s) => s,
        None => return alloc_error_json("template is null or invalid UTF-8"),
    };

    let result = uniffi_api::validate_template(tmpl);

    match serde_json::to_string(&result) {
        Ok(json) => alloc_cstring(&json),
        Err(e) => alloc_error_json(&e.to_string()),
    }
}

/// Apply a template to a JSON array of `{key,value}` tags and return the result.
///
/// On success: `"Artist - Title"` (the computed filename as a JSON string)
/// On failure: `{"error":"<message>"}`
/// Caller must free with `mm_ffi_free_string`.
///
/// # Safety
/// `template` and `tags_json` must be non-null, valid UTF-8 C strings.
#[no_mangle]
pub unsafe extern "C" fn mm_ffi_apply_template(
    template: *const c_char,
    tags_json: *const c_char,
) -> *const c_char {
    let tmpl = match cstr_to_string(template) {
        Some(s) => s,
        None => return alloc_error_json("template is null or invalid UTF-8"),
    };
    let json = match cstr_to_string(tags_json) {
        Some(s) => s,
        None => return alloc_error_json("tags_json is null or invalid UTF-8"),
    };

    let tags: Vec<TagEntry> = match serde_json::from_str(&json) {
        Ok(t) => t,
        Err(e) => return alloc_error_json(&format!("Failed to parse tags_json: {e}")),
    };

    match uniffi_api::apply_template(tmpl, tags) {
        Ok(name) => {
            // Return as a JSON string value
            match serde_json::to_string(&name) {
                Ok(json) => alloc_cstring(&json),
                Err(e) => alloc_error_json(&e.to_string()),
            }
        }
        Err(e) => alloc_error_json(&e.to_string()),
    }
}

/// Return all known tag names as a JSON array of strings.
///
/// Example: `["Artist","Title","Album","Year",...]`
/// Caller must free with `mm_ffi_free_string`.
#[no_mangle]
pub extern "C" fn mm_ffi_list_known_tags() -> *const c_char {
    let tags = uniffi_api::list_known_tags();
    match serde_json::to_string(&tags) {
        Ok(json) => alloc_cstring(&json),
        Err(e) => alloc_error_json(&e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Allocate a C string from a Rust `&str`, transferring ownership to the caller.
///
/// Interior null bytes are replaced with `?` to ensure the string is valid.
/// Returns a raw pointer; caller must free with `mm_ffi_free_string`.
fn alloc_cstring(s: &str) -> *const c_char {
    // Replace any embedded null bytes (which would truncate the C string)
    let safe = s.replace('\0', "?");
    // Allocate and transfer ownership via into_raw()
    CString::new(safe)
        .unwrap_or_else(|_| CString::new("?").unwrap())
        .into_raw()
}

/// Allocate a JSON error object `{"error":"<message>"}` as a C string.
fn alloc_error_json(message: &str) -> *const c_char {
    // Escape the message as a JSON string value
    let json = format!(
        r#"{{"error":{}}}"#,
        serde_json::to_string(message).unwrap_or_else(|_| r#""unknown error""#.into())
    );
    alloc_cstring(&json)
}

/// Safely convert a `*const c_char` to an owned `String`.
///
/// Returns `None` if the pointer is null or the bytes are not valid UTF-8.
///
/// # Safety
/// `ptr` must be null or point to a valid, null-terminated C string.
unsafe fn cstr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    // SAFETY: caller guarantees ptr is null or a valid C string
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .ok()
        .map(|s| s.to_owned())
}

/// Convenience: convert a Result<String, MmFfiError> to a C string.
fn result_to_cstring(result: Result<String, MmFfiError>) -> *const c_char {
    match result {
        Ok(s) => alloc_cstring(&s),
        Err(e) => alloc_error_json(&e.to_string()),
    }
}
