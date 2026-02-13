# 🚀 Getting Started with MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

Welcome to MeedyaManager! This guide will help you install, configure, and run the application for the first time.

---

## 📋 Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [First Run](#first-run)
4. [Basic Configuration](#basic-configuration)
5. [Metadata Lookup](#-metadata-lookup)
6. [Next Steps](#next-steps)

---

## Prerequisites

### For End Users (Release Packages)

> ⚡ **No prerequisites required!** Release packages include everything you need — a bundled, sandboxed Python 3.14 runtime and all dependencies are compiled into the native executable via Nuitka. MeedyaManager will **never** interfere with any other software on your system.

| Component | Requirement |
| --------- | ----------- |
| **OS** | Windows (x64/ARM), macOS (Apple Silicon), or Linux (x64/ARM) |
| **Disk Space** | ~150 MB |
| **RAM** | Minimal — runs as a lightweight background process |
| **Python** | ❌ NOT required — bundled inside the app |
| **MediaInfo** | ❌ NOT required — bundled inside the app |

### For Developers (Building from Source)

If you want to contribute or build from source, you'll need:

| Component | Requirement |
| --------- | ----------- |
| **Python** | 3.14+ — [python.org/downloads](https://www.python.org/downloads/) |
| **MediaInfo** | Latest version (see below) |
| **OS** | Windows (x64/ARM), macOS (Apple Silicon), or Linux (x64/ARM) |
| **Disk Space** | ~300 MB (source + venv + build tools) |

#### Installing MediaInfo (Developers Only)

MeedyaManager uses the [MediaInfo](https://mediaarea.net/en/MediaInfo) library for metadata extraction.

**macOS (Homebrew):**

```bash
brew install mediainfo
```

**Linux (Debian/Ubuntu):**

```bash
sudo apt install mediainfo libmediainfo-dev
```

**Linux (Fedora/RHEL):**

```bash
sudo dnf install mediainfo libmediainfo-devel
```

**Windows:**

Download the installer from [mediaarea.net](https://mediaarea.net/en/MediaInfo/Download/Windows) and run it. Ensure the MediaInfo DLL is in your system PATH.

---

## Installation

### From Release Package (Recommended)

1. Download the latest release for your platform from [GitHub Releases](https://github.com/MWBMPartners/MeedyaManager/releases)
2. Install or extract:
   - **Windows:** Run the `.msi` installer, or extract the `.zip`
   - **macOS:** Open the `.dmg` and drag to Applications
   - **Linux:** Run the `.AppImage`, or install the `.deb` package
3. Launch MeedyaManager — no additional setup needed!

### From Source (Development)

```bash
# 1. Clone the repository
git clone https://github.com/MWBMPartners/MeedyaManager.git
cd MeedyaManager

# 2. Create a virtual environment (keeps your system Python clean)
python3.14 -m venv venv

# Activate it:
# macOS/Linux:
source venv/bin/activate
# Windows:
venv\Scripts\activate

# 3. Install dependencies
pip install -r requirements.txt

# 4. Copy the environment template
cp .env.example .env
```

### Building a Native Executable (via Nuitka)

To produce a standalone native binary (like the release packages):

```bash
# Install build dependencies
pip install nuitka ordered-set

# Build standalone executable
python -m nuitka --standalone --onefile --enable-plugin=pyside6 \
    --output-dir=dist cli/runner.py
```

The resulting executable includes the Python 3.14 runtime and all dependencies — fully self-contained and sandboxed.

---

## First Run

### Scan a Single File

The quickest way to test MeedyaManager is to inspect a single media file:

```bash
meedyamanager debug path/to/your/song.mp3
```

This will display all detected metadata including:

- Media group (Audio/Video)
- Format class (MP3/FLAC/etc.)
- Media class (Music/Movie/etc.)
- Quality type (Lossy/Lossless)
- All embedded tags (artist, album, title, etc.)

### Export Metadata as JSON

```bash
meedyamanager debug path/to/song.mp3 --json --out output/ --mkdir
```

### Batch Scan Watch Folders

```bash
# Scan all configured watch folders and preview renames
meedyamanager scan
```

### Start the Folder Watcher

```bash
# Safe simulation mode (no files moved — just logs what would happen)
meedyamanager watch

# With simulation disabled (actually renames/moves files)
meedyamanager watch --no-simulate
```

### Launch the GUI

```bash
meedyamanager gui
```

> **Tip:** Always run in simulation mode first to verify your rules produce the expected results before enabling actual file operations.

---

## Basic Configuration

Edit `config/settings.json5` to customise MeedyaManager's behaviour:

```json5
{
  // Folders to monitor for new media files
  watch_paths: [
    "~/Downloads/Media",
    "~/Desktop/NewMedia"
  ],

  // File extensions to process (auto-lowercased)
  valid_extensions: [
    "mp3", "flac", "m4a", "mp4", "mkv", "avi",
    "wav", "ogg", "ac3", "alac", "mka", "m4v"
  ],

  // Rename template using MusicBee-style <Tag> syntax
  rename_format: "<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>",

  // Characters to replace in generated filenames
  filename_replacements: {
    "/": "-",
    "\\": "-",
    ":": "-",
    "*": "",
    "?": "",
    "\"": "'",
    "<": "",
    ">": "",
    "|": ""
  }
}
```

### Environment Variables

For API keys and sensitive settings, use a `.env` file:

```bash
cp .env.example .env
# Edit .env with your preferred text editor
```

See [configuration.md](configuration.md) for the full settings reference.

---

## 🔍 Metadata Lookup

MeedyaManager can automatically look up and enrich your media files with metadata from online providers such as Spotify, Apple Music, MusicBrainz, TMDB, and more. This includes artist/album/track information, genre tags, release dates, and cover art — all matched against your existing files and written back into their embedded tags.

### Quick Examples

```bash
# Look up metadata for a song
meedyamanager lookup path/to/song.mp3

# Search specific providers only
meedyamanager lookup song.mp3 -p spotify -p musicbrainz

# Auto-apply the best match (confidence >= 80%)
meedyamanager lookup song.mp3 --auto

# Preview what would change without writing
meedyamanager lookup song.mp3 --apply 1 --dry-run

# List all available providers and their status
meedyamanager lookup --providers-list

# Batch lookup from a file list
meedyamanager lookup --batch files.txt --auto
```

### GUI Lookup Tab

If you are using MeedyaManager's graphical interface, the **Lookup** tab provides the same functionality with a visual match-comparison view, side-by-side tag diffs, and one-click apply. You can also drag and drop files directly onto the tab to begin a lookup.

### Provider Setup

For detailed provider setup, see the guides in [providers/](providers/).

---

## Next Steps

- **Set up metadata providers:** See [providers/](providers/) for setup guides for Spotify, Apple Music, MusicBrainz, TMDB, and more
- **Configure rules:** See [rule-syntax.md](rule-syntax.md) for the complete template syntax guide
- **Check supported formats:** See [supported-formats.md](supported-formats.md)
- **Troubleshooting:** See [troubleshooting.md](troubleshooting.md) if you encounter issues
- **FAQ:** See [faq.md](faq.md) for common questions

---

> 📝 *For the full project plan and roadmap, see [Project_Plan.md](../Project_Plan.md) and [docs/ROADMAP.md](../docs/ROADMAP.md).*
