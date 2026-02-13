# MeedyaManager

<p align="center">
  <img src="branding/meedyamanager-logo-animated.svg" alt="MeedyaManager Logo" width="480" height="160" />
</p>

<p align="center">
  <strong>🎧📁 Smart, cross-platform media manager and auto-organizer</strong>
  <br />
  <em>Inspired by MusicBee's flexibility — built for Windows, macOS & Linux</em>
</p>

<p align="center">
  <img src="https://github.com/MWBMPartners/MeedyaManager/actions/workflows/python-app.yml/badge.svg" alt="CI Tests" />
  <img src="https://img.shields.io/badge/python-3.14+-blue.svg" alt="Python 3.14+" />
  <img src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-green.svg" alt="Platforms" />
  <img src="https://img.shields.io/badge/license-GPL--2.0+-orange.svg" alt="License" />
</p>

---

**(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

---

## 🌟 What is MeedyaManager?

**MeedyaManager** is a cross-platform media file management application that **automatically monitors folders**, reads metadata from audio and video files, and **renames/organizes them** according to user-defined rules.

Think of it as **MusicBee's auto-organize feature** — but available everywhere, supporting both audio and video, and running silently in the background.

### ✨ Key Features

| Feature | Description |
|---------|-------------|
| 👁️ **Real-Time Monitoring** | Watches folders for new media files and processes them automatically |
| 🧠 **Smart Classification** | 4-level hierarchy: Media Group → Format → Class → Quality |
| 📐 **Flexible Rule Engine** | MusicBee-inspired templates with `$If`, `$And`, `$Or`, regex, and nesting |
| 🎵 **Wide Format Support** | MP3, FLAC, ALAC, M4A, MP4, MKV, AVI, OGG, AC3, EAC3, HEVC + more |
| 🔊 **Audio Analysis** | Lossy/Lossless, Dolby Digital/Plus, Spatial Audio, channel count |
| 🔄 **Companion Files** | Moves subtitles, cover art, and disc images alongside media |
| 🛡️ **File-Lock Safety** | Won't touch files in use — queues them for later processing |
| ⚡ **Lightweight** | Minimal resource usage; runs as a background service |
| 🌙 **Service Mode** | Auto-start on boot or login (Windows Service, macOS launchd, Linux systemd) |
| 🎨 **Dark/Light UI** | System-aware theme switching |

---

## 🚀 Quick Start

### For End Users (Release Packages)

> ⚡ **No Python installation required!** MeedyaManager ships as a standalone native executable with its own bundled, sandboxed Python 3.14 runtime (compiled via Nuitka). It will **never** interfere with any other Python on your system.

1. Download the latest release for your platform from [GitHub Releases](https://github.com/MWBMPartners/MeedyaManager/releases)
2. Install/extract and run — that's it!

### For Developers (From Source)

**Prerequisites:**

- **Python 3.14+** — [python.org/downloads](https://www.python.org/downloads/)
- **MediaInfo** — bundled automatically via `pymediainfo` pip wheel (no separate install needed on most platforms). If pymediainfo cannot find the library at runtime, install it manually:
  - macOS: `brew install mediainfo`
  - Linux: `sudo apt install mediainfo` or `sudo dnf install mediainfo`
  - Windows: typically bundled in the pymediainfo wheel; if not, download from [MediaInfo website](https://mediaarea.net/en/MediaInfo)

**Setup:**

```bash
# Clone the repository
git clone https://github.com/MWBMPartners/MeedyaManager.git
cd MeedyaManager

# Create a virtual environment (recommended)
python -m venv venv
source venv/bin/activate  # macOS/Linux
# venv\Scripts\activate   # Windows

# Install dependencies
pip install -r requirements.txt

# Copy environment template
cp .env.example .env
```

### Usage (CLI)

```bash
# Scan watch folders and preview renames
meedyamanager scan

# Inspect a single file's metadata
meedyamanager debug path/to/song.mp3

# Test a rename template with sample data
meedyamanager rule --sample --template "<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>"

# Validate template syntax
meedyamanager rule --validate --template "<Artist>/<Title>.<Ext>"

# View metadata tags for a file
meedyamanager edit path/to/song.mp3

# Edit metadata tags
meedyamanager edit path/to/song.mp3 --set "Artist=New Artist" --set "Genre=Rock"

# Look up metadata for a file (auto-selects best providers)
meedyamanager lookup path/to/song.mp3 --auto

# Look up using a specific provider
meedyamanager lookup path/to/song.mp3 --provider spotify

# Look up and apply matched metadata to file
meedyamanager lookup path/to/song.mp3 --auto --apply

# Preview lookup results without writing (dry-run)
meedyamanager lookup path/to/song.mp3 --auto --dry-run

# Batch lookup all files in a directory
meedyamanager lookup path/to/music/ --batch --auto

# Export lookup results as JSON
meedyamanager lookup path/to/song.mp3 --auto --json

# List all available providers and their status
meedyamanager lookup --providers-list

# Look up only from a specific category
meedyamanager lookup path/to/movie.mkv --category video --auto

# Start the folder watcher (simulation mode — safe, no files moved)
meedyamanager watch

# Launch the GUI
meedyamanager gui
```

### Configuration

Edit `config/settings.json5` to set your preferences:

```json5
{
  // Folders to watch for new media files
  watch_paths: ["~/Downloads/Media", "~/Desktop/NewMedia"],

  // Supported file extensions
  valid_extensions: ["mp3", "flac", "m4a", "mp4", "mkv", "avi", "wav", "ogg"],

  // Rename template using <Tag> and $Function() syntax
  rename_format: "<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>",

  // Characters to replace in filenames
  filename_replacements: { "/": "-", "\\": "-", ":": "-", "*": "", "?": "" }
}
```

---

## 🧠 Metadata Classification

MeedyaManager classifies all media into a 4-level hierarchy:

| Level | Field | Examples |
|-------|-------|---------|
| 1️⃣ | `media_group` | Audio, Video, Image, Book |
| 2️⃣ | `format_class` | MP3, FLAC, MP4, MKV, PDF |
| 3️⃣ | `media_class` | Music, Movie, TV Show, Podcast, Music Video |
| 4️⃣ | `quality_type` | Lossy, Lossless |

This powers intelligent file routing, rule matching, and future UI grouping.

---

## 📐 Rule Engine

MeedyaManager's rule engine is inspired by [MusicBee's template system](https://musicbee.fandom.com/wiki/Templates) with extensions for unlimited custom tags, video support, and 20 built-in functions.

**Example rules:**
```
# Basic music organisation
Music/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>

# Lossless vs Lossy sorting
$If(<Quality Type>=Lossless,
    Music/Lossless/<Album Artist>/<Album>/<Title>.<Ext>,
    Music/Lossy/<Album Artist>/<Album>/<Title>.<Ext>
)

# TV Show organisation
TV Shows/<Show>/Season <$Pad(<Season>,2)>/<Show> - S<$Pad(<Season>,2)>E<$Pad(<Episode>,2)>.<Ext>
```

📖 Full syntax reference: [help/rule-syntax.md](help/rule-syntax.md)

---

## 🔍 Metadata Lookup

MeedyaManager can automatically look up and match your media files against **19 online providers** across music, video, podcasts, and identifier registries. The provider framework features auto-discovery, credential management, rate limiting, cover art retrieval, and fuzzy match scoring.

### Providers

| Category | Providers |
|----------|-----------|
| 🎵 **Music** (10) | Apple Music (JWT), Spotify (OAuth2), MusicBrainz (public), Deezer (public), YouTube Music (cookies), Amazon Music (closed beta), Pandora (stub), Tidal (OAuth2.1), Shazam (fingerprinting), iHeart (undocumented) |
| 🎬 **Video** (5) | TMDB (API key), TheTVDB (API key), IMDb (cinemagoer), Apple TV (public), iTunes Store (public) |
| 🎙️ **Podcasts** (1) | Apple Podcasts (public) |
| 🆔 **Identifiers** (3) | ISRC (federated), EIDR (paid), ISWC (MusicBrainz) |

### Key Capabilities

- **Auto-discovery** — Providers register via `@register_provider` decorator and are loaded automatically
- **4-tier credential management** — `.env` → `settings.json5` → OS keyring → encrypted bundle
- **Rate limiting** — Token bucket algorithm per provider to respect API quotas
- **Cover art** — Static (JPEG/PNG) and animated (MP4 square, portrait, artist spotlight)
- **Fuzzy matching** — Weighted scoring: title (35%), artist (30%), album (20%), ISRC bonus
- **CLI & GUI** — Full `meedyamanager lookup` command and GUI "Lookup" tab with batch support

📖 Provider setup guide: [help/provider-setup.md](help/provider-setup.md)

---

## 🗺️ Milestone Roadmap

| # | Milestone | Status | Description |
|---|-----------|--------|-------------|
| M1 | 🧱 Core Engine | ✅ **Complete** | Watcher, metadata, classification, dry-run rename |
| M2 | 🧙 CLI & UI Frontend | ✅ **Complete** | Interactive CLI, PySide6 GUI, rule builder |
| M3 | 🧩 Rule Engine & Companions | ✅ **Complete** | 20 template functions, companion tracking, 212 tests |
| M4 | ✏️ Metadata Editor | ✅ **Complete** | Tag read/write via mutagen, GUI editor, CLI edit, 342 tests |
| M5 | 🔍 Metadata Lookup | ✅ **Complete** | 19 providers across music, video, podcasts & identifiers |
| M6 | 🎬 TV/Film Metadata Lookup | 🔲 Planned | Video providers (TMDB, TheTVDB, IMDb) partially done in M5 |
| M7 | ☁️ Cloud Monitoring | 🔲 Planned | OneDrive, Google Drive, Dropbox, MEGA, iCloud |
| M8 | 📦 Public Release | 🔲 Planned | Packaged installers, auto-updater |
| M9 | 🗄️ Database Export | 🔲 Planned | MySQL, MariaDB, SQLite, PostgreSQL, SQL Server |
| M10 | 🌐 Secure Media Server | 🔲 Planned | Web interface, access control, multi-format export |

📋 Full details: [Project_Plan.md](Project_Plan.md) | 📊 Status: [PROJECT_STATUS.md](PROJECT_STATUS.md) | 📍 Roadmap: [docs/ROADMAP.md](docs/ROADMAP.md)

---

## 💻 Platform Support

| Platform | Architectures | Service Support |
|----------|---------------|-----------------|
| 🪟 **Windows** | x64, ARM64 | Windows Service |
| 🍎 **macOS** | Apple Silicon only | LaunchDaemon / LaunchAgent |
| 🐧 **Linux** | x86_64, ARM64 | systemd |

---

## 🧪 Development

### Running Tests

```bash
# Run all tests
pytest tests/

# Run with coverage report
pytest tests/ --cov=core --cov=utils --cov=cli --cov-report=term-missing
```

### Environment Variables

Copy `.env.example` to `.env` and fill in your API keys:

```bash
cp .env.example .env
```

See [help/configuration.md](help/configuration.md) for all available settings.

### Post-Install Integrity Check

After downloading a release archive:

```bash
python utils/verify_checksum.py dist/MeedyaManager-macos-arm64.tar.gz dist/MeedyaManager-macos-arm64.tar.gz.sha256
```

---

## 📦 Release Builds

- ✅ GitHub Actions auto-builds packages per platform on tag (`v1.0-M1`, `v1.1-M2`, ...)
- ✅ Includes `.sha256` checksum files for integrity verification
- ✅ Packages contain `core/`, `cli/`, `config/`, `branding/`, `README.md`

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| 📋 [Project_Plan.md](Project_Plan.md) | Complete project plan with architecture & tech stack |
| 📊 [PROJECT_STATUS.md](PROJECT_STATUS.md) | Current progress tracker |
| 📍 [docs/ROADMAP.md](docs/ROADMAP.md) | Milestone timeline |
| 📦 [docs/CHANGELOG.md](docs/CHANGELOG.md) | Detailed change log |
| 📖 [help/getting-started.md](help/getting-started.md) | Getting started guide |
| ⚙️ [help/configuration.md](help/configuration.md) | Configuration reference |
| 📐 [help/rule-syntax.md](help/rule-syntax.md) | Rule template syntax guide |
| 🎵 [help/supported-formats.md](help/supported-formats.md) | Supported file formats |
| 🔍 [help/provider-setup.md](help/provider-setup.md) | Metadata lookup provider setup guides |
| 🔧 [help/troubleshooting.md](help/troubleshooting.md) | Troubleshooting guide |
| ❓ [help/faq.md](help/faq.md) | Frequently asked questions |

---

## ⚖️ License

This project is licensed under the **GPL-2.0-or-later** — see the [LICENSE](LICENSE) file for details.

Compatible with all project dependencies including `mutagen` (GPL-2.0+) and `PySide6` (LGPL-3.0).

---

## 🤝 Contributing

Contributions are welcome! Please see [help/getting-started.md](help/getting-started.md) for development setup instructions.

---

**(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**
