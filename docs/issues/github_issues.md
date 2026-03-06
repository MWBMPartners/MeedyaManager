# MeedyaManager — GitHub Issues Register

> **(C) 2025-2026 MWBM Partners Ltd**
>
> This file is the authoritative local record of all GitHub Issues for the
> MeedyaManager project. All issues are created on GitHub before work begins
> and closed after verification. This document mirrors that state.

---

## Issue Status Key

| Symbol | Meaning |
| ------ | ------- |
| ✅ | Closed — complete |
| 🔲 | Open — not yet started |
| 🔄 | Open — in progress |

---

## M0 — Repository Setup & Scaffolding (Issues #19–#39)

| # | Title | Status |
| - | ----- | ------ |
| #19 | Archive Python v1.x codebase at `v1.5-M6-python-final` | ✅ |
| #20 | Delete Python source tree from main branch | ✅ |
| #21 | Initialise Cargo workspace with 8 crates | ✅ |
| #22 | Scaffold `mm-core` crate with stub modules | ✅ |
| #23 | Scaffold `mm-cli` crate with stub `main.rs` | ✅ |
| #24 | Scaffold `mm-ffi` crate (UniFFI + cbindgen stubs) | ✅ |
| #25 | Scaffold `mm-providers`, `mm-cloud`, `mm-export`, `mm-server`, `mm-update` stubs | ✅ |
| #26 | Scaffold `mm-gtk` crate (GTK4 Linux shell stub) | ✅ |
| #27 | Create macOS SwiftUI project scaffold (`macos/`) | ✅ |
| #28 | Create Windows WinUI 3 project scaffold (`windows/`) | ✅ |
| #29 | Set up Rust toolchain config (rustfmt, clippy, deny, rust-toolchain.toml) | ✅ |
| #30 | Create 8 GitHub Actions CI/CD workflows | ✅ |
| #31 | Create GitHub Projects v2 board with 11 milestones | ✅ |
| #32 | Implement automated version management (`version-bump.yml`) | ✅ |
| #33 | Implement CI version-sync check in `ci-rust.yml` | ✅ |
| #34 | Enhance `release.yml` with 5-platform builds + checksums | ✅ |
| #35 | Create GitHub Wiki — Version Management page | ✅ |
| #36 | Create GitHub Wiki — Release Process page | ✅ |
| #37 | Create GitHub Wiki — CI/CD Pipelines page | ✅ |
| #38 | Write `Dev_Notes.md` (versioning, release, CI reference) | ✅ |
| #39 | Write initial README, Project_Plan.md, PROJECT_STATUS.md, ROADMAP.md | ✅ |

---

## M1 — Core Engine (Issues #40–#51)

| # | Title | Status |
| - | ----- | ------ |
| #40 | Implement error types module (`errors.rs`) with `thiserror` | ✅ |
| #41 | Implement config module (JSON5 + `.env` + env-var override) | ✅ |
| #42 | Implement media classifier (Group/Format/Class/Quality 4-level) | ✅ |
| #43 | Implement metadata extraction + writing (`lofty` integration) | ✅ |
| #44 | Implement file system watcher (`notify` + debounce + filtering) | ✅ |
| #45 | Implement rename simulator + filename sanitizer | ✅ |
| #46 | Implement companion file detector (subtitles, lyrics, art, cue) | ✅ |
| #47 | Implement state manager + single-instance lock file | ✅ |
| #48 | Implement structured logging (tracing + PII redaction) | ✅ |
| #49 | Implement health checks (config, folders, disk, writable) | ✅ |
| #50 | Wire all mm-core modules + achieve 217 passing tests | ✅ |
| #51 | Update docs for M1 completion + bump version to `v0.2.0` | ✅ |

---

## M2 — Rule Engine (no separate issues — done inline)

| # | Title | Status |
| - | ----- | ------ |
| — | Implement lexer (tokenizer: tags, functions, literals, legacy detection) | ✅ |
| — | Implement recursive descent parser (AST, 50-level depth guard) | ✅ |
| — | Implement tag registry (40+ bidirectional mappings, virtual tags) | ✅ |
| — | Implement 24 template functions (logical, string, numeric, lookup) | ✅ |
| — | Implement evaluator (EvalContext, multi-value, missing tag modes) | ✅ |
| — | Implement declarative rule system (conditions, operators, priority) | ✅ |
| — | Wire renamer integration + config extension | ✅ |
| — | Achieve 182 tests + bump version to `v0.3.0` | ✅ |

---

## M3 — CLI (Issues #52–#62)

| # | Title | Status |
| - | ----- | ------ |
| #52 | Implement shared output infrastructure (`output.rs`, `ExitCode`) | ✅ |
| #53 | Implement CLI context (`context.rs`, `CliContext`) | ✅ |
| #54 | Implement `meedya debug` — single-file metadata inspector | ✅ |
| #55 | Implement `meedya rule` — template validator + tag listing + test | ✅ |
| #56 | Implement `meedya config` — show/path/init/export/import | ✅ |
| #57 | Implement `meedya scan` — directory scan + rename preview + execute | ✅ |
| #58 | Implement `meedya edit` — metadata write (--set, --remove, --cover) | ✅ |
| #59 | Implement `meedya watch` — foreground watcher with event logging | ✅ |
| #60 | Implement `meedya lookup` — provider search (stub for M5) | ✅ |
| #61 | Implement `meedya report-bug` — system info + log collection | ✅ |
| #62 | Achieve 45 CLI tests + bump version to `v0.4.0` | ✅ |

---

## M4 — FFI Layer & Native UI Shells (Issues #63–#72)

| # | Title | Status |
| - | ----- | ------ |
| #63 | Implement `mm-ffi` crate — UniFFI scaffolding + types + callbacks | ✅ |
| #64 | Implement `mm-ffi` UniFFI API (8 exported functions wired to mm-core) | ✅ |
| #65 | Implement `mm-ffi` C API (9 `#[no_mangle]` functions + JSON transport) | ✅ |
| #66 | Set up cbindgen + build.rs — generates `include/mm_ffi.h` | ✅ |
| #67 | Implement `mm-gtk` main window (AdwTabView, 4 tabs, AboutDialog) | ✅ |
| #68 | Implement `mm-gtk` scan, metadata, rules, settings panels | ✅ |
| #69 | Implement macOS SwiftUI shell (AppState, ContentView, 4 views, MmCore.swift) | ✅ |
| #70 | Implement Windows WinUI 3 shell (MainWindow, 4 pages, MmCore.cs) | ✅ |
| #71 | Achieve 20 mm-ffi tests + GTK4 state tests | ✅ |
| #72 | Bump version to `v0.5.0` + update docs | ✅ |

---

## M5 — Metadata Lookup Providers (Issues #73–#84)

| # | Title | Status |
| - | ----- | ------ |
| #73 | Implement provider traits (MetadataProvider, SearchQuery, ProviderResult) | ✅ |
| #74 | Implement credential resolution (4-tier: env/config/keyring/file) | ✅ |
| #75 | Implement rate limiter (token bucket, per-provider, registry) | ✅ |
| #76 | Implement fuzzy match scoring (weighted, MatchScorer, rank_results) | ✅ |
| #77 | Implement cover art helpers (size selection, deduplication, URL validators) | ✅ |
| #78 | Implement music providers: MusicBrainz, Spotify, AppleMusic, Deezer | ✅ |
| #79 | Implement 6 stub music providers (YouTube Music, Amazon, Pandora, etc.) | ✅ |
| #80 | Implement video providers: TMDB, TheTVDB, OMDb, Apple TV, iTunes Store | ✅ |
| #81 | Implement podcast provider: Apple Podcasts | ✅ |
| #82 | Implement identifier providers: ISRC, EIDR, ISWC | ✅ |
| #83 | Implement provider registry + fan-out search | ✅ |
| #84 | Achieve 332 tests + bump version to `v0.6.0` + update docs | ✅ |

---

## M6 — Full Native UI (Issues #85–#93)

| # | Title | Status |
| - | ----- | ------ |
| #85 | Implement GTK4 Lookup panel (search + results + cover art) | ✅ |
| #86 | Implement GTK4 full rule builder (template + preview + tag pills) | ✅ |
| #87 | Implement macOS Lookup view + rule builder (full implementation) | ✅ |
| #88 | Implement Windows LookupPage + rule builder | ✅ |
| #89 | Implement drag-and-drop folder import (GTK4/macOS/Windows) | ✅ |
| #90 | Implement real settings save to disk (all 3 platforms) | ✅ |
| #91 | Implement dark/light theme toggle (GTK4 adw::StyleManager) | ✅ |
| #92 | Add error dialogs (GTK4 adw::AlertDialog, macOS/Windows equivalents) | ✅ |
| #93 | Achieve ~90 UI tests + bump version to `v0.7.0` + update docs | ✅ |

---

## M7 — Cloud Storage Monitoring (Issues #94–#102)

| # | Title | Status |
| - | ----- | ------ |
| #94 | Implement `mm-cloud` trait layer (CloudProvider, CloudError, ChangeSet, etc.) | ✅ |
| #95 | Implement SyncManager (polling, conflict resolution, delta cursor) | ✅ |
| #96 | Implement OneDrive provider (Microsoft Graph API, delta queries) | ✅ |
| #97 | Implement Google Drive provider (Drive API v3, changes.list) | ✅ |
| #98 | Implement Dropbox provider (API v2, cursor-based delta) | ✅ |
| #99 | Implement MEGA stub provider (no official API) | ✅ |
| #100 | Implement iCloud stub provider (macOS FileProvider only) | ✅ |
| #101 | Implement Cloud UI tab on all 3 platforms (GTK4, macOS, Windows) | ✅ |
| #102 | Achieve ~90 tests + bump version to `v0.8.0` + update docs | ✅ |

---

## M8 — Packaging & Public Beta (Issues #103–#111)

| # | Title | Status |
| - | ----- | ------ |
| #103 | Implement `mm-update` crate (UpdateChecker, ReleaseInfo, semver comparison) | ✅ |
| #104 | Create Linux packaging manifests (Flatpak, Snap, AppImage, .deb) | ✅ |
| #105 | Create macOS entitlements + DMG creation script | ✅ |
| #106 | Create WinGet manifest (MWBM.MeedyaManager.yaml) | ✅ |
| #107 | Add GTK4 AdwBanner update notification (above tab bar) | ✅ |
| #108 | Add macOS "Updates" section in SettingsView | ✅ |
| #109 | Add Windows InfoBar + CheckForUpdatesAsync() | ✅ |
| #110 | Update `release.yml` (DMG creation, deb/AppImage build steps) | ✅ |
| #111 | Achieve ~30 tests + bump version to `v0.9.0` + update docs | ✅ |

---

## M9 — Database Export (Issues #112–#119)

| # | Title | Status |
| - | ----- | ------ |
| #112 | Implement `mm-export` trait layer (DatabaseExporter, DbDialect, ExportRow, ExportConfig, ExportStats, ExportError) | ✅ |
| #113 | Implement SchemaBuilder with dialect-aware DDL for all 5 backends | ✅ |
| #114 | Implement SQLite backend (INSERT OR REPLACE upsert) | ✅ |
| #115 | Implement MySQL + MariaDB backends (ON DUPLICATE KEY UPDATE upsert) | ✅ |
| #116 | Implement PostgreSQL backend (ON CONFLICT DO UPDATE, $1 params) | ✅ |
| #117 | Implement SQL Server backend (T-SQL MERGE, @param_name style) | ✅ |
| #118 | Implement `meedya export` CLI command (BackendChoice, detect_backend, redact_dsn, --show-schema) | ✅ |
| #119 | Implement Export UI tab on all 3 platforms (GTK4, macOS, Windows) + achieve ~90 tests + bump version to `v0.10.0` | ✅ |

---

## M10 — Secure Media Server + Public Release (Issues #120–#127)

| # | Title | Status |
| - | ----- | ------ |
| #120 | Implement `mm-server` core: axum router, TLS (rustls), config | ✅ |
| #121 | Implement JWT authentication (jsonwebtoken) + middleware | ✅ |
| #122 | Implement media streaming endpoints (Range requests, partial content) | ✅ |
| #123 | Implement REST API endpoints (library, search, metadata, export) | ✅ |
| #124 | Implement `meedya serve` CLI command | ✅ |
| #125 | Implement Server UI tab on all 3 platforms (GTK4, macOS, Windows) | ✅ |
| #126 | Final packaging pass: sign all release artifacts, update WinGet/Flathub | ✅ |
| #127 | Achieve ~90 tests + bump version to `v1.0.0` + full release docs + public release | ✅ |

---

## Cross-Cutting Issues

| # | Title | Priority | Status |
| - | ----- | -------- | ------ |
| #128 | Accessibility support: VoiceOver (macOS), Narrator (Windows), AT-SPI (Linux) | High | ✅ |
| #129 | Release binary hardening: LTO, strip, panic=abort, PIE, code signing | — | ✅ |
| #130 | Translation / Internationalisation (i18n) support — gettextrs CLI+Linux, .xcstrings macOS, .resw Windows | Medium | ✅ |
| #131 | Windows process check via OpenProcess (platform-specific state manager enhancement) | Low | ✅ |

---

## v1.1.0 — Post-Release Improvements (Issues #132–#133)

| # | Title | Status |
| - | ----- | ------ |
| #132 | Add centralised `FiletypeRegistry` module — MIME types, companion scopes, `.zip`/`.rar`/`.itlp` support | ✅ |
| #133 | Extend metadata tag coverage — sort fields, ReplayGain, classical, podcast, encoding tags | ✅ |

---

## Apple Platform Wishlist (Issues #134–#141)

> Apple-specific and Apple-enhanced features planned for future milestones.
> Tracked as open wishlist issues; scheduling TBD after v1.1.0.

| # | Title | Platform | Priority | Status |
| - | ----- | -------- | -------- | ------ |
| #134 | Music.app library import — parse `~/Music/Music/` library to bulk-import metadata, ratings, play counts | macOS | Medium | 🔲 |
| #135 | MusicKit framework integration — replace REST Apple Music provider with native `MusicKit` for richer catalog/library access | macOS | Medium | 🔲 |
| #136 | Quick Look extension — `QLPreviewExtension` for rich Finder media previews showing album art and tag info | macOS | Low | 🔲 |
| #137 | Siri Shortcuts / App Intents — expose scan/rename/lookup as `AppIntent` actions for Shortcuts app and Siri voice control | macOS/iOS | Low | 🔲 |
| #138 | Core ML audio fingerprinting — on-device track identification via Neural Engine (Sound Analysis), fully offline | macOS (Apple Silicon) | Low | 🔲 |
| #139 | Spotlight importer — publish library metadata via `CoreSpotlight` for system-wide Spotlight and Alfred search | macOS | Low | 🔲 |
| #140 | AirPlay 2 streaming — stream from the mm-server media server to AirPlay 2 receivers (HomePod, Apple TV) | macOS | Low | 🔲 |
| #141 | CloudKit settings sync — sync rename rules and config across Apple devices via iCloud / CloudKit | macOS/iOS | Low | 🔲 |

---

## v1.2.0 — Core Infrastructure Hardening (Issues #142–#149)

> Foundational improvements: external config files, service mode, integrity checking, multi-account cloud.

| # | Title | Platform | Priority | Status |
| - | ----- | -------- | -------- | ------ |
| #142 | External JSON5 filetype registry — move `config/filetypes.json5` out of Rust code; embed at compile time with user-override support at runtime | All | High | ✅ |
| #143 | External JSON5 metadata tag definitions — `config/tags.json5` with industry-standard mappings (MusicBrainz, MP3tag, Picard), MeedyaMeta namespace, user-extensible custom tags | All | High | ✅ |
| #144 | Full settings export/import — portable JSON5 config bundle (rules, tag definitions, filetype overrides, API keys, preferences) for device migration and backup | All | High | ✅ |
| #145 | Background service mode — run as systemd unit (Linux), launchd agent (macOS), Windows Service (Windows); toggle in Settings; default ON; minimal CPU/memory footprint | All | High | ✅ |
| #146 | Multi-account cloud storage — support multiple accounts per provider (e.g. two Google Drive accounts); Sign in with Apple for iCloud, MSAL for Microsoft, Google OAuth PKCE | All | Medium | 🔲 |
| #147 | File integrity verification — SHA256 checksums before and after every metadata write; atomic rename (write to `.tmp` then `rename(2)`); rollback on checksum mismatch; corruption log | All | High | ✅ |
| #148 | Binary release hardening (documented) — strip symbols, LTO, `panic=abort`, PIE, anti-debug notes; GPL-compatible only (full source obfuscation not legally permissible under GPL-2.0) | All | Medium | 🔲 |
| #149 | GitHub Wiki — write complete developer and user wiki pages (architecture, service setup, tag customisation, filetype customisation, release process, troubleshooting) | All | Medium | 🔲 |

---

## v1.3.0 — Filetype/Codec Architecture + Custom Type Management (Issues #150–#155)

> Dolby standalone format support, internal codec registry, removal of the end-user JSON file override path, and proper UI-driven custom filetype/tag management with validated text inputs.

| # | Title | Platform | Priority | Status |
| - | ----- | -------- | -------- | ------ |
| #150 | Register standalone Dolby audio formats in filetype registry — AC3, E-AC3 (EAC3/EC3), AC4; add `taggable` flag to `AudioFormat`; surface graceful "no embedded tag support" message in tag editor UI; allow normal watch/rename/classify operations | All | High | 🔲 |
| #151 | Internal codec registry — add `config/codecs.json5` dev-only reference file; define all recognised codecs with properties: `taggable`, `lossless`, `max_channels`, `typical_containers`, display name; consumed at compile time via `include_str!`; drives tagging capability detection independently of file extension | All | Medium | 🔲 |
| #152 | Remove end-user JSON file override for filetypes/tags — delete `load_user_override()` from `filetype_registry.rs` and the equivalent in `tags.rs`; remove `~/.config/meedyamanager/filetypes.json5` user override path; dev builds may keep a `MEEDYA_FILETYPES_OVERRIDE` env var for local testing; all user customisation goes through the Settings UI | All | High | 🔲 |
| #153 | UI: Custom Filetype management panel — add/edit/delete user-defined file types in Settings > File Types on GTK4, macOS (SwiftUI), Windows (WinUI 3); stored in `settings.json5` under `custom_filetypes`; user definitions supplement built-in base (user wins on conflict); list view with enable/disable toggles; no JSON editing exposed to users | All | High | 🔲 |
| #154 | UI: Custom Tag/metadata field management panel — add/edit/delete custom metadata field definitions in Settings > Tags on GTK4, macOS (SwiftUI), Windows (WinUI 3); fields: tag name, display label, data type (string/number/date/bool), per-format mappings (ID3v2 TXXX, Vorbis, MP4 atom); stored in `settings.json5` under `custom_tags`; user tags supplement base tag registry (user wins on conflict); list view with delete/toggle | All | High | 🔲 |
| #155 | Input validation and sanitisation for custom filetype/tag UI text inputs — validate extension (regex `^[a-z0-9]{1,12}$`, no leading dot, no spaces), MIME type (RFC 2045 pattern: `type/subtype[-suffix]`), tag names (regex `^[A-Za-z][A-Za-z0-9_:-]{0,63}$`), display names (max 64 chars, strip HTML/control chars, no null bytes); show inline error messages before user can save; no partial saves on validation failure; applied consistently across all 3 platform UIs | All | High | 🔲 |

### Detailed Issue Descriptions

---

#### #150 — Register Standalone Dolby Audio Formats + `taggable` Flag

**Background:**

Standalone `.ac3`, `.eac3`, `.ec3`, and `.ac4` files are raw Dolby audio bitstreams. Unlike container formats (MKV, MP4) that can carry multiple streams and metadata, these extensions represent the bare encoded audio with no standardised provision for embedded metadata tags. The `lofty` crate (our Rust metadata library) does not support reading or writing tags on these files.

Currently these extensions are not in `config/filetypes.json5` at all, so MeedyaManager ignores them entirely.

**Scope:**

1. Add to `config/filetypes.json5` audio section:

   ```json5
   { "ext": "ac3",  "mime": "audio/ac3",           "name": "Dolby Digital (AC-3)",        "lossless": false, "taggable": false },
   { "ext": "eac3", "mime": "audio/eac3",           "name": "Dolby Digital Plus (E-AC-3)", "lossless": false, "taggable": false },
   { "ext": "ec3",  "mime": "audio/eac3",           "name": "Dolby Digital Plus (E-AC-3)", "lossless": false, "taggable": false },
   { "ext": "ac4",  "mime": "audio/x-ac4",          "name": "Dolby AC-4",                  "lossless": false, "taggable": false },
   { "ext": "mlp",  "mime": "audio/x-mlp",          "name": "Meridian Lossless Packing",   "lossless": true,  "taggable": false },
   { "ext": "truehd","mime": "audio/x-truehd",      "name": "Dolby TrueHD",               "lossless": true,  "taggable": false },
   { "ext": "dts",  "mime": "audio/x-dts",          "name": "DTS Audio",                   "lossless": false, "taggable": false },
   { "ext": "dtshd","mime": "audio/x-dtshd-ma",     "name": "DTS-HD Master Audio",         "lossless": true,  "taggable": false },
   { "ext": "thd",  "mime": "audio/x-truehd",       "name": "Dolby TrueHD (alt ext)",     "lossless": true,  "taggable": false },
   ```

2. Add `taggable: bool` field to the `AudioFormat` struct in `filetype_registry.rs` (default `true` via `#[serde(default = "default_true")]`).

3. In the metadata editor (`meedya edit` and all UI tag editor panels): before attempting to read or write tags, check `taggable`. If `false`:
   - Show an informational message: *"This file format does not support embedded metadata tags. Metadata can be stored in MeedyaManager's library database, and rename templates can reference manually entered values."*
   - Disable the tag write UI controls (grey out / read-only)
   - Still allow rename/watch/classify operations as normal — the file extension being known is enough

4. In the `meedya edit` CLI: if `--tag` or `--cover` is used on a non-taggable file, return a clear error with exit code `2`:

   ```text
   error: .ac3 files do not support embedded metadata tags.
   Use `meedya edit --set-library-meta` to store metadata in the library database.
   ```

5. Add unit tests: `taggable_false_for_ac3`, `taggable_true_for_mp3`, `taggable_true_for_flac`.

6. Update `help/supported-formats.md` and `help/custom-filetypes.md` to document the `taggable` concept.

**Acceptance criteria:**

- `.ac3`, `.eac3`, `.ec3`, `.ac4` files appear in `meedya debug` output with correct classification
- `meedya edit song.ac3 --tag "Artist=Test"` prints a clear error and exits with code 2
- `meedya watch` picks up and renames `.ac3` files using filename/library metadata
- No panic or crash when processing untaggable formats

---

#### #151 — Internal Codec Registry (`config/codecs.json5`)

**Background:**

The filetype registry maps file *extensions* to format metadata. But many containers (.mkv, .mp4, .ts) can carry multiple different *codecs* — the actual encoding algorithms used for audio and video. Having a separate codec registry allows:

- Driving tagging capability at the codec level (not just extension level)
- Accurate quality classification for container-wrapped streams
- Future use: codec-aware transcoding advice, provider match scoring, surround sound tagging
- A single authoritative dev reference for all codecs MeedyaManager recognises

**Proposed `config/codecs.json5` structure:**

```json5
{
  "audio_codecs": [
    {
      "id": "ac3",              // Internal identifier
      "names": ["Dolby Digital", "AC-3", "DD"],
      "mime": "audio/ac3",
      "lossless": false,
      "taggable": false,        // Can embedded tags be written to bare streams?
      "max_channels": 6,        // 5.1 surround
      "typical_extensions": ["ac3"],
      "typical_containers": ["ac3", "ts", "mkv", "mp4", "vob", "m2ts"]
    },
    {
      "id": "eac3",
      "names": ["Dolby Digital Plus", "E-AC-3", "DD+", "Dolby Atmos (lossy)"],
      "mime": "audio/eac3",
      "lossless": false,
      "taggable": false,
      "max_channels": 16,
      "typical_extensions": ["eac3", "ec3"],
      "typical_containers": ["eac3", "ec3", "ts", "mkv", "mp4", "m2ts"]
    },
    {
      "id": "ac4",
      "names": ["Dolby AC-4", "AC4"],
      "mime": "audio/x-ac4",
      "lossless": false,
      "taggable": false,
      "max_channels": 24,
      "typical_extensions": ["ac4"],
      "typical_containers": ["ac4", "ts", "mp4"]
    },
    {
      "id": "truehd",
      "names": ["Dolby TrueHD", "TrueHD", "MLP"],
      "mime": "audio/x-truehd",
      "lossless": true,
      "taggable": false,
      "max_channels": 16,       // Up to Dolby Atmos object-based
      "typical_extensions": ["truehd", "mlp", "thd"],
      "typical_containers": ["truehd", "mlp", "mkv", "m2ts"]
    },
    {
      "id": "dts",
      "names": ["DTS Digital Surround", "DTS"],
      "mime": "audio/x-dts",
      "lossless": false,
      "taggable": false,
      "max_channels": 6,
      "typical_extensions": ["dts"],
      "typical_containers": ["dts", "ts", "mkv", "mp4", "vob"]
    },
    {
      "id": "dts_hd_ma",
      "names": ["DTS-HD Master Audio", "DTS-HD MA"],
      "mime": "audio/x-dtshd-ma",
      "lossless": true,
      "taggable": false,
      "max_channels": 8,
      "typical_extensions": ["dtshd"],
      "typical_containers": ["dtshd", "mkv", "m2ts"]
    },
    {
      "id": "aac",
      "names": ["Advanced Audio Coding", "AAC", "AAC-LC", "HE-AAC"],
      "mime": "audio/aac",
      "lossless": false,
      "taggable": true,
      "max_channels": 8,
      "typical_extensions": ["aac", "m4a", "mp4", "m4b"],
      "typical_containers": ["mp4", "m4a", "ts", "mkv", "3gp"]
    },
    // ... mp3, flac, alac, opus, vorbis, wav, pcm, etc.
  ],
  "video_codecs": [
    {
      "id": "h264",
      "names": ["H.264", "AVC", "MPEG-4 Part 10"],
      "mime": "video/H264",
      "lossless": false,
      "taggable": false,
      "typical_extensions": [],
      "typical_containers": ["mp4", "mkv", "ts", "mov", "avi", "m2ts"]
    },
    {
      "id": "hevc",
      "names": ["H.265", "HEVC", "MPEG-H Part 2"],
      "mime": "video/HEVC",
      "lossless": false,
      "taggable": false,
      "typical_extensions": ["hevc"],
      "typical_containers": ["mp4", "mkv", "ts", "mov", "m2ts"]
    },
    // ... vp9, av1, mpeg2, etc.
  ]
}
```

**Implementation in Rust:**

- `CodecRegistry` struct in `mm-core::codec_registry`
- Embedded via `include_str!("../../../config/codecs.json5")` — identical pattern to `filetype_registry`
- `LazyLock<CodecRegistryData>` singleton
- Public API: `audio_codec_by_id(id) -> Option<&AudioCodec>`, `taggable_by_id(id) -> bool`
- Does **not** load a user override — codecs are an internal dev concern only
- At `meedya debug`, if the file's codec is in the registry and `taggable: false`, surface the message

**Acceptance criteria:**

- `cargo test -p mm-core -- codec` passes
- `meedya debug file.ac3` shows codec info from the registry
- `CodecRegistry::audio_codec_by_id("ac3").unwrap().taggable == false`

---

#### #152 — Remove End-User JSON File Override for Filetypes/Tags

**Background:**

`filetype_registry.rs` currently loads `~/.config/meedyamanager/filetypes.json5` at startup as a user override (replacing the entire built-in registry). Similarly, the tags registry may have a comparable override path.

This was a quick shortcut that is now being superseded by the proper UI-driven custom type/tag system. It has several problems:

- Exposes users to a technical JSON editing workflow
- A single typo in the JSON5 silently falls back to built-in defaults (confusing)
- The override replaces the entire registry (all-or-nothing), making it hard to add just one type
- The file path is undocumented except in code comments — users have to know to look there

**Scope:**

1. Delete `load_user_override()` from `filetype_registry.rs`
2. Delete the `if let Some(user_json5) = load_user_override()` block from the `REGISTRY` initialiser — the `LazyLock` now always loads from `DEFAULT_JSON5`
3. Apply the same change to `tags.rs` / `tag_registry.rs` if it has an equivalent override mechanism
4. Remove any references to `~/.config/meedyamanager/filetypes.json5` from docs, help files, and comments
5. Dev/CI override: add support for `MEEDYA_FILETYPES_OVERRIDE` environment variable (reads a path to a JSON5 file) for developer local testing only — this is not documented in user-facing help
6. Update `help/custom-filetypes.md` and `help/custom-tags.md` to remove any mention of editing JSON files and instead direct users to the Settings UI
7. Add a migration note to `docs/CHANGELOG.md`: if any user had a `~/.config/meedyamanager/filetypes.json5` file, MeedyaManager will now ignore it; users should recreate their custom types via Settings > File Types

**Acceptance criteria:**

- No `load_user_override` function exists in the codebase
- `MEEDYA_FILETYPES_OVERRIDE=/path/to/test.json5 cargo test` works for devs
- User-facing help mentions Settings UI, not JSON file editing
- Existing built-in filetypes still load correctly

---

#### #153 — UI: Custom Filetype Management Panel

**Background:**

With the JSON file override removed (#152), users need a proper UI to add custom file type definitions. This is the Settings > File Types panel.

**Data model (`settings.json5`):**

```json5
custom_filetypes: [
  {
    ext: "tak",
    name: "Tom's Lossless Audio Kompressor (TAK)",
    mime: "audio/x-tak",
    media_group: "Audio",        // "Audio" | "Video" | "Image" | "Document"
    lossless: true,
    taggable: false,             // user can declare this
    enabled: true
  }
]
```

User-defined entries are merged with the built-in registry at startup. On conflict (same extension), the user's entry takes precedence. This allows users to override a built-in classification (e.g. change `m4a` from `lossless: false` to `lossless: true` if they only use ALAC).

**UI per platform:**

- **Settings > File Types** tab/section
- Table/list view: Extension | Name | Media Group | Lossless | Taggable | Enabled
- "Add" button → form sheet/dialog with validated inputs (see #155)
- "Edit" button on each row → same form pre-filled
- "Delete" button with confirmation dialog
- "Enable/Disable" toggle per row (sets `enabled: false` in settings rather than deleting)
- Cannot delete built-in types from this panel — they can only be overridden or disabled
- "Reset to defaults" button clears all user-defined overrides (with confirmation)

**GTK4 (Linux):** `adw::PreferencesPage` with `adw::PreferencesGroup` + custom row widgets; use `adw::NavigationPage` for the add/edit form
**macOS (SwiftUI):** `List` + `NavigationLink` for form; `@State` bindings to draft values; `Form` with `TextField`, `Picker`, `Toggle`
**Windows (WinUI 3):** `ListView` + `ContentDialog` for the add/edit form; `TextBox`, `ComboBox`, `ToggleSwitch`

**CLI equivalent (`meedya config filetype`):**

```bash
meedya config filetype add --ext tak --name "TAK" --mime "audio/x-tak" --group Audio --lossless
meedya config filetype list
meedya config filetype remove --ext tak
meedya config filetype enable --ext tak
meedya config filetype disable --ext tak
```

**Acceptance criteria:**

- Adding a custom type via UI causes MeedyaManager to recognise and classify files with that extension
- Editing a built-in type via UI overrides it (user wins) without touching the built-in JSON5
- Removing an override restores the built-in behaviour
- All changes persist across restarts
- Invalid input is rejected before saving (see #155)

---

#### #154 — UI: Custom Tag/Metadata Field Management Panel

**Background:**

With the JSON file override removed, users need a UI to define custom metadata fields. These extend the base tag registry without editing any files.

**Data model (`settings.json5`):**

```json5
custom_tags: [
  {
    id: "remaster_year",          // Internal identifier (snake_case, alphanumeric + _)
    display_name: "Remaster Year",
    data_type: "string",          // "string" | "number" | "date" | "bool"
    id3v2_frame: "TXXX:REMASTER_YEAR",   // ID3v2 mapping (TXXX with description)
    vorbis_field: "REMASTER_YEAR",        // Vorbis Comment field name
    mp4_atom: "----:com.mwbm:REMASTER_YEAR", // iTunes MP4 custom atom
    ape_key: "Remaster Year",             // APEv2 key
    enabled: true
  }
]
```

User-defined tags supplement the built-in tag registry. On ID conflict (same `id`), user definition wins.

**UI per platform:**

- **Settings > Tags** tab/section
- Table/list: Tag ID | Display Name | Data Type | Enabled
- "Add" button → form sheet with validated inputs (see #155)
- "Edit" / "Delete" / "Enable/Disable" per row
- Cannot delete built-in tags from this panel
- Expandable detail view showing per-format mappings (ID3v2, Vorbis, MP4, APEv2)

**Template integration:**

Custom tags are immediately usable in rename templates as `<Custom:ID>` or `<Custom:Display Name>`:

```text
<Artist>/<Album>/$If(<Custom:Remaster Year>, <Title> [<Custom:Remaster Year>], <Title>).<Ext>
```

**CLI equivalent (`meedya config tag`):**

```bash
meedya config tag add --id remaster_year --name "Remaster Year" --type string
meedya config tag list
meedya config tag remove --id remaster_year
```

**Acceptance criteria:**

- Custom tag appears in `meedya rule list-tags` output
- `meedya rule test --template "<Custom:Remaster Year>" file.mp3` resolves correctly
- `meedya edit file.mp3 --tag "Custom:Remaster Year=2024"` writes the tag using the correct per-format mapping
- Settings persist across restarts

---

#### #155 — Input Validation and Sanitisation for Custom Filetype/Tag UI

**Background:**

All text fields in the custom filetype (#153) and custom tag (#154) UI panels accept user input. Without validation:

- Typos could produce invalid extensions, MIME types, or tag names that break file processing
- Injection attacks (though low-risk in a local app) could corrupt `settings.json5`
- Malformed regex patterns in rename templates could panic the rule engine

**Validation rules (applied before any save is permitted):**

| Field | Validation | Error message |
| ----- | ---------- | ------------- |
| Extension | `^[a-z0-9]{1,12}$` — lowercase alphanumeric, 1–12 chars, no dot, no spaces | "Extension must be 1–12 lowercase letters/digits, e.g. 'tak'" |
| MIME type | `^[a-z][a-z0-9!#$&\-^_]{0,63}/[a-z0-9!#$&\-^_.+]{1,64}(;.*)?$` (RFC 2045 subset) | "Enter a valid MIME type, e.g. 'audio/x-tak'" |
| Tag ID | `^[a-z][a-z0-9_]{0,63}$` — snake_case, 1–64 chars | "Tag ID must start with a letter and contain only lowercase letters, digits, and underscores" |
| Display name | Max 64 chars; strip HTML tags and control chars; no null bytes | "Name must be 1–64 characters" |
| Per-format mappings (ID3v2 frame, etc.) | Optional; if present, basic format check (e.g. ID3v2 must be 4 chars TXXX or TTTT format) | "Enter a valid ID3v2 frame ID (4 uppercase letters, e.g. TXXX)" |
| Media group (filetype) | Enum picker — no free text | N/A (dropdown) |
| Data type (tag) | Enum picker — no free text | N/A (dropdown) |

**Sanitisation (applied regardless of validation):**

- Trim leading/trailing whitespace from all string fields
- Normalise extension to lowercase
- Replace any null bytes with empty string
- Limit all string fields to their maximum length (truncate if necessary)
- Reject any input containing `../`, `\\`, or control characters outside normal text

**Implementation:**

- Extract validation into a reusable Rust module `mm-core::validation::CustomTypeValidator` so that:
  - The CLI (`meedya config filetype add`) uses it
  - The GTK4 UI uses it via FFI
  - Test coverage can be thorough and shared
- macOS/Windows UIs call the FFI validation function and surface errors in their native error presentation (SwiftUI `.alert`, WinUI 3 `InfoBar`)
- All validation errors are shown *inline* before the user can tap Save — the Save button is disabled until all fields pass validation
- Validation runs on every keystroke (with debounce) in the UI, not just on save

**Acceptance criteria:**

- Extension `"TAK"` is rejected (uppercase); `".tak"` is rejected (leading dot); `"ta k"` is rejected (space)
- MIME type `"audio/x-tak"` is accepted; `"audio"` is rejected; `"; rm -rf /"` is rejected
- Tag ID `"remaster_year"` is accepted; `"2bad"` is rejected (starts with digit); `"x".repeat(65)` is rejected
- Display name `"<script>alert(1)</script>"` is stored as `"alert(1)"` (HTML stripped)
- Save button is disabled until all fields pass; once all pass, Save is enabled
- FFI function `mm_validate_custom_filetype(...)` exists and has ≥10 unit tests

---

## GitHub Milestones

Create the following milestones on GitHub (Settings → Milestones):

| Milestone | Due Date | Description |
| --------- | -------- | ----------- |
| M0 — Repository Setup | — | Archive Python, create Cargo workspace, scaffold all platforms |
| M1 — Core Engine | — | mm-core: config, classify, metadata, watcher, renamer |
| M2 — Rule Engine | — | Lexer, parser, tag registry, template functions, evaluator |
| M3 — CLI | — | 8 commands: scan, debug, edit, rule, watch, lookup, config, report-bug |
| M4 — FFI & Shells | — | UniFFI, cbindgen, GTK4/SwiftUI/WinUI 3 shells |
| M5 — Providers | — | 19 metadata lookup providers, credentials, rate limiting |
| M6 — Full Native UI | — | Lookup panel, rule builder, cover art, DnD, settings save |
| M7 — Cloud Storage | — | OneDrive, Google Drive, Dropbox, MEGA stub, iCloud stub |
| M8 — Packaging | — | mm-update, Linux/macOS/Windows packaging, update notifications |
| M9 — Database Export | — | mm-export: 5 backends, SchemaBuilder, meedya export CLI |
| M10 — Media Server | — | axum server, JWT auth, streaming, REST API, v1.0.0 release |

---

## GitHub Project Board — MeedyaManager v0.x

Create a **GitHub Projects v2** board with the following columns/views:

### Views

1. **Board** — Kanban with columns: Backlog / In Progress / Review / Done
2. **Roadmap** — Timeline view grouped by milestone
3. **By Milestone** — Table view filtered by milestone field
4. **Issues** — All issues flat table
5. **Active** — Filtered to open issues only

### Custom Fields

| Field | Type | Values |
| ----- | ---- | ------ |
| Milestone | Single select | M0 through M10 |
| Platform | Multi-select | All / Rust / macOS / Windows / Linux |
| Priority | Single select | Critical / High / Medium / Low |
| Type | Single select | Feature / Bug / Docs / CI/CD / Refactor |

---

> *Last updated: 2026-03-06 (v1.2.0 complete — #142, #143, #144, #145, #147 closed; #146, #148, #149 remain open; v1.3.0 issues #150–#155 added)*
