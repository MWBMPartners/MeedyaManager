# 📋 MeedyaManager — Project Plan

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**
>
> 🎧📁 A smart, cross-platform media manager and auto-organizer — inspired by MusicBee's flexibility, built for everywhere.

---

## 📖 Table of Contents

1. [Project Overview](#-project-overview)
2. [Technology Stack](#-technology-stack)
3. [Architecture](#-architecture)
4. [Platform Support](#-platform-support)
5. [Milestone Roadmap](#-milestone-roadmap)
6. [Rule Engine Design](#-rule-engine-design)
7. [Metadata Hierarchy](#-metadata-hierarchy)
8. [Third-Party Libraries & Components](#-third-party-libraries--components)
9. [API Key Management](#-api-key-management)
10. [CI/CD & Packaging](#-cicd--packaging)
11. [Documentation Strategy](#-documentation-strategy)
12. [Licensing & Copyright](#-licensing--copyright)

---

## 🎯 Project Overview

**MeedyaManager** is a cross-platform media file management application that automatically monitors folders, reads metadata from audio/video files, and renames/organizes them according to user-defined rules — similar to MusicBee's auto-organize feature, but available on **Windows, macOS, and Linux**.

### 🌟 Core Goals

| Goal | Description |
|------|-------------|
| 🖥️ **Cross-Platform** | Windows (x64/ARM), macOS (Apple Silicon), Linux (x64/ARM) |
| 👁️ **Continuous Monitoring** | Real-time folder watching with file-lock awareness |
| 🧠 **Smart Classification** | 4-level media hierarchy (group → format → class → quality) |
| 📐 **Flexible Rules** | MusicBee-inspired template engine with `$If`, `$And`, `$Or`, nesting |
| 🎵 **Format Support** | MP3, FLAC, ALAC, M4A, MP4, MKV, AVI, OGG, AC3, EAC3, HEVC + more |
| 🔊 **Audio Characteristics** | Lossy/Lossless, Dolby Digital/Plus, Spatial Audio detection |
| 🔄 **Companion Files** | Automatically move SRT, LRC, cover art, ISOs alongside media |
| ⚡ **Lightweight** | Minimal resource usage; runs as background service |
| 🌙 **Service Mode** | Auto-start on boot/login; optional system service |
| 🎨 **Dark/Light UI** | System-aware theme switching |

### 🔮 Future Goals (Planned but deferred)

| Goal | Milestone |
|------|-----------|
| ✏️ Manual Metadata Editing | M4 |
| 🎵 Music Metadata Lookup (MusicBrainz, Spotify, Apple Music, etc.) | M5 |
| 🎬 TV/Film Metadata Lookup (TMDb, TheTVDB, IMDb, etc.) | M6 |
| ☁️ Cloud Storage Monitoring (OneDrive, Google Drive, Dropbox, etc.) | M7 |
| 🗄️ External Database Export (MySQL, MariaDB, PostgreSQL, etc.) | M9 |
| 🌐 Secure Media Server with Web Interface | M10 |

---

## 🛠️ Technology Stack

### Primary Language: **Python 3.14+**

> ⚡ **Bundled & Sandboxed Runtime:** MeedyaManager ships with its own embedded Python runtime, compiled via **Nuitka**. This means:
> - The app is completely self-contained — no Python installation required on the host
> - It will **never** interfere with any other Python version or virtual environment on the user's machine
> - Users don't need to know or care that Python is involved at all
> - The runtime is locked to the exact version we test against, eliminating version mismatch issues

Python 3.14 (latest stable as of Feb 2026) was chosen for:
- ✅ True cross-platform support (Windows, macOS, Linux)
- ✅ Excellent media metadata libraries (`pymediainfo`, `mutagen`)
- ✅ Lightweight background service capability
- ✅ Strong ecosystem for file system watching (`watchdog`)
- ✅ Compiles to native code via Nuitka (faster startup, smaller footprint)
- ✅ Rich UI framework options (PySide6/Qt6 for native-looking GUI)

### Packaging & Distribution: **Nuitka**

| Aspect | Detail |
|--------|--------|
| **Compiler** | [Nuitka](https://nuitka.net) — compiles Python to C/C++, then to native machine code |
| **Runtime** | Embedded Python 3.14 runtime, fully isolated from host |
| **Sandboxing** | App uses its own bundled Python — zero interaction with host Python installations |
| **Output** | Single-folder or single-file native executable per platform |
| **Performance** | Faster startup and lower memory usage vs interpreted Python |
| **Platforms** | Produces native binaries for Windows (.exe), macOS (.app), Linux (ELF) |

### Core Dependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `pymediainfo` | >=6.0 | Metadata extraction via MediaInfo library |
| `mutagen` | >=1.47 | Direct tag reading/writing (ID3, MP4, FLAC, OGG, MKV) |
| `watchdog` | >=4.0 | Real-time filesystem event monitoring |
| `json5` | >=0.9 | Config file parsing with comments support |
| `python-dotenv` | >=1.0 | Environment variable loading for API keys |
| `colorama` | >=0.4.6 | Cross-platform terminal colour output |
| `tqdm` | >=4.66 | Progress bars for batch operations |
| `rich` | >=13.0 | Enhanced CLI formatting, tables, and panels |
| `click` | >=8.1 | CLI framework for commands and arguments |
| `pydantic` | >=2.5 | Settings/config validation and type safety |

### UI Framework (M2+): **PySide6 6.10+**

| Package | Version | Purpose |
|---------|---------|---------|
| `PySide6` | >=6.10 | Qt6-based cross-platform GUI (LGPL-compatible) |
| `darkdetect` | >=0.8 | System dark/light mode detection |
| `pyobjc-framework-Cocoa` | >=10.0 | macOS-only: Native AppKit access for Liquid Glass |

#### Native Platform Appearance

PySide6 6.10 (latest as of Feb 2026) provides native platform styling:
- 🪟 **Windows:** Native Windows 11 widget styling (Mica/Acrylic effects, Segoe UI)
- 🐧 **Linux:** Fusion style (clean, consistent across desktops) with GTK/KDE theme integration

#### 🍎 macOS: Liquid Glass Support

On macOS 26 (Tahoe), Apple introduced **Liquid Glass** — a translucent, depth-aware design language that is the biggest visual redesign since iOS 7. MeedyaManager will support Liquid Glass on macOS through a **PyObjC bridge** approach:

| Layer | Technology | What It Does |
|-------|-----------|--------------|
| **Core UI** | PySide6 / Qt6 | Cross-platform widgets, layouts, and interaction |
| **Glass Layer** | PyObjC → `NSGlassEffectView` | Injects native Liquid Glass materials into PySide6 windows |
| **Fallback** | PyObjC → `NSVisualEffectView` | Vibrancy/blur on macOS 10.14+ (pre-Tahoe) |
| **Detection** | Runtime check | Automatically uses the best available effect for the macOS version |

**How it works:**
1. PySide6 creates the window and all widgets as normal (cross-platform code)
2. On macOS, a platform helper module uses `pyobjc` to access the window's native `NSWindow`
3. It injects Apple's native `NSGlassEffectView` (or `NSVisualEffectView` on older macOS) behind Qt's rendering surface
4. The result: genuine Liquid Glass appearance matching native macOS apps, with zero impact on Windows/Linux

**Material options available:** Sidebar, HUD, Popover, Frosted, Clear Glass, and more — matching Apple's native material presets.

> **Note:** Qt is also working on native Liquid Glass support in a future PySide6 release. When that ships, MeedyaManager will adopt it and the PyObjC bridge will become a graceful fallback.

Dark/light mode follows the system setting automatically via `darkdetect` + Qt6's built-in theme awareness + native macOS `NSAppearance`.

### Metadata Lookup Dependencies (M5)

| Package | Purpose |
|---------|---------|
| `httpx` | Async HTTP client for provider API calls |
| `tenacity` | Retry logic for API calls |
| `spotipy` | Spotify Web API client (OAuth2) |
| `musicbrainzngs` | MusicBrainz API client |
| `deezer-python` | Deezer public API client |
| `tidalapi` | Tidal API client (OAuth2.1) |
| `ytmusicapi` | YouTube Music (cookie-based auth) |
| `shazamio` | Shazam audio fingerprinting |
| `tmdbsimple` | TMDb API client |
| `cinemagoer` | IMDb data access |
| `pyjwt[crypto]` | JWT authentication (Apple Music) |
| `cryptography` | AES-256-GCM encrypted credential storage |
| `keyring` | OS-native secure credential storage |
| `fuzzywuzzy` | Fuzzy string matching for metadata scoring |
| `python-Levenshtein` | Fast Levenshtein distance for fuzzy matching |

### Future Dependencies (as needed per milestone)

| Package | Milestone | Purpose |
|---------|-----------|---------|
| `SQLAlchemy` | M9 | Multi-database ORM |

### Development / Build Dependencies

| Package | Purpose |
|---------|---------|
| `nuitka` | Python-to-C compiler for native standalone builds |
| `ordered-set` | Nuitka dependency for optimised compilation |
| `pytest` | Testing framework |
| `pytest-cov` | Coverage reporting |

---

## 🏗️ Architecture

MeedyaManager follows a **modular, layered architecture** designed for maintainability and progressive feature addition.

```
MeedyaManager/
├── 📁 core/                    # Core business logic
│   ├── __init__.py
│   ├── watcher.py              # File system monitoring (watchdog + polling)
│   ├── metadata_extractor.py   # Metadata reading via pymediainfo
│   ├── classify_media.py       # 4-level media classification engine
│   ├── renamer.py              # Rename simulation & execution engine
│   ├── rule_engine.py          # Template parser & conditional evaluator (M2+)
│   ├── companion_tracker.py    # Companion file detection & grouping (M3+)
│   └── file_lock_checker.py    # Cross-platform file-in-use detection
│
├── 📁 cli/                     # Command-line interface
│   ├── __init__.py
│   ├── runner.py               # Main CLI entry point
│   ├── metadata_debugger.py    # Single-file metadata inspector
│   └── rule_tester.py          # Rule validation & preview tool (M2+)
│
├── 📁 ui/                      # GUI application (M2+)
│   ├── __init__.py
│   ├── main_window.py          # Main application window
│   ├── rule_builder.py         # Visual rule editor
│   ├── preview_panel.py        # Rename preview & simulation
│   ├── settings_dialog.py      # Configuration UI
│   ├── platform_style.py       # Per-platform native styling (Liquid Glass, Mica, etc.)
│   └── themes/                 # Dark/light theme assets
│       ├── dark.qss
│       └── light.qss
│
├── 📁 metadata/                # Metadata editing & lookup (M4+)
│   ├── __init__.py
│   ├── editor.py               # Tag reading/writing engine
│   ├── multi_value.py          # Multi-value tag handling
│   └── providers/              # Lookup service integrations (M5-M6)
│       ├── __init__.py
│       ├── musicbrainz.py
│       ├── spotify.py
│       ├── apple_music.py
│       ├── tidal.py
│       ├── amazon_music.py
│       ├── shazam.py
│       ├── acousticbrainz.py
│       ├── tmdb.py
│       ├── tvdb.py
│       ├── imdb.py
│       ├── apple_tv.py
│       ├── itunes_store.py
│       └── eidr.py
│
├── 📁 cloud/                   # Cloud service integration (M7+)
│   ├── __init__.py
│   ├── sync_manager.py
│   └── providers/
│       ├── onedrive.py
│       ├── google_drive.py
│       ├── dropbox.py
│       ├── mega.py
│       └── icloud.py
│
├── 📁 export/                  # Database export & media server (M9-M10)
│   ├── __init__.py
│   ├── db_exporter.py
│   └── media_server.py
│
├── 📁 utils/                   # Shared utilities
│   ├── __init__.py
│   ├── config_loader.py        # JSON5 config with defaults & validation
│   ├── env_loader.py           # .env file loading
│   ├── verify_checksum.py      # Post-install SHA256 verification
│   ├── char_replacer.py        # Filename character sanitisation
│   ├── platform_utils.py       # OS-specific helpers (service install, etc.)
│   └── logger.py               # Centralised logging with PII redaction
│
├── 📁 service/                 # Background service / daemon support
│   ├── __init__.py
│   ├── service_manager.py      # Cross-platform service registration
│   ├── windows_service.py      # Windows Service (via pywin32)
│   ├── macos_launchd.py        # macOS LaunchDaemon/LaunchAgent plist
│   └── linux_systemd.py        # Linux systemd unit file generation
│
├── 📁 config/                  # Configuration files
│   └── settings.json5          # Main user config (JSON5 with comments)
│
├── 📁 tests/                   # Test suite
│   ├── __init__.py
│   ├── test_*.py               # Unit & integration tests
│   └── fixtures/               # Test data files
│
├── 📁 branding/                # Logo and brand assets
│   ├── meedyamanager-logo.svg
│   └── meedyamanager-logo-animated.svg
│
├── 📁 docs/                    # Developer documentation
│   ├── CHANGELOG.md
│   └── ROADMAP.md
│
├── 📁 help/                    # User documentation
│   ├── getting-started.md
│   ├── configuration.md
│   ├── rule-syntax.md
│   ├── supported-formats.md
│   ├── troubleshooting.md
│   └── faq.md
│
├── 📁 .claude/                 # Claude AI context & project brief
│   ├── CLAUDE.md
│   └── ProjectBrief_Chat.claude
│
├── 📁 .github/                 # GitHub configuration
│   ├── workflows/              # CI/CD pipelines
│   │   ├── python-app.yml
│   │   ├── test-suite.yml
│   │   └── build-artifacts.yml
│   └── ISSUE_TEMPLATE/
│
├── .env.example                # Template for environment variables
├── .gitignore                  # Git ignore rules
├── README.md                   # Project overview
├── Project_Plan.md             # This file — detailed project plan
├── PROJECT_STATUS.md           # Current status tracker
├── requirements.txt            # Python dependencies
├── setup.py                    # Package setup (M8+)
└── LICENSE                     # Project licence
```

### Key Architectural Principles

1. **🧩 Modular Design** — Each feature area is its own package with clear interfaces
2. **📦 Progressive Loading** — Optional modules (UI, cloud, export) are loaded only when needed
3. **🔌 Plugin-Style Providers** — Metadata lookup services follow a common interface
4. **⚙️ Config-Driven** — All behaviour is configurable via JSON5 + environment overrides
5. **🛡️ Safety First** — File-lock detection prevents corruption; dry-run mode by default
6. **📊 Observable** — Comprehensive logging with PII redaction for safe troubleshooting

---

## 💻 Platform Support

| Platform | Architectures | Service Support | Notes |
|----------|---------------|-----------------|-------|
| 🪟 **Windows** | x64, ARM64 | Windows Service (via `pywin32`) | Full support |
| 🍎 **macOS** | Apple Silicon (arm64) only | LaunchDaemon / LaunchAgent | M-series Macs only |
| 🐧 **Linux** | x86_64, ARM64 | systemd service unit | Major distros |

### Service / Auto-Start Modes

| Mode | Description | Requires Login? |
|------|-------------|-----------------|
| **System Service** | Runs at boot as a daemon/service | ❌ No |
| **Login Agent** | Starts when a user logs in | ✅ Yes |
| **Manual** | User starts manually via CLI or GUI | ✅ Yes |

---

## 🗺️ Milestone Roadmap

### ✅ M1 — Core Engine *(Completed — June 2025)*

> Foundation: file watching, metadata extraction, classification, dry-run rename simulation.

| Feature | Status |
|---------|--------|
| Folder watcher (`watchdog` + polling fallback) | ✅ Complete |
| Metadata extraction via `pymediainfo` | ✅ Complete |
| 4-level media classification hierarchy | ✅ Complete |
| Dry-run rename simulation engine | ✅ Complete |
| `settings.json5` configuration | ✅ Complete |
| CLI with `--simulate-off`, `--out`, `--mkdir`, `--json` | ✅ Complete |
| PII-safe logging with rotation | ✅ Complete |
| `.env` loader for API keys | ✅ Complete |
| Checksum verification (`verify_checksum.py`) | ✅ Complete |
| GitHub Actions CI (3 OS × 2 Python versions) | ✅ Complete |
| Release packaging with SHA256 checksums | ✅ Complete |
| 17 unit tests (787 lines) | ✅ Complete |

---

### ✅ M2 — CLI & UI Frontend *(Completed — February 2026)*

> Interactive CLI wizard and cross-platform GUI for configuring rules and previewing renames.

| Feature | Status |
|---------|--------|
| `click`-based CLI framework migration (5 subcommands) | ✅ Complete |
| Rich-formatted CLI output with tables and panels | ✅ Complete |
| PySide6 6.10+ (Qt6) cross-platform GUI | ✅ Complete |
| 🍎 macOS Liquid Glass via PyObjC → `NSGlassEffectView` bridge | ✅ Complete |
| 🪟 Windows 11 Mica/Acrylic native styling | ✅ Complete |
| Dark/light theme support (system-aware) | ✅ Complete |
| Rename preview panel with table model, progress bar, search | ✅ Complete |
| Settings dialog (5 tabs) | ✅ Complete |
| Rule builder with syntax highlighting | ✅ Complete |
| System tray icon with context menu | ✅ Complete |
| Drag-and-drop file import | ✅ Complete |
| 73 tests (CLI + GUI + core), all passing | ✅ Complete |

---

### ✅ M3 — Rule Engine & Companion Files *(Completed — February 2026)*

> Advanced template engine, filename sanitisation, and companion file tracking.

| Feature | Status |
|---------|--------|
| Full template syntax: `<Tag>`, `$If()`, `$And()`, `$Or()` | ✅ Complete |
| 20 template functions ($Replace, $RxReplace, $Pad, $Date, etc.) | ✅ Complete |
| Unlimited custom tag support (`<Custom:AnyName>`) | ✅ Complete |
| Companion file detection (SRT, LRC, cover art, ISO) | ✅ Complete |
| Companion file group movement (move all when media moves) | ✅ Complete |
| Advanced filename character replacement (configurable) | ✅ Complete |
| Deep nesting support (50-level depth guard) | ✅ Complete |
| Legacy `{placeholder}` backward compatibility | ✅ Complete |
| 212 tests (139 new), all passing | ✅ Complete |

---

### ✅ M4 — Metadata Editor *(Completed — February 2026)*

> Manual metadata editing with multi-value tag support and batch operations.

| Feature | Status |
|---------|--------|
| Full tag reading/writing via `mutagen` (TagEditor class) | ✅ Complete |
| Support for ID3v2 (MP3), MP4/M4A, FLAC, OGG Vorbis, ASF (read-only) | ✅ Complete |
| Multi-value tag support (artists, genres, composers) | ✅ Complete |
| Custom tag creation and editing (TXXX, freeform, Vorbis) | ✅ Complete |
| Batch tag editing across multiple files (GUI + CLI) | ✅ Complete |
| Tag preview before applying changes (dry-run) | ✅ Complete |
| Cover art management (embed, extract, replace, remove) | ✅ Complete |
| GUI metadata editor panel with tag table, cover art widget | ✅ Complete |
| CLI `meedyamanager edit` command | ✅ Complete |
| 342 tests (130 new), all passing | ✅ Complete |

---

### ✅ M5 — Metadata Lookup *(Completed — February 2026)*

> 19 metadata lookup providers across music, video, podcasts, and identifier registries.

| Provider / Feature | Status |
|----------|--------|
| Provider framework with `@register_provider` auto-discovery | ✅ Complete |
| 4-tier credential management (.env → config → keyring → bundle) | ✅ Complete |
| Token bucket rate limiting per provider | ✅ Complete |
| Cover art: static (JPEG/PNG) + animated (MP4 square, portrait) | ✅ Complete |
| Fuzzy match scoring (title 35%, artist 30%, album 20%, ISRC bonus) | ✅ Complete |
| 🎵 Apple Music, Spotify, MusicBrainz, Deezer, YouTube Music | ✅ Complete |
| 🎵 Amazon Music, Pandora, Tidal, Shazam, iHeart | ✅ Complete |
| 🎬 TMDB, TheTVDB, IMDb, Apple TV, iTunes Store | ✅ Complete |
| 🎙️ Apple Podcasts | ✅ Complete |
| 🆔 ISRC, EIDR, ISWC | ✅ Complete |
| CLI: `meedyamanager lookup` command | ✅ Complete |
| GUI: Lookup tab with provider checkboxes, results table | ✅ Complete |
| 751 tests (409 new), all passing | ✅ Complete |

---

### ✅ M6 — Packaging, Error Handling & Config Profiles *(Completed — February 2026)*

> Centralized logging, crash protection, user-friendly error dialogs, configuration export/import, native platform installers.

| Feature | Status |
|---------|--------|
| Centralized logging with platform-aware log dirs | ✅ Complete |
| Global exception handling + crash reports | ✅ Complete |
| SafeWorker QThread base class | ✅ Complete |
| User-friendly error dialogs with message catalog | ✅ Complete |
| Error reporting (email-based bug reports) | ✅ Complete |
| Startup health checks | ✅ Complete |
| Crash recovery & state management (WatcherState + AppLockFile) | ✅ Complete |
| Config export/import (.mmprofile ZIP bundles) | ✅ Complete |
| pyproject.toml (PEP 621), icon assets, Nuitka entry scripts | ✅ Complete |
| CI: build-installers.yml (macOS .dmg, Windows .exe, Linux .AppImage/.deb) | ✅ Complete |
| 1007 tests (256 new), all passing | ✅ Complete |

---

### ☁️ M7 — Cloud Storage Monitoring

> Connect to cloud services for remote folder monitoring and auto-organisation.

| Provider | Status |
|----------|--------|
| 📁 OneDrive (Personal) | 🔲 Planned |
| 🏢 OneDrive for Business / SharePoint | 🔲 Planned |
| 📁 Google Drive | 🔲 Planned |
| 📁 Dropbox | 🔲 Planned |
| 🔒 MEGA.nz | 🔲 Planned |
| 🍎 iCloud Drive | 🔲 Planned |

Features: OAuth authentication, background sync worker, conflict resolution.

---

### 📦 M8 — Public Release

> Packaging, distribution, and user-facing polish.

| Feature | Status |
|---------|--------|
| Nuitka compilation to native standalone binaries | 🔲 Planned |
| Bundled Python 3.14 runtime (sandboxed, isolated from host) | 🔲 Planned |
| GitHub Actions auto-create packages (Windows x64/ARM, macOS ARM, Linux x64/ARM) | 🔲 Planned |
| Windows: MSI/NSIS installer with embedded runtime | 🔲 Planned |
| macOS: `.app` bundle inside `.dmg` (Apple Silicon native) | 🔲 Planned |
| Linux: DEB/RPM packages + AppImage | 🔲 Planned |
| Auto-updater design | 🔲 Planned |
| First public alpha release | 🔲 Planned |

---

### 🗄️ M9 — Media Library Database Export

> Export library metadata to external databases for web-hosted search/indexing.

Each database engine is a sub-release:

| Database | Sub-Version | Status |
|----------|-------------|--------|
| MySQL | M9.1 | 🔲 Planned |
| MariaDB | M9.2 | 🔲 Planned |
| SQL Server | M9.3 | 🔲 Planned |
| SQLite | M9.4 | 🔲 Planned |
| PostgreSQL | M9.5 | 🔲 Planned |

---

### 🌐 M10 — Secure Media Server

> Optional media file export with web-accessible, access-controlled library.

| Feature | Status |
|---------|--------|
| Export/copy media files to web server | 🔲 Planned |
| Reference links in external database | 🔲 Planned |
| Access control and user authentication | 🔲 Planned |
| Multi-format support (FLAC, ALAC, M4A, MP3) | 🔲 Planned |
| Web interface for browsing/searching library | 🔲 Planned |
| Security hardening to prevent piracy concerns | 🔲 Planned |

---

## 🔧 Rule Engine Design

MeedyaManager's rule engine is inspired by [MusicBee's template system](https://musicbee.fandom.com/wiki/Templates) but extended to support unlimited custom tags and additional media types.

### Template Syntax

```
<Album Artist>/<Album>/<Track #> - <Title>.<Ext>
```

### Tag References

Tags are referenced using angle brackets: `<TagName>`

| Category | Example Tags |
|----------|-------------|
| **Standard Audio** | `<Title>`, `<Artist>`, `<Album>`, `<Album Artist>`, `<Year>`, `<Genre>`, `<Track #>`, `<Disc #>` |
| **Standard Video** | `<Show>`, `<Season>`, `<Episode>`, `<Director>`, `<Resolution>` |
| **Classification** | `<Media Group>`, `<Format Class>`, `<Media Class>`, `<Quality Type>` |
| **Audio Properties** | `<Codec>`, `<Bitrate>`, `<Sample Rate>`, `<Channels>`, `<Spatial Format>` |
| **File Properties** | `<Filename>`, `<Ext>`, `<Path>`, `<File Size>`, `<Date Added>` |
| **Custom** | `<Custom:AnyName>` — unlimited user-defined tags |

### Available Functions

| Function | Syntax | Description |
|----------|--------|-------------|
| `$If` | `$If(<Tag>=value, true, false)` | Conditional evaluation |
| `$And` | `$If($And(cond1, cond2), true, false)` | Both conditions must be true |
| `$Or` | `$If($Or(cond1, cond2), true, false)` | Either condition can be true |
| `$IsNull` | `$IsNull(<Tag>, ifNull, ifPresent)` | Handle missing/empty tags |
| `$Contains` | `$Contains(<Tag>, search)` | Check if tag contains text |
| `$IsMatch` | `$IsMatch(<Tag>, regex)` | Regex pattern matching |
| `$Replace` | `$Replace(<Tag>, find, replace)` | Text replacement |
| `$RxReplace` | `$RxReplace(<Tag>, regex, replace)` | Regex replacement |
| `$Left` | `$Left(<Tag>, n)` | First n characters |
| `$Right` | `$Right(<Tag>, n)` | Last n characters |
| `$Upper` | `$Upper(<Tag>)` | Convert to uppercase |
| `$Lower` | `$Lower(<Tag>)` | Convert to lowercase |
| `$Trim` | `$Trim(<Tag>)` | Remove leading/trailing spaces |
| `$Pad` | `$Pad(<Tag>, n)` | Zero-pad number to n digits |
| `$Split` | `$Split(<Tag>, delim, n)` | Split and get nth part |
| `$First` | `$First(<Tag>)` | First value from multi-value field |
| `$Date` | `$Date(<Tag>, format)` | Format a date field |
| `$Sort` | `$Sort(<Tag>)` | Apply sort-word stripping (The, A, An) |

### Example Rules

**Basic music organisation:**
```
Music/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>
```

**Genre-based sorting with quality awareness:**
```
$If(<Quality Type>=Lossless,
    Music/Lossless/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>,
    Music/Lossy/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>
)
```

**TV show organisation:**
```
$If(<Media Class>=TV Show,
    TV Shows/<Show>/Season <$Pad(<Season>,2)>/<Show> - S<$Pad(<Season>,2)>E<$Pad(<Episode>,2)> - <Title>.<Ext>,
    $If(<Media Class>=Movie,
        Movies/<Title> (<Year>)/<Title>.<Ext>,
        Unsorted/<Filename>.<Ext>
    )
)
```

**Spatial audio detection:**
```
$If($Or($Contains(<Spatial Format>,"Atmos"), $Contains(<Spatial Format>,"360 Reality")),
    Music/Spatial Audio/<Album Artist>/<Album>/<Title>.<Ext>,
    Music/Standard/<Album Artist>/<Album>/<Title>.<Ext>
)
```

---

## 🧠 Metadata Hierarchy

All media processed by MeedyaManager is classified into a 4-level hierarchy:

| Level | Field | Description | Example Values |
|-------|-------|-------------|----------------|
| 1️⃣ | `media_group` | High-level category | Audio, Video, Image, Book |
| 2️⃣ | `format_class` | Container/codec format | MP3, FLAC, MP4, MKV, PDF |
| 3️⃣ | `media_class` | Content type / purpose | Music, Movie, TV Show, Podcast, Radio Show, Music Video |
| 4️⃣ | `quality_type` | Fidelity / compression | Lossy, Lossless |

### Extended Audio Characteristics

| Property | Detection Method | Example Values |
|----------|-----------------|----------------|
| **Codec** | MediaInfo | AAC, FLAC, ALAC, Vorbis, Opus |
| **Lossy/Lossless** | Codec + bitrate analysis | Lossy, Lossless |
| **Channels** | MediaInfo | Mono, Stereo, 5.1, 7.1 |
| **Multichannel Format** | Codec identification | Dolby Digital (AC3), Dolby Digital Plus (EAC3), DTS |
| **Spatial Audio** | Extended codec/format flags | Dolby Atmos, Sony 360 Reality Audio, Apple Spatial |
| **Dolby Vision** | Video HDR metadata | Profile 5, Profile 8, etc. |
| **Bitrate** | MediaInfo | 128 kbps, 320 kbps, 1411 kbps |
| **Sample Rate** | MediaInfo | 44.1 kHz, 48 kHz, 96 kHz, 192 kHz |
| **Bit Depth** | MediaInfo | 16-bit, 24-bit, 32-bit |

### Supported File Formats

| Category | Extensions |
|----------|-----------|
| 🎵 **Audio** | `.mp3`, `.flac`, `.m4a`, `.alac`, `.ogg`, `.wav`, `.ac3`, `.eac3`, `.ac4`, `.mka`, `.opus`, `.wma`, `.aac`, `.aiff` |
| 🎬 **Video** | `.mp4`, `.m4v`, `.mkv`, `.avi`, `.divx`, `.mpg`, `.mpeg`, `.hevc`, `.mov`, `.wmv`, `.webm`, `.ts` |
| 📝 **Companion** | `.srt`, `.lrc`, `.sub`, `.ass`, `.ssa`, `.vtt` (subtitles); `.jpg`, `.png`, `.bmp` (cover art); `.iso`, `.nrg` (disc images); `.cue` (cue sheets) |
| 📖 **Other** | `.pdf` (booklets), `.nfo` (info files) |

---

## 📦 Third-Party Libraries & Components

### Runtime Dependencies

| Library | Licence | Bundleable? | Purpose |
|---------|---------|-------------|---------|
| [MediaInfo](https://mediaarea.net/en/MediaInfo) | BSD-2-Clause | ✅ Yes | Media metadata extraction engine |
| [pymediainfo](https://pypi.org/project/pymediainfo/) | MIT | ✅ Yes | Python wrapper for MediaInfo |
| [mutagen](https://pypi.org/project/mutagen/) | GPL-2.0+ | ✅ Yes (GPL-compatible) | Direct tag reading/writing |
| [watchdog](https://pypi.org/project/watchdog/) | Apache-2.0 | ✅ Yes | Filesystem event monitoring |
| [PySide6](https://pypi.org/project/PySide6/) | LGPL-3.0 | ✅ Yes | Qt6 GUI framework |
| [click](https://pypi.org/project/click/) | BSD-3-Clause | ✅ Yes | CLI framework |
| [pydantic](https://pypi.org/project/pydantic/) | MIT | ✅ Yes | Config validation |
| [rich](https://pypi.org/project/rich/) | MIT | ✅ Yes | Terminal formatting |
| [json5](https://pypi.org/project/json5/) | Apache-2.0 | ✅ Yes | JSON5 config parsing |
| [python-dotenv](https://pypi.org/project/python-dotenv/) | BSD-3-Clause | ✅ Yes | Environment variable loading |
| [darkdetect](https://pypi.org/project/darkdetect/) | BSD-3-Clause | ✅ Yes | OS dark/light mode detection |

### Build & Packaging

| Library | Licence | Purpose |
|---------|---------|---------|
| [Nuitka](https://nuitka.net) | Apache-2.0 | Python-to-C compiler, produces native standalone executables |
| [ordered-set](https://pypi.org/project/ordered-set/) | MIT | Nuitka optimisation dependency |

### Development / Testing Dependencies

| Library | Purpose |
|---------|---------|
| [pytest](https://pypi.org/project/pytest/) | Testing framework |
| [pytest-cov](https://pypi.org/project/pytest-cov/) | Coverage reporting |

All dependencies (including the Python 3.14 runtime itself) are compiled and bundled into the native executable via Nuitka. End users need **zero** pre-installed software — the app is fully self-contained and sandboxed.

---

## 🔑 API Key Management

### Security Strategy

| Scenario | Key Storage | Distribution |
|----------|-------------|--------------|
| **Developer-only keys** | `.env` file (git-ignored) | ❌ Not included in packages |
| **Universal keys** (ToS allows shared use) | Encrypted in app config | ✅ Bundled with app |
| **User-provided keys** | User's local config / UI settings | ✅ User manages |

### Per-Service Configuration

Each API provider has a toggle in the build configuration:

```json5
{
  api_keys: {
    musicbrainz: { include_in_build: true,  key: "..." },
    spotify:     { include_in_build: false, key: "..." },  // User must provide
    tmdb:        { include_in_build: true,  key: "..." },
    // ...
  }
}
```

- **`include_in_build: true`** — Key is safe to bundle (ToS compliant)
- **`include_in_build: false`** — Developer-only; users must provide their own
- Users can always override with their own keys in settings

---

## 🚀 CI/CD & Packaging

### GitHub Actions Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `python-app.yml` | Push/PR to main | CI test matrix (3 OS × Python 3.14) |
| `test-suite.yml` | Push/PR to main | Unit tests + import validation |
| `build-artifacts.yml` | Git tag (`v*`) | Nuitka native build & release packages |

### Release Packaging Matrix (Nuitka Native Builds)

All release builds are **compiled via Nuitka** with an embedded Python 3.14 runtime. The end user does **not** need Python installed.

| Platform | Architecture | Format | Filename Pattern |
|----------|-------------|--------|-----------------|
| 🪟 Windows | x64 | `.msi` / `.zip` | `MeedyaManager-windows-x64-vX.X.msi` |
| 🪟 Windows | ARM64 | `.msi` / `.zip` | `MeedyaManager-windows-arm64-vX.X.msi` |
| 🍎 macOS | Apple Silicon | `.dmg` / `.tar.gz` | `MeedyaManager-macos-arm64-vX.X.dmg` |
| 🐧 Linux | x86_64 | `.AppImage` / `.deb` / `.tar.gz` | `MeedyaManager-linux-x64-vX.X.AppImage` |
| 🐧 Linux | ARM64 | `.AppImage` / `.deb` / `.tar.gz` | `MeedyaManager-linux-arm64-vX.X.AppImage` |

Each release includes:
- ✅ SHA256 checksum file (`.sha256`)
- ✅ Auto-generated release notes from CHANGELOG
- ✅ Platform-specific installation instructions
- ✅ Standalone native executable (no Python or dependencies required on host)

---

## 📚 Documentation Strategy

### Documentation Locations

| Location | Audience | Content |
|----------|----------|---------|
| `README.md` | Everyone | Project overview, quick start |
| `Project_Plan.md` | Developers | This file — full project plan |
| `PROJECT_STATUS.md` | Everyone | Current status & progress |
| `docs/CHANGELOG.md` | Everyone | Detailed change log with dates |
| `docs/ROADMAP.md` | Everyone | Milestone timeline |
| `help/` | End users | Usage docs, troubleshooting, FAQs |
| `.claude/` | AI/Developers | Project brief, Claude context |

### Help Documentation (`help/`)

| File | Content |
|------|---------|
| `getting-started.md` | Installation, first run, quick setup |
| `configuration.md` | Settings reference, watch folders, extensions |
| `rule-syntax.md` | Complete template syntax guide with examples |
| `supported-formats.md` | Full list of supported audio/video formats |
| `troubleshooting.md` | Common issues, error codes, solutions |
| `faq.md` | Frequently asked questions |

### Embedded Help

- **CLI**: `--help` flag on every command with detailed descriptions
- **GUI**: Context-sensitive help tooltips + Help menu linking to docs
- **macOS**: Help Book integration (native Help Viewer)
- **Windows**: CHM or bundled HTML help

---

## ⚖️ Licensing & Copyright

### Licence

The project licence is aligned with the GAMDL source project and component libraries to avoid conflicts. Given the use of `mutagen` (GPL-2.0+), the project uses:

> **GPL-2.0-or-later** — GNU General Public License v2.0 or later

This ensures compatibility with all dependencies.

### Copyright Notice (Automated)

All source files include an automated copyright header:

```python
# ============================================================================
# File: /path/to/file.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# [File description here]
# ============================================================================
```

- **Start year**: 2025 (project inception)
- **End year**: Automatically set to the current year at build time
- **Holder**: MWBM Partners Ltd (d/b/a MW Services)

---

## 📊 Project Metrics (Current)

| Metric | Value |
|--------|-------|
| **Current Milestone** | M6 ✅ Complete |
| **Next Milestone** | M7 — Cloud Storage Monitoring |
| **Source Files** | ~150 |
| **Test Files** | 66 |
| **Test Count** | 1007 |
| **Latest Version** | `v1.5-M6` |
| **CI Platforms** | 3 (Windows, macOS, Linux) |
| **Python Version** | 3.14 |

---

> 📝 *This document is maintained alongside the codebase and updated with each milestone.*
>
> *Last updated: 2026-02-13*
