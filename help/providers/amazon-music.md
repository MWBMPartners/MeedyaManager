# 📦 Amazon Music Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **Amazon Music** metadata provider in MeedyaManager. Amazon Music currently operates as a **stub provider** due to the absence of a public API — this guide explains the current status, what to expect, and how to enable it if API access becomes available.

---

> ⚠️ **Not reachable from the app even if it worked.** `meedya lookup` (the CLI command) is a
> permanent stub that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). So even a future,
> real Amazon Music implementation would need new wiring in both places before a user could reach
> it from the shipped application.

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

`AmazonMusicProvider` is one of six providers built from the same `stub_provider!` macro in
`crates/mm-providers/src/music/mod.rs`. It has a correct `id()` ("`amazon_music`"),
`display_name()` ("Amazon Music") and a declared `ProviderCapabilities` (music search, cover art
flagged as supported), but its `search()` method makes **no HTTP request of any kind** — it always
returns an error:

- If the stub is disabled (the default) → `ProviderError::NotConfigured`.
- If the stub is explicitly enabled → `ProviderError::NotSupported("amazon_music: Provider
  implementation pending API review")`.

There is no OAuth client and no Amazon Music-specific code anywhere in the codebase beyond the
id/name/capabilities declaration. Amazon Music does **not currently provide a public API** — access
is limited to closed beta participants who have been invited by Amazon — and no such integration
has been implemented.

**Current status:** Stub provider — always returns an error, never a result.

**Planned features (when API becomes available):**

- Track and album search via the Amazon Music catalog
- ASIN (Amazon Standard Identification Number) retrieval
- Static cover art
- Integration with Amazon's HD and Ultra HD quality indicators

---

## Authentication

### There is nothing to authenticate today

`search()` never sends a request, so no credential of any kind is read, checked, or required.
There is no `AMAZON_MUSIC_AUTH` variable, no OAuth client, and no fallback to any community-maintained
package — MeedyaManager does not depend on or bundle one.

### If Amazon opens a public API in future

The planned credential flow, when there is code behind it, would likely mirror the pattern used by
Spotify elsewhere in this crate: register an application, obtain OAuth2 credentials, and resolve
them through the generic 4-tier `CredentialStore` (env `MM_AMAZON_MUSIC_*` → config map → OS
keyring → plaintext `credentials.json` — see `crates/mm-providers/src/credentials.rs`; note tier 4
is a plain JSON file, not encrypted). None of that is wired up yet.

---

## Configuration

### Environment Variables (`.env`)

None are read for Amazon Music today.

### Settings (`settings.json5`)

`AmazonMusicProvider::new(enabled: bool)` takes a single boolean. There is no per-provider
`settings.json5` schema for it — no `priority` or `accept_tos_risk` field exists anywhere in the
codebase. The stub defaults to disabled (`enabled_default = false` in the macro invocation).

---

## Available Data

The provider returns no data — `search()` always errors, in both the disabled and enabled states.

| Field | Status |
| ----- | ------ |
| `title` | Not available |
| `artist` | Not available |
| `album` | Not available |
| `asin` | Not available |
| Cover art | Not available |

---

## Custom Tags

No custom tags are produced. Nothing in the codebase writes a `custom_amazon_music_*` tag.

---

## Current Status & Limitations

### What works now

- The provider registers with a correct `id()`, `display_name()`, and declared capabilities.
- Calling `search()` reliably returns a typed error rather than panicking or hanging.

### What does not work

- No network request is ever made.
- No metadata, cover art, or ASIN is ever returned.
- `enabled: true` does not make it functional — it only changes which error variant comes back
  (`NotSupported` instead of `NotConfigured`).

### When will it be available?

Amazon has not announced a public release date for their music metadata API, and no beta
integration has been built here. This page will be updated if that changes.

---

## Troubleshooting

### `NotConfigured` error

**This is expected.** The stub is disabled by default; this is the "not enabled" error, not a sign
of a misconfiguration.

### `NotSupported: Provider implementation pending API review`

**This is expected if you set `enabled: true`.** It confirms the stub is reachable but there is no
real implementation behind it yet.

---

## Legal Notes

- Amazon Music does **not** currently provide a public API for metadata search.
- Access to the Amazon Music API is restricted to **closed beta participants** invited by Amazon.
- MeedyaManager does not ship with, depend on, or require any unofficial Amazon Music libraries —
  the provider is a placeholder only.
- Any future implementation would need to consider [Amazon's Conditions of Use](https://www.amazon.com/gp/help/customer/display.html?nodeId=508088).

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
