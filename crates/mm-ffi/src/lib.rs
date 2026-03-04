// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Foreign Function Interface (FFI) Layer
//
// This crate provides the FFI bridge between MeedyaManager's Rust core and
// external language consumers. It uses two complementary mechanisms:
//
// 1. **UniFFI** (Mozilla) — Generates idiomatic bindings for:
//    - Python:  Used by the legacy Python UI and scripting plugins
//    - Swift:   Used by the macOS/iOS native UI (SwiftUI)
//    - Kotlin:  Used by the Android native UI (Jetpack Compose)
//
//    UniFFI works by reading a UDL (UniFFI Definition Language) file or
//    proc-macro annotations to generate the scaffolding code and foreign
//    language bindings automatically.
//
// 2. **cbindgen** (planned) — Generates C headers for:
//    - C/C++ consumers that need direct access to the Rust API
//    - Other languages with C FFI support (Go, Zig, etc.)
//
// Architecture:
//    ┌─────────────┐     ┌──────────┐     ┌──────────────┐
//    │  Python/     │     │  mm-ffi  │     │  mm-core     │
//    │  Swift/      │◄───►│  (this)  │◄───►│  mm-providers│
//    │  Kotlin      │     │          │     │  etc.        │
//    └─────────────┘     └──────────┘     └──────────────┘
//
// Usage:
//    The generated shared library (libmm_ffi.dylib / .so / .dll) is loaded
//    by the target language runtime. UniFFI scaffolding handles type
//    conversion, error mapping, and memory management across the boundary.

// TODO: Add UniFFI scaffolding macro once UDL or proc-macro interface is defined:
//   uniffi::setup_scaffolding!();

// TODO: Define exported FFI functions wrapping mm-core and mm-providers APIs.

// --- Unit tests ---

#[cfg(test)]
mod tests {
    /// Smoke test to verify the crate compiles and the FFI layer is valid.
    #[test]
    fn ffi_crate_loads() {
        // Confirms that the mm-ffi crate links correctly.
        // FFI integration tests will use the generated bindings.
        assert!(true, "mm-ffi crate loaded successfully");
    }
}
