# ❓ Frequently Asked Questions — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd**

---

## General

### What is MeedyaManager?

MeedyaManager is a cross-platform media file management application that automatically monitors folders, reads metadata from audio and video files, and renames/organizes them according to user-defined rules. It's inspired by MusicBee's auto-organize feature but works on Windows, macOS, and Linux.

### Is MeedyaManager free?

Yes. MeedyaManager is open-source software licensed under GPL-2.0-or-later.

### What platforms are supported?

- **Windows** — x64 and ARM64
- **macOS** — Apple Silicon (M-series) only
- **Linux** — x86_64 and ARM64

### Does it run in the background?

Yes. MeedyaManager can run as:

- A **system service** (starts at boot, no login required)
- A **login agent** (starts when a user logs in)
- **Manually** via the CLI or GUI

### Will it mess up my files?

By default, MeedyaManager runs in **simulation mode** — it logs what it *would* do without actually moving or renaming anything. You must explicitly disable simulation to perform real operations. Additionally, it never touches files that are open in other applications.

---

## File Support

### What audio formats are supported?

MP3, FLAC, ALAC, M4A, AAC, OGG, Opus, WAV, AIFF, WMA, AC3, EAC3, AC4, MKA, DTS, and more. See [supported-formats.md](supported-formats.md) for the full list.

### What video formats are supported?

MP4, M4V, MKV, AVI, DivX, MPG/MPEG, HEVC, MOV, WMV, WebM, TS. See [supported-formats.md](supported-formats.md) for the full list.

### Can it detect Dolby Atmos / Spatial Audio?

Yes. MeedyaManager uses MediaInfo to detect spatial audio formats including Dolby Atmos, Sony 360 Reality Audio, and Apple Spatial Audio. These can be used in sorting rules.

### Can it tell the difference between lossy and lossless?

Yes. The `<Quality Type>` tag automatically classifies files as Lossy or Lossless based on their codec.

### What about subtitle files and cover art?

MeedyaManager recognises companion files (SRT, LRC, ASS, cover art, disc images, CUE sheets) and moves them alongside their associated media files.

---

## Rules & Templates

### How do I define sorting rules?

Rules are defined using a template syntax inspired by MusicBee. Templates combine tag references (`<Tag>`) and functions (`$If`, `$Replace`, etc.) to build file paths. See [rule-syntax.md](rule-syntax.md) for the complete syntax guide.

### Can I use IF/AND/OR conditions?

Yes. The rule engine supports:

- `$If(condition, trueResult, falseResult)` — conditional logic
- `$And(cond1, cond2)` — both must be true
- `$Or(cond1, cond2)` — either can be true
- These can be **nested** to any depth

### Is there a limit on custom tags?

No. Unlike MusicBee's limit of 16-20 custom tags, MeedyaManager supports **unlimited** custom tags using the `<Custom:Name>` syntax.

### Can I preview rules before applying them?

Yes. The simulation mode shows exactly what would happen for each file. The GUI (M2+) will include a visual preview panel.

---

## Configuration

### Where are the settings stored?

- **Main config:** `config/settings.json5`
- **Local overrides:** `settings.local.json5` (not tracked in git)
- **API keys:** `.env` file (not tracked in git)

### What format is the config file?

JSON5 — a superset of JSON that supports comments, trailing commas, and unquoted keys. This makes it much more user-friendly than plain JSON.

### How do I add API keys for metadata lookup?

Copy `.env.example` to `.env` and fill in your keys:

```bash
cp .env.example .env
```

API keys for services like Spotify, TMDb, etc. will be used in future milestones (M5-M6).

---

## Service & Background

### Can it run as a Windows Service?

Planned for M8. It will use `pywin32` to register as a native Windows Service.

### Can it run as a macOS LaunchDaemon?

Planned for M8. It will generate a `.plist` file for macOS's `launchd` system.

### Can it run as a Linux systemd service?

Planned for M8. It will generate a systemd unit file.

### What happens if a file is in use?

MeedyaManager detects file locks and adds the file to a retry queue. Once the file is no longer in use, it will be processed automatically. This prevents file corruption.

---

## Metadata

### What metadata extraction library does it use?

MeedyaManager uses [MediaInfo](https://mediaarea.net) via the `pymediainfo` Python wrapper for reading metadata. For writing metadata (M4+), it will use [mutagen](https://mutagen.readthedocs.io).

### Can it edit metadata tags?

Manual metadata editing is planned for **Milestone 4**. This will include:

- Reading and writing tags across all supported formats
- Multi-value tag support
- Custom tag creation
- Batch editing

### Will it look up metadata online?

Yes, in future milestones:

- **M5:** Music lookup (MusicBrainz, Spotify, Apple Music, Tidal, Amazon Music, Shazam, AcousticBrainz)
- **M6:** TV/Film lookup (TMDb, TheTVDB, IMDb, Apple TV, iTunes Store, EIDR)

### Will it keep my files playable?

Yes. MeedyaManager uses standard container formats and tag specifications wherever possible. Custom tags are stored in a way that preserves playability on all players/devices, even if some players can't read the custom fields.

---

## Cloud & Export

### Can it organise files on cloud storage?

Cloud storage monitoring is planned for **Milestone 7**, supporting OneDrive, Google Drive, Dropbox, MEGA, and iCloud.

### Can it export my library to a database?

Database export is planned for **Milestone 9**, supporting MySQL, MariaDB, SQL Server, SQLite, and PostgreSQL.

---

## Development

### What language is it written in?

Python 3.11+ with plans for a PySide6 (Qt6) GUI.

### How can I contribute?

Contributions are welcome! Check the [GitHub Issues](https://github.com/MWBMPartners/MeedyaManager/issues) for open tasks, or submit a pull request.

### Where do I report bugs?

[GitHub Issues](https://github.com/MWBMPartners/MeedyaManager/issues/new?template=bug_report.md) — please include your OS, Python version, and relevant log output.

---

> 📝 *This FAQ is updated regularly. If your question isn't here, please [open an issue](https://github.com/MWBMPartners/MeedyaManager/issues/new).*
