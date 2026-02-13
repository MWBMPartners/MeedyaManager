# 📺 Apple TV Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)**

This guide explains how to configure and use the **Apple TV** metadata provider in MeedyaManager.

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

The Apple TV provider retrieves movie and TV show metadata from Apple's **iTunes Search API**, using the `media=movie` and `media=tvShow` parameters. It searches the same catalog available through the Apple TV app and the iTunes Store's video section.

This provider is useful for:

- Identifying movies and TV episodes from video files
- Retrieving show, season, and episode metadata for TV series
- Downloading high-resolution movie/TV poster artwork
- Matching video files against Apple's extensive movie and TV catalog
- Obtaining content ratings and descriptions

The provider queries `https://itunes.apple.com/search` with `media=movie` for films and `media=tvShow` with `entity=tvEpisode` for television content.

---

## Authentication

**No authentication is required.** The Apple TV provider uses Apple's public iTunes Search API with video-specific media type parameters. No API key, token, or developer account is needed.

The provider is available immediately after installation with no additional setup.

> **Note:** Apple applies rate limiting to the iTunes Search API (approximately 20 requests per minute). MeedyaManager's built-in rate limiter handles this automatically.

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required for this provider.

### Settings File (`settings.json5`)

You can optionally configure the provider's behaviour in `config/settings.json5`:

```json5
{
  providers: {
    apple_tv: {
      // Enable or disable this provider (default: true)
      enabled: true,

      // Preferred storefront/country code for regional catalog results
      // Uses ISO 3166-1 alpha-2 codes (e.g., "US", "GB", "AU", "DE")
      country: "GB",

      // Maximum number of results to return per search (1-200, default: 10)
      result_limit: 10,

      // Whether to search for both movies and TV shows (default: true)
      // Set to false to only search the media type matching the file's classification
      search_both_types: true,

      // Preferred artwork resolution in pixels (default: 3000)
      artwork_size: 3000,
    }
  }
}
```

### Storefront / Region

The `country` parameter determines which regional catalog is searched. Movie and TV show availability varies significantly by country due to licensing restrictions. If a title is not found:

1. Try `country: "US"` — the US catalog typically has the broadest selection
2. Check that the movie/show has been released in your configured region
3. Some content may only appear under a different regional storefront

---

## Available Data

The Apple TV provider returns the following standard metadata fields:

### For Movies

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Movie title | `"Inception"` |
| `artist` | Director name | `"Christopher Nolan"` |
| `director` | Director name | `"Christopher Nolan"` |
| `genre` | Primary genre | `"Science Fiction"` |
| `year` | Release year | `"2010"` |

### For TV Shows

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Episode title | `"Ozymandias"` |
| `show` | TV show name | `"Breaking Bad"` |
| `season` | Season number | `"5"` |
| `episode` | Episode number | `"14"` |
| `episode_title` | Episode title | `"Ozymandias"` |
| `artist` | Show creator / network | `"Vince Gilligan"` |
| `genre` | Primary genre | `"Drama"` |
| `year` | Episode air year | `"2013"` |

---

## Custom Tags

In addition to standard fields, the provider writes the following custom tags to media files:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_apple_tv_id` | Apple TV movie or episode ID | `"533654256"` |
| `custom_apple_tv_url` | Direct link on Apple TV / iTunes Store | `"https://tv.apple.com/gb/movie/inception/umc.cmc.12345"` |
| `custom_apple_tv_description` | Short description / synopsis of the movie or episode | `"A thief who steals corporate secrets..."` |
| `custom_apple_tv_rating` | Content rating (e.g., PG-13, TV-MA) | `"PG-13"` |

The `custom_apple_tv_description` is stored as a custom tag rather than a standard field because description/synopsis is not part of MeedyaManager's standard tag map. It can still be referenced in rename rules using `{custom_apple_tv_description}`.

The `custom_apple_tv_rating` provides the content advisory rating, useful for sorting media by age-appropriateness or creating parental-control-aware folder structures.

---

## Cover Art

The Apple TV provider supplies **high-resolution static JPEG cover art** at up to **3000x3000 pixels**.

- The API returns an `artworkUrl100` field (100x100 thumbnail URL)
- MeedyaManager automatically scales this URL to the configured `artwork_size` (default: 3000x3000)
  - URL transformation: `100x100bb` is replaced with `3000x3000bb` in the artwork URL
- Image format: JPEG
- Resolution: Up to 3000 x 3000 pixels (actual resolution depends on Apple's available assets)
- The image is saved as `FrontCover.jpg` alongside the media file
- If embedding is enabled, the image is also embedded in the video file's metadata

> **Note:** Movie posters are typically in portrait orientation (approximately 2:3 aspect ratio) while the artwork URL returns a square crop. The actual image served by Apple may be padded or cropped to fit the requested dimensions.

---

## Troubleshooting

### Provider shows "Available" but returns no results

- **Check the country code.** Movie and TV licensing is heavily region-dependent. Try `country: "US"` for the broadest catalog.
- **Check your search query.** Ensure your video file has at least a title tag or a descriptive filename. For TV episodes, having the show name significantly improves matching.
- **Verify the media class.** This provider only searches movies and TV shows. If your file is classified as `"Music"`, this provider will not be queried.

### Wrong movie or TV show matched

The iTunes Search API returns results based on keyword relevance. Common causes of mismatches:

- **Generic titles** (e.g., "Crash", "The Gift") match multiple movies. Adding a year helps disambiguate.
- **TV episodes** with titles matching movie names. Ensure your file has `show` and `season` metadata.
- Review the confidence scores in MeedyaManager's logs — the match scoring system penalises mismatches on year, show name, and season/episode numbers.

### TV episode numbers not matching

The iTunes Search API uses its own episode numbering which may differ from other databases (TMDB, TVDB). This is particularly common for:

- Shows with specials or bonus episodes
- Shows where seasons were split (e.g., Season 5A / 5B)
- Regional episode numbering differences

If you need authoritative episode numbering, consider using the TMDB or TVDB providers as your primary source and Apple TV as a supplementary provider.

### Rate limit warnings in logs

The same rate limits apply as for the Apple Podcasts and iTunes Store providers (shared API endpoint):

1. MeedyaManager automatically queues and retries rate-limited requests
2. Processing large video libraries will proceed at a throttled pace
3. No manual intervention is needed

---

## Legal Notes

- The Apple TV provider uses Apple's **iTunes Search API**, which is publicly available and does not require a licence agreement for personal, non-commercial use.
- Apple, Apple TV, iTunes, and related marks are trademarks of Apple Inc.
- Movie and TV show metadata, artwork, and descriptions are the property of their respective rights holders (studios, networks, distributors).
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media library. No content is redistributed.
- For full terms, see: [Apple iTunes Affiliate Resources](https://affiliate.itunes.apple.com/resources/)

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
