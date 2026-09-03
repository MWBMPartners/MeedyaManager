# 📡 TheTVDB Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains what the **TheTVDB** metadata provider actually does in MeedyaManager — including a known bug (issue #210) that means it currently does **not** work against the real TheTVDB v4 API.

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

> ⚠️ **Known bug — will 401 against real TheTVDB (issue #210).** `TheTvdbProvider`
> (`crates/mm-providers/src/video/mod.rs`) sends the raw configured API key straight through as a
> bearer token: `.bearer_auth(api_key)`. TheTVDB v4 does **not** accept an API key as a bearer
> token — it requires exchanging the key for a JWT via `POST /login` first. The source's own doc
> comment says "Bearer token obtained via `/login`", but **no `/login` request exists anywhere in
> the code.** Until #210 is fixed, every real request this provider makes will fail with HTTP 401.
> The parsing/response-mapping logic below is otherwise implemented and unit-tested against fixture
> JSON, but there is no working end-to-end path today.
>
> **Not reachable from the app regardless.** `meedya lookup` (the CLI command) is a permanent stub
> that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`).

---

## Overview

`TheTvdbProvider` searches `GET {base}/v4/search?query=<title>&limit=<n>` with the configured API
key sent via `Authorization: Bearer <api_key>`. This is the correct endpoint shape for TVDB v4, but
the authentication step is wrong (see the banner above), so it does not work against the live
service today.

This provider is useful, once #210 is fixed, for:

- Looking up TV series (and film) metadata by title
- Retrieving an image URL and short overview
- Obtaining a TVDB entity ID for cross-referencing

It does **not** implement season/episode listings, aired-vs-DVD episode ordering, or any concept of
a "series slug" — none of those appear in the parser (see [Available Data](#available-data)).

---

## Authentication

**An API key is required**, but as explained above, the way MeedyaManager currently sends it will
not work against TheTVDB's real v4 API.

### Getting a TVDB API key

1. Create an account at [thetvdb.com](https://thetvdb.com) and verify your email.
2. Go to [thetvdb.com/dashboard/account/apikeys](https://thetvdb.com/dashboard/account/apikeys)
   and generate a key.

### What MeedyaManager does with it (and why it's broken)

`TheTvdbProvider::new(api_key: Option<String>)` stores the key and later sends it as
`.bearer_auth(key)` on every request. TheTVDB v4's actual flow requires `POST /login` with the API
key in the request body, which returns a short-lived JWT to use as the bearer token instead — that
exchange is not implemented. See issue #210.

There is also **no environment variable or `settings.json5` field read for TheTVDB** —
`mm_core::config::ProviderConfig` has no TVDB field, so even once #210 is fixed, wiring a key in
from configuration would need new code. The generic 4-tier `CredentialStore` in
`crates/mm-providers/src/credentials.rs` would resolve `MM_THETVDB_*` / `MM_TVDB_*` if something
called it — nothing does. Its tier 4 is a **plain JSON file**, not encrypted (issue #209).

---

## Configuration

### Environment Variables (`.env`)

None are read today.

### Settings File (`settings.json5`)

There is no per-provider `settings.json5` schema for TVDB — `language`, `episode_order`,
`result_limit`, and `fetch_episodes` are not real settings; nothing reads them, and there is no
episode-listing feature to configure in the first place. The only real input is the constructor's
`api_key` and an optional `base_url` override used by the crate's own tests.

---

## Available Data

| Field | Response field | Example |
| ----- | --------------- | ------- |
| `title` | `data[].name` | "Game of Thrones" |
| `year` | first 4 digits of `data[].first_air_time` | "2013" |
| cover art | `data[].image_url` (skipped if empty) | one JPEG, no known dimensions |
| provider ID (metadata) | `data[].id` | "121361" |
| media type (metadata) | `data[].type` | "series" |
| overview (metadata) | `data[].overview` | plot summary text |

There is **no** `show`, `season`, `episode`, `episode_title`, `genre`, `status`, or `slug` field —
the response type this parser deserializes into (`TvdbResult`) only has `id`, `name`,
`first_air_time`, `image_url`, `overview`, and `type`. TheTVDB's search endpoint returns
series/movie-level hits, not individual episodes, so per-episode matching is not something this
provider does at all.

---

## Custom Tags

Any `custom_tvdb_*` tag would come from the metadata above once tag-writing wiring exists for this
provider. There is no `custom_tvdb_slug` or `custom_tvdb_status` — neither a slug nor a status
field is ever parsed.

---

## Cover Art

`image_url`, when non-empty, is used as a single cover-art URL with no known width/height. There is
no resolution negotiation and no fallback to a different artwork size.

---

## Troubleshooting

### HTTP 401 on every request

**This is the known bug described above (issue #210).** TheTVDB v4 rejects a raw API key sent as a
bearer token — it needs the `/login` JWT exchange first, which this provider does not implement.
There is no workaround short of patching the provider.

### Expecting season/episode metadata, aired-vs-DVD ordering, or a series slug

**This is expected to be absent.** None of that is parsed by `TheTvdbProvider` — see
[Available Data](#available-data).

---

## Legal Notes

- TheTVDB provides API access under its [API Terms](https://thetvdb.com/api-terms).
- TV show metadata is contributed by the TheTVDB community.
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media
  library. No content is redistributed.
- "TheTVDB" is a trademark of TheTVDB.com LLC.

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
