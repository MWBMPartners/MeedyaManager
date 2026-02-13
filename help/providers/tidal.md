# 🌊 TIDAL Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)**

This guide covers setting up the **TIDAL** metadata provider in MeedyaManager, including obtaining API credentials, configuring environment variables, and understanding TIDAL's unique audio quality and spatial audio metadata.

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

The TIDAL provider uses the **TIDAL API** (both the legacy v1 and OpenAPI v2 endpoints) to search the TIDAL catalog for track and album metadata. TIDAL is unique among providers because it surfaces **audio quality tier** information — including Lossless, Hi-Res, Hi-Res Lossless, and MQA indicators — as well as **spatial audio format** metadata for Dolby Atmos and Sony 360 Reality Audio content.

This makes TIDAL particularly valuable for users who want to sort their media library by audio quality or spatial audio format.

**Key features:**

- Track and album search via the TIDAL catalog
- ISRC code retrieval for precise track identification
- Audio quality tiers: Low, High, Lossless, Hi-Res, Hi-Res Lossless
- Spatial audio indicators: Dolby Atmos, Sony 360 Reality Audio
- Explicit content flags
- Static cover art up to 1280x1280 pixels (JPEG)

---

## Authentication

TIDAL uses **OAuth2.1 Client Credentials** flow. You need a TIDAL Developer account and a registered application.

### Step-by-step setup

1. **Create a TIDAL Developer account**
   - Go to [developer.tidal.com](https://developer.tidal.com/)
   - Click **Sign Up** or **Get Started**
   - Create an account or log in with your existing TIDAL credentials

2. **Register an application**
   - In the TIDAL Developer Dashboard, create a new application
   - Fill in the application details:
     - **App name:** MeedyaManager (or any name you prefer)
     - **Description:** Media metadata enrichment tool
   - Accept the developer terms

3. **Copy your credentials**
   - After creating the application, note the **Client ID** and **Client Secret**
   - These are displayed on the application details page
   - Copy both values to your `.env` file

4. **Verify access**
   - TIDAL's developer programme may have approval processes or waitlists
   - Ensure your application has been approved before testing

---

## Configuration

### Environment Variables (`.env`)

Add the following to your `.env` file:

```env
# TIDAL API credentials (OAuth2.1 Client Credentials)
TIDAL_CLIENT_ID=your_client_id_here
TIDAL_CLIENT_SECRET=your_client_secret_here
```

| Variable | Description |
| -------- | ----------- |
| `TIDAL_CLIENT_ID` | Your TIDAL application's Client ID |
| `TIDAL_CLIENT_SECRET` | Your TIDAL application's Client Secret |

### Settings (`settings.json5`)

```json5
{
  providers: {
    tidal: {
      enabled: true,                    // Enable or disable this provider
      priority: 5,                      // Provider priority (lower = higher priority)
      country_code: "GB",              // ISO 3166-1 alpha-2 country code for catalog region
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `true` | Whether this provider is active |
| `priority` | `5` | Search priority relative to other providers |
| `country_code` | `"GB"` | Country code for TIDAL catalog region (affects availability) |

---

## Available Data

The TIDAL provider returns the following standard metadata fields:

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `track.title` | "Bohemian Rhapsody" |
| `artist` | `track.artists[0].name` | "Queen" |
| `album` | `track.album.title` | "A Night at the Opera" |
| `year` | `track.streamStartDate` or `album.releaseDate` | "1975" |
| `track_num` | `track.trackNumber` | "11" |
| `disc_num` | `track.volumeNumber` | "1" |
| `isrc` | `track.isrc` | "GBUM71029604" |

### Audio Quality Tiers

TIDAL tracks include an `audioQuality` field indicating the available quality:

| TIDAL Quality | MeedyaManager Label | Description |
| ------------- | ------------------- | ----------- |
| `LOW` | Low | 96 kbps AAC |
| `HIGH` | High | 320 kbps AAC |
| `LOSSLESS` | Lossless | CD quality (16-bit/44.1kHz FLAC) |
| `HI_RES` | Hi-Res | MQA encoded (up to 24-bit/96kHz) |
| `HI_RES_LOSSLESS` | Hi-Res Lossless | FLAC up to 24-bit/192kHz |

### Spatial Audio Formats

Some tracks include spatial audio indicators in the `audioModes` array:

| Audio Mode | Description |
| ---------- | ----------- |
| `DOLBY_ATMOS` | Dolby Atmos immersive audio mix |
| `SONY_360RA` | Sony 360 Reality Audio spatial mix |

---

## Custom Tags

The following custom tags are stored in the file's metadata when matched:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_tidal_id` | TIDAL track ID | `"77814670"` |
| `custom_tidal_url` | TIDAL URL for the track | `"https://tidal.com/browse/track/77814670"` |
| `custom_tidal_isrc` | ISRC code from TIDAL | `"GBUM71029604"` |
| `custom_tidal_quality` | Audio quality tier label | `"Hi-Res Lossless"` |
| `custom_tidal_explicit` | Explicit content flag | `"true"` |
| `custom_tidal_dolby_atmos` | Dolby Atmos availability | `"true"` |
| `custom_tidal_sony_360ra` | Sony 360 Reality Audio availability | `"true"` |

These tags are especially useful for quality-aware sorting rules:

```json5
// Sort by audio quality tier
rules: [
  {
    condition: { tag: "custom_tidal_quality", operator: "==", value: "Hi-Res Lossless" },
    destination: "{media_class}/Hi-Res/{artist}/{album}/"
  },
  {
    condition: { tag: "custom_tidal_dolby_atmos", operator: "==", value: "true" },
    destination: "{media_class}/Dolby Atmos/{artist}/{album}/"
  }
]
```

---

## Cover Art

TIDAL provides static cover art via a UUID-based URL template:

| Type | Format | Resolution | Source |
| ---- | ------ | ---------- | ------ |
| **Static (front cover)** | JPEG | 1280x1280 | `album.cover` UUID |

The cover art URL pattern is:

```
https://resources.tidal.com/images/{uuid}/1280x1280.jpg
```

Available resolutions: 160x160, 320x320, 640x640, 1280x1280. MeedyaManager always uses 1280x1280 for maximum quality and saves it as `FrontCover.jpg`.

---

## Troubleshooting

### "Tidal: failed to obtain OAuth2 access token"

**Cause:** Invalid or missing credentials.

**Solutions:**
1. Verify `TIDAL_CLIENT_ID` and `TIDAL_CLIENT_SECRET` are set correctly in `.env`
2. Ensure your TIDAL Developer application has been approved
3. Check that you copied both values completely (no trailing spaces)

### "Tidal: missing client_id or client_secret"

**Cause:** One or both environment variables are not set.

**Solution:** Add both `TIDAL_CLIENT_ID` and `TIDAL_CLIENT_SECRET` to your `.env` file.

### "Tidal search returned 0 results"

**Possible causes:**
- The track may not be available in TIDAL's catalog for your configured `country_code`
- Try changing the `country_code` in `settings.json5`
- TIDAL's catalog is smaller than Spotify or Apple Music for some regions

**Solutions:**
1. Search on [tidal.com](https://tidal.com/) directly to verify the track exists
2. Try with a different `country_code` (e.g. `"US"`, `"GB"`, `"NO"` for Norway, TIDAL's home market)
3. Ensure your search query has sufficient metadata (title + artist works best)

### HTTP 401 — Unauthorized

**Cause:** OAuth2.1 token has expired or credentials have been revoked.

**Solution:**
- MeedyaManager caches tokens for up to 1 hour and refreshes automatically
- If persistent, verify your credentials in the TIDAL Developer Dashboard
- Ensure your application is still active and approved

---

## Legal Notes

- The TIDAL API is provided under the [TIDAL Developer Terms and Conditions](https://developer.tidal.com/documentation/guidelines)
- A TIDAL Developer account is required for API access
- Audio quality tier information is indicative of the available streaming quality; actual playback quality depends on the user's subscription tier
- Dolby Atmos is a trademark of Dolby Laboratories; Sony 360 Reality Audio is a trademark of Sony Group Corporation
- Cover art and metadata are the property of their respective rights holders
- MeedyaManager stores provider IDs, URLs, and quality indicators as custom metadata tags for reference and sorting; this does not imply endorsement by TIDAL Music AS

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
