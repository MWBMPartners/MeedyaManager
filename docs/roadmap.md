# 📍 ROADMAP — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**
>
> 🎧📁 Cross-platform media manager and auto-organizer — Rust core + native UIs

---

## 🔄 v2.0 — The Rust Rewrite

MeedyaManager v2.0 is a complete rewrite from Python to **Rust** with **platform-native UIs** (SwiftUI on macOS, WinUI 3 on Windows, GTK4 on Linux). The Python v1.x codebase (milestones M1–M6, 1007 tests, 19 metadata providers) has been archived at tag **`v1.5-M6-python-final`**.

---

## 📈 Milestone Timeline

| # | Milestone | Status | Version | Description |
|---|-----------|--------|---------|-------------|
| M0 | 🔧 Repository Setup & Rust Migration | ✅ Complete | `v2.0.0-alpha.1` | Archive Python, Cargo workspace, native scaffolds, CI/CD |
| M1 | ⚙️ Core Engine | 🔲 Planned | `v2.0.0-alpha.2` | File watcher (`notify`), metadata extraction (`lofty`/`mediainfo`), classification, rename engine |
| M2 | 💻 CLI Frontend | 🔲 Planned | `v2.0.0-alpha.3` | `clap`-based CLI: scan, debug, watch, rule, edit |
| M3 | 📐 Rule Engine & Companion Files | 🔲 Planned | `v2.0.0-alpha.4` | MusicBee template parser (`nom`/`pest`), companion tracking |
| M4 | ✏️ Metadata Editor | 🔲 Planned | `v2.0.0-alpha.5` | Tag read/write via `lofty`, cover art, batch editing |
| M5 | 🔍 Metadata Lookup | 🔲 Planned | `v2.0.0-alpha.6` | Provider framework, async HTTP (`reqwest`/`tokio`), 19 providers |
| M6 | 🍎 Native GUI — macOS | 🔲 Planned | `v2.0.0-beta.1` | SwiftUI app via UniFFI, Liquid Glass on macOS 26+ |
| M7 | 🪟🐧 Native GUI — Windows & Linux | 🔲 Planned | `v2.0.0-beta.2` | WinUI 3 (C#) + GTK4 (Rust) via cbindgen C API |
| M8 | ☁️ Cloud Storage Monitoring | 🔲 Planned | `v2.0.0-beta.3` | OneDrive, Google Drive, Dropbox, MEGA, iCloud |
| M9 | 🗄️ Database Export | 🔲 Planned | `v2.0.0-rc.1` | MySQL, MariaDB, SQL Server, SQLite, PostgreSQL via `sqlx` |
| M10 | 📦 Public Release & Packaging | 🔲 Planned | `v2.0.0` | Code signing, auto-updater, native installers |

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
|----|-------------|-----------|
| 🍎 macOS | Apple Silicon (arm64) | SwiftUI |
| 🪟 Windows | x64, ARM64 | WinUI 3 (C# / WinAppSDK) |
| 🐧 Linux | x86_64, ARM64 | GTK4 (Rust `gtk4` crate) |

---

> 📝 *This roadmap is maintained alongside the codebase. For current status, see [PROJECT_STATUS.md](../PROJECT_STATUS.md).*
>
> *Last updated: 2026-03-04*
