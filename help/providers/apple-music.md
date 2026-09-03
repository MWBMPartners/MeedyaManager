# 🍎 Apple Music Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **Apple Music** metadata provider in MeedyaManager: what it actually queries (the public, unauthenticated **iTunes Search API** — not the JWT-authenticated Apple Music/MusicKit API), and what data it returns.

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
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). `AppleMusicProvider`
> is real, compiled, and unit-tested (against canned JSON fixtures — there is no wiremock/live-HTTP
> test for it), but nothing in the shipped CLI or GUI constructs it.

---

## Overview

`AppleMusicProvider` (`crates/mm-providers/src/music/mod.rs`) searches the public, unauthenticated
**iTunes Search API** (`https://itunes.apple.com/search?media=music&entity=song`) — the same
endpoint every other "Apple" provider in this crate uses (Apple TV, iTunes Store, Apple Podcasts).

The struct also has a `country` field and no other configuration. Its own doc comment says:

> "Auth: None (JWT for full Apple Music API — JWT path stubbed for M5)"

There is **no JWT signing, no ES256 key handling, and no Apple Developer Program / MusicKit
integration anywhere in the codebase.** The `jsonwebtoken` crate is declared as a workspace
dependency but is not imported or used by this provider (or by anything else in `mm-providers`).
The provider is always enabled — no key, team ID, or developer account is ever required or
checked.

**What it actually returns:** whatever the iTunes Search API's `song` entity returns — title,
artist, album, genre, release year, track/disc numbers, a content-advisory flag, duration, and
JPEG artwork. It does **not** return ISRC codes, composer credits, or animated (MP4) cover art —
none of those appear anywhere in the parser.

---

## Authentication

**None.** `AppleMusicProvider` is unconditionally enabled and never checks a credential. There is
no Apple Developer Program membership, MusicKit key, Team ID, or Key ID to obtain, because none of
that is used.

---

## Configuration

### Environment Variables (`.env`)

None are read. `APPLE_MUSIC_TEAM_ID` / `APPLE_MUSIC_KEY_ID` / `APPLE_MUSIC_PRIVATE_KEY` are not
consulted anywhere in the codebase.

### Settings (`settings.json5`)

There is no per-provider `settings.json5` schema for Apple Music — `enabled`, `priority`, and
`storefront` are not real settings; nothing reads them. The only real input is the constructor's
`country` argument (an ISO 3166-1 alpha-2 code, e.g. `"us"`, `"gb"`) and an optional `base_url`
override used by the crate's own tests.

---

## Available Data

`AppleMusicProvider::search()` sends:

```text
GET {base}/search?term=<title[+artist]>&media=music&entity=song&country=<country>&limit=<n>
```

| Field | iTunes Search response field | Example |
| ----- | ----------------------------- | ------- |
| `title` | `trackName` | "Bohemian Rhapsody" |
| `artist` | `artistName` | "Queen" |
| `album` | `collectionName` | "A Night at the Opera" |
| `genre` | `primaryGenreName` | "Rock" |
| `year` | first 4 digits of `releaseDate` | "1975" |
| `track_num` | `trackNumber` | 11 |
| `disc_num` | `discNumber` | 1 |
| duration (metadata) | `trackTimeMillis` / 1000 | 354.0 |
| track total (metadata) | `trackCount` | 12 |
| content advisory (metadata) | `explicitness` (mapped to `"explicit"`/`"clean"`) | "clean" |
| provider ID (metadata) | `trackId` | "1440833098" |

There is **no `isrc` field and no `composer` field** — the iTunes Search API's `song` entity
response this parser reads (`ItunesTrack`) has no such fields, so any earlier documentation
claiming ISRC or composer data from Apple Music was describing MusicKit, not this provider.

---

## Custom Tags

The provider's own custom-tag identifiers (`custom_apple_music_id`, `custom_apple_music_isrc`,
etc.) are not written by any code in this crate — the standard/custom tag mapping happens
elsewhere in `mm-core`'s tag-writing layer, which (per the reachability note above) is never fed
this provider's results in the shipped app. The values that *would* be available if that wiring
existed are the ones listed under [Available Data](#available-data): a provider ID, track total,
duration, and content advisory — no ISRC.

---

## Cover Art

- Source field: `artworkUrl100` (a 100×100 thumbnail URL).
- MeedyaManager's parser replaces the `100x100` segment of that URL with `3000x3000` to request a
  larger image, and returns **both** the upscaled URL and the original 100×100 URL as two
  `CoverArtInfo` entries — both are JPEG.
- There is **no animated cover art** of any kind (no square/portrait/spotlight MP4). That capability
  does not exist anywhere in `mm-providers` — no `editorialVideo` field is ever read, and no video
  file is ever downloaded for cover art by any provider in this codebase.

---

## Troubleshooting

### "Apple Music search returned 0 results"

- The track may not be available in the configured `country` storefront — construct a new
  `AppleMusicProvider` with a different country code.
- The iTunes Search API does plain keyword matching; very unusual titles/artists may need
  simplifying.

### Expecting ISRC, composer, or animated cover art

**This is expected to be absent.** See [Available Data](#available-data) and
[Cover Art](#cover-art) — none of these exist in this provider's implementation. They would
require a genuine MusicKit (JWT-authenticated) integration, which has not been built.

---

## Legal Notes

- The iTunes Search API is a public, unauthenticated Apple service; usage is subject to Apple's
  general terms for that API.
- This provider does **not** use the Apple Music API / MusicKit and therefore is not subject to
  the Apple Developer Program License Agreement for that API specifically.
- Cover art and metadata are the property of their respective rights holders.
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media
  library; this does not imply endorsement by Apple Inc.

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
