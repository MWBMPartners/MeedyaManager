# File Integrity — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager uses several mechanisms to protect your media files from corruption during rename and move operations.

---

## Table of Contents

1. [Atomic Renames](#atomic-renames)
2. [SHA256 Integrity Checking](#sha256-integrity-checking)
3. [File Lock Detection](#file-lock-detection)
4. [Corruption Recovery](#corruption-recovery)
5. [Integrity CLI Commands](#integrity-cli-commands)

---

## Atomic Renames

When MeedyaManager moves or renames a file, it uses an **atomic rename strategy** to ensure the operation either completes fully or not at all — a partial write can never leave you with a corrupted or truncated file.

### How It Works

1. The file is written to a temporary path alongside the destination: `<destination>.meedya_tmp`
2. On success, the temporary file is atomically renamed to the final destination using the operating system's `rename(2)` syscall (or equivalent on Windows)
3. Because `rename` on the same filesystem is atomic, the destination file is either the old version or the new version — never a mix of both
4. If anything fails mid-write, the `.meedya_tmp` file is discarded and the source file is left untouched

### Cross-Filesystem Moves

When the source and destination are on different filesystems (e.g. moving from an external drive to your main drive), a true atomic `rename` is not possible. In this case, MeedyaManager:

1. Copies the source file to `<destination>.meedya_tmp`
2. Verifies the copy's SHA256 hash matches the source
3. Only then removes the source file and renames the temp file to the final destination

This ensures the source file is never deleted unless the copy is confirmed intact.

---

## SHA256 Integrity Checking

MeedyaManager can compute and verify SHA256 checksums for your media files.

### Automatic Verification (Cross-Filesystem Moves)

During cross-filesystem move operations, checksums are computed automatically — no configuration required. If the checksum does not match after copying, the copy is discarded and an error is logged.

### Manual Integrity Check

```bash
# Compute the checksum of a file
meedya debug path/to/file.mp3 --json | jq .sha256

# Verify a file against a known checksum
meedya debug path/to/file.mp3 --verify-checksum <expected_sha256>
```

### Integrity Log

When a checksum mismatch is detected, the event is written to the integrity log:

| Platform | Log Path |
| -------- | -------- |
| **macOS** | `~/Library/Logs/MeedyaManager/integrity.log` |
| **Linux** | `~/.local/state/MeedyaManager/logs/integrity.log` |
| **Windows** | `%LOCALAPPDATA%\MeedyaManager\logs\integrity.log` |

---

## File Lock Detection

MeedyaManager checks whether a file is open in another application before attempting to rename or move it. This prevents:

- Corrupting a file that is currently being downloaded or written
- Interfering with a file that is open in a media player

### Retry Queue

When a file is locked, MeedyaManager:

1. Logs a warning: `File in use — queued for retry`
2. Adds the file to an in-memory retry queue
3. Retries the operation at the next watcher cycle (configurable via `watch.debounce_ms`)

No user action is required. Once the other application releases the file, MeedyaManager processes it automatically.

---

## Corruption Recovery

If MeedyaManager is interrupted (e.g. power failure, crash) during a file operation, recovery depends on what stage the operation was at:

| Stage | Recovery |
| ----- | -------- |
| Before write started | Source file untouched — no recovery needed |
| During temp file write | `.meedya_tmp` file exists — safely deleted on next startup |
| After temp file complete, before rename | Same as above |
| After atomic rename | Operation completed successfully |

On startup, MeedyaManager scans for and removes any orphaned `.meedya_tmp` files from previous interrupted operations.

---

## Integrity CLI Commands

```bash
# Inspect a file including its classification, tags, and file hash
meedya debug path/to/file.mp3

# Use --json for machine-readable output
meedya debug path/to/file.mp3 --json

# Generate a bug report that includes integrity log summary
meedya report-bug
```
