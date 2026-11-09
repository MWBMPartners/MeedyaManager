# ▶️ YouTube Music Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **YouTube Music** metadata provider in MeedyaManager. YouTube Music currently operates as a **stub provider** — the code exists, but it does not talk to YouTube Music and never returns a result.

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

`YouTubeMusicProvider` is one of six providers built with the same `stub_provider!` macro in
`crates/mm-providers/src/music/mod.rs`. It has a correct `id()` ("`youtube_music`"),
`display_name()` ("YouTube Music") and a declared `ProviderCapabilities` (music search, cover art
flagged as supported), but its `search()` method makes **no HTTP request of any kind** — it always
returns an error:

- If the stub is disabled (the default) → `ProviderError::NotConfigured`.
- If the stub is explicitly enabled → `ProviderError::NotSupported("youtube_music: Provider
  implementation pending API review")`.

There is no cookie/header extraction, no internal-API client, and no YouTube Music-specific code
of any kind beyond the id/name/capabilities declaration. YouTube Music does not offer a public
developer API, and no unofficial integration has been implemented.

**Current status:** Stub provider — always returns an error, never a result.

**Planned features (if this is ever implemented):**

- Track and album search against YouTube Music
- Video IDs for direct YouTube Music links
- Static cover art (thumbnail JPEG)

---

> ⚠️ **Not reachable from the app even if it worked.** `meedya lookup` (the CLI command) is a
> permanent stub that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). So even a future,
> real YouTube Music implementation would need new wiring in both places before a user could reach
> it from the shipped application.

---

## Authentication

There is nothing to authenticate — `search()` never sends a request, so no credential of any kind
is read, checked, or required. Nothing in this project extracts browser cookies, headers, or
tokens for YouTube Music.

### No setup possible

Setting any environment variable for this provider has no effect: no code path reads one.

---

## Configuration

### Environment Variables (`.env`)

None are read. `MM_YOUTUBE_MUSIC_*` variables would be picked up by the generic
`CredentialStore` 4-tier resolver in `crates/mm-providers/src/credentials.rs` if something called
it for this provider — nothing does.

### Settings (`settings.json5`)

`YouTubeMusicProvider::new(enabled: bool)` takes a single boolean. There is no per-provider
`settings.json5` schema for it: `mm_core::config::ProviderConfig` has no `youtube_music_enabled`
field, and nothing in the CLI or GUI constructs this provider with a value read from
`settings.json5` — so there is no `enabled`/`priority` setting to change here. The stub defaults to
disabled (`enabled_default = false` in the macro invocation).

---

## Available Data

The provider returns no data — `search()` always errors, in both the disabled and enabled states.

| Field | Status |
| ----- | ------ |
| `title` | Not available |
| `artist` | Not available |
| `album` | Not available |
| Cover art | Not available |

---

## Custom Tags

No custom tags are produced. Nothing in the codebase writes a `custom_youtube_music_*` tag.

---

## Current Status & Limitations

### Why is this a stub?

YouTube Music has no public metadata API. Building a working provider would mean maintaining an
unofficial integration against Google's internal endpoints — flagged in the source comments as
"pending API review" and never carried out.

### What works now

- The provider registers with a correct `id()`, `display_name()`, and declared capabilities.
- Calling `search()` reliably returns a typed error rather than panicking or hanging.

### What does not work

- No network request is ever made.
- No metadata, cover art, or identifiers are ever returned.
- `enabled: true` does not make it functional — it only changes which error variant comes back
  (`NotSupported` instead of `NotConfigured`).

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

- YouTube Music does **not** provide a public API for metadata search.
- MeedyaManager ships no reverse-engineered or unofficial YouTube Music client — the provider is a
  placeholder only.
- Any future implementation would need to consider [YouTube's Terms of Service](https://www.youtube.com/t/terms).

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
