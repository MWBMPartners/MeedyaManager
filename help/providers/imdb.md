# 🎞️ IMDb Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains how to configure and use the **"IMDb" provider** in MeedyaManager — which, in the actual code, is the **OMDb API**, not IMDb itself.

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

> ⚠️ **This is OMDb, not IMDb.** There is no IMDb client, IMDb scraper, or `cinemagoer`-style
> library anywhere in MeedyaManager. The Rust type behind this page is `OmdbProvider`
> (`crates/mm-providers/src/video/mod.rs`), with `id()` returning `"omdb"` and `display_name()`
> returning `"OMDb / IMDb"`. It calls the third-party **OMDb API** (`www.omdbapi.com`), which itself
> republishes a subset of IMDb's data under its own free/paid tiers. Everything below describes
> what `OmdbProvider` actually does.
>
> **Not reachable from the app today.** `meedya lookup` (the CLI command) is a permanent stub that
> prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). `OmdbProvider` is
> compiled and unit-tested (against canned JSON fixtures, not a live or mocked HTTP response), but
> nothing in the shipped CLI or GUI constructs it.

---

## Overview

`OmdbProvider` searches the **OMDb API** — a free/paid third-party API that serves a subset of
IMDb's catalogue — via a plain HTTP GET, and **requires an OMDb API key**. This provider is useful
for:

- Looking up movie metadata (title, year) by a search string
- Obtaining the OMDb/IMDb title ID (`imdbID`, `tt`-format) for cross-referencing
- Retrieving a poster URL

It does **not**:

- Access imdb.com directly, scrape any web page, or use any unofficial IMDb library
- Search TV shows or episodes — the request hard-codes `type=movie` (see
  [Available Data](#available-data))
- Return director, genre, rating, vote count, or cast/crew data — none of these fields exist on
  the parsed response type

---

## Authentication

**An OMDb API key is required.** OMDb's free tier is documented (in the source comment) as 1,000
requests/day; MeedyaManager does not verify this against a live account.

```bash
curl "https://www.omdbapi.com/?s=Inception&apikey=YOUR_KEY"
```

Get a key from [omdbapi.com/apikey.aspx](https://www.omdbapi.com/apikey.aspx).

### How MeedyaManager would use it

`OmdbProvider::new(api_key: Option<String>)` takes the key directly as a constructor argument.
There is **no `MM_OMDB_API_KEY` or `MM_IMDB_API_KEY` environment variable read anywhere** —
`mm_core::config::ProviderConfig` has no OMDb/IMDb field, and no call site in the CLI or GUI
constructs `OmdbProvider` with a credential today. If you want to use this provider before that
wiring lands, you would pass the key directly to `OmdbProvider::new(...)` in your own code.

If provider wiring is ever added, credentials would most likely resolve through the generic
4-tier `CredentialStore` (env `MM_OMDB_API_KEY` → in-memory config map → OS keyring → local
`credentials.json`) in `crates/mm-providers/src/credentials.rs` — note that tier 4 there is a
**plain JSON file on disk**, not the AES-256-GCM-encrypted bundle earlier project plans described
(issue #209).

---

## Configuration

### Environment Variables (`.env`)

None are read today (see above).

### Settings File (`settings.json5`)

There is no per-provider `settings.json5` schema for OMDb/IMDb. `enabled`, `result_limit`,
`fetch_full_details`, and `request_timeout` are not real settings — nothing in the codebase reads
them. The only real inputs are the constructor's `api_key` and an optional `base_url` override
(used by the crate's own tests to point at a mock server).

---

## Available Data

`OmdbProvider::search()` sends:

```text
GET {base}/?s=<title>&apikey=<key>&type=movie[&y=<year>]
```

`type=movie` is **hard-coded** — the source comment reads "Default to movies; could be
configurable", but nothing makes it configurable, so this provider never matches TV shows or
episodes. `<title>` falls back to `"<query.title> <query.artist>"` trimmed when no title is given
in the query (there is no dedicated free-text field).

The response is OMDb's `Search`-array shape. If the response carries an `Error` field (e.g. "Movie
not found!"), the whole call fails rather than returning zero results.

| Field | Response field | Notes |
| ----- | --------------- | ----- |
| `title` | `Search[].Title` | |
| `year` | `Search[].Year` | first 4 characters parsed as an integer |
| `cover_art` | `Search[].Poster` | skipped when OMDb returns the literal string `"N/A"` or empty |
| provider ID (metadata) | `Search[].imdbID` | stored under the generic provider-ID metadata key, e.g. `tt1375666` |
| media type (metadata) | `Search[].Type` | stored verbatim, e.g. `"movie"` |

There is **no** `director`, `artist`, `genre`, `episode`, `season`, `episode_title`, rating, or
vote-count field anywhere in `OmdbProvider`'s parser — the struct it deserializes into
(`OmdbSearchResult`) only has `imdbID`, `Title`, `Year`, `Poster`, and `Type`.

---

## Custom Tags

The only identifier this provider surfaces is the OMDb/IMDb title ID, stored in the generic
provider-ID metadata slot (not under a dedicated `custom_imdb_*` key — there is no such constant
anywhere in the crate).

| Value | Format | Example |
| ----- | ------ | ------- |
| IMDb title ID | `tt` + 7-8 digits | `tt0111161` |

There is no `custom_imdb_rating`, `custom_imdb_votes`, `custom_imdb_genres`, or `custom_imdb_url`
tag produced by this code — OMDb's free-tier `Search` response doesn't carry rating/vote data in
the first place (that lives on OMDb's separate `?i=<id>` detail endpoint, which this provider
never calls).

---

## Cover Art

The `Poster` field, when present and not `"N/A"`, is used as a single cover-art URL with no known
width or height (OMDb doesn't publish image dimensions in this response). There is no
higher-resolution variant request, no embedding-specific logic beyond what every video provider
shares, and no fallback image source.

---

## Troubleshooting

### Provider errors with "OMDb error: Movie not found!"

OMDb returned a genuine miss for the search string. Try a shorter or differently-spelled title —
remember this provider only ever searches `type=movie`, so a TV show or episode title will never
match.

### Provider errors immediately with `NotConfigured`

No `api_key` was supplied when the provider was constructed. There is no environment variable or
`settings.json5` field to set today — see [Authentication](#authentication).

### Results are missing director, genre, rating, or vote data

**This is expected.** `OmdbProvider`'s parser does not read any of those fields from the `Search`
response — see [Available Data](#available-data). Nothing you configure will make them appear.

---

## Legal Notes

- **This provider is OMDb, not IMDb.** OMDb (`omdbapi.com`) is an independent third-party service;
  it republishes a subset of IMDb data under its own terms and is not affiliated with, endorsed
  by, or a substitute for a licence from IMDb.com, Inc. or Amazon.com, Inc.
- OMDb's terms are at [omdbapi.com](https://www.omdbapi.com/) — review them before using this
  provider for anything beyond personal, non-commercial use.
- IMDb itself is a trademark of IMDb.com, Inc., a subsidiary of Amazon.com, Inc. See
  [imdb.com/conditions](https://www.imdb.com/conditions) for IMDb's own conditions of use, which
  govern any IMDb-sourced data independent of OMDb's terms.
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media
  library. No data is redistributed.

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
