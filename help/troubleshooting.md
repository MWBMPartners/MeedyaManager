# Troubleshooting — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers common issues, error messages, and their solutions.

---

## Table of Contents

1. [Installation Issues](#installation-issues)
2. [Configuration Errors](#configuration-errors)
3. [Watcher Issues](#watcher-issues)
4. [Rename and Move Issues](#rename-and-move-issues)
5. [Metadata and Tag Issues](#metadata-and-tag-issues)
6. [Provider and Lookup Issues](#provider-and-lookup-issues)
7. [Background Service Issues](#background-service-issues)
8. [Platform-Specific Issues](#platform-specific-issues)
9. [Generating a Bug Report](#generating-a-bug-report)

---

## Installation Issues

> **There is no packaged release yet.** The only GitHub release is the pre-rename
> "MetaMancer v1.0-M1" pre-release from 2025-06-16 — there is no current `.msix`, `.dmg`,
> `.deb`, `.rpm`, Flatpak, Snap, or AppImage to download or install. Everything in this section
> assumes you built MeedyaManager from source per [getting-started.md](getting-started.md).

### "meedya: command not found"

**Cause:** The `meedya` binary is not in your PATH.

**Solution:** Run `cargo install --path crates/mm-cli` from the repo root to install the binary,
or add `~/.cargo/bin` to your PATH:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

Alternatively, run the freshly built binary directly:

```bash
target/release/meedya --help
```

---

## Configuration Errors

### "Failed to load configuration: JSON5 parse error"

**Cause:** Syntax error in `settings.json5`.

**Solution:** Common JSON5 mistakes to look for:

- Trailing comma after the last item in an array or object
- Unmatched `{`, `}`, `[`, or `]`
- Unescaped backslashes in Windows paths (use `\\` or forward slashes)
- Smart/curly quotes (`"`) instead of straight quotes (`"`)

There is no `meedya config validate` subcommand — it does not exist. Instead, check your config
by loading it:

```bash
meedya config show
```

This will surface a parse error if the file is invalid.

### "Failed to initialise: ..."

**Cause:** MeedyaManager could not start due to a configuration or environment error.

**Solution:** Run with verbose logging to see the full error:

```bash
meedya -vv config show
```

### Config changes are not taking effect

**Cause:** The running watcher process is using the config that was loaded at startup.

**Solution:** Restart MeedyaManager (or the background service) after editing the config:

```bash
meedya service stop
meedya service start
```

---

## Watcher Issues

### Watcher not detecting new files

**Possible causes:**

- Watch folders not configured
- Watch folders do not exist
- Insufficient read permissions
- Native file system events not supported on the current volume (e.g. network shares, Docker volumes)

**Solutions:**

1. Verify your `folders` list in `settings.json5` — the directories must exist:

   ```bash
   meedya config show
   ```

2. Check the watcher log for warnings:

   ```bash
   meedya -v watch --dry-run
   ```

3. On network mounts or Docker volumes, switch to polling mode by increasing `poll_interval_secs`:

   ```json5
   watch: {
     poll_interval_secs: 10
   }
   ```

### "File disappeared before processing"

**Cause:** The file was moved or deleted by another application between detection and processing.

**Solution:** This is harmless. If it happens frequently, check for conflicts with other file management tools running concurrently.

### High CPU usage from watcher

**Solutions:**

1. Be specific about watch folders — avoid watching your entire home directory
2. Use `exclude_extensions` to skip file types you don't need processed
3. Increase `debounce_ms` to reduce notification frequency for rapidly changing folders

---

## Rename and Move Issues

> ### ⚠️ Before using `meedya scan --execute`
>
> `scan --execute` has a known data-loss bug — issue
> [#201](https://github.com/MWBMPartners/MeedyaManager/issues/201). It only checks whether a
> destination file already exists on disk; it does **not** check whether two files in the same
> scan resolve to the same destination path. If your template collapses two different source
> files onto the same name, the second overwrite silently replaces the first with no prompt and
> no backup. Always inspect the full preview (no `--execute`, or `--dry-run`) for duplicate
> destination paths first. See [cli-reference.md](cli-reference.md#meedya-scan).

### "Simulated rename" — no files actually moved

**Cause:** Dry-run mode is active (the default for safety).

**Solution:** Remove `--dry-run` from the command, or set `dry_run: false` in `settings.json5`:

```bash
meedya watch          # live mode (moves files)
meedya watch --dry-run  # preview only
```

### File is processed but not renamed as expected

**Solution:**

1. Test your template against the file:

   ```bash
   meedya rule test "<Artist>/<Album>/<Title>" path/to/file.mp3
   ```

2. Inspect the file's actual tag values:

   ```bash
   meedya debug path/to/file.mp3
   ```

3. Check `missing_tag_mode` — if set to `"empty"`, missing tags produce blank path segments

### Conflict: file already exists at destination

**Cause:** A file with the same name already exists in the output directory.

**Solution:** Change `conflict_strategy` in `settings.json5`:

```json5
rename: {
  conflict_strategy: "rename"   // append a counter: "Song (1).mp3"
}
```

Options: `"skip"` (default), `"overwrite"`, `"rename"`, `"ask"` (GUI only).

### File in use by another application

**Status: not yet implemented.** There is no file-lock detection or retry-queue in
`crates/mm-core/src/watcher` — a file being written by another application is not specially
detected or deferred. If you see corrupted reads on files that are still being downloaded,
avoid watching directories where writes are actively in progress until this is addressed.

---

## Metadata and Tag Issues

### "No tags found" for a file

**Possible causes:**

- The file has no embedded metadata tags
- The file format is supported but the tag format is unusual

**Solution:**

1. Inspect the file:

   ```bash
   meedya debug path/to/file.mp3
   ```

2. Edit tags manually if needed:

   ```bash
   meedya edit path/to/file.mp3 --set "Artist=My Artist" --set "Title=My Title"
   ```

### Cover art not embedded after lookup

**This is expected right now.** `meedya lookup` is a stub — it does not query any provider or
write any tag yet (see [cli-reference.md](cli-reference.md#meedya-lookup)). If you need cover
art embedded today, use `meedya edit --cover <path>` with an image you already have.

---

## Provider and Lookup Issues

> **`meedya lookup` is a stub** — it does not query any provider yet, so "no results from a
> provider" is expected for every query right now, not a misconfiguration. There is also no
> `--list-providers` flag. See [cli-reference.md](cli-reference.md#meedya-lookup). The notes
> below describe how provider configuration will matter once lookup is wired up, and are useful
> if you are working on the `mm-providers` code directly.

### Enabling / configuring a provider

1. Enable the provider in `settings.json5`:

   ```json5
   providers: {
     spotify_enabled: true,
     spotify_client_id: "...",
     spotify_client_secret: "..."
   }
   ```

2. Or set via environment variable:

   ```bash
   export MM_SPOTIFY_CLIENT_ID=your_id
   export MM_SPOTIFY_CLIENT_SECRET=your_secret
   ```

3. For the 13 other providers that need a key (not covered by the `providers:` block above),
   the pattern is `MM_<PROVIDER>_<KEY>` — see [configuration.md](configuration.md#real-provider-credentials).

### "Rate limited by provider"

**Cause:** Too many requests sent to the provider's API in a short period.

**Solution:** MeedyaManager has built-in rate limiting per provider. If you're hitting limits during batch operations, reduce `max_concurrent_requests`:

```json5
providers: {
  max_concurrent_requests: 2
}
```

### Network timeout during lookup

**Solution:** Increase `request_timeout_secs`:

```json5
providers: {
  request_timeout_secs: 60
}
```

---

## Background Service Issues

### Service not starting

**Solution:**

```bash
# Check service status
meedya service status

# View service logs
meedya -vv watch --dry-run   # run interactively to see startup errors
```

Platform-specific checks:

```bash
# Linux (systemd)
systemctl --user status meedyamanager
journalctl --user -u meedyamanager -n 50

# macOS (launchd)
launchctl list | grep meedyamanager
log show --predicate 'subsystem == "com.mwbm.meedyamanager"' --last 1h
```

See [background-service.md](background-service.md) for full service setup instructions.

---

## Platform-Specific Issues

### macOS: "Operation not permitted"

**Cause:** macOS privacy restrictions prevent access to monitored directories.

**Solution:**

1. Open **System Settings > Privacy & Security > Files and Folders**
2. Grant MeedyaManager access to the relevant directories
3. For external drives or full disk access: **Privacy & Security > Full Disk Access**

### Windows: Long path errors

**Cause:** Windows enforces a 260-character path limit by default.

**Solution:** Enable long path support (requires administrator):

```cmd
reg add "HKLM\SYSTEM\CurrentControlSet\Control\FileSystem" /v LongPathsEnabled /t REG_DWORD /d 1
```

Then reboot. Alternatively, use shorter rename templates to avoid deeply nested output paths.

### Linux: "inotify watch limit reached"

**Cause:** The kernel's inotify watch limit is too low for a large directory tree.

**Solution:**

```bash
# Temporary (until reboot)
sudo sysctl fs.inotify.max_user_watches=524288

# Permanent
echo "fs.inotify.max_user_watches=524288" | sudo tee /etc/sysctl.d/40-meedyamanager.conf
sudo sysctl -p /etc/sysctl.d/40-meedyamanager.conf
```

### Linux: "Permission denied" on mounted drives

**Cause:** External or network drives mounted with restrictive permissions.

**Solution:**

```bash
# Check mount options
mount | grep <drivename>

# Remount with user-accessible permissions (example for ext4)
sudo mount -o remount,uid=$(id -u),gid=$(id -g) /mnt/mydrive
```

---

## Generating a Bug Report

MeedyaManager has a built-in bug report generator that includes system information, health check results, and log excerpts:

```bash
meedya report-bug
```

This produces a `meedya-bug-report-<date>.txt` file in your current directory. When opening an issue on GitHub, attach this file to help us diagnose the problem quickly.

**GitHub Issues:** [github.com/MWBMPartners/MeedyaManager/issues](https://github.com/MWBMPartners/MeedyaManager/issues)
