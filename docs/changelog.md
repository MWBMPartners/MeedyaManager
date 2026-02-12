# 📦 CHANGELOG — MediaMancer

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

All notable changes to this project are documented here. This changelog follows [Keep a Changelog](https://keepachangelog.com/) conventions.

Format: `## [Version] — YYYY-MM-DD`

---

## [Unreleased]

### 📝 Changed — 2026-02-12

- Standardised project name from "MetaMancer" to **MediaMancer** across all documentation
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
  - Static SVG logo (`branding/mediamancer-logo.svg`)
  - Animated SVG logo (`branding/mediamancer-logo-animated.svg`)
    - Waveform sweep animation
    - Gradient colour cycling (4-second loop)
    - Dark/light mode auto-detection via CSS `prefers-color-scheme`

### 🧪 Testing

- 17 unit tests across 787 lines:
  - `test_metadata_extractor.py` — Format and classification logic
  - `test_classify_media_sanity.py` — Classification edge cases
  - `test_simulate_flag_behaviour.py` — Simulation toggle
  - `test_watcher_simulation_trigger.py` — Watcher event simulation
  - `test_simulation_log_output.py` — Log content and redaction
  - `test_batch_rename_simulation.py` — Multi-file integration
  - `test_env_loader.py` — `.env` fallback loader validation
  - `test_verify_checksum.py` — SHA256 verifier edge cases
  - `test_config_required_key.py` — Config key validation
  - `test_runner_cli.py` — CLI argument parsing
  - `test_runner_dryrun_json.py` — JSON export functionality
  - `test_watcher_logging.py` — Watcher log output
  - `test_watcher_modes.py` — Watchdog vs polling modes
  - `test_path_integrity.py` — Path construction safety
  - `test_import_resolution.py` — Module import validation
  - `test_metadata_debugger.py` — Debug tool functionality

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
| `v1.1-M2` | 🔨 CLI & UI | Interactive CLI, PySide6 GUI, rule builder |
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
