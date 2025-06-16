# MetaMancer

<p align="center">
  <img src="branding/mediamancer-logo.svg" alt="MetaMancer Logo" width="200" height="200" />
</p>

🎧📁 **MetaMancer** is a smart, cross-platform media manager and auto-organizer for audio, video, images, and books — inspired by the flexibility of MusicBee but built to run on Windows, macOS (Apple Silicon), and Linux (x86/ARM).

(C) 2025 MWBM Partners Ltd (d/b/a MW Services)

---

## 🚀 Features (Milestone Overview)

### ✅ Milestone 1: Core Engine
- Folder watcher using `watchdog` (fallback to polling)
- Real metadata extraction with MediaInfo
- Rule-based auto-renaming (dry-run mode)
- Configurable CLI runner + `settings.json5` fallback
- Optional JSON output and dry-run export folder
- Simulation log output with redacted user path handling
- `--simulate-off` CLI toggle for dry-run logic
- Rotating log support (size + time)
- GitHub Actions CI + unit/integration test scaffolding
- GitHub Actions multi-platform packaging (ZIP/TAR on release tag)

### 🧠 Metadata Hierarchy
MetaMancer classifies media using a multi-level hierarchy:

| Level | Field          | Example                     |
|-------|----------------|-----------------------------|
| 1️⃣    | `media_group`   | Audio, Video, Image, Book     |
| 2️⃣    | `format_class`  | MP3, FLAC, MP4, PDF, JPEG     |
| 3️⃣    | `media_class`   | Music, Movie, TV Show, Podcast, Photo, Booklet |
| 4️⃣    | `quality_type`  | Lossy, Lossless              |

This enables flexible sorting, rule matching, renaming, and metadata editing.

### 🔄 Future Features (Post-M1)
- Interactive UI (Milestone 2)
- Rule editor and rename preview wizard
- Metadata editing with lookup:
  - MusicBrainz, Apple Music, Spotify, Tidal, Amazon Music, Shazam, AcousticBrainz
  - IMDb, TMDb, TheTVDB, Apple TV, iTunes, EIDR
- Support for animated cover art and PDF booklets
- Cloud sync (OneDrive, Dropbox, Google Drive, iCloud, SharePoint)
- Database export (MySQL, MariaDB, SQLite, PostgreSQL, SQL Server)
- Secure web-accessible library (optional download/export)

---

## 🛠️ Developer Setup

```bash
python -m venv .venv
source .venv/bin/activate  # or .venv\Scripts\activate on Windows
pip install -r requirements.txt
```

To run the CLI watcher:
```bash
python cli/runner.py
```

To debug metadata from a file:
```bash
python cli/metadata_debugger.py /path/to/file
```

With JSON output:
```bash
python cli/metadata_debugger.py /path/file.mp3 --json --out ./metadata --mkdir
```

To disable rename simulation:
```bash
python cli/runner.py --simulate-off
```

---

## 🧪 Testing
```bash
pytest tests/
```
Runs all unit and integration tests (file classification, rename logic, CLI args, JSON export, logging).

---

## 🧭 Roadmap & Structure
See [ROADMAP.md](ROADMAP.md) for milestones and deliverables.  
See [CHANGELOG.md](CHANGELOG.md) for release history.

---

## 📂 Project Layout
```
cli/                   # Entry point tools (watcher, debugger, runner)
core/                  # Core business logic (watcher, extractor, rules)
tests/                 # Unit tests for all modules
logs/                  # Output/debug logs
.github/               # Actions workflows + templates
settings.json5         # App configuration
```

---

## 📦 Platform Support
| OS        | Architecture       |
|-----------|--------------------|
| Windows   | x64, ARM           |
| macOS     | Apple Silicon (M1+) |
| Linux     | x64, ARM           |

---

## 🔐 API Key Policy
- Developer-only API keys (if permitted) are excluded from build packages
- Users may configure their own API keys via settings
- Each provider supports an include/exclude toggle for packaged builds

---

## 🧙‍♂️ Logo & Branding
MetaMancer features an animated SVG logo with light/dark mode compatibility.
An icon set for desktop integration will ship with public alpha builds.

---

## 💬 Feedback & Issues
Use the GitHub [issue templates](.github/ISSUE_TEMPLATE/) for:
- 🐞 Bug reports
- 🧠 Metadata classification bugs
- 🎨 UI feedback or enhancements

---

## 🧾 License
(C) 2025 MWBM Partners Ltd (d/b/a MW Services). All rights reserved.
Commercial licensing and distribution policies to follow.