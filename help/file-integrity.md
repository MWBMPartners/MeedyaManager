# File Integrity — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager includes a SHA256-verified, atomic-write mechanism for **metadata tag writes**,
implemented in `crates/mm-core/src/integrity.rs`, and — as of issue
[#128](https://github.com/MWBMPartners/MeedyaManager/issues/128) — it is wired into every real
tag-writing code path in the application.

> ✅ **`write_tags_safe` (and its siblings) are the only way any real caller writes a tag.**
> `meedya edit` (`crates/mm-cli/src/commands/edit.rs`), the Linux GTK metadata panel
> (`crates/mm-gtk/src/ui/metadata_panel.rs`), and the FFI layer used by the macOS/Windows UIs
> (`crates/mm-ffi/src/uniffi_api.rs`) all call `mm_core::integrity`'s guarded functions —
> `write_tags_safe`, `remove_tag_safe`, `embed_cover_art_safe`, `remove_cover_art_safe` — rather
> than `metadata::write_tags` directly. This was not true before #128: previously only the
> integrity module's own unit tests ever exercised these functions, and every real caller wrote
> straight to the original file with no pre-write hash, no temp-file staging, and no post-write
> verification. That gap is now closed.
>
> This page also does **not** cover file *moves/renames* — those go through a separate module
> (`crates/mm-core/src/renamer`) with its own conflict-detection logic, documented in
> [cli-reference.md](cli-reference.md#meedya-scan). This page is about **tag writes** only.

---

## Table of Contents

1. [What `write_tags_safe` Does](#what-write_tags_safe-does)
2. [Test Mode Interaction](#test-mode-interaction)
3. [The Corruption Log](#the-corruption-log)
4. [What Actually Runs Today](#what-actually-runs-today)
5. [Checking a File's Hash](#checking-a-files-hash)

---

## What `write_tags_safe` Does

Every real tag write in the application goes through this guard. `write_tags_safe` (and
`remove_tag_safe`/`embed_cover_art_safe`/`remove_cover_art_safe`, which share the same
`mutate_file_safe` implementation) protects a metadata write with the following steps:

1. Compute the SHA256 hash of the original file.
2. Copy the original to a temp file **in the same directory** (so the later rename stays on one
   filesystem), named with the marker placed *before* the real extension —
   `track.meedya_tmp.mp3`, not `track.mp3.meedya_tmp`. This ordering matters: `lofty` resolves
   the container format from the path extension alone, with no content-sniffing fallback, so a
   temp file ending in `.meedya_tmp` failed to parse at all. That bug meant the standard
   (non-Test-Mode) path had never actually worked on a real file until it was fixed alongside
   #128 — nothing had called it outside its own failure-path-only tests, so nobody had noticed.
3. Write the new tags into the temp file via `metadata::write_tags`.
4. Compute the SHA256 hash of the written temp file.
5. Atomically `rename(2)` the temp file over the original — or, in Test Mode, over the tracked
   `_MeedyaManager` copy (see below).
6. Log the result — success with both hashes, or failure with an error message appended to
   the corruption log.

If any step fails, the temp file is deleted **only if this call created it** — a failed edit no
longer risks deleting a tracked copy that already holds an earlier successful edit — and the
original is left completely untouched. The atomic-rename step means the destination is always
either the fully-old or fully-new file, never a mix.

There is no cross-filesystem copy-and-verify variant, no file-lock detection, and no retry
queue anywhere in this module or elsewhere in the codebase — those are not implemented.

---

## Test Mode Interaction

`write_tags_safe` (via `mutate_file_safe`) checks `test_mode::is_enabled()` before doing
anything else. If Test Mode is on, it writes the new tags into a `<stem>_MeedyaManager.<ext>`
copy instead of the original, and records the pair in the Test Mode manifest. A second edit to a
file that already has a tracked copy edits that copy **in place** rather than starting over from
a fresh copy of the pristine original, so successive edits accumulate correctly — see
[Test Mode](test-mode.md) for the full picture. This redirection is now honoured by every real
edit path (`meedya edit`, the GTK metadata panel, the FFI layer), not just this function's own
tests.

---

## The Corruption Log

When `write_tags_safe` fails a step, it appends a line to:

```text
<config directory>/corruption.log
```

For example `~/.config/MeedyaManager/corruption.log` on Linux, `~/Library/Application
Support/MeedyaManager/corruption.log` on macOS, or `%APPDATA%\MeedyaManager\corruption.log` on
Windows — the same single config directory resolved by `mm_core::config::app_config_dir()`
(issue #212; see [configuration.md](configuration.md#configuration-file-location)), overridable
with `MM_CONFIG_DIR`. Each line is an ISO 8601 timestamp, the file path, and the failure message.
Because every real edit path now goes through `write_tags_safe` (issue #128, fixed), this log
will genuinely accumulate entries from ordinary use whenever a real write fails — it is no
longer exercised only by the module's own tests.

---

## What Actually Runs Today

Every real caller — `meedya edit`, the GTK metadata panel, and the FFI layer used by the
macOS and Windows apps — calls `mm_core::integrity`'s guarded functions
(`write_tags_safe`/`remove_tag_safe`/`embed_cover_art_safe`/`remove_cover_art_safe`), which
provide:

- A pre-write and post-write SHA256 hash, logged on success.
- Temp-file staging and an atomic rename over the original (or the tracked Test Mode copy).
- A corruption log entry on failure, in addition to the normal CLI/UI error message.
- Test Mode redirection — see [Test Mode](test-mode.md).

The one thing still missing: there is no cross-filesystem copy-and-verify variant, no file-lock
detection, and no retry queue. Those remain unimplemented.

---

## Checking a File's Hash

There is no dedicated CLI flag for this. `meedya debug` does not expose a `--verify-checksum`
option or print a SHA256 hash — it reports classification, tags, audio properties, cover art,
and companion files only (`crates/mm-cli/src/commands/debug.rs`). To compute a file's SHA256
yourself, use your platform's standard tool, e.g. `shasum -a 256 song.mp3` (macOS/Linux) or
`Get-FileHash song.mp3 -Algorithm SHA256` (Windows PowerShell).
