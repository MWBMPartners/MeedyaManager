# ❤️ iHeart Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **iHeart** (iHeartRadio) metadata provider in MeedyaManager. iHeart currently operates as a **stub provider** — the code exists, but it does not talk to iHeartRadio and never returns a result.

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

`iHeartProvider` is one of six providers built from the same `stub_provider!` macro in
`crates/mm-providers/src/music/mod.rs`. It has a correct `id()` ("`iheart`"), `display_name()`
("iHeart") and a declared `ProviderCapabilities` (music search, cover art flagged as supported),
but its `search()` method makes **no HTTP request of any kind** — it always returns an error:

- If the stub is disabled (the default) → `ProviderError::NotConfigured`.
- If the stub is explicitly enabled → `ProviderError::NotSupported("iheart: Provider
  implementation pending API review")`.

There is no iHeartRadio client, no `api.iheart.com` request, and no lyrics/cover-art parsing
anywhere in the codebase beyond the id/name/capabilities declaration.

**Current status:** Stub provider — always returns an error, never a result.

**Planned features (if this is ever implemented):**

- Track search via iHeartRadio's (undocumented) public endpoints
- Basic metadata: title, artist, album
- Static cover art, and lyrics where the response carries them

---

> ⚠️ **Not reachable from the app even if it worked.** `meedya lookup` (the CLI command) is a
> permanent stub that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). So even a future,
> real iHeart implementation would need new wiring in both places before a user could reach it
> from the shipped application.

---

## Authentication

There is nothing to authenticate — `search()` never sends a request, so no credential of any kind
is read, checked, or required. iHeartRadio has no official developer programme.

### No setup possible

There is nothing to install or configure; setting an environment variable for iHeart has no
effect.

---

## Configuration

### Environment Variables (`.env`)

None are read for iHeart.

### Settings (`settings.json5`)

`iHeartProvider::new(enabled: bool)` takes a single boolean. There is no per-provider
`settings.json5` schema for it, and nothing in the CLI or GUI constructs this provider from a
configuration file. The stub defaults to disabled (`enabled_default = false` in the macro
invocation).

---

## Available Data

The provider returns no data — `search()` always errors, in both the disabled and enabled states.

| Field | Status |
| ----- | ------ |
| `title` | Not available |
| `artist` | Not available |
| `album` | Not available |
| `lyrics` | Not available |
| Cover art | Not available |

---

## Custom Tags

No custom tags are produced. Nothing in the codebase writes a `custom_iheart_*` tag.

---

## Current Status & Limitations

### Why is this a stub?

iHeartMedia does not publish a developer API; any integration would rely on undocumented,
reverse-engineered endpoints. The source comments mark this as "pending API review" and no such
client has been written.

### What works now

- The provider registers with a correct `id()`, `display_name()`, and declared capabilities.
- Calling `search()` reliably returns a typed error rather than panicking or hanging.

### What does not work

- No network request is ever made.
- No metadata, lyrics, or cover art is ever returned.
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

- iHeartRadio is a service owned by **iHeartMedia, Inc.** MeedyaManager ships no iHeartRadio
  integration, official or unofficial — the provider is a placeholder only.
- Any future implementation relying on undocumented endpoints would need to consider
  [iHeartMedia's Terms of Use](https://www.iheart.com/content/terms-of-use/).

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
