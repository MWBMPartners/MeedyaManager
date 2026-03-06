# 🏛️ EIDR Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains how to configure and use the **EIDR** (Entertainment Identifier Registry) metadata provider in MeedyaManager.

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Configuration](#configuration)
4. [Available Data](#available-data)
5. [Custom Tags](#custom-tags)
6. [EIDR ID Format Reference](#eidr-id-format-reference)
7. [Troubleshooting](#troubleshooting)
8. [Legal Notes](#legal-notes)

---

## Overview

The EIDR provider looks up metadata from the **Entertainment Identifier Registry** — an industry-standard, globally unique identification system for audiovisual content (movies, TV shows, episodes, edits, distributions). EIDR IDs are used by major studios, broadcasters, and distributors for supply chain management and rights tracking.

This provider is useful for:

- Resolving EIDR content IDs to movie/TV show metadata
- Cross-referencing EIDR IDs with other identifiers (ISAN, IMDB, etc.)
- Identifying specific edits or distributions of a title (e.g., theatrical cut vs. director's cut)
- Professional media asset management workflows

> **Important:** EIDR membership is **paid** and requires an application process. Most home users will not have EIDR access. This provider is included for professional media workflows and will gracefully report itself as unavailable if credentials are not configured.

---

## Authentication

**EIDR requires paid membership and HTTP Basic Auth credentials.**

### Who Needs EIDR Access?

EIDR is primarily used by:

- Film and TV studios
- Broadcast networks
- Distribution platforms (streaming services, VOD providers)
- Post-production companies
- Rights management organisations

If you are managing a personal media library, you likely do not need this provider. TMDB and TVDB provide equivalent metadata for personal use.

### Step-by-Step: Getting EIDR Credentials

1. **Apply for EIDR membership** at [eidr.org/join](https://www.eidr.org/join/)
   - Membership tiers include: Full Member, Associate Member, and Registered User
   - Pricing varies by organisation type and size
2. **Receive your credentials** — after approval, EIDR will provide:
   - A **Client ID** (also called Party ID or User ID)
   - A **Client Secret** (password/token)
3. **Configure MeedyaManager** with these credentials (see [Configuration](#configuration) below)

### Authentication Flow

The EIDR API uses **HTTP Basic Authentication**:

1. MeedyaManager encodes your Client ID and Client Secret as a Base64 `Authorization` header
2. Each API request includes this header
3. EIDR validates the credentials against its member database
4. No token refresh is needed — credentials are sent with every request

---

## Configuration

### Environment Variables (`.env`)

Add your EIDR credentials to the `.env` file in the project root:

```env
# EIDR credentials — requires paid EIDR membership
# Get your credentials from your EIDR member dashboard
EIDR_CLIENT_ID=your_eidr_client_id_here
EIDR_CLIENT_SECRET=your_eidr_client_secret_here
```

### Settings File (`settings.json5`)

You can optionally configure the provider's behaviour in `config/settings.json5`:

```json5
{
  providers: {
    eidr: {
      // Enable or disable this provider (default: true)
      // Even if enabled, provider will report unavailable without valid credentials
      enabled: true,

      // EIDR API endpoint (default: production registry)
      // Do not change unless EIDR directs you to a different endpoint
      api_endpoint: "https://resolve.eidr.org/EIDR/",

      // Maximum number of search results to evaluate (1-20, default: 5)
      result_limit: 5,

      // Whether to resolve full metadata for matched IDs (default: true)
      // When false, only the EIDR ID is stored without additional lookups
      resolve_full_metadata: true,

      // Request timeout in seconds (default: 30)
      request_timeout: 30,
    }
  }
}
```

### Alternative: Credentials in Settings

If you prefer not to use a `.env` file:

```json5
{
  providers: {
    eidr: {
      client_id: "your_eidr_client_id_here",
      client_secret: "your_eidr_client_secret_here",
    }
  }
}
```

> **Security note:** The `.env` approach is strongly preferred because `.env` is git-ignored by default. EIDR credentials represent paid access and should be kept secure.

---

## Available Data

The EIDR provider returns the following standard metadata fields when resolving an EIDR ID:

| Field | Description | Example |
| ----- | ----------- | ------- |
| `title` | Title of the content | `"Inception"` |
| `year` | Release year | `"2010"` |
| `director` | Director name (for movies) | `"Christopher Nolan"` |
| `show` | Series title (for TV episodes) | `"Breaking Bad"` |
| `season` | Season number (for TV episodes) | `"5"` |
| `episode` | Episode number (for TV episodes) | `"14"` |

EIDR's data model is hierarchical:

```text
Abstraction (concept of the work)
  └── Edit (specific version: theatrical, director's cut, etc.)
       └── Distribution (specific release: Blu-ray, streaming, etc.)
```

MeedyaManager resolves the highest-level metadata available for the given EIDR ID.

---

## Custom Tags

The provider writes the following custom tag to media files:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_eidr_id` | Full EIDR Content ID | `"10.5240/7EC7-228A-510A-053E-2B96-C"` |

The EIDR ID is the primary value this provider contributes. It serves as a permanent, globally unique identifier for the content that is recognised across the entertainment industry.

Since EIDR IDs are long and contain special characters (`.` and `/`), they are stored as custom tags rather than being used in file paths. If you need to reference an EIDR ID in a rename rule, use the custom tag syntax:

```json5
// Not recommended for file paths due to special characters,
// but useful in logs or JSON export:
// {custom_eidr_id}
```

---

## EIDR ID Format Reference

An EIDR Content ID follows the DOI (Digital Object Identifier) format:

```text
10.5240/XXXX-XXXX-XXXX-XXXX-XXXX-C
```

| Component | Description |
| --------- | ----------- |
| `10.5240` | EIDR's DOI prefix (always the same for EIDR) |
| `/` | Separator between prefix and suffix |
| `XXXX-XXXX-XXXX-XXXX-XXXX` | 20 hexadecimal characters in 5 groups of 4, separated by hyphens |
| `-C` | Check character (single hex digit for validation) |

**Full example:** `10.5240/7EC7-228A-510A-053E-2B96-C`

### Validation Rules

MeedyaManager validates EIDR IDs against these rules:

- Starts with `10.5240/`
- Followed by 5 groups of 4 hexadecimal characters, separated by hyphens
- Ends with a hyphen and a single check character
- The regex pattern: `^10\.5240/[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]$`

---

## Troubleshooting

### Provider shows "Missing credentials" or "Unavailable"

This is **expected** for most users. EIDR requires paid membership. If you do have credentials:

- Ensure `EIDR_CLIENT_ID` and `EIDR_CLIENT_SECRET` are set in your `.env` file
- Verify there are no extra spaces or newline characters in the values
- Check that your `.env` file is next to `settings.json5`
- Run `meedya lookup --list-providers` to verify provider status

### "401 Unauthorized" errors in logs

- Your EIDR credentials are invalid or have expired
- Contact EIDR support to verify your account status
- Ensure you are using the correct Client ID and Secret pair
- Check if your EIDR membership has been renewed

### "403 Forbidden" errors

- Your EIDR membership tier may not include API access
- Some content in the registry may be restricted to certain member tiers
- Contact EIDR to verify your API access permissions

### Slow responses or timeouts

The EIDR API can be slower than consumer-facing APIs. If you experience timeouts:

1. Increase `request_timeout` in your settings (default: 30 seconds)
2. The EIDR registry may be undergoing maintenance — check [eidr.org/status](https://www.eidr.org)
3. Ensure your network allows outbound HTTPS connections to `resolve.eidr.org`

### EIDR ID not found

- Verify the EIDR ID format matches the pattern described above
- Not all content has been registered with EIDR — the registry primarily covers commercially distributed content
- Check the EIDR registry directly at [ui.eidr.org](https://ui.eidr.org) to confirm the ID exists

---

## Legal Notes

- **EIDR** is operated by the Entertainment Identifier Registry Association, a joint venture of industry organisations.
- EIDR API access requires a **paid membership agreement** with the EIDR Association. Terms are governed by the EIDR Membership Agreement.
- EIDR Content IDs are DOI-based identifiers. The identifier strings themselves are factual references and not subject to copyright.
- Metadata retrieved from EIDR is subject to the terms of your EIDR membership agreement. Redistribution of EIDR data may require additional licensing.
- MeedyaManager stores EIDR IDs in media file tags solely for the purpose of organising the user's own media library.
- For more information: [eidr.org](https://www.eidr.org)

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
