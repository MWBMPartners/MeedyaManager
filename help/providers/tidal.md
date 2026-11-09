# 🌊 TIDAL Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **TIDAL** metadata provider in MeedyaManager. TIDAL currently operates as a **stub provider** — the code exists, but it does not talk to TIDAL and never returns a result.

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

`TidalProvider` is one of six providers built from the same `stub_provider!` macro in
`crates/mm-providers/src/music/mod.rs`. It has a correct `id()` ("`tidal`"), `display_name()`
("Tidal") and a declared `ProviderCapabilities` (music search, cover art flagged as supported),
but its `search()` method makes **no HTTP request of any kind** — it always returns an error:

- If the stub is disabled (the default) → `ProviderError::NotConfigured`.
- If the stub is explicitly enabled → `ProviderError::NotSupported("tidal: Provider
  implementation pending API review")`.

There is no OAuth client, no TIDAL API request-signing, and no audio-quality-tier or spatial-audio
parsing anywhere in the codebase beyond the id/name/capabilities declaration. TIDAL requires a
registered developer application and an approval process, and no such integration has been built.

**Current status:** Stub provider — always returns an error, never a result.

**Planned features (if this is ever implemented):**

- Track and album search against the TIDAL catalog
- ISRC retrieval
- Audio-quality-tier metadata (Lossless / Hi-Res / MQA) and spatial-audio flags (Dolby Atmos,
  Sony 360 Reality Audio), if TIDAL's API exposes them to third-party client credentials
- Static cover art

---

> ⚠️ **Not reachable from the app even if it worked.** `meedya lookup` (the CLI command) is a
> permanent stub that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). So even a future,
> real TIDAL implementation would need new wiring in both places before a user could reach it from
> the shipped application.

---

## Authentication

There is nothing to authenticate — `search()` never sends a request, so no client ID, client
secret, or token is ever read, checked, or required.

### No setup possible

Setting `TIDAL_CLIENT_ID` / `TIDAL_CLIENT_SECRET` (or any other environment variable) has no
effect: no code path reads them for this provider.

---

## Configuration

### Environment Variables (`.env`)

None are read for TIDAL. The generic `CredentialStore` 4-tier resolver in
`crates/mm-providers/src/credentials.rs` (env `MM_<PROVIDER>_<KEY>` → config map → OS keyring →
plaintext `credentials.json`) would pick up `MM_TIDAL_*` values if something called it for this
provider — nothing does.

### Settings (`settings.json5`)

`TidalProvider::new(enabled: bool)` takes a single boolean. There is no per-provider
`settings.json5` schema for it — `mm_core::config::ProviderConfig` has no `tidal_enabled`,
`country_code`, or similar field, and nothing constructs this provider from `settings.json5`. The
stub defaults to disabled (`enabled_default = false` in the macro invocation).

---

## Available Data

The provider returns no data — `search()` always errors, in both the disabled and enabled states.

| Field | Status |
| ----- | ------ |
| `title` | Not available |
| `artist` | Not available |
| `album` | Not available |
| `isrc` | Not available |
| Audio quality / spatial audio | Not available |
| Cover art | Not available |

---

## Custom Tags

No custom tags are produced. Nothing in the codebase writes a `custom_tidal_*` tag.

---

## Current Status & Limitations

### Why is this a stub?

Building a real TIDAL provider means registering a developer application, going through TIDAL's
approval process, and implementing OAuth2 client-credentials request signing — none of which has
been done. The source comments mark this as "pending API review".

### What works now

- The provider registers with a correct `id()`, `display_name()`, and declared capabilities.
- Calling `search()` reliably returns a typed error rather than panicking or hanging.

### What does not work

- No network request is ever made.
- No metadata, cover art, quality tier, or spatial-audio flag is ever returned.
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

- TIDAL's public developer API requires a registered application and TIDAL's approval; MeedyaManager
  does not ship any TIDAL integration, official or otherwise.
- Any future implementation would need to comply with the [TIDAL Developer Terms and Conditions](https://developer.tidal.com/documentation/guidelines).
- "TIDAL" is a trademark of TIDAL Music AS.

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
