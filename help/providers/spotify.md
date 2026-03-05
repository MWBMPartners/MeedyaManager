# 🟢 Spotify Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers setting up the **Spotify** metadata provider in MeedyaManager, including obtaining API credentials, configuring environment variables, and understanding the rich audio feature data returned.

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

The Spotify provider uses the **Spotify Web API** to search the Spotify catalog for track and album metadata. In addition to standard metadata, Spotify is the only provider that offers **audio features** — algorithmically computed attributes like energy, danceability, tempo, and valence (positiveness) — making it especially useful for intelligent playlist organisation and mood-based sorting rules.

**Key features:**

- Track and album search via the Spotify catalog
- ISRC code retrieval for precise track identification
- Audio features: energy, danceability, tempo, valence, key, and mode
- Popularity scores (0-100) and explicit content flags
- Static cover art up to 640x640 pixels (JPEG)

---

## Authentication

Spotify uses **OAuth2 Client Credentials** flow. You need a free Spotify Developer account and a registered application.

### Step-by-step setup

1. **Create a Spotify account** (if you don't have one)
   - Go to [spotify.com](https://www.spotify.com/) and sign up (free tier is sufficient)

2. **Access the Spotify Developer Dashboard**
   - Go to [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard)
   - Log in with your Spotify account
   - Accept the Developer Terms of Service

3. **Create an application**
   - Click **Create App**
   - Fill in the details:
     - **App name:** MeedyaManager (or any name you prefer)
     - **App description:** Media metadata enrichment
     - **Redirect URI:** `http://localhost:8888/callback` (not used for Client Credentials, but required)
   - Tick the **Web API** checkbox
   - Click **Save**

4. **Copy your credentials**
   - On your app's dashboard page, click **Settings**
   - Note the **Client ID** (visible immediately)
   - Click **View client secret** to reveal the **Client Secret**
   - Copy both values to your `.env` file

> **Note:** The Client Credentials flow does not require a Spotify Premium subscription. A free Spotify account is sufficient for API access.

---

## Configuration

### Environment Variables (`.env`)

Add the following to your `.env` file:

```env
# Spotify API credentials (OAuth2 Client Credentials)
SPOTIFY_CLIENT_ID=your_client_id_here
SPOTIFY_CLIENT_SECRET=your_client_secret_here
```

| Variable | Description |
| -------- | ----------- |
| `SPOTIFY_CLIENT_ID` | Your Spotify application's Client ID |
| `SPOTIFY_CLIENT_SECRET` | Your Spotify application's Client Secret |

### Settings (`settings.json5`)

```json5
{
  providers: {
    spotify: {
      enabled: true,                    // Enable or disable this provider
      priority: 1,                      // Provider priority (lower = higher priority)
      fetch_audio_features: true,       // Whether to fetch audio features (extra API call per track)
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `true` | Whether this provider is active |
| `priority` | `1` | Search priority relative to other providers |
| `fetch_audio_features` | `true` | Fetch audio features (energy, danceability, etc.) — requires one additional API call per matched track |

---

## Available Data

The Spotify provider returns the following standard metadata fields:

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `track.name` | "Bohemian Rhapsody" |
| `artist` | `track.artists[0].name` | "Queen" |
| `album` | `track.album.name` | "A Night at the Opera" |
| `year` | `track.album.release_date` | "1975" |
| `track_num` | `track.track_number` | "11" |
| `disc_num` | `track.disc_number` | "1" |
| `isrc` | `track.external_ids.isrc` | "GBUM71029604" |

### Audio Features

When `fetch_audio_features` is enabled, the following additional data is retrieved:

| Feature | Range | Description |
| ------- | ----- | ----------- |
| `energy` | 0.0 - 1.0 | Perceptual intensity and activity |
| `danceability` | 0.0 - 1.0 | How suitable the track is for dancing |
| `tempo` | BPM | Estimated tempo in beats per minute |
| `valence` | 0.0 - 1.0 | Musical positiveness (higher = happier) |
| `key` | 0-11 | Musical key (0 = C, 1 = C#, ... 11 = B) |
| `mode` | 0 or 1 | Major (1) or Minor (0) |

---

## Custom Tags

The following custom tags are stored in the file's metadata when matched:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_spotify_id` | Spotify track ID | `"4u7EnebtmKWzUH433cf5Qv"` |
| `custom_spotify_url` | Spotify URL for the track | `"https://open.spotify.com/track/4u7E..."` |
| `custom_spotify_isrc` | ISRC code from Spotify | `"GBUM71029604"` |
| `custom_spotify_popularity` | Popularity score (0-100) | `"85"` |
| `custom_spotify_energy` | Energy level (0.0-1.0) | `"0.567"` |
| `custom_spotify_danceability` | Danceability score (0.0-1.0) | `"0.432"` |
| `custom_spotify_tempo` | Tempo in BPM | `"143.5"` |
| `custom_spotify_valence` | Valence / positiveness (0.0-1.0) | `"0.234"` |

These tags can be used in sorting rules. For example, to sort music by mood:

```json5
// Sort high-energy tracks into a separate folder
rules: [
  {
    condition: { tag: "custom_spotify_energy", operator: ">", value: "0.8" },
    destination: "{media_class}/High Energy/{artist}/{album}/"
  }
]
```

---

## Cover Art

Spotify provides static cover art only:

| Type | Format | Resolution | Source |
| ---- | ------ | ---------- | ------ |
| **Static (front cover)** | JPEG | 640x640 | `track.album.images[0]` (largest) |

MeedyaManager saves this as `FrontCover.jpg`.

> **Note:** Spotify does not provide animated cover art. For animated covers, use the Apple Music provider.

---

## Troubleshooting

### "Spotify: failed to obtain OAuth2 access token"

**Cause:** Invalid or missing credentials.

**Solutions:**
1. Verify `SPOTIFY_CLIENT_ID` and `SPOTIFY_CLIENT_SECRET` are set correctly in `.env`
2. Ensure you copied the full Client Secret (it is not partially masked)
3. Check that your Spotify app has not been rate-limited or disabled

### "Spotify search returned 0 results"

**Possible causes:**
- The track may not be available in the Spotify catalog
- Search query may be too specific or have unusual characters
- MeedyaManager constructs queries with field prefixes (`track:`, `artist:`, `album:`) for precision — if results are sparse, ensure metadata is reasonably accurate

### HTTP 429 — Rate limit exceeded

**Cause:** Too many API requests in a short period.

**Solution:**
- MeedyaManager includes built-in rate limiting for all providers
- If you encounter this in development, wait 30 seconds and retry
- Spotify's rate limits are generous for Client Credentials flow but can be hit during bulk operations

### Audio features returning `None`

**Cause:** Spotify's Audio Features endpoint may not have data for all tracks. Podcasts, audiobooks, and some regional tracks lack audio feature data.

**Solution:** This is expected behaviour. MeedyaManager will skip audio feature tags for tracks where data is unavailable.

---

## Legal Notes

- The Spotify Web API is provided under the [Spotify Developer Terms of Service](https://developer.spotify.com/terms/)
- A free Spotify account is sufficient for API access (no Premium required)
- Client Credentials tokens are cached for 1 hour and refreshed automatically
- Audio features are computed by Spotify's algorithms and may not always align with human perception
- Cover art and metadata are the property of their respective rights holders
- MeedyaManager stores provider IDs and URLs as custom metadata tags for reference and linking; this does not imply endorsement by Spotify AB

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
