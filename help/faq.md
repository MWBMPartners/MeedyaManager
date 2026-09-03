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
- **macOS** — Apple Silicon (M-series) only, macOS 15+ (`macos/Package.swift` requires it)
- **Linux** — x86_64 and ARM64

There is no public release yet — see [getting-started.md](getting-started.md) for build-from-source instructions.

### Does it run in the background?

Yes. MeedyaManager can run as a system service that starts automatically:

```bash
meedya service install    # register with the OS service manager
meedya service start      # start immediately
meedya service status     # check if running
```

See [background-service.md](background-service.md) for full details.

### Will it mess up my files?

By default, `meedya watch` only **logs** file-system events — it does not rename or move anything at all unless you pass `--organize`, and `--dry-run` previews what `--organize` would do. There is currently no file-lock detection: the file watcher (`crates/mm-core/src/watcher`) has no lock/retry/queue logic, so a file that's still being written by another application could in principle be picked up mid-write. In practice this mostly matters for `--organize`/the background service; plain `meedya watch` without `--organize` never touches files regardless.

---

## File Support

### What audio formats are supported?

MP3, FLAC, ALAC, M4A, AAC, OGG, Opus, WAV, AIFF, WMA, AC3, EAC3, AC4, MKA, DTS, and more. See [supported-formats.md](supported-formats.md) for the full list.

### What video formats are supported?

MP4, M4V, MKV, AVI, DivX, MPG/MPEG, HEVC, MOV, WMV, WebM, TS. See [supported-formats.md](supported-formats.md) for the full list.

### Can it detect Dolby Atmos / Spatial Audio?

> **Status: not yet implemented.** There is currently no spatial-audio or HDR/Dolby Vision
> detection code anywhere in `mm-core` — no Dolby Atmos, Sony 360 Reality Audio, or Apple
> Spatial Audio detection exists yet. This is tracked by open issues
> [#131](https://github.com/MWBMPartners/MeedyaManager/issues/131) (spatial audio) and
> [#164](https://github.com/MWBMPartners/MeedyaManager/issues/164) (HDR).

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

Yes — up to 16. MeedyaManager supports `Custom1` through `Custom16` (`crates/mm-core/src/rule_engine/tag_registry.rs:241-256`); there is no arbitrary-name `<Custom:Name>` syntax. See [custom-tags.md](custom-tags.md).

### Can I preview rules before applying them?

Yes — use `--dry-run` on any command:

```bash
meedya watch --organize --dry-run
meedya scan ~/Music --dry-run
meedya rule test "<Artist>/<Album>/<Title>" ~/Music/song.mp3
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

Yes. MeedyaManager registers as a native Windows Service via `meedya service install`
(`crates/mm-core/src/service.rs` shells out to `sc` on Windows, `launchctl` on macOS, and
`systemctl` on Linux — this is real, working code, unlike some of the other subsystems on this
page). See [background-service.md](background-service.md) for exactly which account/login
context each platform's service runs under.

### Can it run as a macOS LaunchAgent?

Yes. `meedya service install` creates a LaunchAgent that starts at login. For a system-wide LaunchDaemon (all users, no login required), run the command with `sudo`.

### Can it run as a Linux systemd service?

Yes. `meedya service install` creates a systemd user unit that starts at login. See [background-service.md](background-service.md) for the systemd service setup.

### What happens if a file is in use?

> **Status: not yet implemented.** There is no file-lock detection or retry-queue code in
> `crates/mm-core/src/watcher` today — a file open in another application is not specially
> detected or deferred. Avoid pointing watch folders at directories where files are actively
> being downloaded or written until this is addressed.

---

## Metadata

### What metadata library does it use?

MeedyaManager uses [lofty](https://crates.io/crates/lofty) — a pure-Rust audio metadata library that supports reading and writing tags across all major formats (ID3v2, Vorbis Comments, MP4 atoms, APEv2, etc.).

### Can it edit metadata tags?

Yes. Use `meedya edit`:

```bash
meedya edit song.mp3 --set "Artist=My Artist" --set "Title=My Title"
meedya edit song.mp3 --cover /path/to/cover.jpg
meedya edit song.mp3 --remove Comment
```

Note that this writes directly to the original file even when Test Mode is on — `meedya edit`
does not route through the Test Mode safety path yet
(issue [#128](https://github.com/MWBMPartners/MeedyaManager/issues/128)).

### Can it look up metadata online?

The `mm-providers` library implements 19 providers across music, video, and podcasts, of which
13 are real (working, tested integrations) and 6 are disabled stub providers (Tidal, Shazam,
YouTube Music, iHeart, and two others — see [providers/](providers/) for which is which). There
is **no AcoustID provider** despite older documentation mentioning one, and the video provider
sometimes called "IMDb" is actually an OMDb-backed provider (id `omdb`) that requires an API
key.

**However, `meedya lookup` itself is a stub** — it does not query any of these providers yet
(issue tracked as part of the still-open work following the closed M5 milestone). See
[cli-reference.md](cli-reference.md#meedya-lookup).

```bash
# These parse but currently just print a "coming" message — no provider is queried
meedya lookup "song title"
meedya lookup "song title" --provider musicbrainz
```

---

## Cloud and Export

### Can it organise files on cloud storage?

> **Status: architectural scaffolding, not working yet.** `mm-cloud` has typed structures for
> OneDrive, Google Drive, Dropbox, MEGA, and iCloud, but there are no real network calls — OAuth
> flows exist only as comments, and provider response parsing is stubbed (e.g.
> `crates/mm-cloud/src/onedrive.rs:90` explicitly says "in production this parses `reqwest::
> Response` JSON; here it is a stub"). Do not point this at a real cloud account expecting it to
> do anything yet.

### Can it export my library to a database?

> **Status: architectural scaffolding, not working yet.** `meedya export` accepts a full set of
> flags and can print the schema it would create (`--show-schema`), but it never opens a
> database connection or writes a row — `crates/mm-export/src/sqlite.rs:30` says plainly "in
> production this holds a `sqlx::SqlitePool`", meaning it currently does not.

```bash
# Prints a summary but writes nothing to disk or to any database
meedya export --db sqlite:///home/user/library.db
```

Planned backends: SQLite, MySQL, MariaDB, PostgreSQL, SQL Server (`mssql`). See
[cli-reference.md](cli-reference.md#meedya-export) for the real flag names.

### Does it have a media server?

> **Status: architectural scaffolding, not working yet.** `meedya serve` parses server config
> and can print the intended route table, but it never starts a server —
> `crates/mm-cli/src/commands/serve.rs:337-342` prints "Server stub: exiting cleanly" and exits.
> There is no `.route(` call anywhere in `mm-server`, and the repository has zero `.html` files,
> so there is also no bundled web frontend to serve. See
> [cli-reference.md](cli-reference.md#meedya-serve).

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
