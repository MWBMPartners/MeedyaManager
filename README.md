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

**FFI Strategy:**

- **macOS**: UniFFI (Mozilla) auto-generates Swift bindings from Rust
- **Windows**: `cbindgen`/`csbindgen` generates C headers → C# P/Invoke calls Rust `.dll`
- **Linux**: Direct Rust (GTK4 via `gtk4-rs` + `libadwaita`) — no FFI needed

---

## ✨ Features

| Feature | Description |
| --------- | ------------- |
| 👁️ **Real-Time File Watching** | Monitors folders for new media files and processes them automatically (`notify` crate) |
| 📐 **MusicBee-Inspired Rule Engine** | Template syntax with `<Tag>`, `$If()`, `$And()`, `$Or()`, 20+ functions, regex, deep nesting |
| ✏️ **Metadata Editing** | Read/write tags across audio and video formats via `lofty` |
| 🔍 **19 Registered Metadata Providers** | 13 real HTTP clients (MusicBrainz, Spotify, Apple Music, Deezer, TMDb, TheTVDB, OMDb, Apple TV, iTunes Store, Apple Podcasts, ISRC, EIDR, ISWC) + 6 disabled stubs (YouTube Music, Amazon Music, Pandora, Tidal, Shazam, iHeart). **Partial:** `meedya lookup` (CLI) is still a stub; only MusicBrainz is wired into the GTK lookup panel today |
| 🧠 **Smart Classification** | 4-level hierarchy: Media Group → Format → Class → Quality |
| 🔄 **Companion File Tracking** | Moves subtitles, cover art, and disc images alongside media |
| 🗂️ **External JSON5 Config** | File types and metadata tags defined in `filetypes.json5` / `tags.json5` — editable without recompile, user-overridable |
| 🔒 **File Integrity Checking** | SHA256 hash before/after every metadata write; atomic rename (`rename(2)`); rollback + corruption log on failure |
| ⚙️ **Background Service Mode** | Runs as systemd user unit (Linux), launchd agent (macOS), or Windows Service; managed via `meedya service` CLI |
| 📦 **Settings Export / Import** | Portable `.mmprofile` bundles for device migration and backup (`meedya config export/import`) |
| 🧪 **Test Mode** | Safe editing — creates `_MeedyaManager` copies instead of modifying originals; commit or revert when done |
| 🛡️ **Pre-release Safety** | Pre-release builds auto-enable Test Mode; stable upgrade prompts to disable |
| 📜 **Privacy Policy** | No tracking, no analytics; full third-party provider disclosure |
| ☁️ **Cloud Storage Sync** | **Status: not yet implemented.** OneDrive, Google Drive, Dropbox, MEGA, iCloud have real trait definitions and a sync manager, but no provider makes a real network call yet — OAuth flows exist only as comments |
| 🗄️ **Database Export** | **Status: not yet implemented.** MySQL, MariaDB, SQL Server, SQLite, PostgreSQL have real schema/DDL generation and a CLI command, but no backend ever opens a real database connection |
| 🌐 **Secure Media Server** | **Status: not yet implemented.** `mm-server` has real JWT/range-parsing types, but never builds an HTTP router — `meedya serve` prints a stub message and exits. There is no REST API you can call and no web frontend (zero `.html` files in the repo) |
| 🎨 **Native Look & Feel** | SwiftUI + Liquid Glass on macOS, WinUI 3 + Mica on Windows, GTK4 + Libadwaita on Linux |

---

## 💻 Platform Support

| Platform | Architectures | UI Framework | FFI Binding | Store Target |
| ---------- | --------------- | -------------- | ------------- | -------------- |
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

# Build all workspace crates (core, CLI, providers, cloud, export, server, ffi, update)
# Note: mm-gtk (the Linux GTK4 UI) is NOT a workspace member (needs Linux-only
# gettextrs) — build it explicitly with: cargo build -p mm-gtk --release
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

# macOS is a Swift Package Manager package — there is no .xcodeproj.
# Build and run from the command line:
cd macos && swift build
# Or: open Package.swift in Xcode (26.3+ / Swift 6.3 toolchain required)
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

```text
MeedyaManager/
├── Cargo.toml                    # Workspace root
├── rust-toolchain.toml           # Pin Rust version
├── .rustfmt.toml / clippy.toml / deny.toml
│
├── crates/                        # 9 crate directories, 8 workspace members
│   ├── mm-core/                  # Core business logic
│   │   └── src/ (config/, watcher/, classify/, rule_engine/,
│   │            renamer/, companion/, metadata/, state/,
│   │            logging/, health/, error.rs)
│   ├── mm-providers/             # 19 registered metadata lookup providers (13 real, 6 stubs)
│   │   └── src/ (traits.rs, registry.rs, credentials.rs,
│   │            rate_limiter.rs, match_scoring.rs, cover_art.rs,
│   │            music/, video/, podcasts/, identifiers/)
│   ├── mm-cloud/                 # Cloud storage — scaffold only, see M7 below
│   ├── mm-export/                # Database export — scaffold only, see M9 below
│   ├── mm-server/                # Media server — scaffold only, see M10 below
│   ├── mm-cli/                   # Cross-platform CLI (clap)
│   ├── mm-ffi/                   # FFI bindings (UniFFI + cbindgen)
│   ├── mm-update/                # Update-check client (semver, GitHub Releases API)
│   └── mm-gtk/                   # Linux GTK4/Libadwaita UI — NOT a workspace member
│
├── macos/                        # Swift/SwiftUI app (Swift Package Manager — no .xcodeproj)
│   └── MeedyaManager/ (Views/, Models/, Bindings/, Resources/)
│
├── windows/                      # WinUI 3 / C# app
│   ├── MeedyaManager.sln
│   └── MeedyaManager/ (Views/, ViewModels/, Interop/, Assets/)
│
├── config/settings.json5         # Shared default config (schema currently mismatched
│                                  #   with AppConfig — see issue #211; loads as all-defaults)
├── assets/                       # Shared icons/branding
├── branding/                     # Logos
├── docs/                         # Developer docs
├── help/                         # User documentation
├── .github/workflows/            # CI/CD (9 workflows)
├── .claude/                      # Project context
├── Project_Plan.md / PROJECT_STATUS.md / README.md
└── justfile                      # Task runner
```

---

## 🗺️ Milestone Roadmap

| # | Milestone | Status | Description |
| --- | ----------- | -------- | ------------- |
| M0 | 🔧 Repository Setup & Scaffolding | ✅ **Complete** | Archive Python, init Cargo workspace, scaffold native apps, CI setup |
| M1 | 🧱 Core Engine (Rust) | ✅ **Complete** | Config, classification, metadata (`lofty`), watcher (`notify`), renamer, logging |
| M2 | 📐 Rule Engine | ✅ **Complete** | Lexer, recursive descent parser, evaluator, 24 template functions |
| M3 | ⌨️ CLI | ✅ **Complete** (except `lookup`) | `clap`-based commands: scan, debug, watch, rule, edit, config — `meedya lookup` is still a stub |
| M4 | 🖥️ FFI Layer & Native UI Shells | ✅ **Complete** | UniFFI + cbindgen, SwiftUI/WinUI 3/GTK4 app shells |
| M5 | 🔍 Metadata Lookup Providers | ✅ **Complete, partially reachable** | 19 registered providers (13 real HTTP clients, 6 disabled stubs); only MusicBrainz reachable from any UI today |
| M6 | 🎨 Full Native UI | ✅ **Complete** | Rule Builder, Metadata Editor, Lookup Panel on all platforms |
| M7 | ☁️ Cloud Storage Monitoring | ⚠️ **Preview / scaffold only** | Real traits + sync manager, but no provider makes a real network call — OAuth is comment-only |
| M8 | 📦 Packaging & Public Release | ⚠️ **Tooling built, nothing published** | Store submissions, Flatpak/Snap manifests and auto-updater code exist, but no app has been submitted or released; Linux/WinGet manifest versions are out of sync with `Cargo.toml` |
| M9 | 🗄️ Database Export | ⚠️ **Preview / scaffold only** | Schema/DDL generation and CLI plumbing exist for all 5 backends, but no backend opens a real database connection |
| M10 | 🌐 Secure Media Server | ⚠️ **Preview / scaffold only** | JWT + range-parsing types exist, but no `axum` router is ever built; `meedya serve` prints a stub message and exits — no working REST API, streaming, or web frontend |
| — | 🔧 Post-Release Enhancements (i18n, accessibility) | ⚠️ **Preview / scaffold only** | Translation catalogues (`.po`/`.xcstrings`/`.resw`) exist but there are **zero** `gettext()` call sites anywhere in the codebase — translating them has no runtime effect yet |
| — | 🔧 Post-Release Enhancements (tags/integrity/service) | ✅ **Complete** | External JSON5 tag registry, file integrity (SHA256 + atomic write) for metadata writes, background service mode, settings export/import |

> **No version of MeedyaManager has ever been publicly released.** The version in `Cargo.toml` is
> `1.3.0`; the only GitHub release is the pre-rename *"MetaMancer v1.0-M1"* (2025-06-16, pre-release).
> See [PROJECT_STATUS.md](PROJECT_STATUS.md) for the full, current breakdown including test counts
> per crate and open issues.

---

## 🛠️ Technology Stack

### Rust Core

| Purpose | Crate |
| --------- | ------- |
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
| ---------- | ---------- | ----------- | --------- |
| macOS | Swift 6 | SwiftUI | Xcode 26.3+ (Swift 6.3 toolchain) |
| Windows | C# | WinUI 3 / .NET 8 | Visual Studio 2022+ |
| Linux | Rust | GTK4 + Libadwaita | gtk4-rs |

---

## 🍎 Apple Platform Wishlist

The following Apple-specific and Apple-enhanced features are ideas for future releases on macOS (and potentially iOS/iPadOS). These extend MeedyaManager beyond cross-platform parity to take full advantage of the Apple ecosystem.

> **No GitHub issues exist for these yet.** An earlier draft of this table cited issues #134-#141,
> but those numbers belong to unrelated, already-filed issues (mm-core metadata migration, the
> meedya-core dependency, upstream provider contribution, a `deny.toml` fix, FFmpeg packaging, a
> lyrics-file feature, `update.mwbm.io`, and a post-PR cleanup task, respectively) — none of them
> are about this wishlist. File real issues before referencing numbers here again.

| Feature | Description |
| ------- | ----------- |
| 🎵 **Music.app Library Import** | Parse the macOS Music app library (`~/Music/Music/`) to bulk-import existing metadata, ratings, and play counts — zero re-tagging needed for existing collections |
| 🎼 **MusicKit Framework** | Replace the current unauthenticated iTunes Search API lookups with the native `MusicKit` framework for on-device catalog search, richer metadata, and authenticated user-library access |
| 🔭 **Quick Look Extension** | Register a `QLPreviewExtension` so Finder shows rich album-art previews with metadata for any media file managed by MeedyaManager |
| 🗣️ **Siri Shortcuts / App Intents** | Expose MeedyaManager operations (scan folder, rename preview, metadata lookup) as `AppIntent` actions usable in the Shortcuts app and via Siri voice commands |
| 🧠 **Core ML Audio Fingerprinting** | Use Apple's Neural Engine (Core ML / Sound Analysis) for on-device audio fingerprinting — identify tracks without an external API, works fully offline |
| 🔍 **Spotlight Importer** | Publish library metadata to macOS Spotlight via `CoreSpotlight` so every track is searchable system-wide from Spotlight or Alfred |
| 📡 **AirPlay 2 Streaming** | Stream media from the (currently not-yet-implemented, see M10 above) MeedyaManager server to any AirPlay 2 receiver once that server exists |
| ☁️ **CloudKit Settings Sync** | Sync rename rules, config, and preferences across all Apple devices via iCloud / CloudKit — rules set on Mac appear automatically on iPhone/iPad |

> These are unscheduled ideas, not committed work. They will be filed as GitHub issues and
> scheduled into a future milestone once core cross-platform parity is solid.

---

## ⚖️ License

This project is licensed under the **GPL-2.0-or-later**. **A `LICENSE` file is not yet tracked in this repository** (see issue #207) — the licence declaration lives in `Cargo.toml` (`license = "GPL-2.0-or-later"`) until that is fixed.

---

## 📚 Documentation

| Document | Description |
| ---------- | ------------- |
| 📋 [Project_Plan.md](Project_Plan.md) | Full project plan with architecture, milestones & tech stack |
| 📊 [PROJECT_STATUS.md](PROJECT_STATUS.md) | Current progress tracker |
| 📍 [docs/roadmap.md](docs/roadmap.md) | Milestone timeline |
| 📦 [docs/changelog.md](docs/changelog.md) | Detailed change log |
| 📖 [help/getting-started.md](help/getting-started.md) | Getting started guide |
| ⚙️ [help/configuration.md](help/configuration.md) | Configuration reference |
| 📐 [help/rule-syntax.md](help/rule-syntax.md) | Rule template syntax guide |
| 🎵 [help/supported-formats.md](help/supported-formats.md) | Supported file formats |
| 🔍 [help/providers/](help/providers/) | Per-provider metadata lookup setup pages |
| 🔧 [help/troubleshooting.md](help/troubleshooting.md) | Troubleshooting guide |
| ❓ [help/faq.md](help/faq.md) | Frequently asked questions |

---

**(C) 2025–2026 MWBM Partners Ltd**
