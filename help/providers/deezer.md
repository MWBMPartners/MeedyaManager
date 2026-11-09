# 🎶 Deezer Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **Deezer** metadata provider in MeedyaManager. Deezer's public API requires no authentication, making it one of the simplest providers to use — but its actual query syntax and returned fields differ from earlier documentation.

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
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). `DeezerProvider`
> is real and compiled, tested against canned JSON fixtures (no wiremock/live-HTTP test exists for
> it), but nothing in the shipped CLI or GUI constructs it.

---

## Overview

`DeezerProvider` (`crates/mm-providers/src/music/mod.rs`) uses the **Deezer Public API**, which
requires no authentication. It supports two request shapes:

- **ISRC lookup** (used when the query carries an ISRC): `GET {base}/track/isrc:<isrc>` — a
  direct hit on a single track.
- **Keyword search** (otherwise): `GET {base}/search?q=<term>&limit=<n>`, where `<term>` is simply
  `"<title> <artist>"` concatenated with a space — **there are no `track:`/`artist:`/`album:`
  field-prefixed query terms**; that syntax is not sent anywhere in this provider.

---

## Authentication

Deezer's public search API requires **no authentication** — no API keys, tokens, or accounts.

### No setup required

`DeezerProvider::new()` takes no credentials at all.

> **Note:** Deezer does offer authenticated endpoints for user-specific data (playlists, favourites, etc.), but MeedyaManager only ever calls the public `/search` and `/track/isrc:` endpoints, which require no credentials.

---

## Configuration

### Environment Variables (`.env`)

None are required or read for Deezer.

### Settings (`settings.json5`)

There is no per-provider `settings.json5` schema for Deezer — `enabled` and `priority` are not
real settings; nothing reads them. `DeezerProvider::new()` takes no arguments (there is an
optional `base_url` override used by the crate's own tests).

---

## Available Data

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `data[].title` | "Bohemian Rhapsody" |
| `artist` | `data[].artist.name` | "Queen" |
| `album` | `data[].album.title` | "A Night at the Opera" |
| `isrc` | `data[].isrc` | "GBUM71029604" |
| score | `data[].rank` (0-100,000, normalised to 0.0-1.0) | 0.62 |
| content advisory (metadata) | `data[].explicit_lyrics` mapped to `"explicit"`/`"clean"` | "clean" |
| duration (metadata) | `data[].duration` (seconds, used as-is) | 354 |
| provider ID (metadata) | `data[].id` | "3157894" |

There is **no `year` field** — the Deezer track object this parser reads has no release-date
field at all — and **no `track_num`/`disc_num` field**; Deezer's search response doesn't carry
track/disc position and this parser doesn't read one. Any earlier documentation claiming those
three fields from Deezer was inaccurate.

---

## Custom Tags

Any `custom_deezer_*` tag would come from the metadata fields above once tag-writing wiring exists
for this provider (see the reachability banner). The ISRC itself is a **standard tag** (not
custom), since ISRC is part of MeedyaManager's standard tag map.

---

## Cover Art

| Type | Format | Resolution | Source field |
| ---- | ------ | ---------- | ------------ |
| Primary | JPEG | 1000×1000 | `album.cover_xl` |
| Fallback | JPEG | 250×250 | `album.cover_medium` |

Both are returned when present (`cover_xl` first); MeedyaManager does not request `cover_small` or
`cover_big`.

---

## Troubleshooting

### "Deezer search returned 0 results"

- The track may not be available in the Deezer catalogue (regional availability varies).
- The query is a plain `"<title> <artist>"` string with no field weighting — very generic titles
  may return unrelated matches or nothing useful.

### HTTP 429 — Rate limit exceeded

**Cause:** Deezer enforces rate limits on its public API. **MeedyaManager's shared rate limiter is
not consulted for Deezer** — only MusicBrainz, ISRC, and ISWC requests go through the shared
`governor` limiter. A non-2xx response (including 429) from Deezer is surfaced directly as
`ProviderError::NetworkError` with the HTTP status included; there is no automatic retry or
client-side throttling for this provider.

### Missing ISRC, or no year/track number in results

**This is expected** for missing ISRC on some catalogue entries. A missing year or track number is
**always** expected — see [Available Data](#available-data): this parser never populates either
field, regardless of what the track actually has.

---

## Legal Notes

- The Deezer Public API is provided under the [Deezer API Terms of Use](https://developers.deezer.com/termsofuse).
- No account or API key is required for the public search/lookup endpoints used here.
- Cover art and metadata are the property of their respective rights holders.
- MeedyaManager stores provider IDs as custom metadata tags for reference and linking; this does
  not imply endorsement by Deezer SA.

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
