# 🎧 Pandora Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **Pandora** metadata provider in MeedyaManager. Pandora currently operates as a **stub provider** because Pandora does not offer a public metadata API.

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Configuration](#configuration)
4. [Available Data](#available-data)
5. [Custom Tags](#custom-tags)
6. [Current Status & Limitations](#current-status--limitations)
7. [Troubleshooting](#troubleshooting)
8. [Legal Notes](#legal-notes)

---

## Overview

The Pandora provider exists as a **stub** in MeedyaManager's provider framework. Pandora does not provide a public API for music metadata lookup or search. Unlike Amazon Music (which has a closed beta API), Pandora has no known developer programme or API offering for metadata access.

This provider is included in the framework for completeness and to construct reference URLs for the Pandora web interface, allowing users to manually look up tracks on Pandora when needed.

**Current status:** Stub provider — always returns no results.

**What this provider can do:**

- Construct Pandora web search URLs for manual reference
- Store Pandora URLs as custom tags (when manually entered or imported)
- Provide clear status messages in logs and the provider dashboard

**What this provider cannot do:**

- Search the Pandora catalog programmatically
- Retrieve metadata, cover art, or ISRC codes
- Authenticate with Pandora's services

---

## Authentication

Pandora does **not** provide a public API. There is no authentication mechanism, API key, or developer programme available.

### No setup possible

This provider requires no configuration and cannot be made functional through user action. It is included in MeedyaManager's framework as a placeholder.

---

## Configuration

### Environment Variables (`.env`)

No environment variables are required or used for Pandora.

### Settings (`settings.json5`)

```json5
{
  providers: {
    pandora: {
      enabled: false,                   // Disabled by default (no API available)
      priority: 99,                     // Lowest priority (stub provider)
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `false` | Disabled by default — no functional API available |
| `priority` | `99` | Lowest priority (will never be used for searching) |

> **Note:** Even if `enabled` is set to `true`, the provider will return empty results for all searches. Enabling it has no negative effect but provides no benefit.

---

## Available Data

The Pandora provider does **not** return any metadata fields. All searches return empty results.

| Field | Status |
| ----- | ------ |
| `title` | Not available |
| `artist` | Not available |
| `album` | Not available |
| `year` | Not available |
| `isrc` | Not available |
| Cover art | Not available |

---

## Custom Tags

The following custom tag is available for manual use (e.g. when importing metadata from other tools):

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_pandora_url` | Pandora web search URL (manually constructed) | `"https://www.pandora.com/search/Queen"` |

This tag can be populated manually or through metadata import, but will **not** be populated automatically by the Pandora provider.

### URL Construction

The provider includes a helper for constructing Pandora web search URLs:

```text
https://www.pandora.com/search/{url_encoded_query}
```

For example, searching for "Queen Bohemian Rhapsody" generates:

```text
https://www.pandora.com/search/Queen%20Bohemian%20Rhapsody
```

These URLs are for **manual reference only** — opening them in a browser will take you to the Pandora web interface where you can search for the track.

---

## Current Status & Limitations

### Why is this a stub?

Pandora (owned by SiriusXM) has never offered a public metadata API. Their platform is primarily a streaming radio service with personalised stations, and their technical infrastructure is not exposed to third-party developers for metadata lookup.

### What works

- The provider is registered in MeedyaManager's provider framework
- Status reporting shows "no public API" in logs and the provider dashboard
- Manual Pandora web search URLs can be constructed
- The `custom_pandora_url` tag can be stored manually

### What does not work

- The provider is disabled (returns empty results for all searches)
- No automated metadata or cover art retrieval

### Will this change?

There are no known plans for Pandora to release a public metadata API. If Pandora launches a developer programme in the future, this provider will be updated to support it.

---

## Troubleshooting

### "Pandora does not provide a public API"

**This is expected behaviour.** The provider logs this message to confirm that it is correctly recognising the absence of an API. No action is needed.

### "Pandora has no public API — search skipped"

**This is expected behaviour.** MeedyaManager logs this informational message when the Pandora provider is queried during a multi-provider search. The system gracefully falls back to other enabled providers.

### Can I use a third-party Pandora library?

There are no known public APIs or community projects for Pandora metadata access. If Pandora opens their API in the future, MeedyaManager will update this provider to support it.

---

## Legal Notes

- Pandora does **not** provide a public API for metadata lookup
- Pandora is a registered trademark of Pandora Media, LLC (a subsidiary of Sirius XM Holdings Inc.)
- Any reverse-engineering of Pandora's services would likely violate their [Terms of Use](https://www.pandora.com/legal)
- MeedyaManager does not attempt to access Pandora's services programmatically
- The Pandora provider stub constructs web URLs for manual reference only — no automated requests are made to Pandora's servers
- MeedyaManager includes this provider for framework completeness and to support the `custom_pandora_url` tag for metadata imported from other tools

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
