# MeedyaManager — Claude Code Project Instructions

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

## Project Identity

- **Name:** MeedyaManager
- **Type:** Cross-platform media file manager and auto-organizer
- **Languages:** Rust (core engine, CLI, metadata, rules, cloud, export, service) + Swift (macOS UI) + C# (Windows UI) + Rust/GTK4 (Linux UI)
- **Licence:** GPL-2.0-or-later
- **Copyright:** MWBM Partners Ltd (d/b/a MW Services)
- **Platforms:** Windows (x64/ARM), macOS (Apple Silicon only), Linux (x64/ARM)

## Key Architecture Decisions

- **Rust core:** Shared library consumed by all platform UIs via FFI
- **FFI layer:** UniFFI (generates Swift bindings for macOS), cbindgen (generates C headers for Windows C# P/Invoke and Linux GTK4)
- **macOS GUI:** SwiftUI with Liquid Glass on macOS 26+ (falls back to standard vibrancy on older versions)
- **Windows GUI:** WinUI 3 (C# / WinAppSDK) with Mica/Acrylic backdrop
- **Linux GUI:** GTK4 via Rust `gtk4` crate (libadwaita for adaptive styling)
- **Cargo workspace:** 8 crates — `mm-core`, `mm-cli`, `mm-ffi`, `mm-metadata`, `mm-rules`, `mm-cloud`, `mm-export`, `mm-service`
- **Key Rust crates:**
  - `lofty` — Audio tag reading/writing (replaces Python `mutagen`)
  - `notify` — Cross-platform file system watcher (replaces Python `watchdog`)
  - `clap` — CLI argument parsing (replaces Python `click`)
  - `serde` / `serde_json` — Serialization/deserialization
  - `tokio` — Async runtime for cloud and network operations
  - `reqwest` — HTTP client for metadata provider APIs
  - `sqlx` — Async database driver (MySQL, PostgreSQL, SQLite)
  - `nom` or `pest` — Parser combinators for MusicBee template syntax
  - `tracing` — Structured logging (replaces Python `logging`)
  - `anyhow` / `thiserror` — Error handling
- **Config format:** JSON5 (`settings.json5`) with `.env` fallback for secrets

## Coding Standards

- **ALL code** must include detailed comments/annotations (every line where possible)
- **Copyright header** in every source file:

  ```rust
  // (C) 2025-{current_year} MWBM Partners Ltd (d/b/a MW Services)
  ```

  ```swift
  // (C) 2025-{current_year} MWBM Partners Ltd (d/b/a MW Services)
  ```

  ```csharp
  // (C) 2025-{current_year} MWBM Partners Ltd (d/b/a MW Services)
  ```

- **Copyright year** must be automated (start 2025, end current year)
- Code formatting: `rustfmt` for Rust, SwiftFormat for Swift, dotnet-format for C#
- Emojis are welcome in documentation and UI
- Use the canonical name **MeedyaManager** (not MetaMancer) everywhere

## Documentation Requirements

- **Project_Plan.md** — Full project plan (root)
- **PROJECT_STATUS.md** — Current status tracker (root)
- **README.md** — Project overview with quick start (root)
- **docs/CHANGELOG.md** — Detailed change log with dates
- **docs/ROADMAP.md** — Milestone timeline
- **help/** — User documentation (getting-started, config, rules, formats, troubleshooting, FAQ)
- **.claude/** — This file + project brief for session continuity
- **All .md files** must be updated after every code change

## Milestone Order

1. ✅ M0 — Repository Setup & Rust Migration (Complete)
2. 🔲 M1 — Core Engine (file watcher, metadata extraction, classification, rename engine)
3. 🔲 M2 — CLI Frontend (`clap`-based CLI)
4. 🔲 M3 — Rule Engine & Companion Files (template parser in Rust)
5. 🔲 M4 — Metadata Editor (tag read/write via `lofty`)
6. 🔲 M5 — Metadata Lookup (async provider framework)
7. 🔲 M6 — Native GUI — macOS (SwiftUI via UniFFI)
8. 🔲 M7 — Native GUI — Windows & Linux (WinUI 3 + GTK4 via cbindgen)
9. 🔲 M8 — Cloud Storage Monitoring
10. 🔲 M9 — Database Export (`sqlx`)
11. 🔲 M10 — Public Release & Packaging

## API Key Policy

- Developer-only keys in `.env` (git-ignored)
- Per-service `include_in_build` toggle for bundling decisions
- Users can always override with their own keys
- Never bundle keys where ToS prohibits shared use

## Important Context Files

- `.claude/ProjectBrief_Chat.claude` — Full project brief from user
- `Project_Plan.md` — Comprehensive project plan
- `PROJECT_STATUS.md` — Current progress
- `docs/ROADMAP.md` — Milestone details

## Packaging & Distribution

- **Cargo** builds native Rust binaries (no runtime dependency)
- **Xcode** builds macOS SwiftUI app bundle (.app → .dmg)
- **MSBuild** builds Windows WinUI 3 app (.msix / .msi)
- **Cargo** builds Linux GTK4 binary (packaged as .AppImage / .deb / .tar.gz)
- App is fully self-contained — users need ZERO pre-installed software
- All packages include SHA256 checksums

## Git & CI/CD

- **7 GitHub Actions workflows:**
  - `rust-ci.yml` — Cargo build + test + clippy (Ubuntu, macOS, Windows)
  - `swiftui-ci.yml` — Xcode build (macOS only)
  - `winui-ci.yml` — MSBuild (Windows only)
  - `gtk-ci.yml` — GTK4 build (Ubuntu)
  - `release.yml` — Cross-platform release packaging on git tags (`v*`)
  - `lint.yml` — `rustfmt` + `clippy` checks
  - `docs.yml` — `cargo doc` generation
- Platform packages: Windows (MSI/MSIX/ZIP), macOS (DMG/TAR.GZ), Linux (AppImage/DEB/TAR.GZ)
- `.gitignore` covers OS files, Rust `target/`, IDE files, secrets, build artifacts
- Python v1.x archived at tag `v1.5-M6-python-final`
