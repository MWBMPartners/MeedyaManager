# 🎬 TheMovieDB (TMDB) Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains what the **TMDb** metadata provider actually does in MeedyaManager: a real, API-key-gated client against `/3/search/multi`, with a much smaller field set than earlier documentation described.

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
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). `TmdbProvider` is
> real and compiled, tested against canned JSON fixtures (no wiremock/live-HTTP test exists for
> it), but nothing in the shipped CLI or GUI constructs it. `MM_TMDB_API_KEY` **is** parsed into
> `mm_core::config::AppConfig` at startup, but no CLI/GUI command currently constructs a
> `TmdbProvider` with that value — setting it has no effect on a lookup today.

---

## Overview

`TmdbProvider` (`crates/mm-providers/src/video/mod.rs`) queries TMDb's **`/3/search/multi`**
endpoint, which returns a mixed feed of movies, TV shows, and people distinguished by a
`media_type` field. MeedyaManager sends only `api_key`, `query`, `page=1`, and an optional `year`
— there is no `language`, `region`, `include_credits`, or `include_external_ids` parameter ever
sent, and no separate request is ever made for cast/crew or external IDs.

---

## Authentication

**A free TMDb API key is required.**

1. Create an account at [themoviedb.org/signup](https://www.themoviedb.org/signup).
2. Go to [themoviedb.org/settings/api](https://www.themoviedb.org/settings/api) and request an
   API key.
3. Use the **API Key (v3 auth)** value — MeedyaManager sends it as the `api_key` query parameter,
   not as a v4 Bearer token.

### How MeedyaManager reads it today

`TmdbProvider::new(api_key: Option<String>)` takes the key directly as a constructor argument.
Separately, `MM_TMDB_API_KEY` **is** read into `mm_core::config::AppConfig` at startup
(`crates/mm-core/src/config/mod.rs`) — but as the banner above explains, nothing currently threads
that config value into a constructed `TmdbProvider`. The generic 4-tier `CredentialStore` in
`crates/mm-providers/src/credentials.rs` would also resolve `MM_TMDB_*` if something called it for
this provider; nothing does yet. Its tier 4 is a **plain JSON file**, not encrypted (issue #209).

---

## Configuration

### Environment Variables (`.env`)

```env
MM_TMDB_API_KEY=your_api_key_here
```

Parsed into `AppConfig` (see above) but not yet used to construct a working provider.

### Settings File (`settings.json5`)

There is no per-provider `settings.json5` schema for TMDb — `language`, `region`,
`include_credits`, `include_external_ids`, and `result_limit` are not real settings; nothing reads
them, and the features they'd control (localisation, cast/crew, IMDb-ID cross-referencing) do not
exist in this provider's request. The only real input is the constructor's `api_key` and an
optional `base_url` override used by the crate's own tests.

---

## Available Data

| Field | Response field | Notes |
| ----- | --------------- | ----- |
| `title` | `title` (movies) or `name` (TV) | |
| `year` | first 4 digits of `release_date` or `first_air_date` | |
| cover art | `poster_path`, requested at `original` and `w500` | both JPEG |
| score | `vote_average` / 10, clamped to 0.0-1.0 | |
| provider ID (metadata) | `id` | |
| overview (metadata) | `overview`, when non-empty | plot summary text |
| media type (metadata) | `media_type` | e.g. `"movie"`, `"tv"` |

There is **no `artist`/`director`, `genre`, `season`, `episode`, or IMDb-ID field** —
`/3/search/multi`'s response does carry a `genre_ids` array, but this parser reads it into a
field marked `#[allow(dead_code)]` and never turns it into a genre string or attaches it to the
result. No second request is ever made to resolve genre names, credits, or `external_ids`.

---

## Custom Tags

Any `custom_tmdb_*` tag would come from the metadata fields above once tag-writing wiring exists
for this provider (see the reachability banner). There is **no** `custom_tmdb_imdb_id` — no
external-IDs request is ever made — and no `custom_tmdb_rating` distinct from the internal `score`
value.

---

## Cover Art

Two `CoverArtInfo` entries per result, both derived from `poster_path`:

| Size | URL | Notes |
| ---- | --- | ----- |
| `original` | `https://image.tmdb.org/t/p/original{poster_path}` | no known width/height |
| `w500` | `https://image.tmdb.org/t/p/w500{poster_path}` | 500×750 |

No other TMDb image size (`w92`, `w185`, `w780`) is ever requested, and there is no configurable
"preferred size" — the two above are always both returned when a poster exists.

---

## Troubleshooting

### "Missing credentials" — provider not available

`TmdbProvider::new(None)` — no `api_key` was supplied. There is no environment variable or
`settings.json5` field that reaches this provider today; you must pass the key directly to the
constructor (see [Authentication](#authentication)).

### HTTP 429 from TMDb

Mapped directly to `ProviderError::RateLimited("tmdb")`. **MeedyaManager's shared rate limiter is
not consulted before this request** — only MusicBrainz, ISRC, and ISWC go through the shared
`governor` limiter; TMDb has a configured default (`tmdb` → 40 RPM in
`crates/mm-providers/src/rate_limiter.rs::default_rpm_for`) that nothing in the request path
actually checks.

### Expecting cast/crew, IMDb ID, or localisation in another language

**This is expected to be absent.** None of `include_credits`, `include_external_ids`, or
`language` is a real setting — see [Available Data](#available-data). TMDb's response is always
requested in TMDb's default language for the query, with no `language=`/`region=` parameter sent.

---

## Legal Notes

- TMDb provides a **free API** for personal and non-commercial use under its [Terms of Use](https://www.themoviedb.org/terms-of-use).
- Attribution: "This product uses the TMDB API but is not endorsed or certified by TMDB."
- Poster images are the property of their respective rights holders (studios, networks, distributors).
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media library. No content is redistributed.

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
