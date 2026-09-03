# 🟢 Spotify Provider — Setup Guide

> **(C) 2025-2026 MWBM Partners Ltd**

This guide covers the **Spotify** metadata provider in MeedyaManager: a real OAuth2 client-credentials client against the Spotify Web API's `/v1/search` endpoint. It does **not** fetch Spotify's audio-features data (energy, danceability, tempo, valence, key, mode) — no such request exists in this codebase.

---

## 📋 Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Configuration](#configuration)
4. [Available Data](#available-data)
5. [Custom Tags](#custom-tags)
6. [Cover Art](#cover-art)
7. [Troubleshooting](#troubleshooting)
8. [Legal Notes](#legal-notes)

---

> ⚠️ **Not reachable from the app today.** `meedya lookup` (the CLI command) is a permanent stub
> that prints "Provider support is coming in M5" and never calls any provider
> (`crates/mm-cli/src/commands/lookup.rs`) — there is no `--list-providers` flag. The GTK lookup
> panel constructs **MusicBrainz only** (`crates/mm-gtk/src/ui/lookup_panel.rs`). `SpotifyProvider`
> is real and compiled, but nothing in the shipped CLI or GUI constructs it. Setting
> `MM_SPOTIFY_CLIENT_ID` / `MM_SPOTIFY_CLIENT_SECRET` is parsed into `mm_core::config::AppConfig`
> at startup and will drive an "enabled but no credential configured" warning there, but **no
> CLI/GUI command currently constructs a `SpotifyProvider` with those values** — setting them has
> no effect on a lookup today.

---

## Overview

`SpotifyProvider` (`crates/mm-providers/src/music/mod.rs`) obtains a bearer token via the
Client Credentials OAuth2 flow, then calls `GET {base}/v1/search?type=track`. It is a genuine HTTP
client with real request/response handling — but its own test suite covers only response *parsing*
against canned JSON, not a full request/response cycle: the OAuth token endpoint is
**hard-coded to `https://accounts.spotify.com/api/token`** rather than derived from the
provider's configurable `base_url`, so a test that points `base_url` at a mock server still hits
the real Spotify accounts server for the token step. There is no end-to-end (wiremock) test for
Spotify's `search()`, unlike MusicBrainz/ISRC/ISWC.

**Key features actually implemented:**

- Track search via the Spotify catalogue, with an ISRC-first query (`isrc:<code>`) when an ISRC is
  supplied, otherwise `track:"<title>" artist:"<artist>"` field-prefixed terms
- ISRC retrieval from the track's `external_ids.isrc`
- A popularity score (0-100, normalised to 0.0-1.0) and an explicit-content flag
- Static cover art up to whatever resolution `album.images[]` returns (Spotify typically serves up
  to 640×640)

**Not implemented — do not expect these:**

- Audio features (energy, danceability, tempo, valence, key, mode) — there is no call to Spotify's
  audio-features endpoint anywhere in this codebase (this was tracked as issue #172, parked
  upstream; it has not been built)
- Any endpoint beyond `/v1/search` and the token endpoint

---

## Authentication

Spotify uses OAuth2 **Client Credentials** flow, which needs only a Client ID and Client Secret
(no user login, no Spotify Premium required).

### Step-by-step setup

1. Go to [developer.spotify.com/dashboard](https://developer.spotify.com/dashboard) and log in
   with a free Spotify account.
2. **Create an app**, tick the Web API checkbox, and save.
3. Copy the **Client ID** and **Client Secret** from the app's settings page.

### How MeedyaManager reads them today

`SpotifyProvider::new(client_id: Option<String>, client_secret: Option<String>)` takes both
values directly as constructor arguments. Separately, `MM_SPOTIFY_CLIENT_ID` and
`MM_SPOTIFY_CLIENT_SECRET` **are** read into `mm_core::config::AppConfig` at startup
(`crates/mm-core/src/config/mod.rs`) — but as the banner above explains, nothing currently threads
that config into a constructed `SpotifyProvider`. If you want to use Spotify lookups before that
wiring lands, pass the values directly to `SpotifyProvider::new(...)` in your own code.

The generic 4-tier `CredentialStore` in `crates/mm-providers/src/credentials.rs` (env
`MM_<PROVIDER>_<KEY>` → in-memory config map → OS keyring → local `credentials.json`) also exists
and would resolve `MM_SPOTIFY_*` values if something called it — nothing does yet. Its tier 4 is a
**plain JSON file on disk**, not the AES-256-GCM-encrypted bundle earlier project plans described
(issue #209).

---

## Configuration

### Environment Variables (`.env`)

```env
MM_SPOTIFY_CLIENT_ID=your_client_id_here
MM_SPOTIFY_CLIENT_SECRET=your_client_secret_here
```

These are parsed into `AppConfig` (see above) but not yet used to construct a working provider.

### Settings (`settings.json5`)

There is no per-provider `settings.json5` schema — `priority` and `fetch_audio_features` are not
real settings; nothing reads them (and there is no audio-features fetch to toggle in the first
place). The only real inputs are the constructor's `client_id`/`client_secret` and an optional
`base_url` override used by the crate's own tests.

---

## Available Data

| Field | Source | Example |
| ----- | ------ | ------- |
| `title` | `track.name` | "Bohemian Rhapsody" |
| `artist` | `track.artists[].name` joined with `"; "` | "Queen" |
| `album` | `track.album.name` | "A Night at the Opera" |
| `year` | first 4 digits of `track.album.release_date` | "1975" |
| `isrc` | `track.external_ids.isrc` | "GBUM71029604" |
| score | `track.popularity` (0-100) normalised to 0.0-1.0 | 0.85 |
| content advisory (metadata) | `track.explicit` mapped to `"explicit"`/`"clean"` | "clean" |
| duration (metadata) | `track.duration_ms` / 1000 | 354.0 |
| provider ID (metadata) | `track.id` | "4u7EnebtmKWzUH433cf5Qv" |

There is **no `track_num`/`disc_num` field populated** (the parser doesn't read
`track_number`/`disc_number` from the Spotify response) and **no audio-features block of any
kind** — no energy, danceability, tempo, valence, key, or mode.

---

## Custom Tags

Any `custom_spotify_*` tag would come from the fields above once tag-writing wiring exists for
this provider (see the reachability banner). There is **no** `custom_spotify_energy`,
`custom_spotify_danceability`, `custom_spotify_tempo`, or `custom_spotify_valence` — those would
require the audio-features fetch that does not exist.

---

## Cover Art

Every image in `track.album.images[]` is mapped through as-is (URL, width, height, `image/jpeg`) —
Spotify itself determines the sizes returned; MeedyaManager does not request a specific size or
upscale anything.

---

## Troubleshooting

### "Spotify: failed to obtain OAuth2 access token" / `AuthenticationFailed`

- Verify both `client_id` and `client_secret` were passed to `SpotifyProvider::new(...)`.
- A non-2xx response from the token endpoint raises this error with the HTTP status included.

### HTTP 429 from `/v1/search`

- Mapped directly to `ProviderError::RateLimited("spotify")`. MeedyaManager's shared rate limiter
  (`governor`) is **not** consulted before this request — only MusicBrainz, ISRC, and ISWC go
  through the shared limiter. A burst of Spotify searches can hit this with no client-side
  throttling.

### Expecting audio features (energy, danceability, tempo, valence)

**This is expected to be absent.** No code in this provider calls Spotify's audio-features
endpoint — see [Available Data](#available-data).

---

## Legal Notes

- The Spotify Web API is provided under the [Spotify Developer Terms of Service](https://developer.spotify.com/terms/).
- A free Spotify account is sufficient for Client Credentials access (no Premium required).
- Cover art and metadata are the property of their respective rights holders.
- MeedyaManager retrieves metadata solely for the purpose of organising the user's own media
  library; this does not imply endorsement by Spotify AB.

---

> 📝 *See [configuration.md](../configuration.md) for the full settings reference, or return to [getting-started.md](../getting-started.md) for initial setup.*
