# 🎵 MusicBrainz Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers setting up the **MusicBrainz** metadata provider in MeedyaManager. MusicBrainz is a free, community-maintained music database that requires no API key — making it the easiest provider to get started with.

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

## Overview

The MusicBrainz provider uses the **MusicBrainz Web Service API (v2)** to search the world's largest open music database. MusicBrainz is community-maintained and freely accessible, providing authoritative identifiers (MBIDs) that are widely used across the music industry for cataloguing and cross-referencing recordings, releases, and artists.

**Key features:**

- Recording (track) and release (album) search via Lucene query syntax
- Direct ISRC code lookup for precise track identification
- MusicBrainz IDs (MBIDs): unique UUIDs for recordings, releases, and artists
- Cover art via the **Cover Art Archive** — a companion service providing album artwork
- No API key required — only a User-Agent header
- 60 requests/minute (1 request/second, burst 1), enforced automatically by a shared limiter covering the MusicBrainz, ISRC and ISWC providers

---

> ⚠️ **Upcoming MusicBrainz API changes (2026-11-30)** — MusicBrainz has announced breaking changes to its search API, effective **30 November 2026**. The replacement specification has not been published yet, so this project cannot describe the deltas in advance. Every piece of MusicBrainz-specific knowledge — endpoint URLs, query parameters, Lucene query syntax, and response parsing — is centralised in one file, [`crates/mm-providers/src/musicbrainz.rs`](../../crates/mm-providers/src/musicbrainz.rs), so that when the new spec lands, the update can be applied in a single place instead of a hunt-and-peck across the codebase. This guide will be updated once the new behaviour ships.

---

> ℹ️ **The one provider that's actually reachable today.** `meedya lookup` (the CLI command) is
> still a permanent stub that prints "Provider support is coming in M5" and never calls any
> provider (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. But the
> GTK lookup panel (`crates/mm-gtk/src/ui/lookup_panel.rs`) does construct a real, working
> `MusicBrainzProvider` for its background search — MusicBrainz is the **only** one of the 19
> providers documented in `help/providers/` that a user can actually reach from the shipped
> application today.

---

## Authentication

MusicBrainz does **not** require an API key or account. The only requirement is a properly formatted **User-Agent header** identifying your application and providing contact information. MeedyaManager includes this header automatically.

### What MeedyaManager sends

```text
User-Agent: MeedyaManager/1.3.0 (Linux; x86_64) ( support@mwbmpartners.ltd https://www.mwbmpartners.ltd )
```

This follows MusicBrainz's documented `"AppName/Version ( contact-info )"` convention: the standard MeedyaManager User-Agent (name, version, platform), followed by a parenthesised contact segment so the MusicBrainz operators can reach us if our traffic ever misbehaves.

### Customising the contact address

The contact segment defaults to MWBM Partners Ltd's support address, but self-hosters who would rather MusicBrainz contact *them* directly can override it at runtime with the `MUSICBRAINZ_CONTACT_EMAIL` environment variable — no rebuild required:

```env
# .env
MUSICBRAINZ_CONTACT_EMAIL=you@example.com
```

With that set, MeedyaManager sends `User-Agent: MeedyaManager/1.3.0 (Linux; x86_64) ( you@example.com )` instead. Leaving it unset (or empty) falls back to the compiled-in default shown above.

### No setup required

Unlike other providers, MusicBrainz is always available out of the box. There are no credentials to configure, no accounts to create, and no API keys to obtain.

> **Note:** If you are making custom modifications and sending requests directly, you **must** include a valid User-Agent header. Requests without a proper User-Agent will be rejected with HTTP 403.

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required for MusicBrainz.

### Settings (`settings.json5`)

```json5
{
  providers: {
    musicbrainz: {
      enabled: true,                    // Enable or disable this provider
      priority: 3,                      // Provider priority (lower = higher priority)
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `true` | Whether this provider is active |
| `priority` | `3` | Search priority relative to other providers |

---

## Available Data

The MusicBrainz provider returns the following standard metadata fields:

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `recording.title` | "Bohemian Rhapsody" |
| `artist` | `recording.artist-credit` | "Queen" |
| `album` | `recording.releases[0].title` | "A Night at the Opera" |
| `year` | `recording.releases[0].date` | "1975" |
| `isrc` | `recording.isrcs[0]` | "GBUM71029604" |

### ISRC Lookup

When an ISRC code is already present in the file's metadata, MusicBrainz can perform a **direct ISRC lookup** instead of a text search. This provides the highest accuracy match possible:

```text
GET https://musicbrainz.org/ws/2/isrc/GBUM71029604?fmt=json&inc=artist-credits+releases
```

MeedyaManager automatically uses ISRC lookup when an ISRC tag is available. See [isrc.md](isrc.md) for the dedicated ISRC provider's full lookup-plus-fallback behaviour.

### Search Query Building

When title and/or artist are known, MeedyaManager builds a Lucene query with each value **phrase-quoted** (e.g. `recording:"Bohemian Rhapsody" AND artistname:"Queen"`), so punctuation and Lucene operators that legitimately appear in a title or artist name (`AC/DC`, `Wait & See`, a track titled literally `Rock (Live)`) are treated as literal text rather than query syntax.

A free-text fallback path exists in `crate::musicbrainz::recording_query()` for the case where neither title nor artist is known — it would character-escape the free-text term instead of phrase-quoting it. In practice this path is **not reachable with meaningful content today**: the upstream `SearchQuery` struct (from `meedya-core`) has no generic free-text `query` field, only `title`/`artist`/`album`/`year`/identifiers. `MusicBrainzProvider` derives its "free text" candidate by concatenating `title` and `artist` (`search_term()`), which is only ever non-empty when at least one of them is already set — and whenever either is set, `recording_query()` takes the phrase-quoted branch instead, never the free-text one. So the free-text branch is only ever invoked with an empty string. Wiring up genuine free-text search would need an upstream change to `SearchQuery` (tracked on issue #198).

### Zero-Result Loosened Retry

A phrase-quoted query is an **exact, ordered-token match** — it requires the title/artist to appear verbatim, in that order, with nothing in between. Real-world file tags often carry decorations a MusicBrainz title doesn't have (`Comfortably Numb (Remastered 2011)`, `Track Name (Live)`, `Song feat. Someone Else`), so a phrase query built from such a tag can legitimately come back with zero results even though a close match exists in the database.

When that happens — and ONLY when the original query actually used phrase-quoting, i.e. a title and/or artist were supplied — MeedyaManager retries **exactly once** with a loosened query built from the same inputs: the same `recording:`/`artistname:` fields, but with each value character-escaped (`lucene_escape()`) instead of phrase-quoted. Escaping still keeps Lucene operators neutered (the whole point of the original phrase-quoting fix), but no longer requires the tokens to appear as one exact contiguous phrase, giving MusicBrainz's own relevance ranking a chance to find a near-match the strict phrase query couldn't.

This retry is skipped for:

- a **free-text-only** query (no title/artist) — it already went through character-escaping on the first attempt, so there is nothing left to loosen;
- an **ISRC** query — an exact-identifier match has nothing to "loosen".

The retry costs one extra request against the shared rate-limit budget, but only in the miss case — a query that finds something on the first try never triggers it.

### Pagination — not currently reachable

`crate::musicbrainz::search_params()` accepts an `offset` argument and omits it from the wire only
when it is zero, so the low-level plumbing for pagination exists and is unit-tested. However,
**no caller can actually request a non-zero offset today**: `MusicBrainzProvider::search()`,
`IsrcProvider`, and `IswcProvider` all hard-code `offset` to `0` when calling `search_params()`,
because the upstream `SearchQuery` struct — the only way a caller communicates a query to this
provider — has no `offset` field at all. Paginated MusicBrainz search is blocked on an upstream
change to `SearchQuery` and is tracked on issue #198.

---

## Custom Tags

The following custom tags are stored in the file's metadata when matched:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_musicbrainz_recording_id` | Recording MBID (UUID) | `"b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d"` |
| `custom_musicbrainz_release_id` | Release MBID (UUID) | `"a1b2c3d4-e5f6-7890-abcd-ef1234567890"` |
| `custom_musicbrainz_artist_id` | Artist MBID (UUID) | `"0383dadf-2a4e-4d10-a46a-e9e041da8eb3"` |
| `custom_musicbrainz_url` | MusicBrainz recording URL | `"https://musicbrainz.org/recording/b10bb..."` |
| `custom_musicbrainz_isrc` | ISRC code from MusicBrainz | `"GBUM71029604"` |

MusicBrainz IDs (MBIDs) are persistent, globally unique identifiers. They are widely supported by other tools like MusicBrainz Picard, Beets, and MP3tag, making them ideal for cross-referencing:

```json5
rename_format: "{artist}/{album}/{track_num} - {title} [MB-{custom_musicbrainz_recording_id}].{extension}"
```

---

## Cover Art

MusicBrainz itself does not host cover art. Instead, it integrates with the **Cover Art Archive** (CAA), a free companion service:

| Type | Format | Resolution | Source |
| ---- | ------ | ---------- | ------ |
| **Static (front cover)** | JPEG | 500x500 | Cover Art Archive via release MBID |

The URL pattern is:

```text
https://coverartarchive.org/release/{release_mbid}/front-500
```

MeedyaManager automatically constructs cover art URLs when a release MBID is available.

> **Note:** Not all MusicBrainz releases have cover art in the Cover Art Archive. Coverage depends on community contributions. MeedyaManager handles missing art gracefully and falls back to other providers.

---

## Troubleshooting

### "MusicBrainz search returned 0 results"

**Possible causes:**

- The recording may not be in the MusicBrainz database (community-contributed)
- Search terms may be too specific or contain special characters
- MusicBrainz uses Lucene query syntax — MeedyaManager phrase-quotes title and artist automatically (see [Search Query Building](#search-query-building) above) and character-escapes free-text fallback queries, but unusual metadata may still cause issues
- A phrase-quoted query that finds nothing is automatically retried once with a loosened query (see [Zero-Result Loosened Retry](#zero-result-loosened-retry) above) — if you still see zero results after that, the recording genuinely isn't a close match for anything in the database under either query shape

**Solutions:**

1. Try searching on [musicbrainz.org](https://musicbrainz.org/) directly to verify the recording exists
2. Consider tagging your files with MusicBrainz Picard first, which adds MBIDs and ISRCs
3. ISRC lookup is more accurate than text search — ensure ISRC tags are present where possible

### HTTP 429 / HTTP 503 — Rate limit exceeded

**Cause:** MusicBrainz strictly enforces **60 requests/minute (1 request/second, burst 1)** across all traffic, not per feature. Exceeding this results in a `429 Too Many Requests` or `503 Service Unavailable` response, and repeat offenders risk temporary IP blocks.

**Solution:**

- MeedyaManager's built-in rate limiter should prevent this — it is a single shared budget covering the MusicBrainz, ISRC, and ISWC providers together, so a search and an ISRC/ISWC lookup running back-to-back still serialise to 1 request/second rather than each getting their own allowance
- A `429`/`503` that does get through is retried automatically ONCE, honouring the server's `Retry-After` header (capped at 10 seconds) — a second consecutive throttle, or a `Retry-After` that is absent or too large, is surfaced as a rate-limit error rather than retried further
- If you see this error, it may be caused by another application also accessing MusicBrainz from the same IP address
- Wait a few minutes for the block to expire (typically 1-5 minutes)

### HTTP 403 — Forbidden

**Cause:** Missing or invalid User-Agent header.

**Solution:**

- This should not occur during normal MeedyaManager operation (the contact-bearing header is applied automatically — see [Authentication](#authentication) above)
- If you see this after modifying the source code, ensure the User-Agent follows the format: `AppName/Version ( contact-info )`

### Cover art not found (HTTP 404 from Cover Art Archive)

**Cause:** The release does not have cover art uploaded to the Cover Art Archive.

**Solution:** This is expected for some releases. You can contribute cover art at [coverartarchive.org](https://coverartarchive.org/) or rely on other providers (Apple Music, Spotify) for artwork.

---

## Legal Notes

- MusicBrainz is a project of the [MetaBrainz Foundation](https://metabrainz.org/), a non-profit organisation
- The MusicBrainz database is licensed under [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/) (public domain)
- The Cover Art Archive is a joint project of MusicBrainz and the Internet Archive
- Rate limits (60 requests/minute — 1 request/second, burst 1) must be respected — excessive requests may result in IP bans
- There are no API key requirements, fees, or registration processes
- MeedyaManager stores MBIDs as custom metadata tags; these are open identifiers and freely usable

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
