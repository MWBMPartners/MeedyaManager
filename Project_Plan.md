# 📋 MeedyaManager — Project Plan

> **(C) 2025–2026 MWBM Partners Ltd**
>
> 🎧📁 A smart, cross-platform media manager and auto-organizer — Rust core + native UIs, inspired by MusicBee's flexibility.

---

## 📖 Table of Contents

1. [Project Overview](#-project-overview)
2. [Architecture](#-architecture)
3. [Technology Stack](#-technology-stack)
4. [Monorepo Structure](#-monorepo-structure)
5. [Platform Support](#-platform-support)
6. [Milestone Roadmap (M0–M10)](#-milestone-roadmap-m0m10)
7. [Testing Strategy](#-testing-strategy)
8. [CI/CD Pipelines](#-cicd-pipelines)
9. [API Key Management](#-api-key-management)
10. [Packaging & Distribution](#-packaging--distribution)
11. [Licensing & Copyright](#-licensing--copyright)

---

## 🎯 Project Overview

**MeedyaManager** is a cross-platform media file management application that automatically monitors folders, reads metadata from audio/video files, and renames/organizes them according to user-defined rules — similar to MusicBee's auto-organize feature, but available on **Windows, macOS, and Linux**.

The project has been fully rewritten from the original Python/PySide6 implementation to a **Rust core + native UI** architecture. This follows the pattern used by 1Password, Dropbox, and Firefox: a shared Rust library handles all business logic while each platform gets a truly native user interface.

### 🌟 Core Goals

| Goal | Description |
|------|-------------|
| 🖥️ **Cross-Platform** | Windows (x64/ARM), macOS (Apple Silicon), Linux (x64/ARM) |
| 🦀 **Rust Core** | All business logic in a single shared Rust library |
| 🎨 **Native UIs** | SwiftUI on macOS, WinUI 3 on Windows, GTK4 on Linux |
| 👁️ **Continuous Monitoring** | Real-time folder watching with file-lock awareness |
| 🧠 **Smart Classification** | 4-level media hierarchy (group → format → class → quality) |
| 📐 **Flexible Rules** | MusicBee-inspired template engine with `$If`, `$And`, `$Or`, nesting |
| 🎵 **Format Support** | MP3, FLAC, ALAC, M4A, MP4, MKV, AVI, OGG, AC3, EAC3, HEVC + more |
| 🔍 **Metadata Lookup** | 19+ providers across music, video, podcasts, and identifiers |
| ☁️ **Cloud Sync** | OneDrive, Google Drive, Dropbox, MEGA, iCloud |
| 🗄️ **Database Export** | MySQL, MariaDB, SQL Server, SQLite, PostgreSQL |
| 🌐 **Media Server** | Secure web-accessible media library with REST API |

---

## 🏗️ Architecture

### High-Level Diagram

```text
┌─────────────────────────────────────────────────────────┐
│                    Native UI Layer                       │
│  ┌──────────┐   ┌──────────────┐   ┌────────────────┐  │
│  │  macOS    │   │   Windows    │   │     Linux      │  │
│  │ SwiftUI   │   │   WinUI 3   │   │ GTK4 (gtk4-rs) │  │
│  │ (Swift 6) │   │   (C# .NET) │   │   (Rust)       │  │
│  └─────┬─────┘   └──────┬──────┘   └───────┬────────┘  │
│        │                 │                  │            │
│   UniFFI            cbindgen/          Direct Rust       │
│   (auto-gen         P/Invoke           (no FFI)          │
│    Swift)            (C#)                                │
├────────┴─────────────────┴──────────────────┴────────────┤
│                   Rust Core (mm-core)                    │
│  ┌────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐ │
│  │Watcher │ │Rule Eng. │ │Metadata  │ │ Classifier   │ │
│  │(notify)│ │(lexer/   │ │(lofty)   │ │ (4-level)    │ │
│  │        │ │parser/   │ │          │ │              │ │
│  │        │ │evaluator)│ │          │ │              │ │
│  └────────┘ └──────────┘ └──────────┘ └──────────────┘ │
│  ┌────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐ │
│  │Renamer │ │Companion │ │Providers │ │ Config       │ │
│  │        │ │Tracker   │ │(19+ APIs)│ │ (JSON5+.env) │ │
│  └────────┘ └──────────┘ └──────────┘ └──────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### FFI Strategy

| Platform | FFI Mechanism | How It Works |
|----------|---------------|--------------|
| 🍎 **macOS** | UniFFI (Mozilla) | Auto-generates Swift bindings from Rust UDL/proc-macro definitions |
| 🪟 **Windows** | cbindgen / csbindgen | Generates C headers from Rust → C# P/Invoke calls the Rust `.dll` |
| 🐧 **Linux** | Direct Rust | GTK4 UI written in Rust via `gtk4-rs` + `libadwaita` — no FFI layer needed |

### Key Architectural Principles

1. **🦀 Single Source of Truth** — All business logic lives in `mm-core` (Rust). UI layers are thin wrappers.
2. **🧩 Modular Crates** — Each major feature area is its own Rust crate with clear interfaces.
3. **🔌 Plugin-Style Providers** — Metadata lookup providers follow a common trait and register via `inventory`.
4. **⚙️ Config-Driven** — All behaviour is configurable via JSON5 + environment overrides.
5. **🛡️ Safety First** — File-lock detection prevents corruption; dry-run mode by default.
6. **📊 Observable** — Structured logging via `tracing` with PII redaction for safe troubleshooting.
7. **🔄 Async Callbacks** — Watcher events, scan progress, and lookup results flow from Rust to native UIs via async callbacks through the FFI layer.

---

## 🛠️ Technology Stack

### Rust Crates

| Purpose | Crate | Replaces (Python) |
|---------|-------|--------------------|
| File watching | `notify` | `watchdog` |
| Metadata read/write | `lofty` | `mutagen` + `pymediainfo` |
| CLI framework | `clap` | `click` |
| HTTP client | `reqwest` | `requests` / `httpx` |
| Async runtime | `tokio` | N/A |
| Config (JSON5) | `json5` + `serde` | `pyjson5` |
| Environment vars | `dotenvy` | `python-dotenv` |
| Logging | `tracing` + `tracing-subscriber` | `logging` |
| FFI (Swift) | `uniffi` | N/A |
| FFI (C header) | `cbindgen` | N/A |
| GTK4 UI | `gtk4-rs` + `libadwaita` | `PySide6` |
| Rate limiting | `governor` | custom |
| Fuzzy matching | `fuzzy-matcher` | `fuzzywuzzy` |
| Credential storage | `keyring` | `keyring` |
| Error types | `thiserror` | custom |
| Regex | `regex` | `re` |
| OAuth2 | `oauth2` | `spotipy`, etc. |
| JWT | `jsonwebtoken` | `PyJWT` |
| Database (M9) | `sqlx` + `tiberius` | `SQLAlchemy` |
| HTTP server (M10) | `axum` | N/A |
| TLS (M10) | `rustls` | N/A |
| Property testing | `proptest` | N/A |
| Mock HTTP | `wiremock` | N/A |
| Provider registry | `inventory` | custom decorator |

### Native UI Frameworks

| Platform | Language | Framework | Version |
|----------|----------|-----------|---------|
| 🍎 macOS | Swift 6 | SwiftUI | Xcode 16+ |
| 🪟 Windows | C# | WinUI 3 / .NET 8+ | Visual Studio 2022+ |
| 🐧 Linux | Rust | GTK4 + Libadwaita | gtk4-rs (latest) |

---

## 📂 Monorepo Structure

```text
MeedyaManager/
├── Cargo.toml                    # Workspace root
├── rust-toolchain.toml           # Pin Rust version
├── .rustfmt.toml / clippy.toml / deny.toml
│
├── crates/
│   ├── mm-core/                  # Core business logic
│   │   └── src/ (config/, watcher/, classify/, rule_engine/,
│   │            renamer/, companion/, metadata/, state/,
│   │            logging/, health/, error.rs)
│   ├── mm-providers/             # 19+ metadata lookup providers
│   │   └── src/ (traits.rs, registry.rs, credentials.rs,
│   │            rate_limiter.rs, match_scoring.rs, cover_art.rs,
│   │            music/, video/, podcasts/, identifiers/)
│   ├── mm-cloud/                 # Cloud storage (M7)
│   ├── mm-export/                # Database export (M9)
│   ├── mm-server/                # Media server (M10)
│   ├── mm-cli/                   # Cross-platform CLI (clap)
│   ├── mm-ffi/                   # FFI bindings (UniFFI + cbindgen)
│   └── mm-gtk/                   # Linux GTK4/Libadwaita UI
│
├── macos/                        # Swift/SwiftUI app
│   ├── MeedyaManager.xcodeproj/
│   └── MeedyaManager/ (Views/, Models/, Bindings/, Resources/)
│
├── windows/                      # WinUI 3 / C# app
│   ├── MeedyaManager.sln
│   └── MeedyaManager/ (Views/, ViewModels/, Interop/, Assets/)
│
├── config/settings.json5         # Shared default config
├── assets/                       # Shared icons/branding
├── branding/                     # Logos
├── docs/                         # Developer docs (CHANGELOG, ROADMAP)
├── help/                         # User documentation
├── .github/workflows/            # CI/CD (7 workflows)
├── .claude/                      # Project context
├── Project_Plan.md               # This file
├── PROJECT_STATUS.md             # Current progress
├── README.md                     # Project overview
├── LICENSE                       # GPL-2.0-or-later
└── justfile                      # Task runner
```

---

## 💻 Platform Support

| Platform | Architectures | UI Framework | FFI | Store Target |
|----------|---------------|--------------|-----|--------------|
| 🍎 **macOS** | Apple Silicon (arm64) | SwiftUI (Swift 6) | UniFFI | App Store |
| 🪟 **Windows** | x64, ARM64 | WinUI 3 (C# .NET 8+) | cbindgen / P/Invoke | Microsoft Store |
| 🐧 **Linux** | x86_64, ARM64 | GTK4 + Libadwaita (gtk4-rs) | Direct Rust | Flatpak / Snap |

### Native Platform Features

| Feature | macOS | Windows | Linux |
|---------|-------|---------|-------|
| Visual style | Liquid Glass / Vibrancy | Mica / Acrylic | Adwaita |
| Dark/light theme | System-following | System-following | System-following |
| System tray | Menu bar extra | System tray icon | Status indicator |
| Accessibility | VoiceOver | Narrator | Orca |
| Drag-and-drop | NSPasteboard | WinUI DnD | GDK DnD |
| Background service | launchd | Windows Service | systemd |

---

## 🗺️ Milestone Roadmap (M0–M10)

### 🔧 M0 — Repository Setup & Scaffolding (In Progress)

> Archive the Python codebase, initialize the Rust workspace, scaffold native app projects, set up CI.

| Deliverable | Description |
|-------------|-------------|
| Archive Python code | Tag `v1.5-M6-python-final`, create `archive/python-v1.5` branch |
| Clean main branch | Delete all Python files, retain docs/assets/config/branding |
| Cargo workspace | Initialize with stub crates: mm-core, mm-providers, mm-cloud, mm-export, mm-server, mm-cli, mm-ffi, mm-gtk |
| macOS project | Scaffold Xcode project with empty SwiftUI app |
| Windows project | Scaffold Visual Studio solution with empty WinUI 3 app |
| Rust toolchain config | `.rustfmt.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml` |
| CI workflows | Create 7 GitHub Actions workflow stubs |
| GitHub Projects v2 | Board with custom fields, views, labels; close 18 stale issues + 9 old milestones |
| Documentation | Update README.md, Project_Plan.md, PROJECT_STATUS.md, CLAUDE.md, ROADMAP.md |

---

### 🧱 M1 — Core Engine (Rust)

> All foundational business logic in the `mm-core` crate.

| Deliverable | Description |
|-------------|-------------|
| Configuration | JSON5 + `.env` loading via `serde` + `dotenvy` |
| Media classification | 4-level hierarchy: Group / Format / Class / Quality |
| Metadata extraction | Audio + video metadata via `lofty` crate |
| Metadata writing | Tag writing (ID3v2, MP4, Vorbis, FLAC) via `lofty` |
| Multi-value fields | Semicolon-delimited parsing for artists, genres, composers |
| File watcher | `notify` crate with polling fallback |
| Rename simulator | Dry-run path computation |
| Filename sanitizer | Configurable character replacement mappings |
| Companion tracker | SRT, LRC, CUE, ISO, cover art detection and grouping |
| State manager | Application state + single-instance lock file |
| Structured logging | `tracing` with PII redaction filter + daily rotation |
| Health checks | Startup health checks + unified error types (`thiserror`) |
| **Test target** | 200+ unit tests |

---

### 📐 M2 — Rule Engine

> MusicBee-inspired template engine in `mm-core::rule_engine`.

| Deliverable | Description |
|-------------|-------------|
| Lexer | Tokenizer for `<Tag>`, `$Func()`, literals |
| Parser | Recursive descent parser with AST generation, 50-level depth guard |
| Evaluator | AST evaluation against metadata `HashMap` |
| Template functions | 20+ functions: `$If`, `$And`, `$Or`, `$IsNull`, `$Contains`, `$Replace`, `$Pad`, `$Date`, etc. |
| Tag registry | 40+ bidirectional tag mappings + custom tag support |
| Legacy compat | `{placeholder}` backward compatibility detection |
| **Test target** | 150+ tests |

---

### ⌨️ M3 — CLI

> Cross-platform command-line interface via the `mm-cli` crate.

| Deliverable | Description |
|-------------|-------------|
| CLI framework | `clap`-based main binary with subcommand routing |
| `scan` command | Directory scan, rename preview, JSON/formatted output |
| `debug` command | Single-file metadata inspector |
| `watch` command | Foreground file watcher with event logging |
| `rule` command | Template validation, tag listing, sample testing |
| `edit` command | `--set`, `--remove`, `--cover`, `--remove-cover`, `--dry-run`, `--json` |
| `lookup` command | Provider search, `--auto`, `--apply`, `--batch` |
| `config` command | Export/import `.mmprofile` bundles |
| `report-bug` command | System info + log collection |
| Rich output | Colored terminal output, tables, progress bars |
| **Test target** | 40+ integration tests |

---

### 🖥️ M4 — FFI Layer & Native UI Shells

> Bridge between Rust core and native UIs on all three platforms.

| Deliverable | Description |
|-------------|-------------|
| UniFFI interface | Swift binding definitions from Rust |
| cbindgen headers | C header generation for C# P/Invoke |
| Async callbacks | Watcher events, scan progress, lookup results flowing to UIs |
| macOS shell | SwiftUI app: tab navigation, UniFFI integration, menu bar, Liquid Glass |
| macOS panels | Basic PreviewPanel + SettingsView connected to Rust core |
| Windows shell | WinUI 3 app: NavigationView, P/Invoke integration, system tray, Mica |
| Windows panels | Basic PreviewPanel + SettingsPage connected to Rust core |
| Linux shell | GTK4/Libadwaita app: AdwTabView, Adwaita theming |
| Linux panels | Basic preview panel + settings dialog |
| **Test target** | 10+ UI tests per platform |

---

### 🔍 M5 — Metadata Lookup Providers

> 19 metadata providers in the `mm-providers` crate.

| Deliverable | Description |
|-------------|-------------|
| Provider trait | `BaseProvider` trait + `ProviderResult` / `Capabilities` types |
| Auto-registration | Provider discovery via `inventory` crate |
| Credentials | 4-tier resolution: env → config → keyring → encrypted |
| Rate limiting | Token bucket per provider via `governor` crate |
| Fuzzy matching | Weighted scoring: title (35%), artist (30%), album (20%), ISRC bonus |
| Cover art | Static (JPEG/PNG) + animated (MP4 square, portrait, artist spotlight) |
| 🎵 Music (10) | Apple Music (JWT), Spotify (OAuth2), MusicBrainz, Deezer, YouTube Music, Amazon Music, Pandora, Tidal (OAuth2.1), Shazam (fingerprinting), iHeart |
| 🎬 Video (5) | TMDB, TheTVDB, IMDb, Apple TV, iTunes Store |
| 🎙️ Podcasts (1) | Apple Podcasts |
| 🆔 Identifiers (3) | ISRC (federated), EIDR (paid), ISWC (MusicBrainz) |
| **Test target** | 300+ tests (mock HTTP via `wiremock`) |

---

### 🎨 M6 — Full Native UI

> Complete all views on all three platforms.

| Deliverable | Description |
|-------------|-------------|
| Rule Builder | Syntax highlighting, tag palette, live preview (all platforms) |
| Metadata Editor | Tag table, cover art widget, batch editing (all platforms) |
| Lookup Panel | Provider checkboxes, results table, apply/batch (all platforms) |
| Preview Panel | Full rename preview with progress (all platforms) |
| Drag-and-drop | File import on all platforms |
| Accessibility | VoiceOver (macOS), Narrator (Windows), Orca (Linux) |
| Themes | Dark/light toggle, system-following default |
| Error dialogs | User-friendly error messages + config export/import UI |
| **Test target** | 30+ UI tests per platform |

---

### ☁️ M7 — Cloud Storage Monitoring

> Cloud sync in the `mm-cloud` crate.

| Deliverable | Description |
|-------------|-------------|
| CloudProvider trait | Sync manager architecture |
| OneDrive | Personal + Business via Microsoft Graph |
| Google Drive | Drive API v3 |
| Dropbox | API v2 |
| MEGA.nz | MEGA API |
| iCloud Drive | macOS only via FileProvider framework |
| UI | Cloud tab on all platforms (connection status, folder browser, sync status) |
| Background sync | Conflict resolution, OAuth2 token refresh |
| **Test target** | 80+ tests |

---

### 📦 M8 — Packaging & Public Release

> Native packaging for all platforms and store submissions.

| Deliverable | Description |
|-------------|-------------|
| macOS | `.app` bundle in `.dmg`, code-signed + notarized |
| macOS App Store | Submission via `xcrun altool` |
| Windows | MSIX package, code-signed |
| Microsoft Store | Store submission |
| Linux | Flatpak manifest + Snap `snapcraft.yaml` + AppImage + `.deb` |
| Auto-updater | Platform-specific auto-update integration |
| Release pipeline | SHA256 checksums + release notes auto-generation |
| First public beta | 🎉 |

---

### 🗄️ M9 — Database Export

> Library export in the `mm-export` crate.

| Deliverable | Description |
|-------------|-------------|
| DbExporter trait | Shared table schema |
| MySQL | Via `sqlx` |
| MariaDB | Via `sqlx` |
| SQL Server | Via `tiberius` |
| SQLite | Via `sqlx` |
| PostgreSQL | Via `sqlx` |
| UI + CLI | Export tab on all platforms + `export` CLI command |
| **Test target** | 60+ tests |

---

### 🌐 M10 — Secure Media Server

> Web-accessible media library in the `mm-server` crate.

| Deliverable | Description |
|-------------|-------------|
| HTTP server | `axum` with REST API scaffold |
| Authentication | JWT + bcrypt password hashing |
| Media streaming | Range request support |
| Access control | Per-user library visibility |
| Web frontend | Embedded static files (HTMX or lightweight JS) |
| TLS | `rustls` + CLI `serve` command |
| **Test target** | 50+ tests |

---

## 🧪 Testing Strategy

| Layer | Tooling | Approach |
|-------|---------|----------|
| **Rust core** | `#[test]` + `#[tokio::test]` | Unit tests for every module, property-based testing via `proptest` |
| **HTTP mocking** | `wiremock` | Mock all 19 provider APIs for deterministic testing |
| **Coverage** | `cargo-tarpaulin` / `cargo-llvm-cov` | Target: 80%+ coverage on `mm-core` and `mm-providers` |
| **macOS** | XCTest + XCUITest | Unit tests, snapshot tests, accessibility audit |
| **Windows** | MSTest + WinAppDriver | Unit tests, UI Automation validation |
| **Linux** | Rust `#[test]` with GTK harness | Tests run under `xvfb-run` in CI |
| **E2E** | Shared test scenarios | 10 media files → expected rename paths, verified on all platforms |
| **Linting** | `clippy`, `cargo-deny` | Zero warnings policy, dependency license + advisory auditing |

### Test Targets by Milestone

| Milestone | Test Count Target |
|-----------|-------------------|
| M1 — Core Engine | 200+ |
| M2 — Rule Engine | 150+ |
| M3 — CLI | 40+ |
| M4 — UI Shells | 10+ per platform |
| M5 — Providers | 300+ |
| M6 — Full UI | 30+ per platform |
| M7 — Cloud | 80+ |
| M9 — Export | 60+ |
| M10 — Server | 50+ |

---

## 🔄 CI/CD Pipelines

### 7 GitHub Actions Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci-rust.yml` | Push/PR to main | Rust core: `fmt` + `clippy` + `test` + coverage (3 OS matrix) |
| `ci-macos.yml` | Push/PR to main | Build Rust FFI → UniFFI bindings → `xcodebuild` + test |
| `ci-windows.yml` | Push/PR to main | Build Rust FFI → cbindgen → `dotnet build` + test |
| `ci-linux.yml` | Push/PR to main | `cargo build` mm-gtk + test (`xvfb-run`) |
| `release.yml` | Git tag (`v*`) | Build all platforms, code-sign, notarize, create GitHub Release |
| `audit.yml` | Weekly (cron) | `cargo-deny` (licenses, advisories) + `cargo-audit` |
| `docs.yml` | Push to main | Build `cargo doc`, optionally publish to GitHub Pages |

### CI Matrix (ci-rust.yml)

| OS | Rust | Targets |
|----|------|---------|
| ubuntu-latest | stable | `cargo fmt --check`, `cargo clippy --workspace --exclude mm-gtk -- -D warnings`, `cargo test --workspace --exclude mm-gtk` |
| macos-latest (ARM) | stable | Same as above (mm-gtk excluded — GTK4 not available on macOS runners) |
| windows-latest | stable | Same as above (mm-gtk excluded — GTK4 not available on Windows runners) |

### Verification Checklist (M0 Completion)

- `cargo build --workspace` succeeds
- `cargo test --workspace` passes (stub tests)
- `cargo clippy --workspace -- -D warnings` is clean
- macOS: `xcodebuild build` succeeds
- Windows: `dotnet build` succeeds
- All 7 CI workflows pass (green baseline)

---

## 🔑 API Key Management

### Security Strategy

| Scenario | Key Storage | Distribution |
|----------|-------------|--------------|
| **Developer-only keys** | `.env` file (git-ignored) | Not included in packages |
| **Universal keys** (ToS allows shared use) | Encrypted in app config | Bundled with app |
| **User-provided keys** | User's local config / UI settings | User manages |

### 4-Tier Credential Resolution

| Priority | Source | Description |
|----------|--------|-------------|
| 1 (highest) | Environment variables | `.env` file via `dotenvy` |
| 2 | Config file | `settings.json5` `api_keys` section |
| 3 | OS keyring | Native credential storage via `keyring` crate |
| 4 | Encrypted bundle | AES-256-GCM encrypted credential file |

### Per-Service Configuration

Each API provider has a toggle in the build configuration:

```json5
{
  api_keys: {
    musicbrainz: { include_in_build: true,  key: "..." },
    spotify:     { include_in_build: false, key: "..." },  // User must provide
    tmdb:        { include_in_build: true,  key: "..." },
  }
}
```

- **`include_in_build: true`** — Key is safe to bundle (ToS compliant)
- **`include_in_build: false`** — Developer-only; users must provide their own
- Users can always override with their own keys in settings

---

## 📦 Packaging & Distribution

### Per-Platform Packaging

| Platform | Package Format | Signing | Store |
|----------|---------------|---------|-------|
| 🍎 **macOS** | `.app` in `.dmg` | Apple Developer ID + notarization | App Store (via `xcrun altool`) |
| 🪟 **Windows** (x64) | MSIX | Code-signed (Authenticode) | Microsoft Store |
| 🪟 **Windows** (ARM64) | MSIX | Code-signed (Authenticode) | Microsoft Store |
| 🐧 **Linux** (x86_64) | Flatpak, Snap, AppImage, `.deb` | — | Flathub, Snap Store |
| 🐧 **Linux** (ARM64) | Flatpak, Snap, AppImage, `.deb` | — | Flathub, Snap Store |

### Release Artifacts

Each release includes:

- Platform-specific native package (see table above)
- SHA256 checksum file (`.sha256`)
- Auto-generated release notes from CHANGELOG
- Platform-specific installation instructions

### Auto-Updater Strategy

| Platform | Mechanism |
|----------|-----------|
| macOS | Sparkle framework or App Store auto-update |
| Windows | MSIX auto-update or WinGet |
| Linux | Flatpak/Snap auto-update via their respective stores |

---

## 🍎 Apple Platform Wishlist (v1.2.0+)

> The following Apple-specific features extend MeedyaManager beyond cross-platform parity.
> Each is tracked as an open GitHub issue. They will be scheduled into future milestones once
> cross-platform core quality is established. All require macOS-only Swift code in `macos/`.

| # | Feature | Description | Effort |
| - | ------- | ----------- | ------ |
| #134 | **Music.app Library Import** | Parse `~/Music/Music/` (SQLite + XML) to bulk-import metadata, ratings, play counts, and playlists into MeedyaManager without re-tagging | Medium |
| #135 | **MusicKit Framework** | Replace the REST-based Apple Music provider with the native `MusicKit` framework for richer catalog access, authenticated user-library sync, and on-device catalog search | Medium |
| #136 | **Quick Look Extension** | Register a `QLPreviewExtension` target so Finder shows rich previews (album art, tags, waveform) for media files managed by MeedyaManager | Small |
| #137 | **Siri Shortcuts / App Intents** | Implement `AppIntent` conformances for key operations (scan folder, rename preview, tag lookup) so users can automate MeedyaManager from the Shortcuts app or Siri | Small |
| #138 | **Core ML Audio Fingerprinting** | Use Apple's Sound Analysis framework and Neural Engine to identify tracks on-device without any external API — works fully offline on Apple Silicon | Large |
| #139 | **Spotlight Importer** | Publish the MeedyaManager library to macOS Spotlight via `CoreSpotlight` so every track is instantly findable system-wide, including from Alfred and Raycast | Small |
| #140 | **AirPlay 2 Streaming** | Extend `mm-server` to advertise itself as an AirPlay 2 source, enabling playback on HomePod, Apple TV, and any AirPlay-enabled speaker | Medium |
| #141 | **CloudKit Settings Sync** | Synchronise rename rules, provider credentials, and app preferences across all Apple devices via iCloud / CloudKit — configuration set on Mac appears on iPhone/iPad automatically | Medium |

---

## ⚖️ Licensing & Copyright

### Licence

> **GPL-2.0-or-later** — GNU General Public License v2.0 or later

This ensures compatibility with all dependencies.

### Copyright Notice

All source files include a copyright header:

**Rust files:**

```rust
// (C) 2025-2026 MWBM Partners Ltd
```

**Swift files:**

```swift
// (C) 2025-2026 MWBM Partners Ltd
```

**C# files:**

```csharp
// (C) 2025-2026 MWBM Partners Ltd
```

- **Start year**: 2025 (project inception)
- **End year**: Automatically set to the current year at build time
- **Holder**: MWBM Partners Ltd

---

## 📚 Documentation

| Location | Audience | Content |
|----------|----------|---------|
| `README.md` | Everyone | Project overview, quick start |
| `Project_Plan.md` | Developers | This file — full project plan |
| `PROJECT_STATUS.md` | Everyone | Current status & progress |
| `docs/changelog.md` | Everyone | Detailed change log with dates |
| `docs/ROADMAP.md` | Everyone | Milestone timeline |
| `help/` | End users | Usage docs, troubleshooting, FAQs |
| `.claude/` | AI/Developers | Project brief, Claude context |

---

> 📝 *This document is maintained alongside the codebase and updated with each milestone.*
>
> *Last updated: 2026-03-04*
