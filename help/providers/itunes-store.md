# 🎵 iTunes Store Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)**

This guide explains how to configure and use the **iTunes Store** metadata provider in MeedyaManager.

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

The iTunes Store provider retrieves music track and album metadata from Apple's **iTunes Search API**. It searches the same catalog that powers the iTunes Store and Apple Music, covering millions of tracks, albums, and artists across all genres.

This provider is useful for:

- Looking up music track metadata (title, artist, album, genre, year)
- Retrieving high-resolution album artwork (up to 3000x3000 pixels)
- Cross-referencing tracks by artist and title against Apple's catalog
- Supplementing metadata from other providers (MusicBrainz, Spotify, etc.)

The provider uses the public `https://itunes.apple.com/search` endpoint with `media=music` and `entity=song` (or `entity=album`) parameters.

---

## Authentication

**No authentication is required.** The iTunes Store provider uses Apple's public iTunes Search API, which is freely accessible without any API key, token, or developer account.

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
    itunes_store: {
      // Enable or disable this provider (default: true)
      enabled: true,

      // Preferred storefront/country code for regional catalog results
      // Uses ISO 3166-1 alpha-2 codes (e.g., "US", "GB", "JP", "DE")
      country: "GB",

      // Maximum number of results to return per search (1-200, default: 10)
      result_limit: 10,

      // Whether to also search albums (not just tracks) (default: true)
      search_albums: true,

      // Preferred artwork resolution in pixels (default: 3000)
      // The API returns artworkUrl100; MeedyaManager scales the URL
      // to the requested size (common values: 600, 1200, 3000)
      artwork_size: 3000,
    }
  }
}
```

### Storefront / Region

The `country` parameter controls which regional iTunes Store catalog is searched. Music availability and pricing vary by country. The US catalog (`"US"`) typically has the broadest selection, but using your local country code ensures you see region-specific releases and correct release dates.

---

## Available Data

The iTunes Store provider returns the following standard metadata fields:

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Track title | `"Bohemian Rhapsody"` |
| `artist` | Artist name | `"Queen"` |
| `album` | Album name | `"A Night at the Opera"` |
| `album_artist` | Album-level artist | `"Queen"` |
| `genre` | Primary genre | `"Rock"` |
| `year` | Release year | `"1975"` |
| `track_num` | Track number on the album | `"11"` |
| `total_tracks` | Total tracks on the album | `"12"` |
| `disc_num` | Disc number | `"1"` |
| `total_discs` | Total discs | `"1"` |

---

## Custom Tags

In addition to standard fields, the provider writes the following custom tags to media files:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_itunes_store_id` | iTunes Store track or album ID | `"1440833098"` |
| `custom_itunes_store_url` | Direct link on the iTunes Store | `"https://music.apple.com/gb/album/bohemian-rhapsody/1440833098?i=1440833491"` |
| `custom_itunes_collection_id` | iTunes collection (album) ID | `"1440833098"` |

The `custom_itunes_collection_id` is particularly useful for grouping tracks that belong to the same album across your library. It provides a unique, stable identifier that persists even if album names vary slightly between files.

---

## Cover Art

The iTunes Store provider supplies **high-resolution static JPEG cover art** at up to **3000x3000 pixels**.

- The API returns an `artworkUrl100` field (100x100 thumbnail URL)
- MeedyaManager automatically scales this URL to the configured `artwork_size` (default: 3000x3000)
  - URL transformation: `artworkUrl100` value has `100x100bb` replaced with `3000x3000bb`
- Image format: JPEG
- Resolution: Up to 3000 x 3000 pixels (actual size depends on what Apple has available)
- The image is saved as `FrontCover.jpg` alongside the media file
- If embedding is enabled, the image is also embedded in the file's tags (APIC for ID3, covr for MP4)

> **Tip:** The iTunes Store typically provides the highest-resolution album artwork of any free provider. If you only enable one provider for cover art, this is an excellent choice for music files.

---

## Troubleshooting

### Provider shows "Available" but returns no results

- **Check the country code.** Music catalog availability varies by region. Try `country: "US"` for the broadest catalog.
- **Check the search terms.** The API performs keyword matching on artist + track title. Ensure your file has at least a title and artist tag.
- **Verify the media class.** This provider only searches music. If your file is classified as `"Podcast"` or `"Movie"`, this provider will not be queried.

### Artwork is lower resolution than expected

- Not all albums have 3000x3000 artwork in Apple's catalog. Older or obscure releases may only have 600x600 or 1200x1200 images.
- The URL transformation always requests the configured size, but Apple may serve a smaller image if the original is not available at that resolution.
- Check the saved `FrontCover.jpg` dimensions to verify what was actually downloaded.

### Duplicate results for the same track

The iTunes Store often lists the same track across multiple releases (original album, greatest hits, deluxe edition, etc.). MeedyaManager's match scoring system ranks results by confidence, preferring:

1. Exact title matches
2. Matching album name (if your file has album metadata)
3. Matching year
4. Track number consistency

If duplicates are still an issue, ensure your files have album metadata populated.

### Rate limit warnings in logs

If you are processing a large batch of music files simultaneously:

1. MeedyaManager automatically queues and retries rate-limited requests
2. Reduce `result_limit` to minimise API calls per file
3. The rate limiter spaces requests to stay within Apple's limits

---

## Legal Notes

- The iTunes Store provider uses Apple's **iTunes Search API**, which is publicly available and does not require a licence agreement for personal, non-commercial use.
- Apple, iTunes, and Apple Music are trademarks of Apple Inc.
- Album artwork and music metadata are the property of their respective rights holders (artists, labels, publishers).
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media library. No content is redistributed or made available to third parties.
- For full terms, see: [Apple iTunes Affiliate Resources](https://affiliate.itunes.apple.com/resources/)

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
