# MeedyaManager — Project Status

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Cross-platform media manager and auto-organizer — Rust core + native UIs

---

## Quick Status

| Item | Status |
| ---- | ------ |
| **Current Version** | `v1.3.0` (`Cargo.toml` `[workspace.package].version` — the single source of truth) |
| **Real, working milestones** | M0-M6 — repository scaffold, core engine, rule engine, CLI, FFI/UI shells, metadata providers, full native UI |
| **Scaffold-only milestones** | M7 (Cloud Storage), M9 (Database Export), M10 (Secure Media Server) — architectural scaffolding: real types and tests, but no real network calls, no database connections, and no HTTP server ever starts. See "Milestone Progress" below for evidence and the GitHub issues these were reopened as |
| **Public releases** | **None.** The only GitHub release is *"MetaMancer v1.0-M1"* (pre-rename project name), dated 2025-06-16, pre-release. The only git tag is `v1.0-M1` |
| **Python v1.x archive** | The `v1.5-M6-python-final` tag referenced elsewhere in this repo's history **does not exist** — treat any reference to it as stale |
| **GitHub issues** | 202 total, 57 open (as of the 2026-09-03 audit/reconciliation sweep — see `.claude/HANDOFF.md`) |
| **Build Status** | ![CI](https://github.com/MWBMPartners/MeedyaManager/actions/workflows/ci-rust.yml/badge.svg) |

> Versions **1.3.1** and **1.3.2** appear in `docs/changelog.md` and in older drafts of this file, but
> neither was ever set in `Cargo.toml` — the version has only ever been bumped as far as `1.3.0`.
> Treat any "v1.3.1" / "v1.3.2" label elsewhere as a planning label, not a shipped version.

---

## Milestone Progress

### M0 — Repository Setup & Scaffolding *(Complete)*

> Started: 2026-03-04 | Version: `v0.1.0`

**Progress: 100%** | Issues: #19-#31, #32-#39 (all closed)

| Deliverable | Status | Notes |
| ----------- | ------ | ----- |
| Archive Python v1.x codebase | Done | Python source removed from the tree |
| Delete Python source tree | Done | All `.py` files removed |
| Cargo workspace — 9 crate directories, 8 workspace members | Done | `mm-core`, `mm-providers`, `mm-cloud`, `mm-export`, `mm-server`, `mm-cli`, `mm-ffi`, `mm-update` are workspace members; `mm-gtk` is a 9th crate directory **excluded** from `[workspace] members` (needs Linux-only `gettextrs`) |
| macOS SwiftUI scaffold | Done | `macos/` with `Package.swift` (Swift Package Manager — there is no `.xcodeproj`) |
| Windows WinUI 3 scaffold | Done | `windows/` with `.sln`/`.csproj` |
| Rust toolchain configuration | Done | `.rustfmt.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml` |
| CI/CD workflows (9 workflows) | Done | `pr-gate`, `ci-rust`, `ci-macos`, `ci-windows`, `ci-linux`, `version-bump`, `release`, `audit`, `docs` |
| GitHub Projects v2 board | Done | 11 milestones, custom fields |
| Documentation update | Done | All `.md` files rewritten |
| Automated version management | Done | `version-bump.yml` workflow, version-sync CI check (covers `Cargo.toml` → macOS `Info.plist` + Windows `Package.appxmanifest` only — the Linux and WinGet packaging manifests are **not** covered and are currently out of sync, see M8 below) |
| Release build pipeline | Done | `release.yml` builds 5 platform artifacts + checksums + draft release, but no tag has ever been pushed against it |
| GitHub Wiki | Done | Version Management, Release Process, CI/CD Pipelines pages |
| Developer notes | Done | `Dev_Notes.md` |

---

### M1 — Core Engine *(Complete)*

> Started: 2026-03-04 | Completed: 2026-03-05 | Version: `v0.2.0`

**Progress: 100%** | Issues: #40-#51

| Deliverable | Status |
| ----------- | ------ |
| Error types (`thiserror`) | Done |
| Config module (JSON5 + .env + env overrides) | Done |
| Media classification (4-level: Group/Format/Class/Quality) | Done |
| Metadata extraction & writing (`lofty`) | Done |
| File watcher (`notify` + debounce + filtering) | Done |
| Rename simulator + filename sanitizer | Done |
| Companion file detector (subtitles, lyrics, art, cue) | Done |
| State manager + single-instance lock file | Done |
| Structured logging (tracing + PII redaction) | Done |
| Health checks (config, folders, disk, writable) | Done |

> The original "217 tests" figure for M1 and "182 tests" for M2 are historical snapshots and no
> longer tracked separately. `mm-core` (which holds both M1's engine modules and M2's rule engine)
> now totals **512 test functions** — see the Architecture Health table below for current,
> verified per-crate counts.

---

### M2 — Rule Engine *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.3.0`

**Progress: 100%**

| Deliverable | Status |
| ----------- | ------ |
| Lexer (tokenizer: tags, functions, literals, legacy detection) | Done |
| Parser (recursive descent, AST, 50-level depth guard) | Done |
| Tag registry (40+ bidirectional mappings, virtual tags) | Done |
| Template functions — **24 implemented**: `if`, `and`, `or`, `not`, `isnull`, `contains`, `replace`, `upper`, `lower`, `left`, `right`, `mid`, `trim`, `split`, `pad`, `date`, `format`, `count`, `sort`, `ismatch`, `lookup`, `mediaclass`, `mediagroup`, `firstvalue` (`crates/mm-core/src/rule_engine/functions.rs:120-153`) | Done |
| Evaluator (EvalContext, multi-value, missing tag modes) | Done |
| Rule system (conditions, operators, priority ordering, apply_rules) | Done |
| Renamer integration (`simulate_rename_with_rules`) | Done |
| Config extension (`rules` + `missing_tag_mode` in RenameConfig) | Done |

> `help/rule-syntax.md` currently documents 4 functions that do not exist (`$RxReplace`,
> `$RSplit`, `$First`, `$Group`) and omits 8 real ones — see the `help/` documentation task for
> the fix; the 24 above are the verified, real set.

---

### M3 — CLI *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.4.0`

**Progress: 100%** | Issues: #52-#62 (all closed) | `mm-cli` currently has **73 tests**

| Deliverable | Status |
| ----------- | ------ |
| Output infrastructure (`output.rs`) | Done |
| CLI context (`context.rs`) | Done |
| `main.rs` restructure (Commands enum, global flags, dispatch) | Done |
| `meedya debug` — single-file metadata inspector | Done |
| `meedya rule` — validate, tags, test, legacy | Done |
| `meedya config` — show, path, init, export, import, test-mode | Done |
| `meedya scan` — directory scan + rename preview + execute | Done — **see caution below** |
| `meedya edit` — metadata write (`--set`, `--remove`, `--cover`, `--remove-cover`, `--dry-run`) | Done |
| `meedya watch` — foreground watcher with event logging | Done (only logs unless `--organize` is passed) |
| `meedya lookup` — provider search | **Status: not yet implemented** — prints "not yet available. Provider support is coming in M5" and exits (`crates/mm-cli/src/commands/lookup.rs:79-85`), unchanged since M3 despite M5 having since landed |
| `meedya report-bug` — system info + log collection | Done |
| `meedya serve` / `meedya export` / `meedya service` | Added after M3, see M9/M10 below |

> **Known data-loss risk (#201, highest severity open finding):** `meedya scan --execute` computes
> its "would this overwrite an existing file" flag at preview time only and never tracks
> duplicate destinations created within the same batch
> (`crates/mm-core/src/renamer/mod.rs:349` vs `crates/mm-cli/src/commands/scan.rs:173`), so two
> files renamed to the same destination in one run can silently overwrite one another via
> `std::fs::rename`. `mm-core`'s own `simulate_rename` gets this right; the CLI does not use it.
> Reproduced empirically. Do not run `--execute` unattended until #201 is resolved.

---

### M4 — FFI Layer & Native UI Shells *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.5.0`

**Progress: 100%** | Issues: #63-#72 | `mm-ffi` currently has **23 tests**; `mm-gtk` (Rust GTK4 UI, not a workspace member) has **67 tests**

| Deliverable | Status | Notes |
| ----------- | ------ | ----- |
| `mm-ffi` crate — UniFFI proc-macro scaffolding | Done | `setup_scaffolding!("mm_ffi")` |
| FFI types, callback interfaces, C API, cbindgen headers | Done | |
| `mm-gtk` crate — main window, scan/metadata/rules/settings panels | Done | Excluded from `cargo build --workspace` — build it explicitly with `cargo build -p mm-gtk --release` |
| macOS shell — `AppState`, `ScanModel`/`MetadataModel`, `MmCore.swift` P/Invoke bridge, SwiftUI views | Done | |
| Windows shell — `MmCore.cs` P/Invoke bridge, XAML pages, `MainWindow` navigation | Done | |

---

### M5 — Metadata Lookup Providers *(Complete, with reachability caveats)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.6.0`

**Progress: 100%** | Issues: #73-#84 (hardened further under #198) | `mm-providers` currently has **289 tests**

| Deliverable | Status | Notes |
| ----------- | ------ | ----- |
| `traits.rs`, `credentials.rs`, `rate_limiter.rs`, `match_scoring.rs`, `cover_art.rs`, `registry.rs` | Done | Core infrastructure is real |
| **19 registered providers** | **13 real HTTP clients + 6 disabled stubs** | See table below |
| `meedya lookup` CLI reachability | **Status: not yet implemented** | The command is still a stub (see M3) |
| GTK "Lookup" panel reachability | **Partial** | Wires up MusicBrainz only; the other 12 real providers are compiled and unit-tested but not reachable from any UI or CLI entry point yet |

**Provider inventory (verified against `crates/mm-providers/src/{music,video,podcasts,identifiers}/mod.rs`):**

| Category | Real HTTP clients | Disabled stubs |
| -------- | ------------------ | --------------- |
| 🎵 Music | MusicBrainz, Spotify (OAuth2), Apple Music *(iTunes Search API, not MusicKit)*, Deezer | YouTube Music, Amazon Music, Pandora, Tidal, Shazam, iHeart — all return `NotSupported`, disabled by default |
| 🎬 Video | TMDb, TheTVDB *(bearer auth uses the raw API key — the documented `/login` token exchange is never called)*, **OMDb** *(the provider shipped as "IMDb" is OMDb, id `omdb`, requires an API key — there is no scraper and no key-less IMDb access)*, Apple TV, iTunes Store | — |
| 🎙️ Podcasts | Apple Podcasts | — |
| 🆔 Identifiers | ISRC *(via MusicBrainz only — not a federated multi-registry lookup)*, EIDR *(response parser shape is unverified against a real response)*, ISWC | — |

> No Discogs, AcoustID, or Spotify audio-features (danceability/energy/tempo) code exists anywhere
> in the crate — these are documented in some `help/providers/*.md` pages but were never built.

---

### M6 — Full Native UI *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.7.0`

**Progress: 100%** | Issues: #85-93 | **134 macOS Swift tests + 124 Windows C# tests = 258 UI tests** (GTK4 Rust UI tests are counted inside `mm-gtk`'s 67, above)

| Deliverable | Status | Platform |
| ----------- | ------ | -------- |
| Lookup panel (search + results + providers) | Done | GTK4, macOS, Windows |
| Full rule builder (template + live preview + tag pills) | Done | GTK4, macOS, Windows |
| Cover art display | Done | GTK4, macOS, Windows |
| Drag-and-drop folder import | Done | GTK4, macOS, Windows |
| Real settings save to disk | Done | GTK4, macOS, Windows |
| Dark/light theme toggle | Done | GTK4 |
| Error dialogs | Done | GTK4 |

---

### M7 — Cloud Storage Monitoring — **Status: not yet implemented (scaffold only)**

> Started: 2026-03-05 | "Completed" 2026-03-05, **reopened 2026-09-03** as #94-#102

**What actually exists:** real trait definitions and types (`CloudProvider`, `CloudError`, `CloudFile`, `ChangeSet`, `SyncConfig`), a `SyncManager` with polling/conflict-resolution logic, and Cloud UI tabs on all three platforms. `mm-cloud` currently has **117 tests**.

**What does not exist:** no provider makes a real network call. OAuth flows for OneDrive, Google Drive, and Dropbox exist only as comments (`crates/mm-cloud/src/onedrive.rs:90`: *"In production this parses `reqwest::Response` JSON; here it is a stub"*); `reqwest` is never actually invoked. MEGA and iCloud are explicit stubs. Treat this milestone as architecture ready to be wired up, not a working feature — do not tell users cloud sync works.

---

### M8 — Packaging & Public Beta *(Tooling built; nothing actually published)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.9.0`

**Progress:** packaging scripts and manifests exist for every platform, but:

- **No release has ever been published.** The only GitHub release is the pre-rename *"MetaMancer v1.0-M1"* (2025-06-16); "First public beta release" (#110) has no artifact.
- **App Store submission** (#104) and **Microsoft Store submission** (#106) have not happened — no store listings exist.
- **Linux and WinGet packaging manifests are not version-synced with `Cargo.toml` (1.3.0):** `linux/snap/snapcraft.yaml` and `linux/deb/control` say `0.9.0`; the AppStream `metainfo.xml` lists releases at `0.9.0`/`0.8.0`; `linux/flatpak/…yaml` pins `tag: v1.0.0` with a literal `commit: placeholder-pin-to-actual-commit-sha`; `windows/winget/manifests/…` is at `1.0.0`. The CI version-sync check only compares `Cargo.toml` against the macOS `Info.plist` and the Windows `Package.appxmanifest` — it does not catch any of this drift.
- `mm-update` crate (semver comparison, GitHub Releases API polling) is real and tested — currently **29 tests**.

---

### M9 — Database Export — **Status: not yet implemented (scaffold only)**

> Started: 2026-03-05 | "Completed" 2026-03-05, **reopened 2026-09-03** as #112-#119

**What actually exists:** real trait/type definitions (`DatabaseExporter`, `DbDialect`, `ExportRow`, `ExportConfig`), dialect-aware DDL generation in `schema.rs` for all 5 backends, the `meedya export` CLI command with DSN parsing/redaction, and an Export UI tab on all three platforms. `mm-export` currently has **111 tests**.

**What does not exist:** no backend ever opens a real database connection or executes SQL. `sqlx` and `tiberius` are declared dependencies, but (per `crates/mm-export/src/sqlite.rs:30`) *"In production this holds a `sqlx::SqlitePool`; for M9 the pool is …"* a placeholder. Treat every export backend as unimplemented until this is wired up.

---

### M10 — Secure Media Server — **Status: not yet implemented (scaffold only)**

> Started: 2026-03-05 | "Completed" 2026-03-05, **reopened 2026-09-03** as #120-#127

**What actually exists:** real JWT auth types (`JwtService`, `Claims`), a real RFC 7233 byte-range parser (`RangeParser`), route-table constants, and a Server UI tab on all three platforms. `mm-server` currently has **74 tests**.

**What does not exist:** `mm-server` never builds an `axum` router — there is no `.route(` call anywhere in the crate. Running `meedya serve` prints *"Server stub: exiting cleanly"* and exits (`crates/mm-cli/src/commands/serve.rs:337-342`). There is no REST API you can actually call, no media streaming, and no web frontend — the repository contains **zero `.html` files**, so issue #124's "embedded static files" deliverable has nothing to embed. Do not describe this as a working server.

---

## Architecture Health

Verified counts as of the 2026-09-03 audit (`.claude/HANDOFF.md`); 44,183 Rust LOC and 1,392 `#[test]`/`#[tokio::test]` functions repo-wide.

| Crate / Component | Path | Real status | Tests |
| ------------------ | ---- | ------------ | ----- |
| `mm-core` | `crates/mm-core/` | Working — config, classify, metadata, watcher, renamer, companion, state, logging, health, rule engine | 512 |
| `mm-providers` | `crates/mm-providers/` | Working, partially reachable — 19 registered providers, 13 real HTTP clients + 6 disabled stubs; only MusicBrainz is wired into any UI | 289 |
| `mm-cloud` | `crates/mm-cloud/` | **Scaffold only** — no real network calls (see M7) | 117 |
| `mm-export` | `crates/mm-export/` | **Scaffold only** — no real DB connections (see M9) | 111 |
| `mm-server` | `crates/mm-server/` | **Scaffold only** — no axum router ever built (see M10) | 74 |
| `mm-cli` | `crates/mm-cli/` | Working, except `lookup` (stub) and `scan --execute` (data-loss risk, #201) | 73 |
| `mm-ffi` | `crates/mm-ffi/` | Working | 23 |
| `mm-update` | `crates/mm-update/` | Working | 29 |
| `mm-gtk` | `crates/mm-gtk/` | Working (not a workspace member — build with `-p mm-gtk`) | 67 |
| macOS SwiftUI app | `macos/` | Working UI shell | 134 |
| Windows WinUI 3 app | `windows/` | Working UI shell | 124 |

`cargo test --workspace` (the 8 workspace members, `mm-gtk` excluded) currently reports **1,240 passed, 0 failed**.

---

## Platform Support Matrix

| Platform | Architecture | CI Tested | Native UI | Package Format |
| -------- | ------------ | --------- | --------- | -------------- |
| macOS | Apple Silicon (arm64) | Yes | SwiftUI | .dmg / .tar.gz |
| Windows | x64 | Yes | WinUI 3 | MSIX / .zip |
| Windows | ARM64 | Planned | WinUI 3 | MSIX / .zip |
| Linux | x86_64 | Yes | GTK4 | Flatpak / Snap / AppImage / .deb |
| Linux | ARM64 | Planned | GTK4 | .tar.gz |

---

## CI/CD Infrastructure

**9 workflows** (see `.claude/CLAUDE.md` for the full "umbrella PR Gate" architecture):

| Workflow | File | Status |
| -------- | ---- | ------ |
| PR Gate (umbrella) | `pr-gate.yml` | Active — the single required status check on `main`; detects changed paths and conditionally invokes the 4 platform CIs |
| Rust Core CI | `ci-rust.yml` | Active (format, lint, test, version-sync) — reusable, invoked by `pr-gate.yml` |
| macOS CI | `ci-macos.yml` | Active — reusable |
| Windows CI | `ci-windows.yml` | Active — reusable; **temporarily de-scoped from PR Gate** pending #148 |
| Linux CI | `ci-linux.yml` | Active — reusable |
| Version Bump | `version-bump.yml` | Active (manual trigger) |
| Release Build | `release.yml` | Active (tag trigger) — never actually triggered by a real tag push |
| Security Audit | `audit.yml` | Active (weekly + push) — **currently failing**, see #203 |
| Documentation | `docs.yml` | Active |

---

## Known open issues worth tracking here

| # | Issue |
| - | ----- |
| #201 | `meedya scan --execute` can silently overwrite files on intra-batch destination collisions — highest-severity open finding |
| #128 | Test Mode is never enforced — all production write paths call `metadata::write_tags` directly instead of the safe `integrity::write_tags_safe` |
| #199 | `mm-gtk` cannot be built standalone / workspace `exclude` handling |
| #198 | MusicBrainz provider hardening (rate limiting, escaping, tolerant parsing) |
| #200 | `mm-cloud` clippy warnings |
| #207 | No `LICENSE` file is tracked despite `license = "GPL-2.0-or-later"` being declared everywhere |
| #203 | `audit.yml` (Security Audit) is currently failing |
| #211 | Config schema mismatch — the shipped `config/settings.json5` does not match the `AppConfig` struct it is supposed to populate |
| #212 | Two config directories in use (`MeedyaManager/` for settings vs. lowercase `meedyamanager/` for Test Mode manifest + corruption log) |
| #214 | Cut a real pre-release — nothing has ever been published to GitHub Releases under the MeedyaManager name |

---

## Recent Activity

| Date | Activity |
| ---- | -------- |
| 2026-09-03 | **Branch consolidation** — merged the four outstanding work-in-progress branches into the single working branch: `claude/issue-196-identifier-convergence` (real work, below), plus `.claude/` configuration from `chore/claude-config-recovery-2026-07-20` and `feature/134-mm-core-metadata-migration`. `feature/MeedyaManager_MeedyaSuite-core_integration` was superseded by PR #159 and carried no unique work. |
| 2026-09-03 | **Full project-state reconciliation** — audited all GitHub issues against the actual codebase, reopened 40 issues closed as complete but found to be scaffolding (including all of M7/M9/M10), filed 15 new issues (#201-#215), and rewrote the top-level documentation (this file, `README.md`, `Project_Plan.md`, `Dev_Notes.md`) to describe verified reality instead of aspirational status. See `.claude/HANDOFF.md`. |
| 2026-09-01 | MusicBrainz provider hardening work (issue #198) landed on a feature branch: centralised MusicBrainz endpoint/query/response handling in `crates/mm-providers/src/musicbrainz.rs`, contact-bearing User-Agent, Lucene phrase-quoting/escaping, a shared rate limiter, ISRC direct lookup with search fallback, ISWC composer enrichment. **Note:** this work was *not* accompanied by a version bump — `Cargo.toml` remained at `1.3.0` throughout, despite changelog drafts labelling it `v1.3.2`. |
| 2026-08-03 | **#196 — Identifier-key convergence** — the eight `mm-providers` `META_*` keys converged onto MeedyaSuite-core's canonical unprefixed `extra_keys` names (`mm_iswc` → `iswc`, and so on), with a read-both shim (`read_meta()` + `LEGACY_META_PREFIX`) kept for one release; ISWC tag registry entry (`TXXX:ISWC`); coherence guard test that fails if a key drifts from core's name. |
| 2026-03-06 | Workspace lint configuration — `[workspace.lints]` with pedantic+nursery clippy groups added across the workspace crates. **Note:** despite changelog drafts labelling this `v1.3.1`, `Cargo.toml` was never bumped past `1.3.0`. Issue #129. |
| 2026-03-05 | M10 tooling landed (`mm-server` crate, `meedya serve` CLI command, Server tab on all 3 platforms) — see the M10 section above for why this does not amount to a working server. |
| 2026-03-05 | M9 tooling landed (`mm-export` crate, `meedya export` CLI command, Export tab on all 3 platforms) — see the M9 section above for why this does not amount to working database export. |
| 2026-03-05 | M8 tooling landed (`mm-update` crate, platform packaging manifests, update-notification UI) — see the M8 section above for what has and has not actually been published. |
| 2026-03-05 | M7 tooling landed (`mm-cloud` crate, Cloud UI tab on all platforms) — see the M7 section above for why this does not amount to working cloud sync. |
| 2026-03-05 | M6 Complete — Full native UI: lookup panel, rule builder, cover art, drag-and-drop, real settings save, theming, error dialogs on all 3 platforms. |
| 2026-03-05 | M5 Complete — 19 metadata providers registered (13 real HTTP clients, 6 disabled stubs), credentials, rate limiting (MusicBrainz only), fuzzy scoring, cover art. |
| 2026-03-05 | M4 Complete — FFI layer (UniFFI + cbindgen) and native UI shells on all 3 platforms. |
| 2026-03-05 | M3 Complete — CLI: scan, debug, edit, rule, watch, lookup (stub), config, report-bug. |
| 2026-03-05 | M2 Complete — Rule engine: lexer, recursive descent parser, tag registry, 24 template functions, evaluator, rule system, renamer integration. |
| 2026-03-04 | M1 Complete — All `mm-core` foundational modules implemented (Issues #40-#51). |
| 2026-03-04 | Version/Release Infrastructure — version-bump workflow, version-sync CI check, release pipeline, GitHub Wiki, `Dev_Notes.md` (Issues #32-#39). |
| 2026-03-04 | M0 Complete — Cargo workspace scaffolded, all platform shells created, CI/CD set up (Issues #19-#31). |

---

> *This file is updated with each significant change. For detailed changelog, see [docs/changelog.md](docs/changelog.md) — note that its `v1.3.1`/`v1.3.2` entries describe work that landed without a corresponding version bump; `Cargo.toml` remains at `1.3.0`.*
>
> *Last updated: 2026-09-03 — full documentation reconciliation pass.*
