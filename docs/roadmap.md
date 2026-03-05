# 📍 ROADMAP — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**
>
> 🎧📁 Cross-platform media manager and auto-organizer — Rust core + native UIs

---

## 🔄 v0.x — The Rust Rewrite (Pre-release)

MeedyaManager is a complete rewrite from Python to **Rust** with **platform-native UIs** (SwiftUI on macOS, WinUI 3 on Windows, GTK4 on Linux). The project uses **pre-release versioning** (`v0.x.y`) with `v1.0.0` reserved for the first public release at M10. The Python codebase (milestones M1–M6, 1007 tests) has been archived at tag **`v1.5-M6-python-final`**.

---

## 📈 Milestone Timeline

| # | Milestone | Status | Version | Issues | Description |
| --- | --------- | ------ | ------- | ------ | ----------- |
| M0 | 🔧 Repository Setup & Scaffolding | ✅ Complete | `v0.1.0` | #19-39 | Archive Python, Cargo workspace, native scaffolds, CI/CD |
| M1 | ⚙️ Core Engine | ✅ Complete | `v0.2.0` | #40-51 | Config, classify, metadata, watcher, renamer, companion, state, logging, health (217 tests) |
| M2 | 📐 Rule Engine | ✅ Complete | `v0.3.0` | — | Lexer, parser, evaluator, 24 template functions, 40+ tag mappings, rule system (182 tests) |
| M3 | ⌨️ CLI | ✅ Complete | `v0.4.0` | #52-62 | 8 commands: scan, debug, edit, rule, watch, lookup, config, report-bug (45 tests) |
| M4 | 🖥️ FFI Layer & Native UI Shells | ✅ Complete | `v0.5.0` | #63-72 | UniFFI (Swift), cbindgen (C#), async callbacks, SwiftUI/WinUI 3/GTK4 shells (20 tests) |
| M5 | 🔍 Metadata Lookup Providers | ✅ Complete | `v0.6.0` | #73-84 | 19 providers (music, video, podcasts, identifiers), credentials, rate limiting, fuzzy scoring (332 tests) |
| M6 | 🎨 Full Native UI | ✅ Complete | `v0.7.0` | #85-93 | Lookup Panel (all 3 platforms), full rule builder, cover art, DnD, real settings save, dark/light theme (GTK4), 90+ UI tests |
| M7 | ☁️ Cloud Storage Monitoring | ✅ Complete | `v0.8.0` | #94-102 | OneDrive, Google Drive, Dropbox, MEGA stub, iCloud stub — `mm-cloud` crate + Cloud UI tab on all platforms (~90 tests) |
| M8 | 📦 Packaging & Public Beta | ✅ Complete | `v0.9.0` | #103-111 | `mm-update` crate, Flatpak/Snap/AppImage/.deb, DMG script, WinGet manifest, update notification UI (~30 tests) |
| M9 | 🗄️ Database Export | ✅ Complete | `v0.10.0` | #112-119 | `mm-export` crate: `DatabaseExporter` trait + 5 backends (SQLite, MySQL, MariaDB, PostgreSQL, SQL Server), `SchemaBuilder` DDL, `meedya export` CLI command, Export tab UI on all 3 platforms (~90 tests) |
| M10 | 🌐 Secure Media Server + Public Release | 🔲 Planned | `v1.0.0` | #120-127 | axum HTTP server, JWT auth, media streaming, TLS, web frontend |

**Total: ~1076 tests passing** (399 mm-core + 45 mm-cli + 20 mm-ffi + 332 mm-providers + 90 mm-cloud + 33 mm-update + 42 mm-gtk + 64 macOS Swift + 70 Windows C# + 90 mm-export/UI — est.)

---

## 📋 Notes

- **v1.x Python era** is preserved at tag `v1.5-M6-python-final` for reference
- All builds produce platform-native binaries — no runtime dependencies for end users
- GitHub Actions CI covers Rust (3 OS), SwiftUI (macOS), WinUI 3 (Windows), GTK4 (Linux)
- API keys remain developer-only in `.env` (git-ignored); users can override with their own keys
- Documentation (.md files) updated with every milestone

---

## 💻 Platform Support

| OS | Architecture | Native UI |
| ---- | ------------- | --------- |
| 🍎 macOS | Apple Silicon (arm64) | SwiftUI |
| 🪟 Windows | x64, ARM64 | WinUI 3 (C# / WinAppSDK) |
| 🐧 Linux | x86_64, ARM64 | GTK4 (Rust `gtk4` crate) |

---

> 📝 *This roadmap is maintained alongside the codebase. For current status, see [PROJECT_STATUS.md](../PROJECT_STATUS.md).*
>
> *Last updated: 2026-03-05 (M9 complete — Database Export)*
