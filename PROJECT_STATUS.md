# MeedyaManager — Project Status

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Cross-platform media manager and auto-organizer — Rust core + native UIs

---

## Quick Status

| Item | Status |
| ---- | ------ |
| **Current Milestone** | Post-M10 — v1.2.0 enhancements — **Complete** |
| **Overall Progress** | **100%** core milestones complete; cross-cutting enhancements complete |
| **Latest Version** | `v1.2.0` |
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
| Developer notes | Done | `Dev_Notes.md` |

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

### M6 — Full Native UI *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.7.0`

**Progress: 100%** | Issues: #85-93 | **~90 UI tests** (53 macOS Swift + 58 Windows C# = 111 total; GTK4 Rust tests counted in mm-gtk)

| Deliverable | Status | Platform |
| ----------- | ------ | -------- |
| Lookup panel (search + results + providers) | Done | GTK4, macOS, Windows |
| Full rule builder (template + live preview + tag pills) | Done | GTK4, macOS, Windows |
| Cover art display | Done | GTK4 (gtk::Picture), macOS (AsyncImage), Windows (Image/BitmapImage) |
| Drag-and-drop folder import | Done | GTK4 (DropTarget), macOS (onDrop), Windows (DragOver/Drop) |
| Real settings save to disk | Done | GTK4, macOS, Windows |
| Dark/light theme toggle | Done | GTK4 (adw::StyleManager) |
| Error dialogs | Done | GTK4 (adw::AlertDialog) |
| macOS 5-tab navigation | Done | Lookup tab added to ContentView |
| Windows LookupPage | Done | LookupPage.xaml + .xaml.cs |
| macOS XCTest target (53 tests) | Done | AppTab, RenamePreviewItem, LookupResult, ProviderEntry, MetadataModel, ScanModel |
| Windows xUnit project (58 tests) | Done | PreviewRow, LookupResultRow, ProviderEntry, TemplateValidation, SettingsSave |

---

### M7 — Cloud Storage Monitoring *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.8.0`

**Progress: 100%** | Issues: #94-102 | **~90 tests**

| Deliverable | Status | Tests |
| ----------- | ------ | ----- |
| `traits.rs` — `CloudProvider` trait, `CloudError`, `CloudFile`, `ChangeSet`, `CloudCapabilities`, `SyncStatus`, `SyncState`, `ConflictResolution`, `SyncConfig` | Done | 40 |
| `sync_manager.rs` — `SyncManager` with `SyncEvent`, polling, conflict resolution, delta cursor | Done | 15 |
| `onedrive.rs` — `OneDriveProvider` (Microsoft Graph API, delta queries) | Done | 14 |
| `google_drive.rs` — `GoogleDriveProvider` (Drive API v3, `changes.list`) | Done | 13 |
| `dropbox.rs` — `DropboxProvider` (Dropbox API v2, cursor-based delta) | Done | 14 |
| `mega.rs` — `MegaProvider` (stub — no official API) | Done | 6 |
| `icloud.rs` — `ICloudProvider` (stub — macOS FileProvider native only) | Done | 7 |
| `lib.rs` — Re-exports + integration tests | Done | 15 |
| GTK4 `cloud_panel.rs` — Cloud tab (6 tabs total), provider rows, event log | Done | 7 |
| macOS `CloudView.swift` — Cloud tab (6 tabs total), `CloudModel`, event log | Done | 11 |
| macOS `AppState.swift` — `.cloud` case added to `AppTab` (6 cases) | Done | 2 |
| Windows `CloudPage.xaml(.cs)` — Cloud page, provider rows, simulated async sync | Done | 12 |

---

### M8 — Packaging & Public Beta *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.9.0`

**Progress: 100%** | Issues: #103-111 | **~30 tests**

| Deliverable | Status | Notes |
| ----------- | ------ | ----- |
| `mm-update` crate — `UpdateChecker` + `ReleaseInfo` + `UpdateError` | Done | semver comparison, GitHub Releases API |
| `mm-update/release.rs` — `GitHubRelease`, `ReleaseInfo` | Done | 9 unit tests |
| `mm-update/checker.rs` — `UpdateChecker`, async `check()` | Done | 14 unit tests |
| `mm-update/lib.rs` — `UpdateError`, integration tests | Done | 10 integration tests |
| Flatpak manifest (`ltd.MWBMpartners.MeedyaManager.yaml`) | Done | GNOME 47 runtime, cargo vendor offline build |
| `.desktop` entry (`ltd.MWBMpartners.MeedyaManager.desktop`) | Done | Freedesktop standard |
| AppStream MetaInfo (`ltd.MWBMpartners.MeedyaManager.metainfo.xml`) | Done | OARS 1.1, categories, release history |
| Snap manifest (`snapcraft.yaml`) | Done | core22 base, GNOME 42 extension, strict confinement |
| AppImage build script (`build-appimage.sh`) | Done | AppDir assembly + appimagetool |
| Debian package script (`build-deb.sh` + `control`) | Done | dpkg-deb, Depends: libgtk-4-1 + libadwaita-1-0 |
| macOS entitlements (`MeedyaManager.entitlements`) | Done | App Sandbox, hardened runtime |
| macOS DMG creation script (`create-dmg.sh`) | Done | codesign + notarytool + create-dmg/hdiutil |
| WinGet manifest (`MWBM.MeedyaManager.yaml`) | Done | v1.6.0 schema, x64 + arm64 MSIX |
| GTK4 `AdwBanner` update notification | Done | Above tab bar, hidden until update found |
| macOS "Updates" section in SettingsView | Done | Check button, status text, Download link |
| Windows `InfoBar` + `CheckForUpdatesAsync()` | Done | Background Task.Delay stub, DispatcherQueue |
| `release.yml` updated | Done | DMG creation + deb/AppImage build + upload steps |
| Version bumped `0.8.0` → `0.9.0` | Done | Cargo.toml, Info.plist, Package.appxmanifest |

---

### M9 — Database Export *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v0.10.0`

**Progress: 100%** | Issues: #112-119 | **~90 tests**

| Deliverable | Status | Tests |
| ----------- | ------ | ----- |
| `mm-export/traits.rs` — `DbDialect` (5 variants), `ExportRow`, `RenameEvent`, `ExportConfig`, `ExportStats`, `ExportError`, `DatabaseExporter` async trait (RPITIT) | Done | 22 |
| `mm-export/schema.rs` — `SchemaBuilder` dialect-aware DDL (files, tags, history tables) for all 5 backends | Done | 15 |
| `mm-export/sqlite.rs` — `SqliteExporter` with `INSERT OR REPLACE` upsert | Done | 15 |
| `mm-export/mysql.rs` — `MySqlExporter` with `ON DUPLICATE KEY UPDATE` upsert | Done | 10 |
| `mm-export/mariadb.rs` — `MariaDbExporter` wrapping same SQL as MySQL, distinct `DbDialect::MariaDb` | Done | 10 |
| `mm-export/postgres.rs` — `PostgresExporter` with `ON CONFLICT … DO UPDATE SET EXCLUDED.*` + `$1` positional params | Done | 11 |
| `mm-export/mssql.rs` — `MssqlExporter` with T-SQL `MERGE INTO … WHEN MATCHED/NOT MATCHED` + `@param_name` style | Done | 12 |
| `mm-export/lib.rs` — Re-exports + integration tests (dialect uniqueness, DSN validation, stats, serde round-trip) | Done | 15 |
| `meedya export` CLI command (`commands/export.rs`) — `BackendChoice`, `ExportArgs`, `detect_backend()`, `redact_dsn()`, `--show-schema` DDL preview | Done | 14 |
| GTK4 `export_panel.rs` — `AdwPreferencesGroup` layout, `ComboBoxText` backend picker, live DSN placeholder, `SchemaBuilder` DDL preview | Done | 7 |
| macOS `ExportView.swift` — `ExportBackend` enum, `@Observable ExportModel`, backend `Picker`, DSN/prefix/batchSize controls, log view | Done | 12 |
| macOS `ExportModelTests.swift` — 12 test funcs (replica structs for SPM test isolation) | Done | 12 |
| Windows `ExportPage.xaml/.cs` — `ComboBox` backend picker, `ToggleSwitch` dry-run, async stub, `StringBuilder` log, hint text | Done | 15 |
| Windows `ExportPageTests.cs` — 15 xUnit tests (backend hints, DSN validation, credential redaction, stats) | Done | 15 |
| `mm-cli` and `mm-gtk` `Cargo.toml` updated with `mm-export` dependency | Done | — |
| macOS `AppState.swift` — `.export` case added to `AppTab` (7 cases) | Done | — |
| macOS `ContentView.swift` — Export `Tab(...)` added (7 tabs), min width 960 | Done | — |
| Windows `MainWindow.xaml` — Export `NavigationViewItem` added (7 items) | Done | — |
| Windows `MainWindow.xaml.cs` — Export route added to switch | Done | — |
| Version bumped `0.9.0` → `0.10.0` | Done | Cargo.toml, Info.plist, Package.appxmanifest |

---

### M10 — Secure Media Server *(Complete)*

> Started: 2026-03-05 | Completed: 2026-03-05 | Version: `v1.0.0`

**Progress: 100%** | Issues: #120-127 | **~90 tests**

| Deliverable | Status | Tests |
| ----------- | ------ | ----- |
| `mm-server/auth.rs` — `ServerConfig`, `UserRole`, `Claims`, `AuthError` (6 variants), `LoginRequest`, `LoginResponse`, `JwtService` (`issue()`, `validate()`, `extract_bearer()`) | Done | 20 |
| `mm-server/streaming.rs` — `StreamConfig`, `StreamRequest` (Full/Range/FromStart/Suffix), `StreamResponse`, `StreamError` (7 variants), `RangeParser::parse()` (RFC 7233), `MediaStreamer` | Done | 20 |
| `mm-server/routes.rs` — `ApiResponse<T>` JSON envelope, `HealthResponse`, `LibraryItem`, `LibraryResponse`, `SearchQuery`, `ServerInfoResponse`, 7 handler stubs | Done | 24 |
| `mm-server/lib.rs` — Re-exports + integration tests (JWT round-trip, login, library, stream, range, server info, health, config validation) | Done | 15 |
| `meedya serve` CLI command (`commands/serve.rs`) — `ServeArgs`, `build_server_config()`, `validate_config()`, route table constant (8 routes), `--show-routes`, `--check-config` | Done | 14 |
| GTK4 `server_panel.rs` — network/TLS/auth/CORS `adw::PreferencesGroup` layout, `PasswordEntryRow` for JWT secret, status label, start/stop buttons, log `TextView` | Done | 6 |
| macOS `ServerView.swift` — `ServerStatus` enum, `@Observable ServerModel` (`startServer()`, `stopServer()`, `validationError` computed property), 8-section form + log | Done | 18 |
| macOS `AppState.swift` — `.server` case added to `AppTab` (8 cases, `"network"` icon) | Done | — |
| macOS `ContentView.swift` — Server `Tab(...)` added (8 tabs), min width 1000 | Done | — |
| macOS `ServerModelTests.swift` — 18 Swift Testing tests (defaults, validation, log, status display) | Done | 18 |
| Windows `ServerPage.xaml/.cs` — network/TLS/auth/CORS config, `PasswordBox` JWT secret, `ToggleSwitch` no-TLS, start/stop/routes buttons, `ValidateConfig()`, access log | Done | 26 |
| Windows `MainWindow.xaml` — Server `NavigationViewItem` added (Globe symbol) | Done | — |
| Windows `MainWindow.xaml.cs` — Server route added: `Tag: "Server"` → `typeof(ServerPage)` | Done | — |
| Windows `ServerPageTests.cs` — 26 xUnit tests (`ServerRoutes`, `ServerConfigValidator`, `JwtHelper`) | Done | 26 |
| `mm-cli` and `mm-gtk` `Cargo.toml` updated with `mm-server` dependency | Done | — |
| Version bumped `0.10.0` → `1.0.0` | Done | Cargo.toml, Info.plist, Package.appxmanifest |

---

## Architecture Health

| Crate / Component | Path | Status |
| ----------------- | ---- | ------ |
| `mm-core` | `crates/mm-core/` | **M2 Complete** (399 tests) |
| `mm-providers` | `crates/mm-providers/` | **M5 Complete** (332 tests, 19 providers) |
| `mm-cloud` | `crates/mm-cloud/` | **M7 Complete** (~90 tests — `CloudProvider` trait, OneDrive, Google Drive, Dropbox, MEGA stub, iCloud stub, `SyncManager`) |
| `mm-update` | `crates/mm-update/` | **M8 Complete** (~33 tests — `UpdateChecker`, `ReleaseInfo`, `UpdateError`, semver comparison) |
| `mm-export` | `crates/mm-export/` | **M9 Complete** (~90 tests — `DatabaseExporter` trait, 5 backends, `SchemaBuilder` DDL) |
| `mm-server` | `crates/mm-server/` | **M10 Complete** (~79 tests — `JwtService`, `RangeParser`, `MediaStreamer`, handler stubs) |
| `mm-cli` | `crates/mm-cli/` | **M10 Complete** (45+14+14 tests — `meedya serve` command added) |
| `mm-ffi` | `crates/mm-ffi/` | **M4 Complete** (20 tests) |
| `mm-gtk` | `crates/mm-gtk/` | **M10 Complete** (8 tabs + Server panel, 42+7+6 tests) |
| macOS SwiftUI app | `macos/` | **M10 Complete** (8 tabs + ServerView, 64+12+18 tests) |
| Windows WinUI 3 app | `windows/` | **M10 Complete** (8 pages + ServerPage, 70+15+26 tests) |

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
| 2026-03-05 | **M10 Complete** (`v1.0.0`) — Secure Media Server: `mm-server` crate (JWT/HS256, RFC 7233 range streaming, REST API handler stubs), `meedya serve` CLI command, Server tab on all 3 platforms (GTK4/macOS/Windows); ~90 new tests (~1076 → ~1166 total) |
| 2026-03-05 | **M9 Complete** (`v0.10.0`) — Database Export: `mm-export` crate (`DatabaseExporter` trait, 5 backends, `SchemaBuilder` DDL), `meedya export` CLI command, Export tab on all 3 platforms (GTK4/macOS/Windows); ~90 new tests (~986 → ~1076 total) |
| 2026-03-05 | **M8 Complete** (`v0.9.0`) — Packaging & Public Beta: `mm-update` crate (UpdateChecker, semver), Flatpak/Snap/AppImage/.deb manifests, macOS entitlements + DMG script, WinGet manifest, update notification UI (GTK4 AdwBanner, macOS Updates section, Windows InfoBar); ~30 new tests (~956 → ~986 total) |
| 2026-03-05 | **M7 Complete** (`v0.8.0`) — Cloud Storage Monitoring: `mm-cloud` crate (`CloudProvider` trait, OneDrive, Google Drive, Dropbox, MEGA stub, iCloud stub, `SyncManager`), Cloud UI tab on all platforms; ~90 new tests (~866 → ~956 total) |
| 2026-03-05 | **M6 Complete** (`v0.7.0`) — Full Native UI: Lookup panel (all 3 platforms), rule builder, cover art, DnD, real settings save, dark/light theme (GTK4), error dialogs; ~90 UI tests (776 → ~866 total) |
| 2026-03-05 | **M5 Complete** (`v0.6.0`) — Metadata Lookup Providers: 19 providers, credentials, rate limiting, fuzzy scoring, cover art; 332 new tests (776 total) |
| 2026-03-05 | **M4 Complete** (`v0.5.0`) — FFI Layer & Native UI Shells: mm-ffi (UniFFI + cbindgen), mm-gtk (GTK4/Adwaita Linux shell), macOS SwiftUI shell (4 views), Windows WinUI 3 shell (4 pages), 20 new tests (464 total) |
| 2026-03-05 | **M3 Complete** (`v0.4.0`) — CLI: 8 commands (scan, debug, edit, rule, watch, lookup, config, report-bug), shared output infrastructure, CLI context, dual output modes (Human/JSON), 45 new tests (444 total) |
| 2026-03-05 | **M2 Complete** (`v0.3.0`) — Rule engine: lexer, recursive descent parser, 40+ tag registry, 24 template functions, evaluator with EvalContext, declarative rule system, renamer integration, config extension. 182 new tests (399 total) |
| 2026-03-05 | **M1 Complete** (`v0.2.0`) — All mm-core modules implemented: config, classify, metadata, watcher, renamer, companion, state, logging, health. 217 tests passing (Issues #40-#51) |
| 2026-03-04 | **Version/Release Infrastructure** — Added version-bump workflow, version-sync CI check, enhanced release pipeline with checksums, created GitHub Wiki, Dev_Notes.md (Issues #32-#39) |
| 2026-03-04 | **M0 Complete** (`v0.1.0`) — Archived Python, created Cargo workspace, scaffolded all platforms, set up CI/CD, GitHub Projects v2 (Issues #19-#31) |
| 2026-03-04 | **v1.x archived** — Tagged `v1.5-M6-python-final` (1007 tests, 6 milestones, 19 providers) |

---

> *This file is updated with each significant change. For detailed changelog, see [docs/changelog.md](docs/changelog.md).*
>
> *Last updated: 2026-03-05 (M10 complete — Secure Media Server + Public Release v1.0.0)*
