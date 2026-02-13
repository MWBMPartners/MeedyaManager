# 📍 ROADMAP — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

This file outlines all milestone objectives and sequencing for MeedyaManager — a smart, cross-platform media manager focused on intelligent metadata-driven file organization.

---

## 🧠 Metadata Hierarchy

MeedyaManager classifies all media according to a strict, extensible hierarchy:

| Level | Field | Purpose | Example Values |
| ----- | ----- | ----- | ----- |
| 1️⃣ | `media_group` | High-level category | Audio, Video, Image, Book |
| 2️⃣ | `format_class` | Codec/container format | MP3, FLAC, MP4, MKV, PDF, JPEG |
| 3️⃣ | `media_class` | Intent/type of content | Music, Movie, TV Show, Podcast, eBook, Booklet |
| 4️⃣ | `quality_type` | Fidelity class | Lossy, Lossless |

Additional relationships:

- Albums may link to Booklets (PDFs) or Animated Album Art (MP4 Square/Portrait)
- Tracks/Albums can be linked to TV Shows, Movies, or Episodes
- Media collections support MusicBrainz-style hierarchy: Collection -> Album Group -> Album -> Track
- Classification is automatic via `mediainfo`, with manual overrides supported (M4+)

---

## 📈 Milestone Timeline

### ✅ M1 — Core Engine *(Completed June 2025)*

**Release:** `v1.0-M1`

- ✅ Folder watcher (Watchdog + Polling fallback)
- ✅ Retry logic for in-use files
- ✅ Metadata extraction with `mediainfo`
- ✅ Classification into 4-level hierarchy
- ✅ `settings.json5` fallback config
- ✅ CLI: `runner.py`, `metadata_debugger.py`
- ✅ Dry-run simulation of rename paths
- ✅ `--simulate-off`, `--out`, `--mkdir` CLI arguments
- ✅ JSON export of parsed metadata
- ✅ Rotating logs with redaction (PII safe)
- ✅ Simulation logging and validation tests
- ✅ GitHub Actions CI matrix (3 OSes, 2 Python versions)
- ✅ GitHub Releases auto-packaging ZIP/TAR
- ✅ `.env` loader for fallback API keys
- ✅ Post-install SHA256 checker (`verify_checksum.py`)
- ✅ Auto-generated checksum upload to GitHub Releases
- ✅ Unit tests for env loader, rename engine, checksum logic
- ✅ Animated SVG logo with dark/light mode support

---

### ✅ M2 — CLI & UI Frontend *(Completed February 2026)*

**Release:** `v1.1-M2`

- ✅ Migration to `click`-based CLI framework (5 subcommands: scan, debug, watch, rule, gui)
- ✅ Rich-formatted CLI output with tables and panels
- ✅ PySide6 6.10+ (Qt6) cross-platform GUI
- ✅ 🍎 macOS Liquid Glass support via PyObjC → `NSGlassEffectView` bridge
- ✅ 🪟 Windows 11 Mica/Acrylic native styling via DWM API
- ✅ Dark/light theme support (system-aware via `darkdetect` + QSS stylesheets)
- ✅ Rename preview panel with table model, progress bar, search filter
- ✅ Settings dialog (5 tabs: watch folders, extensions, template, fallback, replacements)
- ✅ Rule builder with syntax highlighting for `{placeholder}` tokens
- ✅ Drag-and-drop file import
- ✅ System tray icon with context menu
- ✅ 73 tests (CLI + GUI + core), all passing
- ✅ MusicBee-inspired template syntax parser (completed in M3)
- 🔲 Per-rule dry-run and file override support (deferred to M4+)
- 🔲 Visual rule builder with AND/OR/nested conditions (deferred to M4+)

---

### ✅ M3 — Rule Engine & Companion Files *(Completed February 2026)*

**Release:** `v1.2-M3`

- ✅ Full MusicBee-style template syntax: `<Tag>`, `$If()`, `$And()`, `$Or()`
- ✅ String functions: `$Replace()`, `$RxReplace()`, `$Left()`, `$Right()`, `$Upper()`, `$Lower()`, `$Trim()`
- ✅ Logic functions: `$Contains()`, `$IsMatch()`, `$IsNull()`
- ✅ Splitting: `$Split()`, `$RSplit()`, `$First()`
- ✅ Formatting: `$Pad()`, `$Date()`, `$Sort()`, `$Group()`
- ✅ `$First()` for multi-value field extraction
- ✅ Unlimited custom tag support (`<Custom:AnyName>`)
- ✅ Companion file detection (SRT, LRC, cover art, ISO, CUE, NFO)
- ✅ Companion file destination computation (same-name + directory-level)
- ✅ Advanced filename character replacement (configurable via settings.json5)
- ✅ Deeply nested condition support (50-level depth guard)
- ✅ Legacy `{placeholder}` backward compatibility with auto-detection
- ✅ 212 tests (139 new), all passing

---

### ✅ M4 — Metadata Editor *(Completed February 2026)*

**Release:** `v1.3-M4`

- ✅ Full tag reading/writing via `mutagen` (TagEditor class)
- ✅ Supported formats: ID3v2 (MP3, AIFF), MP4/M4A atoms, FLAC Vorbis Comments, OGG Vorbis/Opus, ASF (read-only)
- ✅ Multi-value tag support (artists, genres, composers — semicolon-delimited)
- ✅ Custom tag creation and editing (TXXX, freeform atoms, Vorbis Comment keys)
- ✅ Batch tag editing across multiple files (GUI + CLI)
- ✅ Tag preview before applying changes (dry-run mode)
- ✅ Cover art management (embed, extract, replace, remove)
- ✅ GUI metadata editor panel with tag table, cover art widget, save/revert
- ✅ CLI `meedyamanager edit` command with --set, --remove, --cover, --dry-run, --json
- ✅ Metadata extractor enriched with mutagen tags (two-stage pipeline)
- ✅ 342 tests (130 new), all passing
- 🔲 Booklet (PDF) and animated album art attachment (deferred to M5+)
- 🔲 MKV/MKA tag writing (mutagen limitation — deferred to future sub-milestone)

---

### ✅ M5 — Metadata Lookup *(Completed February 2026)*

**Release:** `v1.4-M5`

> **Note:** M5 was expanded to include video, podcast, and identifier providers ahead of schedule (originally planned for M6). All 19 providers were implemented in this milestone.

**Provider Framework:**

- ✅ Auto-discovery with `@register_provider` decorator
- ✅ 4-tier credential management: .env → settings.json5 → OS keyring → encrypted bundle
- ✅ Token bucket rate limiting per provider
- ✅ Cover art management: static (JPEG/PNG) + animated (MP4 square, portrait, artist spotlight)
- ✅ Fuzzy match scoring: title (35%), artist (30%), album (20%), ISRC bonus

**Music Providers (10):**

- ✅ **Apple Music** — JWT authentication, track matching, artwork
- ✅ **Spotify** — OAuth2 via spotipy, track matching, audio features
- ✅ **MusicBrainz** — Public API via musicbrainzngs, tags, release info, MBIDs
- ✅ **Deezer** — Public API via deezer-python, track matching
- ✅ **YouTube Music** — Cookie-based auth via ytmusicapi
- ✅ **Amazon Music** — Closed beta API
- ✅ **Pandora** — Stub implementation
- ✅ **Tidal** — OAuth2.1 via tidalapi, HiFi metadata
- ✅ **Shazam** — Audio fingerprinting via shazamio
- ✅ **iHeart** — Undocumented API

**Video Providers (5):**

- ✅ **TMDB** — API key auth via tmdbsimple, movie/TV matching
- ✅ **TheTVDB** — API key auth, TV show/episode matching
- ✅ **IMDb** — cinemagoer library, movie/TV identification
- ✅ **Apple TV** — Public API, TV/movie matching, artwork
- ✅ **iTunes Store** — Public API, purchase metadata, artwork

**Podcast Providers (1):**

- ✅ **Apple Podcasts** — Public API

**Identifier Providers (3):**

- ✅ **ISRC** — Federated lookup
- ✅ **EIDR** — Paid Entertainment Identifier Registry
- ✅ **ISWC** — MusicBrainz-backed

**CLI & GUI:**

- ✅ CLI: `meedyamanager lookup` with --provider, --category, --auto, --apply, --dry-run, --json, --batch, --providers-list
- ✅ GUI: Lookup tab with provider checkboxes, results table, detail panel, apply/batch buttons
- ✅ LookupWorker QThread for background async lookups
- ✅ 751 tests (409 new), all passing

---

### 🎬 M6 — Metadata Lookup: TV & Film (Enhancements)

**Release:** `v1.5-M6`

> **Note:** The core video providers (TMDB, TheTVDB, IMDb, Apple TV, iTunes Store, EIDR) were implemented ahead of schedule in M5. This milestone will focus on enhancements and deeper integration.

Providers already completed in M5:

- ✅ **Apple TV** — TV/movie matching, artwork
- ✅ **iTunes Store** — Purchase metadata, artwork
- ✅ **TheTVDB** — TV show/episode matching
- ✅ **TheMovieDB (TMDb)** — Movie matching, cast, crew
- ✅ **IMDb** — Movie/TV identification, ratings
- ✅ **EIDR** — Entertainment Identifier Registry lookup & embed

Remaining enhancements:

- 🔲 Enhanced season/episode matching with filename parsing
- 🔲 Batch TV series lookup with automatic episode assignment
- 🔲 Download animated cover art (square and portrait) as MP4

---

### ☁️ M7 — Cloud Storage Monitoring

**Release:** `v1.6-M7`

Cloud providers (OAuth/token authentication):

- 🔲 OneDrive (Personal)
- 🔲 OneDrive for Business / SharePoint
- 🔲 Google Drive
- 🔲 Dropbox
- 🔲 MEGA.nz
- 🔲 iCloud Drive

Features:

- 🔲 Background sync worker
- 🔲 Conflict resolution
- 🔲 Selective sync filtering

---

### 📦 M8 — Public Release

**Release:** `v2.0-M8`

- 🔲 Nuitka compilation to native standalone binaries (Python 3.14 runtime bundled & sandboxed)
- 🔲 GitHub Actions auto-create packages:
  - Windows x64 + ARM64 (MSI/ZIP)
  - macOS Apple Silicon (DMG/TAR.GZ)
  - Linux x86_64 + ARM64 (AppImage/DEB/TAR.GZ)
- 🔲 PySide6 6.10+ GUI with native platform styling (Cocoa, Win11, Fusion)
- 🔲 Zero-dependency install — users need NO pre-installed software
- 🔲 Auto-updater design
- 🔲 First public alpha release
- 🔲 Feedback capture mechanism

---

### 🗄️ M9 — Media Library Database Export

**Release:** `v2.1-M9` (sub-releases per database)

Export library metadata to external databases:

- 🔲 M9.1 — MySQL
- 🔲 M9.2 — MariaDB
- 🔲 M9.3 — SQL Server
- 🔲 M9.4 — SQLite
- 🔲 M9.5 — PostgreSQL

Purpose: Create searchable intranet/web-hosted media library index.

---

### 🌐 M10 — Secure Media Server

**Release:** `v2.2-M10`

- 🔲 Export/copy media files to web server
- 🔲 Reference links stored in external database
- 🔲 Access-controlled downloads with user authentication
- 🔲 Multi-format support (FLAC, ALAC, M4A, MP3)
- 🔲 Web interface for browsing/searching exported library
- 🔲 Security hardening (piracy prevention, access controls)

---

## 🛠️ Planned Utilities

- `metadata_debugger.py` — Single file debug/export (✅ M1)
- Rule tester & simulator (CLI + UI) — ✅ M2 (`meedyamanager rule`, Rule Builder GUI)
- Format recogniser tool — ✅ M2 (`meedyamanager debug`)
- Auto-log redactor for troubleshooting — ✅ M1
- Smart CLI auto-detection/scan mode — ✅ M2 (`meedyamanager scan`)
- `verify_checksum.py` post-install validator — ✅ M1

---

## 💻 Platform Support

| OS | Architectures |
| ----- | ----- |
| 🪟 Windows | x64, ARM64 |
| 🍎 macOS | Apple Silicon (arm64) only |
| 🐧 Linux | x86_64, ARM64 |

---

## 📋 Notes

- All builds support dark/light UIs and animated SVG assets
- GitHub Actions produces ZIP/TAR artifacts per milestone tag
- All 3rd-party API keys must be developer-only unless ToS allows redistribution
- Users may override keys in `settings.json5`, `.env`, or via UI
- Continuous test and documentation updates follow every milestone
- Documentation (.md files) updated automatically with each change

---

> 📝 *This roadmap is maintained alongside the codebase. For current status, see [PROJECT_STATUS.md](../PROJECT_STATUS.md).*
>
> *Last updated: 2026-02-15*
