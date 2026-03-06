# 🎶 Deezer Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers setting up the **Deezer** metadata provider in MeedyaManager. Deezer's public API requires no authentication, making it one of the simplest providers to use.

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

The Deezer provider uses the **Deezer Public API** to search the Deezer music catalog for track and album metadata. The API is freely accessible with no authentication required, providing reliable metadata including ISRC codes, track/disc positioning, duration, and high-resolution cover art.

**Key features:**

- Track and album search via the Deezer catalog
- ISRC code retrieval for precise track identification
- Track position and disc number data
- Duration information (in seconds)
- Static cover art up to 1000x1000 pixels (JPEG)
- No authentication required — fully public API

---

## Authentication

Deezer's public search API requires **no authentication**. There are no API keys, tokens, or accounts needed. MeedyaManager can use this provider immediately without any setup.

### No setup required

The Deezer provider is always available out of the box. Simply ensure it is enabled in your settings (it is enabled by default).

> **Note:** Deezer does offer authenticated API endpoints for user-specific data (playlists, favourites, etc.), but MeedyaManager only uses the public search endpoints which require no credentials.

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required for Deezer.

### Settings (`settings.json5`)

```json5
{
  providers: {
    deezer: {
      enabled: true,                    // Enable or disable this provider
      priority: 4,                      // Provider priority (lower = higher priority)
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `true` | Whether this provider is active |
| `priority` | `4` | Search priority relative to other providers |

---

## Available Data

The Deezer provider returns the following standard metadata fields:

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `track.title` | "Bohemian Rhapsody" |
| `artist` | `track.artist.name` | "Queen" |
| `album` | `track.album.title` | "A Night at the Opera" |
| `year` | `track.release_date` | "1975" |
| `track_num` | `track.track_position` | "11" |
| `disc_num` | `track.disk_number` | "1" |
| `isrc` | `track.isrc` | "GBUM71029604" |

### Search Syntax

MeedyaManager constructs Deezer search queries using field prefixes for precision:

```text
track:"Bohemian Rhapsody" artist:"Queen" album:"A Night at the Opera"
```

This targeted syntax produces more accurate results than simple keyword searches.

---

## Custom Tags

The following custom tags are stored in the file's metadata when matched:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_deezer_id` | Deezer track ID | `"3157894"` |
| `custom_deezer_url` | Deezer URL for the track | `"https://www.deezer.com/track/3157894"` |
| `custom_deezer_isrc` | ISRC code from Deezer | `"GBUM71029604"` |
| `custom_deezer_duration` | Duration in seconds | `"354"` |

These tags can be used in MeedyaManager rename templates and sorting rules:

```json5
rename_format: "{artist}/{album}/{track_num} - {title}.{extension}"
```

---

## Cover Art

Deezer provides static cover art in multiple resolutions. MeedyaManager uses the largest available:

| Type | Format | Resolution | Source |
| ---- | ------ | ---------- | ------ |
| **Static (front cover)** | JPEG | 1000x1000 | `album.cover_xl` |

Deezer cover art resolutions:

| Size Key | Resolution |
| -------- | ---------- |
| `cover_small` | 56x56 |
| `cover_medium` | 250x250 |
| `cover_big` | 500x500 |
| `cover_xl` | 1000x1000 |

MeedyaManager always uses `cover_xl` for the best quality and saves it as `FrontCover.jpg`.

---

## Troubleshooting

### "Deezer search returned 0 results"

**Possible causes:**
- The track may not be available in the Deezer catalog (regional availability varies)
- The search query may be too specific or contain unusual characters

**Solutions:**
1. Try searching on [deezer.com](https://www.deezer.com/) directly to confirm the track exists
2. Simplify the search — try with just title and artist (without album)
3. Check for unusual characters in the metadata that might interfere with the search syntax

### HTTP 429 — Rate limit exceeded

**Cause:** Deezer enforces rate limits on their public API (approximately 50 requests per 5 seconds).

**Solution:**
- MeedyaManager includes built-in rate limiting to prevent this
- If you encounter this during bulk operations, the rate limiter will automatically slow requests
- Wait a few seconds and retry

### HTTP 500 / 502 — Server error

**Cause:** Deezer API may be temporarily unavailable.

**Solution:**
- These are transient errors on Deezer's side
- MeedyaManager will automatically retry failed requests
- If persistent, check [Deezer's status page](https://status.deezer.com/) for outages

### Missing ISRC in results

**Cause:** Not all Deezer tracks include ISRC codes. Some catalogue entries, particularly older or regional releases, may lack this field.

**Solution:** This is expected. MeedyaManager will use ISRC data from other providers (Spotify, MusicBrainz, Apple Music) when Deezer does not provide it.

---

## Legal Notes

- The Deezer Public API is provided under the [Deezer API Terms of Use](https://developers.deezer.com/termsofuse)
- No account or API key is required for public search endpoints
- Deezer's API is free to use for non-commercial and personal projects
- Rate limits apply — MeedyaManager respects these automatically
- Cover art and metadata are the property of their respective rights holders
- MeedyaManager stores provider IDs and URLs as custom metadata tags for reference and linking; this does not imply endorsement by Deezer SA

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
