# Settings Export and Import — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager supports exporting your entire configuration to a portable `.mmprofile` bundle, which can be imported on another device or platform. This is useful for:

- Migrating from one computer to another
- Sharing a configuration with a team
- Backing up your settings before a major change
- Moving between platforms (e.g. Linux to macOS)

---

## Table of Contents

1. [Export Settings](#export-settings)
2. [Import Settings](#import-settings)
3. [Profile Format](#profile-format)
4. [Cross-Platform Path Handling](#cross-platform-path-handling)
5. [API Keys and Secrets](#api-keys-and-secrets)

---

## Export Settings

### Via CLI

```bash
# Export to a .mmprofile bundle
meedya config export --out ~/my-settings.mmprofile

# Include a human-readable name for the profile
meedya config export --out ~/my-settings.mmprofile --name "Home Mac"

# Include API keys in the bundle (encrypted — see API Keys section)
meedya config export --out ~/my-settings.mmprofile --include-secrets

# Preview what would be exported without writing the file
meedya config export --out ~/my-settings.mmprofile --dry-run
```

### Via GUI

1. Open **Settings** (gear icon or `Cmd+,` / `Ctrl+,`)
2. Click **Export Settings...**
3. Choose a save location
4. Optionally check **Include API keys** to bundle provider credentials

---

## Import Settings

### Via CLI

```bash
# Preview what the import would change (no files written)
meedya config import ~/my-settings.mmprofile --dry-run

# Import, merging with existing settings
meedya config import ~/my-settings.mmprofile

# Replace current settings entirely (prompts for confirmation)
meedya config import ~/my-settings.mmprofile --mode replace

# Replace without confirmation prompt
meedya config import ~/my-settings.mmprofile --mode replace --yes
```

**Import modes:**

| Mode | Behaviour |
| ---- | --------- |
| `merge` (default) | Add new settings from the bundle; existing settings take priority |
| `replace` | Replace the entire current config with the bundle's settings |

### Via GUI

1. Open **Settings**
2. Click **Import Settings...**
3. Select a `.mmprofile` file
4. Choose **Merge** or **Replace**
5. Review the diff and click **Apply**

---

## Profile Format

A `.mmprofile` file is a standard JSON file (not ZIP) containing a versioned `SettingsBundle` struct:

```json
{
  "version": "1.2.0",
  "profile_name": "Home Mac",
  "created_at": "2026-03-06T12:00:00Z",
  "platform": "macos",
  "config": {
    "app_name": "MeedyaManager",
    "dry_run": false,
    "watch": { "..." },
    "rename": { "..." },
    "logging": { "..." },
    "providers": { "..." }
  },
  "custom_filetypes": null,
  "custom_tags": null
}
```

| Field | Description |
| ----- | ----------- |
| `version` | MeedyaManager version that created the bundle |
| `profile_name` | Human-readable name set at export time |
| `created_at` | UTC timestamp of export |
| `platform` | Source platform (`macos`, `linux`, `windows`) |
| `config` | Full `AppConfig` — all settings sections |
| `custom_filetypes` | Custom filetype registry entries (if any) |
| `custom_tags` | Custom tag definitions (if any) |

---

## Cross-Platform Path Handling

Watch folders and output directories are stored using portable path tokens so that a profile exported on macOS can be imported on Windows or Linux:

| Token | macOS | Windows | Linux |
| ----- | ----- | ------- | ----- |
| `{HOME}` | `/Users/you` | `C:\Users\you` | `/home/you` |
| `{DESKTOP}` | `~/Desktop` | `~\Desktop` | `~/Desktop` |
| `{DOWNLOADS}` | `~/Downloads` | `~\Downloads` | `~/Downloads` |
| `{MUSIC}` | `~/Music` | `~\Music` | `~/Music` |
| `{VIDEOS}` | `~/Movies` | `~\Videos` | `~/Videos` |
| `{DOCUMENTS}` | `~/Documents` | `~\Documents` | `~/Documents` |

When you export, absolute paths are automatically tokenized. When you import, tokens are expanded to the correct paths for the current platform.

**Example:** A watch folder of `/Users/alice/Downloads/Media` on macOS becomes `{HOME}/Downloads/Media` in the bundle, and expands to `C:\Users\alice\Downloads\Media` when imported on Windows.

---

## API Keys and Secrets

By default, API keys are **not included** in exported profiles (they remain in your local `settings.json5` or environment variables). This is the safe default — `.mmprofile` files are plain JSON and can accidentally end up in cloud storage or email.

To include secrets in the export:

```bash
meedya config export --out ~/my-settings.mmprofile --include-secrets
```

When `--include-secrets` is used:

- API keys and tokens are included in the `config.providers` section of the bundle
- The file is **not encrypted** — treat it like a password file
- Store it securely and do not share it publicly

When importing a profile that contains secrets, MeedyaManager will prompt before applying them:

```
This profile contains API credentials. Apply them? [y/N]
```

Use `--yes` to skip the prompt in automated/scripting scenarios.
