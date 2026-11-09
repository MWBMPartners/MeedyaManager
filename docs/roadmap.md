# 📍 ROADMAP — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd**
>
> 🎧📁 Cross-platform media manager and auto-organizer — Rust core + native UIs

---

## 🔄 Current state (pre-release)

MeedyaManager is a complete rewrite from Python to **Rust** with **platform-native UIs** (SwiftUI
on macOS, WinUI 3 on Windows, GTK4 on Linux). The workspace version in `Cargo.toml`
(`[workspace.package].version`) is **`1.4.0-alpha.1`** — previously `1.3.0`, bumped by a Round
1/Round 2 alpha-readiness pass (issue #214) into the project's first-ever semver pre-release.

**No public release has been cut for this version yet.** The only GitHub release that exists is
*"MetaMancer v1.0-M1"* (2025-06-16, under the project's pre-rename name), and the only git tag is
`v1.0-M1`. There is no `v1.5-M6-python-final` tag, nor any other `v0.x.0`/`v1.x.0` tag — a
per-milestone `v0.x.0`/`v1.0.0` versioning scheme was planned at one point but was never actually
applied to `Cargo.toml` or tagged in git. Versions `1.3.1` and `1.3.2` appear in
`docs/changelog.md` but were likewise never set in `Cargo.toml` — both are folded into the
`[v1.4.0-alpha.1]` entry. No tag has been pushed for `1.4.0-alpha.1` yet; that follows a green
`release.yml` `workflow_dispatch` dry run (added by issue #202) and the merge to `alpha`.

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
| M3 | ⌨️ CLI | ✅ Closed | #52-62 | Done — `scan`, `debug`, `edit`, `rule`, `watch`, `lookup`, `config`, `report-bug` commands exist, though `lookup` itself is a stub (below), now exiting `3` (`NOT_IMPLEMENTED`) instead of `0`; `scan --execute`'s data-loss bug (#201) and a batch of smaller correctness bugs (#206) are fixed |
| M4 | 🖥️ FFI Layer & Native UI Shells | ✅ Closed | #63-72 | Done — UniFFI (Swift), cbindgen (C#), SwiftUI/WinUI 3/GTK4 shells |
| M5 | 🔍 Metadata Lookup Providers | Mixed — open + closed | #73-84 | **Partial.** 13 of 19 registered providers make real HTTP calls (MusicBrainz, Spotify, Apple Music, Deezer, TMDb, TheTVDB, OMDb/IMDb, Apple TV, iTunes Store, Apple Podcasts, ISRC, EIDR, ISWC); 6 are deliberate stubs (YouTube Music, Amazon Music, Pandora, Tidal, Shazam, iHeart — all `enabled_default: false`). The CLI's own `meedya lookup` command is **still a stub** — it now prints "Metadata lookup is not available in this alpha (see issue #83)." and exits `3` rather than `0` (`crates/mm-cli/src/commands/lookup.rs`); the only production caller of the provider registry is `mm-gtk`'s lookup panel, which wires up MusicBrainz only |
| M6 | 🎨 Full Native UI | ✅ Closed | #85-93 | Done — lookup panel, rule builder, cover art, drag-and-drop, settings save, theming |
| M7 | ☁️ Cloud Storage Monitoring | Reopened this session | #94-102 | **Status: not yet implemented — the Cloud UI tab is now marked preview-only with its controls disabled (#205).** No real network calls; OAuth flows exist only as comments (e.g. `crates/mm-cloud/src/onedrive.rs:90`) |
| M8 | 📦 Packaging & Public Beta | ✅ Closed | #103-111 | Partial — `mm-update` crate and packaging scripts exist; `release.yml` is now capable of producing a usable alpha build (#202); see `docs/wiki/Release-Process.md` for what actually builds (no Flatpak step exists despite earlier plans) |
| M9 | 🗄️ Database Export | Reopened this session | #112-119 | **Status: not yet implemented — the Export UI tab is now marked preview-only, and `meedya export` exits `3` instead of fabricating success (#205).** `sqlx`/`tiberius` are declared dependencies; no connection pool is ever created and no SQL is ever executed (`crates/mm-export/src/sqlite.rs:30`) |
| M10 | 🌐 Secure Media Server + Public Release | Reopened this session | #120-127 | **Status: not yet implemented — the Server UI tab is now marked preview-only, and `meedya serve` exits `3` instead of fabricating success (#205).** `mm-server` never builds an axum router (no `.route(` call exists anywhere in the crate). The repository has **zero `.html` files**, so #124's web frontend has no deliverable |

**Test totals:**

| Crate | LOC | Test functions |
| ----- | --- | --------------- |
| mm-core | 14,830 | 512 baseline, grown by Round 1 (static count at `9f3719b`: 565) |
| mm-providers | 10,552 | 289 — unchanged by Round 1/2 (untouched by either round's diff) |
| mm-gtk | 5,043 | 67 |
| mm-cli | 4,517 | 88 — up from 73, verified by `cargo test -p mm-cli` in commit `44b9ad8` |
| mm-cloud | 2,580 | 117 — unchanged (confirmed in the #200 commit message) |
| mm-export | 2,474 | 111 |
| mm-server | 1,956 | 74 |
| mm-ffi | 1,626 | 23 baseline, grown by Round 1 (static count at `9f3719b`: 25) |
| mm-update | 605 | 29 |
| **Total** | **~44,183** | **~1,365 static count** (excluding `mm-gtk`: ~1,298) — the workspace `cargo test` total below is authoritative |

`cargo test --workspace` (`mm-gtk` excluded — it needs Linux-only `gettextrs` and is not a
workspace member) reports **1,304 passing, 0 failing** at `9f3719b` (per that commit's own
verification), up from the 1,240 recorded earlier in the Round 1/2 work. LOC and the mm-core/
mm-ffi test counts above were not independently re-measured this pass; they are a static count of
`#[test]`/`#[tokio::test]` attributes in the source, offered as an approximation, not a fresh
`cargo test` run. The small remaining gap between the ~1,298 static count (workspace, `mm-gtk`
excluded) and the 1,304 authoritative figure is most likely doctests, which `cargo test`
exercises but a static `#[test]`-attribute count does not — this repo has shown exactly this kind
of small function-count/doctest gap before (see `docs/changelog.md`'s MusicBrainz section).
Swift/.NET UI tests are separate again and aren't part of `cargo test` at all.

---

## 📋 Notes

- All builds produce platform-native binaries — no runtime dependencies for end users
- GitHub Actions CI (10 workflows, including `lint.yml`) covers Rust (3-OS matrix, pinned
  toolchain `1.98.0`), SwiftUI (macOS), WinUI 3 (Windows), GTK4 (Linux), gated through the
  `pr-gate.yml` umbrella workflow, which now runs for PRs to `main`, `alpha` and `beta` (issue
  #204) — see `docs/wiki/CI-CD-Pipelines.md`
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
