# MeedyaManager — Project Status

> **(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)**
>
> Cross-platform media manager and auto-organizer — Rust core + native UIs

---

## Quick Status

| Item | Status |
| ---- | ------ |
| **Current Milestone** | M3 — CLI — **Complete** |
| **Overall Progress** | **36%** (4 of 11 milestones complete) |
| **Latest Version** | `v2.0.0-alpha.4` |
| **Python v1.x** | Archived at tag `v1.5-M6-python-final` |
| **Build Status** | ![CI](https://github.com/MWBMPartners/MeedyaManager/actions/workflows/ci-rust.yml/badge.svg) |

---

## Milestone Progress

### M0 — Repository Setup & Scaffolding *(Complete)*

> Started: 2026-03-04 | Version: `v2.0.0-alpha.1`

**Progress: 100%** | Issues: #19-#31, #32-#39 (all closed)

| Deliverable | Status | Notes |
| ----------- | ------ | ----- |
| Archive Python v1.x codebase | Done | Tagged `v1.5-M6-python-final` |
| Delete Python source tree | Done | All `.py` files removed |
| Cargo workspace with 8 crates | Done | mm-core, mm-providers, mm-cloud, mm-export, mm-server, mm-cli, mm-ffi, mm-gtk |
| macOS SwiftUI scaffold | Done | `macos/` with Package.swift |
| Windows WinUI 3 scaffold | Done | `windows/` with .sln/.csproj |
| Rust toolchain configuration | Done | `.rustfmt.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml` |
| CI/CD workflows (8 workflows) | Done | ci-rust, ci-macos, ci-windows, ci-linux, version-bump, release, audit, docs |
| GitHub Projects v2 board | Done | 11 milestones, custom fields, 5 views |
| Documentation update | Done | All `.md` files rewritten |
| Automated version management | Done | `version-bump.yml` workflow, version-sync CI check |
| Release build pipeline | Done | `release.yml` with 5 platform builds, checksums, draft releases |
| GitHub Wiki | Done | Version Management, Release Process, CI/CD Pipelines pages |
| Developer notes | Done | `docs/Dev_Notes.md` |

---

### M1 — Core Engine *(Complete)*

> Started: 2026-03-04 | Completed: 2026-03-05 | Version: `v2.0.0-alpha.2`

**Progress: 100%** | Issues: #40-#51 | **217 tests** (214 unit + 3 doc-tests)

| Deliverable | Status | Tests |
| ----------- | ------ | ----- |
| Error types (`thiserror`) | Done | 5 |
| Config module (JSON5 + .env + env overrides) | Done | 22 |
| Media classification (4-level: Group/Format/Class/Quality) | Done | 38 |
| Metadata extraction & writing (`lofty`) | Done | 36 |
| File watcher (`notify` + debounce + filtering) | Done | 15 |
| Rename simulator + filename sanitizer | Done | 16 |
| Companion file detector (subtitles, lyrics, art, cue) | Done | 16 |
| State manager + single-instance lock file | Done | 13 |
| Structured logging (tracing + PII redaction) | Done | 13 |
| Health checks (config, folders, disk, writable) | Done | 14 |
| Rule engine (stub — deferred to M2) | Stub | 0 |

---

### M2 — Rule Engine *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v2.0.0-alpha.3`

**Progress: 100%** | **182 tests** (181 unit + 1 doc-test)

| Deliverable | Status | Tests |
| ----------- | ------ | ----- |
| Lexer (tokenizer: tags, functions, literals, legacy detection) | Done | 26 |
| Parser (recursive descent, AST, 50-level depth guard) | Done | 24 |
| Tag registry (40+ bidirectional mappings, virtual tags) | Done | 24 |
| Template functions (24: logical, string, numeric, lookup, extensions) | Done | 47 |
| Evaluator (EvalContext, multi-value, missing tag modes) | Done | 30 |
| Rule system (conditions, operators, priority ordering, apply_rules) | Done | 30 |
| Renamer integration (`simulate_rename_with_rules`) | Done | — |
| Config extension (`rules` + `missing_tag_mode` in RenameConfig) | Done | — |

---

### M3 — CLI *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v2.0.0-alpha.4`

**Progress: 100%** | Issues: #52-#62 (all closed) | **45 tests**

| Deliverable | Status | Tests |
| ----------- | ------ | ----- |
| Output infrastructure (`output.rs`) | Done | 4 |
| CLI context (`context.rs`) | Done | 3 |
| `main.rs` restructure (Commands enum, global flags, dispatch) | Done | — |
| `meedya debug` — single-file metadata inspector | Done | 5 |
| `meedya rule` — template validation, tag listing, test, legacy detection | Done | 6 |
| `meedya config` — show, path, init, export, import | Done | 5 |
| `meedya scan` — directory scan + rename preview + execute | Done | 7 |
| `meedya edit` — metadata write (--set, --remove, --cover) | Done | 6 |
| `meedya watch` — foreground watcher with event logging | Done | 4 |
| `meedya lookup` — provider search (stub for M5) | Done | 2 |
| `meedya report-bug` — system info + log collection | Done | 3 |
| Documentation updates | Done | — |

---

### M4 — FFI Layer & Native UI Shells *(Planned)*

> Target: `v2.0.0-alpha.5`

UniFFI (Swift), cbindgen (C#), async callbacks. Basic SwiftUI/WinUI 3/GTK4 shells.

---

### M5 — Metadata Lookup Providers *(Planned)*

> Target: `v2.0.0-alpha.6`

19 providers via `reqwest`/`tokio`: Music (10), Video (5), Podcasts (1), Identifiers (3).

---

### M6 — Full Native UI *(Planned)*

> Target: `v2.0.0-beta.1`

Complete views on all 3 platforms: Rule Builder, Metadata Editor, Lookup Panel, accessibility.

---

### M7 — Cloud Storage Monitoring *(Planned)*

> Target: `v2.0.0-beta.2`

OneDrive, Google Drive, Dropbox, MEGA, iCloud.

---

### M8 — Packaging & Public Release *(Planned)*

> Target: `v2.0.0-beta.3`

App Store (macOS), Microsoft Store (Windows), Flatpak/Snap (Linux). Code signing, notarization.

---

### M9 — Database Export *(Planned)*

> Target: `v2.0.0-rc.1`

MySQL, MariaDB, SQL Server, SQLite, PostgreSQL via `sqlx`/`tiberius`.

---

### M10 — Secure Media Server *(Planned)*

> Target: `v2.0.0`

`axum` HTTP server, REST API, JWT auth, media streaming, TLS.

---

## Architecture Health

| Crate / Component | Path | Status |
| ----------------- | ---- | ------ |
| `mm-core` | `crates/mm-core/` | **M2 Complete** (399 tests) |
| `mm-providers` | `crates/mm-providers/` | Scaffold (stubs) |
| `mm-cloud` | `crates/mm-cloud/` | Scaffold (stubs) |
| `mm-export` | `crates/mm-export/` | Scaffold (stubs) |
| `mm-server` | `crates/mm-server/` | Scaffold (stubs) |
| `mm-cli` | `crates/mm-cli/` | **M3 Complete** (45 tests) |
| `mm-ffi` | `crates/mm-ffi/` | Scaffold (stubs) |
| `mm-gtk` | `crates/mm-gtk/` | Scaffold (Linux only) |
| macOS SwiftUI app | `macos/` | Shell (tabs, empty views) |
| Windows WinUI 3 app | `windows/` | Shell (NavigationView, Mica) |

---

## Platform Support Matrix

| Platform | Architecture | CI Tested | Native UI | Package Format |
| -------- | ------------ | --------- | --------- | -------------- |
| macOS | Apple Silicon (arm64) | Yes | SwiftUI | .dmg / .tar.gz |
| Windows | x64 | Yes | WinUI 3 | MSIX / .zip |
| Windows | ARM64 | Planned | WinUI 3 | MSIX / .zip |
| Linux | x86_64 | Yes | GTK4 | Flatpak / Snap / AppImage / .deb |
| Linux | ARM64 | Planned | GTK4 | .tar.gz |

---

## CI/CD Infrastructure

| Workflow | File | Status |
| -------- | ---- | ------ |
| Rust Core CI | `ci-rust.yml` | Active (format, lint, test, version-sync) |
| macOS CI | `ci-macos.yml` | Active |
| Windows CI | `ci-windows.yml` | Active |
| Linux CI | `ci-linux.yml` | Active |
| Version Bump | `version-bump.yml` | Active (manual trigger) |
| Release Build | `release.yml` | Active (tag trigger) |
| Security Audit | `audit.yml` | Active (weekly + push) |
| Documentation | `docs.yml` | Active |

---

## Recent Activity

| Date | Activity |
| ---- | -------- |
| 2026-03-05 | **M3 Complete** — CLI: 8 commands (scan, debug, edit, rule, watch, lookup, config, report-bug), shared output infrastructure, CLI context, dual output modes (Human/JSON), 45 new tests (444 total) |
| 2026-03-05 | **M2 Complete** — Rule engine: lexer, recursive descent parser, 40+ tag registry, 24 template functions, evaluator with EvalContext, declarative rule system, renamer integration, config extension. 182 new tests (399 total) |
| 2026-03-05 | **M1 Complete** — All mm-core modules implemented: config, classify, metadata, watcher, renamer, companion, state, logging, health. 217 tests passing (Issues #40-#51) |
| 2026-03-04 | **Version/Release Infrastructure** — Added version-bump workflow, version-sync CI check, enhanced release pipeline with checksums, created GitHub Wiki, Dev_Notes.md (Issues #32-#39) |
| 2026-03-04 | **M0 Complete** — Archived Python, created Cargo workspace, scaffolded all platforms, set up CI/CD, GitHub Projects v2 (Issues #19-#31) |
| 2026-03-04 | **v1.x archived** — Tagged `v1.5-M6-python-final` (1007 tests, 6 milestones, 19 providers) |

---

> *This file is updated with each significant change. For detailed changelog, see [docs/CHANGELOG.md](docs/CHANGELOG.md).*
>
> *Last updated: 2026-03-05 (M3 complete)*
