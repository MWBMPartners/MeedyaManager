# 📦 Amazon Music Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **Amazon Music** metadata provider in MeedyaManager. Amazon Music currently operates as a **stub provider** due to the absence of a public API — this guide explains the current status, what to expect, and how to enable it if API access becomes available.

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

The Amazon Music provider is designed to integrate with the **Amazon Music API** for track and album metadata searches. However, Amazon Music does **not currently provide a public API** — access is limited to closed beta participants who have been invited by Amazon.

MeedyaManager includes this provider as a framework stub that will become functional when Amazon opens their API to the public, or when a user gains access to the closed beta programme.

**Current status:** Stub provider — returns no results for most users (no public API).

**Planned features (when API becomes available):**

- Track and album search via the Amazon Music catalog
- ASIN (Amazon Standard Identification Number) retrieval
- Static cover art
- Integration with Amazon's HD and Ultra HD quality indicators

---

## Authentication

### When the API becomes available

Amazon Music is expected to use **OAuth2** authentication when their API launches publicly. The planned credential flow is:

1. Register for the Amazon Music Developer programme
2. Create an application and obtain OAuth tokens
3. Configure the `AMAZON_MUSIC_AUTH` environment variable

### Current state

Since the API is in closed beta, no authentication setup is possible for most users.

> **Note:** A community-maintained Python package `amazon-music` exists that uses reverse-engineered endpoints. If installed, MeedyaManager may attempt to use it as a fallback. However, this is **not recommended** due to Terms of Service risks — see [Legal Notes](#legal-notes).

---

## Configuration

### Environment Variables (`.env`)

```env
# Amazon Music API credentials (when available)
AMAZON_MUSIC_AUTH=your_oauth_token_here
```

| Variable | Description |
| -------- | ----------- |
| `AMAZON_MUSIC_AUTH` | OAuth authentication token (when API becomes publicly available) |

### Settings (`settings.json5`)

```json5
{
  providers: {
    amazon_music: {
      enabled: false,                   // Disabled by default (API not publicly available)
      priority: 8,                      // Provider priority (lower = higher priority)
      accept_tos_risk: false,           // Must be explicitly set to true to enable unofficial access
    }
  }
}
```

| Setting | Default | Description |
| ------- | ------- | ----------- |
| `enabled` | `false` | Disabled by default — enable only when you have API access |
| `priority` | `8` | Search priority relative to other providers |
| `accept_tos_risk` | `false` | Must be explicitly set to `true` to use unofficial community packages |

> **Important:** The `accept_tos_risk` flag exists as a safeguard. Using unofficial Amazon Music access methods may violate Amazon's Terms of Service. By setting this to `true`, you acknowledge this risk.

---

## Available Data

When the API becomes available, the Amazon Music provider is expected to return:

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | Track title | "Bohemian Rhapsody" |
| `artist` | Artist name | "Queen" |
| `album` | Album title | "A Night at the Opera" |
| `year` | Release year | "1975" |
| `asin` | Amazon Standard Identification Number | "B000002J0F" |

---

## Custom Tags

The following custom tags will be stored when the provider becomes functional:

| Custom Tag | Description | Example |
| ---------- | ----------- | ------- |
| `custom_amazon_music_asin` | Amazon Standard Identification Number | `"B000002J0F"` |
| `custom_amazon_music_url` | Amazon Music URL for the track | `"https://music.amazon.com/albums/B000002J0F"` |

---

## Current Status & Limitations

### What works now

- The provider is registered in MeedyaManager's provider framework
- Status reporting is functional (shows "closed beta" message in logs and UI)
- Manual reference URLs can be constructed for the Amazon Music web interface
- The framework is ready to be activated when API access is granted

### What does not work

- The provider returns empty results — it will not be used in searches
- No cover art is available

### When will it be available?

Amazon has not announced a public release date for their music metadata API. MeedyaManager will update this provider when a public API becomes available.

---

## Troubleshooting

### "Amazon Music API is in closed beta — provider unavailable"

**This is expected behaviour.** The provider is intentionally disabled because Amazon has not released a public API.

**If you have closed beta access:**
1. Set `AMAZON_MUSIC_AUTH` in your `.env` file with your OAuth token
2. Set `enabled: true` in `settings.json5` under `providers.amazon_music`
3. The provider will be updated in a future release to validate beta credentials

### "Amazon Music API not available — search skipped"

**This is expected behaviour.** MeedyaManager logs this informational message when the Amazon Music provider is queried but unavailable. It does not indicate an error — the system gracefully falls back to other enabled providers.

> **MeedyaManager does not officially support or endorse unofficial Amazon Music API access.** If Amazon opens their API publicly, this provider will be updated accordingly.

---

## Legal Notes

- Amazon Music does **not** currently provide a public API for metadata search
- Access to the Amazon Music API is restricted to **closed beta participants** invited by Amazon
- Use of unofficial or reverse-engineered access methods may violate [Amazon's Conditions of Use](https://www.amazon.com/gp/help/customer/display.html?nodeId=508088)
- MeedyaManager does not ship with or require any unofficial Amazon Music libraries
- The `accept_tos_risk` configuration flag serves as an explicit user acknowledgement of potential Terms of Service violations when using unofficial methods
- When the API becomes publicly available, this provider will be updated with official authentication flows
- MeedyaManager stores ASINs and URLs as custom metadata tags for reference only

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
