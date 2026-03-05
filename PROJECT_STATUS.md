# MeedyaManager — Project Status

> **(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)**
>
> Cross-platform media manager and auto-organizer — Rust core + native UIs

---

## Quick Status

| Item | Status |
| ---- | ------ |
| **Current Milestone** | M6 — Full Native UI — **Up Next** |
| **Overall Progress** | **55%** (6 of 11 milestones complete) |
| **Latest Version** | `v0.6.0` |
| **Python v1.x** | Archived at tag `v1.5-M6-python-final` |
| **Build Status** | ![CI](https://github.com/MWBMPartners/MeedyaManager/actions/workflows/ci-rust.yml/badge.svg) |

---

## Milestone Progress

### M0 — Repository Setup & Scaffolding *(Complete)*

> Started: 2026-03-04 | Version: `v0.1.0`

**Progress: 100%** | Issues: #19-#31, #32-#39 (all closed)

| Deliverable | Status | Notes |
| ----------- | ------ | ----- |
| Archive Python v1.x codebase | Done | Tagged `v1.5-M6-python-final` |
| Delete Python source tree | Done | All `.py` files removed |
| Cargo workspace with 8 crates | Done | mm-core, mm-providers, mm-cloud, mm-export, mm-server, mm-cli, mm-ffi, mm-gtk |
| macOS SwiftUI scaffold | Done | `macos/` with Package.swift |
| Windows WinUI 3 scaffold | Done | `windows/` with .sln/.csproj |
| Rust toolchain configuration | Done | `.rustfmt.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml` |
| CI/CD workflows (8 workflows) | Done | ci-rust, ci-macos, ci-windows, ci-linux, version-bump, release, audit, docs |
| GitHub Projects v2 board | Done | 11 milestones, custom fields, 5 views |
| Documentation update | Done | All `.md` files rewritten |
| Automated version management | Done | `version-bump.yml` workflow, version-sync CI check |
| Release build pipeline | Done | `release.yml` with 5 platform builds, checksums, draft releases |
| GitHub Wiki | Done | Version Management, Release Process, CI/CD Pipelines pages |
| Developer notes | Done | `docs/Dev_Notes.md` |

---

### M1 — Core Engine *(Complete)*

> Started: 2026-03-04 | Completed: 2026-03-05 | Version: `v0.2.0`

**Progress: 100%** | Issues: #40-#51 | **217 tests** (214 unit + 3 doc-tests)

| Deliverable | Status | Tests |
| ----------- | ------ | ----- |
| Error types (`thiserror`) | Done | 5 |
| Config module (JSON5 + .env + env overrides) | Done | 22 |
| Media classification (4-level: Group/Format/Class/Quality) | Done | 38 |
| Metadata extraction & writing (`lofty`) | Done | 36 |
| File watcher (`notify` + debounce + filtering) | Done | 15 |
| Rename simulator + filename sanitizer | Done | 16 |
| Companion file detector (subtitles, lyrics, art, cue) | Done | 16 |
| State manager + single-instance lock file | Done | 13 |
| Structured logging (tracing + PII redaction) | Done | 13 |
| Health checks (config, folders, disk, writable) | Done | 14 |
| Rule engine (stub — deferred to M2) | Stub | 0 |

---

### M2 — Rule Engine *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.3.0`

**Progress: 100%** | **182 tests** (181 unit + 1 doc-test)

| Deliverable | Status | Tests |
| ----------- | ------ | ----- |
| Lexer (tokenizer: tags, functions, literals, legacy detection) | Done | 26 |
| Parser (recursive descent, AST, 50-level depth guard) | Done | 24 |
| Tag registry (40+ bidirectional mappings, virtual tags) | Done | 24 |
| Template functions (24: logical, string, numeric, lookup, extensions) | Done | 47 |
| Evaluator (EvalContext, multi-value, missing tag modes) | Done | 30 |
| Rule system (conditions, operators, priority ordering, apply_rules) | Done | 30 |
| Renamer integration (`simulate_rename_with_rules`) | Done | — |
| Config extension (`rules` + `missing_tag_mode` in RenameConfig) | Done | — |

---

### M3 — CLI *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.4.0`

**Progress: 100%** | Issues: #52-#62 (all closed) | **45 tests**

| Deliverable | Status | Tests |
| ----------- | ------ | ----- |
| Output infrastructure (`output.rs`) | Done | 4 |
| CLI context (`context.rs`) | Done | 3 |
| `main.rs` restructure (Commands enum, global flags, dispatch) | Done | — |
| `meedya debug` — single-file metadata inspector | Done | 5 |
| `meedya rule` — template validation, tag listing, test, legacy detection | Done | 6 |
| `meedya config` — show, path, init, export, import | Done | 5 |
| `meedya scan` — directory scan + rename preview + execute | Done | 7 |
| `meedya edit` — metadata write (--set, --remove, --cover) | Done | 6 |
| `meedya watch` — foreground watcher with event logging | Done | 4 |
| `meedya lookup` — provider search (stub for M5) | Done | 2 |
| `meedya report-bug` — system info + log collection | Done | 3 |
| Documentation updates | Done | — |

---

### M4 — FFI Layer & Native UI Shells *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.5.0`

**Progress: 100%** | Issues: #63-#72 | **20 tests** (mm-ffi unit + mm-gtk state)

| Deliverable | Status | Notes |
| ----------- | ------ | ----- |
| `mm-ffi` crate — UniFFI proc-macro scaffolding | Done | `setup_scaffolding!("mm_ffi")` |
| FFI types (`types.rs`) — TagEntry, RenamePreviewFfi, AudioPropertiesFfi, ValidationResult, WatchEventFfi, MmFfiError | Done | `uniffi::Record` / `uniffi::Error` derives |
| UniFFI callback interfaces (`callbacks.rs`) — WatchCallback, ScanProgressCallback | Done | `#[uniffi::export(callback_interface)]` |
| UniFFI API (`uniffi_api.rs`) — 8 exported functions wired to mm-core | Done | scan_directory, get_metadata, write_metadata, get_audio_properties, validate_template, list_known_tags, start_watch, stop_watch |
| C API (`capi.rs`) — 9 `#[no_mangle]` functions with JSON transport | Done | mm_ffi_version, mm_ffi_config_path, mm_ffi_scan_directory, mm_ffi_get_metadata, mm_ffi_write_metadata, mm_ffi_validate_template, mm_ffi_apply_template, mm_ffi_list_known_tags, mm_ffi_free_string |
| cbindgen config + build.rs — generates `include/mm_ffi.h` | Done | cbindgen 0.27, language C |
| UDL reference file (`mm_ffi.udl`) | Done | Documentation only |
| mm-ffi unit tests (lib.rs) — 20 tests | Done | Error display, TagEntry, RenamePreview, ValidationResult, AudioProperties, WatchEvent, UniFFI API, C API |
| `mm-gtk` crate — lib.rs + main.rs | Done | `mm_gtk::run_app()` entry point |
| `mm-gtk` state module (`state.rs`) — ScanState, MetadataState | Done | 10 unit tests |
| `mm-gtk` main window (`main_window.rs`) — AdwTabView + 4 tabs + AboutDialog | Done | adw::ApplicationWindow |
| `mm-gtk` scan panel (`scan_panel.rs`) — folder picker, scan, preview, execute | Done | gtk::FileDialog async |
| `mm-gtk` metadata panel (`metadata_panel.rs`) — file picker, tag editor, save, revert | Done | gtk::ListBox |
| `mm-gtk` rules panel (`rules_panel.rs`) — template validator + tag pills (M4 stub) | Done | M6 TODO |
| `mm-gtk` settings panel (`settings_panel.rs`) — AdwPreferencesGroup, raw JSON5 view | Done | adw::Clamp |
| macOS `AppState.swift` — `@Observable` AppState + AppTab enum | Done | selectedTab, ScanModel, MetadataModel |
| macOS `ScanModel.swift` + `MetadataModel.swift` — observable models | Done | scan(), executeRenames(), loadFile(), saveAll() |
| macOS `MmCore.swift` — P/Invoke bridge with `#if MM_FFI_AVAILABLE` guards | Done | All functions stubbed for development |
| macOS `ContentView.swift` — TabView with .sidebarAdaptable, Liquid Glass | Done | `#available(macOS 26.0, *)` |
| macOS `ScanView.swift` — HSplitView, fileImporter, TemplateValidationBadge | Done | |
| macOS `MetadataView.swift` — toolbar, TagEditorList, status bar | Done | |
| macOS `RulesView.swift` — template validator, live preview, tag pills (M4 stub) | Done | M6 TODO |
| macOS `SettingsView.swift` — preferences, config path, raw JSON5 view | Done | |
| Windows `MmCore.cs` — P/Invoke bridge with DLL availability guard + stubs | Done | JSON transport |
| Windows `ScanPage.xaml/.cs` — folder picker, template validator, rename preview | Done | FolderPicker + async Task |
| Windows `MetadataPage.xaml/.cs` — file picker, editable tag grid, save/revert | Done | INotifyPropertyChanged |
| Windows `RulesPage.xaml/.cs` — template validator, live preview, tag pills (M4 stub) | Done | M6 TODO |
| Windows `SettingsPage.xaml/.cs` — preferences, config path, raw JSON5 view | Done | |
| Windows `MainWindow.xaml.cs` — NavigationView routing to 4 pages | Done | ContentFrame.Navigate() |

---

### M5 — Metadata Lookup Providers *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.6.0`

**Progress: 100%** | Issues: #73-#84 | **332 tests**

| Deliverable | Status | Tests |
| ----------- | ------ | ----- |
| `traits.rs` — MetadataProvider, SearchQuery, ProviderResult, CoverArtInfo, Capabilities, ProviderError, MediaType | Done | 20 |
| `credentials.rs` — 4-tier resolution (env/config/keyring/file), CredentialStore | Done | 30 |
| `rate_limiter.rs` — token-bucket per-provider, RateLimiterRegistry, default_rpm_for() | Done | 25 |
| `match_scoring.rs` — weighted fuzzy scoring, MatchScorer, ScoringWeights, rank_results() | Done | 40 |
| `cover_art.rs` — CoverArtSize, select/filter/deduplicate helpers, URL validators | Done | 20 |
| `registry.rs` — ProviderRegistry, search() fan-out, search_provider(), find_by_name() | Done | 25 |
| `MusicBrainzProvider` — XML2 REST, ISRC lookup | Done | 20 |
| `SpotifyProvider` — OAuth2 client-credentials, album art | Done | 18 |
| `AppleMusicProvider` — iTunes Search API, hi-res cover | Done | 14 |
| `DeezerProvider` — public JSON API, ISRC via endpoint | Done | 18 |
| 6 stub providers (YouTube Music, Amazon Music, Pandora, Tidal, Shazam, iHeart) | Done | 12 |
| `TmdbProvider` — TMDb multi-search, movie+TV | Done | 15 |
| `TheTvdbProvider` — TheTVDB v4 Bearer auth | Done | 10 |
| `OmdbProvider` — OMDb query + N/A handling | Done | 12 |
| `AppleTvProvider` — iTunes movie search, hi-res cover | Done | 8 |
| `ItunesStoreProvider` — iTunes tvShow/tvSeason search | Done | 10 |
| `ApplePodcastsProvider` — iTunes podcast search, feed_url/episode_count in extra | Done | 12 |
| `IsrcProvider` — MusicBrainz recording by ISRC | Done | 10 |
| `EidrProvider` — EIDR registry Basic-auth | Done | 10 |
| `IswcProvider` — MusicBrainz work by ISWC | Done | 10 |
| `lib.rs` integration smoke tests (15 tests) | Done | 15 |

---

### M6 — Full Native UI *(Planned)*

> Target: `v0.7.0`

Complete views on all 3 platforms: Rule Builder, Metadata Editor, Lookup Panel, accessibility.

---

### M7 — Cloud Storage Monitoring *(Planned)*

> Target: `v0.8.0`

OneDrive, Google Drive, Dropbox, MEGA, iCloud.

---

### M8 — Packaging & Public Release *(Planned)*

> Target: `v0.9.0`

App Store (macOS), Microsoft Store (Windows), Flatpak/Snap (Linux). Code signing, notarization.

---

### M9 — Database Export *(Planned)*

> Target: `v0.10.0`

MySQL, MariaDB, SQL Server, SQLite, PostgreSQL via `sqlx`/`tiberius`.

---

### M10 — Secure Media Server *(Planned)*

> Target: `v1.0.0`

`axum` HTTP server, REST API, JWT auth, media streaming, TLS.

---

## Architecture Health

| Crate / Component | Path | Status |
| ----------------- | ---- | ------ |
| `mm-core` | `crates/mm-core/` | **M2 Complete** (399 tests) |
| `mm-providers` | `crates/mm-providers/` | **M5 Complete** (332 tests, 19 providers) |
| `mm-cloud` | `crates/mm-cloud/` | Scaffold (stubs) |
| `mm-export` | `crates/mm-export/` | Scaffold (stubs) |
| `mm-server` | `crates/mm-server/` | Scaffold (stubs) |
| `mm-cli` | `crates/mm-cli/` | **M3 Complete** (45 tests) |
| `mm-ffi` | `crates/mm-ffi/` | **M4 Complete** (20 tests) |
| `mm-gtk` | `crates/mm-gtk/` | **M4 Complete** (GTK4/Adwaita Linux shell) |
| macOS SwiftUI app | `macos/` | **M4 Complete** (4 views: Scan, Metadata, Rules, Settings) |
| Windows WinUI 3 app | `windows/` | **M4 Complete** (4 pages: Scan, Metadata, Rules, Settings) |

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

| Workflow | File | Status |
| -------- | ---- | ------ |
| Rust Core CI | `ci-rust.yml` | Active (format, lint, test, version-sync) |
| macOS CI | `ci-macos.yml` | Active |
| Windows CI | `ci-windows.yml` | Active |
| Linux CI | `ci-linux.yml` | Active |
| Version Bump | `version-bump.yml` | Active (manual trigger) |
| Release Build | `release.yml` | Active (tag trigger) |
| Security Audit | `audit.yml` | Active (weekly + push) |
| Documentation | `docs.yml` | Active |

---

## Recent Activity

| Date | Activity |
| ---- | -------- |
| 2026-03-05 | **M4 Complete** (`v0.5.0`) — FFI Layer & Native UI Shells: mm-ffi (UniFFI + cbindgen), mm-gtk (GTK4/Adwaita Linux shell), macOS SwiftUI shell (4 views), Windows WinUI 3 shell (4 pages), 20 new tests (464 total) |
| 2026-03-05 | **M3 Complete** (`v0.4.0`) — CLI: 8 commands (scan, debug, edit, rule, watch, lookup, config, report-bug), shared output infrastructure, CLI context, dual output modes (Human/JSON), 45 new tests (444 total) |
| 2026-03-05 | **M2 Complete** (`v0.3.0`) — Rule engine: lexer, recursive descent parser, 40+ tag registry, 24 template functions, evaluator with EvalContext, declarative rule system, renamer integration, config extension. 182 new tests (399 total) |
| 2026-03-05 | **M1 Complete** (`v0.2.0`) — All mm-core modules implemented: config, classify, metadata, watcher, renamer, companion, state, logging, health. 217 tests passing (Issues #40-#51) |
| 2026-03-04 | **Version/Release Infrastructure** — Added version-bump workflow, version-sync CI check, enhanced release pipeline with checksums, created GitHub Wiki, Dev_Notes.md (Issues #32-#39) |
| 2026-03-04 | **M0 Complete** (`v0.1.0`) — Archived Python, created Cargo workspace, scaffolded all platforms, set up CI/CD, GitHub Projects v2 (Issues #19-#31) |
| 2026-03-04 | **v1.x archived** — Tagged `v1.5-M6-python-final` (1007 tests, 6 milestones, 19 providers) |

---

> *This file is updated with each significant change. For detailed changelog, see [docs/CHANGELOG.md](docs/CHANGELOG.md).*
>
> *Last updated: 2026-03-05 (M4 complete, M5 in progress, version scheme updated to v0.x.y)*
