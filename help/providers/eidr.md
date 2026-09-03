# 🏛️ EIDR Provider — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

This guide explains what the **EIDR** (Entertainment Identifier Registry) metadata provider actually does in MeedyaManager — including why its response parsing is **unverified against a real EIDR response**.

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

> ⚠️ **Untested against a real EIDR response.** `EidrProvider`'s JSON parser
> (`crates/mm-providers/src/identifiers/mod.rs`) expects a shape (`ResourceName.value`,
> `ExtraObjectMetadata.movie.directors`) that is a **best-effort guess**, not something verified
> against a live or recorded EIDR response. EIDR's documented registry format is XML; this
> provider requests `Accept: application/json` and assumes EIDR honours that, but there is no
> wiremock or fixture test in this codebase that exercises `parse_eidr_json()` against anything
> resembling a real registry payload — only a hand-written JSON fixture the developer believes is
> representative. Treat this provider as unverified until someone tests it against a real EIDR
> account.
>
> **Not reachable from the app regardless.** `meedya lookup` (the CLI command) is a permanent stub
> that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`).

---

## Overview

The EIDR provider looks up metadata from the **Entertainment Identifier Registry** — an industry-standard, globally unique identification system for audiovisual content (movies, TV shows, episodes, edits, distributions). EIDR IDs are used by major studios, broadcasters, and distributors for supply chain management and rights tracking.

`EidrProvider::search()` requires an EIDR ID already present in the query (there is no title/artist
search path for EIDR) and sends:

```text
GET {base}/EIDR/object/<DOI>
Authorization: Basic <base64(username:password)>
Accept: application/json
```

> **Important:** EIDR membership is **paid** and requires an application process. Most home users will not have EIDR access. This provider is included for professional media workflows and will report itself as unconfigured if credentials are not supplied.

---

## Authentication

**EIDR requires paid membership and HTTP Basic Auth credentials.**

### Who Needs EIDR Access?

EIDR is primarily used by film/TV studios, broadcast networks, distribution platforms, post-production companies, and rights management organisations. If you are managing a personal media library, you likely do not need this provider — TMDb and TheTVDB provide equivalent metadata for personal use.

### Getting EIDR Credentials

1. Apply for EIDR membership at [eidr.org/join](https://www.eidr.org/join/).
2. After approval, EIDR provides a username and password for HTTP Basic Auth.

### How MeedyaManager uses them (and how it doesn't, yet)

`EidrProvider::new(username: Option<String>, password: Option<String>)` takes both directly as
constructor arguments and sends them via `.basic_auth(...)` on every request. **There is no
environment variable or `settings.json5` field read for EIDR anywhere in the codebase** —
`mm_core::config::ProviderConfig` has no EIDR field, and no CLI/GUI call site constructs
`EidrProvider` with a credential today. If you want to use this provider, you would pass the
username/password directly to `EidrProvider::new(...)` in your own code.

The generic 4-tier `CredentialStore` in `crates/mm-providers/src/credentials.rs` (env
`MM_EIDR_*` → in-memory config map → OS keyring → local `credentials.json`) would resolve those
values if something called it for this provider — nothing does. Its tier 4 is a **plain JSON
file on disk**, not the AES-256-GCM-encrypted bundle earlier project plans described (issue #209).

---

## Configuration

### Environment Variables (`.env`)

None are read. `EIDR_CLIENT_ID` / `EIDR_CLIENT_SECRET` (or any `MM_EIDR_*` name) are not consulted
anywhere in the codebase today.

### Settings File (`settings.json5`)

There is no per-provider `settings.json5` schema for EIDR — `result_limit`,
`resolve_full_metadata`, and `request_timeout` are not real settings; nothing reads them. The only
real inputs are the constructor's `username`/`password` and an optional `base_url` override used
by the crate's own tests (default: `https://id.eidr.org`).

---

## Available Data

`parse_eidr_json()` maps a single EIDR record into one `ProviderResult`:

| Field | Response path (as guessed by this parser) | Notes |
| ----- | ------------------------------------------ | ----- |
| `title` | `ResourceName.value` | |
| `artist` | `ExtraObjectMetadata.movie.directors[0]` | first director only, mapped into the generic `artist` field |
| `year` | first 4 digits of `ReleaseDate` | |
| provider ID / EIDR ID (metadata) | `ID` | stored in both the generic provider-ID slot and a dedicated EIDR metadata key |

There is **no** `show`, `season`, or `episode` field — this parser only ever returns a single flat
result, not the Abstraction/Edit/Distribution hierarchy EIDR's data model actually has. Because
the response shape itself is an unverified guess (see the banner above), treat every field in this
table as "what the code currently assumes", not "what EIDR is documented to return".

---

## Custom Tags

| Value | Description | Example |
| ----- | ----------- | ------- |
| EIDR ID | Full EIDR Content ID, stored in a dedicated metadata key | `"10.5240/7EC7-228A-510A-053E-2B96-C"` |

Since EIDR IDs contain special characters (`.` and `/`), using them directly in a file-path rename
template is not recommended.

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
| `-C` | Check character (single hex digit) |

**Full example:** `10.5240/7EC7-228A-510A-053E-2B96-C`

### Validation Rules

`validate_eidr()` is a **loose** check: it only confirms the value starts with `10.5240/` and is
longer than 10 characters. It does **not** verify the 5-groups-of-4-hex-digits structure or the
check character shown above, despite that structure being documented — a malformed suffix will
still pass this validator.

---

## Troubleshooting

### Provider errors with `NotConfigured`

No username/password was supplied when the provider was constructed — expected for most users, as
this is a paid, invite-only registry with no wiring into `settings.json5` today (see
[Authentication](#authentication)).

### "401 Unauthorized" errors

Your EIDR credentials are invalid, expired, or your membership tier lacks API access. Contact EIDR
support to verify your account.

### Results look wrong or fields are missing

**This is expected given the banner at the top of this page** — the JSON shape this provider
parses is a guess, never verified against a real EIDR response. If your account returns a
differently-shaped payload, this provider will likely parse it incorrectly or return mostly-empty
results rather than erroring loudly.

---

## Legal Notes

- **EIDR** is operated by the Entertainment Identifier Registry Association, a joint venture of industry organisations.
- EIDR API access requires a **paid membership agreement** with the EIDR Association.
- EIDR Content IDs are DOI-based identifiers; the identifier strings themselves are factual references and not subject to copyright.
- Metadata retrieved from EIDR is subject to the terms of your EIDR membership agreement.
- MeedyaManager stores EIDR IDs in media file tags solely for the purpose of organising the user's own media library.
- For more information: [eidr.org](https://www.eidr.org)

---

> 📝 *For general configuration help, see [configuration.md](../configuration.md). For other providers, see the [providers directory](./).*
