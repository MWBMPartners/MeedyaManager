# MeedyaManager — GitHub Issues Register

> **(C) 2025-2026 MWBM Partners Ltd**
>
> **This file is auto-generated from the live GitHub tracker — do not hand-edit it.**
> A previous hand-maintained version of this register drifted badly from reality (119 of
> 137 titles wrong, 56 issues missing, 6 phantom issue numbers) because it was written from
> a planned numbering scheme rather than from GitHub itself. Regenerate instead of patching:
>
> ```bash
> gh issue list --state all --limit 500 \
>   --json number,title,state,labels,milestone \
>   > /tmp/issues_raw.json
> # then re-run the grouping script that produced this file
> ```
>
> **Snapshot taken:** 2026-09-03 — **202 issues total** (72 open, 130 closed).
> Grouped by milestone (GitHub milestone field, where set), then by issue number within each
> group. Issues with no milestone assigned on GitHub — mostly pre-M0 mirrors of MeedyaDL
> issues, MeedyaSuite-core migration work, CI/audit fixes, and the 2026-09 reconciliation
> issues #193–#215 — are listed under **No milestone** at the end, in numeric order.

---

## M0 — Repository Setup & Scaffolding — 21 issues (1 open / 20 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #19 | 🔲 Open | `milestone:M0`, `type:chore`, `priority:P0` | M0: Archive Python codebase (tag v1.5-M6-python-final + archive branch) |
| #20 | ✅ Closed | `milestone:M0`, `type:chore`, `priority:P0` | M0: Delete all Python source files from main branch |
| #21 | ✅ Closed | `milestone:M0`, `platform:core`, `type:feature`, `priority:P0` | M0: Initialize Cargo workspace with stub crates |
| #22 | ✅ Closed | `milestone:M0`, `platform:macos`, `type:feature`, `priority:P0` | M0: Scaffold macOS Xcode project (empty SwiftUI app) |
| #23 | ✅ Closed | `milestone:M0`, `platform:windows`, `type:feature`, `priority:P0` | M0: Scaffold Windows Visual Studio solution (empty WinUI 3 app) |
| #24 | ✅ Closed | `milestone:M0`, `platform:core`, `type:chore`, `priority:P1` | M0: Set up Rust toolchain config (.rustfmt.toml, clippy.toml, deny.toml, rust-toolchain.toml) |
| #25 | ✅ Closed | `milestone:M0`, `type:ci`, `priority:P1` | M0: Create CI workflow — ci-rust.yml (Rust core test matrix) |
| #26 | ✅ Closed | `milestone:M0`, `platform:macos`, `type:ci`, `priority:P1` | M0: Create CI workflow — ci-macos.yml (SwiftUI build + test) |
| #27 | ✅ Closed | `milestone:M0`, `platform:windows`, `type:ci`, `priority:P1` | M0: Create CI workflow — ci-windows.yml (WinUI 3 build + test) |
| #28 | ✅ Closed | `milestone:M0`, `platform:linux`, `type:ci`, `priority:P1` | M0: Create CI workflow — ci-linux.yml (GTK4 build + test) |
| #29 | ✅ Closed | `milestone:M0`, `type:ci`, `priority:P2` | M0: Create CI workflows — release.yml, audit.yml, docs.yml |
| #30 | ✅ Closed | `milestone:M0`, `type:chore`, `priority:P0` | M0: Set up GitHub Projects v2 board with custom fields, views, and labels |
| #31 | ✅ Closed | `milestone:M0`, `type:docs`, `priority:P1` | M0: Update all documentation (README, Project_Plan, PROJECT_STATUS, CLAUDE.md, ROADMAP) |
| #32 | ✅ Closed | `milestone:M0`, `type:ci`, `platform:all` | Create GitHub Actions workflow for automated version bumping |
| #33 | ✅ Closed | `milestone:M0`, `type:ci`, `platform:all` | Add version-sync CI check to ci-rust.yml |
| #34 | ✅ Closed | `milestone:M0`, `platform:macos`, `type:bug` | Add version fields to macOS Info.plist |
| #35 | ✅ Closed | `milestone:M0`, `platform:windows`, `type:bug` | Fix stale Windows MSIX version in Package.appxmanifest |
| #36 | ✅ Closed | `milestone:M0`, `type:ci`, `platform:all` | Enhance release workflow with real build steps and checksums |
| #37 | ✅ Closed | `milestone:M0`, `type:feature`, `platform:all` | Add version/release commands to justfile |
| #38 | ✅ Closed | `milestone:M0`, `type:docs` | Create developer notes documentation (docs/Dev_Notes.md) |
| #39 | ✅ Closed | `milestone:M0`, `type:docs` | Create GitHub Wiki pages for versioning, releases, and CI/CD |

---

## Milestone M2 – Rule Engine — 4 issues (0 open / 4 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #5 | ✅ Closed | `milestone:M2` | M2-UI-02: Rule Builder Component |
| #6 | ✅ Closed | `milestone:M2` | M2-UI-03: Rename Preview Table |
| #7 | ✅ Closed | `milestone:M2` | M2-UI-04: Drag-and-Drop File Import |
| #8 | ✅ Closed | `milestone:M2` | M2-TEST-01: Unit Tests for Wizard CLI |

---

## M1 — Core Engine (Rust) — 12 issues (2 open / 10 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #40 | ✅ Closed | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Configuration loading (JSON5 + .env via serde + dotenvy) |
| #41 | ✅ Closed | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Media classification engine (4-level hierarchy) |
| #42 | ✅ Closed | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Metadata extraction via lofty crate |
| #43 | ✅ Closed | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Metadata tag writing via lofty |
| #44 | ✅ Closed | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Multi-value field handling (semicolon-delimited parsing) |
| #45 | 🔲 Open | `milestone:M1`, `platform:core`, `type:feature` | mm-core: File system watcher (notify crate + polling fallback) |
| #46 | ✅ Closed | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Rename simulator (dry-run path computation) |
| #47 | ✅ Closed | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Filename character sanitizer (configurable replacement mappings) |
| #48 | ✅ Closed | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Companion file detector and grouper |
| #49 | ✅ Closed | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Application state manager + single-instance lock file |
| #50 | 🔲 Open | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Structured logging (tracing + PII redaction + daily rotation) |
| #51 | ✅ Closed | `milestone:M1`, `platform:core`, `type:feature` | mm-core: Startup health checks + unified error types (thiserror) |

---

## M3 — CLI — 11 issues (0 open / 11 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #52 | ✅ Closed | `enhancement` | M3: CLI shared infrastructure (output.rs + context.rs) |
| #53 | ✅ Closed | `enhancement` | M3: Restructure main.rs (Commands enum, global flags, dispatch) |
| #54 | ✅ Closed | `enhancement` | M3: `meedya debug` — single-file metadata inspector |
| #55 | ✅ Closed | `enhancement` | M3: `meedya rule` — template validation and tag listing |
| #56 | ✅ Closed | `enhancement` | M3: `meedya config` — configuration management |
| #57 | ✅ Closed | `enhancement` | M3: `meedya scan` — directory scan with rename preview |
| #58 | ✅ Closed | `enhancement` | M3: `meedya edit` — metadata editor |
| #59 | ✅ Closed | `enhancement` | M3: `meedya watch` — foreground file watcher |
| #60 | ✅ Closed | `enhancement` | M3: `meedya lookup` — provider search (stub) |
| #61 | ✅ Closed | `enhancement` | M3: `meedya report-bug` — system info and log collection |
| #62 | ✅ Closed | `enhancement` | M3: Documentation updates (PROJECT_STATUS, CHANGELOG) |

---

## M4 — FFI Layer & Native UI Shells — 10 issues (2 open / 8 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #63 | ✅ Closed | `enhancement` | M4: UniFFI interface definitions for macOS Swift bindings |
| #64 | 🔲 Open | `enhancement` | M4: cbindgen/csbindgen C headers for Windows C# P/Invoke |
| #65 | ✅ Closed | `enhancement` | M4: Async callback bridge (Rust → native UIs) |
| #66 | 🔲 Open | `enhancement` | M4: macOS SwiftUI shell (tab navigation, UniFFI integration, Liquid Glass) |
| #67 | ✅ Closed | `enhancement` | M4: macOS basic panels (PreviewPanel + SettingsView) |
| #68 | ✅ Closed | `enhancement` | M4: Windows WinUI 3 shell (NavigationView, P/Invoke, Mica) |
| #69 | ✅ Closed | `enhancement` | M4: Windows basic panels (PreviewPanel + SettingsPage) |
| #70 | ✅ Closed | `enhancement` | M4: Linux GTK4/Libadwaita shell (AdwTabView, Adwaita theming) |
| #71 | ✅ Closed | `enhancement` | M4: Linux basic panels (preview panel + settings dialog) |
| #72 | ✅ Closed | `documentation` | M4: Documentation updates (PROJECT_STATUS, CHANGELOG, ROADMAP) |

---

## M5 — Metadata Lookup Providers — 12 issues (4 open / 8 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #73 | ✅ Closed | `enhancement` | M5: BaseProvider trait + ProviderResult + Capabilities types |
| #74 | 🔲 Open | `enhancement` | M5: Provider auto-registration via inventory crate |
| #75 | ✅ Closed | `enhancement` | M5: 4-tier credential resolution (env → config → keyring → encrypted) |
| #76 | ✅ Closed | `enhancement` | M5: Token bucket rate limiter (governor crate) |
| #77 | ✅ Closed | `enhancement` | M5: Fuzzy match scoring (weighted: title 35%, artist 30%, album 20%, duration 15%) |
| #78 | 🔲 Open | `enhancement` | M5: Cover art management (static JPEG/PNG + animated MP4) |
| #79 | ✅ Closed | `enhancement` | M5: Music providers — Apple Music, Spotify, MusicBrainz, Deezer, YouTube Music |
| #80 | 🔲 Open | `enhancement` | M5: Music providers — Amazon Music, Pandora, Tidal, Shazam, iHeart |
| #81 | ✅ Closed | `enhancement` | M5: Video providers — TMDB, TheTVDB, IMDb, Apple TV, iTunes Store |
| #82 | ✅ Closed | `enhancement` | M5: Podcast + identifier providers — Apple Podcasts, ISRC, EIDR, ISWC |
| #83 | 🔲 Open | `enhancement` | M5: CLI lookup command integration + batch mode |
| #84 | ✅ Closed | `documentation` | M5: Documentation updates (PROJECT_STATUS, CHANGELOG) |

---

## M6 — Full Native UI — 9 issues (2 open / 7 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #85 | ✅ Closed | `enhancement` | M6: Rule Builder view (syntax highlighting, tag palette, live preview) — all platforms |
| #86 | ✅ Closed | `enhancement` | M6: Metadata Editor view (tag table, cover art widget, batch editing) — all platforms |
| #87 | 🔲 Open | `enhancement` | M6: Lookup Panel view (provider checkboxes, results table, apply/batch) — all platforms |
| #88 | ✅ Closed | `enhancement` | M6: Preview Panel (full rename preview with progress) — all platforms |
| #89 | ✅ Closed | `enhancement` | M6: Drag-and-drop file import — all platforms |
| #90 | ✅ Closed | `enhancement` | M6: Accessibility compliance (VoiceOver, Narrator, Orca) |
| #91 | ✅ Closed | `enhancement` | M6: Dark/light theme toggle + system-following default |
| #92 | 🔲 Open | `enhancement` | M6: User-friendly error dialogs + config export/import UI |
| #93 | ✅ Closed | `documentation` | M6: Documentation updates (PROJECT_STATUS, CHANGELOG) |

---

## M7 — Cloud Storage Monitoring — 9 issues (8 open / 1 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #94 | 🔲 Open | `enhancement` | M7: CloudProvider trait + sync manager architecture |
| #95 | 🔲 Open | `enhancement` | M7: OneDrive provider (Personal + Business via Microsoft Graph) |
| #96 | 🔲 Open | `enhancement` | M7: Google Drive provider (Drive API v3) |
| #97 | 🔲 Open | `enhancement` | M7: Dropbox provider (API v2) |
| #98 | 🔲 Open | `enhancement` | M7: MEGA.nz provider (MEGA API) |
| #99 | 🔲 Open | `enhancement` | M7: iCloud Drive provider (macOS only, FileProvider framework) |
| #100 | 🔲 Open | `enhancement` | M7: Cloud UI tab (connection status, folder browser, sync status) — all platforms |
| #101 | 🔲 Open | `enhancement` | M7: Background sync + conflict resolution + OAuth2 token refresh |
| #102 | ✅ Closed | `documentation` | M7: Documentation updates (PROJECT_STATUS, CHANGELOG) |

---

## M8 — Packaging & Public Release — 9 issues (5 open / 4 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #103 | ✅ Closed | `enhancement` | M8: macOS .app bundle in .dmg (code-signed + notarized) |
| #104 | 🔲 Open | `enhancement` | M8: macOS App Store submission |
| #105 | 🔲 Open | `enhancement` | M8: Windows MSIX package (code-signed) |
| #106 | 🔲 Open | `enhancement` | M8: Microsoft Store submission |
| #107 | ✅ Closed | `enhancement` | M8: Linux packages (Flatpak, Snap, AppImage, .deb) |
| #108 | 🔲 Open | `enhancement` | M8: Auto-updater integration (per-platform) |
| #109 | ✅ Closed | `enhancement` | M8: Release pipeline (SHA256 checksums + release notes auto-generation) |
| #110 | 🔲 Open | `enhancement` | M8: First public beta release |
| #111 | ✅ Closed | `documentation` | M8: Documentation updates (PROJECT_STATUS, CHANGELOG) |

---

## M9 — Database Export — 8 issues (6 open / 2 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #112 | ✅ Closed | `enhancement` | M9: DbExporter trait + shared table schema |
| #113 | 🔲 Open | `enhancement` | M9: MySQL export via sqlx |
| #114 | 🔲 Open | `enhancement` | M9: MariaDB export via sqlx |
| #115 | 🔲 Open | `enhancement` | M9: SQL Server export via tiberius |
| #116 | 🔲 Open | `enhancement` | M9: SQLite export via sqlx |
| #117 | 🔲 Open | `enhancement` | M9: PostgreSQL export via sqlx |
| #118 | 🔲 Open | `enhancement` | M9: Export UI tab + CLI export command |
| #119 | ✅ Closed | `documentation` | M9: Documentation updates (PROJECT_STATUS, CHANGELOG) |

---

## M10 — Secure Media Server — 8 issues (7 open / 1 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #120 | 🔲 Open | `enhancement` | M10: axum HTTP server with REST API scaffold |
| #121 | 🔲 Open | `enhancement` | M10: JWT authentication + bcrypt password hashing |
| #122 | 🔲 Open | `enhancement` | M10: Media streaming with range request support |
| #123 | 🔲 Open | `enhancement` | M10: Per-user access control + library visibility |
| #124 | 🔲 Open | `enhancement` | M10: Web frontend (embedded static files) |
| #125 | 🔲 Open | `enhancement` | M10: TLS via rustls + CLI serve command |
| #126 | 🔲 Open | `enhancement` | M10: Media file copy/export to server with database references |
| #127 | ✅ Closed | `documentation` | M10: Documentation updates (PROJECT_STATUS, CHANGELOG) |

---

## No milestone — 89 issues (35 open / 54 closed)

| # | State | Labels | Title |
| - | ----- | ------ | ----- |
| #1 | ✅ Closed | `milestone:M2` | M2-CLI-01: Interactive CLI Rename Wizard |
| #2 | ✅ Closed | `milestone:M2` | M2-CLI-02: Nested IF/AND/OR Logic Support |
| #3 | ✅ Closed | `milestone:M2` | M2-CLI-03: Preview + Confirm Before Rename |
| #4 | ✅ Closed | `milestone:M2` | M2-UI-01: Scaffold Cross-Platform GUI |
| #9 | ✅ Closed | `enhancement` | feat: comprehensive metadata tag reading and writing (mirror of MeedyaDL #123) |
| #10 | ✅ Closed | `enhancement` | feat: adopt tags.toml config-driven tag system (mirror of MeedyaDL #120) |
| #11 | ✅ Closed | `enhancement` | feat: comprehensive subtitle and lyrics format support (mirror of MeedyaDL) |
| #12 | ✅ Closed | `enhancement` | feat: variant-specific ISRC detection and display (mirror of MeedyaDL #121) |
| #13 | ✅ Closed | `enhancement` | feat: enhanced metadata enrichment from multiple service APIs (mirror of MeedyaDL #122) |
| #14 | ✅ Closed | `enhancement` | feat: isDownmix and isBinaural audio classification tags (mirror of MeedyaDL #119) |
| #15 | ✅ Closed | `enhancement` | feat: API field audit and metadata discovery tool (mirror of MeedyaDL #124) |
| #16 | ✅ Closed | `enhancement` | feat: lyrics format conversion chain (TTML/LRC/SRT/WebVTT/ASS) (mirror of MeedyaDL #87) |
| #17 | ✅ Closed | `enhancement` | feat: AcoustID fingerprinting and MusicBrainz integration (mirror of MeedyaDL #82, #94, #95) |
| #18 | ✅ Closed | `enhancement` | feat: accessibility (a11y) support — screen readers, colour blindness themes (mirror of MeedyaDL #125) |
| #128 | 🔲 Open | `enhancement` | feat: Test Mode, Privacy Policy, and Pre-release Safety |
| #129 | ✅ Closed | `enhancement` | chore: Add workspace lint configuration and resolve all clippy warnings |
| #130 | 🔲 Open | — | feat: bundle MediaInfo CLI as managed dependency with update checking |
| #131 | 🔲 Open | `enhancement` | Add Spatial Audio detection (MPEG-H MHM1/MHA1, Dolby Atmos, DTS:X, 360 Reality Audio, ASAF, APAC, Eclipsa) |
| #132 | ✅ Closed | — | Migrate mm-providers trait system to MeedyaSuite-core |
| #133 | ✅ Closed | — | Migrate provider infrastructure to MeedyaSuite-core |
| #134 | 🔲 Open | — | Migrate mm-core metadata I/O to MeedyaSuite-core |
| #135 | ✅ Closed | — | Add meedya-core as Cargo dependency and remove redundant crates |
| #136 | 🔲 Open | — | Contribute provider implementations upstream to MeedyaSuite-core |
| #137 | ✅ Closed | `bug` | fix: update deny.toml schema for cargo-deny 0.19.x compatibility |
| #138 | 🔲 Open | `enhancement` | Extract and package ALL FFmpeg components (ffmpeg, ffprobe, ffplay) |
| #139 | 🔲 Open | `enhancement`, `lyrics`, `library` | [FEATURE] read / display / edit LyricsFile (.lyrics) sidecars via MeedyaSuite-core |
| #140 | 🔲 Open | — | Long-term in-app updater: route through update.mwbm.io (shared MWBM-wide endpoint) |
| #141 | ✅ Closed | — | chore: standing task — post-PR dev cache cleanup (workspace + Rust global) |
| #143 | ✅ Closed | — | ci: umbrella PR Gate workflow + convert platform CIs to reusable |
| #145 | ✅ Closed | — | ci: fix 4 pre-existing CI failures surfaced by PR Gate smoke test |
| #146 | 🔲 Open | — | macos: investigate accessibilityLiveRegion API resolution failure on macos-15 runner |
| #148 | 🔲 Open | — | ci(windows): WinUI 3 XamlCompiler.exe silently fails — needs offline binlog investigation |
| #149 | ✅ Closed | — | ci(audit): deny.toml schema drift — cargo-deny v0.16+ rejects 'warn' for unmaintained |
| #151 | ✅ Closed | — | ci(audit): complete deny.toml v2 migration — [licenses].unlicensed/copyleft also removed |
| #153 | ✅ Closed | — | ci(audit): port SPDX fixes + security advisory ignores from MeedyaSuite-core branch |
| #155 | ✅ Closed | — | ci(audit): replace rustsec/audit-check action with cargo-audit CLI |
| #157 | ✅ Closed | — | ci(audit): drop redundant cargo-audit step — cargo-deny already covers RustSec DB |
| #162 | 🔲 Open | — | ci/upstream: expand meedya-metadata::CommonTag to cover MM's tag superset (blocks full #134) |
| #164 | 🔲 Open | — | Add HDR detection (HDR10, HDR10+, HLG, Dolby Vision 1/2, SL-HDR1/2/3, HDR Vivid, combinations) |
| #165 | ✅ Closed | `enhancement`, `area:dj-tools`, `area:tagging` | Music Stems tagging — custom namespace + per-component instrument tags |
| #166 | ✅ Closed | `enhancement`, `area:ai-provenance`, `area:tagging` | AI provenance tags — isAI / AIused / AIenhanced / detailAIenhance (generic + MeedyaSuite namespace) |
| #167 | ✅ Closed | `for consideration`, `area:tagging` | OneTagger inspiration — umbrella for DJ-tagging features (links to sub-issues) |
| #168 | ✅ Closed | `for consideration`, `area:dj-tools` | Beatport metadata provider (DJ-essential — labels, BPM, key, genre, release date) |
| #169 | ✅ Closed | `for consideration`, `area:dj-tools` | Discogs metadata provider (catalogue + vinyl-focused; classics + rare releases) |
| #170 | ✅ Closed | `for consideration` | Bandcamp metadata provider (independent artists, niche labels) |
| #171 | ✅ Closed | `for consideration`, `area:dj-tools` | Additional DJ marketplace providers (Beatsource / Traxsource / JunoDownload) |
| #172 | ✅ Closed | `for consideration`, `area:dj-tools` | Spotify Audio Features enrichment — extend SpotifyProvider with BPM/key/energy/danceability via ISRC match |
| #173 | ✅ Closed | `for consideration`, `area:dj-tools` | Audio analysis: local BPM detection pipeline (offline, no Spotify dependency) |
| #174 | ✅ Closed | `for consideration`, `area:dj-tools` | Audio analysis: local key detection + Camelot wheel notation (offline) |
| #175 | ✅ Closed | `for consideration`, `area:dj-tools` | Audio analysis: local energy / mood / danceability classification (offline) |
| #176 | ✅ Closed | `for consideration`, `area:dj-tools` | Quick-tag keyboard mode — rapid batch tagging UI |
| #177 | ✅ Closed | `for consideration`, `area:tagging` | Batch tag editor — regex find/replace + bulk-set across selection |
| #178 | ✅ Closed | `for consideration`, `area:tagging` | Tag-driven file rename templating (mid-batch, with preview + undo) |
| #179 | ✅ Closed | `for consideration` | EBU R128 / BS.1770 LUFS loudness measurement (broadcast standard, in addition to ReplayGain) |
| #180 | ✅ Closed | `for consideration` | Watch folders / auto-organize daemon (service-mode auto-processing) |
| #181 | ✅ Closed | `for consideration` | Smart playlists — saved rule-engine queries as named, auto-refreshing collections |
| #182 | ✅ Closed | `for consideration` | Duplicate detection via AcoustID fingerprinting (cross-format, cross-bitrate) |
| #183 | ✅ Closed | `for consideration`, `area:dj-tools` | DJ crate export — write to Serato / Traktor / rekordbox DB formats |
| #184 | ✅ Closed | `for consideration`, `area:dj-tools` | Cue point management — read/write/sync cue points across DJ formats |
| #185 | ✅ Closed | `for consideration`, `area:dj-tools` | Beat grid extraction / sync (alongside BPM) for tight DJ mixing |
| #186 | ✅ Closed | `for consideration`, `area:tagging` | Version / mix-variant tagging (Original Mix, Extended Mix, Remix by X, Radio Edit, etc.) |
| #187 | ✅ Closed | `for consideration` | Cover art workflow — extraction, scaling, embedding, conflict resolution, multi-image |
| #188 | ✅ Closed | `for consideration` | Format conversion pipeline (FLAC↔ALAC, lossless→AAC/MP3 for DJ-compat, etc.) |
| #189 | ✅ Closed | `for consideration`, `area:tagging` | Provider conflict resolution UI (when 3 providers give 3 different tag values) |
| #190 | ✅ Closed | `for consideration` | Catalog number / barcode / matrix-runout structured tagging (collector workflow) |
| #191 | ✅ Closed | `for consideration`, `area:dj-tools` | DJ ecosystem support — Serato/Traktor/rekordbox interop + DJ providers + analysis (umbrella) |
| #193 | 🔲 Open | `type:ci` | ci: add actionlint workflow-lint CI |
| #194 | 🔲 Open | `enhancement` | macOS: group the app under /Applications/MeedyaSuite/ (org-wide convention) |
| #195 | 🔲 Open | — | Adopt the suite-wide remote feature-control contract (Rust client) |
| #196 | 🔲 Open | — | Consume core identifier-types registry · converge mm_* META keys (mm_iswc → iswc) · add ISWC file-tag |
| #197 | 🔲 Open | `type:bug`, `type:ci` | CI clippy gate spontaneously breaks: floating `stable` toolchain + `nursery` lints at `-D warnings` (manual_assert_eq now red on main) |
| #198 | 🔲 Open | — | MusicBrainz: harden search integration ahead of the 2026-11-30 breaking changes |
| #199 | 🔲 Open | `platform:linux`, `type:bug` | mm-gtk cannot build standalone: workspace Cargo.toml has no `exclude` key |
| #200 | 🔲 Open | `type:ci` | mm-cloud: ~40 pre-existing clippy errors block a green `cargo clippy --workspace -- -D warnings |
| #201 | 🔲 Open | `platform:cli`, `type:bug`, `priority:P0` | `meedya scan --execute` silently overwrites on duplicate destinations and flattens folder templates |
| #202 | 🔲 Open | `type:ci`, `platform:all` | Release pipeline: make `release.yml` able to produce a first draft pre-release (files filter, macos runner, appimagetool, Dev_Notes path) |
| #203 | 🔲 Open | `type:bug`, `type:ci` | Security Audit red on every weekly run since 2026-07-06 (RUSTSEC-2026-0190/0244/0258/0194/0195) |
| #204 | 🔲 Open | `type:ci` | `alpha`/`beta` have no CI: extend `pr-gate.yml` and `ci-*.yml` triggers beyond `main` |
| #205 | 🔲 Open | `type:bug`, `platform:all` | Stubs must not report success: `export`/`serve` exit non-zero, UI Server/Export/Cloud tabs marked preview-only, `--help` strings |
| #206 | 🔲 Open | `platform:cli`, `type:bug` | mm-cli correctness batch: `-r` cannot be disabled, extensionless files get a trailing dot, `edit --set` reports success for unmapped keys, `export` DSN detection |
| #207 | 🔲 Open | `type:chore`, `priority:P1` | Add the `LICENSE` file (GPL-2.0-or-later declared, no licence text tracked) |
| #208 | 🔲 Open | `platform:windows`, `type:bug` | Windows P/Invoke: UTF-8 marshalling (`LPStr`/`PtrToStringAnsi`) and three undeclared exports |
| #209 | 🔲 Open | `api-key-handling`, `type:bug` | Tier-4 credentials are stored as plain JSON, not the specified AES-256-GCM bundle |
| #210 | 🔲 Open | `type:bug` | TheTVDB provider sends the raw API key as a bearer token; v4 requires a `/login` JWT exchange |
| #211 | 🔲 Open | `platform:core`, `type:bug` | `config/settings.json5` and `settings.schema.json` do not match `AppConfig`; loading the shipped file yields all-defaults silently |
| #212 | 🔲 Open | `platform:core`, `type:bug` | Two config directories (`MeedyaManager/` vs `meedyamanager/`) |
| #213 | 🔲 Open | `type:chore` | Drop or feature-gate dead heavyweight deps (sqlx, tiberius, axum, tower, rustls, oauth2, reqwest in mm-cloud) until used |
| #214 | 🔲 Open | `type:chore` | Cut a real pre-release version (`1.4.0-alpha.1`) so `is_current_prerelease()` fires; retire phantom 1.3.1/1.3.2 changelog entries |
| #215 | 🔲 Open | `enhancement` | Successor issues for #9 / #11 / #12 / #14 / #16 / #17 if still wanted (or upstream MeedyaSuite-core issues) |

---

## Notes

- **Version reality check:** the current released version is `1.3.0`. Versions `1.3.1` and
  `1.3.2` referenced in older changelog drafts were **never cut** — see #214, which tracks
  cutting a real pre-release (`1.4.0-alpha.1`) instead.
- **Only one GitHub Release exists:** *"MetaMancer v1.0-M1"* (2025-06-16, pre-rename), tag
  `v1.0-M1`. The archive tag `v1.5-M6-python-final` referenced by #19 does **not** exist —
  #19 remains open for this reason.
- Issues #165–#191 (27 DJ/tagging feature issues) are correctly closed under `for
  consideration` labels per owner direction (2026-05-29): that work is tracked and developed
  directly in MeedyaSuite-core, with MM-side consumer issues to be refiled once the upstream
  is ready to integrate against. This is a deliberate closure, not an oversight.
- Milestones M7 (Cloud), M9 (Database Export) and M10 (Secure Media Server) are
  **architectural scaffolding only** — no real network calls, no database connections, and
  `mm-server` never builds an axum router. Most of their per-provider/per-backend issues are
  correctly open; do not read the low closed-count-per-milestone rows above as "barely
  started" — the scaffolding commits landed under the umbrella issues (`#112`, `#120`, etc.)
  which the tracker autoclosed alongside their doc-update issues.
