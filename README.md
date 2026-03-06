# 🎧📁 MeedyaManager

<p align="center">
  <img src="branding/meedyamanager-logo-animated.svg" alt="MeedyaManager Logo" width="480" height="160" />
</p>

<p align="center">
  <strong>🎵🎬 Smart, cross-platform media file manager and auto-organizer</strong>
  <br />
  <em>Rust core + native UIs — inspired by MusicBee's flexibility, built for everywhere</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/rust-stable-orange.svg" alt="Rust" />
  <img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-green.svg" alt="Platforms" />
  <img src="https://img.shields.io/badge/license-GPL--2.0+-blue.svg" alt="License" />
</p>

---

**(C) 2025–2026 MWBM Partners Ltd**

---

## 🌟 What is MeedyaManager?

**MeedyaManager** is a cross-platform media file management application that automatically monitors folders, reads metadata from audio and video files, and renames/organizes them according to user-defined rules — inspired by MusicBee's auto-organize feature. It is built on a shared **Rust core library** with fully **native UIs** on each platform: SwiftUI on macOS, WinUI 3 on Windows, and GTK4 on Linux. This architecture — the same pattern used by 1Password, Dropbox, and Firefox — delivers native look-and-feel on every platform while sharing all business logic through a single Rust codebase.

---

## 🏗️ Architecture

```
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

**FFI Strategy:**
- **macOS**: UniFFI (Mozilla) auto-generates Swift bindings from Rust
- **Windows**: `cbindgen`/`csbindgen` generates C headers → C# P/Invoke calls Rust `.dll`
- **Linux**: Direct Rust (GTK4 via `gtk4-rs` + `libadwaita`) — no FFI needed

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 👁️ **Real-Time File Watching** | Monitors folders for new media files and processes them automatically (`notify` crate) |
| 📐 **MusicBee-Inspired Rule Engine** | Template syntax with `<Tag>`, `$If()`, `$And()`, `$Or()`, 20+ functions, regex, deep nesting |
| ✏️ **Metadata Editing** | Read/write tags across audio and video formats via `lofty` |
| 🔍 **19+ Metadata Lookup Providers** | Music (10), Video (5), Podcasts (1), Identifiers (3) — with fuzzy matching and cover art |
| 🧠 **Smart Classification** | 4-level hierarchy: Media Group → Format → Class → Quality |
| 🔄 **Companion File Tracking** | Moves subtitles, cover art, and disc images alongside media |
| ☁️ **Cloud Storage Sync** | OneDrive, Google Drive, Dropbox, MEGA, iCloud (planned) |
| 🗄️ **Database Export** | MySQL, MariaDB, SQL Server, SQLite, PostgreSQL (planned) |
| 🌐 **Secure Media Server** | REST API with JWT auth, media streaming, web frontend (planned) |
| 🎨 **Native Look & Feel** | SwiftUI + Liquid Glass on macOS, WinUI 3 + Mica on Windows, GTK4 + Libadwaita on Linux |

---

## 💻 Platform Support

| Platform | Architectures | UI Framework | FFI Binding | Store Target |
|----------|---------------|--------------|-------------|--------------|
| 🍎 **macOS** | Apple Silicon (arm64) | SwiftUI (Swift 6) | UniFFI | App Store |
| 🪟 **Windows** | x64, ARM64 | WinUI 3 (C# .NET 8) | cbindgen / P/Invoke | Microsoft Store |
| 🐧 **Linux** | x86_64, ARM64 | GTK4 + Libadwaita (gtk4-rs) | Direct Rust | Flatpak / Snap |

---

## 🚀 Quick Start

### Prerequisites

- **Rust** (stable, via [rustup](https://rustup.rs/))
- Platform-specific toolchain (see below)

### Build the Rust Core & CLI

```bash
# Clone the repository
git clone https://github.com/MWBMPartners/MeedyaManager.git
cd MeedyaManager

# Build all Rust crates (core, CLI, providers, GTK UI)
cargo build --workspace

# Run all tests
cargo test --workspace

# Run the CLI
cargo run -p mm-cli -- scan ~/Music
```

### Build the macOS App (SwiftUI)

```bash
# Build the Rust FFI library for macOS
cargo build -p mm-ffi --release

# Open in Xcode and build
open macos/MeedyaManager.xcodeproj
# Or build from command line:
cd macos && swift build
```

### Build the Windows App (WinUI 3)

```powershell
# Build the Rust FFI library for Windows
cargo build -p mm-ffi --release

# Build the C# / WinUI 3 project
cd windows
dotnet build
```

### Build the Linux App (GTK4)

```bash
# Build the GTK4 UI directly (no FFI needed)
cargo build -p mm-gtk --release
```

---

## 📂 Project Structure

```
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
├── docs/                         # Developer docs
├── help/                         # User documentation
├── .github/workflows/            # CI/CD (7 workflows)
├── .claude/                      # Project context
├── Project_Plan.md / PROJECT_STATUS.md / README.md
└── justfile                      # Task runner
```

---

## 🗺️ Milestone Roadmap

| # | Milestone | Status | Description |
|---|-----------|--------|-------------|
| M0 | 🔧 Repository Setup & Scaffolding | ✅ **Complete** | Archive Python, init Cargo workspace, scaffold native apps, CI stubs |
| M1 | 🧱 Core Engine (Rust) | ✅ **Complete** | Config, classification, metadata (`lofty`), watcher (`notify`), renamer, logging (217 tests) |
| M2 | 📐 Rule Engine | ✅ **Complete** | Lexer, recursive descent parser, evaluator, 20+ template functions (182 tests) |
| M3 | ⌨️ CLI | ✅ **Complete** | `clap`-based commands: scan, debug, watch, rule, edit, lookup, config (45 tests) |
| M4 | 🖥️ FFI Layer & Native UI Shells | ✅ **Complete** | UniFFI + cbindgen, SwiftUI/WinUI 3/GTK4 app shells (20 tests) |
| M5 | 🔍 Metadata Lookup Providers | ✅ **Complete** | 19+ providers, fuzzy matching, rate limiting, cover art (332 tests) |
| M6 | 🎨 Full Native UI | ✅ **Complete** | Rule Builder, Metadata Editor, Lookup Panel on all platforms (~90 tests) |
| M7 | ☁️ Cloud Storage Monitoring | ✅ **Complete** | OneDrive, Google Drive, Dropbox, MEGA stub, iCloud stub (~90 tests) |
| M8 | 📦 Packaging & Public Release | ✅ **Complete** | App Store, Microsoft Store, Flatpak/Snap, auto-updater (~33 tests) |
| M9 | 🗄️ Database Export | ✅ **Complete** | MySQL, MariaDB, SQL Server, SQLite, PostgreSQL (~90 tests) |
| M10 | 🌐 Secure Media Server | ✅ **Complete** | `axum` HTTP server, REST API, JWT auth, media streaming — **v1.0.0 released** (~90 tests) |

---

## 🛠️ Technology Stack

### Rust Core

| Purpose | Crate |
|---------|-------|
| File watching | `notify` |
| Metadata read/write | `lofty` |
| CLI framework | `clap` |
| HTTP client | `reqwest` |
| Async runtime | `tokio` |
| Config (JSON5) | `json5` + `serde` |
| Environment vars | `dotenvy` |
| Logging | `tracing` + `tracing-subscriber` |
| FFI (Swift) | `uniffi` |
| FFI (C header) | `cbindgen` |
| GTK4 UI | `gtk4-rs` + `libadwaita` |
| Rate limiting | `governor` |
| Fuzzy matching | `fuzzy-matcher` |
| Credential storage | `keyring` |
| Error types | `thiserror` |
| Regex | `regex` |
| OAuth2 | `oauth2` |
| JWT | `jsonwebtoken` |

### Native UIs

| Platform | Language | Framework | Version |
|----------|----------|-----------|---------|
| macOS | Swift 6 | SwiftUI | Xcode 16+ |
| Windows | C# | WinUI 3 / .NET 8 | Visual Studio 2022+ |
| Linux | Rust | GTK4 + Libadwaita | gtk4-rs |

---

## 🍎 Apple Platform Wishlist

The following Apple-specific and Apple-enhanced features are planned for future releases on macOS (and potentially iOS/iPadOS). These extend MeedyaManager beyond cross-platform parity to take full advantage of the Apple ecosystem.

| Feature | Description | GitHub Issue |
| ------- | ----------- | ------------ |
| 🎵 **Music.app Library Import** | Parse the macOS Music app library (`~/Music/Music/`) to bulk-import existing metadata, ratings, and play counts — zero re-tagging needed for existing collections | #134 |
| 🎼 **MusicKit Framework** | Replace REST-based Apple Music lookups with the native `MusicKit` framework for on-device catalog search, richer metadata, and authenticated user-library access | #135 |
| 🔭 **Quick Look Extension** | Register a `QLPreviewExtension` so Finder shows rich album-art previews with metadata for any media file managed by MeedyaManager | #136 |
| 🗣️ **Siri Shortcuts / App Intents** | Expose MeedyaManager operations (scan folder, rename preview, metadata lookup) as `AppIntent` actions usable in the Shortcuts app and via Siri voice commands | #137 |
| 🧠 **Core ML Audio Fingerprinting** | Use Apple's Neural Engine (Core ML / Sound Analysis) for on-device audio fingerprinting — identify tracks without an external API, works fully offline | #138 |
| 🔍 **Spotlight Importer** | Publish library metadata to macOS Spotlight via `CoreSpotlight` so every track is searchable system-wide from Spotlight or Alfred | #139 |
| 📡 **AirPlay 2 Streaming** | Stream media from the built-in MeedyaManager server to any AirPlay 2 receiver (HomePod, Apple TV, AirPlay-enabled speaker) | #140 |
| ☁️ **CloudKit Settings Sync** | Sync rename rules, config, and preferences across all Apple devices via iCloud / CloudKit — rules set on Mac appear automatically on iPhone/iPad | #141 |

> These features are tracked as **wishlist GitHub Issues** and will be scheduled in a future milestone once core cross-platform parity is solid.

---

## ⚖️ License

This project is licensed under the **GPL-2.0-or-later** — see the [LICENSE](LICENSE) file for details.

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| 📋 [Project_Plan.md](Project_Plan.md) | Full project plan with architecture, milestones & tech stack |
| 📊 [PROJECT_STATUS.md](PROJECT_STATUS.md) | Current progress tracker |
| 📍 [docs/ROADMAP.md](docs/ROADMAP.md) | Milestone timeline |
| 📦 [docs/changelog.md](docs/changelog.md) | Detailed change log |
| 📖 [help/getting-started.md](help/getting-started.md) | Getting started guide |
| ⚙️ [help/configuration.md](help/configuration.md) | Configuration reference |
| 📐 [help/rule-syntax.md](help/rule-syntax.md) | Rule template syntax guide |
| 🎵 [help/supported-formats.md](help/supported-formats.md) | Supported file formats |
| 🔍 [help/provider-setup.md](help/provider-setup.md) | Metadata lookup provider setup |
| 🔧 [help/troubleshooting.md](help/troubleshooting.md) | Troubleshooting guide |
| ❓ [help/faq.md](help/faq.md) | Frequently asked questions |

---

**(C) 2025–2026 MWBM Partners Ltd**
