# Rust Crate APIs

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager's Rust workspace has 9 crates. 8 are workspace members (built by a plain
`cargo build`); `mm-gtk` is deliberately excluded (see its section below) but shares the same
dependency shape. This page is a guided tour derived directly from each crate's `Cargo.toml` and
`src/lib.rs` — module lists and entry points below are the real, current public surface, not an
aspirational one.

```bash
# Full generated API documentation for every workspace crate, opened locally
cargo doc --workspace --no-deps --open
```

`cargo doc --workspace --no-deps` is a CI gate (`.github/workflows/docs.yml`, and again as part of
`ci-rust.yml`) run with `RUSTDOCFLAGS="-D warnings"` — any broken doc link or invalid rustdoc
syntax fails the build. There is, however, **no `missing_docs` lint** anywhere in
`[workspace.lints.rust]` (root `Cargo.toml`) — undocumented `pub` items compile and pass CI without
a warning. Coverage of the public surface below is a matter of convention and code review, not a
tooling gate.

`mm-gtk` is outside the workspace (see its own section), so `cargo doc --workspace` does not cover
it; run `cargo doc --manifest-path crates/mm-gtk/Cargo.toml --no-deps --open` separately.

## Dependency graph

```text
meedya-core (upstream — MWBMPartners/MeedyaSuite-core, pinned git rev, NOT part of this repo)
  └── mm-core   (this repo's base engine — depends on nothing else here)
        ├── mm-providers   (also depends on meedya-core directly, for provider trait re-exports)
        ├── mm-cloud
        ├── mm-update
        ├── mm-export
        ├── mm-server
        ├── mm-ffi
        └── mm-gtk         (excluded from the Cargo workspace — see below)

mm-cli  → mm-core, mm-providers, mm-export, mm-server
mm-gtk  → mm-core, mm-providers, mm-export, mm-server   (same set as mm-cli)
```

Nothing in this workspace depends on `mm-cli`, `mm-ffi` or `mm-gtk` — they are the three "leaf"
crates, one per platform-facing surface (CLI binary, FFI cdylib for Windows/macOS, Linux native
binary). `mm-cloud` and `mm-update` are library crates with **no current consumer at all** — see
their sections below.

## mm-core

**Purpose:** the shared engine every other crate builds on — configuration, file watching, media
classification, the rule/template engine, rename simulation and execution, metadata read/write,
companion-file tracking, Test Mode, integrity checking, background service management, i18n,
logging and health checks. Depends only on the upstream `meedya-core` git dependency; no other
crate in this repository.

**Modules** (from `crates/mm-core/src/lib.rs`): `config`, `classify`, `watcher`, `rule_engine`,
`renamer`, `companion`, `metadata`, `state`, `logging`, `health`, `error`, `i18n`,
`filetype_registry`, `integrity`, `service`, `settings_bundle`, `useragent`, `test_mode`.

**Main entry points:**

- `config::AppConfig::{load, load_from, default}` — JSON5 config loading with `.env` fallback
- `classify::{classify_by_path, classify_by_extension}` — the 4-level media classification
  hierarchy (Group → Format → Class → Quality)
- `watcher::start_watcher(&WatcherConfig)` — returns `(RecommendedWatcher, Receiver<WatchEvent>)`
- `rule_engine::{parse_template, evaluate_template}` — the MusicBee-inspired template
  lexer/parser/evaluator
- `renamer::{simulate_rename, execute_rename}` — the single source of truth for computing and
  performing renames; every consumer (`mm-cli scan`, `mm-ffi`'s `scan_directory`/`execute_renames`)
  delegates here rather than reimplementing destination logic
- `metadata::{extract_tags, extract_audio_properties, extract_cover_art}` — `lofty`-backed reading
- `integrity::{write_tags_safe, remove_tag_safe}` — the **only** sanctioned mutation path; this is
  where Test Mode is enforced (a direct call to `metadata::` write functions bypasses Test Mode)
- `test_mode::{enable, disable, is_enabled, commit_files, revert_files, tracked_files,
  tracked_file_count}` — non-destructive "duplicate-on-write" editing
- `service::{install_service, uninstall_service, start_service, stop_service, service_status}` —
  systemd/launchd/Windows Service management
- `settings_bundle::SettingsBundle::{capture, export, import, apply}` — portable `.mmprofile`
  configuration bundles

## mm-providers

**Purpose:** 19 metadata lookup providers across music (10), video (5), podcasts (1) and
identifiers (3). Depends on `mm-core` and directly on `meedya-core` (re-exports its provider trait
surface through `traits.rs`).

**Modules:** `traits`, `registry`, `credentials`, `rate_limiter`, `match_scoring`, `cover_art`,
`music`, `video`, `podcasts`, `identifiers`, `musicbrainz` (a centralised integration seam — every
MusicBrainz endpoint URL, Lucene query fragment and the shared rate limiter live only here, ahead
of MusicBrainz's announced 2026-11-30 breaking search-API change), `http` (private — shared
`reqwest::Client` factory).

**Main entry points:**

- `ProviderRegistry` — dispatches a `SearchQuery` concurrently across every registered provider
- `MetadataProvider` — the trait every provider implements (`id`, `display_name`, `capabilities`,
  category-specific search methods)
- Real, working providers: `MusicBrainzProvider`, `SpotifyProvider`, `AppleMusicProvider`,
  `DeezerProvider` (music); `TmdbProvider`, `TheTvdbProvider`, `OmdbProvider`, `AppleTvProvider`,
  `ItunesStoreProvider` (video); `ApplePodcastsProvider` (podcasts); `IsrcProvider`,
  `EidrProvider`, `IswcProvider` plus `validate_isrc`/`validate_eidr`/`validate_iswc` (identifiers)
- Stub providers (compile and register, but carry no working API integration —
  `YouTubeMusicProvider`, `AmazonMusicProvider`, `PandoraProvider`, `TidalProvider`,
  `ShazamProvider`, `iHeartProvider`)
- `CredentialStore` — 4-tier credential resolution: env var → config map → OS keyring → local file
- `RateLimiterRegistry` / `ProviderRateLimiter` — per-provider token-bucket limiting (`governor`)
- `MatchScorer` / `rank_results` — weighted fuzzy match scoring (title/artist/album/year/ISRC)

This crate is fully implemented with 15 crate-level integration tests. **Nobody calls into it
yet**: `meedya lookup` (the CLI command that would use it) is a hand-written stub that always
returns exit code 3 without touching `mm-providers` at all — see [`cli.md`](cli.md).

## mm-cloud

**Purpose (per the crate's own module doc):** cloud storage monitoring across OneDrive, Google
Drive, Dropbox, MEGA and iCloud. Depends only on `mm-core`.

**Modules:** `traits`, `sync_manager`, `onedrive`, `google_drive`, `dropbox`, `mega` (stub — no
official API), `icloud` (stub — real iCloud access is native macOS FileProvider, not this crate).

**Main entry points:** `CloudProvider` trait, `SyncManager` (polling orchestration, conflict
resolution, per-provider `SyncState`), concrete `OneDriveProvider` / `GoogleDriveProvider` /
`DropboxProvider` / `MegaProvider` / `ICloudProvider`.

**This crate is scaffolding, verified directly in the source.** Every `CloudProvider` method on
every "real" backend (`authenticate`, `list_files`, `watch_changes`, …) returns a canned or empty
result rather than making an HTTP call — `onedrive.rs` says so outright:
`parse_drive_item(...) // In production this parses reqwest::Response JSON; here it is a stub`.
No file under `crates/mm-cloud/src/` other than `onedrive.rs` even imports `reqwest`. Consistent
with that, no consumer currently depends on this crate at all — it is absent from both
`mm-cli/Cargo.toml` and `mm-gtk/Cargo.toml`. `mm-gtk`'s own `cloud_panel.rs` UI exists but disables
its Connect button with the comment *"mm-cloud does not yet make a real network call for any
provider"* (issue #205).

## mm-update

**Purpose:** queries the GitHub Releases API and compares semver to report available updates.
Depends only on `mm-core` (for the shared User-Agent string builder).

**Main entry points:** `UpdateChecker::new(current_version)` / `.check()` → `Option<ReleaseInfo>`;
`UpdateError` (with `is_retryable()`).

**Not consumed anywhere yet.** There is no `meedya update` subcommand — `mm-cli/src/main.rs`'s
`Commands` enum has no such variant — and `mm-update` is not a dependency of `mm-cli`, `mm-ffi` or
`mm-gtk`. `mm-gtk/src/ui/main_window.rs` mentions it only in forward-looking comments ("wired in
mm-update integration, M9+"; "detected by the background update checker in mm-update"). The crate
itself is real and unit-tested — it is genuinely unwired, not internally stubbed.

## mm-export

**Purpose (per the crate's module doc):** export media library metadata to MySQL, MariaDB,
PostgreSQL, SQLite and SQL Server. Depends only on `mm-core`, plus direct (non-workspace-shared)
dependencies on `sqlx` (MySQL/Postgres/SQLite) and `tiberius` (SQL Server TDS).

**Modules:** `traits`, `schema`, `mysql`, `postgres`, `sqlite`, `mariadb`, `mssql`.

**Main entry points:** `DatabaseExporter` trait (`ensure_schema`, `export_file`, `export_batch`,
`record_rename`, `disconnect`), `SchemaBuilder` (DDL generation), and one struct per backend:
`SqliteExporter`, `MySqlExporter`, `MariaDbExporter`, `PostgresExporter`, `MssqlExporter`.

**Read this before assuming `mm-export` can export anything.** `SchemaBuilder`'s DDL generation is
real end-to-end — it produces genuine dialect-specific `CREATE TABLE` statements for all five
backends, and this is exactly what `meedya export --show-schema` exercises. Everything else is not:
despite declaring `sqlx` and `tiberius` as dependencies, **none of the five `DatabaseExporter`
implementations open a real connection or execute real SQL**. Every method in every backend
(`ensure_schema`, `export_file`, `record_rename`, `disconnect`) is implemented as
`std::future::ready(Ok(()))` after validating that input strings are non-empty — verified
identically across `sqlite.rs`, `mysql.rs`, `mariadb.rs`, `postgres.rs` and `mssql.rs`. The source
comments say so directly, e.g. `sqlite.rs`:

```rust
fn export_file(&self, row: &ExportRow) -> impl Future<Output = Result<(), ExportError>> {
    // Production: execute self.upsert_file_sql() then replace_tags_sql().
    // Stub: validate that required fields are present.
    ...
}
```

`meedya export` reports exit code 3 (`NOT_IMPLEMENTED`) for precisely this reason — see
[`cli.md`](cli.md) and issues #113 / #118.

## mm-server

**Purpose (per the crate's module doc):** serve the media library over HTTPS with JWT
authentication and byte-range streaming. Depends only on `mm-core`, plus direct dependencies on
`axum`, `tower` and `tower-http`.

**Modules:** `auth` (real — `ServerConfig`, `JwtService`, `Claims`, `UserRole`; JWT issue/validate
via the `jsonwebtoken` crate's actual `encode`/`decode`), `streaming` (real —
`RangeParser`, `MediaStreamer`, byte-range response construction), `routes` (route handler *logic*
as plain functions returning typed `ApiResponse<T>`: `handle_health`, `handle_login`,
`handle_library`, `handle_library_item`, `handle_search`, `handle_stream`, `handle_server_info`).

**Why this does not add up to an HTTP server:** `axum`, `tower` and `tower-http` are declared
specifically so a transport layer *could* be wired up, and the module doc comment says as much
("Transport layer ... is wired in the `meedya serve` CLI command"). It is not, anywhere. A
repository-wide search for `axum::Router`, `Router::new()`, `.serve(` or `TcpListener::bind`
returns zero matches. `auth` and `streaming` are real, tested logic; `routes` are real, tested
handler functions; but nothing ever builds a router from them or binds a socket. This is exactly
why there is no OpenAPI document for this project (see [`README.md`](README.md)). `meedya serve`
reports exit code 3 for the same reason `mm-export` does — see [`cli.md`](cli.md) and issues
#120–#126.

## mm-cli

**Purpose:** the `meedya` binary — MeedyaManager's primary user- and script-facing surface.
Binary-only crate (`src/main.rs`; no `src/lib.rs`, so nothing here is `pub` to other crates).
Depends on `mm-core`, `mm-providers`, `mm-export`, `mm-server` — **not** `mm-cloud` or `mm-update`.

There is no library API to document here; see [`cli.md`](cli.md) for the full command, exit-code
and `--json` surface instead.

## mm-ffi

**Purpose:** the foreign-function-interface bridge — UniFFI for Swift, `cbindgen`-generated C for
C#. Depends only on `mm-core`. Builds both a `cdylib` (linked by Swift/C#) and a plain `lib` (so
its own test binary, and in principle other Rust crates, can use it directly).

**Modules:** `types` (FFI-safe shared structs/enums: `TagEntry`, `RenamePreviewFfi`,
`AudioPropertiesFfi`, `ValidationResult`, `WatchEventFfi`, `MmFfiError`), `callbacks`
(`WatchCallback`, `ScanProgressCallback` — UniFFI callback interfaces), `uniffi_api`
(`#[uniffi::export]` functions — the Swift-facing surface), `capi` (`#[unsafe(no_mangle)] extern
"C"` functions — the C/C#-facing surface).

**The two bridges are not the same size.** `uniffi_api.rs` exports 19 functions plus 2 callback
interfaces; `capi.rs` exports 11. See [`ffi-c.md`](ffi-c.md) and [`ffi-swift.md`](ffi-swift.md) for
the full function-by-function breakdown and exactly which 9 UniFFI functions have no C-side
equivalent at all.

## mm-gtk

**Purpose:** the Linux desktop UI (GTK4 + Libadwaita). Written in Rust, so it consumes `mm-core`,
`mm-providers`, `mm-export` and `mm-server` as ordinary crate path dependencies — no FFI layer is
needed, since it never crosses a language boundary.

**Deliberately excluded from the Cargo workspace:** the root `Cargo.toml` has
`[workspace] exclude = ["crates/mm-gtk"]`, because its `gettextrs` dependency needs Linux-only
system libraries (`libintl`) that would otherwise break macOS/Windows CI resolution. Build and
test it explicitly:

```bash
cargo build --manifest-path crates/mm-gtk/Cargo.toml --release
cargo test  --manifest-path crates/mm-gtk/Cargo.toml
```

**Modules:** `app` (application struct, activation, window construction), `state` (non-GTK
application state, testable without a display), `ui` (component root: `main_window`, `scan_panel`,
`metadata_panel`, `rules_panel`, `settings_panel`, `cloud_panel`, and others).

**Public crate-root surface:** `APP_ID` (the reverse-DNS application identifier,
`ltd.MWBMpartners.MeedyaManager`) and `run_app()`, which blocks until the GTK application exits.
