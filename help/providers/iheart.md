# ❤️ iHeart Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers setting up the **iHeart** (iHeartRadio) metadata provider in MeedyaManager. iHeart uses undocumented public API endpoints that require no authentication, making it a zero-configuration provider — though with the caveat that these endpoints may change without notice.

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

The iHeart provider uses **undocumented public endpoints** from the iHeartRadio API to search for track metadata. iHeartRadio is primarily a streaming radio platform, but their search API provides basic song metadata including title, artist, album, cover art, and in some cases lyrics.

This provider is classified as **best-effort** — it works reliably as of the time of writing, but because the endpoints are undocumented, they may change, move, or be restricted by iHeartMedia at any time without prior notice.

**Key features:**

- Track search via undocumented public API endpoints
- Basic metadata: title, artist, album
- Static cover art (JPEG, resolution varies)
- Lyrics (when available in the API response)
- No authentication required
- Zero configuration setup

---

## Authentication

The iHeart provider requires **no authentication**. The undocumented API endpoints used by MeedyaManager are publicly accessible without API keys, tokens, or accounts.

### No setup required

The iHeart provider is immediately available with no configuration needed. Simply ensure it is enabled in your settings (it is enabled by default).

> **Note:** Because these are undocumented endpoints, there is no official developer programme, documentation, or support from iHeartMedia. Access could be restricted at any time.

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required for iHeart.

### Settings (`settings.json5`)

```json5
{
  providers: {
    iheart: {
      enabled: true,                    // Enable or disable this provider
      priority: 9,                      // Provider priority (lower = higher priority)
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `true` | Whether this provider is active |
| `priority` | `9` | Search priority relative to other providers (set low due to best-effort nature) |

> **Recommendation:** Set iHeart's priority lower than dedicated music metadata providers (Spotify, Apple Music, MusicBrainz) since it provides less comprehensive metadata. iHeart is most useful as a supplementary provider.

---

## Available Data

The iHeart provider returns the following metadata fields:

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `song.title` | "Bohemian Rhapsody" |
| `artist` | `song.artistName` | "Queen" |
| `album` | `song.albumName` | "A Night at the Opera" |
| `lyrics` | `song.lyrics` | (full lyrics text, when available) |

### Data limitations

The iHeart API provides basic metadata only:

- No ISRC codes
- No track or disc numbers
- No release year or date
- No genre classification
- No audio quality information
- Limited album data

For complete metadata enrichment, use iHeart alongside more comprehensive providers like Spotify, Apple Music, or MusicBrainz.

### API Endpoint

MeedyaManager queries the following undocumented endpoint:

```
GET https://api.iheart.com/api/v3/search/all?keywords={query}&maxRows=10
```

The response includes a `results` object with a `songs` array containing matched tracks.

---

## Custom Tags

The following custom tags are stored in the file's metadata when matched:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_iheart_id` | iHeartRadio song ID | `"12345678"` |
| `custom_iheart_url` | Constructed iHeartRadio URL | `"https://www.iheart.com/artist/Queen/12345678"` |

> **Note:** The `custom_iheart_url` is constructed by MeedyaManager from the song ID and artist name, as the API does not always return a direct URL. These URLs may not always resolve to valid pages on iHeartRadio's website.

---

## Cover Art

iHeart provides static cover art when available in the API response:

| Type | Format | Resolution | Source |
| ---- | ------ | ---------- | ------ |
| **Static (album artwork)** | JPEG | Varies | `song.imageUrl` |

MeedyaManager saves this as `FrontCover.jpg`.

> **Note:** Not all songs in the iHeart API include cover art. The resolution varies and is generally lower quality than dedicated music services. For best cover art, prioritise Apple Music (3000x3000) or Deezer (1000x1000).

---

## Troubleshooting

### "iHeart search returned 0 results"

**Possible causes:**
- The track may not be in iHeart's catalog
- The undocumented API may have changed or become unavailable
- Search terms may be too specific

**Solutions:**
1. Try a simpler search (just the song title, without artist or album)
2. Verify the API is still accessible by testing in a browser:
   ```
   https://api.iheart.com/api/v3/search/all?keywords=bohemian+rhapsody&maxRows=5
   ```
3. If the API has changed, MeedyaManager will need an update — check for new releases

### "iHeart search failed" (connection error)

**Possible causes:**
- Network connectivity issues
- iHeart's API may be temporarily down
- The endpoint URL may have changed

**Solutions:**
1. Check your internet connection
2. Try again in a few minutes — the error may be transient
3. If persistent, the undocumented endpoint may have been moved or removed
4. Check the MeedyaManager logs for the specific HTTP error code

### HTTP 403 or 429 — Access denied or rate limited

**Cause:** iHeartMedia may have added rate limiting or access restrictions to their public endpoints.

**Solutions:**
- MeedyaManager includes built-in rate limiting to prevent aggressive requests
- If you receive 403 errors consistently, the endpoint may have been restricted
- Consider disabling the iHeart provider and relying on other providers

### Missing or incorrect cover art

**Cause:** The `imageUrl` field in the API response may point to a generic placeholder image or an incorrect album cover.

**Solution:** This is a limitation of the undocumented API. For reliable cover art, prioritise Apple Music, Spotify, or Deezer providers.

### Direct song lookup not supported

**Cause:** The iHeart API's undocumented endpoints do not include a reliable direct song lookup by ID. `lookup_by_id()` returns `None`.

**Solution:** This is expected behaviour. The iHeart provider only supports search-based matching, not direct ID-based lookups.

---

## Legal Notes

- iHeartRadio is a service owned by **iHeartMedia, Inc.**
- The endpoints used by this provider are **undocumented and unofficial** — iHeartMedia does not provide a public developer API
- These endpoints may change, move, or be restricted at any time without notice
- Use of undocumented endpoints may be subject to [iHeartMedia's Terms of Use](https://www.iheart.com/content/terms-of-use/)
- MeedyaManager includes this provider as a **best-effort** convenience with the understanding that it relies on undocumented infrastructure
- No sensitive data is sent to iHeart's servers — only search query strings
- Cover art and metadata are the property of their respective rights holders
- MeedyaManager stores provider IDs and URLs as custom metadata tags for reference only; this does not imply endorsement by iHeartMedia, Inc.

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
