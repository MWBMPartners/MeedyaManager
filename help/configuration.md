# ⚙️ Configuration Reference — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

MeedyaManager is configured through two primary mechanisms: a **JSON5 settings file** and **environment variables**.

---

## 📋 Table of Contents

1. [Configuration Files](#configuration-files)
2. [Settings Reference (settings.json5)](#settings-reference)
3. [Environment Variables (.env)](#environment-variables)
4. [API Key Management](#api-key-management)
5. [Platform-Specific Settings](#platform-specific-settings)

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
// Current M1 placeholder syntax:
rename_format: "{media_class}/{artist}/{album}/{track_num} - {title}.{extension}"

// Future M3 MusicBee-style syntax:
// rename_format: "<Media Class>/<Album Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>"
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
MUSICBRAINZ_API_KEY=
SPOTIFY_CLIENT_ID=
SPOTIFY_CLIENT_SECRET=
APPLE_MUSIC_TOKEN=
TIDAL_SESSION_ID=
AMAZON_MUSIC_AUTH=
SHAZAM_API_KEY=
ACOUSTICBRAINZ_API_KEY=
```

### TV/Film Metadata API Keys (M6)

```env
TMDB_API_KEY=
TVDB_API_KEY=
IMDB_ACCESS_TOKEN=
EIDR_CLIENT_ID=
EIDR_CLIENT_SECRET=
APPLE_TV_LOOKUP_TOKEN=
```

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

## Platform-Specific Settings

### macOS (Apple Silicon)

- MediaInfo: Install via `brew install mediainfo`
- Service: LaunchAgent (per-user) or LaunchDaemon (system-wide)
- Paths use `/` separators

### Windows (x64/ARM)

- MediaInfo: Install from [mediaarea.net](https://mediaarea.net)
- Service: Windows Service via `pywin32`
- Paths use `\` separators (JSON5 requires `\\` escaping)

### Linux (x64/ARM)

- MediaInfo: Install via package manager (`apt`, `dnf`, `pacman`)
- Service: systemd unit file
- Paths use `/` separators

---

> 📝 *For troubleshooting configuration issues, see [troubleshooting.md](troubleshooting.md).*
