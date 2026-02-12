# 🔧 Troubleshooting — MediaMancer

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

This guide covers common issues, error messages, and their solutions.

---

## 📋 Table of Contents

1. [Installation Issues](#installation-issues)
2. [MediaInfo Problems](#mediainfo-problems)
3. [Watcher Issues](#watcher-issues)
4. [Configuration Errors](#configuration-errors)
5. [Rename/Move Issues](#renamemove-issues)
6. [Platform-Specific Issues](#platform-specific-issues)
7. [Getting Help](#getting-help)

---

## Installation Issues

### "ModuleNotFoundError: No module named 'pymediainfo'"

**Cause:** Dependencies not installed.

**Solution:**

```bash
pip install -r requirements.txt
```

If using a virtual environment, make sure it's activated first:

```bash
source venv/bin/activate  # macOS/Linux
venv\Scripts\activate     # Windows
```

### "Python version X.X is not supported"

**Cause:** MediaMancer requires Python 3.10 or newer.

**Solution:** Install Python 3.11+ from [python.org](https://www.python.org/downloads/).

### Checksum verification fails

**Cause:** Downloaded file may be corrupted or tampered with.

**Solution:**

1. Re-download the release archive
2. Re-run verification:

```bash
python utils/verify_checksum.py <archive> <archive>.sha256
```

3. If it still fails, report it as a [security issue](https://github.com/MWBMPartners/MediaMancer/issues)

---

## MediaInfo Problems

### "MediaInfo library not found"

**Cause:** MediaInfo is not installed or not in the system PATH.

**Solutions by platform:**

**macOS:**

```bash
brew install mediainfo
```

**Linux (Debian/Ubuntu):**

```bash
sudo apt install mediainfo libmediainfo-dev
```

**Linux (Fedora):**

```bash
sudo dnf install mediainfo libmediainfo-devel
```

**Windows:**

1. Download from [mediaarea.net](https://mediaarea.net/en/MediaInfo/Download/Windows)
2. Run the installer
3. Ensure the install directory is in your system PATH

### "No metadata extracted" for a file

**Possible causes:**

- File is corrupted or incomplete (still being copied)
- File format not recognised by MediaInfo
- File has no embedded metadata tags

**Solutions:**

1. Verify the file plays correctly in a media player
2. Check MediaInfo directly: `mediainfo path/to/file`
3. If the file is being copied, wait and retry — MediaMancer's retry queue handles this automatically

---

## Watcher Issues

### Watcher not detecting new files

**Possible causes:**

- Watch path doesn't exist
- Insufficient permissions
- watchdog library issue

**Solutions:**

1. Check your `config/settings.json5` — verify `watch_paths` are correct
2. Check logs at `logs/watcher_events.log` for warnings
3. Ensure the directories exist and are readable:

```bash
ls -la ~/Downloads/Media
```

4. Try polling mode if watchdog has issues:

```json5
watch_mode: "polling"
```

### "File disappeared before processing"

**Cause:** The file was moved or deleted between detection and processing.

**Solution:** This is usually harmless — it means another application moved the file first. If it happens frequently, check for conflicts with other file management tools.

### High CPU usage from watcher

**Cause:** Watching too many directories or using polling mode on large directory trees.

**Solutions:**

1. Reduce the number of `watch_paths`
2. Ensure `watch_mode` is set to `"watchdog"` (not `"polling"`)
3. Avoid watching the entire home directory — be specific

---

## Configuration Errors

### "JSON5 parse error"

**Cause:** Syntax error in `config/settings.json5`.

**Solution:** Check for:

- Missing commas between items
- Unmatched brackets or braces
- Invalid escape sequences in strings

Use a JSON5-aware editor or validator to find the issue.

### "Config key not found"

**Cause:** A required configuration key is missing.

**Solution:** Ensure your `settings.json5` includes all required keys. Compare against the [configuration reference](configuration.md).

### ".env file not loaded"

**Cause:** `.env` file doesn't exist or is in the wrong location.

**Solution:**

```bash
cp .env.example .env
# Edit .env with your values
```

The `.env` file must be in the project root directory.

---

## Rename/Move Issues

### "Simulated rename" but no actual files moved

**Cause:** Simulation mode is enabled (this is the default, safe behaviour).

**Solution:** To actually move files, run with simulation disabled:

```bash
python cli/runner.py --simulate-off
```

Or set in config:

```json5
simulate_watcher: false
```

### File in use — "queued for retry"

**Cause:** The file is open in another application.

**Solution:** This is expected behaviour. MediaMancer will:

1. Detect the file is locked
2. Queue it for later processing
3. Retry when the file is no longer in use

No action needed — just close the other application when ready.

### Filename contains invalid characters

**Cause:** Metadata tags contain characters not allowed in file paths.

**Solution:** Configure `filename_replacements` in `settings.json5`:

```json5
filename_replacements: {
  "/": "-",
  ":": "-",
  "*": "",
  "?": ""
}
```

---

## Platform-Specific Issues

### macOS: "Operation not permitted"

**Cause:** macOS security restrictions (especially on system directories or external drives).

**Solution:**

1. Go to **System Preferences > Privacy & Security > Files and Folders**
2. Grant access to the Terminal or Python application
3. For full disk access: **Privacy & Security > Full Disk Access**

### Windows: Long path errors

**Cause:** Windows has a 260-character path limit by default.

**Solution:**

1. Enable long paths in Windows: Run as Administrator:

```cmd
reg add "HKLM\SYSTEM\CurrentControlSet\Control\FileSystem" /v LongPathsEnabled /t REG_DWORD /d 1
```

2. Use shorter rename templates to avoid deep nesting

### Linux: Permission denied on mounted drives

**Cause:** External or network drives may have restrictive mount permissions.

**Solution:**

1. Check mount options: `mount | grep <drive>`
2. Remount with appropriate permissions if needed
3. Run MediaMancer as a user with access to the mount point

---

## Getting Help

If your issue isn't covered here:

1. **Check the logs:** `logs/watcher_events.log` and `logs/rename_preview.log`
2. **Check the FAQ:** [faq.md](faq.md)
3. **Open an issue:** [GitHub Issues](https://github.com/MWBMPartners/MediaMancer/issues/new)
   - Include your OS, Python version, and relevant log output
   - Use the appropriate issue template (bug report, feature request, etc.)

---

> 📝 *This guide is updated as new issues are discovered and resolved.*
