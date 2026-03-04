# MeedyaManager — Claude Code Project Instructions

> **(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)**

## Project Identity

- **Name:** MeedyaManager
- **Type:** Cross-platform media file manager and auto-organizer
- **Languages:** Rust (core engine + CLI + Linux GTK4 UI) + Swift (macOS UI) + C# (Windows UI)
- **Licence:** GPL-2.0-or-later
- **Copyright:** MWBM Partners Ltd (d/b/a MW Services)
- **Platforms:** Windows (x64/ARM), macOS (Apple Silicon only), Linux (x64/ARM)

## Key Architecture Decisions

- **Rust core:** Shared library (`mm-core`) consumed by all platform UIs via FFI
- **FFI layer:** UniFFI (generates Swift bindings for macOS), cbindgen/csbindgen (generates C headers for Windows C# P/Invoke)
- **macOS GUI:** SwiftUI with Liquid Glass on macOS 26+ (falls back to standard vibrancy on older versions)
- **Windows GUI:** WinUI 3 (C# / WinAppSDK 1.6) with Mica backdrop
- **Linux GUI:** GTK4 via `gtk4-rs` + `libadwaita` (direct Rust, no FFI needed)
- **Cargo workspace:** 8 crates — `mm-core`, `mm-providers`, `mm-cloud`, `mm-export`, `mm-server`, `mm-cli`, `mm-ffi`, `mm-gtk`
- **Key Rust crates:**
  - `lofty` — Audio/video metadata read/write
  - `notify` — Cross-platform file system watcher
  - `clap` — CLI argument parsing
  - `serde` / `json5` — Config serialization (JSON5 format)
  - `tokio` — Async runtime
  - `reqwest` — HTTP client for metadata provider APIs
  - `sqlx` — Async database driver (MySQL, PostgreSQL, SQLite)
  - `tiberius` — SQL Server driver (default-features = false for rustls compat)
  - `tracing` — Structured logging
  - `thiserror` / `anyhow` — Error handling
  - `uniffi` — FFI binding generation for Swift
  - `governor` — Rate limiting for API providers
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
- **docs/Dev_Notes.md** — Developer notes (versioning, release process, CI/CD)
- **help/** — User documentation
- **.claude/** — This file + project brief for session continuity
- **GitHub Wiki** — Version Management, Release Process, CI/CD Pipelines
- **All .md files** must be updated after every code change

## Milestone Order

1. M0 — Repository Setup & Scaffolding (Complete)
2. M1 — Core Engine (config, classify, metadata, watcher, renamer, companion, state, logging, health)
3. M2 — Rule Engine (lexer, parser, evaluator, 20+ template functions)
4. M3 — CLI (clap-based: scan, debug, watch, rule, edit, lookup, config, report-bug)
5. M4 — FFI Layer & Native UI Shells (UniFFI, cbindgen, SwiftUI/WinUI/GTK shells)
6. M5 — Metadata Lookup Providers (19 providers: music, video, podcasts, identifiers)
7. M6 — Full Native UI (Rule Builder, Metadata Editor, Lookup Panel, accessibility)
8. M7 — Cloud Storage Monitoring (OneDrive, Google Drive, Dropbox, MEGA, iCloud)
9. M8 — Packaging & Public Release (App Store, Microsoft Store, Flatpak/Snap)
10. M9 — Database Export (MySQL, MariaDB, SQL Server, SQLite, PostgreSQL)
11. M10 — Secure Media Server (axum, REST API, JWT auth, media streaming)

## Version Management

- **Single source of truth:** `Cargo.toml` `[workspace.package].version`
- **Automated bumping:** `version-bump.yml` GitHub Actions workflow
- **CI sync check:** `ci-rust.yml` verifies all platform files match
- **Platform mapping:** semver → MSIX 4-part (2.0.0.0), CFBundle 3-part (2.0.0)
- See `docs/Dev_Notes.md` for full details

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
- `docs/Dev_Notes.md` — Developer notes and release process

## Packaging & Distribution

- **Cargo** builds native Rust binaries (no runtime dependency)
- **Swift Package Manager** builds macOS SwiftUI app
- **MSBuild/.NET 8** builds Windows WinUI 3 app (MSIX package)
- **Cargo** builds Linux GTK4 binary (Flatpak/Snap/AppImage/.deb)
- App is fully self-contained — users need ZERO pre-installed software
- All packages include SHA256 checksums
- Release workflow generates draft GitHub Releases with artifacts

## Git & CI/CD

- **8 GitHub Actions workflows:**
  - `ci-rust.yml` — Cargo fmt + clippy + test + version-sync (3-OS matrix)
  - `ci-macos.yml` — Build mm-ffi + SwiftUI app (macos-14)
  - `ci-windows.yml` — Build mm-ffi + WinUI 3 app (windows-latest)
  - `ci-linux.yml` — Build mm-gtk with GTK4/Libadwaita (ubuntu-latest)
  - `version-bump.yml` — Automated version bumping across all files (manual trigger)
  - `release.yml` — 5-platform release builds + checksums + GitHub Release (tag trigger)
  - `audit.yml` — cargo-deny + cargo-audit (weekly + push)
  - `docs.yml` — cargo doc generation
- Platform packages: Windows (MSIX), macOS (.dmg/.tar.gz), Linux (Flatpak/Snap/AppImage/.deb)
- `.gitignore` covers OS files, Rust `target/`, IDE files, secrets, build artifacts
- Python v1.x archived at tag `v1.5-M6-python-final`
- **Every task** must have a GitHub Issue created BEFORE work begins and closed AFTER verification
- **Commit but do NOT push** — user pushes manually
