# 🔊 Shazam Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers setting up the **Shazam** metadata provider in MeedyaManager. Shazam is unique among providers because it supports **audio fingerprinting** — the ability to identify tracks by analysing the audio content itself, not just metadata tags.

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Configuration](#configuration)
4. [Available Data](#available-data)
5. [Custom Tags](#custom-tags)
6. [Audio Fingerprinting](#audio-fingerprinting)
7. [Troubleshooting](#troubleshooting)
8. [Legal Notes](#legal-notes)

---

## Overview

The Shazam provider communicates with Shazam's audio recognition API directly via HTTP to identify and search for music. Its most powerful feature is **audio fingerprinting**: MeedyaManager can analyse the first 10-15 seconds of an audio file to identify the track, even when the file has no metadata tags at all.

This makes Shazam especially useful for:

- Identifying completely untagged audio files
- Verifying existing metadata against audio content
- Handling files where metadata is corrupt, missing, or incorrect
- Processing audio recordings, rips, or captures with no tags

**Key features:**

- Audio fingerprinting from audio file content (most accurate identification method)
- Text-based track search by title and artist
- Genre classification
- Static cover art from Shazam's CDN
- Shazam URLs for sharing and linking
- No API key required

---

## Authentication

Shazam requires **no API key or authentication**. MeedyaManager communicates with Shazam's recognition API using reverse-engineered endpoints. No accounts, tokens, or credentials are needed — the Shazam provider is available out of the box.

### No setup required

There is nothing to install or configure for basic functionality.

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required for Shazam.

### Settings (`settings.json5`)

```json5
{
  providers: {
    shazam: {
      enabled: true,                    // Enable or disable this provider
      priority: 7,                      // Provider priority (lower = higher priority)
      fingerprint_enabled: true,        // Enable audio fingerprinting (reads file audio data)
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `true` | Whether this provider is active |
| `priority` | `7` | Search priority relative to other providers |
| `fingerprint_enabled` | `true` | Whether to use audio fingerprinting when file path is available |

> **Performance note:** Audio fingerprinting reads the first 10-15 seconds of each audio file and sends a fingerprint to Shazam's servers. This is slightly slower than text-based search but significantly more accurate, especially for untagged files.

---

## Available Data

The Shazam provider returns the following metadata fields:

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `track.title` | "Bohemian Rhapsody" |
| `artist` | `track.subtitle` | "Queen" |
| `genre` | `track.genres.primary` | "Rock" |

### Data limitations

Shazam's API is focused on track identification rather than full metadata cataloguing:

- No ISRC codes
- No album name (track-level identification only)
- No track/disc numbers
- No release year
- No duration data

For complete metadata, use Shazam for identification and cross-reference with other providers (Spotify, MusicBrainz, Apple Music) for full tag population.

---

## Custom Tags

The following custom tags are stored in the file's metadata when matched:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_shazam_id` | Shazam track key (unique identifier) | `"5405758"` |
| `custom_shazam_url` | Shazam URL for the track | `"https://www.shazam.com/track/5405758/bohemian-rhapsody"` |
| `custom_shazam_key` | Shazam track key (alias of ID) | `"5405758"` |

These tags primarily serve as cross-references to the Shazam catalog:

```json5
// Example: use Shazam for identification, then reference the Shazam URL
rename_format: "{artist}/{album}/{track_num} - {title}.{extension}"
```

---

## Audio Fingerprinting

Audio fingerprinting is the Shazam provider's most distinctive feature. When a file path is available in the search query, MeedyaManager sends a fingerprint of the audio content to Shazam for identification.

### How it works

1. MeedyaManager reads the first 10-15 seconds of audio from the file
2. MeedyaManager generates an audio fingerprint (a compact digital signature)
3. The fingerprint is sent to Shazam's recognition API
4. Shazam returns the matched track information (if found)

### When fingerprinting is used

- When a file has no metadata tags at all
- When metadata is incomplete (no title or artist)
- When `fingerprint_enabled: true` is set and a file path is available in the query
- As a verification step to confirm existing metadata against audio content

### Performance considerations

| Aspect | Detail |
| ------ | ------ |
| **File read** | Only the first 10-15 seconds of audio are read |
| **Network** | Sends a small fingerprint (a few KB) to Shazam's servers |
| **Response time** | Typically 1-3 seconds per file |
| **Accuracy** | Very high for commercially released music; lower for live recordings, remixes, or covers |

### Cover art from fingerprinting

When a track is identified via fingerprinting, Shazam returns cover art from its CDN:

| Type | Format | Resolution | Source |
| ---- | ------ | ---------- | ------ |
| **Static (cover art)** | JPEG | Varies | `track.images.coverart` |

MeedyaManager saves this as `FrontCover.jpg`.

---

## Troubleshooting

### Audio fingerprinting fails or returns no results

**Possible causes:**
- The audio file may be corrupt or too short (less than a few seconds)
- The track may not be in Shazam's database (rare for commercial releases, common for obscure content)
- The audio may be heavily distorted, remixed, or a live performance
- Network connectivity issues preventing communication with Shazam servers

**Solutions:**
1. Verify the audio file plays correctly in a media player
2. Try a text-based search instead (provide title and artist in the query)
3. Check network connectivity — Shazam fingerprinting requires internet access
4. For live recordings or covers, fingerprinting may not match; use text search instead

### "Shazam text search failed" or empty results

**Cause:** The text search query may not match Shazam's catalog entries.

**Solutions:**
1. Try with just the title (without artist) or vice versa
2. Shazam's text search is less comprehensive than its fingerprinting — use other providers for text-based metadata search
3. Check for unusual characters or very long search terms

### Rate limiting or connection errors

**Cause:** Making too many requests to Shazam's API in a short period.

**Solution:**
- MeedyaManager includes built-in conservative rate limiting for Shazam
- If you encounter connection errors during bulk operations, consider reducing the number of concurrent provider queries
- Shazam's unofficial API does not publish rate limits; MeedyaManager errs on the side of caution

---

## Legal Notes

- Shazam is a service owned by **Apple Inc.**
- This integration uses **reverse-engineered API endpoints** and may break without notice if Shazam changes their API
- Use of this provider may be subject to [Shazam's Terms of Use](https://www.shazam.com/terms) and [Apple's Terms of Service](https://www.apple.com/legal/internet-services/terms/site.html)
- Audio fingerprinting sends a compact digital signature of the audio content to Shazam's servers — the full audio file is **not** uploaded
- MeedyaManager includes this provider as a convenience with the understanding that it is unofficial and best-effort
- Cover art and metadata are the property of their respective rights holders
- MeedyaManager's Shazam integration is not affiliated with Shazam or Apple Inc.

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
