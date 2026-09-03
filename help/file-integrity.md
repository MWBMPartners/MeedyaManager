# File Integrity — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager includes a SHA256-verified, atomic-write mechanism for **metadata tag writes**,
implemented in `crates/mm-core/src/integrity.rs`. This page describes exactly what it does —
and, importantly, that it is currently **not wired into any of MeedyaManager's tag-writing
code paths**, so the protection it provides is not yet active in practice.

> ⚠️ **`write_tags_safe` is never called.** The integrity module's guarded write function,
> `integrity::write_tags_safe`, is exercised only by its own unit tests. All three real
> consumers — `meedya edit` (`crates/mm-cli/src/commands/edit.rs:116`), the Linux GTK metadata
> panel (`crates/mm-gtk/src/ui/metadata_panel.rs:313`), and the FFI layer used by the macOS/
> Windows UIs (`crates/mm-ffi/src/uniffi_api.rs:233`) — call `metadata::write_tags` directly,
> which writes straight to the original file with no pre-write hash, no temp-file staging, and
> no post-write verification. See issue
> [#128](https://github.com/MWBMPartners/MeedyaManager/issues/128) (reopened).
>
> This page also does **not** cover file *moves/renames* — those go through a separate module
> (`crates/mm-core/src/renamer`) with its own conflict-detection logic, which has its own known
> issue (`meedya scan --execute` can silently overwrite a file — see
> [#201](https://github.com/MWBMPartners/MeedyaManager/issues/201)). This page is about
> **tag writes** only.

---

## Table of Contents

1. [What `write_tags_safe` Does](#what-write_tags_safe-does)
2. [Test Mode Interaction](#test-mode-interaction)
3. [The Corruption Log](#the-corruption-log)
4. [What Actually Runs Today](#what-actually-runs-today)
5. [Checking a File's Hash](#checking-a-files-hash)

---

## What `write_tags_safe` Does

When it *is* called (currently: only from its own tests), `write_tags_safe` guards a metadata
write with the following steps:

1. Compute the SHA256 hash of the original file.
2. Copy the original to `<path>.meedya_tmp` (same directory, so the later rename stays on one
   filesystem).
3. Write the new tags into the temp file via `metadata::write_tags`.
4. Compute the SHA256 hash of the written temp file.
5. Atomically `rename(2)` the temp file over the original.
6. Log the result — success with both hashes, or failure with an error message appended to
   the corruption log.

If any step fails, the temp file is deleted and the original is left completely untouched —
there is no partial-write state a user can end up in *if this path is used*. The atomic-rename
step means the destination is always either the fully-old or fully-new file, never a mix.

There is no cross-filesystem copy-and-verify variant, no file-lock detection, and no retry
queue anywhere in this module or elsewhere in the codebase — those are not implemented.

---

## Test Mode Interaction

`write_tags_safe` checks `test_mode::is_enabled()` before doing anything else. If Test Mode is
on, it writes the new tags into a `<stem>_MeedyaManager.<ext>` copy instead of the original,
and records the pair in the Test Mode manifest — see [Test Mode](test-mode.md) for the full
picture, including the same enforcement gap (Test Mode is only honoured by this unused
function, so it is not actually enforced by any real edit path either).

---

## The Corruption Log

When `write_tags_safe` fails a step, it appends a line to:

```text
<OS config directory>/meedyamanager/corruption.log
```

For example `~/.config/meedyamanager/corruption.log` on Linux/macOS or
`%APPDATA%\meedyamanager\corruption.log` on Windows (`dirs::config_dir()` joined with
`meedyamanager`). Each line is an ISO 8601 timestamp, the file path, and the failure message.
Because the function that writes this log is not called from any real edit path today, the
log will not accumulate entries from normal use of MeedyaManager — only from the module's own
tests, or from any future code that starts calling `write_tags_safe`.

---

## What Actually Runs Today

Every real caller — `meedya edit`, the GTK metadata panel, and the FFI layer used by the
macOS and Windows apps — calls `metadata::write_tags` directly:

- No pre-write or post-write hash.
- No temp-file staging or atomic rename — the tag write happens in place, using the
  underlying `lofty` crate's own save routine.
- No corruption log entry on failure — errors surface as a normal CLI/UI error message.
- No Test Mode redirection — see [Test Mode](test-mode.md).

This is functionally similar to what most tag editors do, and `lofty`'s own writer has its own
internal safeguards, but MeedyaManager's specific SHA256/atomic-rename guarantees described
above do not apply until #128 is resolved.

---

## Checking a File's Hash

There is no dedicated CLI flag for this. `meedya debug` does not expose a `--verify-checksum`
option or print a SHA256 hash — it reports classification, tags, audio properties, cover art,
and companion files only (`crates/mm-cli/src/commands/debug.rs`). To compute a file's SHA256
yourself, use your platform's standard tool, e.g. `shasum -a 256 song.mp3` (macOS/Linux) or
`Get-FileHash song.mp3 -Algorithm SHA256` (Windows PowerShell).
