# 📦 CHANGELOG — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

All notable changes to this project are documented here. This changelog follows [Keep a Changelog](https://keepachangelog.com/) conventions.

Format: `## [Version] — YYYY-MM-DD`

---

## [v1.1-M2] — 2026-02-13 — CLI & UI Frontend

> 🏷️ **Milestone 2** — Click-based CLI framework and PySide6 cross-platform GUI.

### 🚀 Added

- **Click CLI Framework** (`cli/__init__.py`, `cli/commands/`)
  - Migrated from argparse to Click with subcommand architecture
  - `meedyamanager scan` — Batch scan with `--json`, `--out`, `--mkdir`, `--simulate-off`, `--path`
  - `meedyamanager debug <file>` — Single-file metadata inspector with `--json`, `--out`, `--mkdir`
  - `meedyamanager watch` — Real-time folder monitoring with `--mode`, `--simulate/--no-simulate`
  - `meedyamanager rule` — Template testing with `--sample`, `--file`, `--template`
  - `meedyamanager gui` — Launch graphical interface (lazy PySide6 import)
  - `--version` flag shows `MeedyaManager v1.1-M2`
  - Rich-formatted output with tables and panels

- **PySide6 GUI** (`ui/`)
  - `MainWindow` — Tabbed interface (Scan/Preview, Rules), menu bar, toolbar, status bar
  - `PreviewPanel` — Table view with sort/filter, scan button, progress bar, search field
  - `RenamePreviewModel` — Qt model/view for efficient large-file-list display
  - `SettingsDialog` — 5-tab settings: Watch Folders, Extensions, Rename Template, Fallback Metadata, Character Replacements
  - `RuleBuilder` — Template editor with syntax highlighting for `{placeholder}` tokens, tag dropdown, test button
  - `SystemTrayIcon` — Tray icon with context menu (show/hide, scan, watch toggle, quit)
  - `ScanWorker` — QThread-based background scanning with progress signals
  - Drag-and-drop file import support

- **Platform-Native Styling** (`ui/platform_style.py`)
  - macOS: Liquid Glass (NSGlassEffectView) via PyObjC with NSVisualEffectView fallback
  - Windows: Mica/Acrylic backdrop via DWM API (ctypes)
  - Linux: Qt Fusion style for consistent cross-desktop appearance
  - System dark/light mode detection via `darkdetect`

- **Theme Stylesheets** (`ui/themes/`)
  - `dark.qss` — Dark theme with #1e1e1e base, #4fc3f7 accent
  - `light.qss` — Light theme with #ffffff base, #1976d2 accent
  - Full styling for all Qt widgets (tables, buttons, tabs, menus, progress bars, etc.)

- **GUI Tests** (`tests/test_gui_smoke.py`, `tests/test_gui_preview_model.py`)
  - 11 smoke tests: all widgets instantiate without crashing (offscreen mode)
  - 12 model tests: empty state, data insertion, headers, data retrieval, tooltips

- **CLI Tests** (`tests/test_cli_*.py`)
  - 18 new CliRunner-based tests replacing old subprocess tests
  - Tests for scan, debug, rule, and version commands

### 🔧 Fixed

- **Config key mismatches** — Code now uses `watch_paths`, `rename_format`, `fallback_metadata` matching config/settings.json5
- **Circular dependency** — `core/watcher.py` no longer imports from `cli/runner.py`
- **Missing `handle_file()` function** — Added to `core/watcher.py` for full pipeline processing
- **Missing `cli/__init__.py`** — Created as Click group entry point
- **Matroska classification** — Added `"matroska"` to video format list in `classify_media.py`
- **Classification priority** — "movie"/"film" now checked before "episode"/"tv" for media_class
- **`sanitize_filename_component`** — Handles None input (returns "Unknown")
- **Template expansion** — Dynamic `template.format(**sanitized)` supports any metadata key
- **Watcher logging tests** — Migrated from file-based to `caplog` for reliable test assertions
- **`redact()` function** — Handles non-string input with `str()` conversion
- **`CliRunner(mix_stderr=False)`** — Removed deprecated parameter for Click 8.3.1 compatibility

### 🗑️ Removed

- `tests/test_runner_cli.py` — Replaced by `test_cli_scan.py`
- `tests/test_runner_dryrun_json.py` — Replaced by `test_cli_scan.py`
- `tests/test_metadata_debugger.py` — Replaced by `test_cli_debug.py`

### 🧪 Testing

- **73 tests** all passing (up from 17 in M1)
- New test categories: CLI commands (18), GUI smoke (11), GUI model (12)
- All tests use offscreen Qt rendering for CI compatibility

---

## [Unreleased]

### 📝 Changed — 2026-02-12

- Standardised project name from "MetaMancer" to **MeedyaManager** across all documentation
- Created comprehensive [Project_Plan.md](../Project_Plan.md) with full architecture, tech stack, and milestone details
- Created [PROJECT_STATUS.md](../PROJECT_STATUS.md) as the go-to project status tracker
- Rewrote [README.md](../README.md) with branding, badges, quick start guide, and full documentation links
- Updated [ROADMAP.md](ROADMAP.md) to align with revised milestone ordering
- Updated [CHANGELOG.md](CHANGELOG.md) (this file) with proper formatting and conventions
- Created user documentation in `help/` directory:
  - `getting-started.md` — Installation and first run guide
  - `configuration.md` — Settings reference
  - `rule-syntax.md` — Complete template syntax reference
  - `supported-formats.md` — Full format support list
  - `troubleshooting.md` — Common issues and solutions
  - `faq.md` — Frequently asked questions
- Updated `.claude/CLAUDE.md` with consolidated project brief
- Saved full project brief to `.claude/ProjectBrief_Chat.claude`

---

## [v1.0-M1] — 2025-06-XX — Core Engine & Simulation Framework

> 🏷️ **Milestone 1** — Foundation release with file watching, metadata extraction, and dry-run rename simulation.

### 🚀 Added

- **Folder Watcher** (`core/watcher.py`)
  - Real-time file system monitoring via `watchdog` library
  - Automatic fallback to polling mode if `watchdog` is unavailable
  - Threaded event queue with 1.5s stabilisation delay for file copies
  - Retry queue for locked/in-use files

- **Metadata Extraction** (`core/metadata_extractor.py`)
  - Full metadata parsing via `pymediainfo` (MediaInfo library)
  - Returns structured dictionary of all available tags

- **Media Classification** (`core/classify_media.py`)
  - 4-level classification hierarchy:
    - Level 1: `media_group` (Audio, Video, Image, Book)
    - Level 2: `format_class` (MP3, FLAC, MP4, MKV, PDF, etc.)
    - Level 3: `media_class` (Music, Movie, TV Show, Podcast, etc.)
    - Level 4: `quality_type` (Lossy, Lossless)

- **Dry-Run Rename Engine** (`core/renamer.py`)
  - Simulated rename path generation based on template + metadata
  - Filename character sanitisation (cross-platform safe characters)
  - Logged output of FROM → TO paths for review

- **CLI Tools**
  - `cli/runner.py` — Main CLI entry point with flags:
    - `--simulate-off` — Disable rename simulation
    - `--json` — Export metadata as JSON files
    - `--out <dir>` — Specify output directory
    - `--mkdir` — Create output directories if missing
  - `cli/metadata_debugger.py` — Single-file metadata inspector

- **Configuration** (`config/settings.json5`)
  - JSON5 format with comments support
  - Watch paths, valid extensions, rename template, fallback metadata
  - Character replacement mapping for filename sanitisation

- **Environment Support**
  - `.env` file loading via `python-dotenv`
  - `.env.example` template with all API key placeholders
  - Fallback for API keys, region, language, and log level

- **Logging System**
  - PII-safe logging with path redaction (`/Users/Name` → `<user>`)
  - Dual rotation: daily (midnight) + size-based (5 MB)
  - 7-day timed backup retention, 5 size-based backups

- **Checksum Verification** (`utils/verify_checksum.py`)
  - SHA256 hash comparison for downloaded release archives
  - Post-install integrity validation tool

- **Branding**
  - Static SVG logo (`branding/meedyamanager-logo.svg`)
  - Animated SVG logo (`branding/meedyamanager-logo-animated.svg`)
    - Waveform sweep animation
    - Gradient colour cycling (4-second loop)
    - Dark/light mode auto-detection via CSS `prefers-color-scheme`

### 🧪 Testing

- 17 unit tests across 787 lines

### 🏗️ CI/CD

- GitHub Actions CI matrix: Ubuntu, Windows, macOS × Python 3.10, 3.11
- Automated test suite with coverage reporting (Codecov)
- Build pipeline auto-packages ZIP (Windows) and TAR.GZ (macOS, Linux)
- SHA256 checksum generation for all release artifacts
- GitHub Release publishing with attached artifacts
- Test failure log upload as CI artifacts

### 📁 Project Structure

- Modular architecture: `core/`, `cli/`, `utils/`, `config/`, `tests/`
- GitHub Issue templates: bug report, feature request, task, UI feedback
- Comprehensive `.gitignore` for Python, IDEs, secrets, and build artifacts

---

## 📋 Milestone Reference

| Version | Milestone | Description |
|---------|-----------|-------------|
| `v1.0-M1` | ✅ Core Engine | Watcher, metadata, classification, dry-run rename |
| `v1.1-M2` | ✅ CLI & UI | Interactive CLI, PySide6 GUI, rule builder |
| `v1.2-M3` | 🔲 Rule Engine | Full template syntax, companion file tracking |
| `v1.3-M4` | 🔲 Metadata Editor | Tag editing, multi-value support |
| `v1.4-M5` | 🔲 Music Lookup | MusicBrainz, Spotify, Apple Music, Shazam |
| `v1.5-M6` | 🔲 TV/Film Lookup | TMDb, TheTVDB, IMDb, EIDR |
| `v1.6-M7` | 🔲 Cloud Monitoring | OneDrive, Google Drive, Dropbox, MEGA, iCloud |
| `v2.0-M8` | 🔲 Public Release | Packaged installers, auto-updater |
| `v2.1-M9` | 🔲 DB Export | MySQL, MariaDB, SQLite, PostgreSQL, SQL Server |
| `v2.2-M10` | 🔲 Media Server | Secure web interface, access control |

---

> 📝 *This file is updated with every significant change. For current status, see [PROJECT_STATUS.md](../PROJECT_STATUS.md).*
