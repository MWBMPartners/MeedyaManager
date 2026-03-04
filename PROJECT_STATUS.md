# 📊 MeedyaManager — Project Status

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**
>
> 🎧📁 Cross-platform media manager and auto-organizer — Rust core + native UIs

---

## 🏁 Quick Status

| Item | Status |
|------|--------|
| **Current Milestone** | M0 — Repository Setup & Rust Migration |
| **Overall Progress** | ░░░░░░░░░░ **0%** (0 of 11 milestones) |
| **Latest Version** | `v2.0.0-alpha.1` |
| **Python v1.x** | Archived at tag `v1.5-M6-python-final` |
| **Build Status** | ![CI](https://github.com/MWBMPartners/MeedyaManager/actions/workflows/rust-ci.yml/badge.svg) |

---

## 📈 Milestone Progress

### 🔧 M0 — Repository Setup & Rust Migration *(In Progress)*

> 🗓️ Started: 2026-03-04 | 📦 Target: `v2.0.0-alpha.1`

**Progress: ██████████ 100%**

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Archive Python v1.x codebase | ✅ Done | Tagged `v1.5-M6-python-final` |
| Delete Python source tree | ✅ Done | Removed `core/`, `cli/`, `ui/`, `metadata/`, `utils/`, `tests/` |
| Cargo workspace with 8 crates | ✅ Done | mm-core, mm-cli, mm-ffi, mm-metadata, mm-rules, mm-cloud, mm-export, mm-service |
| macOS SwiftUI scaffold | ✅ Done | `native/macos/` with Xcode project |
| Windows WinUI 3 scaffold | ✅ Done | `native/windows/` with .csproj |
| Rust toolchain configuration | ✅ Done | `rust-toolchain.toml` (stable) |
| CI/CD workflows (7 workflows) | ✅ Done | rust-ci, swiftui-ci, winui-ci, gtk-ci, release, lint, docs |
| GitHub Projects v2 board | ✅ Done | 11 milestones, issue templates |
| Documentation update | ✅ Done | All .md files rewritten for Rust era |

---

### 🔲 M1 — Core Engine *(Planned)*

> Rust rewrite of file watcher, metadata extraction, media classification, and rename engine.

---

### 🔲 M2 — CLI Frontend *(Planned)*

> `clap`-based CLI with scan, debug, watch, rule, and edit subcommands.

---

### 🔲 M3 — Rule Engine & Companion Files *(Planned)*

> MusicBee-inspired template parser in Rust with `nom` or `pest`. Companion file tracking.

---

### 🔲 M4 — Metadata Editor *(Planned)*

> Tag reading/writing via `lofty` crate. Cover art management.

---

### 🔲 M5 — Metadata Lookup *(Planned)*

> Provider framework with async HTTP (`reqwest`/`tokio`). Music, video, podcast, identifier providers.

---

### 🔲 M6 — Native GUI — macOS *(Planned)*

> SwiftUI app consuming Rust core via UniFFI. Liquid Glass on macOS 26+.

---

### 🔲 M7 — Native GUI — Windows & Linux *(Planned)*

> WinUI 3 (C#) on Windows, GTK4 (Rust `gtk4` crate) on Linux. Both consume Rust core via cbindgen C API.

---

### 🔲 M8 — Cloud Storage Monitoring *(Planned)*

> OneDrive, Google Drive, Dropbox, MEGA, iCloud integration via async Rust.

---

### 🔲 M9 — Database Export *(Planned)*

> MySQL, MariaDB, SQL Server, SQLite, PostgreSQL export via `sqlx` crate.

---

### 🔲 M10 — Public Release & Packaging *(Planned)*

> Code signing, auto-updater, native installers, first public release.

---

## 🏗️ Architecture Health

| Crate / Component | Path | Status |
|-------------------|------|--------|
| `mm-core` | `crates/mm-core/` | 🔲 Scaffold |
| `mm-cli` | `crates/mm-cli/` | 🔲 Scaffold |
| `mm-ffi` | `crates/mm-ffi/` | 🔲 Scaffold |
| `mm-metadata` | `crates/mm-metadata/` | 🔲 Scaffold |
| `mm-rules` | `crates/mm-rules/` | 🔲 Scaffold |
| `mm-cloud` | `crates/mm-cloud/` | 🔲 Scaffold |
| `mm-export` | `crates/mm-export/` | 🔲 Scaffold |
| `mm-service` | `crates/mm-service/` | 🔲 Scaffold |
| macOS SwiftUI app | `native/macos/` | 🔲 Scaffold |
| Windows WinUI 3 app | `native/windows/` | 🔲 Scaffold |
| Linux GTK4 app | `native/linux/` | 🔲 Planned |

---

## 📦 Platform Support Matrix

| Platform | Architecture | CI Tested | Native UI | Package Format |
|----------|-------------|-----------|-----------|----------------|
| 🍎 macOS | Apple Silicon (arm64) | ✅ | SwiftUI | .dmg / .tar.gz |
| 🪟 Windows | x64 | 🔲 Planned | WinUI 3 | .msi / .zip |
| 🪟 Windows | ARM64 | 🔲 Planned | WinUI 3 | .msi / .zip |
| 🐧 Linux | x86_64 | 🔲 Planned | GTK4 | .AppImage / .deb / .tar.gz |
| 🐧 Linux | ARM64 | 🔲 Planned | GTK4 | .AppImage / .deb / .tar.gz |

---

## 📅 Recent Activity

| Date | Activity |
|------|----------|
| 2026-03-04 | **M0 Started** — Rust rewrite begins: archived Python codebase, created Cargo workspace, scaffolded macOS SwiftUI + Windows WinUI 3, configured Rust toolchain, added 7 CI/CD workflows, created GitHub Projects v2 board, rewrote all documentation |
| 2026-03-04 | **v1.x Python era archived** — Tagged `v1.5-M6-python-final` (1007 tests, 6 milestones, 19 metadata providers) |

---

> 📝 *This file is updated with each significant change. For detailed changelog, see [docs/CHANGELOG.md](docs/CHANGELOG.md).*
>
> *Last updated: 2026-03-04*
