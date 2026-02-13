# 🎞️ IMDb Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)**

This guide explains how to configure and use the **IMDb** metadata provider in MeedyaManager.

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

The IMDb provider retrieves movie and TV show metadata from the **Internet Movie Database** (IMDb) — the world's most popular and comprehensive source of film, television, and celebrity information. Instead of using a paid API, this provider uses the **cinemagoer** library (formerly IMDbPY) to access IMDb data programmatically.

This provider is useful for:

- Looking up movie metadata (title, year, genres, IMDb ratings, vote counts)
- Retrieving TV series and episode information
- Obtaining IMDb IDs for cross-referencing with other databases (TMDB, TVDB, etc.)
- Accessing IMDb's authoritative rating and vote data
- Downloading movie/TV poster artwork

> **Important:** This provider has a **GPL-2.0 licence dependency**. See [Legal Notes](#legal-notes) for details.

---

## Authentication

**No API authentication is required.** The IMDb provider uses the **cinemagoer** Python library to access IMDb data without an API key or account.

However, the cinemagoer library is an **OPTIONAL dependency** that must be installed separately due to its GPL-2.0 licence.

### Installing cinemagoer

```bash
# Install cinemagoer (optional — only needed for IMDb provider)
pip install cinemagoer
```

If cinemagoer is not installed, the IMDb provider will gracefully report itself as **unavailable** and MeedyaManager will continue to function normally with all other providers.

You can verify the provider's status with:

```bash
python cli/runner.py --providers-list
```

If cinemagoer is installed, you will see:

```
imdb          video       No auth required    Available
```

If cinemagoer is not installed, you will see:

```
imdb          video       No auth required    Unavailable (cinemagoer not installed)
```

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required for this provider.

### Settings File (`settings.json5`)

You can optionally configure the provider's behaviour in `config/settings.json5`:

```json5
{
  providers: {
    imdb: {
      // Enable or disable this provider (default: true)
      // Set to false to skip even if cinemagoer is installed
      enabled: true,

      // Maximum number of search results to evaluate (1-20, default: 5)
      result_limit: 5,

      // Whether to fetch full movie/show details (default: true)
      // Fetching details requires additional page loads per result,
      // which increases lookup time but provides more complete metadata
      fetch_full_details: true,

      // Timeout in seconds for IMDb page loads (default: 30)
      // Increase if on a slow connection; decrease if speed is a priority
      request_timeout: 30,
    }
  }
}
```

---

## Available Data

The IMDb provider returns the following standard metadata fields:

### For Movies

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Movie title | `"The Shawshank Redemption"` |
| `director` | Director name | `"Frank Darabont"` |
| `artist` | Director name (mapped for consistency) | `"Frank Darabont"` |
| `genre` | Genres (comma-separated) | `"Drama"` |
| `year` | Release year | `"1994"` |

### For TV Shows / Episodes

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Episode title | `"Ozymandias"` |
| `show` | TV series name | `"Breaking Bad"` |
| `season` | Season number | `"5"` |
| `episode` | Episode number | `"14"` |
| `episode_title` | Episode title | `"Ozymandias"` |
| `year` | Episode air year | `"2013"` |

---

## Custom Tags

In addition to standard fields, the provider writes the following custom tags to media files:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_imdb_id` | IMDb title ID (tt-format) | `"tt0111161"` |
| `custom_imdb_url` | Direct link on IMDb | `"https://www.imdb.com/title/tt0111161/"` |
| `custom_imdb_rating` | IMDb user rating (0.0-10.0 scale) | `"9.3"` |
| `custom_imdb_votes` | Total number of IMDb user votes | `"2800000"` |
| `custom_imdb_genres` | Full genre list from IMDb | `"Drama"` |

The `custom_imdb_id` is the universally recognised IMDb title identifier. It follows the format `tt` followed by 7-8 digits (e.g., `tt0111161`). This ID is invaluable for:

- Cross-referencing with TMDB (which accepts IMDb IDs for lookups)
- Linking to IMDb pages from your media player
- Ensuring unique identification of movies/shows regardless of title variations

The `custom_imdb_rating` and `custom_imdb_votes` can be used in rename rules for quality-based organisation:

```json5
// Example: Sort highly-rated movies into a "Top Rated" folder
rename_format: "Movies/{custom_imdb_rating >= 8.0 ? 'Top Rated' : 'All'}/{title} ({year}).{extension}"
```

---

## Cover Art

The IMDb provider supplies **static JPEG poster images** extracted from IMDb pages.

- The poster URL is retrieved from the movie/show's primary image on IMDb
- Image format: JPEG
- Resolution: Varies (typically 500-1000 pixels wide; IMDb does not guarantee specific dimensions)
- The image is saved as `FrontCover.jpg` alongside the media file
- If embedding is enabled, the image is also embedded in the file's metadata

> **Note:** IMDb poster images are generally lower resolution than those from TMDB. For the best cover art quality, use TMDB as your primary poster source and IMDb for its authoritative ratings data.

---

## Troubleshooting

### Provider shows "Unavailable (cinemagoer not installed)"

The cinemagoer library is not installed. Install it with:

```bash
pip install cinemagoer
```

If you are using MeedyaManager's virtual environment, ensure it is activated first:

```bash
source venv/bin/activate  # macOS/Linux
venv\Scripts\activate     # Windows
pip install cinemagoer
```

### Provider shows "Unavailable" even after installing cinemagoer

- Verify the installation: `python -c "import imdb; print(imdb.VERSION)"`
- If using a release package (Nuitka build), cinemagoer may need to be installed into the bundled environment — check the release notes for instructions
- Ensure the `enabled` setting is not set to `false` in your configuration

### Searches are slow

The cinemagoer library works by loading and parsing IMDb web pages, which is inherently slower than a dedicated API. To improve speed:

1. Reduce `result_limit` to minimise the number of pages loaded
2. Set `fetch_full_details: false` to skip detailed lookups (you will get less metadata but faster results)
3. Reduce `request_timeout` if you prefer to skip slow responses rather than wait
4. Consider using TMDB as your primary video provider and IMDb as a supplementary source for ratings data

### "Connection error" or "Timeout" during searches

- Check your internet connection
- IMDb may be temporarily unreachable — the provider will retry automatically
- Increase `request_timeout` if you are on a slow connection
- The cinemagoer library accesses IMDb's web pages directly, so corporate firewalls or content filters may block these requests

### Results are missing or incomplete

The cinemagoer library depends on IMDb's page structure. If IMDb changes their website layout:

1. Update cinemagoer to the latest version: `pip install --upgrade cinemagoer`
2. Check the [cinemagoer GitHub](https://github.com/cinemagoer/cinemagoer) for known issues
3. Report persistent issues on the MeedyaManager issue tracker

---

## Legal Notes

### GPL-2.0 Licence Notice

> **The cinemagoer library is licensed under the GNU General Public License v2.0 (GPL-2.0).** This is a copyleft licence that has implications for software distribution.

**What this means for MeedyaManager users:**

- **Personal use:** You can freely install and use cinemagoer alongside MeedyaManager for personal media library management. There are no restrictions on personal use.
- **Separate installation:** cinemagoer is **not bundled** with MeedyaManager. It is an optional dependency that users install separately. This design choice ensures MeedyaManager's own licence is not affected by GPL-2.0 requirements.
- **If cinemagoer is not installed:** The IMDb provider simply reports itself as unavailable. All other MeedyaManager functionality continues to work normally.

### IMDb Data Usage

- IMDb data is provided by IMDb.com, Inc., a subsidiary of Amazon.com, Inc.
- IMDb is a trademark of IMDb.com, Inc.
- Information retrieved is for personal use only — redistribution of IMDb data requires a commercial licence from IMDb.
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media library. No data is redistributed.
- For IMDb's conditions of use, see: [imdb.com/conditions](https://www.imdb.com/conditions)

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
