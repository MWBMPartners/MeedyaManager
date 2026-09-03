# 📍 ROADMAP — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd**
>
> 🎧📁 Cross-platform media manager and auto-organizer — Rust core + native UIs

---

## 🔄 Current state (pre-release)

MeedyaManager is a complete rewrite from Python to **Rust** with **platform-native UIs** (SwiftUI
on macOS, WinUI 3 on Windows, GTK4 on Linux). The workspace version in `Cargo.toml`
(`[workspace.package].version`) is **1.3.0**.

**No public release has been cut for this version.** The only GitHub release that exists is
*"MetaMancer v1.0-M1"* (2025-06-16, under the project's pre-rename name), and the only git tag is
`v1.0-M1`. There is no `v1.5-M6-python-final` tag, nor any other `v0.x.0`/`v1.x.0` tag — a
per-milestone `v0.x.0`/`v1.0.0` versioning scheme was planned at one point but was never actually
applied to `Cargo.toml` or tagged in git. Versions `1.3.1` and `1.3.2` appear in
`docs/changelog.md` but were likewise never set in `Cargo.toml` — see that file's `[Unreleased]`
section for the correction. See open issue #214 to actually cut a real pre-release.

---

## 📈 Milestone Timeline

**"Complete" below means the GitHub issues for that milestone are closed — not that the
feature works end-to-end.** Several milestones were closed on the strength of scaffolding: a
typed struct, a DSN parser, or a route table, without the real I/O behind it. Where that's the
case it's called out explicitly. See `.claude/HANDOFF.md` §2 for the full verified ground truth.

| # | Milestone | GitHub state | Issues | Reality |
| --- | --------- | ------------- | ------ | -------- |
| M0 | 🔧 Repository Setup & Scaffolding | ✅ Closed | #19-39 | Done — Cargo workspace, native scaffolds, CI/CD |
| M1 | ⚙️ Core Engine | ✅ Closed | #40-51 | Done — config, classify, metadata, watcher, renamer, companion, state, logging, health |
| M2 | 📐 Rule Engine | ✅ Closed | — | Done — lexer, parser, evaluator, 24 template functions, tag mappings |
| M3 | ⌨️ CLI | ✅ Closed | #52-62 | Done — `scan`, `debug`, `edit`, `rule`, `watch`, `lookup`, `config`, `report-bug` commands exist, though `lookup` itself is a stub (below) |
| M4 | 🖥️ FFI Layer & Native UI Shells | ✅ Closed | #63-72 | Done — UniFFI (Swift), cbindgen (C#), SwiftUI/WinUI 3/GTK4 shells |
| M5 | 🔍 Metadata Lookup Providers | Mixed — open + closed | #73-84 | **Partial.** 13 of 19 registered providers make real HTTP calls (MusicBrainz, Spotify, Apple Music, Deezer, TMDb, TheTVDB, OMDb/IMDb, Apple TV, iTunes Store, Apple Podcasts, ISRC, EIDR, ISWC); 6 are deliberate stubs (YouTube Music, Amazon Music, Pandora, Tidal, Shazam, iHeart — all `enabled_default: false`). The CLI's own `meedya lookup` command is **still a stub** printing *"Provider support is coming in M5"* (`crates/mm-cli/src/commands/lookup.rs:85`); the only production caller of the provider registry is `mm-gtk`'s lookup panel, which wires up MusicBrainz only |
| M6 | 🎨 Full Native UI | ✅ Closed | #85-93 | Done — lookup panel, rule builder, cover art, drag-and-drop, settings save, theming |
| M7 | ☁️ Cloud Storage Monitoring | Reopened this session | #94-102 | **Status: not yet implemented.** No real network calls; OAuth flows exist only as comments (e.g. `crates/mm-cloud/src/onedrive.rs:90`) |
| M8 | 📦 Packaging & Public Beta | ✅ Closed | #103-111 | Partial — `mm-update` crate and packaging scripts exist; see `docs/wiki/Release-Process.md` for what actually builds (no Flatpak step exists despite earlier plans) |
| M9 | 🗄️ Database Export | Reopened this session | #112-119 | **Status: not yet implemented.** `sqlx`/`tiberius` are declared dependencies; no connection pool is ever created and no SQL is ever executed (`crates/mm-export/src/sqlite.rs:30`) |
| M10 | 🌐 Secure Media Server + Public Release | Reopened this session | #120-127 | **Status: not yet implemented.** `mm-server` never builds an axum router (no `.route(` call exists anywhere in the crate); `crates/mm-cli/src/commands/serve.rs:337-342` prints *"Server stub: exiting cleanly"*. The repository has **zero `.html` files**, so #124's web frontend has no deliverable |

**Test totals (verified by actually running the suite, not by summing docstrings):**

| Crate | LOC | Test functions |
| ----- | --- | --------------- |
| mm-core | 14,830 | 512 |
| mm-providers | 10,552 | 386 |
| mm-gtk | 5,043 | 67 |
| mm-cli | 4,517 | 73 |
| mm-cloud | 2,580 | 117 |
| mm-export | 2,474 | 111 |
| mm-server | 1,956 | 74 |
| mm-ffi | 1,626 | 23 |
| mm-update | 605 | 29 |
| **Total** | **~44,183** | **1,392** |

`cargo test --workspace` (`mm-gtk` excluded — it needs Linux-only `gettextrs` and is not a
workspace member) currently reports **1,240 passing, 0 failing**. That is higher than the 1,207 of
earlier in the day because rewiring the MusicBrainz, ISRC and ISWC providers onto the hardened
`musicbrainz` module (issue #198) added 33 provider tests. The remaining gap between 1,392 test
*functions* and 1,240 *passing* is `mm-gtk`'s 67 (run separately via its own manifest path) plus
Swift/.NET tests that aren't part of `cargo test` at all.

---

## 📋 Notes

- All builds produce platform-native binaries — no runtime dependencies for end users
- GitHub Actions CI covers Rust (3-OS matrix), SwiftUI (macOS), WinUI 3 (Windows), GTK4 (Linux),
  gated through the `pr-gate.yml` umbrella workflow (see `docs/wiki/CI-CD-Pipelines.md`)
- API keys remain developer-only in `.env` (git-ignored); users can override with their own keys
- Documentation (`.md` files) is meant to be updated with every milestone — this file is one of
  the pages that had drifted, corrected 2026-09-03

---

## 💻 Platform Support

| OS | Architecture | Native UI |
| ---- | ------------- | --------- |
| 🍎 macOS | Apple Silicon (arm64) | SwiftUI |
| 🪟 Windows | x64, ARM64 | WinUI 3 (C# / WinAppSDK) |
| 🐧 Linux | x86_64, ARM64 | GTK4 (Rust `gtk4` crate) |

---

> 📝 *This roadmap is maintained alongside the codebase. For current status, see [PROJECT_STATUS.md](../PROJECT_STATUS.md) and `.claude/HANDOFF.md` for the fullest verified detail.*
>
> *Last updated: 2026-09-03*
