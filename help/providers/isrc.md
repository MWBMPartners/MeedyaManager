# 🔢 ISRC Lookup Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)**

This guide explains how to configure and use the **ISRC Lookup** metadata provider in MeedyaManager.

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Configuration](#configuration)
4. [Available Data](#available-data)
5. [Custom Tags](#custom-tags)
6. [ISRC Format Reference](#isrc-format-reference)
7. [Troubleshooting](#troubleshooting)
8. [Legal Notes](#legal-notes)

---

## Overview

The ISRC Lookup provider validates and resolves **International Standard Recording Codes** (ISRCs) — the globally unique identifiers assigned to individual audio and music video recordings. Every commercially released song or music video has (or should have) an ISRC.

This provider is useful for:

- **Validating** ISRC codes already present in your media files' tags
- **Looking up** recording metadata by ISRC via MusicBrainz
- **Cross-referencing** ISRCs with GTIN/UPC barcodes for album identification
- **Resolving** recording details (title, artist, release) from an ISRC code
- **Enriching** metadata by federated lookup across multiple sources

The ISRC provider uses a **federated approach** — it queries MusicBrainz as its primary source and can optionally cross-reference results with Spotify and Deezer if those providers are configured and available.

---

## Authentication

**No dedicated authentication is required.** The ISRC provider's primary data source is MusicBrainz, which is freely accessible without an API key.

### Federated Sources

The ISRC provider queries the following sources (in priority order):

| Source | Auth Required | Purpose |
| ------ | ------------- | ------- |
| **MusicBrainz** | No (free, rate-limited) | Primary ISRC lookup, recording details, GTIN/barcode cross-reference |
| **Spotify** | Yes (if configured) | Optional: verify ISRC, fetch additional metadata |
| **Deezer** | No (free) | Optional: verify ISRC, fetch additional metadata |

If Spotify credentials are configured in your `.env` file, the ISRC provider will automatically use Spotify as an additional verification source. Deezer's public API is used when available. Neither is required — MusicBrainz alone provides complete ISRC lookup functionality.

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required specifically for the ISRC provider. However, if Spotify is configured for music lookups, the ISRC provider will leverage it automatically:

```env
# Optional: Spotify credentials improve ISRC cross-referencing
SPOTIFY_CLIENT_ID=your_spotify_client_id
SPOTIFY_CLIENT_SECRET=your_spotify_client_secret
```

### Settings File (`settings.json5`)

You can optionally configure the provider's behaviour in `config/settings.json5`:

```json5
{
  providers: {
    isrc: {
      // Enable or disable this provider (default: true)
      enabled: true,

      // Whether to validate ISRC format before lookup (default: true)
      // Rejects malformed ISRCs early, before making network requests
      validate_format: true,

      // Whether to cross-reference with GTIN/UPC barcodes (default: true)
      // If a matching release has a barcode, it is stored as custom_gtin
      resolve_barcodes: true,

      // Whether to use Spotify for cross-referencing (default: true)
      // Only effective if Spotify credentials are configured
      use_spotify: true,

      // Whether to use Deezer for cross-referencing (default: true)
      use_deezer: true,

      // MusicBrainz rate limit: requests per second (default: 1.0)
      // MusicBrainz enforces a 1 request/second limit for unauthenticated access
      musicbrainz_rate_limit: 1.0,
    }
  }
}
```

---

## Available Data

The ISRC provider returns the following standard metadata fields when resolving an ISRC:

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Recording title | `"Bohemian Rhapsody"` |
| `artist` | Artist name(s) | `"Queen"` |
| `album` | Release/album name (from first matching release) | `"A Night at the Opera"` |
| `year` | Release year | `"1975"` |
| `isrc` | The validated ISRC code itself | `"GBUM71029604"` |
| `track_num` | Track position on the release (if available) | `"11"` |

---

## Custom Tags

In addition to standard fields, the provider writes the following custom tags to media files:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_isrc_source` | Which source confirmed the ISRC | `"musicbrainz"` |
| `custom_gtin` | GTIN/UPC/EAN barcode for the release (if found) | `"0602537492374"` |

The `isrc` field itself is written as a **standard tag** (not a custom tag) because ISRC is part of MeedyaManager's standard tag map and is natively supported by most audio file formats (ID3v2 TSRC frame, Vorbis ISRC comment, MP4 ----:com.apple.iTunes:ISRC).

The `custom_gtin` stores the Global Trade Item Number (GTIN) — also known as UPC or EAN barcode — for the album/release that contains the recording. This is useful for:

- Identifying which specific release (pressing, remaster, deluxe edition) a track belongs to
- Cross-referencing with music databases that use barcodes (Discogs, MusicBrainz)
- Linking physical and digital release metadata

The `custom_isrc_source` indicates which data source confirmed the ISRC, useful for auditing and debugging metadata chain of trust.

---

## ISRC Format Reference

An ISRC is a 12-character alphanumeric code with the following structure:

```
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

MeedyaManager validates ISRCs against these rules:

- Exactly 12 characters when unformatted (hyphens are stripped)
- First 2 characters: uppercase letters (country code)
- Next 3 characters: uppercase letters or digits (registrant code)
- Next 2 characters: digits (year)
- Last 5 characters: digits (designation)
- The regex pattern: `^[A-Z]{2}[A-Z0-9]{3}[0-9]{7}$`

> **Note:** The year component (`YY`) represents when the ISRC was assigned, not when the recording was released. A 1975 recording reissued in 2010 may have year code `10`.

---

## Troubleshooting

### Provider shows "Available" but returns no results for an ISRC

- **Verify the ISRC is correct.** Check the format matches the pattern described above. Common errors include lowercase letters, extra spaces, or missing digits.
- **Check MusicBrainz directly.** Search at [musicbrainz.org](https://musicbrainz.org/search?type=isrc) — if the ISRC is not in MusicBrainz, no results will be returned.
- **Not all recordings have ISRCs in MusicBrainz.** The MusicBrainz database is community-maintained; older or obscure recordings may not have ISRC entries. Consider tagging your files with MusicBrainz Picard first.

### "Invalid ISRC format" warning

The ISRC in your file's tags does not match the expected 12-character format. Common causes:

- Extra whitespace or hidden characters in the tag value
- Lowercase letters (ISRCs should be uppercase)
- Hyphens included in the stored value (MeedyaManager strips these, but verify the underlying data)
- The tag contains a different identifier type (e.g., a UPC barcode in the ISRC field)

### Rate limit warnings from MusicBrainz

MusicBrainz enforces a rate limit of 1 request per second for unauthenticated access. If processing many files:

1. MeedyaManager's rate limiter handles this automatically
2. Processing will proceed at approximately 1 file per second for ISRC lookups
3. This rate cannot be increased without a MusicBrainz authentication token (future feature)

### GTIN/barcode not found

Not all releases in MusicBrainz have barcodes attached. The `custom_gtin` field will only be populated when:

- The ISRC resolves to a recording in MusicBrainz
- That recording is linked to at least one release
- That release has a barcode/GTIN associated with it

---

## Legal Notes

- **MusicBrainz** data is available under the [CC0 (public domain)](https://creativecommons.org/publicdomain/zero/1.0/) licence for core data and [CC BY-NC-SA 3.0](https://creativecommons.org/licenses/by-nc-sa/3.0/) for supplementary data. See [musicbrainz.org/doc/MusicBrainz_License](https://musicbrainz.org/doc/MusicBrainz_License).
- The ISRC system is managed by the **International ISRC Registration Authority** (IFPI). ISRCs themselves are factual identifiers and are not subject to copyright.
- Spotify and Deezer data (when used) is subject to their respective API Terms of Service.
- MeedyaManager retrieves ISRC and related metadata solely for the purpose of organising the user's own media library. No data is redistributed.

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
