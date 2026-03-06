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

### "meedya: command not found"

**Cause:** The `meedya` binary is not in your PATH.

**Solutions:**

- **Release package:** Re-run the installer. The installer adds `meedya` to your PATH automatically.
- **From source:** Run `cargo install --path crates/mm-cli` to install the binary, or add `~/.cargo/bin` to your PATH:

  ```bash
  echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
  source ~/.bashrc
  ```

- **Linux AppImage:** Make the AppImage executable and place it somewhere on your PATH:

  ```bash
  chmod +x MeedyaManager-*.AppImage
  mv MeedyaManager-*.AppImage ~/.local/bin/meedya
  ```

### Checksum verification fails after download

**Cause:** The downloaded file is corrupted or incomplete.

**Solution:** Re-download the release from [GitHub Releases](https://github.com/MWBMPartners/MeedyaManager/releases) and verify the SHA256 checksum published on the release page:

```bash
# Linux / macOS
sha256sum -c MeedyaManager-linux-x86_64.tar.gz.sha256

# macOS (alternative)
shasum -a 256 -c MeedyaManager-macos-arm64.tar.gz.sha256

# Windows (PowerShell)
Get-FileHash MeedyaManager-windows-x64.msix -Algorithm SHA256
```

If verification still fails, report it as a [security issue](https://github.com/MWBMPartners/MeedyaManager/security).

---

## Configuration Errors

### "Failed to load configuration: JSON5 parse error"

**Cause:** Syntax error in `settings.json5`.

**Solution:** Common JSON5 mistakes to look for:

- Trailing comma after the last item in an array or object
- Unmatched `{`, `}`, `[`, or `]`
- Unescaped backslashes in Windows paths (use `\\` or forward slashes)
- Smart/curly quotes (`"`) instead of straight quotes (`"`)

Validate your config:

```bash
meedya config validate
```

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
   meedya rule test --template "<Artist>/<Album>/<Title>" path/to/file.mp3
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

### File in use — "queued for retry"

**Cause:** The file is open in another application (e.g. being written by a download client).

**Solution:** This is expected behaviour. MeedyaManager detects the lock, queues the file, and retries automatically. No action required — just wait for the other application to finish.

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
   meedya edit path/to/file.mp3 --tag "Artist=My Artist" --tag "Title=My Title"
   ```

### Cover art not embedded after lookup

**Cause:** The provider returned a result but cover art download was not enabled, or the image exceeded the configured maximum resolution.

**Solution:** Check your `providers` config and ensure `lookup` output confirms art was downloaded. Re-run with `-v` for detail:

```bash
meedya -v lookup path/to/file.mp3
```

---

## Provider and Lookup Issues

### "Provider not configured" or no results from a provider

**Cause:** The provider is disabled or missing credentials.

**Solution:**

1. Check which providers are active:

   ```bash
   meedya lookup --list-providers
   ```

2. Enable the provider in `settings.json5`:

   ```json5
   providers: {
     spotify_enabled: true,
     spotify_client_id: "...",
     spotify_client_secret: "..."
   }
   ```

3. Or set via environment variable:

   ```bash
   export MM_SPOTIFY_CLIENT_ID=your_id
   export MM_SPOTIFY_CLIENT_SECRET=your_secret
   ```

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
log show --predicate 'subsystem == "ltd.mwbm.meedyamanager"' --last 1h
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
