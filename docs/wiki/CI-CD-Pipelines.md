# CI/CD Pipelines

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Overview of all 9 GitHub Actions workflows in MeedyaManager.

---

## Workflow Summary

| Workflow | File | Trigger | Description |
| -------- | ---- | ------- | ------------ |
| PR Gate | `pr-gate.yml` | `pull_request` on `main`, no path filter | Umbrella branch-protection check; detects changed paths and conditionally invokes the 4 platform CIs |
| Rust Core CI | `ci-rust.yml` | Push to `main` (`crates/**`) + `workflow_call` from PR Gate | Format, clippy, test, version-sync (3-OS matrix) |
| macOS CI | `ci-macos.yml` | Push to `main` (`macos/**`, `crates/mm-ffi/**`) + `workflow_call` | Build SwiftUI app on `macos-26` |
| Windows CI | `ci-windows.yml` | Push to `main` (`windows/**`, `crates/mm-ffi/**`) + `workflow_call` | Build WinUI 3 app on `windows-2022` |
| Linux CI | `ci-linux.yml` | Push to `main` (`crates/mm-gtk/**`) + `workflow_call` | Build GTK4 app under Xvfb |
| Lint Workflows | `lint.yml` | Push/PR touching `.github/workflows/**`, manual | `actionlint` against every workflow file |
| Version Bump | `version-bump.yml` | Manual (`workflow_dispatch`) | Bump version across all platform files |
| Release Build | `release.yml` | Tag push (`v*`) | Build 5 platform targets, SHA256 checksums, draft release |
| Security Audit | `audit.yml` | Weekly schedule + push to `main` | `cargo deny check` |
| Documentation | `docs.yml` | Push to `main` (`docs/**`, `help/**`, `crates/*/src/**`) | `cargo doc --no-deps --workspace` |

---

## PR Gate (`pr-gate.yml`)

The **single required status check on `main`** (see `.claude/CLAUDE.md` for the full rationale —
do not regress this pattern). Runs on every pull request to `main` with **no path filter**, so it
always reports.

**Steps:**

1. `changes` job — plain `git diff` between the PR base and head SHAs (no third-party action)
   detects which of `rust` / `macos` / `windows` / `linux` paths the PR touched. Any change under
   `.github/workflows/**` forces a full sweep of all four.
2. `rust`, `macos`, `windows`, `linux` jobs — each conditionally invokes the matching `ci-*.yml`
   as a reusable workflow (`workflow_call:`), only when its path matched.
3. `gate` job — `if: always()`; passes when every upstream job's result is `success` or
   `skipped`, fails if any is `failure`/`cancelled`.

**Windows is currently de-scoped from the gate** (tracked in issue #148 — a silent
`XamlCompiler.exe` failure under `windows-latest`). The `changes` job hardcodes `windows=false`
after path detection; `ci-windows.yml` still runs on direct pushes to `main` and via its own path
filter, so Windows regressions on `main` remain visible — they just don't block PRs.

---

## Rust Core CI (`ci-rust.yml`)

Reached two ways: `workflow_call:` from PR Gate on PRs, and a native `push:` trigger on direct
pushes to `main` (paths `crates/**`, `Cargo.toml`, `Cargo.lock`). It carries **no**
`pull_request:` trigger of its own — adding one would duplicate PR runs.

Runs on: Ubuntu, macOS, Windows (3-OS matrix, `rust: [stable]`)

**Steps:**

1. `cargo fmt --all --check` — verify formatting
2. `cargo clippy --workspace --all-targets --exclude mm-gtk -- -D warnings` — lint (`mm-gtk` needs
   GTK4 system libraries, only installed in `ci-linux.yml`)
3. `cargo test --workspace --exclude mm-gtk` — run workspace tests
4. `version-check` (separate job) — verifies `Cargo.toml` version matches
   `Package.appxmanifest` (MSIX, `X.Y.Z.0`) and `Info.plist` (`CFBundleShortVersionString`,
   `X.Y.Z`); does **not** check `CFBundleVersion`, or the Linux/WinGet package manifests

**Status badge:**

```text
![CI](https://github.com/MWBMPartners/MeedyaManager/actions/workflows/ci-rust.yml/badge.svg)
```

---

## macOS CI (`ci-macos.yml`)

Reached via `workflow_call:` from PR Gate, and `push:` to `main` (paths `macos/**`,
`crates/mm-ffi/**`).

Runs on: `macos-26` — required because `macos/Package.swift` declares
`swift-tools-version: 6.3`, which needs Xcode 26.3+ (Swift 6.3); `macos-26`'s default toolchain is
Xcode 26.4.1 / Swift 6.3, so no `xcode-select` step is needed.

**Steps:**

1. Print toolchain diagnostics (`xcodebuild -version`, `swift --version`, `sw_vers`)
2. Build `mm-ffi` (`cargo build -p mm-ffi --release`) — the UniFFI-based crate that produces
   `libmm_ffi.dylib` for the Swift bindings
3. `swift build -c release` — build the SwiftUI app via Swift Package Manager
4. `swift test` — run Swift unit tests (pure-logic helpers only; no WinUI/XAML-equivalent runtime
   dependency)

---

## Windows CI (`ci-windows.yml`)

Reached via `workflow_call:` from PR Gate (currently skipped there — see PR Gate above), and
`push:` to `main` (paths `windows/**`, `crates/mm-ffi/**`).

Runs on: **`windows-2022`** — deliberately pinned, *not* `windows-latest`. `windows-latest` now
resolves to `windows-2025` with a .NET 10 SDK host, under which `XamlCompiler.exe` fails silently
(exit code 1, no actionable error) even though the project targets `net8.0` and reference
assemblies were confirmed correct. `windows-2022`'s default .NET 8 SDK host is used to isolate the
regression (see issue #148).

**Steps:**

1. Build `mm-ffi` (`cargo build -p mm-ffi --release`) — the cbindgen/csbindgen crate producing
   `mm_ffi.dll` and the C header for C# P/Invoke
2. `dotnet restore` / `dotnet build` (diagnostic verbosity, binlog + log file captured)
3. On failure: dump any `XamlCompiler` `input.json`/`output.json` and the surrounding
   `msbuild.log` error context, and upload `msbuild.binlog`/`msbuild.log`/`build.out` as a
   7-day-retention artifact
4. `dotnet test --no-build` — run xUnit tests (logic-only; no live WinUI 3 XAML runtime on the
   runner)

---

## Linux CI (`ci-linux.yml`)

Reached via `workflow_call:` from PR Gate, and `push:` to `main` (path `crates/mm-gtk/**`).

Runs on: `ubuntu-latest`

**Steps:**

1. Install system dependencies: `libgtk-4-dev`, `libadwaita-1-dev`, `gettext` (the last for the
   `gettextrs` crate's GNU gettext bindings)
2. `cargo build --manifest-path crates/mm-gtk/Cargo.toml` — `mm-gtk` is **excluded from the
   Cargo workspace** (it needs Linux-only `gettextrs`), so it is built via its own manifest path
   rather than `--workspace`
3. `xvfb-run cargo test --manifest-path crates/mm-gtk/Cargo.toml` — run GTK4 tests under a
   virtual display

---

## Lint Workflows (`lint.yml`)

Lints the workflow files themselves with `actionlint` (pinned `v1.7.12`, checksum-verified by its
own official installer script), catching undefined `needs:` jobs, malformed `${{ }}`
expressions, invalid `runs-on`, and genuine shell errors inside `run:` blocks. The embedded
`shellcheck` runs at `SHELLCHECK_OPTS=--severity=error`, so cosmetic style/info/warning lint never
fails the build — only real breakage does.

**Triggers:** `push` and `pull_request` when `.github/workflows/**` changes, plus manual
`workflow_dispatch`. It costs no CI minutes on ordinary code pushes.

---

## Version Bump (`version-bump.yml`)

Manual trigger via `workflow_dispatch`.

**Inputs:**

| Input | Description | Default |
| ----- | ------------ | ------- |
| `version` | Explicit version string (e.g. `2.0.0-alpha.2`); leave blank to use `bump_type` | — |
| `bump_type` | `patch`, `minor`, `major`, `pre-alpha`, `pre-beta`, `pre-rc` (ignored if `version` is set) | `patch` |
| `create_tag` | Create and push a git tag after bumping | `false` |
| `create_pr` | Open a pull request for the bump (commits directly to the trigger branch if `false`) | `true` |

**What it updates:**

- `Cargo.toml` `[workspace.package].version`
- `windows/MeedyaManager/Package.appxmanifest` `Identity.Version` (MSIX 4-part, pre-release
  stripped, `.0` appended)
- `macos/MeedyaManager/Info.plist` `CFBundleShortVersionString` (pre-release stripped)
- `docs/changelog.md` — inserts a new `## [vX.Y.Z] — <date>` section (after `## [Unreleased]` if
  present, otherwise before the first existing version section)

It does **not** touch the Linux package manifests (`snapcraft.yaml`, `linux/deb/control`,
`*.metainfo.xml`) or the WinGet manifest — those must be kept in sync by hand.

---

## Release Build (`release.yml`)

Triggered by pushing a `v*` tag (e.g. `v1.0.0`).

**Architecture:**

```text
prepare ──┬── release-macos-arm64
          ├── release-windows-x64
          ├── release-windows-arm64
          ├── release-linux-x64
          └── release-linux-arm64
                      │
              create-github-release (draft)
```

**Each build job:**

1. Runs `cargo build --profile dist` (full hardening — see the Release Process wiki)
2. Packages the binary for its platform
3. Generates a SHA256 checksum
4. Uploads the artifact to the workflow run

The Linux jobs attempt a `.deb` (`linux/deb/build-deb.sh`) and an AppImage
(`linux/appimage/build-appimage.sh`), each falling back to a `::warning::` and continuing if the
packaging tool (`dpkg-deb`/`appimagetool`) is unavailable, plus a plain `.tar.gz` of the raw
binaries. **There is no Flatpak build step anywhere in `release.yml`.**

**create-github-release job:**

1. Collects all artifacts
2. Concatenates checksums into `SHA256SUMS.txt`
3. Creates a **draft** GitHub Release with all artifacts and auto-generated notes

---

## Security Audit (`audit.yml`)

Runs weekly (Monday 09:00 UTC) and on every push to `main`. It has **no `pull_request:` trigger**.

**Tool:** `cargo deny check` only — licence, security-advisory, and duplicate-crate checks driven
by `deny.toml`. A separate `cargo audit` step existed previously but was removed: it was
redundant with `cargo-deny`'s own advisories check and had runner-side advisory-database-fetch
issues (see the workflow's own comment and the historical `#156`/`#157` issue trail).

**`deny.toml` policies:**

- Licences: allow `MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-DFS-2016`
- Reject: `GPL-3.0-only` (incompatible with GPL-2.0-or-later)
- Duplicates: warn on duplicate crates at different versions

---

## Documentation (`docs.yml`)

Runs on push to `main` touching `docs/**`, `help/**`, or `crates/*/src/**`.

**Steps:**

1. Install GTK4/Libadwaita/gettext system dependencies (needed so `mm-gtk` is included in the doc
   build)
2. `cargo doc --no-deps --workspace` — build API documentation

**There is no GitHub Pages deployment step.** The workflow only builds the docs inside the runner;
nothing publishes them anywhere. A `https://mwbmpartners.github.io/MeedyaManager/` URL does not
exist.

---

## Secrets Required

| Secret | Used by | Description |
| ------ | ------- | ------------ |
| `APPLE_CERT_P12` | `release.yml` | macOS Developer ID certificate (base64 PFX) |
| `APPLE_CERT_PASSWORD` | `release.yml` | Password for the P12 certificate |
| `APPLE_ID` | `release.yml` | Apple ID for notarisation |
| `APPLE_PASSWORD` | `release.yml` | App-specific password for notarisation |
| `APPLE_TEAM_ID` | `release.yml` | Apple Developer Team ID |
| `WINDOWS_CERT_PFX` | `release.yml` | Windows code signing certificate (base64 PFX) |
| `WINDOWS_CERT_PASSWORD` | `release.yml` | Password for the PFX certificate |

Without these secrets, `release.yml` will build **unsigned** artifacts.
All other workflows run without secrets.

---

*Last updated: 2026-09-03*
