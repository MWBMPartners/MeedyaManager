# 📡 TheTVDB Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)**

This guide explains how to configure and use the **TheTVDB** metadata provider in MeedyaManager.

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

The TheTVDB provider retrieves TV series, season, and episode metadata from **TheTVDB** (thetvdb.com), a long-established community-driven database of television information. TVDB is widely used as the canonical source for TV episode numbering by media servers like Plex, Kodi, and Sonarr.

This provider is useful for:

- Looking up TV series metadata (show name, network, status, genres)
- Retrieving season and episode listings with air dates
- Matching video files to specific TV episodes by show + season + episode number
- Downloading series banners, posters, and episode screenshots
- Resolving episode ordering differences (aired order vs. DVD order)

The provider uses the **TVDB API v4** at `https://api4.thetvdb.com/v4/`.

---

## Authentication

**A free API key is required.** TheTVDB provides free API access to registered users.

### Step-by-Step: Getting Your TVDB API Key

1. **Create a TVDB account** at [thetvdb.com](https://thetvdb.com) (click **Subscribe** or **Sign Up**)
2. **Verify your email address** via the confirmation link TVDB sends you
3. **Navigate to your API keys:**
   - Go to your dashboard: [thetvdb.com/dashboard](https://thetvdb.com/dashboard)
   - Click **API Keys** in the sidebar
   - Or go directly to: [thetvdb.com/dashboard/account/apikeys](https://thetvdb.com/dashboard/account/apikeys)
4. **Generate a new API key:**
   - Enter a project name (e.g., "MeedyaManager")
   - Click **Generate**
5. **Copy your API key** — this is the long alphanumeric string displayed on the page

### JWT Token Flow

The TVDB v4 API uses a JWT (JSON Web Token) authentication flow:

1. MeedyaManager sends your API key to `POST /login`
2. TVDB returns a JWT bearer token
3. All subsequent API requests include this token in the `Authorization` header
4. The token is valid for approximately 30 days
5. MeedyaManager automatically refreshes the token before it expires

You do not need to manage the JWT token manually — MeedyaManager handles the entire token lifecycle.

---

## Configuration

### Environment Variables (`.env`)

Add your TVDB API key to the `.env` file in the project root:

```env
# TheTVDB API key — get it from thetvdb.com/dashboard/account/apikeys
TVDB_API_KEY=your_api_key_here
```

### Settings File (`settings.json5`)

You can optionally configure the provider's behaviour in `config/settings.json5`:

```json5
{
  providers: {
    tvdb: {
      // Enable or disable this provider (default: true)
      enabled: true,

      // Preferred language for metadata (ISO 639-1, default: "eng")
      // Note: TVDB uses 3-letter language codes (ISO 639-2)
      language: "eng",

      // Episode ordering preference (default: "aired")
      // Options: "aired" (broadcast order), "dvd" (DVD release order)
      // Some shows have different episode ordering between aired and DVD versions
      episode_order: "aired",

      // Maximum number of search results to evaluate (1-20, default: 5)
      result_limit: 5,

      // Whether to fetch full episode lists for matched series (default: true)
      // Required for season/episode matching but adds extra API calls
      fetch_episodes: true,
    }
  }
}
```

### Episode Ordering: Aired vs. DVD

TVDB maintains two episode orderings for many shows:

- **Aired order:** Episodes numbered in the order they were originally broadcast. This is the default and most common ordering.
- **DVD order:** Episodes numbered as they appear on DVD/Blu-ray releases. This sometimes differs from aired order (e.g., Firefly, Star Wars: The Clone Wars).

Choose the ordering that matches how your files are numbered. If unsure, use `"aired"`.

---

## Available Data

The TVDB provider returns the following standard metadata fields:

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Episode title | `"The Rains of Castamere"` |
| `show` | TV series name | `"Game of Thrones"` |
| `season` | Season number | `"3"` |
| `episode` | Episode number | `"9"` |
| `episode_title` | Episode title (same as title) | `"The Rains of Castamere"` |
| `genre` | Series genres (comma-separated) | `"Drama, Fantasy, Adventure"` |
| `year` | Episode air year | `"2013"` |

---

## Custom Tags

In addition to standard fields, the provider writes the following custom tags to media files:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_tvdb_id` | TVDB series or episode ID | `"121361"` |
| `custom_tvdb_url` | Direct link on TheTVDB | `"https://thetvdb.com/series/game-of-thrones"` |
| `custom_tvdb_slug` | URL-friendly series slug | `"game-of-thrones"` |
| `custom_tvdb_status` | Series status | `"Ended"` |

The `custom_tvdb_slug` is the URL-friendly identifier for the series on TVDB. It can be useful for building predictable folder names in rename rules:

```json5
rename_format: "TV Shows/{custom_tvdb_slug}/Season {season}/{show} - S{season}E{episode} - {title}.{extension}"
```

The `custom_tvdb_status` indicates whether the series is `"Continuing"`, `"Ended"`, or `"Upcoming"`. This can be used in conditional rename rules to separate ongoing series from completed ones.

---

## Cover Art

The TVDB provider supplies **static JPEG images** from its extensive artwork database.

- Series posters, banners, and episode screenshots are available
- Images are fetched from the TVDB image CDN: `https://artworks.thetvdb.com/banners/`
- Image format: JPEG
- Resolution: Varies by artwork type (posters are typically 680 x 1000; episode screenshots around 400 x 225)
- The primary series poster is saved as `FrontCover.jpg` alongside the media file
- If embedding is enabled, the image is also embedded in the file's metadata

> **Note:** TVDB's image resolution is generally lower than TMDB's. For the highest-quality TV show artwork, consider using TMDB as your primary provider and TVDB for authoritative episode numbering.

---

## Troubleshooting

### "Missing credentials" — Provider not available

- Ensure `TVDB_API_KEY` is set in your `.env` file
- Verify the key is correct — copy it from [thetvdb.com/dashboard/account/apikeys](https://thetvdb.com/dashboard/account/apikeys)
- Check that your `.env` file is in the project root directory
- Run `python cli/runner.py --providers-list` to verify provider status

### "401 Unauthorized" or "JWT token expired" errors

- Your API key may be invalid or revoked — regenerate it on the TVDB dashboard
- MeedyaManager automatically refreshes JWT tokens, but if the underlying API key is invalid, the refresh will also fail
- Check the logs for `Token refresh failed` messages

### Episode not found for a known series

- **Check the episode ordering.** If your files use DVD ordering but the config is set to `"aired"`, episode numbers may not match. Try switching `episode_order` to `"dvd"`.
- **Check for specials.** TVDB places specials in Season 0. If your file is a special episode, ensure it has `season: "0"`.
- **Verify on TVDB directly.** Search for the show at [thetvdb.com](https://thetvdb.com) and confirm the episode numbering matches your files.

### Search returns wrong series (name collision)

Some show names are shared across multiple series (e.g., "Battlestar Galactica" 1978 vs. 2004). To improve matching:

1. Ensure your files include a `year` tag
2. The match scoring system uses year as a disambiguation signal
3. You can manually specify the TVDB ID in the file's `custom_tvdb_id` tag to force an exact match

### Rate limiting

The TVDB v4 API has rate limits (varies by subscription tier). MeedyaManager's rate limiter handles this automatically:

1. Free tier: approximately 100 requests per minute
2. If you see rate-limit warnings, processing will slow down but continue
3. Reduce `result_limit` and consider disabling `fetch_episodes` for initial scans

---

## Legal Notes

- TheTVDB provides a **free API** for registered users under its [API Terms](https://thetvdb.com/api-terms).
- TV show metadata is contributed by the TheTVDB community and is made available under their subscriber agreement.
- Series artwork, banners, and screenshots are the property of their respective rights holders (studios, networks, distributors).
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media library. No content is redistributed.
- "TheTVDB" is a trademark of TheTVDB.com LLC.

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
