# 🎼 ISWC Lookup Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains how the **ISWC Lookup** metadata provider actually works in MeedyaManager, including two configuration options documented in older guides (`resolve_relations`, `lookup_from_recording`) that **do not exist** in the code.

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Configuration](#configuration)
4. [Available Data](#available-data)
5. [Custom Tags](#custom-tags)
6. [ISWC Format Reference](#iswc-format-reference)
7. [Troubleshooting](#troubleshooting)
8. [Legal Notes](#legal-notes)

---

> ⚠️ **Not reachable from the app today.** `meedya lookup` (the CLI command) is a permanent stub
> that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`), not `IswcProvider`
> specifically. `IswcProvider` is real and has genuine wiremock-backed tests (`crates/mm-providers/
> src/identifiers/mod.rs`), but nothing in the shipped CLI or GUI constructs it.

---

## Overview

The ISWC Lookup provider resolves **International Standard Musical Work Codes** — globally unique identifiers assigned to musical works (compositions). While an ISRC identifies a specific *recording* of a song, an ISWC identifies the underlying *musical work* (the composition itself, regardless of who performs or records it).

This provider is useful for:

- **Resolving** ISWCs to work-level metadata (work title, composer)
- **Identifying** different recordings of the same composition (cover versions, remixes, live performances)

The ISWC provider retrieves data via **MusicBrainz's works API** — MusicBrainz maintains a database of musical works with associated ISWCs and artist relationships.

### ISRC vs. ISWC — What's the Difference?

| Identifier | Identifies | Example |
| ---------- | ---------- | ------- |
| **ISRC** | A specific *recording* (performance captured in audio) | Queen's 1975 studio recording of "Bohemian Rhapsody" |
| **ISWC** | A *musical work* (the composition/song itself) | The composition "Bohemian Rhapsody" by Freddie Mercury |

One ISWC can have many ISRCs (every cover version, live recording, and remix of the same song shares the same ISWC).

---

> ⚠️ **Upcoming MusicBrainz API changes (2026-11-30)** — MusicBrainz has announced breaking changes to its search API, effective **30 November 2026**. The replacement specification has not been published yet, so this project cannot describe the deltas in advance. Every piece of MusicBrainz-specific knowledge this provider depends on — endpoint URLs, query parameters, and response parsing — is centralised in one file, [`crates/mm-providers/src/musicbrainz.rs`](../../crates/mm-providers/src/musicbrainz.rs), so that when the new spec lands, the update can be applied in a single place instead of a hunt-and-peck across the codebase. This guide will be updated once the new behaviour ships.

---

## Authentication

**No authentication is required.** The ISWC provider uses MusicBrainz's public web service API, which is freely accessible without an API key or account.

MusicBrainz does require that API consumers identify themselves with a meaningful `User-Agent` header. MeedyaManager automatically sets this to:

```text
User-Agent: MeedyaManager/1.3.0 (Linux; x86_64) ( support@mwbmpartners.ltd https://www.mwbmpartners.ltd )
```

This follows MusicBrainz's documented `"AppName/Version ( contact-info )"` convention — see [musicbrainz.md](musicbrainz.md#authentication) for the full format and for how to override the contact address with the `MUSICBRAINZ_CONTACT_EMAIL` environment variable.

> **Note:** MusicBrainz enforces a rate limit of 60 requests/minute (1 request/second, burst 1) for unauthenticated access, shared across the MusicBrainz, ISRC, and ISWC providers via one token bucket (`crate::musicbrainz::mb_get()`'s shared per-host limiter).

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required for this provider.

### Settings File (`settings.json5`)

There is no per-provider `settings.json5` schema for ISWC. In particular, **`resolve_relations`
and `lookup_from_recording` are not real settings** — nothing in `IswcProvider` reads them, and
the enrichment behaviour they'd supposedly toggle happens unconditionally (see
[How the Lookup Works](#how-the-lookup-works) below); there is also no "chain from an ISRC/MBID to
discover an ISWC" feature to enable. The only real input is the constructor's User-Agent string
and an optional `base_url` override used by the crate's own tests.

### How the Lookup Works

The ISWC provider currently requires the file to already carry an **ISWC tag** — it does not (yet) chain through an ISRC or MusicBrainz recording ID to discover one. Given a valid ISWC, the lookup proceeds in two stages:

1. **Search.** The provider queries MusicBrainz's works endpoint by Lucene `iswc:<code>`:

   ```text
   GET https://musicbrainz.org/ws/2/work/?query=iswc:T0345246801&limit=<n>&fmt=json
   ```

   This returns the matching work(s) — title and ISWC — but a plain search response never includes composer credits.

   > **Punctuated input queries both forms.** MusicBrainz canonically stores and displays ISWCs punctuated (`T-034.524.680-1`), unlike ISRCs. Whether MusicBrainz's search index normalises punctuation out of `iswc:` queries server-side is an analyzer detail this project has no way to verify without live network access — so when your ISWC tag is punctuated, MeedyaManager queries BOTH forms, `OR`-ed together: `iswc:T0345246801 OR iswc:"T-034.524.680-1"` (the punctuated form phrase-quoted, so its hyphens/dots can't be read as Lucene syntax). An already-bare, unpunctuated ISWC tag is queried as a single term with no redundant `OR`. This costs nothing extra on the wire — still one HTTP request either way.

2. **Enrichment (first result only, when it looks like a real MBID, always attempted — not optional).** Because composer data isn't in the search response, the provider issues ONE additional lookup-by-id for the *first* result, requesting the artist-relations sub-resource:

   ```text
   GET https://musicbrainz.org/ws/2/work/<mbid>?fmt=json&inc=artist-rels
   ```

   Only the first result is enriched — enriching every row in a multi-result search would multiply outbound requests by up to the result limit, which the shared 1 request/second budget can't absorb. MeedyaManager also skips this lookup entirely when the first result's `id` doesn't structurally look like a MusicBrainz Identifier (a UUID-shaped value) — most notably when a search result carried no `id` at all, which would otherwise turn the lookup URL into the works *collection* endpoint rather than a single resource: a guaranteed error that would still spend a rate-limit token for nothing. If the lookup is attempted and fails for any reason (network error, rate limit, an unparseable response), it degrades gracefully: the composer is simply left unset and the un-enriched search result is returned rather than failing the whole lookup.

If the file has no ISWC tag at all, the provider returns no results — it cannot resolve one from anything else.

---

## Available Data

The ISWC provider returns the following standard metadata fields:

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Work title (composition name) | `"Bohemian Rhapsody"` |
| `artist` | Composer name(s) from the enrichment lookup, comma-separated if multiple | `"Freddie Mercury"` |

> **Note:** There is no separate `composer` field on a provider result — the upstream `ProviderResult` type has no such field, so the composer(s) found via `inc=artist-rels` are stored in the generic `artist` field. The `title` returned is the *work* title, which may differ slightly from the recording title (e.g., a work may be titled "Bohemian Rhapsody" while a specific recording is titled "Bohemian Rhapsody - Remastered 2011").

---

## Custom Tags

The ISWC value and work MBID are carried in the result's generic metadata slots (there is no
dedicated `custom_iswc_work_title` constant in the codebase — that would need new tag-writing
wiring, per the reachability note above):

| Value | Description | Example |
| ----- | ----------- | ------- |
| ISWC | The ISWC found in the work search response | `"T-034.524.680-1"` |
| Work MBID | The MusicBrainz work identifier (used internally for the enrichment lookup) | `"10c1a2b3-..."` |

### Use Cases for ISWC Data

- **Identifying cover versions:** Two files with the same ISWC but different `artist` tags are different recordings of the same composition.
- **Songwriter credits:** The composer name(s), populated into the generic `artist` field from the enrichment lookup, is often missing from recording-level metadata.
- **Music publishing:** ISWC is the standard identifier used in music publishing and royalty tracking.

---

## ISWC Format Reference

An ISWC follows this structure:

```text
T-NNN.NNN.NNN-C
```

| Component | Description | Example |
| --------- | ----------- | ------- |
| `T` | Prefix (always the letter T for musical works) | `T` |
| `-` | Separator | `-` |
| `NNN.NNN.NNN` | 9-digit work identifier, formatted in groups of 3 with dots | `034.524.680` |
| `-` | Separator | `-` |
| `C` | Single check digit (0-9) | `1` |

**Full example:** `T-034.524.680-1`

### Validation Rules

`validate_iswc()` checks, after uppercasing and stripping non-alphanumeric characters:

- Exactly 11 characters remain
- Starts with `T`
- The remaining 10 characters are all ASCII digits (9-digit work ID + 1 check digit)

There is no separate ISO 15707 modulo-10 check-digit *verification* step beyond this structural
check.

> **Note:** ISWCs are sometimes stored without formatting (e.g., `T0345246801`). Whichever form your tag uses, both the punctuated and bare forms are queried (see [How the Lookup Works](#how-the-lookup-works)).

---

## Troubleshooting

### Provider shows "Available" but returns no ISWC

- **Ensure the file has an ISWC tag.** The provider currently resolves work-level metadata FROM an existing ISWC — it does not discover an ISWC starting from an ISRC tag or MusicBrainz recording ID alone (see [How the Lookup Works](#how-the-lookup-works)).
- **Not all recordings have associated works in MusicBrainz.** MusicBrainz's work coverage is extensive but not complete. Older, obscure, or independently released tracks may not have work entries.

### "Invalid ISWC format" warning

The ISWC in your file's tags does not match the expected format — see [Validation Rules](#validation-rules).

### Composer (`artist`) field is empty even though the work resolved

- The enrichment lookup runs automatically on the first result whenever its ID looks like a valid
  MBID — there is no `resolve_relations` toggle to check.
- The work exists in MusicBrainz but may not have composer relationships attached — this is a gap
  in MusicBrainz's own data, not something MeedyaManager can configure around.
- The enrichment lookup itself may have failed (network error, rate limit) — this degrades
  gracefully to an un-enriched result rather than an error, so you'll see the work title but no
  composer.

### Rate limit warnings from MusicBrainz

MusicBrainz enforces 60 requests/minute (1 request/second, burst 1) for unauthenticated access, shared across the MusicBrainz, ISRC, and ISWC providers via one token bucket:

1. A `429`/`503` is retried automatically once, honouring the server's `Retry-After` header.
2. ISWC lookups may cost two requests per file: the work search, plus the composer-enrichment lookup for the first result.
3. This rate cannot be increased without a MusicBrainz authentication token (not implemented).

---

## Legal Notes

- **MusicBrainz** data is available under the [CC0 (public domain)](https://creativecommons.org/publicdomain/zero/1.0/) licence for core data and [CC BY-NC-SA 3.0](https://creativecommons.org/licenses/by-nc-sa/3.0/) for supplementary data. See [musicbrainz.org/doc/MusicBrainz_License](https://musicbrainz.org/doc/MusicBrainz_License).
- The ISWC system is administered by **CISAC** (Confederation of Societies of Authors and Composers) under the ISO 15707 standard. ISWCs themselves are factual identifiers and are not subject to copyright.
- MeedyaManager retrieves ISWC and related metadata solely for the purpose of organising the user's own media library. No data is redistributed.
- For more information about ISWC: [iswc.org](https://www.iswc.org)

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
