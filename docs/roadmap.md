# 📍 ROADMAP.md

(C) 2025 MWBM Partners Ltd (d/b/a MW Services)

This file outlines all milestone objectives and sequencing for the MetaMancer project — a smart, cross-platform media manager focused on intelligent metadata-driven file organization.

---

## 🧱 Metadata Hierarchy Reference
MetaMancer classifies all media according to a strict, extensible hierarchy:

| Level         | Field           | Purpose                          | Example Values                         |
|---------------|------------------|----------------------------------|----------------------------------------|
| 1️⃣           | `media_group`   | High-level category              | Audio, Video, Image, Book              |
| 2️⃣           | `format_class`  | Codec/container format           | MP3, FLAC, MP4, Matroska, PDF, JPEG    |
| 3️⃣           | `media_class`   | Intent/type of content           | Music, Movie, TV Show, Podcast, eBook, Booklet |
| 4️⃣           | `quality_type`  | Fidelity class                   | Lossy, Lossless                        |

Additional relationships:
- Albums may link to Booklets (PDFs) or Animated Album Art (MP4 Square/Portrait)
- Tracks/Albums can be linked to TV Shows, Movies, or Episodes
- Media collections support MusicBrainz-style hierarchy:
  - Collection → Album Group → Album → Track
- Classification is automatic via `mediainfo`, with manual overrides supported (M4+)

---

## 📈 Milestone Timeline

### ✅ M1 – Core Engine (✅ Completed June 2025)
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
- ✅ GitHub Actions CI (Pytest + Artifacts)
- ✅ GitHub Releases auto-packaging ZIP/TAR

### 🧙 M2 – CLI & UI Frontend
- Interactive CLI rename preview wizard
- Rule builder with conditional logic (AND/OR/NESTED)
- Cross-platform UI (light/dark support)
- Rename preview queue and simulation panel
- Per-rule dry-run and file override support

### 🧩 M3 – Rule Engine Enhancements
- Advanced variable substitution logic
- Filename sanitization system (customizable)
- Extension filtering and fallback patterns
- Complex nesting (IF/AND/OR + tag fallback)

### 🧠 M4 – Metadata Editor
- Full manual metadata editor UI
- Multi-value tag support
- Tag writing (ID3, MP4, MKV/Matroska)
- Attach Booklets + Animated Art to Albums
- Preview metadata before applying

### 🔍 M5 – Metadata Lookup (Music)
- Integrations:
  - ✅ MusicBrainz
  - ✅ Apple Music
  - ✅ Spotify
  - ✅ Tidal
  - ✅ Amazon Music
  - ✅ Shazam (w/ fingerprint storage)
  - ✅ AcousticBrainz
- Store direct URLs & IDs in custom tags

### 🎬 M6 – Metadata Lookup (TV/Film)
- Lookups:
  - ✅ IMDb, TMDb, TheTVDB
  - ✅ Apple TV / iTunes
  - ✅ EIDR
- Link albums/songs to TV/Movie metadata context

### ☁️ M7 – Cloud Monitor (Optional)
- Monitor + sync folders:
  - ✅ OneDrive, Google Drive, SharePoint
  - ✅ Dropbox, MEGA, iCloud
- Auth via secure tokens / OAuth
- Background sync worker

### 🌐 M8 – Public Release Track
- Alpha releases post-M4
- Per-milestone updates with packaged builds
- GitHub Releases for Windows/macOS/Linux
- Feedback capture + auto-updater design

### 🗄 M9 – Media Library Export
- Export to external DBs:
  - ✅ MySQL
  - ✅ MariaDB
  - ✅ SQLite
  - ✅ PostgreSQL
  - ✅ SQL Server

### 🔐 M10 – Secure Media Server (Optional)
- Copy/export media to secure server with reference
- Multi-format support (MP3, FLAC, ALAC, M4A)
- Access-controlled downloads
- Web interface to browse/search exported media

---

## 🛠 Planned Utilities
- `metadata_debugger.py`: single file debug/export
- Rule tester & simulator (CLI + UI)
- Format recognizer tool
- Auto-log redactor for troubleshooting
- Smart CLI auto-detection/scan mode

---

## 📦 Platform Support
| OS        | Architectures       |
|-----------|---------------------|
| Windows   | x64, ARM            |
| macOS     | Apple Silicon only  |
| Linux     | x64, ARM            |

---

## 🧾 Notes
- All builds support dark/light UIs and animated SVG assets
- GitHub Actions produces ZIP/TAR artifacts per milestone tag
- All 3rd-party API keys must be developer-only unless ToS allows redistribution
- Users may override keys in `settings.json5` or via UI
- Continuous test and documentation updates follow every milestone