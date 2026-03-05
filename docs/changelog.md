# 📦 CHANGELOG — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

All notable changes to this project are documented here. This changelog follows [Keep a Changelog](https://keepachangelog.com/) conventions.

Format: `## [Version] — YYYY-MM-DD`

---

## [v2.0.0-alpha.4] — 2026-03-05 — CLI (M3)

> **Milestone 3** — Full `clap`-based CLI (`meedya` binary) with 8 commands, shared output infrastructure, dual output modes (Human/JSON), and 45 new tests.

### Added

- **Output infrastructure** (`output.rs`) — `OutputFormat` enum (Human/Json), `ExitCode` constants (0/1/2), `print_table()` (colored column-aligned), `print_key_value()`, `print_json()` (pretty-printed), `print_success/warning/error()` (colored status), `print_header()` (section separator), `print_progress()` (carriage-return overwrite). 4 tests.
- **CLI context** (`context.rs`) — `CliContext` struct holding loaded `AppConfig`, `OutputFormat`, verbosity level, dry-run flag. `CliContext::build()` loads config from custom path or platform default with fallback to defaults. 3 tests.
- **Main entry point** (`main.rs`) — Restructured with `Commands` enum (8 commands + Export stub), global flags (`--verbose`, `--config`, `--json`, `--dry-run`), `tokio::main` async runtime, tracing subscriber initialization.
- **`meedya debug`** (`commands/debug.rs`) — Single-file metadata inspector showing classification, all tags, audio properties, cover art info, companion files. `--raw` flag for lofty tag names. `--cover <path>` extracts embedded cover art. Human (colored tables) and JSON output. 5 tests.
- **`meedya rule`** (`commands/rule.rs`) — 4 subcommands:
  - `validate <template>` — Parse template into AST, show validity and AST dump
  - `tags` — List all 40+ known tag names with their types (Metadata/Virtual/Custom)
  - `test <template> <file>` — Evaluate template against a real media file's metadata
  - `legacy <template>` — Detect legacy MusicBee `{key}` syntax with migration hints
  - 6 tests.
- **`meedya config`** (`commands/config_cmd.rs`) — 5 subcommands:
  - `show` — Display loaded configuration as pretty-printed JSON
  - `path` — Print platform-default config file path
  - `init [path]` — Write default config to config directory or specified path
  - `export <output>` — Bundle config as `.mmprofile` JSON wrapper
  - `import <profile>` — Load `.mmprofile` and write to config directory
  - 5 tests.
- **`meedya scan`** (`commands/scan.rs`) — Directory scan with media classification summary and optional rename preview. `--recursive` (default true), `--template` override, `--output-dir`, `--execute` (perform renames), `--dry-run` safety guard. Classification summary table, rename preview with conflict detection, execute mode. 7 tests.
- **`meedya edit`** (`commands/edit.rs`) — Metadata tag editor. `--set key=value` (repeatable), `--remove key` (repeatable), `--cover <image>` (embed cover art), `--remove-cover`, `--dry-run` preview. Parses key=value format, reports per-action success/error. 6 tests.
- **`meedya watch`** (`commands/watch.rs`) — Foreground file watcher with color-coded event logging. `--no-recursive`, `--organize` (auto-rename on events). Uses `tokio::task::spawn_blocking` to bridge `std::sync::mpsc` → async. Graceful shutdown via `Ctrl+C`. 4 tests.
- **`meedya lookup`** (`commands/lookup.rs`) — Stub command for M5 metadata providers. Lists 19 planned providers across 4 categories. 2 tests.
- **`meedya report-bug`** (`commands/report_bug.rs`) — System info collector: OS, arch, version, config path, watch folders, health check results. `--include-logs` (last 200 log lines), `--output <path>` (save report to file). Markdown (human) or JSON output. 3 tests.

### Changed

- **`mm-core` config structs** (`config/mod.rs`) — Added `Serialize` derive to `AppConfig`, `WatchConfig`, `RenameConfig`, `LoggingConfig`, `ProviderConfig` for JSON serialization from CLI.
- **Tag registry** (`rule_engine/tag_registry.rs`) — Added `pub fn all_tags()` returning sorted registry entries for the `rule tags` command.
- **`mm-cli` Cargo.toml** — Added `serde`, `dirs`, `chrono` runtime deps; `tempfile` dev-dep.

---

## [v2.0.0-alpha.3] — 2026-03-05 — Rule Engine (M2)

> **Milestone 2** — MusicBee-inspired template language with lexer, recursive descent parser, evaluator, 24 template functions, 40+ tag mappings, and declarative rule/condition system. 182 new tests (181 unit + 1 doc-test).

### Added

- **Lexer** (`rule_engine/lexer.rs`) — Character-by-character tokenizer recognising `<Tag>`, `$Func()`, `"quoted literals"`, `(`, `)`, `,`, bare text, and legacy `{key}` passthrough. 26 tests.
- **Parser** (`rule_engine/parser.rs`) — Hand-written recursive descent parser producing `Node::Literal`, `Node::Tag`, `Node::FuncCall`, `Node::Sequence` AST. 50-level nesting depth guard. Legacy syntax detection via regex with `tracing::warn!()`. 24 tests.
- **Tag registry** (`rule_engine/tag_registry.rs`) — 40+ bidirectional mappings: 19 standard audio tags (from `TAG_*` constants), 12 extended tags (sort fields, conductor, mood, etc.), 15 virtual/computed tags (Filename, Duration, Bitrate, MediaClass, etc.), Custom1–Custom16, MeedyaMeta.* namespace. Case-insensitive lookup via `OnceLock<HashMap>`. 24 tests.
- **Template functions** (`rule_engine/functions.rs`) — 24 functions in 5 categories:
  - Logical (6): `$If`, `$And`, `$Or`, `$Not`, `$IsNull`, `$Contains`
  - String (8): `$Replace`, `$Upper`, `$Lower`, `$Left`, `$Right`, `$Mid`, `$Trim`, `$Split`
  - Numeric (4): `$Pad`, `$Date`, `$Format`, `$Count`
  - Lookup (3): `$Sort`, `$IsMatch` (cached regex), `$Lookup` (genre_folder, quality_folder tables)
  - Extensions (3): `$MediaClass`, `$MediaGroup`, `$FirstValue`
  - All functions receive pre-evaluated `&[String]` args. 47 tests.
- **Evaluator** (`rule_engine/evaluator.rs`) — `EvalContext<'a>` with builder pattern, borrowing `TagMap`, `AudioProperties`, `MediaClassification`, and file path. Multi-value tag handling (path mode = first value, display mode = joined). `MissingTagMode` enum (Empty/Literal/Error). Convenience `evaluate_template()`. 30 tests + 1 doc-test.
- **Rule system** (`rule_engine/mod.rs`) — `Rule`, `Condition`, `ConditionOp` (9 operators: Equals, NotEquals, Contains, StartsWith, EndsWith, Matches, IsEmpty, IsNotEmpty), `ConditionMode` (All/Any). `evaluate_rule()` and `apply_rules()` with priority ordering. Serde support for JSON5 config. 30 tests.
- **Renamer integration** (`renamer/mod.rs`) — `simulate_rename_with_rules()` accepting `&[Rule]` and a context builder closure, with fallback to default template.
- **Config extension** (`config/mod.rs`) — Added `rules: Vec<Rule>` and `missing_tag_mode: String` fields to `RenameConfig` with `#[serde(default)]`.

---

## [v2.0.0-alpha.2] — 2026-03-05 — Core Engine (M1)

> **Milestone 1** — Full implementation of `mm-core` crate with 217 tests (214 unit + 3 doc-tests).

### Added

- **Config module** — JSON5 config loading (`settings.json5`), `.env` fallback, `MM_*` environment variable overrides, nested config sections (watch, rename, logging, providers). 22 tests.
- **Media classification** — 4-level hierarchy: MediaGroup (6), MediaFormat (100+), MediaClass (12), MediaQuality (9). Extension-based and path-based classification, quality detection. 38 tests.
- **Metadata extraction/writing** — Tag reading/writing via `lofty` (ID3v2, Vorbis, MP4, APE). Cover art embed/remove, multi-value field support, 19 canonical tag keys. 36 tests.
- **File watcher** — `notify` crate v7 with debouncing, extension filtering, ignore patterns (hidden files, temp files, system files). Initial directory scan. 15 tests.
- **Rename simulator** — Template-based rename preview, conflict detection, filename sanitization (Windows-compatible), custom character replacements. 16 tests.
- **Companion file detector** — 9 companion types (subtitles, lyrics, cue sheets, cover art, disc images, NFO, playlists, chapters, booklets). Grouping by stem. 16 tests.
- **State manager** — JSON state persistence (scan times, counters), atomic writes, single-instance lock file with PID validation (Unix `kill(0)` + stale lock cleanup). 13 tests.
- **Structured logging** — `tracing` + `tracing-subscriber` with console + JSON file layers, PII redaction (path masking, username redaction, SHA-256 hashing). 13 tests.
- **Health checks** — Startup verification (config file, watch folders, config dir writable, disk space). Consolidated health report with pass/warn/fail status. 14 tests.
- **Error types** — `thiserror` with 13 variants, `From` conversions for `std::io::Error`, `serde_json::Error`, `notify::Error`, `lofty::LoftyError`. 5 tests.

### Changed

- Added `resolver = "3"` to workspace `Cargo.toml` (required by edition 2024)
- Added workspace dependencies: `dirs`, `chrono`, `sha2`, `uuid`, `libc`

---

## [v2.0.0-alpha.1] — 2026-03-04 — Rust Rewrite (M0: Repository Setup)

> 🏷️ **Milestone 0** — Complete architecture change from Python to Rust core with platform-native UIs. The Python v1.x codebase (M1–M6) is archived at tag `v1.5-M6-python-final`.

### 🔄 Architecture Change

- **Language:** Python 3.14 → **Rust** (core engine, CLI, metadata, rules, cloud, export, service)
- **GUI:** PySide6/Qt6 → **Platform-native UIs** (SwiftUI on macOS, WinUI 3 on Windows, GTK4 on Linux)
- **FFI:** UniFFI (Swift bindings) and cbindgen (C header generation for C#/GTK)
- **Build:** Nuitka → **Cargo** (Rust workspace) + Xcode + MSBuild

### 🗃️ Archived

- Python v1.x source tree archived at tag `v1.5-M6-python-final`
- Removed: `core/`, `cli/`, `ui/`, `metadata/`, `utils/`, `tests/`, `config/`, `build/`, `scripts/`
- Preserved: 6 completed milestones, 1007 tests, 19 metadata providers in git history

### 🚀 Added

- **Cargo workspace** with 8 crates:
  - `mm-core` — File watcher, metadata extraction, classification, rename engine
  - `mm-cli` — `clap`-based command-line interface
  - `mm-ffi` — UniFFI/cbindgen FFI bridge for native UIs
  - `mm-metadata` — Tag reading/writing via `lofty`, metadata lookup providers
  - `mm-rules` — MusicBee-inspired template parser and evaluator
  - `mm-cloud` — Cloud storage monitoring (OneDrive, Google Drive, Dropbox, MEGA, iCloud)
  - `mm-export` — Database export (MySQL, MariaDB, SQL Server, SQLite, PostgreSQL)
  - `mm-service` — Background service / daemon
- **macOS SwiftUI scaffold** (`native/macos/`) — Xcode project consuming Rust core via UniFFI
- **Windows WinUI 3 scaffold** (`native/windows/`) — .csproj consuming Rust core via cbindgen C API
- **Rust toolchain configuration** (`rust-toolchain.toml`) — Stable channel
- **7 CI/CD workflows** (`.github/workflows/`):
  - `rust-ci.yml` — Cargo build + test + clippy (Ubuntu, macOS, Windows)
  - `swiftui-ci.yml` — Xcode build (macOS)
  - `winui-ci.yml` — MSBuild (Windows)
  - `gtk-ci.yml` — GTK4 build (Ubuntu)
  - `release.yml` — Cross-platform release packaging on git tags
  - `lint.yml` — `rustfmt` + `clippy` checks
  - `docs.yml` — `cargo doc` generation
- **GitHub Projects v2 board** — 11 milestones (M0–M10) with issue tracking
- **11 new milestones** (M0–M10) replacing the original 10 Python milestones

### 📝 Changed

- All documentation rewritten for Rust architecture
- `.claude/CLAUDE.md` updated with Rust coding standards and architecture
- `PROJECT_STATUS.md` rewritten with new milestone structure
- `docs/ROADMAP.md` rewritten with v2.0 timeline
- `docs/CHANGELOG.md` updated (this entry)

---

## [v1.5-M6] — 2026-02-13 — Packaging, Error Handling & Config Profiles

> 🏷️ **Milestone 6** — Centralized logging, crash protection, user-friendly error dialogs, configuration export/import, native platform installers via Nuitka, and CI/CD build pipeline.
>
> ⚠️ **This is the final Python release.** The codebase was archived at tag `v1.5-M6-python-final` before the Rust rewrite.

### 🚀 Added

- **Centralized Logging** (`utils/log_config.py`)
  - `setup_logging()` — single setup function replacing all ad-hoc handlers
  - Platform-aware log directories (macOS `~/Library/Logs/`, Windows `%LOCALAPPDATA%/`, Linux `~/.local/state/`)
  - `PIIRedactionFilter` — automatic path redaction from all log records
  - `TimedRotatingFileHandler` (daily) + `RotatingFileHandler` (10 MB safety net)
  - Auto-cleanup of logs older than 30 days

- **Global Exception Handling** (`utils/exception_handler.py`)
  - `install_exception_hooks()` — hooks for `sys.excepthook` and `threading.excepthook`
  - Crash report files written to log directory (`crash_YYYY-MM-DD_HHMMSS.txt`)
  - `SafeWorker` base class in `ui/workers.py` — QThread with safety-net try/except

- **User-Facing Error Dialogs** (`ui/error_dialog.py`, `utils/error_messages.py`)
  - `ErrorDialog(QDialog)` — headline, explanation, suggestion, collapsible technical details
  - Error message catalog mapping exception types to user-friendly messages
  - MRO-based exception resolution with context-aware message selection
  - "Copy to Clipboard" and "Show Details" functionality

- **Error Reporting** (`utils/error_reporter.py`, `cli/commands/report_bug.py`)
  - `prepare_report()` — collects system info, app version, error details
  - `open_email_client()` — opens default email client via `mailto:` URL
  - PII redaction before composing report body
  - CLI: `meedyamanager report-bug [--include-logs] [--no-system-info]`
  - GUI: Help → "Report Bug..." menu action

- **Startup Health Checks** (`utils/health_check.py`)
  - `run_startup_checks()` — validates Python version, config, watch dirs, log dir, disk space
  - `Severity` enum (OK, WARNING, CRITICAL) and `HealthCheckResult` dataclass
  - `format_results_for_cli()` — Rich-formatted terminal output
  - Integrated into GUI startup (`ui/app.py`) and CLI startup

- **Crash Recovery & State Management** (`core/state_manager.py`)
  - `WatcherState` — persists in-progress/deferred/completed files to JSON
  - `AppLockFile` — PID-based single-instance detection and crash recovery
  - Atomic save (write `.tmp`, rename) for crash-safe persistence

- **Configuration Export/Import** (`utils/config_profile.py`)
  - `.mmprofile` ZIP bundle format with manifest, settings, env template
  - Cross-platform path tokenization ({HOME}, {MUSIC}, {VIDEOS}, etc.)
  - Replace and merge import modes with dry-run preview
  - CLI: `meedyamanager config export/import` commands
  - GUI: Settings dialog Export/Import buttons + File menu actions

- **Native Packaging & Installers**
  - `pyproject.toml` — PEP 621 metadata with hatchling build backend
  - Entry scripts: `meedyamanager_gui.py`, `meedyamanager_cli.py` (Nuitka targets)
  - Icon assets generated from SVG: `assets/icon.png`, `icon.ico`, `icon.icns`
  - `build/innosetup.iss` — Windows installer script (Inno Setup)
  - `build/meedyamanager.desktop` — Linux desktop entry file
  - `scripts/generate_icons.sh` — Icon generation from SVG
  - `.github/workflows/build-installers.yml` — 3-platform CI:
    - macOS (ARM64): `.dmg` with drag-to-Applications
    - Windows (x64): `.exe` installer via Inno Setup
    - Linux (x64): `.AppImage` + `.deb` package
  - SHA256 checksums for all release artifacts

### 🔧 Changed

- **CLI version** — Updated to `v1.5-M6`
- **Workers** (`ui/workers.py`) — `ScanWorker`, `TagWriteWorker`, `LookupWorker` now inherit `SafeWorker` base class (run() → safe_run())
- **Watcher** (`core/watcher.py`) — Removed ad-hoc handlers, renamed logger to `MeedyaManager.Watcher`
- **Renamer** (`core/renamer.py`) — Removed ad-hoc handlers, uses centralized logging
- **Config loader** (`utils/config_loader.py`) — Added `reload_config()` and `get_config_path()`
- **Settings dialog** (`ui/settings_dialog.py`) — Added Export/Import section with profile buttons
- **Main window** (`ui/main_window.py`) — Added File → Export/Import Settings, Help → Report Bug
- **App launcher** (`ui/app.py`) — Startup health checks + centralized logging initialization
- `.gitignore` — Added Nuitka cache, AppImage, .deb, .build, .dist entries

### 🧪 Testing

- **1007 tests** all passing (up from 751 in M5)
- 256 new tests across 12 new test files:
  - `test_log_config.py`, `test_exception_handler.py`, `test_error_messages.py`
  - `test_error_dialog.py`, `test_safe_worker.py`, `test_error_reporter.py`
  - `test_state_manager.py`, `test_health_check.py`
  - `test_config_profile.py`, `test_cli_config.py`

---

## [v1.4-M5] — 2026-02-13 — Metadata Lookup

> 🏷️ **Milestone 5** — 19 metadata lookup providers across music, video, podcasts, and identifier registries. Provider framework with auto-discovery, credential management, rate limiting, cover art management, fuzzy match scoring, CLI lookup command, and GUI lookup panel.

### 🚀 Added

- **Provider Framework** (`metadata/providers/`)
  - Plugin architecture with `@register_provider` decorator and auto-discovery
  - Base provider class with standardized search/match/apply interface
  - Provider registry with category-based filtering (music, video, podcast, identifier)

- **4-Tier Credential Management** (`metadata/providers/credentials.py`)
  - Tier 1: `.env` file (environment variables)
  - Tier 2: `settings.json5` (config-based keys)
  - Tier 3: OS keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service)
  - Tier 4: Encrypted bundle (AES-256-GCM via `cryptography`)
  - Secure storage via `keyring` and `pyjwt[crypto]`

- **Token Bucket Rate Limiter** (`metadata/providers/rate_limiter.py`)
  - Per-provider rate limits respecting API quotas
  - Automatic request throttling with burst allowance
  - Configurable tokens per second and bucket capacity

- **Cover Art Management** (`metadata/providers/cover_art.py`)
  - Static cover art: JPEG and PNG download, resize, and embed
  - Animated cover art: MP4 square, portrait, and artist spotlight formats
  - Thumbnail generation for GUI preview

- **Fuzzy Match Scoring** (`metadata/providers/match_scoring.py`)
  - Weighted scoring algorithm: title (35%), artist (30%), album (20%), duration (15%)
  - ISRC exact-match bonus for high-confidence identification
  - Configurable thresholds via `fuzzywuzzy` and `python-Levenshtein`

- **Music Providers (10)**
  - `apple_music.py` — JWT authentication, track/album search, artwork retrieval
  - `spotify.py` — OAuth2 via `spotipy`, track matching, audio features
  - `musicbrainz.py` — Public API via `musicbrainzngs`, release/recording lookup, MBIDs
  - `deezer.py` — Public API via `deezer-python`, track/album search
  - `youtube_music.py` — Cookie-based auth via `ytmusicapi`, video/song matching
  - `amazon_music.py` — Closed beta API, track matching
  - `pandora.py` — Stub implementation for future expansion
  - `tidal.py` — OAuth2.1 via `tidalapi`, HiFi/MQA metadata
  - `shazam.py` — Audio fingerprinting via `shazamio`, recognition and ID storage
  - `iheart.py` — Undocumented API, station/track matching

- **Video Providers (5)**
  - `tmdb.py` — API key auth via `tmdbsimple`, movie/TV show matching, cast, crew
  - `thetvdb.py` — API key auth, TV show/episode matching, season info
  - `imdb.py` — `cinemagoer` library, movie/TV identification, ratings
  - `apple_tv.py` — Public API, TV/movie matching, artwork retrieval
  - `itunes_store.py` — Public API, purchase metadata, artwork

- **Podcast Providers (1)**
  - `apple_podcasts.py` — Public API, podcast/episode search

- **Identifier Providers (3)**
  - `isrc.py` — Federated ISRC lookup across multiple registries
  - `eidr.py` — Paid Entertainment Identifier Registry lookup
  - `iswc.py` — ISWC lookup via MusicBrainz works database

- **CLI: `meedyamanager lookup` command** (`cli/commands/lookup.py`)
  - `meedyamanager lookup <file>` — Look up metadata for a media file
  - `--provider <name>` — Use a specific provider
  - `--category <music|video|podcast|identifier>` — Filter by provider category
  - `--auto` — Auto-select best providers based on media type
  - `--apply` — Write matched metadata back to file
  - `--dry-run` — Preview matched metadata without writing
  - `--json` — Export results as JSON
  - `--batch` — Batch lookup for directories
  - `--providers-list` — List all available providers and their status

- **GUI: Lookup Tab** (`ui/lookup_panel.py`)
  - Provider checkboxes for selecting which services to query
  - Results table with provider, confidence score, and matched fields
  - Detail panel showing full matched metadata
  - Apply button to write selected match to file
  - Batch lookup button for multi-file processing

- **GUI: LookupWorker** (`ui/workers.py`)
  - QThread-based background worker for async provider lookups
  - Progress signals for UI feedback during batch operations
  - Error handling with per-provider failure isolation

### 🔧 Changed

- **CLI version** — Updated to `v1.4-M5`
- **requirements.txt** — Added `httpx`, `tenacity`, `spotipy`, `musicbrainzngs`, `deezer-python`, `tidalapi`, `ytmusicapi`, `shazamio`, `tmdbsimple`, `cinemagoer`, `pyjwt[crypto]`, `cryptography`, `keyring`, `fuzzywuzzy`, `python-Levenshtein`

### 🧪 Testing

- **751 tests** all passing (up from 342 in M4)
- New test files: provider framework tests, individual provider tests (19 providers), credential management tests, rate limiter tests, cover art tests, match scoring tests, CLI lookup tests, GUI lookup panel tests, LookupWorker tests
- 409 new tests across 22 new test files
- Updated: `test_gui_smoke.py` (4 tabs), `test_cli_version.py` (v1.4-M5)

---

## [v1.3-M4] — 2026-02-13 — Metadata Editor

> 🏷️ **Milestone 4** — Full tag reading/writing via mutagen, metadata editor GUI, CLI edit command, cover art management, and batch editing support.

### 🚀 Added

- **Tag Editor Engine** (`metadata/editor.py`)
  - Unified `TagEditor` class normalizing ID3v2, MP4 atoms, and Vorbis Comments to TAG_MAP internal keys
  - Format-specific mappings: `ID3_TAG_MAP`, `MP4_TAG_MAP`, `VORBIS_TAG_MAP` with reverse maps for writing
  - Methods: `read_tags()`, `write_tags()`, `read_cover_art()`, `write_cover_art()`, `remove_cover_art()`, `get_supported_format()`
  - Track/disc number splitting: ID3 "3/12" and MP4 (3, 12) tuples → `track_num` + `total_tracks`
  - Custom tag support: TXXX frames (ID3), freeform atoms (MP4), any Vorbis Comment key
  - Cover art: APIC (MP3), covr atom (MP4), Picture blocks (FLAC), base64 METADATA_BLOCK_PICTURE (OGG)
  - ASF/WMA read-only support
  - Dry-run mode for write preview
  - Custom exceptions: `UnsupportedFormatError`, `TagWriteError`
  - `CoverArt` dataclass for cover art images

- **Multi-Value Field Handling** (`metadata/multi_value.py`)
  - `parse_multi_value()` — Converts strings, lists, None to normalized value lists
  - `format_multi_value()` — Joins values with semicolons for display
  - `is_multi_value_field()` — Identifies fields with multiple values (artist, genre, composer, album_artist)

- **Metadata Extractor Integration** (`core/metadata_extractor.py`)
  - Two-stage pipeline: pymediainfo (technical) + mutagen/TagEditor (embedded tags)
  - All TAG_MAP fields now populated from actual file tags (artist, album, genre, year, etc.)
  - Merge strategy: mutagen preferred for title/description, pymediainfo for technical fields

- **Tag Registry Additions** (`core/tag_registry.py`)
  - `TECHNICAL_TAGS` set — 20 read-only fields (codec, bitrate, classification, etc.)
  - `is_editable_tag()` function — Distinguishes writable vs read-only fields
  - New TAG_MAP entries: ISRC, Lyrics

- **GUI: Metadata Editor Panel** (`ui/metadata_editor.py`)
  - `TagTableModel` — Two-column table model (Tag Name, Value) with editability flags
  - `CoverArtWidget` — Thumbnail display with Replace, Remove, Extract buttons
  - `MetadataEditorPanel` — Full editor with tag table, cover art, Save/Revert/Add Custom Tag
  - Batch editing support — Multi-file selection shows `<Multiple>` for differing values
  - Change tracking with modified values highlighted in blue

- **GUI: MainWindow Updates** (`ui/main_window.py`)
  - "Metadata" tab (3rd tab) with MetadataEditorPanel
  - Edit → "Edit Metadata" menu action (Ctrl+M)
  - Preview panel selection connected to metadata editor
  - About dialog updated to v1.3-M4

- **GUI: Preview Panel Updates** (`ui/preview_panel.py`)
  - `ExtendedSelection` mode for multi-file selection (Ctrl+click, Shift+click)
  - `files_selected` signal emitted on selection change
  - Right-click context menu with "Edit Metadata" and "Copy Path"
  - Double-click loads file in metadata editor

- **GUI: TagWriteWorker** (`ui/workers.py`)
  - QThread-based background worker for batch tag writing
  - Progress, per-file results, and error signals

- **CLI: `meedyamanager edit` command** (`cli/commands/edit.py`)
  - Display all tags in Rich table (default, no options)
  - `--set "Key=Value"` — Set tag values (multiple allowed)
  - `--remove Tag` — Remove tags (multiple allowed)
  - `--cover image.jpg` — Set cover art from image file
  - `--remove-cover` — Remove all cover art
  - `--dry-run` — Preview changes without writing
  - `--json` — Export tags as JSON
  - Accepts display names ("Album Artist"), internal keys ("album_artist"), or custom tags

### 🔧 Changed

- **CLI version** — Updated to `v1.3-M4`
- **requirements.txt** — Added `mutagen>=1.47`

### 🧪 Testing

- **342 tests** all passing (up from 212 in M3)
- New test files: `test_tag_editor.py` (33), `test_multi_value.py` (25), `test_extractor_integration.py` (35), `test_metadata_editor_gui.py` (22), `test_cli_edit.py` (15)
- Updated: `test_gui_smoke.py` (3 tabs), `test_cli_version.py` (v1.3-M4)
- Real media file fixtures in `conftest.py` (`real_mp3_file`, `real_flac_file`)

---

## [v1.2-M3] — 2026-02-12 — Rule Engine & Companion Files

> 🏷️ **Milestone 3** — Full MusicBee-inspired template engine with recursive descent parser, 20 template functions, companion file tracking, and configurable character replacement.

### 🚀 Added

- **Tag Registry** (`core/tag_registry.py`)
  - Bidirectional mapping of 40+ display tag names ↔ internal snake_case keys
  - Unlimited custom tag support via `<Custom:AnyName>` prefix
  - Functions: `resolve_tag()`, `get_internal_key()`, `get_display_name()`, `get_display_tags()`, `is_valid_tag()`

- **Rule Engine** (`core/rule_engine.py`)
  - Three-stage pipeline: Lexer (tokenizer) → Parser (AST) → Evaluator
  - Context-sensitive lexer disambiguates `<`/`>` as tag delimiters vs comparison operators
  - Support for `<$Func()>` angle bracket wrappers (MusicBee convention)
  - 50-level nesting depth guard
  - Template validation without evaluation (`validate()`)
  - 20 template functions:
    - Conditional: `$If`, `$And`, `$Or`
    - Logic: `$IsNull`, `$Contains`, `$IsMatch`
    - String: `$Replace`, `$RxReplace`, `$Left`, `$Right`, `$Upper`, `$Lower`, `$Trim`
    - Splitting: `$Split`, `$RSplit`, `$First`
    - Formatting: `$Pad`, `$Date`, `$Sort`, `$Group`

- **Character Replacer** (`utils/char_replacer.py`)
  - Two-stage sanitization: user-configured per-character replacements, then regex fallback
  - Activates the `filename_replacements` config key from settings.json5
  - Functions: `sanitize_component()`, `sanitize_path()`

- **Companion File Tracker** (`core/companion_tracker.py`)
  - Same-name companion detection: subtitles (.srt, .sub, .ass, .ssa, .vtt, .idx), lyrics (.lrc), cue sheets (.cue), metadata (.nfo), disc images (.iso, .img, .bin)
  - Directory-level companion detection: cover art (cover.jpg, folder.jpg, artwork.jpg, front.jpg, album.jpg + PNG/BMP variants)
  - Destination computation: same-name companions follow media file's new name, cover art follows directory
  - Human-readable companion summary for UI tooltips

- **CLI `--validate` flag** (`cli/commands/rule.py`)
  - Syntax-only template checking without evaluation
  - Available tags table display from tag registry

- **Preview Panel companions column** (`ui/preview_panel.py`)
  - "Companions" column showing count per file
  - Tooltip with companion filenames on hover

### 🔧 Changed

- **Renamer** (`core/renamer.py`) — Integrated rule engine with auto-detection of template syntax; legacy `{placeholder}` syntax still works with deprecation warning
- **Rule Builder** (`ui/rule_builder.py`) — Syntax highlighter now supports `<Tag>` (cyan), `$Function(` (green), and legacy `{placeholder}` (yellow); tag dropdown populated from registry; test button uses RuleEngine
- **Settings Dialog** (`ui/settings_dialog.py`) — Rename template tab updated with `<Tag>` syntax help text and RuleEngine-powered live preview
- **Scan Worker** (`ui/workers.py`) — Companion file detection integrated into scan results
- **Watcher** (`core/watcher.py`) — Logs companion files found during file processing
- **Default template** (`config/settings.json5`) — Updated to `<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>`

### 🧪 Testing

- **212 tests** all passing (up from 73 in M2)
- New test files: `test_rule_engine.py` (77), `test_companion_tracker.py` (26), `test_tag_registry.py` (20), `test_char_replacer.py` (14)
- Updated: `test_cli_rule.py` (9 tests with new syntax), `test_gui_smoke.py`, `test_gui_preview_model.py`

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
| `v2.0.0-alpha.4` | ✅ M3: CLI | 8 commands (scan, debug, edit, rule, watch, lookup, config, report-bug), 45 tests |
| `v2.0.0-alpha.3` | ✅ M2: Rule Engine | Lexer, parser, evaluator, 24 template functions, 182 tests |
| `v2.0.0-alpha.2` | ✅ M1: Core Engine | Config, classify, metadata, watcher, renamer, companion, 217 tests |
| `v2.0.0-alpha.1` | 🔧 M0: Repository Setup | Rust rewrite, Cargo workspace, native scaffolds, CI/CD |
| `v1.5-M6` | ✅ Packaging & Error Handling | Centralized logging, crash protection, config profiles, native installers |
| `v1.4-M5` | ✅ Metadata Lookup | 19 providers (music, video, podcasts, identifiers), framework, CLI, GUI |
| `v1.3-M4` | ✅ Metadata Editor | Tag editing, mutagen integration, GUI panel, CLI edit |
| `v1.2-M3` | ✅ Rule Engine | Full template syntax, companion file tracking |
| `v1.1-M2` | ✅ CLI & UI | Interactive CLI, PySide6 GUI, rule builder |
| `v1.0-M1` | ✅ Core Engine | Watcher, metadata, classification, dry-run rename |

---

> 📝 *This file is updated with every significant change. For current status, see [PROJECT_STATUS.md](../PROJECT_STATUS.md).*
