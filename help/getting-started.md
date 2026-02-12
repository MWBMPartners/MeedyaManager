# 🚀 Getting Started with MediaMancer

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

Welcome to MediaMancer! This guide will help you install, configure, and run the application for the first time.

---

## 📋 Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [First Run](#first-run)
4. [Basic Configuration](#basic-configuration)
5. [Next Steps](#next-steps)

---

## Prerequisites

### For End Users (Release Packages)

> ⚡ **No prerequisites required!** Release packages include everything you need — a bundled, sandboxed Python 3.14 runtime and all dependencies are compiled into the native executable via Nuitka. MediaMancer will **never** interfere with any other software on your system.

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

MediaMancer uses the [MediaInfo](https://mediaarea.net/en/MediaInfo) library for metadata extraction.

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

1. Download the latest release for your platform from [GitHub Releases](https://github.com/MWBMPartners/MediaMancer/releases)
2. Install or extract:
   - **Windows:** Run the `.msi` installer, or extract the `.zip`
   - **macOS:** Open the `.dmg` and drag to Applications
   - **Linux:** Run the `.AppImage`, or install the `.deb` package
3. Launch MediaMancer — no additional setup needed!

### From Source (Development)

```bash
# 1. Clone the repository
git clone https://github.com/MWBMPartners/MediaMancer.git
cd MediaMancer

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

The quickest way to test MediaMancer is to scan a single media file:

```bash
python cli/metadata_debugger.py path/to/your/song.mp3
```

This will display all detected metadata including:

- Media group (Audio/Video)
- Format class (MP3/FLAC/etc.)
- Media class (Music/Movie/etc.)
- Quality type (Lossy/Lossless)
- All embedded tags (artist, album, title, etc.)

### Export Metadata as JSON

```bash
python cli/metadata_debugger.py path/to/song.mp3 --json --out output/ --mkdir
```

### Start the Folder Watcher

```bash
# Safe simulation mode (no files moved — just logs what would happen)
python cli/runner.py

# With simulation disabled (actually renames/moves files)
python cli/runner.py --simulate-off
```

> **Tip:** Always run in simulation mode first to verify your rules produce the expected results before enabling actual file operations.

---

## Basic Configuration

Edit `config/settings.json5` to customise MediaMancer's behaviour:

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

  // Rename template using metadata placeholders
  rename_format: "{media_class}/{artist}/{album}/{track_num} - {title}.{extension}",

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

## Next Steps

- **Configure rules:** See [rule-syntax.md](rule-syntax.md) for the complete template syntax guide
- **Check supported formats:** See [supported-formats.md](supported-formats.md)
- **Troubleshooting:** See [troubleshooting.md](troubleshooting.md) if you encounter issues
- **FAQ:** See [faq.md](faq.md) for common questions

---

> 📝 *For the full project plan and roadmap, see [Project_Plan.md](../Project_Plan.md) and [docs/ROADMAP.md](../docs/ROADMAP.md).*
