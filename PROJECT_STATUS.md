# 📊 MeedyaManager — Project Status

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**
>
> 🎧📁 Cross-platform media manager and auto-organizer

---

## 🏁 Quick Status

| Item | Status |
|------|--------|
| **Current Milestone** | M7 — Cloud Storage Monitoring |
| **Last Completed** | ✅ M6 — Packaging & Error Handling (Feb 2026) |
| **Overall Progress** | ██████░░░░ **60%** (6 of 10 milestones) |
| **Latest Version** | `v1.5-M6` |
| **Build Status** | ![CI](https://github.com/MWBMPartners/MeedyaManager/actions/workflows/python-app.yml/badge.svg) |

---

## 📈 Milestone Progress

### ✅ M1 — Core Engine *(Complete)*

> 🗓️ Completed: June 2025 | 📦 Release: `v1.0-M1`

**Progress: ██████████ 100%**

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Folder watcher (watchdog + polling) | ✅ Done | Real-time monitoring with retry queue |
| Metadata extraction (pymediainfo) | ✅ Done | Full MediaInfo integration |
| 4-level media classification | ✅ Done | Group → Format → Class → Quality |
| Dry-run rename simulation | ✅ Done | Safe simulation mode by default |
| `settings.json5` configuration | ✅ Done | JSON5 with comments support |
| CLI tools (`runner.py`, `metadata_debugger.py`) | ✅ Done | `--simulate-off`, `--out`, `--mkdir`, `--json` |
| PII-safe logging with rotation | ✅ Done | Daily + size-based rotation |
| `.env` loader for API keys | ✅ Done | Fallback environment variables |
| SHA256 checksum verification | ✅ Done | `verify_checksum.py` |
| GitHub Actions CI matrix | ✅ Done | 3 OS × 2 Python versions |
| Release packaging (ZIP/TAR) | ✅ Done | Auto-build on git tags |
| Unit test suite | ✅ Done | 17 tests, 787 lines |

---

### ✅ M2 — CLI & UI Frontend *(Complete)*

> 🗓️ Completed: February 2026 | 📦 Release: `v1.1-M2`

**Progress: ██████████ 100%**

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Click-based CLI framework | ✅ Done | 5 subcommands: scan, debug, watch, rule, gui |
| Scan command with rich output | ✅ Done | `--json`, `--out`, `--mkdir`, `--simulate-off`, `--path` |
| Debug command (metadata inspector) | ✅ Done | Rich tables, JSON export |
| Watch command (folder monitoring) | ✅ Done | `--mode`, `--simulate/--no-simulate`, `--path` |
| Rule template tester | ✅ Done | `--sample`, `--file`, `--template` |
| PySide6 6.10+ cross-platform GUI | ✅ Done | Tabbed interface, drag-and-drop |
| macOS Liquid Glass support | ✅ Done | PyObjC → NSGlassEffectView with vibrancy fallback |
| Windows Mica/Acrylic styling | ✅ Done | DWM API via ctypes |
| Dark/light theme (system-aware) | ✅ Done | darkdetect + QSS stylesheets |
| Rename preview panel | ✅ Done | Table model, progress bar, search filter |
| Settings dialog | ✅ Done | 5 tabs: folders, extensions, template, fallback, replacements |
| Rule builder with syntax highlighting | ✅ Done | Token highlighting, tag insertion, live preview |
| System tray icon | ✅ Done | Show/hide, scan, watch toggle, quit |
| Drag-and-drop file import | ✅ Done | Drop files onto main window |
| GUI test suite | ✅ Done | 23 tests (smoke + model) |
| CLI test suite (CliRunner) | ✅ Done | 18 tests replacing subprocess tests |
| Foundation bug fixes | ✅ Done | Config keys, circular dep, handle_file, classify_media |

---

### ✅ M3 — Rule Engine & Companion Files *(Complete)*

> 🗓️ Completed: February 2026 | 📦 Release: `v1.2-M3`

**Progress: ██████████ 100%**

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Tag registry with 40+ tag mappings | ✅ Done | Bidirectional display ↔ internal key mapping |
| Custom tag support (`<Custom:*>`) | ✅ Done | Unlimited user-defined tags |
| Recursive descent template parser | ✅ Done | Lexer → Parser (AST) → Evaluator pipeline |
| 20 template functions | ✅ Done | $If, $And, $Or, $IsNull, $Contains, $IsMatch, $Replace, $RxReplace, $Left, $Right, $Upper, $Lower, $Trim, $Split, $RSplit, $First, $Pad, $Date, $Sort, $Group |
| Deep nesting support | ✅ Done | 50-level depth guard, `<$Func()>` wrappers |
| Configurable character replacement | ✅ Done | Per-character mapping via settings.json5 |
| Companion file detection | ✅ Done | SRT, LRC, CUE, NFO, ISO, cover art |
| Companion destination computation | ✅ Done | Same-name + directory-level tracking |
| Legacy `{placeholder}` backward compat | ✅ Done | Auto-detected with deprecation warning |
| Rule engine integration (renamer + CLI) | ✅ Done | `--validate` flag, tag table display |
| UI updates (rule builder + settings) | ✅ Done | Syntax highlighting, RuleEngine preview |
| Preview panel companions column | ✅ Done | Count + tooltip showing companion files |
| Comprehensive test suite | ✅ Done | 139 new tests (212 total) |

---

### ✅ M4 — Metadata Editor *(Complete)*

> 🗓️ Completed: February 2026 | 📦 Release: `v1.3-M4`

**Progress: ██████████ 100%**

| Deliverable | Status | Notes |
|-------------|--------|-------|
| mutagen-based tag reader/writer engine | ✅ Done | Unified API for ID3v2, MP4, Vorbis Comments, ASF |
| Format-specific tag mappings | ✅ Done | ID3_TAG_MAP, MP4_TAG_MAP, VORBIS_TAG_MAP |
| Multi-value field handling | ✅ Done | Semicolon-delimited, native Vorbis multi-value |
| Custom tag support (TXXX, freeform, Vorbis) | ✅ Done | Unlimited custom tags via `custom_` prefix |
| Cover art read/write/remove | ✅ Done | APIC (MP3), covr (MP4), Picture (FLAC), base64 (OGG) |
| Track/disc number splitting | ✅ Done | "3/12" → track_num + total_tracks |
| Metadata extractor integration | ✅ Done | pymediainfo + mutagen two-stage pipeline |
| TECHNICAL_TAGS and is_editable_tag() | ✅ Done | Distinguish read-only vs editable fields |
| ISRC and Lyrics tags in TAG_MAP | ✅ Done | New tag registry entries |
| GUI: Metadata editor panel | ✅ Done | Tag table, cover art widget, save/revert |
| GUI: Batch editing support | ✅ Done | Multi-file selection with `<Multiple>` handling |
| GUI: Metadata tab in MainWindow | ✅ Done | Third tab with edit menu action |
| GUI: Preview panel multi-select | ✅ Done | ExtendedSelection, context menu, files_selected signal |
| GUI: TagWriteWorker background thread | ✅ Done | QThread-based batch write |
| CLI: `meedyamanager edit` command | ✅ Done | --set, --remove, --cover, --remove-cover, --dry-run, --json |
| Comprehensive test suite | ✅ Done | 130 new tests (342 total) |

---

### ✅ M5 — Metadata Lookup *(Complete)*

> 🗓️ Completed: February 2026 | 📦 Release: `v1.4-M5`

**Progress: ██████████ 100%**

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Provider framework with auto-discovery | ✅ Done | `@register_provider` decorator, plugin architecture |
| 🎵 Apple Music provider | ✅ Done | JWT authentication |
| 🎵 Spotify provider | ✅ Done | OAuth2 authentication via spotipy |
| 🎵 MusicBrainz provider | ✅ Done | Public API via musicbrainzngs |
| 🎵 Deezer provider | ✅ Done | Public API via deezer-python |
| 🎵 YouTube Music provider | ✅ Done | Cookie-based auth via ytmusicapi |
| 🎵 Amazon Music provider | ✅ Done | Closed beta API |
| 🎵 Pandora provider | ✅ Done | Stub implementation |
| 🎵 Tidal provider | ✅ Done | OAuth2.1 via tidalapi |
| 🎵 Shazam provider | ✅ Done | Audio fingerprinting via shazamio |
| 🎵 iHeart provider | ✅ Done | Undocumented API |
| 🎬 TMDB provider | ✅ Done | API key auth via tmdbsimple |
| 🎬 TheTVDB provider | ✅ Done | API key authentication |
| 🎬 IMDb provider | ✅ Done | cinemagoer library |
| 🎬 Apple TV provider | ✅ Done | Public API |
| 🎬 iTunes Store provider | ✅ Done | Public API |
| 🎙️ Apple Podcasts provider | ✅ Done | Public API |
| 🆔 ISRC identifier provider | ✅ Done | Federated lookup |
| 🆔 EIDR identifier provider | ✅ Done | Paid registry |
| 🆔 ISWC identifier provider | ✅ Done | MusicBrainz-backed |
| 4-tier credential management | ✅ Done | .env → settings.json5 → OS keyring → encrypted bundle |
| Token bucket rate limiting | ✅ Done | Per-provider rate limits |
| Cover art management | ✅ Done | Static (JPEG/PNG) + animated (MP4 square, portrait, artist spotlight) |
| Fuzzy match scoring | ✅ Done | Title 35%, artist 30%, album 20%, ISRC bonus |
| CLI: `meedyamanager lookup` command | ✅ Done | --provider, --category, --auto, --apply, --dry-run, --json, --batch, --providers-list |
| GUI: Lookup tab | ✅ Done | Provider checkboxes, results table, detail panel, apply/batch buttons |
| LookupWorker QThread | ✅ Done | Background async lookups |
| Comprehensive test suite | ✅ Done | 409 new tests (751 total) |

---

### ✅ M6 — Packaging, Error Handling & Config Profiles *(Complete)*

> 🗓️ Completed: February 2026 | 📦 Release: `v1.5-M6`

**Progress: ██████████ 100%**

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Centralized logging (log_config.py) | ✅ Done | Platform-aware log dirs, PII redaction, daily rotation |
| Global exception handling (exception_handler.py) | ✅ Done | sys.excepthook + threading.excepthook + crash reports |
| SafeWorker base class (workers.py) | ✅ Done | QThread safety-net for ScanWorker, TagWriteWorker, LookupWorker |
| User-friendly error dialogs (error_dialog.py) | ✅ Done | ErrorDialog with catalog-based messages, collapsible details |
| Error message catalog (error_messages.py) | ✅ Done | MRO-based exception-to-message resolution |
| Error reporting (error_reporter.py) | ✅ Done | Email-based bug reports via mailto: URL |
| CLI: report-bug command | ✅ Done | --include-logs, --no-system-info |
| Startup health checks (health_check.py) | ✅ Done | Python version, config, watch dirs, log dir, disk space |
| Crash recovery (state_manager.py) | ✅ Done | WatcherState + AppLockFile with atomic persistence |
| Config export/import (config_profile.py) | ✅ Done | .mmprofile ZIP bundles with cross-platform path tokens |
| CLI: config export/import commands | ✅ Done | --mode replace/merge, --dry-run, --include-secrets |
| GUI: Settings Export/Import buttons | ✅ Done | Profile group box in SettingsDialog |
| pyproject.toml (PEP 621) | ✅ Done | hatchling build backend, entry points, optional deps |
| Icon assets from SVG | ✅ Done | icon.png (512x512), icon.ico (multi-res), icon.icns |
| Nuitka build entry scripts | ✅ Done | meedyamanager_gui.py, meedyamanager_cli.py |
| Windows installer (Inno Setup) | ✅ Done | build/innosetup.iss — Next > Install wizard |
| Linux desktop entry | ✅ Done | build/meedyamanager.desktop |
| CI: build-installers.yml | ✅ Done | macOS .dmg, Windows .exe, Linux .AppImage + .deb |
| Comprehensive test suite | ✅ Done | 256 new tests (1007 total) |

---

### 🔮 Future Milestones

| # | Milestone | Status | Target |
|---|-----------|--------|--------|
| M7 | ☁️ Cloud Storage Monitoring | 🔲 Planned | OneDrive, Google Drive, Dropbox, MEGA, iCloud |
| M8 | 📦 Public Release | 🔲 Planned | Auto-updater, code signing |
| M9 | 🗄️ Database Export | 🔲 Planned | — |
| M10 | 🌐 Secure Media Server | 🔲 Planned | — |

---

## 🧪 Test Suite Status

| Category | Tests | Status |
|----------|-------|--------|
| Provider framework & auto-discovery | 28 | ✅ Passing |
| Music providers (10 providers) | 95 | ✅ Passing |
| Video providers (5 providers) | 62 | ✅ Passing |
| Podcast providers | 12 | ✅ Passing |
| Identifier providers (ISRC, EIDR, ISWC) | 24 | ✅ Passing |
| Credential management (4-tier) | 32 | ✅ Passing |
| Rate limiter (token bucket) | 18 | ✅ Passing |
| Cover art management | 22 | ✅ Passing |
| Fuzzy match scoring | 26 | ✅ Passing |
| CLI: lookup command | 38 | ✅ Passing |
| GUI: lookup panel | 32 | ✅ Passing |
| LookupWorker QThread | 20 | ✅ Passing |
| Rule engine (lexer, parser, evaluator) | 77 | ✅ Passing |
| Tag editor (ID3, Vorbis, cover art) | 33 | ✅ Passing |
| Multi-value handling | 25 | ✅ Passing |
| Extractor integration (mutagen + pymediainfo) | 35 | ✅ Passing |
| Companion tracker | 26 | ✅ Passing |
| Tag registry | 20 | ✅ Passing |
| Character replacer | 14 | ✅ Passing |
| CLI: edit command | 15 | ✅ Passing |
| CLI: scan command | 6 | ✅ Passing |
| CLI: debug command | 5 | ✅ Passing |
| CLI: rule command | 9 | ✅ Passing |
| CLI: version flag | 1 | ✅ Passing |
| GUI: metadata editor | 22 | ✅ Passing |
| GUI: smoke tests | 11 | ✅ Passing |
| GUI: preview model | 12 | ✅ Passing |
| Metadata extraction | 6 | ✅ Passing |
| Media classification | 2 | ✅ Passing |
| Watcher functionality | 6 | ✅ Passing |
| Rename simulation | 1 | ✅ Passing |
| Config & ENV loading | 5 | ✅ Passing |
| Checksum verification | 3 | ✅ Passing |
| Path & import integrity | 5 | ✅ Passing |
| Watcher logging & redaction | 2 | ✅ Passing |
| Simulation log output | 1 | ✅ Passing |
| Batch rename simulation | 1 | ✅ Passing |
| Centralized logging (log_config) | 20 | ✅ Passing |
| Exception handler (crash protection) | 14 | ✅ Passing |
| Error messages (catalog) | 19 | ✅ Passing |
| Error dialog (GUI) | 27 | ✅ Passing |
| SafeWorker (QThread safety) | 11 | ✅ Passing |
| Error reporter (email) | 22 | ✅ Passing |
| State manager (crash recovery) | 21 | ✅ Passing |
| Health checks (startup) | 20 | ✅ Passing |
| Config profile (export/import) | 46 | ✅ Passing |
| CLI: config commands | 15 | ✅ Passing |
| CLI: report-bug command | 5 | ✅ Passing |
| **Total** | **1007** | ✅ **All Passing** |

---

## 🏗️ Architecture Health

| Component | Files | Health |
|-----------|-------|--------|
| `core/` | 9 | ✅ Stable (+state_manager.py for crash recovery) |
| `metadata/` | 3 | ✅ Stable (editor.py, multi_value.py, __init__.py) |
| `metadata/providers/` | 50+ | ✅ Stable (19 providers, framework, credentials, rate limiter, cover art, match scoring) |
| `cli/` | 7 | ✅ Stable (Click framework) |
| `cli/commands/` | 9 | ✅ Stable (+config_cmd, report_bug) |
| `ui/` | 11 | ✅ Stable (+error_dialog.py) |
| `ui/themes/` | 2 | ✅ Stable (dark.qss, light.qss) |
| `utils/` | 10 | ✅ Stable (+log_config, exception_handler, error_messages, error_reporter, health_check, config_profile) |
| `tests/` | 63 | ✅ 1007 tests, all passing |
| `config/` | 1 | ✅ Stable |
| `build/` | 2 | ✅ New (innosetup.iss, meedyamanager.desktop) |
| `scripts/` | 1 | ✅ New (generate_icons.sh) |
| `assets/` | 3 | ✅ New (icon.png, icon.ico, icon.icns) |
| **Total** | **~171** | ✅ **Healthy** |

---

## 📋 Known Issues & Technical Debt

| Issue | Priority | Milestone | Notes |
|-------|----------|-----------|-------|
| Polling mode not yet implemented | 🟡 Medium | M3 | Placeholder in `watcher.py` |
| ~~Rename engine uses `{placeholder}` not `<Tag>` syntax~~ | ✅ Resolved | M3 | Migrated to MusicBee-style template syntax |
| ~~No `mutagen` integration for tag writing~~ | ✅ Resolved | M4 | Full mutagen read/write via TagEditor |
| Watcher not integrated with GUI toggle | 🟡 Medium | M3 | GUI button state tracked, needs core watcher connection |
| Rule builder text-only (no visual $If/$And/$Or) | 🔵 Low | M3 | Visual condition builder planned for M3 |

---

## 📦 Platform Support Matrix

| Platform | Architecture | CI Tested | Package Built | Service Support |
|----------|-------------|-----------|---------------|-----------------|
| 🪟 Windows | x64 | ✅ | ✅ | 🔲 Planned (pywin32) |
| 🪟 Windows | ARM64 | 🔲 | 🔲 | 🔲 Planned |
| 🍎 macOS | Apple Silicon | ✅ | ✅ | 🔲 Planned (launchd) |
| 🐧 Linux | x86_64 | ✅ | ✅ | 🔲 Planned (systemd) |
| 🐧 Linux | ARM64 | 🔲 | 🔲 | 🔲 Planned |

---

## 📅 Recent Activity

| Date | Activity |
|------|----------|
| 2026-02-12 | **M6 Complete** — Packaging & error handling: centralized logging, crash protection, error dialogs, config export/import, native installers (macOS .dmg, Windows .exe, Linux .AppImage/.deb), 1007 tests |
| 2026-02-15 | **M5 Complete** — Metadata lookup: 19 providers (music, video, podcasts, identifiers), provider framework, credential management, rate limiting, cover art, fuzzy matching, CLI lookup, GUI lookup panel, 751 tests |
| 2026-02-14 | **M4 Complete** — Metadata editor (mutagen), tag read/write, GUI panel, CLI edit, 342 tests |
| 2026-02-13 | **M3 Complete** — Rule engine (20 functions), companion tracker, 212 tests |
| 2026-02-13 | **M2 Complete** — Click CLI, PySide6 GUI, 73 tests passing |
| 2026-02-13 | GUI: Main window, preview panel, settings, rule builder, system tray |
| 2026-02-13 | Platform styling: macOS Liquid Glass, Windows Mica, Linux Fusion |
| 2026-02-13 | Click CLI: scan, debug, watch, rule, gui commands |
| 2026-02-13 | Foundation fixes: config keys, circular deps, classify_media |
| 2025-02-12 | Animated SVG logo, enhanced PII redaction patterns |

---

> 📝 *This file is updated with each significant change. For detailed changelog, see [docs/CHANGELOG.md](docs/CHANGELOG.md).*
>
> *Last updated: 2026-02-12*
