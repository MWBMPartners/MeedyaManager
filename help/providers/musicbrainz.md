# 🎵 MusicBrainz Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers setting up the **MusicBrainz** metadata provider in MeedyaManager. MusicBrainz is a free, community-maintained music database that requires no API key — making it the easiest provider to get started with.

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

The MusicBrainz provider uses the **MusicBrainz Web Service API (v2)** to search the world's largest open music database. MusicBrainz is community-maintained and freely accessible, providing authoritative identifiers (MBIDs) that are widely used across the music industry for cataloguing and cross-referencing recordings, releases, and artists.

**Key features:**

- Recording (track) and release (album) search via Lucene query syntax
- Direct ISRC code lookup for precise track identification
- MusicBrainz IDs (MBIDs): unique UUIDs for recordings, releases, and artists
- Cover art via the **Cover Art Archive** — a companion service providing album artwork
- No API key required — only a User-Agent header
- Strict 1 request per second rate limit (enforced automatically)

---

## Authentication

MusicBrainz does **not** require an API key or account. The only requirement is a properly formatted **User-Agent header** identifying your application and providing contact information. MeedyaManager includes this header automatically.

### What MeedyaManager sends

```text
User-Agent: MeedyaManager/1.4 (lance.manasse@mwbmpartners.com)
```

This header follows MusicBrainz's mandatory format: `AppName/Version (contact_email)`.

### No setup required

Unlike other providers, MusicBrainz is always available out of the box. There are no credentials to configure, no accounts to create, and no API keys to obtain.

> **Note:** If you are making custom modifications and sending requests directly, you **must** include a valid User-Agent header. Requests without a proper User-Agent will be rejected with HTTP 403.

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required for MusicBrainz.

### Settings (`settings.json5`)

```json5
{
  providers: {
    musicbrainz: {
      enabled: true,                    // Enable or disable this provider
      priority: 3,                      // Provider priority (lower = higher priority)
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `true` | Whether this provider is active |
| `priority` | `3` | Search priority relative to other providers |

---

## Available Data

The MusicBrainz provider returns the following standard metadata fields:

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `recording.title` | "Bohemian Rhapsody" |
| `artist` | `recording.artist-credit` | "Queen" |
| `album` | `recording.releases[0].title` | "A Night at the Opera" |
| `year` | `recording.releases[0].date` | "1975" |
| `isrc` | `recording.isrcs[0]` | "GBUM71029604" |

### ISRC Lookup

When an ISRC code is already present in the file's metadata, MusicBrainz can perform a **direct ISRC lookup** instead of a text search. This provides the highest accuracy match possible:

```text
GET https://musicbrainz.org/ws/2/isrc/GBUM71029604?fmt=json&inc=artist-credits+releases
```

MeedyaManager automatically uses ISRC lookup when an ISRC tag is available.

---

## Custom Tags

The following custom tags are stored in the file's metadata when matched:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_musicbrainz_recording_id` | Recording MBID (UUID) | `"b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d"` |
| `custom_musicbrainz_release_id` | Release MBID (UUID) | `"a1b2c3d4-e5f6-7890-abcd-ef1234567890"` |
| `custom_musicbrainz_artist_id` | Artist MBID (UUID) | `"0383dadf-2a4e-4d10-a46a-e9e041da8eb3"` |
| `custom_musicbrainz_url` | MusicBrainz recording URL | `"https://musicbrainz.org/recording/b10bb..."` |
| `custom_musicbrainz_isrc` | ISRC code from MusicBrainz | `"GBUM71029604"` |

MusicBrainz IDs (MBIDs) are persistent, globally unique identifiers. They are widely supported by other tools like MusicBrainz Picard, Beets, and MP3tag, making them ideal for cross-referencing:

```json5
rename_format: "{artist}/{album}/{track_num} - {title} [MB-{custom_musicbrainz_recording_id}].{extension}"
```

---

## Cover Art

MusicBrainz itself does not host cover art. Instead, it integrates with the **Cover Art Archive** (CAA), a free companion service:

| Type | Format | Resolution | Source |
| ---- | ------ | ---------- | ------ |
| **Static (front cover)** | JPEG | 500x500 | Cover Art Archive via release MBID |

The URL pattern is:

```text
https://coverartarchive.org/release/{release_mbid}/front-500
```

MeedyaManager automatically constructs cover art URLs when a release MBID is available.

> **Note:** Not all MusicBrainz releases have cover art in the Cover Art Archive. Coverage depends on community contributions. MeedyaManager handles missing art gracefully and falls back to other providers.

---

## Troubleshooting

### "MusicBrainz search returned 0 results"

**Possible causes:**
- The recording may not be in the MusicBrainz database (community-contributed)
- Search terms may be too specific or contain special characters
- MusicBrainz uses Lucene query syntax — MeedyaManager escapes special characters automatically, but unusual metadata may still cause issues

**Solutions:**
1. Try searching on [musicbrainz.org](https://musicbrainz.org/) directly to verify the recording exists
2. Consider tagging your files with MusicBrainz Picard first, which adds MBIDs and ISRCs
3. ISRC lookup is more accurate than text search — ensure ISRC tags are present where possible

### HTTP 503 — Rate limit exceeded

**Cause:** MusicBrainz strictly enforces **1 request per second**. Exceeding this results in temporary IP blocks.

**Solution:**
- MeedyaManager's built-in rate limiter should prevent this
- If you see this error, it may be caused by another application also accessing MusicBrainz from the same IP address
- Wait a few minutes for the block to expire (typically 1-5 minutes)

### HTTP 403 — Forbidden

**Cause:** Missing or invalid User-Agent header.

**Solution:**
- This should not occur during normal MeedyaManager operation (the header is hardcoded)
- If you see this after modifying the source code, ensure the User-Agent follows the format: `AppName/Version (contact_email)`

### Cover art not found (HTTP 404 from Cover Art Archive)

**Cause:** The release does not have cover art uploaded to the Cover Art Archive.

**Solution:** This is expected for some releases. You can contribute cover art at [coverartarchive.org](https://coverartarchive.org/) or rely on other providers (Apple Music, Spotify) for artwork.

---

## Legal Notes

- MusicBrainz is a project of the [MetaBrainz Foundation](https://metabrainz.org/), a non-profit organisation
- The MusicBrainz database is licensed under [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) (public domain)
- The Cover Art Archive is a joint project of MusicBrainz and the Internet Archive
- Rate limits (1 request/second) must be respected — excessive requests may result in IP bans
- There are no API key requirements, fees, or registration processes
- MeedyaManager stores MBIDs as custom metadata tags; these are open identifiers and freely usable

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
