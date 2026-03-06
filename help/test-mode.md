# Test Mode (Safe Edit Mode)

> **(C) 2025-2026 MWBM Partners Ltd**

## What is Test Mode?

Test Mode is a safety feature that prevents MeedyaManager from modifying
your original media files. When enabled, all edit and tagging operations
create a **duplicate** file with a `_MeedyaManager` suffix instead of
overwriting the original.

For example, editing `track.mp3` in Test Mode creates
`track_MeedyaManager.mp3` with your changes, while `track.mp3` remains
untouched.

---

## When to Use Test Mode

- **First time using MeedyaManager** — verify that tagging and renaming
  produce the results you expect before committing to changes.
- **Testing new templates or rules** — preview the effect of complex rename
  rules on real files without risk.
- **Pre-release versions** — Test Mode is automatically enabled on
  pre-release builds to protect your library from potential bugs.
- **Batch operations** — enable Test Mode before a large batch edit, review
  the results, then commit or revert.

---

## Enabling and Disabling

### GUI (macOS / Windows / Linux)

Navigate to **Settings** and toggle **Test Mode** on or off.

### CLI

```bash
# Enable test mode
meedya config test-mode on

# Disable test mode
meedya config test-mode off

# Check current status
meedya config test-mode status
```

### Environment Variable

```bash
export MM_TEST_MODE=true
```

---

## How It Works

### When Test Mode is ON

1. You edit a file (e.g. set artist tag on `song.flac`)
2. MeedyaManager copies `song.flac` to `song_MeedyaManager.flac`
3. Tags are written to the copy — the original is untouched
4. The file pair is recorded in the **test-mode manifest**
5. The manifest persists across application sessions

### When You Disable Test Mode

If there are tracked test-mode files, you are prompted:

> **Keep only the tagged files?**
>
> - **Yes** — Delete the originals and rename the copies (remove the
>   `_MeedyaManager` suffix). Your edited files replace the originals.
> - **No** — Keep both the originals and the copies. The manifest is
>   cleared but no files are deleted.

This prompt applies to **all** test-mode files, not just those edited in
the current session. A "test mode session" spans from when you enable it
to when you disable it, even across multiple application launches.

---

## Managing Test Mode Files

### CLI Commands

```bash
# View all tracked test-mode files
meedya config test-mode status

# Commit: delete originals, rename copies to original names
meedya config test-mode commit

# Revert: keep both, clear the manifest
meedya config test-mode revert
```

### Manifest Location

The test-mode manifest is stored at:

| Platform | Path |
| -------- | ---- |
| Linux | `~/.config/meedyamanager/testmode_manifest.json` |
| macOS | `~/Library/Application Support/meedyamanager/testmode_manifest.json` |
| Windows | `%APPDATA%\meedyamanager\testmode_manifest.json` |

---

## Pre-release Version Behaviour

When you launch a **pre-release** version of MeedyaManager (e.g.
`1.3.0-beta.1`, `2.0.0-alpha`):

1. A notice appears: *"You are using a pre-release version of MeedyaManager"*
2. If a stable version is available, you are offered the option to update
3. **Test Mode is automatically enabled** with a notice:
   *"Test Mode has been enabled to protect your files"*

When you later upgrade to a **stable** release:

- If Test Mode is still enabled, you are notified and asked whether you
  would like to disable it
- You can choose to keep Test Mode on or turn it off (with the usual
  commit/revert prompt)

---

## File Naming Convention

| Original | Test Mode Copy |
| -------- | -------------- |
| `song.mp3` | `song_MeedyaManager.mp3` |
| `video.mkv` | `video_MeedyaManager.mkv` |
| `cover.jpg` | `cover_MeedyaManager.jpg` |
| `README` | `README_MeedyaManager` |

The suffix is always `_MeedyaManager`, inserted before the file extension.
Files without an extension have the suffix appended to the end.

---

## Tips

- Test Mode works with all edit operations: tag writing, cover art
  embedding, and cover art removal.
- You can enable and disable Test Mode as many times as you like — the
  manifest accumulates files across all sessions until you commit or revert.
- Use `meedya config test-mode status` to see exactly which files are
  tracked before committing.
- If you manually delete a test-mode copy, the commit step will skip that
  entry gracefully.
