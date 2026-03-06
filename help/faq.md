# Frequently Asked Questions — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

---

## General

### What is MeedyaManager?

MeedyaManager is a cross-platform media file manager and auto-organizer. It monitors folders, reads metadata from audio and video files, and renames/organizes them according to user-defined rules. It's inspired by MusicBee's auto-organize feature but runs natively on Windows, macOS, and Linux.

### Is MeedyaManager free?

Yes. MeedyaManager is open-source software licensed under GPL-2.0-or-later.

### What platforms are supported?

- **Windows** — x64 and ARM64 (Windows 10/11)
- **macOS** — Apple Silicon (M-series) only, macOS 13+
- **Linux** — x86_64 and ARM64

### Does it run in the background?

Yes. MeedyaManager can run as a system service that starts automatically:

```bash
meedya service install    # register with the OS service manager
meedya service start      # start immediately
meedya service status     # check if running
```

See [background-service.md](background-service.md) for full details.

### Will it mess up my files?

By default, `meedya watch` runs in **preview mode** — it logs what it *would* do without moving or renaming anything. Pass `--dry-run` explicitly to any command to always preview first. MeedyaManager also detects file locks and never touches files that are open in another application.

---

## File Support

### What audio formats are supported?

MP3, FLAC, ALAC, M4A, AAC, OGG, Opus, WAV, AIFF, WMA, AC3, EAC3, AC4, MKA, DTS, and more. See [supported-formats.md](supported-formats.md) for the full list.

### What video formats are supported?

MP4, M4V, MKV, AVI, DivX, MPG/MPEG, HEVC, MOV, WMV, WebM, TS. See [supported-formats.md](supported-formats.md) for the full list.

### Can it detect Dolby Atmos / Spatial Audio?

Yes. MeedyaManager detects spatial audio formats including Dolby Atmos, Sony 360 Reality Audio, and Apple Spatial Audio. These properties are available as tags in your rename templates.

### Can it tell the difference between lossy and lossless?

Yes. The `<Quality Type>` tag automatically classifies files as `Lossy` or `Lossless` based on their codec.

### What about subtitle files and cover art?

MeedyaManager recognises companion files (SRT, LRC, ASS, cover art, disc images, CUE sheets) and moves them alongside their associated media files when the primary file is renamed.

---

## Rules and Templates

### How do I define sorting rules?

Rules use a template syntax inspired by MusicBee. Templates combine tag references (`<Tag>`) and functions (`$If`, `$Replace`, `$Pad`, etc.) to build file paths. See [rule-syntax.md](rule-syntax.md) for the complete syntax guide.

### Can I use IF/AND/OR conditions?

Yes. The rule engine supports:

- `$If(condition, trueResult, falseResult)` — conditional logic
- `$And(cond1, cond2)` — both must be true
- `$Or(cond1, cond2)` — either can be true

These can be nested to any depth.

### Is there a limit on custom tags?

No. MeedyaManager supports unlimited custom tags via the `<Custom:Name>` syntax.

### Can I preview rules before applying them?

Yes — use `--dry-run` on any command:

```bash
meedya watch --dry-run
meedya scan ~/Music --dry-run
meedya rule test --template "<Artist>/<Album>/<Title>" ~/Music/song.mp3
```

---

## Configuration

### Where are the settings stored?

| Platform | Path |
| -------- | ---- |
| **macOS** | `~/Library/Application Support/MeedyaManager/settings.json5` |
| **Linux** | `~/.config/MeedyaManager/settings.json5` |
| **Windows** | `%APPDATA%\MeedyaManager\settings.json5` |

### What format is the config file?

JSON5 — a superset of JSON that supports comments, trailing commas, and unquoted keys. A default config is created on first run.

### How do I add API keys for metadata providers?

Add them to your `settings.json5` or, preferably, as environment variables to keep secrets out of the config file:

```bash
export MM_SPOTIFY_CLIENT_ID=your_id
export MM_SPOTIFY_CLIENT_SECRET=your_secret
export MM_TMDB_API_KEY=your_key
```

Or in a `.env` file next to `settings.json5`. See [configuration.md](configuration.md) for the full list.

---

## Background Service

### Can it run as a Windows Service?

Yes. MeedyaManager registers as a native Windows Service via `meedya service install`. It starts automatically at boot, no login required.

### Can it run as a macOS LaunchAgent?

Yes. `meedya service install` creates a LaunchAgent that starts at login. For a system-wide LaunchDaemon (all users, no login required), run the command with `sudo`.

### Can it run as a Linux systemd service?

Yes. `meedya service install` creates a systemd user unit that starts at login. See [background-service.md](background-service.md) for the systemd service setup.

### What happens if a file is in use?

MeedyaManager detects file locks and queues the file for automatic retry. Once the lock is released, the file is processed. This prevents corruption from partial reads.

---

## Metadata

### What metadata library does it use?

MeedyaManager uses [lofty](https://crates.io/crates/lofty) — a pure-Rust audio metadata library that supports reading and writing tags across all major formats (ID3v2, Vorbis Comments, MP4 atoms, APEv2, etc.).

### Can it edit metadata tags?

Yes. Use `meedya edit`:

```bash
meedya edit song.mp3 --tag "Artist=My Artist" --tag "Title=My Title"
meedya edit song.mp3 --cover /path/to/cover.jpg
meedya edit song.mp3 --remove-tag Comment
```

### Can it look up metadata online?

Yes. MeedyaManager supports 19+ metadata providers across music, video, and podcast categories:

**Music:** MusicBrainz, Spotify, Apple Music, Tidal, Deezer, Amazon Music, YouTube Music, iHeart, Pandora, Shazam, ISRC, ISWC, AcoustID

**Video:** TMDb, TVDB, IMDb, Apple TV, iTunes Store, EIDR

**Podcasts:** Apple Podcasts

```bash
meedya lookup song.mp3               # search all enabled providers
meedya lookup song.mp3 -p musicbrainz  # search specific provider
meedya lookup --list-providers       # see all providers and their status
```

---

## Cloud and Export

### Can it organise files on cloud storage?

Yes. MeedyaManager supports monitoring OneDrive, Google Drive, Dropbox, MEGA, and iCloud — including detecting new files added via sync clients. See [providers/](providers/) for cloud setup.

### Can it export my library to a database?

Yes. Use `meedya export`:

```bash
meedya export --format sqlite --out ~/library.db
meedya export --format postgres --url postgresql://user:pass@host/db
```

Supported databases: SQLite, MySQL, MariaDB, PostgreSQL, SQL Server.

### Does it have a media server?

Yes. `meedya serve` starts an HTTPS media server with JWT authentication:

```bash
meedya serve --port 8443 --cert /path/to/cert.pem --key /path/to/key.pem
```

---

## Development

### What language is it written in?

Rust — the core engine (`mm-core`) is a Rust library shared by all platform UIs via FFI. The CLI (`meedya`) and Linux GTK4 UI are pure Rust. The macOS UI is SwiftUI and the Windows UI is WinUI 3 (C#).

### How can I contribute?

Contributions are welcome. Check [GitHub Issues](https://github.com/MWBMPartners/MeedyaManager/issues) for open tasks, or submit a pull request.

### Where do I report bugs?

Use the built-in bug reporter first — it captures system info and logs:

```bash
meedya report-bug
```

Then open an issue at [GitHub Issues](https://github.com/MWBMPartners/MeedyaManager/issues/new?template=bug_report.md) and attach the generated report.
