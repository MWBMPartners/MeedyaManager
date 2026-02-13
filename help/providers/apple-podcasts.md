# 🎙️ Apple Podcasts Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)**

This guide explains how to configure and use the **Apple Podcasts** metadata provider in MeedyaManager.

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

The Apple Podcasts provider retrieves podcast show and episode metadata from Apple's **iTunes Search API**. It searches the Apple Podcasts catalog — the same data that powers the Apple Podcasts app and the iTunes Store's podcast directory.

This provider is useful for:

- Identifying podcast episodes from audio files
- Retrieving show-level metadata (show name, author, genre)
- Downloading podcast artwork for embedding or saving alongside files
- Matching episode titles and durations against the Apple Podcasts catalog

The provider uses the public `https://itunes.apple.com/search` and `https://itunes.apple.com/lookup` endpoints with `media=podcast` and `entity=podcastEpisode` parameters.

---

## Authentication

**No authentication is required.** The Apple Podcasts provider uses Apple's public iTunes Search API, which does not require an API key, token, or any form of registration.

The provider is available immediately after installation with no additional setup.

> **Note:** Apple applies rate limiting to the iTunes Search API. MeedyaManager's built-in rate limiter automatically respects these limits (approximately 20 requests per minute). Under normal usage you will never hit the limit.

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required for this provider.

### Settings File (`settings.json5`)

You can optionally configure the provider's behaviour in `config/settings.json5`:

```json5
{
  providers: {
    apple_podcasts: {
      // Enable or disable this provider (default: true)
      enabled: true,

      // Preferred storefront/country code for regional catalog results
      // Uses ISO 3166-1 alpha-2 codes (e.g., "US", "GB", "AU", "DE")
      country: "GB",

      // Maximum number of results to return per search (1-200, default: 10)
      result_limit: 10,

      // Whether to search episodes in addition to shows (default: true)
      search_episodes: true,
    }
  }
}
```

### Storefront / Region

The `country` parameter determines which regional Apple Podcasts catalog is searched. Podcast availability varies by country. If a podcast is not found with one country code, try `"US"` as a fallback — the US catalog tends to have the broadest listing.

---

## Available Data

The Apple Podcasts provider returns the following standard metadata fields:

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Episode title | `"S2E5: The Great Escape"` |
| `artist` | Podcast author / host | `"Jane Smith"` |
| `album` | Podcast show name | `"True Crime Weekly"` |
| `genre` | Podcast genre(s) | `"True Crime"` |
| `year` | Release year of the episode | `"2025"` |
| `track_num` | Episode number (when identifiable) | `"5"` |
| `episode_title` | Episode title (same as title) | `"S2E5: The Great Escape"` |
| `show` | Podcast show name | `"True Crime Weekly"` |

---

## Custom Tags

In addition to standard fields, the provider writes the following custom tags to media files:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_apple_podcast_id` | Apple Podcasts show or episode ID | `"1234567890"` |
| `custom_apple_podcast_url` | Direct link to the show/episode on Apple Podcasts | `"https://podcasts.apple.com/gb/podcast/id1234567890"` |
| `custom_apple_podcast_feed_url` | RSS feed URL for the podcast show | `"https://feeds.example.com/truecrime.xml"` |
| `custom_apple_podcast_duration_ms` | Episode duration in milliseconds | `"2580000"` |

These custom tags are stored in the file using the provider's `extra_tags` mechanism and can be referenced in rename rules using the `{custom_apple_podcast_feed_url}` syntax.

---

## Cover Art

The Apple Podcasts provider supplies **static JPEG cover art** at **600x600 pixels**.

- The artwork URL is extracted from the API's `artworkUrl600` field
- Image format: JPEG
- Resolution: 600 x 600 pixels
- The image is saved as `FrontCover.jpg` alongside the media file
- If embedding is enabled, the image is also embedded in the file's tags

> **Note:** Apple Podcasts artwork is typically lower resolution than music artwork from the iTunes Store. If you need higher-resolution podcast artwork, consider using the RSS feed URL (stored in `custom_apple_podcast_feed_url`) to fetch the original image from the podcast's own feed.

---

## Troubleshooting

### Provider shows "Available" but returns no results

- **Check the country code.** Some podcasts are region-restricted. Try setting `country: "US"` in your settings.
- **Check your search query.** The iTunes Search API performs keyword matching — ensure your file's metadata or filename contains enough identifying information (show name, episode title).
- **Check the genre.** The API only searches podcasts. If your file is classified as `"Music"` by MeedyaManager's media classifier, this provider will not be queried. Ensure the file is classified as `"Podcast"` or set `media_class` manually.

### Rate limit warnings in logs

The iTunes Search API limits requests to approximately 20 per minute. If you see rate-limit warnings:

1. MeedyaManager automatically queues and retries requests — no action needed
2. If processing a large batch of podcast files, expect slower throughput
3. You can reduce `result_limit` to minimise API calls per file

### Episode matching is inaccurate

Episode matching relies on title similarity and duration comparison. For best results:

- Ensure your podcast files have accurate title metadata
- Files tagged with tools like MusicBrainz Picard or MP3tag will match more reliably
- The `custom_apple_podcast_duration_ms` tag can help disambiguate episodes with similar titles

---

## Legal Notes

- The Apple Podcasts provider uses Apple's **iTunes Search API**, which is publicly available and does not require a licence agreement for personal or non-commercial use.
- Apple, iTunes, and Apple Podcasts are trademarks of Apple Inc.
- Podcast metadata and artwork are the property of their respective owners and publishers.
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media library. No content is redistributed.
- For full terms, see: [Apple iTunes Affiliate Resources](https://affiliate.itunes.apple.com/resources/)

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
