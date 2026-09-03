# 🎵 iTunes Store Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains what the **iTunes Store** metadata provider actually does in MeedyaManager. **This is a video (TV season) provider, not a music provider** — earlier documentation describing music track/album lookups, ISRC, track numbers, or disc numbers was describing a different provider entirely.

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

> ⚠️ **This provider searches TV seasons, not music.** `ItunesStoreProvider`
> (`crates/mm-providers/src/video/mod.rs`) is registered under `video_caps()` (`video_search:
> true`, `music_search: false`) and its own doc comment says: "Uses the same iTunes Search API as
> AppleTvProvider but with the `tvShow` entity to fetch TV series." It queries
> `media=tvShow&entity=tvSeason` and reuses `AppleTvProvider`'s exact response parser
> (`parse_itunes_video`) — the same minimal field set as the [Apple TV provider](apple-tv.md), not
> the music-catalogue fields (album artist, track/disc numbers, ISRC) an older version of this page
> described.
>
> **Not reachable from the app today.** `meedya lookup` (the CLI command) is a permanent stub that
> prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`).

---

## Overview

`ItunesStoreProvider` sends:

```text
GET {base}/search?term=<title>&media=tvShow&entity=tvSeason&country=<country>&limit=<n>
```

and parses the response with the identical `ItunesVideoResult` struct `AppleTvProvider` uses (see
[apple-tv.md](apple-tv.md)) — so its fields, limitations, and cover-art behaviour are the same as
that provider's, just scoped to TV-season-shaped results from iTunes rather than movies.

---

## Authentication

**No authentication is required.** No API key, token, or developer account is used.

---

## Configuration

### Environment Variables (`.env`)

None are required for this provider.

### Settings File (`settings.json5`)

There is no per-provider `settings.json5` schema for iTunes Store — `result_limit`,
`search_albums`, and `artwork_size` are not real settings; nothing reads them (and "search
albums" describes a music feature this provider does not have, since it queries TV seasons). The
only real inputs are the constructor's `country` argument (ISO 3166-1 alpha-2, e.g. `"US"`) and an
optional `base_url` override used by the crate's own tests.

---

## Available Data

| Field | Response field | Notes |
| ----- | --------------- | ----- |
| `title` | `trackName` | |
| `artist` | `artistName` | |
| `album` | `collectionName` | |
| `genre` | `primaryGenreName` | |
| `year` | first 4 digits of `releaseDate` | |
| cover art | `artworkUrl100`, rescaled to 600×600 plus the original 100×100 | both JPEG |
| content advisory (metadata) | `contentAdvisoryRating` | |
| duration (metadata) | `trackTimeMillis` / 1000 | |
| provider ID (metadata) | `trackId` | |

There is **no `album_artist`, `track_num`, `total_tracks`, `disc_num`, `total_discs`, or `isrc`
field** — none of these appear anywhere in the shared parser this provider uses. Those fields
belong to a music catalogue lookup, and this provider does not perform one.

---

## Custom Tags

Any custom tag here would come from the metadata fields above once tag-writing wiring exists for
this provider (see the reachability banner). There is no `custom_itunes_collection_id` distinct
from the generic provider-ID metadata slot.

---

## Cover Art

Identical mechanism to [Apple TV](apple-tv.md#cover-art): `artworkUrl100` rescaled to 600×600, plus
the original 100×100 image, both JPEG — **not** 3000×3000.

---

## Troubleshooting

### Provider shows "Available" but returns no results

- TV licensing/availability is region-dependent — try a different `country`.
- Only `tvShow`/`tvSeason` results are searched; a music track or album title will never match
  here.

### Expecting music metadata (ISRC, track number, disc number, album artist)

**This is expected to be absent.** This provider searches TV seasons, not music — see the banner
at the top of this page and [Available Data](#available-data). If you were looking for a
music-catalogue lookup against Apple's iTunes Search API, that is
[apple-music.md](apple-music.md)'s `AppleMusicProvider` (`media=music&entity=song`).

### Rate limit warnings

**MeedyaManager's shared rate limiter is not consulted for iTunes Store** — only MusicBrainz,
ISRC, and ISWC requests go through the shared `governor` limiter. A non-2xx response from iTunes
is surfaced directly as `ProviderError::NetworkError`.

---

## Legal Notes

- This provider uses Apple's **iTunes Search API**, publicly available without a licence agreement for personal, non-commercial use.
- Apple and iTunes are trademarks of Apple Inc.
- TV metadata and artwork are the property of their respective rights holders (studios, networks, distributors).
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media library. No content is redistributed or made available to third parties.

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
