# C ABI (Windows P/Invoke)

> **(C) 2025-2026 MWBM Partners Ltd**

This page documents the plain-C ABI that `crates/mm-ffi/src/capi.rs` exports, the header cbindgen
generates from it (`include/mm_ffi.h`), and how the Windows WinUI 3 C# project consumes it. It is
cross-checked line-by-line against the real P/Invoke declarations in
`windows/MeedyaManager/Interop/MmCore.cs` — the discrepancies below are verified against both
sides, not assumed from one.

## How it is generated

`crates/mm-ffi/build.rs` runs `cbindgen` (configured by `crates/mm-ffi/cbindgen.toml`) against
every `#[unsafe(no_mangle)] pub extern "C" fn` in `capi.rs`, **on every build that compiles
`mm-ffi`** — there is no separate manual generation step:

```bash
cargo build -p mm-ffi        # regenerates include/mm_ffi.h as a side effect
```

The build script fails outright if the generated header contains fewer than 11 `mm_ffi_`
occurrences, specifically to prevent a repeat of issue #64 (cbindgen 0.27 silently produced a
header with zero prototypes because it did not recognise the Rust-2024 `#[unsafe(no_mangle)]`
attribute form; cbindgen was bumped to ≥0.29 and the count guard was added so an empty/truncated
header fails CI instead of shipping unnoticed).

`grep -c 'mm_ffi_' include/mm_ffi.h` currently reports **23** — that counts *lines* containing the
substring `mm_ffi_` (most function doc comments mention `mm_ffi_free_string` in their "caller must
free with" line, so this is higher than the function count). The header declares **11 distinct
functions**, listed below.

## The 11 exported functions

All string parameters are `const char *` — UTF-8, null-terminated. All string return values are
heap-allocated `char *` (owned by the caller) or `const char *` (also caller-owned despite the
`const`; this is a quirk of the Rust signatures — see the note under "Ownership" below) that
**must** be released with `mm_ffi_free_string`. On error, every string-returning function returns
`{"error":"<message>"}` as JSON rather than a null pointer or a distinguishable error code.

| Function | Signature | Returns |
| -------- | --------- | ------- |
| `mm_ffi_free_string` | `void mm_ffi_free_string(char *ptr)` | Frees a string previously returned by any `mm_ffi_*` function. Null-safe. |
| `mm_ffi_version` | `const char *mm_ffi_version(void)` | The `CARGO_PKG_VERSION` string, e.g. `"1.4.0-alpha.1"` |
| `mm_ffi_config_path` | `const char *mm_ffi_config_path(void)` | Platform path to `settings.json5` |
| `mm_ffi_config_load` | `const char *mm_ffi_config_load(void)` | The loaded config as a JSON object, or `{"error":...}` |
| `mm_ffi_scan_directory` | `const char *mm_ffi_scan_directory(const char *directory, const char *template_, bool recursive)` | `[{"source":...,"destination":...,"conflict":bool,"unchanged":bool},...]` |
| `mm_ffi_get_metadata` | `const char *mm_ffi_get_metadata(const char *path)` | `[{"key":...,"value":...},...]` |
| `mm_ffi_write_metadata` | `const char *mm_ffi_write_metadata(const char *path, const char *tags_json)` | `{"ok":true}` or `{"error":...}` |
| `mm_ffi_remove_tag` | `const char *mm_ffi_remove_tag(const char *path, const char *tag_key)` | `{"ok":true}` or `{"error":...}` |
| `mm_ffi_validate_template` | `const char *mm_ffi_validate_template(const char *template_)` | `{"is_valid":bool,"error_message":"","warnings":[]}` |
| `mm_ffi_apply_template` | `const char *mm_ffi_apply_template(const char *template_, const char *tags_json)` | The computed filename as a JSON string, or `{"error":...}` |
| `mm_ffi_list_known_tags` | `const char *mm_ffi_list_known_tags(void)` | `["Artist","Title","Album",...]` |

## Ownership / lifetime rules (verified against `capi.rs`)

- Every function that returns a string allocates it with `CString::new(...).into_raw()`
  (`alloc_cstring` / `alloc_error_json` helpers in `capi.rs`). The caller owns that allocation and
  **must** pass it to `mm_ffi_free_string` exactly once — passing a pointer that `mm_ffi_free_string`
  did not allocate is undefined behaviour (the header's own doc comment says so).
- `mm_ffi_free_string(NULL)` is explicitly safe (a documented no-op) — confirmed by the crate's own
  test `c_api_free_string_null_safe`.
- Interior NUL bytes in a Rust `&str` being returned are replaced with `?` before allocation
  (`alloc_cstring`), so a malformed value cannot truncate the C string early.
- Input string parameters (`directory`, `template_`, `path`, `tags_json`, `tag_key`) are borrowed —
  the C caller retains ownership and must not free them; mm-ffi only reads them via
  `CStr::from_ptr` for the duration of the call.
- A null input pointer never crashes: every function checks with `cstr_to_string` and returns
  `{"error":"<param> is null or invalid UTF-8"}` instead of dereferencing.

## Cross-check against `windows/MeedyaManager/Interop/MmCore.cs`

Two independent, verified discrepancies exist between the header above and the real C# P/Invoke
layer, both already tracked as **issue #208** (opened from the same kind of audit this page is
part of):

### 1. Three real exports have no `[DllImport]` at all

`MmCore.cs` declares only 8 of the 11 real exports:

```text
Declared:      FreeString, NativeVersion, NativeConfigPath, NativeValidateTemplate,
               NativeListKnownTags, NativeScanDirectory, NativeGetMetadata, NativeWriteMetadata
Missing:       mm_ffi_config_load, mm_ffi_remove_tag, mm_ffi_apply_template
```

The Windows app therefore cannot call config-load, tag-removal, or template-application through
the FFI at all, even though the Rust side has supported all three since Round 3. (`MmCore.cs`'s own
`ConfigPath()` method falls back to a locally-computed `%APPDATA%` path rather than ever calling
`mm_ffi_config_load` — there is no code path that reaches it.)

### 2. Every string is marshalled as ANSI, not UTF-8

Every `[DllImport]` string parameter uses `[MarshalAs(UnmanagedType.LPStr)]`
(`MmCore.cs` lines 104, 110–113, 117, 120–122) and every string return is read with
`Marshal.PtrToStringAnsi(ptr)` (`MmCore.cs` line 141). Both are the **ANSI code page**, not UTF-8.
The Rust side is UTF-8 throughout: `capi.rs`'s own module doc says "All string parameters are
`*const c_char` (UTF-8, null-terminated)", and every returned string comes from a Rust `String`
via `CString::new`.

**Impact:** any file path or tag value containing non-ASCII characters (accented names, non-Latin
scripts, emoji in a comment tag) that survives the round trip will be silently corrupted rather
than rejected with an error — ANSI and UTF-8 agree only in the 7-bit ASCII range. This is a latent
data-corruption bug, not a crash, so it is easy to miss in testing with ASCII-only sample files.

**The fix** (per issue #208): switch every P/Invoke string parameter to
`[MarshalAs(UnmanagedType.LPUTF8Str)]` and every return to `Marshal.PtrToStringUTF8` (or the
.NET 8 `Utf8StringMarshaller`), and add the three missing `[DllImport]` declarations. Neither is
done in the source as it stands at commit `633f8e3`.

### 3. The DLL-bundling path in the `.csproj` almost certainly never resolves

`windows/MeedyaManager/MeedyaManager.csproj` bundles the built DLL with:

```xml
<Content Include="..\..\..\target\release\mm_ffi.dll"
         Condition="Exists('..\..\..\target\release\mm_ffi.dll')">
```

`ci-windows.yml` runs `cargo build -p mm-ffi --release` from the repository root before the .NET
build, and there is no `.cargo/config.toml` `target-dir` override in this repository, so Cargo
places the DLL at `<repo-root>/target/release/mm_ffi.dll`. From
`windows/MeedyaManager/MeedyaManager.csproj`'s own directory, reaching the repository root needs
exactly **two** `..` segments (`windows/MeedyaManager/../..` = repo root), not three. The `Condition`
as written resolves to `<parent-of-repo-root>/target/release/mm_ffi.dll` — one level *above* the
checkout — which will not exist on any normal clone (CI or a developer's machine), verified with
`os.path.normpath('windows/MeedyaManager/../../../target/release/mm_ffi.dll') ==
'../target/release/mm_ffi.dll'`. The same three-`..` prefix is used again two lines later for the
`LICENSE` file, so that bundling step is equally affected.

**Practical effect:** the `Condition` guard exists specifically so a missing DLL degrades
gracefully rather than failing the build — and given this path, it is *always* missing from the
`.csproj`'s point of view, every time. Combined with `MmCore.cs`'s own `_dllAvailable` check
(`File.Exists` on `mm_ffi.dll` beside the executable at runtime), this means the built Windows
app currently runs entirely on `MmCore.cs`'s hand-written stub methods — not because
`MM_FFI_AVAILABLE`-style linking was deliberately deferred (there is no such flag on the C# side),
but because the DLL is silently never copied into the output directory in the first place. Fixing
this needs only correcting `..\..\..\` to `..\..\` in both `<Content Include>` entries.

## What could not be verified

Whether the built `mm_ffi.dll` is architecturally/ABI-compatible with the WinUI 3 host process
(e.g. matching MSVC vs. GNU Rust target, x64 vs. ARM64) was not checked — that is a separate
question from the path bug above and was outside what this audit covered.
