# 🔢 ISRC Lookup Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains how the **ISRC Lookup** metadata provider actually works in MeedyaManager: a **MusicBrainz-only** identifier lookup, not a federated cross-reference across multiple registries.

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [How the Lookup Works](#how-the-lookup-works)
3. [Authentication](#authentication)
4. [Configuration](#configuration)
5. [Available Data](#available-data)
6. [Custom Tags](#custom-tags)
7. [ISRC Format Reference](#isrc-format-reference)
8. [Troubleshooting](#troubleshooting)
9. [Legal Notes](#legal-notes)

---

> ⚠️ **Not reachable from the app today.** `meedya lookup` (the CLI command) is a permanent stub
> that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`), not `IsrcProvider`
> specifically. `IsrcProvider` is real and has genuine wiremock-backed tests (`crates/mm-providers/
> src/identifiers/mod.rs`), but nothing in the shipped CLI or GUI constructs it.

---

## Overview

`IsrcProvider` (`crates/mm-providers/src/identifiers/mod.rs`) validates and resolves
**International Standard Recording Codes** (ISRCs) using **MusicBrainz alone**. There is no
Spotify or Deezer cross-referencing, no GTIN/barcode resolution, and no way to search by anything
other than an ISRC the caller already has — this provider cannot discover an ISRC from a title/artist
search; it only resolves metadata *from* an ISRC you already supply.

This provider is useful for:

- **Validating** the format of an ISRC already present in your media files' tags
- **Resolving** recording details (title, artist, release) from a known ISRC code via MusicBrainz

It is **not** a federated lookup: earlier documentation describing Spotify/Deezer
cross-referencing or barcode (GTIN/UPC) resolution described a feature that does not exist in this
codebase.

---

> ⚠️ **Upcoming MusicBrainz API changes (2026-11-30)** — MusicBrainz has announced breaking changes to its search API, effective **30 November 2026**. The replacement specification has not been published yet, so this project cannot describe the deltas in advance. Every piece of MusicBrainz-specific knowledge this provider depends on — endpoint URLs, query parameters, and response parsing — is centralised in one file, [`crates/mm-providers/src/musicbrainz.rs`](../../crates/mm-providers/src/musicbrainz.rs), so that when the new spec lands, the update can be applied in a single place instead of a hunt-and-peck across the codebase. This guide will be updated once the new behaviour ships.

---

## How the Lookup Works

MeedyaManager looks up an ISRC in two stages, both against MusicBrainz:

1. **Direct lookup (primary).** A single, cheap, exact-match request against the dedicated ISRC endpoint:

   ```text
   GET https://musicbrainz.org/ws/2/isrc/GBUM71029604?fmt=json&inc=artist-credits+releases
   ```

2. **Recording search (fallback).** MeedyaManager falls back to a general recording search queried by `isrc:<code>` — the same endpoint MusicBrainz's own search provider uses — whenever the direct lookup didn't produce a genuine, usable hit. That covers THREE cases, all treated identically:
   - a request-level failure other than a rate limit (a 404 "not registered", a network error, or the endpoint having moved);
   - a 200 response whose body doesn't parse as JSON at all; and
   - a 200 response that parses cleanly but yields **zero recordings**.

   That third case matters because every field of the lookup response is optional: if `/ws/2/isrc/` changes shape under MusicBrainz's announced 2026-11-30 breaking release, a restructured response can parse without error yet carry no recordings — a silent miss rather than a loud one. Treating "parsed OK but empty" the same as "failed to parse" is what makes the fallback actually cover that scenario instead of quietly returning zero results. This fallback costs one extra request for ISRCs the dedicated endpoint doesn't recognise (or whose response this build can no longer make sense of).

A **rate-limited** response from the direct lookup is returned immediately, without attempting the fallback — piling a second request onto a server that just asked us to back off would be exactly the wrong response.

If the caller doesn't supply an ISRC in the query at all, `search()` returns `NotSupported`
immediately — there is no path from a title/artist search to an ISRC in this provider.

> **Result limit.** The direct lookup endpoint takes no `limit` parameter of its own — MusicBrainz returns every recording registered against the ISRC. MeedyaManager truncates the parsed results to the requested result count itself, so a direct hit never returns more results than the fallback search would.

---

## Authentication

**No dedicated authentication is required.** MusicBrainz is freely accessible without an API key;
only a contact-bearing User-Agent header is needed (see [musicbrainz.md](musicbrainz.md#authentication)).

There is **no Spotify or Deezer integration in this provider at all** — `IsrcProvider`'s struct
holds only an HTTP client, a base URL, and a User-Agent string. Any earlier documentation
describing "federated sources" or Spotify credentials improving ISRC results was describing a
feature that isn't in the code.

---

## Configuration

### Environment Variables (`.env`)

None are required or read by `IsrcProvider` specifically.

### Settings File (`settings.json5`)

There is no per-provider `settings.json5` schema for ISRC — `validate_format`, `resolve_barcodes`,
`use_spotify`, `use_deezer`, and `musicbrainz_rate_limit` are not real settings; nothing reads
them, and the features they'd control (barcode resolution, Spotify/Deezer cross-referencing) do
not exist in the code. The only real inputs are the constructor's User-Agent string and an
optional `base_url` override used by the crate's own tests.

Rate limiting is real, but it isn't a per-provider dial: `IsrcProvider` shares **one** rate-limit
bucket with `MusicBrainzProvider` and `IswcProvider` (60 requests/minute — 1 request/second, burst
1) via `crate::musicbrainz::mb_get()`'s shared per-host limiter. See
[musicbrainz.md](musicbrainz.md#authentication) for details.

---

## Available Data

The ISRC provider returns the following standard metadata fields when resolving an ISRC:

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Recording title | `"Bohemian Rhapsody"` |
| `artist` | Artist name(s) | `"Queen"` |
| `album` | Release/album name (from first matching release) | `"A Night at the Opera"` |
| `year` | Release year | `"1975"` |
| `isrc` | The queried ISRC, backfilled onto every result | `"GBUM71029604"` |

There is **no `track_num` field populated** by this provider, and **no GTIN/barcode field** —
neither is read from the MusicBrainz response by this code path.

---

## Custom Tags

This provider does not write a `custom_isrc_source` or `custom_gtin` tag — neither exists anywhere
in the codebase. The `isrc` field itself is a **standard tag** (not a custom tag) because ISRC is
part of MeedyaManager's standard tag map and is natively supported by most audio file formats
(ID3v2 TSRC frame, Vorbis ISRC comment, MP4 `----:com.apple.iTunes:ISRC`).

---

## ISRC Format Reference

An ISRC is a 12-character alphanumeric code with the following structure:

```text
XX-XXX-YY-NNNNN
```

| Component | Length | Description | Example |
| --------- | ------ | ----------- | ------- |
| `XX` | 2 chars | **Country code** — ISO 3166-1 alpha-2 (registrant's country) | `GB` |
| `XXX` | 3 chars | **Registrant code** — assigned by the national ISRC agency | `UM7` |
| `YY` | 2 digits | **Year of reference** — year the code was assigned (not the release year) | `10` |
| `NNNNN` | 5 digits | **Designation code** — unique recording identifier within the registrant | `29604` |

**Full example:** `GBUM71029604` (or formatted: `GB-UM7-10-29604`)

### Validation Rules

`validate_isrc()` checks, after stripping non-alphanumeric characters:

- Exactly 12 characters
- First 2 characters: ASCII letters (country code)
- Next 3 characters: ASCII letters or digits (registrant code)
- Next 2 characters: ASCII digits (year)
- Last 5 characters: ASCII digits (designation)

> **Note:** The year component (`YY`) represents when the ISRC was assigned, not when the recording was released. A 1975 recording reissued in 2010 may have year code `10`.

---

## Troubleshooting

### Provider errors with `NotSupported: ISRC query requires an ISRC code`

You queried without an ISRC. This provider cannot search by title/artist — supply an `isrc` on the
query.

### Provider errors with "Invalid ISRC format"

The supplied ISRC does not match the expected 12-character pattern once punctuation is stripped —
see [Validation Rules](#validation-rules) above.

### Provider shows "Available" but returns no results for a valid ISRC

- **Check MusicBrainz directly.** Search at [musicbrainz.org](https://musicbrainz.org/search?type=isrc) — if the ISRC is not in MusicBrainz, no results will be returned.
- **Not all recordings have ISRCs in MusicBrainz.** The MusicBrainz database is community-maintained; older or obscure recordings may not have ISRC entries. Consider tagging your files with MusicBrainz Picard first.

### Rate limit warnings from MusicBrainz

MusicBrainz enforces 60 requests/minute (1 request/second, burst 1) for unauthenticated access,
shared across the MusicBrainz, ISRC, and ISWC providers via one token bucket. A `429`/`503` is
retried automatically once, honouring the server's `Retry-After` header (capped at 10 seconds).
An ISRC that falls back to the recording search costs a second request, so those lookups take
roughly twice as long.

### Expecting GTIN/barcode data or a Spotify/Deezer cross-reference

**This is expected to be absent.** Neither exists in this provider — see
[Available Data](#available-data) and [Authentication](#authentication).

---

## Legal Notes

- **MusicBrainz** data is available under the [CC0 (public domain)](https://creativecommons.org/publicdomain/zero/1.0/) licence for core data and [CC BY-NC-SA 3.0](https://creativecommons.org/licenses/by-nc-sa/3.0/) for supplementary data. See [musicbrainz.org/doc/MusicBrainz_License](https://musicbrainz.org/doc/MusicBrainz_License).
- The ISRC system is managed by the **International ISRC Registration Authority** (IFPI). ISRCs themselves are factual identifiers and are not subject to copyright.
- MeedyaManager retrieves ISRC and related metadata solely for the purpose of organising the user's own media library. No data is redistributed.

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
