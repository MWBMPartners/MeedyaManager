# Privacy Policy

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Last updated: 2026-09-03

## Overview

MeedyaManager is a local-first media file manager. Your files, your data,
your control. This policy explains what information MeedyaManager accesses,
how it is used, and what is sent to third-party services.

---

## What MeedyaManager Does NOT Do

- **No analytics or telemetry** — MeedyaManager does not collect usage data,
  crash reports, or behavioural analytics.
- **No user accounts** — There is no sign-up, login, or registration.
- **No tracking** — No cookies, fingerprinting, or advertising identifiers.
- **No cloud storage** — All configuration and media files remain on your device. Cloud
  storage monitoring (OneDrive, Google Drive, Dropbox, MEGA, iCloud) is architectural
  scaffolding only right now: `mm-cloud` makes no real network calls and no OAuth flow
  actually runs, so there is nothing to disclose here yet (issues #94 through #102, reopened
  as stubs).

---

## Data Stored Locally

MeedyaManager stores the following data on your device:

| Data | Location | Purpose |
| ---- | -------- | ------- |
| Settings | `settings.json5` in your platform config directory | Application preferences |
| Test Mode manifest | `testmode_manifest.json` in config directory | Tracks test-mode file pairs |
| Corruption log | `corruption.log` in config directory | Records failed tag-write operations (see [File Integrity](file-integrity.md) — this log is not currently populated by any real edit path) |
| API credentials | OS keyring (macOS Keychain, Windows Credential Manager, Linux Secret Service) | Authentication for metadata providers |

None of this data is transmitted to MWBM Partners Ltd or any third party.

---

## Third-Party Metadata Providers

When you enable a metadata provider and perform a lookup, MeedyaManager
sends limited search queries to that provider's API. The data sent typically
includes:

- **Artist name**, **track title**, **album name** (text-only search terms)
- Your configured **API key** (sent directly to the provider, not to MWBM)

MeedyaManager does **not** send audio files, file paths, or any personally
identifiable information to these services.

### Provider Privacy Policies

This list is exactly the set of providers that have a real, network-calling implementation in
`crates/mm-providers` (verified by checking each provider's `impl MetadataProvider` for an
actual HTTP call). Several other provider names appear in the codebase (YouTube Music, Amazon
Music, Pandora, Tidal, Shazam, iHeart) but are `stub_provider!` placeholders that make no
network calls at all and are disabled by default — nothing you do can cause them to transmit
your data today, so they are intentionally omitted here. Discogs and AcoustID do not exist in
the codebase in any form.

| Provider | Used For | Privacy Policy |
| -------- | -------- | -------------- |
| MusicBrainz | Music metadata; also backs ISRC and ISWC lookups | <https://musicbrainz.org/doc/MusicBrainz_Privacy_Policy> |
| Spotify | Music metadata | <https://www.spotify.com/legal/privacy-policy/> |
| Deezer | Music metadata | <https://www.deezer.com/legal/personal-datas> |
| Apple (iTunes Search API) | Apple Music, Apple TV, iTunes Store, and Apple Podcasts metadata | <https://www.apple.com/legal/privacy/> |
| TMDb | Movie/TV metadata | <https://www.themoviedb.org/privacy-policy> |
| TheTVDB | TV metadata | <https://thetvdb.com/privacy-policy> |
| OMDb (IMDb data) | Movie/TV metadata — requires your own API key | <https://www.omdbapi.com/legal.htm> |
| EIDR | Identifier (EIDR) lookups | <https://eidr.org/privacy-policy/> |

ISRC and ISWC lookups are performed against MusicBrainz's own API
(`musicbrainz.org/ws/2/recording` and `/ws/2/work`), so they share MusicBrainz's privacy
policy rather than having one of their own.

### Enabling Providers is Opt-In

No provider is contacted until you explicitly enable it in Settings and
perform a lookup. MusicBrainz is enabled by default because it is a free,
open-data service that does not require an API key.

---

## Update Checks

When enabled, MeedyaManager checks the GitHub Releases API for newer
versions. This sends:

- The MeedyaManager **User-Agent string** (application name + version + platform)
- A standard HTTPS request to `api.github.com`

No personal data is included. Update checks can be disabled in Settings.

---

## Network Access

MeedyaManager requires outbound network access (`internetClient` /
`NSNetworkClient`) for:

1. Metadata provider API calls (only when you perform a lookup)
2. Cover art downloads (from provider CDNs)
3. Update checks (GitHub Releases API)

MeedyaManager does **not** make any network requests at idle. All network
activity is user-initiated.

There is no media server mode to disclose here yet: `meedya serve` currently exits immediately
after printing a stub message and never opens a network listener
(`crates/mm-cli/src/commands/serve.rs`) — no axum router is ever built. This page will be
updated with a real network-access disclosure once M10 (Secure Media Server) delivers a
working server.

---

## Children's Privacy

MeedyaManager does not knowingly collect any information from children
under the age of 13. The application does not require or request personal
information from any user.

---

## Open Source

MeedyaManager is open-source software licensed under GPL-2.0-or-later.
The complete source code is available for inspection at:

<https://github.com/MWBMPartners/MeedyaManager>

---

## Contact

For privacy-related questions or concerns:

- **Email:** dev@mwbm.co.uk
- **GitHub Issues:** <https://github.com/MWBMPartners/MeedyaManager/issues>

---

## Changes to This Policy

This privacy policy may be updated as new features are added. The "Last
updated" date at the top of this document reflects the most recent revision.
Significant changes will be noted in the changelog.
