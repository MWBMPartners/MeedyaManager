# MeedyaManager — Claude Code Project Instructions

> **(C) 2025-2026 MWBM Partners Ltd**

## Project Identity

- **Name:** MeedyaManager
- **Type:** Cross-platform media file manager and auto-organizer
- **Languages:** Rust (core engine + CLI + Linux GTK4 UI) + Swift (macOS UI) + C# (Windows UI)
- **Licence:** GPL-2.0-or-later
- **Copyright:** MWBM Partners Ltd
- **Platforms:** Windows (x64/ARM), macOS (Apple Silicon only), Linux (x64/ARM)

## Key Architecture Decisions

- **Rust core:** Shared library (`mm-core`) consumed by all platform UIs via FFI
- **FFI layer:** UniFFI (generates Swift bindings for macOS), cbindgen/csbindgen (generates C headers for Windows C# P/Invoke)
- **macOS GUI:** SwiftUI with Liquid Glass on macOS 26+ (falls back to standard vibrancy on older versions)
- **Windows GUI:** WinUI 3 (C# / WinAppSDK 1.6) with Mica backdrop
- **Linux GUI:** GTK4 via `gtk4-rs` + `libadwaita` (direct Rust, no FFI needed)
- **Cargo workspace:** 8 crates — `mm-core`, `mm-providers`, `mm-cloud`, `mm-export`, `mm-server`, `mm-cli`, `mm-ffi`, `mm-gtk`
- **Key Rust crates:**
  - `lofty` — Audio/video metadata read/write
  - `notify` — Cross-platform file system watcher
  - `clap` — CLI argument parsing
  - `serde` / `json5` — Config serialization (JSON5 format)
  - `tokio` — Async runtime
  - `reqwest` — HTTP client for metadata provider APIs
  - `sqlx` — Async database driver (MySQL, PostgreSQL, SQLite)
  - `tiberius` — SQL Server driver (default-features = false for rustls compat)
  - `tracing` — Structured logging
  - `thiserror` / `anyhow` — Error handling
  - `uniffi` — FFI binding generation for Swift
  - `governor` — Rate limiting for API providers
- **Config format:** JSON5 (`settings.json5`) with `.env` fallback for secrets

## Coding Standards

- **ALL code** must include detailed comments/annotations (every line where possible)
- **Copyright header** in every source file:

  ```rust
  // (C) 2025-{current_year} MWBM Partners Ltd
  ```

  ```swift
  // (C) 2025-{current_year} MWBM Partners Ltd
  ```

  ```csharp
  // (C) 2025-{current_year} MWBM Partners Ltd
  ```

- **Copyright year** must be automated (start 2025, end current year)
- Code formatting: `rustfmt` for Rust, SwiftFormat for Swift, dotnet-format for C#
- Emojis are welcome in documentation and UI
- Use the canonical name **MeedyaManager** (not MetaMancer) everywhere

## Documentation Requirements

- **Project_Plan.md** — Full project plan (root)
- **PROJECT_STATUS.md** — Current status tracker (root)
- **README.md** — Project overview with quick start (root)
- **docs/CHANGELOG.md** — Detailed change log with dates
- **docs/ROADMAP.md** — Milestone timeline
- **Dev_Notes.md** — Developer notes (versioning, release process, CI/CD)
- **help/** — User documentation
- **.claude/** — This file + project brief for session continuity
- **GitHub Wiki** — Version Management, Release Process, CI/CD Pipelines
- **All .md files** must be updated after every code change

## Milestone Order

1. M0 — Repository Setup & Scaffolding (Complete)
2. M1 — Core Engine (Complete — 217 tests)
3. M2 — Rule Engine (Complete — 182 tests, 399 total)
4. M3 — CLI (Complete — 45 tests, 444 total)
5. M4 — FFI Layer & Native UI Shells (Issues #63-72)
6. M5 — Metadata Lookup Providers (Issues #73-84)
7. M6 — Full Native UI (Issues #85-93)
8. M7 — Cloud Storage Monitoring (Issues #94-102)
9. M8 — Packaging & Public Release (Issues #103-111)
10. M9 — Database Export (Issues #112-119)
11. M10 — Secure Media Server (Issues #120-127)

## Version Management

- **Single source of truth:** `Cargo.toml` `[workspace.package].version`
- **Automated bumping:** `version-bump.yml` GitHub Actions workflow
- **CI sync check:** `ci-rust.yml` verifies all platform files match
- **Platform mapping:** semver → MSIX 4-part (2.0.0.0), CFBundle 3-part (2.0.0)
- See `Dev_Notes.md` for full details

## API Key Policy

- Developer-only keys in `.env` (git-ignored)
- Per-service `include_in_build` toggle for bundling decisions
- Users can always override with their own keys
- Never bundle keys where ToS prohibits shared use

## Important Context Files

- `.claude/ProjectBrief_Chat.claude` — Full project brief from user
- `Project_Plan.md` — Comprehensive project plan
- `PROJECT_STATUS.md` — Current progress
- `docs/ROADMAP.md` — Milestone details
- `Dev_Notes.md` — Developer notes and release process

## Packaging & Distribution

- **Cargo** builds native Rust binaries (no runtime dependency)
- **Swift Package Manager** builds macOS SwiftUI app
- **MSBuild/.NET 8** builds Windows WinUI 3 app (MSIX package)
- **Cargo** builds Linux GTK4 binary (Flatpak/Snap/AppImage/.deb)
- App is fully self-contained — users need ZERO pre-installed software
- All packages include SHA256 checksums
- Release workflow generates draft GitHub Releases with artifacts

## Git & CI/CD

- **9 GitHub Actions workflows:**
  - `pr-gate.yml` — **Umbrella for PR branch protection.** No path filter; runs on every PR, detects changed paths, conditionally invokes the 4 platform CIs as reusable workflows, aggregates results in a `Gate` job. The `Gate` context is the single required check on `main`. See "Branch protection" section below.
  - `ci-rust.yml` — Cargo fmt + clippy + test + version-sync (3-OS matrix). **Reusable** (`workflow_call:`) — invoked by `pr-gate.yml` for PRs; native `push:` trigger still fires on direct main pushes.
  - `ci-macos.yml` — Build mm-ffi + SwiftUI app (macos-15). **Reusable.**
  - `ci-windows.yml` — Build mm-ffi + WinUI 3 app (windows-latest). **Reusable.**
  - `ci-linux.yml` — Build mm-gtk with GTK4/Libadwaita (ubuntu-latest). **Reusable.**
  - `version-bump.yml` — Automated version bumping across all files (manual trigger)
  - `release.yml` — 5-platform release builds + checksums + GitHub Release (tag trigger)
  - `audit.yml` — cargo-deny + cargo-audit (weekly + push)
  - `docs.yml` — cargo doc generation
- Platform packages: Windows (MSIX), macOS (.dmg/.tar.gz), Linux (Flatpak/Snap/AppImage/.deb)
- `.gitignore` covers OS files, Rust `target/`, IDE files, secrets, build artifacts
- Python v1.x archived at tag `v1.5-M6-python-final`
- **Every task** must have a GitHub Issue created BEFORE work begins and closed AFTER verification
- **Commit but do NOT push** — user pushes manually (exception: pushing a *new feature branch* is OK once the user has explicitly asked for a PR)

## Branch protection on `main` — umbrella PR Gate pattern

`main` is protected by the **"Protect main branch"** ruleset (id `14829223`). The required-status-check rule lists exactly one context: the `Gate` job from `pr-gate.yml` (verify the exact recorded name with `gh pr checks <PR#>` on a fresh PR; orphan contexts soft-lock every future PR — see global CLAUDE.md CI/CD lesson #1).

**Why this pattern (do not regress):**

- Per-platform CIs (`ci-rust`, `ci-macos`, `ci-windows`, `ci-linux`) have **path filters** to keep CI cheap. Making them directly required would soft-lock any PR that doesn't touch their paths (the check never reports = never passes = blocked forever).
- The umbrella `pr-gate.yml` runs on every PR with **no path filter**. It detects which platform code changed (plain `git diff` against the PR base — no third-party action) and conditionally invokes the 4 platform workflows as reusable (`workflow_call:`) jobs. The final `Gate` job uses `if: always()` and treats `success` and `skipped` as OK, only failing if a platform job actually `failure`d/`cancelled`.
- A docs-only PR → all platforms skip → Gate passes immediately, no CI cost.
- A `crates/mm-gtk/` PR → only linux runs → Gate passes if linux passed.
- A `crates/mm-ffi/` PR → rust + macos + windows run → Gate passes if all three passed.

**Hard rules — do not violate without explicit user go-ahead:**

1. **Never add another required status check context** without first running it on a real PR and capturing the exact recorded context name. New orphans = soft-lock.
2. **Never add a `pull_request:` trigger to `ci-rust/macos/windows/linux.yml`.** They MUST be reached only via `workflow_call:` from `pr-gate.yml`, otherwise PR runs duplicate.
3. **Keep `pr-gate.yml` without path filters.** Adding one would re-introduce the soft-lock problem.
4. **Keep the path-detection logic in `pr-gate.yml`'s `changes` job in sync with each `ci-*.yml`'s `push:` `paths:`.** Drift means PR Gate triggers a platform whose own `push:` filter would've ignored the change, or vice versa.
5. **If a new platform CI is added** (e.g. `ci-android.yml`), it must follow the same template: `workflow_call:` trigger, no `pull_request:`, then add a corresponding `if`-gated job + a new `needs:` entry in `gate` inside `pr-gate.yml`.

## Post-PR housekeeping — full dev-cache cleanup

After a PR has been **successfully created** (URL returned), run a full dev-cache cleanup covering the workspace plus Rust global caches. Cache is fully regeneratable; the user accepts the cold-build cost (~5–15 min on next build for this 8-crate workspace) in exchange for disk-pressure relief.

**When:** *after* the PR is created — never before or during, because the cleanup invalidates the `cargo check`/`cargo test` caches the PR flow itself relies on.

**Commands (run from repo root, in order):**

```bash
# 1. Workspace Rust cache (all 8 crates' target/)
cargo clean

# 2. Swift Package Manager build dirs under macos/
find macos -type d -name '.build' -prune -exec rm -rf {} +

# 3. C# WinUI 3 build output (scoped to known dirs — do NOT bare-find bin/obj)
rm -rf windows/MeedyaManager/bin windows/MeedyaManager/obj \
       windows/MeedyaManager.Tests/bin windows/MeedyaManager.Tests/obj

# 4. Rust global caches (affects ALL Rust projects on this machine — user accepts this)
rm -rf ~/.cargo/registry/cache ~/.cargo/registry/src \
       ~/.cargo/git/checkouts ~/.cargo/git/db
```

**Scope is fixed at "workspace + Rust global".** Do not silently extend to Xcode `DerivedData`, NuGet `~/.nuget/packages`, or other toolchain caches without re-confirming with the user — those were explicitly excluded on 2026-05-24.

**If the C# project layout changes** (a new project added under `windows/`), update the `rm` list in step 3 rather than reaching for `find -name bin -o -name obj` — those names are too common to wildcard safely.

**After cleanup,** briefly mention to the user that the next build will be cold so they're not surprised.
