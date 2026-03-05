# 🍎 Apple Music Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers setting up the **Apple Music** metadata provider in MeedyaManager, including obtaining API credentials, configuring environment variables, and understanding the data returned.

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

The Apple Music provider uses the **Apple Music API (MusicKit)** to search the Apple Music catalog for track and album metadata. It returns rich data including titles, artists, albums, ISRC codes, genres, release dates, and cover art in multiple formats — including Apple's unique animated cover art (MP4 video loops).

**Key features:**

- Track and album search via the Apple Music catalog
- ISRC code retrieval for precise track identification
- Static cover art up to 3000x3000 pixels (JPEG)
- Animated cover art: square (1:1), portrait (tall), and artist spotlight (16:9) as MP4 video
- Genre classification and release date data
- Content rating and duration metadata

---

## Authentication

Apple Music requires **JWT Developer Tokens** signed with the **ES256** (Elliptic Curve) algorithm. You need an Apple Developer Program membership to obtain the necessary credentials.

### Step-by-step setup

1. **Join the Apple Developer Program**
   - Go to [developer.apple.com/programs](https://developer.apple.com/programs/)
   - Enrol in the Apple Developer Program (annual fee applies)

2. **Enable MusicKit**
   - Sign in to [developer.apple.com/account](https://developer.apple.com/account)
   - Go to **Certificates, Identifiers & Profiles**
   - Under **Keys**, click the **+** button to create a new key
   - Give your key a name (e.g. "MeedyaManager MusicKit")
   - Tick the **MusicKit** checkbox
   - Click **Continue**, then **Register**

3. **Download your private key**
   - After creating the key, click **Download** to save the `.p8` file
   - **Important:** You can only download this file once! Store it securely.
   - Note the **Key ID** shown on the key details page (10-character alphanumeric string)

4. **Find your Team ID**
   - Go to [developer.apple.com/account](https://developer.apple.com/account)
   - Your **Team ID** is shown in the top-right corner of the Membership page
   - It is a 10-character alphanumeric string (e.g. `A1B2C3D4E5`)

5. **Note the required Python packages**
   - MeedyaManager uses `pyjwt` and `cryptography` to sign JWT tokens
   - These are included in `requirements.txt` and bundled with release builds

---

## Configuration

### Environment Variables (`.env`)

Add the following to your `.env` file:

```env
# Apple Music API credentials (MusicKit)
APPLE_MUSIC_TEAM_ID=A1B2C3D4E5
APPLE_MUSIC_KEY_ID=ABCDEF1234
APPLE_MUSIC_PRIVATE_KEY=/path/to/AuthKey_ABCDEF1234.p8
```

| Variable | Description |
| -------- | ----------- |
| `APPLE_MUSIC_TEAM_ID` | Your 10-character Apple Developer Team ID |
| `APPLE_MUSIC_KEY_ID` | The Key ID of your MusicKit private key |
| `APPLE_MUSIC_PRIVATE_KEY` | Path to the `.p8` private key file, or the PEM-encoded key string directly |

> **Tip:** The private key can be specified as a file path (recommended) or as the raw PEM string. If using the PEM string, include the full `-----BEGIN PRIVATE KEY-----` header and footer.

### Settings (`settings.json5`)

```json5
{
  providers: {
    apple_music: {
      enabled: true,                    // Enable or disable this provider
      storefront: "gb",                 // ISO 3166-1 alpha-2 country code (e.g. "us", "gb", "de")
      priority: 2,                      // Provider priority (lower = higher priority)
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `true` | Whether this provider is active |
| `storefront` | `"gb"` | Apple Music storefront region (affects catalog availability) |
| `priority` | `2` | Search priority relative to other providers |

---

## Available Data

The Apple Music provider returns the following standard metadata fields:

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `attributes.name` | "Bohemian Rhapsody" |
| `artist` | `attributes.artistName` | "Queen" |
| `album` | `attributes.albumName` | "A Night at the Opera" |
| `genre` | `attributes.genreNames` | "Rock, Classic Rock" |
| `year` | `attributes.releaseDate` | "1975" |
| `track_num` | `attributes.trackNumber` | "11" |
| `disc_num` | `attributes.discNumber` | "1" |
| `composer` | `attributes.composerName` | "Freddie Mercury" |
| `isrc` | `attributes.isrc` | "GBUM71029604" |

---

## Custom Tags

The following custom tags are stored in the file's metadata when matched:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_apple_music_id` | Apple Music catalog song ID | `"1440833098"` |
| `custom_apple_music_url` | Apple Music URL for the track | `"https://music.apple.com/gb/album/..."` |
| `custom_apple_music_isrc` | ISRC code from Apple Music | `"GBUM71029604"` |
| `custom_apple_music_content_rating` | Content advisory rating | `"explicit"` |
| `custom_apple_music_duration_ms` | Duration in milliseconds | `"354000"` |

These tags can be used in MeedyaManager rename templates and sorting rules. For example:

```json5
rename_format: "{media_class}/{artist}/{album}/{track_num} - {title} [{custom_apple_music_isrc}].{extension}"
```

---

## Cover Art

Apple Music provides cover art in multiple formats — more than any other provider:

| Type | Format | Resolution | Source Field |
| ---- | ------ | ---------- | ------------ |
| **Static (front cover)** | JPEG | Up to 3000x3000 | `artwork.url` template with `{w}x{h}` |
| **Animated square** | MP4 | Varies | `editorialVideo.motionSquareVideo1x1` |
| **Animated portrait** | MP4 | Varies (tall) | `editorialVideo.motionDetailTall` |
| **Artist spotlight** | MP4 | 16:9 wide | `editorialVideo.motionArtistWide16x9` |

MeedyaManager saves these as:

- `FrontCover.jpg` — Static album artwork (JPEG, maximum resolution)
- `FrontCover.mp4` — Animated square cover loop
- `PortraitCover.mp4` — Animated portrait cover loop
- `ArtistCover.mp4` — Artist spotlight video

> **Note:** Not all tracks have animated cover art. Animated covers are typically available for featured albums and popular releases. MeedyaManager will only download the formats that are available.

---

## Troubleshooting

### "Apple Music: failed to generate JWT token"

**Cause:** Missing or invalid credentials.

**Solutions:**
1. Verify all three environment variables are set: `APPLE_MUSIC_TEAM_ID`, `APPLE_MUSIC_KEY_ID`, `APPLE_MUSIC_PRIVATE_KEY`
2. Check that the `.p8` file path is correct and the file is readable
3. Ensure `pyjwt` and `cryptography` are installed: `pip install pyjwt cryptography`

### "pyjwt not installed — Apple Music provider unavailable"

**Cause:** The `pyjwt` Python package is not available.

**Solution:**
```bash
pip install pyjwt[crypto]
```

> Release builds include this dependency automatically.

### "Apple Music search returned 0 results"

**Possible causes:**
- The track may not be available in your configured storefront region
- Try changing `storefront` in `settings.json5` to a different country code
- Ensure your search query has sufficient metadata (title + artist works best)

### JWT token expired or rejected (HTTP 401)

**Cause:** Token lifetime has exceeded Apple's maximum (6 months) or the key has been revoked.

**Solution:**
- MeedyaManager caches tokens for up to 6 months and refreshes automatically
- If you revoked the key in Apple Developer, create a new key and update your `.env`

---

## Legal Notes

- The Apple Music API is provided under the [Apple Developer Program License Agreement](https://developer.apple.com/terms/)
- Apple Developer Program membership requires an annual fee
- MusicKit API usage is subject to Apple's rate limits and fair use policies
- Cover art and metadata are the property of their respective rights holders
- Animated cover art is a premium Apple Music feature and may not be available for all content
- MeedyaManager stores provider IDs and URLs as custom metadata tags for reference and linking; this does not imply endorsement by Apple Inc.

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
