# 📍 ROADMAP — MediaMancer

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

This file outlines all milestone objectives and sequencing for MediaMancer — a smart, cross-platform media manager focused on intelligent metadata-driven file organization.

---

## 🧠 Metadata Hierarchy

MediaMancer classifies all media according to a strict, extensible hierarchy:

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

### 🔨 M2 — CLI & UI Frontend *(Next)*

**Release:** `v1.1-M2`

- 🔲 Interactive CLI rename preview wizard
- 🔲 Migration to `click`-based CLI framework
- 🔲 Rule builder with conditional logic (AND/OR/nested)
- 🔲 MusicBee-inspired template syntax parser
- 🔲 PySide6 6.10+ (Qt6) cross-platform GUI
- 🔲 🍎 macOS Liquid Glass support via PyObjC → `NSGlassEffectView` bridge
- 🔲 🪟 Windows 11 Mica/Acrylic native styling
- 🔲 Dark/light theme support (system-aware via `darkdetect` + native APIs)
- 🔲 Rename preview queue and simulation panel
- 🔲 Drag-and-drop file import
- 🔲 Per-rule dry-run and file override support
- 🔲 Settings dialog for watch folders, extensions, rules
- 🔲 Rule validation with real-time error reporting

---

### 🧩 M3 — Rule Engine & Companion Files

**Release:** `v1.2-M3`

- 🔲 Full MusicBee-style template syntax: `<Tag>`, `$If()`, `$And()`, `$Or()`
- 🔲 String functions: `$Replace()`, `$RxReplace()`, `$Left()`, `$Right()`, `$Upper()`, `$Lower()`, `$Trim()`
- 🔲 Logic functions: `$Contains()`, `$IsMatch()`, `$IsNull()`
- 🔲 Splitting: `$Split()`, `$RSplit()`, `$RxSplit()`
- 🔲 Formatting: `$Pad()`, `$Date()`, `$Sort()`, `$Group()`
- 🔲 `$First()` for multi-value field extraction
- 🔲 Unlimited custom tag support (`<Custom:AnyName>`)
- 🔲 Companion file detection (SRT, LRC, cover art, ISO, CUE)
- 🔲 Companion file group movement (move all when media moves)
- 🔲 Advanced filename character replacement (configurable per-rule)
- 🔲 Extension filtering and fallback patterns
- 🔲 Deeply nested condition support

---

### ✏️ M4 — Metadata Editor

**Release:** `v1.3-M4`

- 🔲 Full tag reading/writing via `mutagen`
- 🔲 Supported formats: ID3v2 (MP3), MP4/M4A atoms, FLAC Vorbis Comments, OGG, MKV/MKA
- 🔲 Multi-value tag support (artists, genres, etc.)
- 🔲 Custom tag creation and editing (unlimited)
- 🔲 Batch tag editing across multiple files
- 🔲 Tag preview before applying changes
- 🔲 Booklet (PDF) and animated album art attachment
- 🔲 Cover art management (embed, extract, replace)
- 🔲 Ensure playability preserved across all players/devices
- 🔲 Standard containers used wherever possible for custom tags

---

### 🎵 M5 — Metadata Lookup: Music

**Release:** `v1.4-M5`

Integrations (each stores direct URL + ID in custom tags):

- 🔲 **MusicBrainz** — Tags, release info, MBIDs
- 🔲 **Apple Music** — Track matching, artwork
- 🔲 **Spotify** — Track matching, audio features
- 🔲 **Tidal** — HiFi metadata
- 🔲 **Amazon Music** — Track matching
- 🔲 **Shazam** — Audio fingerprinting, ID/fingerprint string storage
- 🔲 **AcousticBrainz** — Audio analysis data

---

### 🎬 M6 — Metadata Lookup: TV & Film

**Release:** `v1.5-M6`

Integrations (each stores direct URL + ID in custom tags):

- 🔲 **Apple TV** — TV/movie matching, artwork
- 🔲 **iTunes Store** — Purchase metadata, artwork
- 🔲 **TheTVDB** — TV show/episode matching
- 🔲 **TheMovieDB (TMDb)** — Movie matching, cast, crew
- 🔲 **IMDb** — Movie/TV identification, ratings
- 🔲 **EIDR** — Entertainment Identifier Registry lookup & embed

Additional:

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
- Rule tester & simulator (CLI + UI) — M2/M3
- Format recogniser tool — M2
- Auto-log redactor for troubleshooting — ✅ M1
- Smart CLI auto-detection/scan mode — M2
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
> *Last updated: 2026-02-12*
