# Settings Export and Import — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager can export your configuration to a portable `.mmprofile` bundle and import it
back — on the same machine or a different one. This is useful for:

- Migrating from one computer to another
- Backing up your settings before a major change
- Moving between platforms (e.g. Linux to macOS)

---

## Table of Contents

1. [Export Settings](#export-settings)
2. [Import Settings](#import-settings)
3. [Profile Format](#profile-format)
4. [API Keys and Secrets](#api-keys-and-secrets)

---

## Export Settings

```bash
meedya config export ~/my-settings.mmprofile
```

That single positional path is the entire command — there is no `--out`, `--name`, or
`--include-secrets` flag (`crates/mm-cli/src/commands/config_cmd.rs:41-45`). To preview the
export without writing the file, use the **global** `--dry-run` flag *before* the subcommand:

```bash
meedya --dry-run config export ~/my-settings.mmprofile
```

---

## Import Settings

```bash
meedya config import ~/my-settings.mmprofile
```

Again, one positional path — there is no `--mode`, `merge`/`replace` choice, and no `--yes`
flag. Import always **fully overwrites** your current `settings.json5` (and, if the bundle
carries them, your `filetypes.json5`/`tags.json5` overrides) with the bundle's contents —
there is no merge behaviour and no confirmation prompt
(`crates/mm-core/src/settings_bundle.rs:193-238`, doc comment: "Existing files are
overwritten"). Preview with the global `--dry-run` flag first if you want to check the bundle
before committing to it:

```bash
meedya --dry-run config import ~/my-settings.mmprofile
```

MeedyaManager must be restarted after an import for the new settings to take effect.

---

## Profile Format

A `.mmprofile` file is plain JSON (not a ZIP) holding a `SettingsBundle`
(`crates/mm-core/src/settings_bundle.rs:47-62`) with exactly these five fields:

```json
{
  "version": "1.3.0",
  "exported_at": "2026-03-06T12:00:00Z",
  "settings": {
    "app_name": "MeedyaManager",
    "dry_run": false,
    "test_mode": false,
    "watch": { "...": "..." },
    "rename": { "...": "..." },
    "logging": { "...": "..." },
    "providers": { "...": "..." }
  },
  "custom_filetypes": null,
  "custom_tags": null
}
```

| Field | Description |
| ----- | ----------- |
| `version` | MeedyaManager version that created the bundle (`CARGO_PKG_VERSION` at export time) |
| `exported_at` | UTC timestamp of export (RFC 3339) |
| `settings` | The full `AppConfig` — every settings section |
| `custom_filetypes` | Raw contents of your `filetypes.json5` override file, if one exists; otherwise `null` |
| `custom_tags` | Raw contents of your `tags.json5` override file, if one exists; otherwise `null` |

There is no `profile_name` or `platform` field, and there is **no cross-platform path
tokenisation** — watch folders and output directories are stored as plain absolute paths
exactly as they appear in your local `settings.json5`. Importing a bundle exported on one
platform onto another (e.g. macOS → Windows) will carry over paths that don't exist on the
new machine, and you will need to fix them by hand afterwards.

---

## API Keys and Secrets

There is no opt-in/opt-out for secrets: `settings` is the complete `AppConfig`, which includes
`providers` — so **any API keys you have configured are always included** in the exported
bundle, with no encryption. `Dev_Notes.md` correctly warns that bundles "may contain API
keys", and the CLI prints the same warning after a successful export:

```text
Warning: this bundle may contain API keys — keep it private.
```

There is no `--include-secrets` flag to opt in (it's unconditional) and no confirmation prompt
on import. Treat every `.mmprofile` file as sensitive — store it securely and never share it
publicly or commit it to a repository.
