# Milestone M1 — Core Engine

> **(C) 2025-2026 MWBM Partners Ltd**

**Crate:** `mm-core` (`crates/mm-core/`)
**GitHub issues:** #40–#51 (see `docs/issues/github_issues.md` for the live, generated register)
**Status:** mostly complete — **2 of 12 issues reopened** during the 2026-09 documentation
reconciliation because the shipped code does not fully match its own issue title. See
[Known gaps](#known-gaps) below before treating this milestone as 100% done.

---

## Summary

M1 built the shared Rust core engine consumed by every platform UI and by `mm-cli`: config
loading, media classification, metadata read/write, file-system watching, rename simulation,
companion-file detection, application state, structured logging, and startup health checks.

---

## Issues

| # | State | Title |
| - | ----- | ----- |
| #40 | ✅ Closed | mm-core: Configuration loading (JSON5 + .env via serde + dotenvy) |
| #41 | ✅ Closed | mm-core: Media classification engine (4-level hierarchy) |
| #42 | ✅ Closed | mm-core: Metadata extraction via lofty crate |
| #43 | ✅ Closed | mm-core: Metadata tag writing via lofty |
| #44 | ✅ Closed | mm-core: Multi-value field handling (semicolon-delimited parsing) |
| #45 | 🔲 **Reopened** | mm-core: File system watcher (notify crate + polling fallback) |
| #46 | ✅ Closed | mm-core: Rename simulator (dry-run path computation) |
| #47 | ✅ Closed | mm-core: Filename character sanitizer (configurable replacement mappings) |
| #48 | ✅ Closed | mm-core: Companion file detector and grouper |
| #49 | ✅ Closed | mm-core: Application state manager + single-instance lock file |
| #50 | 🔲 **Reopened** | mm-core: Structured logging (tracing + PII redaction + daily rotation) |
| #51 | ✅ Closed | mm-core: Startup health checks + unified error types (thiserror) |

---

## What is actually implemented

- **Config** (`config/`) — JSON5 loading with `.env` fallback and environment-variable overrides.
- **Classifier** (`classify/`) — the four-level Group/Format/Class/Quality hierarchy.
- **Metadata** (`metadata/`) — real read/write via `lofty`.
- **Renamer** (`renamer/`) — `simulate_rename` correctly detects intra-batch destination
  collisions (`renamer/mod.rs:225`); this is the code path `meedya scan` should be using but
  currently is not for `--execute` — see issue #201.
- **Companion files** (`companion/`) — subtitle/lyrics/art/cue grouping.
- **State** (`state/`) — application state + single-instance lock file.
- **Health** (`health/`) — startup checks (config, folders, disk, writable).
- **Errors** (`error.rs`) — unified `thiserror` error types.
- **Watcher** (`watcher/`) — real native file-system events (`notify` crate) with filtering and
  an initial directory scan.
- **Logging** (`logging/`) — `tracing` initialised with a console layer and a JSON file layer.

## Known gaps

Both reopened issues describe a title deliverable that the code does not fully provide:

- **#45 — watcher.** `watcher/mod.rs:184` computes a debounce value and discards it; `:185` sets
  `with_poll_interval` on `NotifyConfig`, but `:188` always constructs a `RecommendedWatcher`
  (FSEvents/inotify/ReadDirectoryChanges) — the poll interval only matters for
  `notify::PollWatcher`, which is never constructed anywhere in the workspace. There is **no
  polling fallback, no debounce/coalescing, and no retry queue**, despite `lib.rs` and
  `PROJECT_STATUS.md` describing all three. `config::WatchConfig.poll_interval_secs` is parsed
  from JSON5/env but never consumed.
- **#50 — logging.** Console and JSON-file logging are real. **Daily rotation is absent** — one
  file named with the current date is opened once at startup and never rotated
  (`logging/mod.rs:84-99`); there is no `tracing-appender` rolling-file layer. PII-redaction
  helpers (`redact_pii`, `redact_path`, `redact_username`, `logging/mod.rs:120-172`) exist and are
  unit-tested, but **nothing wires them into the subscriber** — no non-test code path calls them,
  so no output actually gets redacted today.

Neither gap is a correctness regression from a working feature; both are deliverables the
original issue title promised that the implementation stopped short of. Reopened 2026-09-03
during the project-wide issue reconciliation sweep (see `.claude/HANDOFF.md`).

## Tests

`mm-core` currently carries 512 `#[test]`/`#[tokio::test]` functions (14,830 LOC). Older docs
citing "217 tests" for M1 describe the count at the time #50 was first closed (2025-era) and
predate the M2–M10 work, the MeedyaSuite-core migration, and this milestone's own subsequent
growth — see `docs/changelog.md` for the current workspace-wide total.
