# Privacy Policy

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Last updated: 2026-03-06

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
- **No cloud storage** — All configuration and media files remain on your
  device unless you explicitly enable cloud monitoring (which only reads
  file metadata from services you connect).

---

## Data Stored Locally

MeedyaManager stores the following data on your device:

| Data | Location | Purpose |
| ---- | -------- | ------- |
| Settings | `settings.json5` in your platform config directory | Application preferences |
| Test Mode manifest | `testmode_manifest.json` in config directory | Tracks test-mode file pairs |
| Corruption log | `corruption.log` in config directory | Records failed write operations |
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

Each provider has its own privacy policy governing how they handle your
queries. MeedyaManager does not control these services.

| Provider | Privacy Policy |
| -------- | -------------- |
| MusicBrainz | <https://musicbrainz.org/doc/MusicBrainz_Privacy_Policy> |
| Discogs | <https://www.discogs.com/privacy> |
| Spotify | <https://www.spotify.com/legal/privacy-policy/> |
| TMDb | <https://www.themoviedb.org/privacy-policy> |
| TVDb | <https://thetvdb.com/privacy-policy> |
| AcoustID | <https://acoustid.org/privacy> |
| Deezer | <https://www.deezer.com/legal/personal-datas> |
| Tidal | <https://tidal.com/privacy> |
| Shazam | <https://www.shazam.com/privacy> |
| IMDb | <https://www.imdb.com/privacy> |
| ISRC | <https://isrc.ifpi.org/en/privacy-policy> |
| ISWC | <https://www.iswc.org/privacy> |
| iHeartRadio | <https://www.iheart.com/privacy/> |
| Pandora | <https://www.pandora.com/privacy> |

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
4. Media server mode (local network only, user-initiated)

MeedyaManager does **not** make any network requests at idle. All network
activity is user-initiated.

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
