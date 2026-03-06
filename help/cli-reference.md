# CLI Reference — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

Complete reference for the `meedya` command-line interface.

---

## Table of Contents

1. [Global Flags](#global-flags)
2. [meedya debug](#meedya-debug)
3. [meedya scan](#meedya-scan)
4. [meedya watch](#meedya-watch)
5. [meedya edit](#meedya-edit)
6. [meedya lookup](#meedya-lookup)
7. [meedya rule](#meedya-rule)
8. [meedya config](#meedya-config)
9. [meedya service](#meedya-service)
10. [meedya export](#meedya-export)
11. [meedya serve](#meedya-serve)
12. [meedya report-bug](#meedya-report-bug)

---

## Global Flags

These flags are accepted by every `meedya` subcommand:

| Flag | Short | Description |
| ---- | ----- | ----------- |
| `--verbose` | `-v` | Increase log verbosity (repeat for more: `-v`, `-vv`, `-vvv`) |
| `--config <path>` | `-c` | Use a custom config file path |
| `--json` | | Emit machine-parseable JSON output |
| `--dry-run` | | Preview changes without modifying any files |

```bash
# Examples
meedya -v scan ~/Music
meedya --json lookup song.mp3
meedya --dry-run watch
meedya --config /etc/meedya/settings.json5 service status
```

---

## meedya debug

Inspect a single media file — display its classification, all embedded tags, and file properties.

```text
meedya debug <FILE> [OPTIONS]
```

| Argument / Flag | Description |
| --------------- | ----------- |
| `<FILE>` | Path to the media file to inspect |
| `--json` | Output as JSON |

### Examples

```bash
# Human-readable output
meedya debug ~/Music/song.mp3

# JSON output (for scripting)
meedya debug ~/Music/song.mp3 --json

# Verbose — include detailed parsing info
meedya -v debug ~/Music/song.mp3
```

### Sample Output

```text
File:         song.mp3
Media Group:  Audio
Format Class: MP3
Media Class:  Music
Quality Type: Lossy

Tags:
  Title:       Never Gonna Give You Up
  Artist:      Rick Astley
  Album:       Whenever You Need Somebody
  Track #:     1
  Year:        1987
  Genre:       Pop
  Duration:    3:32
  Bitrate:     256 kbps
```

---

## meedya scan

Scan a directory for media files and preview what renames would be applied.

```text
meedya scan [PATH] [OPTIONS]
```

| Argument / Flag | Description |
| --------------- | ----------- |
| `[PATH]` | Directory to scan (defaults to configured watch folders) |
| `--recursive` | Scan subdirectories (default: true) |
| `--dry-run` | Preview only — no files moved (default behaviour) |
| `--json` | Output rename preview as JSON |

### Examples

```bash
# Preview renames for a directory
meedya scan ~/Downloads/Media

# Scan non-recursively
meedya scan ~/Music --recursive=false

# Output as JSON
meedya scan ~/Music --json
```

---

## meedya watch

Monitor directories for new or changed media files and process them as they arrive.

```text
meedya watch [PATHS...] [OPTIONS]
```

| Argument / Flag | Description |
| --------------- | ----------- |
| `[PATHS...]` | Directories to watch (overrides config `watch.folders`) |
| `--dry-run` | Preview mode — log what would happen, no files moved |

### Examples

```bash
# Watch configured folders (from settings.json5)
meedya watch

# Watch a specific directory
meedya watch ~/Downloads/Media

# Safe preview mode
meedya watch --dry-run

# Verbose output showing each detected event
meedya -v watch
```

> **Note:** Press `Ctrl+C` to stop the watcher gracefully.

---

## meedya edit

Edit metadata tags and cover art on a media file.

```text
meedya edit <FILE> [OPTIONS]
```

| Argument / Flag | Description |
| --------------- | ----------- |
| `<FILE>` | Path to the media file to edit |
| `--tag <Key=Value>` | Set a tag (repeatable) |
| `--remove-tag <Key>` | Remove a tag by name |
| `--cover <PATH>` | Embed a cover art image |
| `--remove-cover` | Remove embedded cover art |
| `--dry-run` | Show what would change without writing |

### Examples

```bash
# Set multiple tags at once
meedya edit song.mp3 --tag "Artist=Rick Astley" --tag "Title=Never Gonna Give You Up"

# Embed cover art
meedya edit song.mp3 --cover cover.jpg

# Remove a tag
meedya edit song.mp3 --remove-tag Comment

# Preview changes without writing
meedya edit song.mp3 --tag "Year=1987" --dry-run
```

---

## meedya lookup

Search metadata providers for a media file and optionally apply the best match.

```text
meedya lookup <FILE> [OPTIONS]
meedya lookup --list-providers
```

| Argument / Flag | Description |
| --------------- | ----------- |
| `<FILE>` | Media file to look up |
| `--providers <list>` | Comma-separated list of providers to query |
| `--auto` | Apply the best match automatically (confidence >= 80%) |
| `--apply <N>` | Apply result number N from the match list |
| `--dry-run` | Show what would change without writing tags |
| `--list-providers` | List all configured providers and their status |
| `--json` | Output results as JSON |

### Examples

```bash
# Look up all enabled providers
meedya lookup song.mp3

# Use specific providers only
meedya lookup song.mp3 --providers musicbrainz,spotify

# Auto-apply the best match
meedya lookup song.mp3 --auto

# Preview before writing
meedya lookup song.mp3 --apply 1 --dry-run

# JSON output for scripting
meedya lookup song.mp3 --json

# Show all providers and status
meedya lookup --list-providers
```

---

## meedya rule

Validate templates, list available tags, and test rules against files.

```text
meedya rule <SUBCOMMAND>
```

### meedya rule test

Test a template against a file to preview the output.

```text
meedya rule test --template <TEMPLATE> <FILE>
```

```bash
meedya rule test --template "<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>" song.mp3
```

### meedya rule validate

Validate a template for syntax errors.

```text
meedya rule validate --template <TEMPLATE>
```

```bash
meedya rule validate --template "<Artist>/<Album>/<Title>.<Ext>"
```

### meedya rule list-tags

List all available tags and their descriptions.

```text
meedya rule list-tags
meedya rule list-tags --json
```

---

## meedya config

Manage MeedyaManager configuration.

```text
meedya config <SUBCOMMAND>
```

| Subcommand | Description |
| ---------- | ----------- |
| `show` | Print the current configuration |
| `validate` | Check the config file for errors |
| `path` | Print the config file path |
| `export` | Export settings to a `.mmprofile` bundle |
| `import` | Import settings from a `.mmprofile` bundle |
| `reset` | Reset config to defaults (with confirmation) |

### Examples

```bash
# Show current config
meedya config show
meedya config show --json

# Validate for errors
meedya config validate

# Show config file path
meedya config path

# Export to a portable bundle
meedya config export --out ~/my-settings.mmprofile

# Import from a bundle (preview first)
meedya config import ~/my-settings.mmprofile --dry-run
meedya config import ~/my-settings.mmprofile

# Reset to defaults
meedya config reset
```

For export/import details, see [settings-export-import.md](settings-export-import.md).

---

## meedya service

Manage the MeedyaManager background service.

```text
meedya service <SUBCOMMAND>
```

| Subcommand | Description |
| ---------- | ----------- |
| `install` | Register with the OS service manager (systemd / launchd / Windows Service) |
| `uninstall` | Remove the service registration |
| `start` | Start the service |
| `stop` | Stop the service |
| `status` | Display current service status |

### Examples

```bash
# Install and start
meedya service install
meedya service start

# Check status
meedya service status
meedya service status --json   # machine-readable

# Stop and remove
meedya service stop
meedya service uninstall

# Install with a custom binary path
meedya service install --bin-path /opt/meedya/bin/meedya
```

For full service setup instructions, see [background-service.md](background-service.md).

---

## meedya export

Export media library metadata to a database.

```text
meedya export [OPTIONS]
```

| Flag | Description |
| ---- | ----------- |
| `--format <fmt>` | Database format: `sqlite`, `mysql`, `postgres`, `sqlserver` |
| `--out <path>` | Output path (SQLite) or connection string (network databases) |
| `--dry-run` | Count files and validate connection without writing |
| `--json` | Output progress and summary as JSON |

### Examples

```bash
# Export to SQLite
meedya export --format sqlite --out ~/library.db

# Export to PostgreSQL
meedya export --format postgres --out "postgresql://user:pass@localhost/meedya"

# Dry-run (validate only)
meedya export --format sqlite --out ~/library.db --dry-run
```

---

## meedya serve

Start the HTTPS media server with JWT authentication.

```text
meedya serve [OPTIONS]
```

| Flag | Description |
| ---- | ----------- |
| `--port <port>` | HTTPS port (default: 8443) |
| `--cert <path>` | TLS certificate file (PEM) |
| `--key <path>` | TLS private key file (PEM) |
| `--bind <addr>` | Bind address (default: 0.0.0.0) |
| `--dry-run` | Validate config without starting the server |

### Examples

```bash
# Start with custom cert
meedya serve --port 8443 --cert /etc/meedya/cert.pem --key /etc/meedya/key.pem

# Validate config
meedya serve --dry-run
```

---

## meedya report-bug

Generate a diagnostic bug report with system info, health check results, and recent log excerpts.

```text
meedya report-bug [OPTIONS]
```

| Flag | Description |
| ---- | ----------- |
| `--out <path>` | Output path for the report file (default: current directory) |
| `--json` | Emit the report as JSON instead of a text file |

### Example

```bash
meedya report-bug
# Creates: meedya-bug-report-2026-03-06.txt
```

Attach the generated file when opening an issue at [GitHub Issues](https://github.com/MWBMPartners/MeedyaManager/issues).
