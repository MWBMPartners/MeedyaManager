# MeedyaManager — GitHub Issues Register

> **(C) 2025-2026 MWBM Partners Ltd**
>
> This file is the authoritative local record of all GitHub Issues for the
> MeedyaManager project. All issues are created on GitHub before work begins
> and closed after verification. This document mirrors that state.

---

## Issue Status Key

| Symbol | Meaning |
| ------ | ------- |
| ✅ | Closed — complete |
| 🔲 | Open — not yet started |
| 🔄 | Open — in progress |

---

## M0 — Repository Setup & Scaffolding (Issues #19–#39)

| # | Title | Status |
| - | ----- | ------ |
| #19 | Archive Python v1.x codebase at `v1.5-M6-python-final` | ✅ |
| #20 | Delete Python source tree from main branch | ✅ |
| #21 | Initialise Cargo workspace with 8 crates | ✅ |
| #22 | Scaffold `mm-core` crate with stub modules | ✅ |
| #23 | Scaffold `mm-cli` crate with stub `main.rs` | ✅ |
| #24 | Scaffold `mm-ffi` crate (UniFFI + cbindgen stubs) | ✅ |
| #25 | Scaffold `mm-providers`, `mm-cloud`, `mm-export`, `mm-server`, `mm-update` stubs | ✅ |
| #26 | Scaffold `mm-gtk` crate (GTK4 Linux shell stub) | ✅ |
| #27 | Create macOS SwiftUI project scaffold (`macos/`) | ✅ |
| #28 | Create Windows WinUI 3 project scaffold (`windows/`) | ✅ |
| #29 | Set up Rust toolchain config (rustfmt, clippy, deny, rust-toolchain.toml) | ✅ |
| #30 | Create 8 GitHub Actions CI/CD workflows | ✅ |
| #31 | Create GitHub Projects v2 board with 11 milestones | ✅ |
| #32 | Implement automated version management (`version-bump.yml`) | ✅ |
| #33 | Implement CI version-sync check in `ci-rust.yml` | ✅ |
| #34 | Enhance `release.yml` with 5-platform builds + checksums | ✅ |
| #35 | Create GitHub Wiki — Version Management page | ✅ |
| #36 | Create GitHub Wiki — Release Process page | ✅ |
| #37 | Create GitHub Wiki — CI/CD Pipelines page | ✅ |
| #38 | Write `docs/Dev_Notes.md` (versioning, release, CI reference) | ✅ |
| #39 | Write initial README, Project_Plan.md, PROJECT_STATUS.md, ROADMAP.md | ✅ |

---

## M1 — Core Engine (Issues #40–#51)

| # | Title | Status |
| - | ----- | ------ |
| #40 | Implement error types module (`errors.rs`) with `thiserror` | ✅ |
| #41 | Implement config module (JSON5 + `.env` + env-var override) | ✅ |
| #42 | Implement media classifier (Group/Format/Class/Quality 4-level) | ✅ |
| #43 | Implement metadata extraction + writing (`lofty` integration) | ✅ |
| #44 | Implement file system watcher (`notify` + debounce + filtering) | ✅ |
| #45 | Implement rename simulator + filename sanitizer | ✅ |
| #46 | Implement companion file detector (subtitles, lyrics, art, cue) | ✅ |
| #47 | Implement state manager + single-instance lock file | ✅ |
| #48 | Implement structured logging (tracing + PII redaction) | ✅ |
| #49 | Implement health checks (config, folders, disk, writable) | ✅ |
| #50 | Wire all mm-core modules + achieve 217 passing tests | ✅ |
| #51 | Update docs for M1 completion + bump version to `v0.2.0` | ✅ |

---

## M2 — Rule Engine (no separate issues — done inline)

| # | Title | Status |
| - | ----- | ------ |
| — | Implement lexer (tokenizer: tags, functions, literals, legacy detection) | ✅ |
| — | Implement recursive descent parser (AST, 50-level depth guard) | ✅ |
| — | Implement tag registry (40+ bidirectional mappings, virtual tags) | ✅ |
| — | Implement 24 template functions (logical, string, numeric, lookup) | ✅ |
| — | Implement evaluator (EvalContext, multi-value, missing tag modes) | ✅ |
| — | Implement declarative rule system (conditions, operators, priority) | ✅ |
| — | Wire renamer integration + config extension | ✅ |
| — | Achieve 182 tests + bump version to `v0.3.0` | ✅ |

---

## M3 — CLI (Issues #52–#62)

| # | Title | Status |
| - | ----- | ------ |
| #52 | Implement shared output infrastructure (`output.rs`, `ExitCode`) | ✅ |
| #53 | Implement CLI context (`context.rs`, `CliContext`) | ✅ |
| #54 | Implement `meedya debug` — single-file metadata inspector | ✅ |
| #55 | Implement `meedya rule` — template validator + tag listing + test | ✅ |
| #56 | Implement `meedya config` — show/path/init/export/import | ✅ |
| #57 | Implement `meedya scan` — directory scan + rename preview + execute | ✅ |
| #58 | Implement `meedya edit` — metadata write (--set, --remove, --cover) | ✅ |
| #59 | Implement `meedya watch` — foreground watcher with event logging | ✅ |
| #60 | Implement `meedya lookup` — provider search (stub for M5) | ✅ |
| #61 | Implement `meedya report-bug` — system info + log collection | ✅ |
| #62 | Achieve 45 CLI tests + bump version to `v0.4.0` | ✅ |

---

## M4 — FFI Layer & Native UI Shells (Issues #63–#72)

| # | Title | Status |
| - | ----- | ------ |
| #63 | Implement `mm-ffi` crate — UniFFI scaffolding + types + callbacks | ✅ |
| #64 | Implement `mm-ffi` UniFFI API (8 exported functions wired to mm-core) | ✅ |
| #65 | Implement `mm-ffi` C API (9 `#[no_mangle]` functions + JSON transport) | ✅ |
| #66 | Set up cbindgen + build.rs — generates `include/mm_ffi.h` | ✅ |
| #67 | Implement `mm-gtk` main window (AdwTabView, 4 tabs, AboutDialog) | ✅ |
| #68 | Implement `mm-gtk` scan, metadata, rules, settings panels | ✅ |
| #69 | Implement macOS SwiftUI shell (AppState, ContentView, 4 views, MmCore.swift) | ✅ |
| #70 | Implement Windows WinUI 3 shell (MainWindow, 4 pages, MmCore.cs) | ✅ |
| #71 | Achieve 20 mm-ffi tests + GTK4 state tests | ✅ |
| #72 | Bump version to `v0.5.0` + update docs | ✅ |

---

## M5 — Metadata Lookup Providers (Issues #73–#84)

| # | Title | Status |
| - | ----- | ------ |
| #73 | Implement provider traits (MetadataProvider, SearchQuery, ProviderResult) | ✅ |
| #74 | Implement credential resolution (4-tier: env/config/keyring/file) | ✅ |
| #75 | Implement rate limiter (token bucket, per-provider, registry) | ✅ |
| #76 | Implement fuzzy match scoring (weighted, MatchScorer, rank_results) | ✅ |
| #77 | Implement cover art helpers (size selection, deduplication, URL validators) | ✅ |
| #78 | Implement music providers: MusicBrainz, Spotify, AppleMusic, Deezer | ✅ |
| #79 | Implement 6 stub music providers (YouTube Music, Amazon, Pandora, etc.) | ✅ |
| #80 | Implement video providers: TMDB, TheTVDB, OMDb, Apple TV, iTunes Store | ✅ |
| #81 | Implement podcast provider: Apple Podcasts | ✅ |
| #82 | Implement identifier providers: ISRC, EIDR, ISWC | ✅ |
| #83 | Implement provider registry + fan-out search | ✅ |
| #84 | Achieve 332 tests + bump version to `v0.6.0` + update docs | ✅ |

---

## M6 — Full Native UI (Issues #85–#93)

| # | Title | Status |
| - | ----- | ------ |
| #85 | Implement GTK4 Lookup panel (search + results + cover art) | ✅ |
| #86 | Implement GTK4 full rule builder (template + preview + tag pills) | ✅ |
| #87 | Implement macOS Lookup view + rule builder (full implementation) | ✅ |
| #88 | Implement Windows LookupPage + rule builder | ✅ |
| #89 | Implement drag-and-drop folder import (GTK4/macOS/Windows) | ✅ |
| #90 | Implement real settings save to disk (all 3 platforms) | ✅ |
| #91 | Implement dark/light theme toggle (GTK4 adw::StyleManager) | ✅ |
| #92 | Add error dialogs (GTK4 adw::AlertDialog, macOS/Windows equivalents) | ✅ |
| #93 | Achieve ~90 UI tests + bump version to `v0.7.0` + update docs | ✅ |

---

## M7 — Cloud Storage Monitoring (Issues #94–#102)

| # | Title | Status |
| - | ----- | ------ |
| #94 | Implement `mm-cloud` trait layer (CloudProvider, CloudError, ChangeSet, etc.) | ✅ |
| #95 | Implement SyncManager (polling, conflict resolution, delta cursor) | ✅ |
| #96 | Implement OneDrive provider (Microsoft Graph API, delta queries) | ✅ |
| #97 | Implement Google Drive provider (Drive API v3, changes.list) | ✅ |
| #98 | Implement Dropbox provider (API v2, cursor-based delta) | ✅ |
| #99 | Implement MEGA stub provider (no official API) | ✅ |
| #100 | Implement iCloud stub provider (macOS FileProvider only) | ✅ |
| #101 | Implement Cloud UI tab on all 3 platforms (GTK4, macOS, Windows) | ✅ |
| #102 | Achieve ~90 tests + bump version to `v0.8.0` + update docs | ✅ |

---

## M8 — Packaging & Public Beta (Issues #103–#111)

| # | Title | Status |
| - | ----- | ------ |
| #103 | Implement `mm-update` crate (UpdateChecker, ReleaseInfo, semver comparison) | ✅ |
| #104 | Create Linux packaging manifests (Flatpak, Snap, AppImage, .deb) | ✅ |
| #105 | Create macOS entitlements + DMG creation script | ✅ |
| #106 | Create WinGet manifest (MWBM.MeedyaManager.yaml) | ✅ |
| #107 | Add GTK4 AdwBanner update notification (above tab bar) | ✅ |
| #108 | Add macOS "Updates" section in SettingsView | ✅ |
| #109 | Add Windows InfoBar + CheckForUpdatesAsync() | ✅ |
| #110 | Update `release.yml` (DMG creation, deb/AppImage build steps) | ✅ |
| #111 | Achieve ~30 tests + bump version to `v0.9.0` + update docs | ✅ |

---

## M9 — Database Export (Issues #112–#119)

| # | Title | Status |
| - | ----- | ------ |
| #112 | Implement `mm-export` trait layer (DatabaseExporter, DbDialect, ExportRow, ExportConfig, ExportStats, ExportError) | ✅ |
| #113 | Implement SchemaBuilder with dialect-aware DDL for all 5 backends | ✅ |
| #114 | Implement SQLite backend (INSERT OR REPLACE upsert) | ✅ |
| #115 | Implement MySQL + MariaDB backends (ON DUPLICATE KEY UPDATE upsert) | ✅ |
| #116 | Implement PostgreSQL backend (ON CONFLICT DO UPDATE, $1 params) | ✅ |
| #117 | Implement SQL Server backend (T-SQL MERGE, @param_name style) | ✅ |
| #118 | Implement `meedya export` CLI command (BackendChoice, detect_backend, redact_dsn, --show-schema) | ✅ |
| #119 | Implement Export UI tab on all 3 platforms (GTK4, macOS, Windows) + achieve ~90 tests + bump version to `v0.10.0` | ✅ |

---

## M10 — Secure Media Server + Public Release (Issues #120–#127)

| # | Title | Status |
| - | ----- | ------ |
| #120 | Implement `mm-server` core: axum router, TLS (rustls), config | ✅ |
| #121 | Implement JWT authentication (jsonwebtoken) + middleware | ✅ |
| #122 | Implement media streaming endpoints (Range requests, partial content) | ✅ |
| #123 | Implement REST API endpoints (library, search, metadata, export) | ✅ |
| #124 | Implement `meedya serve` CLI command | ✅ |
| #125 | Implement Server UI tab on all 3 platforms (GTK4, macOS, Windows) | ✅ |
| #126 | Final packaging pass: sign all release artifacts, update WinGet/Flathub | ✅ |
| #127 | Achieve ~90 tests + bump version to `v1.0.0` + full release docs + public release | ✅ |

---

## Cross-Cutting Issues

| # | Title | Priority | Status |
| - | ----- | -------- | ------ |
| #128 | Accessibility support: VoiceOver (macOS), Narrator (Windows), AT-SPI (Linux) | High | ✅ |
| #129 | Release binary hardening: LTO, strip, panic=abort, PIE, code signing | — | ✅ |
| #130 | Translation / Internationalisation (i18n) support — gettextrs CLI+Linux, .xcstrings macOS, .resw Windows | Medium | ✅ |
| #131 | Windows process check via OpenProcess (platform-specific state manager enhancement) | Low | ✅ |

---

## Prioritised Backlog (v1.1.0+)

Issues ordered by recommended implementation sequence:

| Priority | # | Title | Rationale |
| -------- | - | ----- | --------- |
| 1 | #128 | Accessibility support | App Store requirement; legal compliance (WCAG 2.1); high user impact |
| 2 | #130 | Translation / i18n | Expands addressable market; localisable string infrastructure needed before scaling |
| 3 | #131 | Windows OpenProcess check | Minor correctness gap in state manager; low risk |

---

## GitHub Milestones

Create the following milestones on GitHub (Settings → Milestones):

| Milestone | Due Date | Description |
| --------- | -------- | ----------- |
| M0 — Repository Setup | — | Archive Python, create Cargo workspace, scaffold all platforms |
| M1 — Core Engine | — | mm-core: config, classify, metadata, watcher, renamer |
| M2 — Rule Engine | — | Lexer, parser, tag registry, template functions, evaluator |
| M3 — CLI | — | 8 commands: scan, debug, edit, rule, watch, lookup, config, report-bug |
| M4 — FFI & Shells | — | UniFFI, cbindgen, GTK4/SwiftUI/WinUI 3 shells |
| M5 — Providers | — | 19 metadata lookup providers, credentials, rate limiting |
| M6 — Full Native UI | — | Lookup panel, rule builder, cover art, DnD, settings save |
| M7 — Cloud Storage | — | OneDrive, Google Drive, Dropbox, MEGA stub, iCloud stub |
| M8 — Packaging | — | mm-update, Linux/macOS/Windows packaging, update notifications |
| M9 — Database Export | — | mm-export: 5 backends, SchemaBuilder, meedya export CLI |
| M10 — Media Server | — | axum server, JWT auth, streaming, REST API, v1.0.0 release |

---

## GitHub Project Board — MeedyaManager v0.x

Create a **GitHub Projects v2** board with the following columns/views:

### Views
1. **Board** — Kanban with columns: Backlog / In Progress / Review / Done
2. **Roadmap** — Timeline view grouped by milestone
3. **By Milestone** — Table view filtered by milestone field
4. **Issues** — All issues flat table
5. **Active** — Filtered to open issues only

### Custom Fields
| Field | Type | Values |
| ----- | ---- | ------ |
| Milestone | Single select | M0 through M10 |
| Platform | Multi-select | All / Rust / macOS / Windows / Linux |
| Priority | Single select | Critical / High / Medium / Low |
| Type | Single select | Feature / Bug / Docs / CI/CD / Refactor |

---

> *Last updated: 2026-03-06 (v1.1.0 — #128 Accessibility, #130 i18n, #131 Windows OpenProcess all closed; all post-v1.0 issues resolved)*
