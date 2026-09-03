# 📺 Apple TV Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains what the **Apple TV** metadata provider actually does in MeedyaManager: it searches **movies only** — it never queries TV shows, despite its name and earlier documentation.

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

> ⚠️ **Not reachable from the app today.** `meedya lookup` (the CLI command) is a permanent stub
> that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). `AppleTvProvider`
> is real and compiled, tested against canned JSON fixtures (no wiremock/live-HTTP test exists for
> it), but nothing in the shipped CLI or GUI constructs it.

---

## Overview

`AppleTvProvider` (`crates/mm-providers/src/video/mod.rs`) queries Apple's public iTunes Search API
with:

```text
GET {base}/search?term=<title>&media=movie&country=<country>&limit=<n>
```

**Only `media=movie` is ever sent** — there is no `entity=tvEpisode` (or any TV-related) parameter
anywhere in this provider's request. Despite its name and the doc comment's mention of
`media=tvEpisode`, this provider never searches or matches TV shows, seasons, or episodes.

---

## Authentication

**No authentication is required.** No API key, token, or developer account is used.

---

## Configuration

### Environment Variables (`.env`)

None are required for this provider.

### Settings File (`settings.json5`)

There is no per-provider `settings.json5` schema for Apple TV — `result_limit`,
`search_both_types`, and `artwork_size` are not real settings; nothing reads them, and there is no
"search both movies and TV" toggle because TV is never searched at all. The only real inputs are
the constructor's `country` argument (ISO 3166-1 alpha-2, e.g. `"US"`) and an optional `base_url`
override used by the crate's own tests.

---

## Available Data

| Field | Response field | Notes |
| ----- | --------------- | ----- |
| `title` | `trackName` | |
| `artist` | `artistName` | labelled "director" in the source's own comment, but this is whatever iTunes returns in `artistName` for a movie — not a verified director field |
| `album` | `collectionName` | |
| `genre` | `primaryGenreName` | |
| `year` | first 4 digits of `releaseDate` | |
| cover art | `artworkUrl100`, rescaled to 600×600 plus the original 100×100 | both JPEG |
| content advisory (metadata) | `contentAdvisoryRating` | e.g. `"PG-13"` |
| duration (metadata) | `trackTimeMillis` / 1000 | |
| provider ID (metadata) | `trackId` | |

There is **no `show`, `season`, `episode`, or `episode_title` field** — this provider has no
concept of a TV episode at all, and there is **no separate `director` field** distinct from the
generic `artist` mapping described above.

---

## Custom Tags

Any `custom_apple_tv_*` tag would come from the metadata fields above once tag-writing wiring
exists for this provider (see the reachability banner). There is no
`custom_apple_tv_description` tag — no synopsis/overview field is ever read by this parser.

---

## Cover Art

- Source field: `artworkUrl100` (a 100×100 thumbnail URL).
- The parser replaces `100x100` with `600x600` in that URL and returns both the upscaled 600×600
  image and the original 100×100 image, both JPEG.
- This is **not** 3000×3000 — that resolution is what `AppleMusicProvider`'s cover-art scaling
  uses; Apple TV's scaling targets 600×600.

---

## Troubleshooting

### Provider shows "Available" but returns no results

- Movie licensing is region-dependent — try a different `country`.
- Only movies are searched; a TV show title will never match here regardless of configuration.

### Expecting TV show / season / episode matching

**This is expected to be absent.** `AppleTvProvider` only ever sends `media=movie` — see
[Overview](#overview). There is no code path that searches TV content in this provider.

### Rate limit warnings

**MeedyaManager's shared rate limiter is not consulted for Apple TV** — only MusicBrainz, ISRC,
and ISWC requests go through the shared `governor` limiter. A non-2xx response from iTunes is
surfaced directly as `ProviderError::NetworkError`.

---

## Legal Notes

- This provider uses Apple's **iTunes Search API**, publicly available without a licence agreement for personal, non-commercial use.
- Apple, Apple TV, iTunes, and related marks are trademarks of Apple Inc.
- Movie metadata and artwork are the property of their respective rights holders (studios, distributors).
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media library. No content is redistributed.

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
