# 🎧 Pandora Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **Pandora** metadata provider in MeedyaManager. Pandora currently operates as a **stub provider** because Pandora does not offer a public metadata API.

---

> ⚠️ **Not reachable from the app even if it worked.** `meedya lookup` (the CLI command) is a
> permanent stub that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`).

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

`PandoraProvider` is one of six providers built from the same `stub_provider!` macro in
`crates/mm-providers/src/music/mod.rs`. It has a correct `id()` ("`pandora`"), `display_name()`
("Pandora") and a declared `ProviderCapabilities` (music search, cover art flagged as supported),
but its `search()` method makes **no HTTP request of any kind** — it always returns an error:

- If the stub is disabled (the default) → `ProviderError::NotConfigured`.
- If the stub is explicitly enabled → `ProviderError::NotSupported("pandora: Provider
  implementation pending API review")`.

Pandora does not provide a public API for music metadata lookup or search, and no unofficial
integration or URL-construction helper exists in the codebase — there is no code that builds a
Pandora search URL, and nothing writes a Pandora-related tag.

**Current status:** Stub provider — always returns an error, never a result.

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

`PandoraProvider::new(enabled: bool)` takes a single boolean. There is no per-provider
`settings.json5` schema for it — no `priority` field exists anywhere in the codebase. The stub
defaults to disabled (`enabled_default = false` in the macro invocation).

> **Note:** Even if `enabled` is set to `true`, the provider still returns an error for every
> search — it changes only which error variant comes back (`NotSupported` instead of
> `NotConfigured`).

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

No custom tags are produced. Nothing in the codebase writes a `custom_pandora_*` tag, and there is
no URL-construction helper of any kind for Pandora's web interface.

---

## Current Status & Limitations

### Why is this a stub?

Pandora (owned by SiriusXM) has never offered a public metadata API. Their platform is primarily a streaming radio service with personalised stations, and their technical infrastructure is not exposed to third-party developers for metadata lookup.

### What works

- The provider registers with a correct `id()`, `display_name()`, and declared capabilities.
- Calling `search()` reliably returns a typed error rather than panicking or hanging.

### What does not work

- No network request is ever made.
- No metadata, cover art, or identifier is ever returned.

### Will this change?

There are no known plans for Pandora to release a public metadata API. If Pandora launches a developer programme in the future, this provider will be updated to support it.

---

## Troubleshooting

### `NotConfigured` error

**This is expected.** The stub is disabled by default; this is the "not enabled" error, not a sign
of a misconfiguration.

### `NotSupported: Provider implementation pending API review`

**This is expected if you set `enabled: true`.** It confirms the stub is reachable but there is no
real implementation behind it yet.

### Can I use a third-party Pandora library?

There are no known public APIs or community projects for Pandora metadata access, and MeedyaManager
does not bundle or depend on one. If Pandora opens their API in the future, this page will be
updated.

---

## Legal Notes

- Pandora does **not** provide a public API for metadata lookup.
- Pandora is a registered trademark of Pandora Media, LLC (a subsidiary of Sirius XM Holdings Inc.)
- MeedyaManager does not attempt to access Pandora's services in any way — the provider is a
  placeholder only. Any future integration would need to consider Pandora's
  [Terms of Use](https://www.pandora.com/legal).

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
