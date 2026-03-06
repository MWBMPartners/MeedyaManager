# CI/CD Pipelines

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Overview of all 8 GitHub Actions workflows in MeedyaManager.

---

## Workflow Summary

| Workflow | File | Trigger | Description |
|----------|------|---------|-------------|
| Rust Core CI | `ci-rust.yml` | Push/PR on `main` (crates/**) | Format, clippy, test, version-sync (3-OS matrix) |
| macOS CI | `ci-macos.yml` | Push/PR on `main` (macos/**) | Build SwiftUI app on `macos-14` |
| Windows CI | `ci-windows.yml` | Push/PR on `main` (windows/**) | Build WinUI 3 app on `windows-latest` |
| Linux CI | `ci-linux.yml` | Push/PR on `main` (crates/mm-gtk/**) | Build GTK4 app under Xvfb |
| Version Bump | `version-bump.yml` | Manual (`workflow_dispatch`) | Bump version across all platform files |
| Release Build | `release.yml` | Tag push (`v*`) | Build all 5 platforms, SHA256 checksums, draft release |
| Security Audit | `audit.yml` | Weekly schedule + push to `main` | `cargo deny` + `cargo audit` |
| Documentation | `docs.yml` | Push to `main` (crates/**) | Generate and publish `cargo doc` |

---

## Rust Core CI (`ci-rust.yml`)

Runs on: Ubuntu, macOS, Windows (3-OS matrix)

**Steps:**
1. `cargo fmt --all -- --check` — verify formatting
2. `cargo clippy --all-targets -- -D warnings` — lint (deny all warnings)
3. `cargo test --all` — run all workspace tests
4. `version-check` — verify Cargo.toml version matches Info.plist + Package.appxmanifest

**Status badge:**

```text
![CI](https://github.com/MWBMPartners/MeedyaManager/actions/workflows/ci-rust.yml/badge.svg)
```

---

## macOS CI (`ci-macos.yml`)

Runs on: `macos-14` (Apple Silicon)

**Steps:**
1. Build `mm-ffi` (UniFFI bindings for Swift)
2. Copy `libmm_ffi.dylib` to `macos/MeedyaManager/`
3. `swift build` — build SwiftUI app via Swift Package Manager
4. `swift test` — run all Swift unit tests

---

## Windows CI (`ci-windows.yml`)

Runs on: `windows-latest`

**Steps:**
1. Build `mm-ffi` (cbindgen headers + DLL for C# P/Invoke)
2. Copy `mm_ffi.dll` + `mm_ffi.h` to `windows/MeedyaManager/`
3. `dotnet build` — build WinUI 3 project
4. `dotnet test` — run all xUnit tests

---

## Linux CI (`ci-linux.yml`)

Runs on: `ubuntu-latest`

**Steps:**
1. Install system dependencies: `libgtk-4-dev`, `libadwaita-1-dev`
2. Start Xvfb (virtual display for GTK widget tests)
3. `cargo build -p mm-gtk` — build GTK4 binary
4. `cargo test -p mm-gtk` — run GTK4 tests under Xvfb

---

## Version Bump (`version-bump.yml`)

Manual trigger via `workflow_dispatch`.

**Inputs:**
| Input | Description | Default |
|-------|-------------|---------|
| `version` | Explicit version string (e.g., `1.0.0`) | — |
| `bump_type` | Increment type: `major`, `minor`, `patch` | — |
| `create_tag` | Create and push a git tag | `false` |
| `create_pr` | Open a pull request for the bump | `true` |

**What it updates:**
- `Cargo.toml` `[workspace.package].version`
- `macos/MeedyaManager/Info.plist` `CFBundleShortVersionString`
- `windows/MeedyaManager/Package.appxmanifest` `Identity.Version`

---

## Release Build (`release.yml`)

Triggered by pushing a `v*` tag (e.g., `v1.0.0`).

**Architecture:**

```text
prepare ──┬── build-macos-arm64
          ├── build-windows-x64
          ├── build-windows-arm64
          ├── build-linux-x64
          └── build-linux-arm64
                      │
              create-github-release (draft)
```

**Each build job:**
1. Runs `cargo build --profile dist` (full hardening — see Release Process wiki)
2. Packages the binary (`create-dmg`, WinAppSDK MSIX, `tar.gz`)
3. Generates SHA256 checksum
4. Uploads artifact to the workflow run

**create-github-release job:**
1. Collects all artifacts
2. Concatenates checksums into `SHA256SUMS.txt`
3. Creates a draft GitHub Release with all artifacts and auto-generated notes

---

## Security Audit (`audit.yml`)

Runs weekly (Sunday midnight) and on every push to `main`.

**Tools:**
- `cargo deny check` — license, security, and duplicate crate checks (`deny.toml`)
- `cargo audit` — cross-references against the RustSec advisory database

**`deny.toml` policies:**
- Licences: allow `MIT`, `Apache-2.0`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-DFS-2016`
- Reject: `GPL-3.0-only` (incompatible with GPL-2.0-or-later)
- Duplicates: warn on duplicate crates at different versions

---

## Documentation (`docs.yml`)

Runs on push to `main` (Rust crate files only).

**Steps:**
1. `cargo doc --no-deps --all-features` — build API documentation
2. Deploy to GitHub Pages at `https://mwbmpartners.github.io/MeedyaManager/`

---

## Secrets Required

| Secret | Used by | Description |
|--------|---------|-------------|
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

*Last updated: 2026-03-05*
