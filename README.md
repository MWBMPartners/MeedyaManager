# MediaMancer

<p align="center">
  <img src="branding/mediamancer-logo-animated.svg" alt="MediaMancer Logo" width="480" height="160" />
</p>

<p align="center">
  <strong>🎧📁 Smart, cross-platform media manager and auto-organizer</strong>
  <br />
  <em>Inspired by MusicBee's flexibility — built for Windows, macOS & Linux</em>
</p>

<p align="center">
  <img src="https://github.com/MWBMPartners/MediaMancer/actions/workflows/python-app.yml/badge.svg" alt="CI Tests" />
  <img src="https://img.shields.io/badge/python-3.11+-blue.svg" alt="Python 3.11+" />
  <img src="https://img.shields.io/badge/platforms-Windows%20%7C%20macOS%20%7C%20Linux-green.svg" alt="Platforms" />
  <img src="https://img.shields.io/badge/license-GPL--2.0+-orange.svg" alt="License" />
</p>

---

**(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

---

## 🌟 What is MediaMancer?

**MediaMancer** is a cross-platform media file management application that **automatically monitors folders**, reads metadata from audio and video files, and **renames/organizes them** according to user-defined rules.

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

### Prerequisites

- **Python 3.11+** (3.10 also supported)
- **MediaInfo** library installed on your system
  - macOS: `brew install mediainfo`
  - Linux: `sudo apt install mediainfo` or `sudo dnf install mediainfo`
  - Windows: Download from [MediaInfo website](https://mediaarea.net/en/MediaInfo)

### Installation

```bash
# Clone the repository
git clone https://github.com/MWBMPartners/MediaMancer.git
cd MediaMancer

# Install dependencies
pip install -r requirements.txt

# Copy environment template
cp .env.example .env
```

### Usage

```bash
# Scan a single file and view its metadata
python cli/metadata_debugger.py path/to/song.mp3

# Scan and export metadata as JSON
python cli/metadata_debugger.py path/to/song.mp3 --json --out output/

# Run the folder watcher (simulation mode — safe, no files moved)
python cli/runner.py

# Run with simulation disabled (actually rename/move files)
python cli/runner.py --simulate-off
```

### Configuration

Edit `config/settings.json5` to set your preferences:

```json5
{
  // Folders to watch for new media files
  watch_paths: ["~/Downloads/Media", "~/Desktop/NewMedia"],

  // Supported file extensions
  valid_extensions: ["mp3", "flac", "m4a", "mp4", "mkv", "avi", "wav", "ogg"],

  // Rename template (uses metadata placeholders)
  rename_format: "{media_class}/{artist}/{album}/{track_num} - {title}.{extension}",

  // Characters to replace in filenames
  filename_replacements: { "/": "-", "\\": "-", ":": "-", "*": "", "?": "" }
}
```

---

## 🧠 Metadata Classification

MediaMancer classifies all media into a 4-level hierarchy:

| Level | Field | Examples |
|-------|-------|---------|
| 1️⃣ | `media_group` | Audio, Video, Image, Book |
| 2️⃣ | `format_class` | MP3, FLAC, MP4, MKV, PDF |
| 3️⃣ | `media_class` | Music, Movie, TV Show, Podcast, Music Video |
| 4️⃣ | `quality_type` | Lossy, Lossless |

This powers intelligent file routing, rule matching, and future UI grouping.

---

## 📐 Rule Engine (Coming in M2/M3)

MediaMancer's rule engine is inspired by [MusicBee's template system](https://musicbee.fandom.com/wiki/Templates) with extensions for unlimited custom tags and video support.

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

## 🗺️ Milestone Roadmap

| # | Milestone | Status | Description |
|---|-----------|--------|-------------|
| M1 | 🧱 Core Engine | ✅ **Complete** | Watcher, metadata, classification, dry-run rename |
| M2 | 🧙 CLI & UI Frontend | 🔨 **Next** | Interactive CLI, PySide6 GUI, rule builder |
| M3 | 🧩 Rule Engine & Companions | 🔲 Planned | Full template syntax, companion file tracking |
| M4 | ✏️ Metadata Editor | 🔲 Planned | Manual tag editing, multi-value support |
| M5 | 🎵 Music Metadata Lookup | 🔲 Planned | MusicBrainz, Spotify, Apple Music, Shazam + more |
| M6 | 🎬 TV/Film Metadata Lookup | 🔲 Planned | TMDb, TheTVDB, IMDb, EIDR + more |
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
python utils/verify_checksum.py dist/MediaMancer-macos-arm64.tar.gz dist/MediaMancer-macos-arm64.tar.gz.sha256
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
