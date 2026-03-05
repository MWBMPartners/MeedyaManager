# 🎬 TheMovieDB (TMDB) Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains how to configure and use the **TMDB** (TheMovieDB) metadata provider in MeedyaManager.

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

The TMDB provider retrieves movie and TV show metadata from **TheMovieDB** (themoviedb.org), one of the most comprehensive community-maintained databases of film and television information. TMDB is widely used by media centre applications (Plex, Kodi, Jellyfin, Emby) and provides rich, structured data.

This provider is useful for:

- Looking up movie metadata (title, director, year, genres, ratings, cast, crew)
- Retrieving TV series information including season and episode details
- Downloading high-resolution movie posters and TV show artwork
- Cross-referencing with IMDb IDs for interoperability with other tools
- Obtaining audience ratings, overviews, and synopses

The provider uses the **TMDB API v3** at `https://api.themoviedb.org/3/`.

---

## Authentication

**A free API key is required.** TMDB provides free API access for personal and non-commercial use.

### Step-by-Step: Getting Your TMDB API Key

1. **Create a TMDB account** at [themoviedb.org/signup](https://www.themoviedb.org/signup)
2. **Verify your email address** via the confirmation link TMDB sends you
3. **Navigate to API settings:**
   - Click your avatar (top right) > **Settings**
   - Select **API** from the left sidebar
   - Or go directly to: [themoviedb.org/settings/api](https://www.themoviedb.org/settings/api)
4. **Request an API key:**
   - Click **Create** or **Request an API key**
   - Select **Developer** as the usage type
   - Fill in the application details (name: "MeedyaManager", type: "Personal", description: "Personal media library organiser")
   - Accept the Terms of Use
5. **Copy your API key (v3 auth)** — this is the long alphanumeric string, not the Bearer token

> **Important:** Use the **API Key (v3 auth)** value, not the API Read Access Token (v4). MeedyaManager uses v3 API authentication.

---

## Configuration

### Environment Variables (`.env`)

Add your TMDB API key to the `.env` file in the project root:

```env
# TMDB API key (v3 auth) — get it from themoviedb.org/settings/api
TMDB_API_KEY=your_api_key_here
```

### Settings File (`settings.json5`)

You can optionally configure the provider's behaviour in `config/settings.json5`:

```json5
{
  providers: {
    tmdb: {
      // Enable or disable this provider (default: true)
      enabled: true,

      // Preferred language for metadata (ISO 639-1, default: "en")
      // Determines the language of titles, overviews, and genre names
      language: "en",

      // Preferred region for release dates and certifications (ISO 3166-1, default: "GB")
      region: "GB",

      // Whether to fetch cast and crew data (default: true)
      // Adds director, cast names to results but requires an extra API call per result
      include_credits: true,

      // Whether to fetch external IDs like IMDb ID (default: true)
      include_external_ids: true,

      // Maximum number of search results to evaluate (1-20, default: 5)
      result_limit: 5,
    }
  }
}
```

### Alternative: API Key in Settings

If you prefer not to use a `.env` file, you can place the API key directly in `settings.json5`:

```json5
{
  providers: {
    tmdb: {
      api_key: "your_api_key_here",
    }
  }
}
```

> **Security note:** The `.env` approach is preferred because `.env` is git-ignored by default, preventing accidental commits of your API key.

---

## Available Data

The TMDB provider returns the following standard metadata fields:

### For Movies

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Movie title | `"Inception"` |
| `artist` | Director name | `"Christopher Nolan"` |
| `director` | Director name | `"Christopher Nolan"` |
| `genre` | Genres (comma-separated) | `"Science Fiction, Action, Adventure"` |
| `year` | Release year | `"2010"` |

### For TV Shows / Episodes

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Episode title | `"Ozymandias"` |
| `show` | TV series name | `"Breaking Bad"` |
| `season` | Season number | `"5"` |
| `episode` | Episode number | `"14"` |
| `episode_title` | Episode title | `"Ozymandias"` |
| `director` | Episode director | `"Rian Johnson"` |
| `genre` | Series genres | `"Drama, Crime, Thriller"` |
| `year` | Episode air year | `"2013"` |

---

## Custom Tags

In addition to standard fields, the provider writes the following custom tags to media files:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_tmdb_id` | TMDB movie or TV show ID | `"27205"` |
| `custom_tmdb_url` | Direct link on TMDB | `"https://www.themoviedb.org/movie/27205"` |
| `custom_tmdb_imdb_id` | Corresponding IMDb ID (if available) | `"tt1375666"` |
| `custom_tmdb_rating` | TMDB user rating (0.0-10.0 scale) | `"8.4"` |
| `custom_tmdb_overview` | Plot summary / synopsis | `"A thief who steals corporate secrets..."` |

The `custom_tmdb_imdb_id` is particularly valuable for cross-referencing with other databases and tools. Many media managers and players recognise IMDb IDs natively.

The `custom_tmdb_rating` can be used in rename rules — for example, to sort movies into folders by rating tier:

```json5
// Example: Sort by rating tier
rename_format: "Movies/{custom_tmdb_rating > 7 ? 'Highly Rated' : 'Standard'}/{title} ({year}).{extension}"
```

---

## Cover Art

The TMDB provider supplies **high-resolution static JPEG poster images** at original resolution.

- Movie posters and TV show posters are fetched from the TMDB image CDN
- Image URL format: `https://image.tmdb.org/t/p/original/{poster_path}`
- Image format: JPEG
- Resolution: Original resolution (typically 2000x3000 for movie posters)
- The image is saved as `FrontCover.jpg` alongside the media file
- If embedding is enabled, the image is also embedded in the file's metadata

Available image sizes from TMDB (configurable):

| Size | Typical Dimensions | Use Case |
| ---- | ------------------ | -------- |
| `w92` | 92 x 138 | Thumbnail |
| `w185` | 185 x 278 | Small display |
| `w500` | 500 x 750 | Medium display |
| `w780` | 780 x 1170 | Large display |
| `original` | ~2000 x 3000 | Full resolution (default) |

> **Tip:** If you have limited disk space or are embedding art into many files, consider configuring a smaller image size to reduce file sizes.

---

## Troubleshooting

### "Missing credentials" — Provider not available

- Ensure `TMDB_API_KEY` is set in your `.env` file (or in `settings.json5` under `providers.tmdb.api_key`)
- Verify the key is correct — copy it directly from [themoviedb.org/settings/api](https://www.themoviedb.org/settings/api)
- Check that your `.env` file is in the project root directory
- Run `python cli/runner.py --providers-list` to verify provider status

### "401 Unauthorized" errors in logs

- Your API key is invalid or has been revoked
- Log in to TMDB and check your API settings — you may need to regenerate the key
- Ensure you are using the **API Key (v3 auth)**, not the v4 Read Access Token

### Results are in the wrong language

- Set the `language` parameter in your provider config (e.g., `language: "en"` for English)
- TMDB falls back to the original language if a translation is not available
- Not all movies/shows have translations in all languages

### TV episode matching is inaccurate

- TMDB uses **absolute episode ordering** for some anime series, which differs from season-based ordering. Check TMDB's entry for the show to understand its episode structure.
- Ensure your files have `show`, `season`, and `episode` metadata populated for best matching
- Specials and bonus episodes may have different numbering on TMDB compared to TVDB or IMDb

### API rate limiting

TMDB allows approximately 40 requests per 10-second window. MeedyaManager's rate limiter handles this automatically. If processing a very large video library:

1. Processing will proceed at a throttled pace — no manual intervention needed
2. You can reduce `result_limit` and disable `include_credits` to reduce API calls per file
3. Check the logs for rate-limit retry messages

---

## Legal Notes

- TMDB provides a **free API** for personal and non-commercial use under its [Terms of Use](https://www.themoviedb.org/terms-of-use).
- Attribution: "This product uses the TMDB API but is not endorsed or certified by TMDB."
- Movie and TV metadata are contributed by TMDB's community and are available under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) licence for data fields.
- Poster images are the property of their respective rights holders (studios, networks, distributors).
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media library. No content is redistributed.

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
