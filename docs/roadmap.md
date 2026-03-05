# 📍 ROADMAP — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**
>
> 🎧📁 Cross-platform media manager and auto-organizer — Rust core + native UIs

---

## 🔄 v2.0 — The Rust Rewrite

MeedyaManager v2.0 is a complete rewrite from Python to **Rust** with **platform-native UIs** (SwiftUI on macOS, WinUI 3 on Windows, GTK4 on Linux). The Python v1.x codebase (milestones M1–M6, 1007 tests, 19 metadata providers) has been archived at tag **`v1.5-M6-python-final`**.

---

## 📈 Milestone Timeline

| # | Milestone | Status | Version | Issues | Description |
| --- | --------- | ------ | ------- | ------ | ----------- |
| M0 | 🔧 Repository Setup & Scaffolding | ✅ Complete | `v2.0.0-alpha.1` | #19-39 | Archive Python, Cargo workspace, native scaffolds, CI/CD |
| M1 | ⚙️ Core Engine | ✅ Complete | `v2.0.0-alpha.2` | #40-51 | Config, classify, metadata, watcher, renamer, companion, state, logging, health (217 tests) |
| M2 | 📐 Rule Engine | ✅ Complete | `v2.0.0-alpha.3` | — | Lexer, parser, evaluator, 24 template functions, 40+ tag mappings, rule system (182 tests) |
| M3 | ⌨️ CLI | ✅ Complete | `v2.0.0-alpha.4` | #52-62 | 8 commands: scan, debug, edit, rule, watch, lookup, config, report-bug (45 tests) |
| M4 | 🖥️ FFI Layer & Native UI Shells | 🔲 Planned | `v2.0.0-alpha.5` | #63-72 | UniFFI (Swift), cbindgen (C#), async callbacks, SwiftUI/WinUI 3/GTK4 shells |
| M5 | 🔍 Metadata Lookup Providers | 🔲 Planned | `v2.0.0-alpha.6` | #73-84 | 19 providers (music, video, podcasts, identifiers), framework, credentials, rate limiting |
| M6 | 🎨 Full Native UI | 🔲 Planned | `v2.0.0-beta.1` | #85-93 | Rule Builder, Metadata Editor, Lookup Panel, accessibility, themes |
| M7 | ☁️ Cloud Storage Monitoring | 🔲 Planned | `v2.0.0-beta.2` | #94-102 | OneDrive, Google Drive, Dropbox, MEGA, iCloud |
| M8 | 📦 Packaging & Public Release | 🔲 Planned | `v2.0.0-beta.3` | #103-111 | App Store, Microsoft Store, Flatpak/Snap, auto-updater, first public beta |
| M9 | 🗄️ Database Export | 🔲 Planned | `v2.0.0-rc.1` | #112-119 | MySQL, MariaDB, SQL Server, SQLite, PostgreSQL via sqlx/tiberius |
| M10 | 🌐 Secure Media Server | 🔲 Planned | `v2.0.0` | #120-127 | axum HTTP server, JWT auth, media streaming, TLS, web frontend |

**Total: 444 tests passing** (399 mm-core + 45 mm-cli)

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
> *Last updated: 2026-03-05 (M3 complete, all milestone issues created)*
