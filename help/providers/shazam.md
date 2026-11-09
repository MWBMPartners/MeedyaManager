# 🔊 Shazam Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **Shazam** metadata provider in MeedyaManager. Shazam currently operates as a **stub provider** — the code exists, but it does not talk to Shazam, does not fingerprint audio, and never returns a result.

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

`ShazamProvider` is one of six providers built from the same `stub_provider!` macro in
`crates/mm-providers/src/music/mod.rs`. It has a correct `id()` ("`shazam`"), `display_name()`
("Shazam") and a declared `ProviderCapabilities` (music search, cover art flagged as supported),
but its `search()` method makes **no HTTP request and reads no audio** — it always returns an
error:

- If the stub is disabled (the default) → `ProviderError::NotConfigured`.
- If the stub is explicitly enabled → `ProviderError::NotSupported("shazam: Provider
  implementation pending API review")`.

There is **no audio fingerprinting code anywhere in MeedyaManager.** No file is opened and read to
build a fingerprint, no fingerprinting algorithm is implemented or linked (there is no
`chromaprint`/`rusty-chromaprint`-style dependency), and no request of any kind is sent to Shazam.
`SearchQuery` (the struct every provider receives) carries text fields only — title, artist,
album, year, identifiers — with nothing that represents "a chunk of decoded audio", so there is
no channel for a fingerprint to travel through even in principle.

**Current status:** Stub provider — always returns an error, never a result.

**Planned features (if this is ever implemented):**

- Text-based track search by title/artist
- Audio fingerprinting from file content (this would require new infrastructure well beyond this
  provider — a fingerprinting library, an audio-decoding path, and a `SearchQuery` extension)
- Static cover art

---

> ⚠️ **Not reachable from the app even if it worked.** `meedya lookup` (the CLI command) is a
> permanent stub that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). So even a future,
> real Shazam implementation would need new wiring in both places before a user could reach it
> from the shipped application.

---

## Authentication

There is nothing to authenticate — `search()` never sends a request, so no credential of any kind
is read, checked, or required.

### No setup possible

There is nothing to install or configure; setting an environment variable for Shazam has no
effect.

---

## Configuration

### Environment Variables (`.env`)

None are read for Shazam.

### Settings (`settings.json5`)

`ShazamProvider::new(enabled: bool)` takes a single boolean. There is no per-provider
`settings.json5` schema for it, and in particular there is **no `fingerprint_enabled` setting** —
that would imply a fingerprinting feature that does not exist. The stub defaults to disabled
(`enabled_default = false` in the macro invocation).

---

## Available Data

The provider returns no data — `search()` always errors, in both the disabled and enabled states.

| Field | Status |
| ----- | ------ |
| `title` | Not available |
| `artist` | Not available |
| `genre` | Not available |
| Cover art | Not available |

---

## Custom Tags

No custom tags are produced. Nothing in the codebase writes a `custom_shazam_*` tag.

---

## Current Status & Limitations

### Why is this a stub?

Audio fingerprinting is a substantial feature — decoding audio, generating a fingerprint, and
matching it against a recognition service — and the source comments mark this integration as
"pending API review". None of that work has been started.

### What works now

- The provider registers with a correct `id()`, `display_name()`, and declared capabilities.
- Calling `search()` reliably returns a typed error rather than panicking or hanging.

### What does not work

- No network request is ever made and no audio file is ever read.
- No metadata, cover art, or identifier is ever returned.
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

- Shazam is a service owned by **Apple Inc.** MeedyaManager ships no Shazam integration, official
  or reverse-engineered — the provider is a placeholder only.
- Any future implementation would need to consider [Shazam's Terms of Use](https://www.shazam.com/terms)
  and [Apple's Terms of Service](https://www.apple.com/legal/internet-services/terms/site.html).

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
