# Configuration Reference — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager is configured through a single **JSON5 settings file** — no command-line flags required for basic operation, and no environment variables needed unless you want to store API keys outside the config file.

---

## Table of Contents

1. [Configuration File Location](#configuration-file-location)
2. [AppConfig — Root Settings](#appconfig--root-settings)
3. [watch — File System Watcher](#watch--file-system-watcher)
4. [rename — File Organisation](#rename--file-organisation)
5. [logging — Diagnostics](#logging--diagnostics)
6. [providers — Metadata Lookup](#providers--metadata-lookup)
7. [Environment Variable Overrides](#environment-variable-overrides)
8. [CLI Config Commands](#cli-config-commands)

---

## Configuration File Location

MeedyaManager automatically creates a default configuration file on first run at the platform-appropriate location:

| Platform | Path |
| -------- | ---- |
| **macOS** | `~/Library/Application Support/MeedyaManager/settings.json5` |
| **Linux** | `~/.config/MeedyaManager/settings.json5` |
| **Windows** | `%APPDATA%\MeedyaManager\settings.json5` |

To view or edit the configuration from the CLI:

```bash
meedya config show        # Print current config to terminal
meedya config validate    # Check config for errors
meedya config path        # Print the config file path
```

To use a custom config file path:

```bash
meedya --config /path/to/my-settings.json5 scan ~/Music
```

---

## AppConfig — Root Settings

```json5
{
  // Human-readable application name (informational only)
  app_name: "MeedyaManager",

  // Global dry-run — when true, no files are moved or renamed
  dry_run: false,

  watch:     { /* ... */ },
  rename:    { /* ... */ },
  logging:   { /* ... */ },
  providers: { /* ... */ }
}
```

| Field | Type | Default | Description |
| ----- | ---- | ------- | ----------- |
| `app_name` | string | `"MeedyaManager"` | Application name (informational) |
| `dry_run` | bool | `false` | When `true`, no files are moved or renamed globally |

> Setting `dry_run: true` in the config file is equivalent to passing `--dry-run` on every command.

---

## watch — File System Watcher

Controls which directories MeedyaManager monitors and how events are debounced.

```json5
watch: {
  // Directories to watch for new or changed media files
  folders: [
    "~/Downloads/Media",
    "~/Desktop/Incoming",
    "/mnt/nas/Unsorted"
  ],

  // Watch subdirectories recursively
  recursive: true,

  // Polling interval (seconds) — used as fallback when native FS events
  // are unavailable (e.g. network mounts, Docker volumes)
  poll_interval_secs: 5,

  // Debounce window (milliseconds) — events within this window are merged
  // into a single notification, preventing duplicate processing
  debounce_ms: 200,

  // Extensions to process (empty list = all recognised media types)
  include_extensions: ["mp3", "flac", "m4a", "mp4", "mkv"],

  // Extensions to always skip
  exclude_extensions: ["tmp", "part", "crdownload"]
}
```

| Field | Type | Default | Description |
| ----- | ---- | ------- | ----------- |
| `folders` | `string[]` | `[]` | Directories to monitor (must be configured) |
| `recursive` | bool | `true` | Watch subdirectories recursively |
| `poll_interval_secs` | int | `5` | Polling fallback interval in seconds |
| `debounce_ms` | int | `200` | Event debounce window in milliseconds |
| `include_extensions` | `string[]` | `[]` | Restrict processing to these extensions (empty = all) |
| `exclude_extensions` | `string[]` | `[]` | Always skip these extensions |

> **Note:** No default watch folders are configured. You must add at least one folder before `meedya watch` will process any files.

---

## rename — File Organisation

Controls how files are renamed and where they are placed.

```json5
rename: {
  // MusicBee-style template for building the destination file path.
  // Uses <Tag> placeholders and $Function() calls.
  // See help/rule-syntax.md for the full syntax reference.
  template: "<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>",

  // Root directory where organised files are placed.
  // Relative paths are resolved from the user's home directory.
  output_dir: "~/Media/Organised",

  // What to do when a destination file already exists:
  //   "skip"       — leave the source file untouched (safest)
  //   "overwrite"  — replace the destination
  //   "rename"     — append a counter to make the name unique
  //   "ask"        — prompt interactively (GUI only)
  conflict_strategy: "skip",

  // Create missing directories in the output path automatically
  create_dirs: true,

  // Copy the source file instead of moving it
  copy_mode: false,

  // Behaviour when a tag is missing during template evaluation:
  //   "empty"    — insert an empty string (default)
  //   "literal"  — insert the tag name verbatim, e.g. <Artist>
  //   "error"    — abort the rename for this file
  missing_tag_mode: "empty",

  // Conditional rules evaluated before the fallback template above.
  // First matching rule wins. See help/rule-syntax.md for syntax.
  rules: []
}
```

| Field | Type | Default | Description |
| ----- | ---- | ------- | ----------- |
| `template` | string | `"<Artist>/<Album>/<Title>"` | Default rename template |
| `output_dir` | string or null | `null` | Output root (null = same dir as source) |
| `conflict_strategy` | string | `"skip"` | Conflict resolution: `"skip"`, `"overwrite"`, `"rename"`, `"ask"` |
| `create_dirs` | bool | `true` | Create missing output directories |
| `copy_mode` | bool | `false` | Copy files instead of moving |
| `missing_tag_mode` | string | `"empty"` | Missing tag behaviour: `"empty"`, `"literal"`, `"error"` |
| `rules` | array | `[]` | Conditional rule list (evaluated in priority order) |

---

## logging — Diagnostics

Controls log verbosity, output destinations, and PII redaction.

```json5
logging: {
  // Minimum log level: "trace", "debug", "info", "warn", "error"
  level: "info",

  // Emit logs to the terminal (stdout)
  console: true,

  // Optional log file path (null = no file logging)
  // Use an absolute path or one starting with ~/
  file: null,

  // Maximum log file size in bytes before rotating (default: 10 MB)
  max_file_size_bytes: 10485760,

  // Number of rotated log files to retain
  max_rotated_files: 3,

  // Redact file paths and other PII from log records
  redact_pii: true
}
```

| Field | Type | Default | Description |
| ----- | ---- | ------- | ----------- |
| `level` | string | `"info"` | Log level: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"` |
| `console` | bool | `true` | Print logs to terminal |
| `file` | string or null | `null` | Log file path (null = disabled) |
| `max_file_size_bytes` | int | `10485760` | 10 MB max per log file before rotation |
| `max_rotated_files` | int | `3` | Number of old log files to keep |
| `redact_pii` | bool | `true` | Redact file paths and user data from logs |

> Verbose output can also be enabled per-command with `-v` (info), `-vv` (debug), or `-vvv` (trace). This overrides the config `level` for that run.

---

## providers — Metadata Lookup

Configures which metadata providers are enabled and supplies their credentials. All keys can be stored in the config file or overridden via environment variables (recommended for secrets).

```json5
providers: {
  // MusicBrainz — free, no credentials required
  musicbrainz_enabled: true,

  // Discogs — requires a personal access token
  discogs_enabled: false,
  discogs_token: null,        // or set MM_DISCOGS_TOKEN env var

  // Spotify — requires OAuth client credentials
  spotify_enabled: false,
  spotify_client_id: null,    // or MM_SPOTIFY_CLIENT_ID
  spotify_client_secret: null, // or MM_SPOTIFY_CLIENT_SECRET

  // TMDb (The Movie Database) — requires an API key
  tmdb_enabled: false,
  tmdb_api_key: null,         // or MM_TMDB_API_KEY

  // AcoustID — acoustic fingerprint matching, requires an API key
  acoustid_enabled: false,
  acoustid_api_key: null,     // or MM_ACOUSTID_API_KEY

  // Global timeout for all provider HTTP requests (seconds)
  request_timeout_secs: 30,

  // Maximum number of concurrent provider API requests
  max_concurrent_requests: 4
}
```

| Field | Type | Default | Description |
| ----- | ---- | ------- | ----------- |
| `musicbrainz_enabled` | bool | `true` | Enable MusicBrainz lookups (no key needed) |
| `discogs_enabled` | bool | `false` | Enable Discogs lookups |
| `discogs_token` | string or null | `null` | Discogs personal access token |
| `spotify_enabled` | bool | `false` | Enable Spotify lookups |
| `spotify_client_id` | string or null | `null` | Spotify client ID |
| `spotify_client_secret` | string or null | `null` | Spotify client secret |
| `tmdb_enabled` | bool | `false` | Enable TMDb (movie/TV) lookups |
| `tmdb_api_key` | string or null | `null` | TMDb API key |
| `acoustid_enabled` | bool | `false` | Enable AcoustID fingerprint lookups |
| `acoustid_api_key` | string or null | `null` | AcoustID API key |
| `request_timeout_secs` | int | `30` | Per-request timeout in seconds |
| `max_concurrent_requests` | int | `4` | Maximum concurrent API requests |

For provider-specific setup instructions, see the guides in [providers/](providers/).

---

## Environment Variable Overrides

API keys can be stored as environment variables instead of in the config file, which keeps secrets out of the settings file:

| Environment Variable | Config Field | Provider |
| -------------------- | ------------ | -------- |
| `MM_DISCOGS_TOKEN` | `providers.discogs_token` | Discogs |
| `MM_SPOTIFY_CLIENT_ID` | `providers.spotify_client_id` | Spotify |
| `MM_SPOTIFY_CLIENT_SECRET` | `providers.spotify_client_secret` | Spotify |
| `MM_TMDB_API_KEY` | `providers.tmdb_api_key` | TMDb |
| `MM_ACOUSTID_API_KEY` | `providers.acoustid_api_key` | AcoustID |

Environment variables always take priority over values in `settings.json5`.

You can set them in your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) or in a `.env` file placed next to the `settings.json5` file. The `.env` file is not created automatically — create it manually if you want to use it:

```bash
# ~/.config/MeedyaManager/.env
MM_SPOTIFY_CLIENT_ID=your_client_id_here
MM_SPOTIFY_CLIENT_SECRET=your_client_secret_here
MM_TMDB_API_KEY=your_tmdb_key_here
```

> Keep your `.env` file private. Do not commit it to version control.

---

## CLI Config Commands

```bash
# Show the current config (pretty-printed)
meedya config show

# Show as JSON (useful for scripting)
meedya config show --json

# Validate the config file and report any errors
meedya config validate

# Print the config file path
meedya config path

# Export current settings to a portable .mmprofile bundle
meedya config export --out ~/my-settings.mmprofile

# Import settings from a .mmprofile bundle
meedya config import ~/my-settings.mmprofile

# Reset config to defaults (with confirmation prompt)
meedya config reset
```

For export/import details, see [settings-export-import.md](settings-export-import.md).

---

> For troubleshooting configuration issues, see [troubleshooting.md](troubleshooting.md).
