# 🎼 ISWC Lookup Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains how to configure and use the **ISWC Lookup** metadata provider in MeedyaManager.

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

## Overview

The ISWC Lookup provider resolves **International Standard Musical Work Codes** — globally unique identifiers assigned to musical works (compositions). While an ISRC identifies a specific *recording* of a song, an ISWC identifies the underlying *musical work* (the composition itself, regardless of who performs or records it).

This provider is useful for:

- **Resolving** ISWCs to work-level metadata (work title, composers, lyricists)
- **Linking** recordings to their underlying musical compositions
- **Identifying** different recordings of the same composition (cover versions, remixes, live performances)
- **Enriching** metadata with songwriter and publisher information

The ISWC provider retrieves data via **MusicBrainz work relations** — MusicBrainz maintains a database of musical works with associated ISWCs, linked to recordings and releases through its relationship system.

### ISRC vs. ISWC — What's the Difference?

| Identifier | Identifies | Example |
| ---------- | ---------- | ------- |
| **ISRC** | A specific *recording* (performance captured in audio) | Queen's 1975 studio recording of "Bohemian Rhapsody" |
| **ISWC** | A *musical work* (the composition/song itself) | The composition "Bohemian Rhapsody" by Freddie Mercury |

One ISWC can have many ISRCs (every cover version, live recording, and remix of the same song shares the same ISWC).

---

## Authentication

**No authentication is required.** The ISWC provider uses MusicBrainz's public web service API, which is freely accessible without an API key or account.

MusicBrainz does require that API consumers identify themselves with a meaningful `User-Agent` header. MeedyaManager automatically sets this to:

```text
MeedyaManager/1.0 (https://github.com/MWBMPartners/MeedyaManager)
```

> **Note:** MusicBrainz enforces a rate limit of approximately 1 request per second for unauthenticated access. MeedyaManager's built-in rate limiter handles this automatically.

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required for this provider.

### Settings File (`settings.json5`)

You can optionally configure the provider's behaviour in `config/settings.json5`:

```json5
{
  providers: {
    iswc: {
      // Enable or disable this provider (default: true)
      enabled: true,

      // Whether to resolve work relations (composers, lyricists) (default: true)
      // When true, fetches the full work record including creator relationships
      // When false, only stores the ISWC and work title
      resolve_relations: true,

      // Whether to look up ISWC from recording relations (default: true)
      // If a file has an ISRC or MusicBrainz recording ID but no ISWC,
      // this option enables looking up the associated work and its ISWC
      lookup_from_recording: true,

      // MusicBrainz rate limit: requests per second (default: 1.0)
      // MusicBrainz enforces a 1 request/second limit for unauthenticated access
      musicbrainz_rate_limit: 1.0,
    }
  }
}
```

### How the Lookup Works

The ISWC provider follows this resolution chain:

1. **If the file already has an ISWC tag:** Validates the format and looks up the work on MusicBrainz to fetch additional metadata (title, composers).
2. **If the file has an ISRC but no ISWC:** Looks up the recording by ISRC on MusicBrainz, follows the "recording-of" work relation, and retrieves the associated ISWC.
3. **If the file has a MusicBrainz recording ID:** Follows the work relation directly to find the associated ISWC.
4. **If none of the above:** The provider cannot resolve an ISWC for this file and returns no results.

---

## Available Data

The ISWC provider returns the following standard metadata fields:

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Work title (composition name) | `"Bohemian Rhapsody"` |
| `composer` | Composer name(s) (comma-separated if multiple) | `"Freddie Mercury"` |

> **Note:** The `title` returned is the work title, which may differ slightly from the recording title (e.g., a work may be titled "Bohemian Rhapsody" while a specific recording is titled "Bohemian Rhapsody - Remastered 2011").

---

## Custom Tags

The provider writes the following custom tags to media files:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_iswc` | The ISWC for the musical work | `"T-034.524.680-1"` |
| `custom_iswc_work_title` | The canonical work title from MusicBrainz | `"Bohemian Rhapsody"` |

The `custom_iswc` stores the full ISWC in its standard formatted form (with hyphens and dots). This identifier is permanent and globally unique to the musical composition.

The `custom_iswc_work_title` stores the canonical title of the work as registered in MusicBrainz, which serves as the "official" composition title independent of any specific recording or release.

### Use Cases for ISWC Data

- **Identifying cover versions:** Two files with the same `custom_iswc` but different `artist` tags are different recordings of the same composition.
- **Songwriter credits:** The `composer` field populated from work relations provides the songwriter(s), which is often missing from recording-level metadata.
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

MeedyaManager validates ISWCs against these rules:

- Starts with `T-`
- Followed by 9 digits in three groups of 3, separated by dots
- Ends with a hyphen and a single check digit (0-9)
- The regex pattern: `^T-\d{3}\.\d{3}\.\d{3}-\d$`
- The check digit is verified using the ISO 15707 modulo-10 algorithm

> **Note:** ISWCs are sometimes stored without formatting (e.g., `T0345246801`). MeedyaManager normalises all ISWCs to the standard formatted form with hyphens and dots.

---

## Troubleshooting

### Provider shows "Available" but returns no ISWC

- **Not all recordings have associated works in MusicBrainz.** MusicBrainz's work coverage is extensive but not complete. Older, obscure, or independently released tracks may not have work entries.
- **Check MusicBrainz directly.** Search for the recording at [musicbrainz.org](https://musicbrainz.org) and check if it has a "recording of" work relationship.
- **Ensure the file has identifying metadata.** The provider needs at least one of: an existing ISWC tag, an ISRC tag, or a MusicBrainz recording ID to perform a lookup.

### "Invalid ISWC format" warning

The ISWC in your file's tags does not match the expected format. Common causes:

- Missing the `T-` prefix
- Incorrect number of digits (must be exactly 9 digits plus 1 check digit)
- Extra whitespace or hidden characters
- Confusion with other identifier types (ISRC, ISAN, etc.)

### Composer field is empty even though ISWC resolved

- The work exists in MusicBrainz but may not have composer relationships attached
- MusicBrainz work relationships are community-maintained and may be incomplete
- Set `resolve_relations: true` in your settings to ensure relation lookups are enabled
- You can contribute missing relationships to MusicBrainz to improve data quality

### Rate limit warnings from MusicBrainz

MusicBrainz enforces 1 request per second for unauthenticated access:

1. MeedyaManager's rate limiter handles this automatically
2. ISWC lookups may require 2-3 requests per file (recording lookup + work lookup + relations)
3. Large batch processing will proceed at approximately 1 file every 2-3 seconds
4. This rate cannot be increased without MusicBrainz authentication (future feature)

### Multiple works found for one recording

Some recordings are linked to multiple works in MusicBrainz (e.g., a medley). In such cases:

- MeedyaManager selects the primary "recording of" work relation
- Additional works are logged but not stored (to avoid ambiguity)
- Check MusicBrainz directly if you need the complete work relation graph

---

## Legal Notes

- **MusicBrainz** data is available under the [CC0 (public domain)](https://creativecommons.org/publicdomain/zero/1.0/) licence for core data and [CC BY-NC-SA 3.0](https://creativecommons.org/licenses/by-nc-sa/3.0/) for supplementary data. See [musicbrainz.org/doc/MusicBrainz_License](https://musicbrainz.org/doc/MusicBrainz_License).
- The ISWC system is administered by **CISAC** (Confederation of Societies of Authors and Composers) under the ISO 15707 standard. ISWCs themselves are factual identifiers and are not subject to copyright.
- MeedyaManager retrieves ISWC and related metadata solely for the purpose of organising the user's own media library. No data is redistributed.
- For more information about ISWC: [iswc.org](https://www.iswc.org)

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
