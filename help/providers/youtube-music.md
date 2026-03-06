# ▶️ YouTube Music Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers setting up the **YouTube Music** metadata provider in MeedyaManager, including generating the authentication file, configuring environment variables, and understanding the limitations of this unofficial API integration.

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Configuration](#configuration)
4. [Available Data](#available-data)
5. [Custom Tags](#custom-tags)
6. [Cover Art](#cover-art)
7. [Troubleshooting](#troubleshooting)
8. [Legal Notes](#legal-notes)

---

## Overview

The YouTube Music provider communicates with YouTube Music's internal API via HTTP to search the YouTube Music catalog for track and album metadata. Because YouTube Music does not offer a public developer API, this provider relies on browser cookie extraction for authentication.

**Key features:**

- Track and album search via the YouTube Music catalog
- Video IDs for direct YouTube Music links
- Duration data
- Static cover art (JPEG thumbnails, resolution varies)
- Works with a free YouTube/Google account

**Limitations:**

- Unofficial API — may break without notice if YouTube changes their internal endpoints
- Requires periodic re-authentication when browser cookies expire
- Does not provide ISRC codes
- Responses may be slower than official APIs due to the unofficial nature of the access

---

## Authentication

YouTube Music authentication works by extracting cookies and headers from a logged-in YouTube Music browser session and saving them to a `headers_auth.json` file.

### Step-by-step setup

1. **Extract authentication headers from your browser**
   - Open [music.youtube.com](https://music.youtube.com) in your browser
   - Log in to your Google account
   - Open the browser Developer Tools (F12 or Cmd+Opt+I)
   - Go to the **Network** tab
   - Click on any request to `music.youtube.com`
   - Copy the following headers from the request:
     - `Cookie`
     - `X-Goog-AuthUser`
     - `Authorization` (if present)

2. **Create the auth file** — save the headers as `headers_auth.json`:
   ```json
   {
     "Cookie": "...",
     "X-Goog-AuthUser": "0",
     "Authorization": "..."
   }
   ```

3. **Move the auth file to a secure location**
   ```bash
   mv headers_auth.json ~/.config/MeedyaManager/yt_headers_auth.json
   ```

4. **Set the environment variable**
   ```bash
   echo 'YOUTUBE_MUSIC_HEADERS_AUTH=~/.config/MeedyaManager/yt_headers_auth.json' >> .env
   ```

---

## Configuration

### Environment Variables (`.env`)

Add the following to your `.env` file:

```env
# YouTube Music authentication file path
YOUTUBE_MUSIC_HEADERS_AUTH=/path/to/headers_auth.json
```

| Variable | Description |
| -------- | ----------- |
| `YOUTUBE_MUSIC_HEADERS_AUTH` | Absolute path to the `headers_auth.json` file containing browser session headers |

### Settings (`settings.json5`)

```json5
{
  providers: {
    youtube_music: {
      enabled: true,                    // Enable or disable this provider
      priority: 6,                      // Provider priority (lower = higher priority)
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `true` | Whether this provider is active |
| `priority` | `6` | Search priority relative to other providers |

---

## Available Data

The YouTube Music provider returns the following metadata fields:

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `song.title` | "Bohemian Rhapsody" |
| `artist` | `song.artists[0].name` | "Queen" |
| `album` | `song.album.name` | "A Night at the Opera" |

> **Note:** YouTube Music does not provide ISRC codes, track numbers, disc numbers, or release dates. For these fields, rely on other providers (Spotify, MusicBrainz, Apple Music).

---

## Custom Tags

The following custom tags are stored in the file's metadata when matched:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_youtube_music_id` | YouTube Music video ID | `"fJ9rUzIMcZQ"` |
| `custom_youtube_music_url` | YouTube Music URL for the track | `"https://music.youtube.com/watch?v=fJ9rUzIMcZQ"` |
| `custom_youtube_music_duration` | Duration in seconds | `"354"` |

These tags enable direct linking to YouTube Music content:

```json5
// The YouTube Music URL can be used for quick access to the track
rename_format: "{artist}/{album}/{title}.{extension}"
```

---

## Cover Art

YouTube Music provides static thumbnails in various resolutions:

| Type | Format | Resolution | Source |
| ---- | ------ | ---------- | ------ |
| **Static (thumbnail)** | JPEG | Varies (up to 720x720) | `song.thumbnails[-1]` (largest available) |

MeedyaManager selects the largest thumbnail from the available options and saves it as `FrontCover.jpg`.

> **Note:** YouTube Music thumbnail quality is generally lower than dedicated music services like Apple Music (3000x3000) or Deezer (1000x1000). For best cover art quality, prioritise those providers.

---

## Troubleshooting

### "YouTube Music headers_auth.json not found"

**Cause:** The authentication file path is incorrect or the file does not exist.

**Solutions:**
1. Verify the `YOUTUBE_MUSIC_HEADERS_AUTH` path in your `.env` file
2. Ensure the file was generated successfully: `ls -la /path/to/headers_auth.json`
3. Re-extract browser headers and regenerate the file if missing

### "Failed to initialize YouTube Music client"

**Cause:** The `headers_auth.json` file may be corrupted, expired, or in an incorrect format.

**Solutions:**

1. Re-extract browser headers and recreate the `headers_auth.json` file (see Authentication above)
2. Ensure you are logged in to a valid Google account when extracting cookies
3. Check that the JSON file is well-formed (verify it has valid JSON syntax)

### Searches returning empty results or HTTP errors

**Cause:** YouTube Music's internal API may have changed or your authentication cookies may have expired.

**Solutions:**
1. Re-generate `headers_auth.json` with fresh cookies
2. Check the MeedyaManager logs for specific error messages
3. Check for a MeedyaManager update if the API endpoints have changed
4. Note that YouTube Music API changes are out of MeedyaManager's control

### "Operation timed out" or slow responses

**Cause:** YouTube Music's API occasionally responds slowly. This can cause delays in batch lookups.

**Solution:** This is expected behaviour. Timeout is set to 30 seconds. If a lookup times out, MeedyaManager falls back to other enabled providers.

---

## Legal Notes

- YouTube Music does **not** provide an official public API for metadata search
- MeedyaManager accesses YouTube Music's internal API via reverse-engineered endpoints
- This integration may break at any time if YouTube changes their internal endpoints or authentication mechanisms
- Use of this provider may be subject to [YouTube's Terms of Service](https://www.youtube.com/t/terms)
- MeedyaManager includes this provider as a convenience, with the understanding that it is unofficial and best-effort
- Cover art and metadata are the property of their respective rights holders
- MeedyaManager stores provider IDs and URLs as custom metadata tags for reference only

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
