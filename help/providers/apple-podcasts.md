# 🎙️ Apple Podcasts Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains what the **Apple Podcasts** metadata provider actually does in MeedyaManager: a **show-level** iTunes Search API lookup, not an episode-level one.

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
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`).
> `ApplePodcastsProvider` is real and compiled, tested against canned JSON fixtures (no
> wiremock/live-HTTP test exists for its `search()` beyond a `disabled` check), but nothing in the
> shipped CLI or GUI constructs it.

---

## Overview

`ApplePodcastsProvider` (`crates/mm-providers/src/podcasts/mod.rs`) searches Apple's iTunes Search
API with `media=podcast&entity=podcast` — **not** `entity=podcastEpisode`. This means it matches
**podcast shows**, not individual episodes: every field it returns is show-level (show title,
author, genre, episode count), never an episode title, episode number, or per-episode duration.

---

## Authentication

**No authentication is required.** The endpoint is Apple's public iTunes Search API.

---

## Configuration

### Environment Variables (`.env`)

None are required for this provider.

### Settings File (`settings.json5`)

There is no per-provider `settings.json5` schema for Apple Podcasts — `result_limit` and
`search_episodes` are not real settings (there is no episode search to toggle in the first place).
The only real input is the constructor's `country` argument (ISO 3166-1 alpha-2, e.g. `"US"`) and
an optional `base_url` override used by the crate's own tests.

---

## Available Data

```text
GET {base}/search?term=<title or artist>&media=podcast&entity=podcast&country=<country>&limit=<n>
```

| Field | Response field | Notes |
| ----- | --------------- | ----- |
| `title` | `collectionName` | the **podcast show name**, not an episode title |
| `artist` | `artistName` | podcast author/network |
| `genre` | `primaryGenreName` | |
| `year` | first 4 digits of `releaseDate` | the show's release-date field as reported by iTunes, not a specific episode's air date |
| cover art | `artworkUrl600` (primary), `artworkUrl100` (fallback) | both JPEG |
| provider ID (metadata) | `collectionId` | |
| feed URL (metadata) | `feedUrl` | |
| podcast URL (metadata) | `collectionViewUrl` | |
| episode count (metadata) | `trackCount` | total episodes in the show, not which one matched |

There is **no `episode_title`, `track_num` (as an episode number), or `show` field distinct from
`title`** — this provider has no concept of an individual episode at all.

---

## Custom Tags

Any `custom_apple_podcast_*` tag would come from the metadata fields above once tag-writing wiring
exists for this provider (see the reachability banner). There is no
`custom_apple_podcast_duration_ms` tag — episode duration is never returned by a show-level search.

---

## Cover Art

- Primary: `artworkUrl600` (600×600 JPEG)
- Fallback: `artworkUrl100` (100×100 JPEG), included alongside the primary when present

---

## Troubleshooting

### Provider shows "Available" but returns no results

- Check the `country` storefront — some podcasts are region-restricted.
- The iTunes Search API does keyword matching against the search term (built from title and/or
  artist); ensure the query has enough identifying text.

### Expecting episode-level matching (episode title, episode number, duration)

**This is expected to be absent.** This provider only ever returns show-level results — see
[Available Data](#available-data). There is no episode-matching feature in this codebase.

### Rate limit warnings

**MeedyaManager's shared rate limiter is not consulted for Apple Podcasts** — only MusicBrainz,
ISRC, and ISWC requests go through the shared `governor` limiter. A non-2xx response from iTunes
is surfaced directly as `ProviderError::NetworkError`.

---

## Legal Notes

- This provider uses Apple's **iTunes Search API**, publicly available without a licence agreement for personal or non-commercial use.
- Apple, iTunes, and Apple Podcasts are trademarks of Apple Inc.
- Podcast metadata and artwork are the property of their respective owners and publishers.
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media library. No content is redistributed.

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
