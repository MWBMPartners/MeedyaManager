# Getting Started with MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

Welcome to MeedyaManager — a cross-platform media file manager and auto-organizer. This guide walks you through installation, first run, and basic configuration.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [First Run](#first-run)
4. [Basic Configuration](#basic-configuration)
5. [Metadata Lookup](#metadata-lookup)
6. [Next Steps](#next-steps)

---

## Prerequisites

**No prerequisites required for end users.** MeedyaManager is a self-contained native binary — no runtime, no interpreter, no external libraries to install.

| Component | Requirement |
| --------- | ----------- |
| **OS** | Windows 10/11 (x64 or ARM64), macOS 13+ (Apple Silicon), or Linux (x64 or ARM64) |
| **Disk Space** | ~20 MB |
| **RAM** | ~30 MB at idle; lightweight background watcher |

### For Developers (Building from Source)

| Component | Requirement |
| --------- | ----------- |
| **Rust** | 1.85+ — [rustup.rs](https://rustup.rs) |
| **Platform tools** | Linux: `libgtk-4-dev`, `libadwaita-1-dev`; macOS/Windows: none |
| **OS** | Windows (x64/ARM64), macOS (Apple Silicon), or Linux (x64/ARM64) |

---

## Installation

### From a Release Package (Recommended)

1. Download the latest release for your platform from [GitHub Releases](https://github.com/MWBMPartners/MeedyaManager/releases)
2. Install:
   - **Windows:** Run the `.msix` installer (or double-click from File Explorer)
   - **macOS:** Open the `.dmg` and drag MeedyaManager to Applications
   - **Linux:** Install via `.deb`, `.rpm`, Flatpak, Snap, or run the `.AppImage` directly
3. Launch MeedyaManager. No additional setup required.

> The `meedya` CLI binary is added to your PATH by the installer on all platforms.

### From Source (Development)

```bash
# Clone the repository
git clone https://github.com/MWBMPartners/MeedyaManager.git
cd MeedyaManager

# Build all crates
cargo build --release

# The CLI binary is at:
#   target/release/meedya           (macOS / Linux)
#   target\release\meedya.exe       (Windows)
```

To install the CLI binary system-wide:

```bash
cargo install --path crates/mm-cli
```

---

## First Run

### Inspect a Single File

The quickest way to test MeedyaManager is to inspect a media file:

```bash
meedya debug path/to/your/song.mp3
```

This displays all detected metadata, including:

- Media group (Audio / Video / Image / Document)
- Format class (MP3, FLAC, MP4, MKV, etc.)
- Media class (Music, Movie, TV Show, Podcast, etc.)
- Quality type (Lossy / Lossless / Uncompressed)
- All embedded tags (artist, album, title, track number, etc.)

For JSON output (useful for scripting):

```bash
meedya debug path/to/song.mp3 --json
```

### Preview Renames for a Directory

Scan a directory and preview what MeedyaManager would rename each file to, without touching anything:

```bash
meedya scan ~/Music --dry-run
```

### Start the Folder Watcher

Watch directories for new media files and process them automatically:

```bash
# Preview mode — logs what would happen, no files moved
meedya watch --dry-run

# Live mode — renames/moves files according to your rules
meedya watch
```

> **Tip:** Always run with `--dry-run` first to verify your rules produce the expected results before enabling live file operations.

### Launch the GUI

**Linux (GTK4):**

```bash
meedya-gtk
```

**macOS:** Open MeedyaManager from Launchpad or Spotlight.

**Windows:** Open MeedyaManager from the Start menu.

---

## Basic Configuration

MeedyaManager stores its configuration in a JSON5 file. The location is:

| Platform | Path |
| -------- | ---- |
| **macOS** | `~/Library/Application Support/MeedyaManager/settings.json5` |
| **Linux** | `~/.config/MeedyaManager/settings.json5` |
| **Windows** | `%APPDATA%\MeedyaManager\settings.json5` |

A default configuration is created automatically on first run. To open it:

```bash
meedya config show
```

### Minimal Configuration Example

```json5
{
  watch: {
    // Folders to monitor for new media files
    folders: [
      "~/Downloads/Media",
      "~/Desktop/NewMedia"
    ],
    recursive: true
  },

  rename: {
    // Output directory for organised files
    output_dir: "~/Media",

    // Rename template using MusicBee-style <Tag> syntax
    template: "<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>",

    // What to do when a destination file already exists
    conflict_strategy: "rename"   // "rename", "skip", or "overwrite"
  }
}
```

See [configuration.md](configuration.md) for the full settings reference.

---

## Metadata Lookup

MeedyaManager can look up and enrich your media files with metadata from 19+ online providers including Spotify, Apple Music, MusicBrainz, TMDB, TVDB, and more.

### Quick Examples

```bash
# Look up metadata for a single file
meedya lookup path/to/song.mp3

# Restrict to specific providers
meedya lookup song.mp3 --providers spotify,musicbrainz

# Auto-apply the best match (confidence >= 80%)
meedya lookup song.mp3 --auto

# Preview changes without writing tags
meedya lookup song.mp3 --dry-run

# List all configured providers and their status
meedya lookup --list-providers
```

### GUI Lookup

In the native GUI, the **Lookup** panel provides the same functionality with a visual match-comparison view, side-by-side tag diffs, and one-click apply. Files can be dragged and dropped directly onto the panel.

See the [providers/](providers/) directory for setup guides for each provider.

---

## Next Steps

- **Configure rules:** [rule-syntax.md](rule-syntax.md) — full template syntax reference
- **CLI reference:** [cli-reference.md](cli-reference.md) — every command and option
- **Configuration reference:** [configuration.md](configuration.md) — all settings explained
- **Supported formats:** [supported-formats.md](supported-formats.md)
- **Background service:** [background-service.md](background-service.md) — run MeedyaManager at startup
- **Metadata providers:** [providers/](providers/) — setup guides for all 19+ providers
- **Troubleshooting:** [troubleshooting.md](troubleshooting.md)
- **FAQ:** [faq.md](faq.md)
