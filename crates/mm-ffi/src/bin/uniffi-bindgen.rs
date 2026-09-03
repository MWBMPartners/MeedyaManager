// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — uniffi-bindgen entry point
//
// A thin CLI wrapper that hands off to UniFFI's own bindgen driver (enabled
// via the "cli" feature on the `uniffi` dependency). This is the standard
// way a library crate using the proc-macro scaffolding approach (rather than
// a build.rs + UDL pipeline) generates foreign-language bindings for itself:
// there is no separate `uniffi-bindgen` binary that already knows about our
// exported functions and types, so each UniFFI-consuming crate ships its own
// copy of this same three-line wrapper.
//
// Usage (from the workspace root):
//
//   cargo build -p mm-ffi --release
//   cargo run -p mm-ffi --bin uniffi-bindgen -- generate \
//       --library target/release/libmm_ffi.dylib \
//       --language swift \
//       --out-dir macos/MeedyaManager/Bindings/generated
//
// The `--library` form (rather than pointing at the UDL file) is used
// because this crate registers its exports with `uniffi::setup_scaffolding!`
// via proc-macros, so the compiled cdylib's embedded metadata is the source
// of truth cbindgen — sorry, uniffi-bindgen — reads from.
fn main() {
    // Delegates entirely to uniffi's own CLI implementation: argument
    // parsing, the `generate` subcommand, language backends, all of it.
    uniffi::uniffi_bindgen_main();
}
