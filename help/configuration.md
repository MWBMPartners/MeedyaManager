# ⚙️ Configuration Reference — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

MeedyaManager is configured through two primary mechanisms: a **JSON5 settings file** and **environment variables**.

---

## 📋 Table of Contents

1. [Configuration Files](#configuration-files)
2. [Settings Reference (settings.json5)](#settings-reference)
3. [Environment Variables (.env)](#environment-variables)
4. [API Key Management](#api-key-management)
5. [Metadata Lookup Providers](#metadata-lookup-providers)
6. [Platform-Specific Settings](#platform-specific-settings)

---

## Configuration Files

| File | Purpose | Tracked in Git? |
| ---- | ------- | --------------- |
| `config/settings.json5` | Main application settings | ✅ Yes (defaults) |
| `settings.local.json5` | Local overrides (per machine) | ❌ No (git-ignored) |
| `.env` | API keys and sensitive values | ❌ No (git-ignored) |
| `.env.example` | Template for `.env` | ✅ Yes |

### Priority Order

Settings are resolved in this order (highest priority first):

1. **Environment variables** (`.env` or system)
2. **Local config** (`settings.local.json5`)
3. **Main config** (`config/settings.json5`)
4. **Built-in defaults** (hardcoded fallbacks)

---

## Settings Reference

### `watch_paths` — Folders to Monitor

```json5
watch_paths: [
  "~/Downloads/Media",
  "~/Desktop/NewMedia",
  "/Volumes/ExternalDrive/Incoming"
]
```

- Array of directory paths to watch for new media files
- Supports `~` expansion for home directory
- Non-existent paths are logged as warnings but don't cause errors
- Subdirectories are watched recursively

### `valid_extensions` — Accepted File Types

```json5
valid_extensions: [
  "mp3", "flac", "m4a", "alac", "ogg", "wav", "ac3", "eac3", "ac4",
  "mp4", "m4v", "mkv", "mka", "avi", "mpg", "mpeg", "hevc", "divx",
  "mov", "wmv", "webm", "ts", "opus", "aac", "aiff", "wma"
]
```

- All extensions are auto-lowercased during comparison
- Do not include the leading dot (use `"mp3"` not `".mp3"`)

### `rename_format` — Rename Template

```json5
// MusicBee-style <Tag> syntax with template functions:
rename_format: "<Media Class>/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>"
```

See [rule-syntax.md](rule-syntax.md) for the full template syntax reference.

### `fallback_metadata` — Default Values

```json5
fallback_metadata: {
  media_group: "Audio",
  format_class: "unknown",
  media_class: "Music",
  quality_type: "Lossy"
}
```

Used when MediaInfo cannot determine a classification value.

### `filename_replacements` — Character Substitution

```json5
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
```

Characters on the left are replaced with the character on the right in generated filenames. This prevents filesystem errors across platforms.

### `watch_mode` — Watcher Backend

```json5
watch_mode: "watchdog"  // Options: "watchdog", "polling"
```

- `"watchdog"` — Uses native OS file system events (recommended, lower CPU)
- `"polling"` — Periodically scans directories (fallback if watchdog unavailable)

### `simulate_watcher` — Safe Mode

```json5
simulate_watcher: true  // true = dry-run only, false = actually move/rename files
```

When `true`, no files are moved or renamed — operations are only logged.

---

## Environment Variables

Create a `.env` file by copying the template:

```bash
cp .env.example .env
```

### Music Metadata API Keys (M5)

```env
SPOTIFY_CLIENT_ID=
SPOTIFY_CLIENT_SECRET=
APPLE_MUSIC_TEAM_ID=
APPLE_MUSIC_KEY_ID=
APPLE_MUSIC_PRIVATE_KEY=
TIDAL_CLIENT_ID=
TIDAL_CLIENT_SECRET=
YOUTUBE_MUSIC_HEADERS_AUTH=
```

### Video Metadata API Keys (M5)

```env
TMDB_API_KEY=
TVDB_API_KEY=
EIDR_CLIENT_ID=
EIDR_CLIENT_SECRET=
```

### Providers That Need No API Key

The following providers work without any API key or credentials:

- **MusicBrainz** — Open music metadata database
- **Deezer** — Public search API
- **Apple Podcasts** — Public catalogue lookup
- **Apple TV** — Public catalogue lookup
- **iTunes Store** — Public search API
- **Shazam** — Public recognition API
- **iHeart** — Public search API

### Application Overrides

```env
METAMANCER_PROFILE_NAME=dev       # Profile name for logging
METAMANCER_FALLBACK_LANGUAGE=en   # Default language for metadata
METAMANCER_REGION_DEFAULT=GB      # Default region code
METAMANCER_LOG_LEVEL=INFO         # Logging level: DEBUG, INFO, WARNING, ERROR
```

---

## API Key Management

MeedyaManager supports three tiers of API key management:

### 1. Developer Keys (Private)

Stored in `.env` (git-ignored). For development and testing only.

### 2. Universal Keys (Bundled)

For services whose Terms of Service allow a shared API key, the key can be bundled with the application. Controlled via build config:

```json5
api_keys: {
  musicbrainz: { include_in_build: true },   // ToS allows shared key
  spotify:     { include_in_build: false },   // User must provide own
}
```

### 3. User-Provided Keys

End users can always provide their own API keys via:

- The `.env` file
- The Settings dialog (UI, M2+)
- `settings.local.json5`

User-provided keys always take priority over bundled keys.

---

## 🔍 Metadata Lookup Providers

MeedyaManager can look up and enrich media file metadata from a variety of online providers. This section covers how to configure providers, cover art behaviour, and credential resolution.

### Provider Configuration

Enable or disable individual providers in `settings.json5`:

```json5
providers: {
  spotify: {
    enabled: true,
    // Credentials loaded from .env or OS keyring
  },
  apple_music: {
    enabled: true,
    storefront: "gb",    // Country code for search results
  },
  musicbrainz: {
    enabled: true,
    // No credentials required
  },
  tmdb: {
    enabled: true,
  },
  // ... more providers
}
```

Each provider entry supports an `enabled` flag to toggle it on or off. Provider-specific options (such as `storefront` for Apple Music) are documented in the individual provider guides.

### Cover Art Configuration

Control how MeedyaManager handles cover art downloaded during metadata lookup:

```json5
cover_art: {
  download_static: true,      // Download static cover art (FrontCover.jpg)
  download_animated: true,    // Download animated covers (FrontCover.mp4, PortraitCover.mp4)
  embed_in_file: true,        // Embed cover art in media file tags
  save_alongside: true,       // Save cover art files next to media files
  max_resolution: 3000,       // Maximum resolution for static art
}
```

### Credential Priority Chain

When resolving API credentials for a provider, MeedyaManager checks the following sources in order (highest priority first):

1. **Environment variables** (`.env`) — Keys defined in your `.env` file or exported in your shell environment
2. **Config file** (`settings.json5` → `providers` section) — Credentials specified inline in the provider configuration
3. **OS keyring** — Platform-native secure storage (macOS Keychain, Windows Credential Manager, Linux SecretService via D-Bus)
4. **Encrypted bundle** — The application's bundled fallback keys (for providers whose Terms of Service permit shared keys)

The first source that provides a valid credential wins. This means you can always override bundled or keyring-stored keys by setting an environment variable.

### Per-Provider Setup Guides

See individual provider guides in [help/providers/](providers/) for detailed setup instructions, including how to obtain API keys, configure OAuth flows, and troubleshoot authentication issues.

---

## Logging Configuration

MeedyaManager uses centralized logging with platform-appropriate log directories and automatic PII redaction.

### Log Settings

```json5
logging: {
  level: "INFO",           // DEBUG, INFO, WARNING, ERROR, CRITICAL
  max_log_days: 30,        // Retain log files for this many days
  max_log_size_mb: 10,     // Safety-net file size cap per log file
  console_level: "WARNING", // Minimum level for console output
  redact_pii: true,        // Redact user paths from all log records
}
```

### Log File Location

| Platform | Directory |
|----------|-----------|
| macOS | `~/Library/Logs/MeedyaManager/` |
| Windows | `%LOCALAPPDATA%\MeedyaManager\logs\` |
| Linux | `~/.local/state/MeedyaManager/logs/` |

Override with the `METAMANCER_LOG_LEVEL` environment variable:

```env
METAMANCER_LOG_LEVEL=DEBUG
```

### Log Rotation

- **Daily rotation** — New log file each day at midnight
- **Size safety net** — 10 MB max per file (5 backups)
- **Auto-cleanup** — Logs older than 30 days are removed on startup

---

## Configuration Profiles (Export / Import)

MeedyaManager supports exporting and importing settings as portable `.mmprofile` bundles for migration between platforms.

### Export

```bash
meedyamanager config export --out ~/backup.mmprofile --name "Home Mac"
meedyamanager config export --out ~/backup.mmprofile --include-secrets
```

Or via **Settings dialog** → **Export Settings...** button.

### Import

```bash
meedyamanager config import ~/backup.mmprofile --dry-run        # Preview changes
meedyamanager config import ~/backup.mmprofile --mode merge      # Additive merge
meedyamanager config import ~/backup.mmprofile --mode replace -y # Replace without prompt
```

Or via **Settings dialog** → **Import Settings...** button.

### Profile Format

A `.mmprofile` file is a ZIP archive containing:

| File | Purpose |
|------|---------|
| `manifest.json` | Version, platform, timestamp, profile name |
| `settings.json5` | Full config with paths tokenized for portability |
| `env.template` | API key names with blank values |
| `env.secrets` | Actual API key values (only if `--include-secrets`) |

### Cross-Platform Path Tokens

Paths are automatically converted between platforms:

| Token | macOS | Windows | Linux |
|-------|-------|---------|-------|
| `{HOME}` | `/Users/name` | `C:\Users\name` | `/home/name` |
| `{DESKTOP}` | `~/Desktop` | `~\Desktop` | `~/Desktop` |
| `{DOWNLOADS}` | `~/Downloads` | `~\Downloads` | `~/Downloads` |
| `{MUSIC}` | `~/Music` | `~\Music` | `~/Music` |
| `{VIDEOS}` | `~/Movies` | `~\Videos` | `~/Videos` |

---

## Platform-Specific Settings

### macOS (Apple Silicon)

- MediaInfo: Bundled via `pymediainfo` pip wheel (fallback: `brew install mediainfo`)
- Service: LaunchAgent (per-user) or LaunchDaemon (system-wide)
- Paths use `/` separators

### Windows (x64/ARM)

- MediaInfo: Bundled via `pymediainfo` pip wheel (fallback: install from [mediaarea.net](https://mediaarea.net))
- Service: Windows Service via `pywin32`
- Paths use `\` separators (JSON5 requires `\\` escaping)

### Linux (x64/ARM)

- MediaInfo: Install via package manager (`apt install mediainfo`, `dnf install mediainfo`, `pacman -S mediainfo`)
- Service: systemd unit file
- Paths use `/` separators

---

> 📝 *For troubleshooting configuration issues, see [troubleshooting.md](troubleshooting.md).*
