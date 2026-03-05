# MeedyaManager — Developer Notes

> **(C) 2025-2026 MWBM Partners Ltd**

## Table of Contents

- [Version Management](#version-management)
- [How to Cut a Release](#how-to-cut-a-release)
- [Version Format Conventions](#version-format-conventions)
- [Platform Version Mapping](#platform-version-mapping)
- [CI/CD Pipeline Overview](#cicd-pipeline-overview)
- [GitHub Projects Workflow](#github-projects-workflow)

---

## Version Management

### Single Source of Truth

The **canonical version** lives in the root `Cargo.toml` under `[workspace.package].version`. All other version locations are derived from it:

| File | Format | Example |
|------|--------|---------|
| `Cargo.toml` `[workspace.package]` | Full semver | `2.0.0-alpha.2` |
| `windows/.../Package.appxmanifest` `Identity.Version` | 4-part (no pre-release) | `2.0.0.0` |
| `macos/.../Info.plist` `CFBundleShortVersionString` | 3-part (no pre-release) | `2.0.0` |
| `macos/.../Info.plist` `CFBundleVersion` | Build number | `1` (incremented per build) |

### Automated Version Bumping

Version bumps are managed via the **`version-bump.yml`** GitHub Actions workflow. This ensures all version files stay in sync automatically.

**How to trigger:**

```bash
# Explicit version
gh workflow run version-bump.yml -f version=2.0.0-alpha.2

# Increment by type
gh workflow run version-bump.yml -f bump_type=patch
gh workflow run version-bump.yml -f bump_type=minor
gh workflow run version-bump.yml -f bump_type=pre-beta

# Also create a git tag
gh workflow run version-bump.yml -f version=2.0.0-beta.1 -f create_tag=true

# Skip PR, commit directly
gh workflow run version-bump.yml -f version=2.0.0-beta.1 -f create_pr=false
```

Or use the GitHub Actions UI: **Actions** > **Version Bump** > **Run workflow**.

### CI Version Sync Check

The `ci-rust.yml` workflow includes a `version-check` job that verifies all platform version files match `Cargo.toml`. If versions drift out of sync, CI will fail with a clear error message pointing to the mismatched file.

---

## How to Cut a Release

### Step-by-step

1. **Ensure all work is merged to `main`**
   - All milestone issues closed
   - CI is green

2. **Bump the version**
   ```bash
   gh workflow run version-bump.yml \
     -f version=2.0.0-beta.1 \
     -f create_tag=true \
     -f create_pr=true
   ```

3. **Review and merge the version bump PR**
   - Verify all version files are consistent
   - Merge to `main`

4. **Push the tag** (if not already created by the workflow)
   ```bash
   git tag -a v2.0.0-beta.1 -m "Release v2.0.0-beta.1"
   git push origin v2.0.0-beta.1
   ```

5. **The `release.yml` workflow runs automatically** on tag push:
   - Builds all platforms (macOS arm64, Windows x64/arm64, Linux x64/arm64)
   - Generates SHA256 checksums
   - Creates a **draft** GitHub Release with artifacts and release notes

6. **Review the draft release** on GitHub
   - Edit release notes if needed
   - Publish when ready

### Hotfix Releases

For urgent patches on a released version:

1. Create a branch from the release tag: `git checkout -b hotfix/v2.0.1 v2.0.0`
2. Apply fixes
3. Bump version: `gh workflow run version-bump.yml -f version=2.0.1`
4. Merge to `main` and tag

---

## Version Format Conventions

We follow [Semantic Versioning 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH[-PRE_RELEASE]
```

### Pre-release Labels

| Label | Usage | Example |
|-------|-------|---------|
| `alpha.N` | Early development, API unstable | `2.0.0-alpha.3` |
| `beta.N` | Feature-complete, bug-fixing phase | `2.0.0-beta.1` |
| `rc.N` | Release candidate, final testing | `2.0.0-rc.2` |
| *(none)* | Stable release | `2.0.0` |

### Milestone-to-Version Mapping

| Milestone | Version | Status |
|-----------|---------|--------|
| M0 — Repository Setup | `v0.1.0` | ✅ Released |
| M1 — Core Engine | `v0.2.0` | ✅ Released |
| M2 — Rule Engine | `v0.3.0` | ✅ Released |
| M3 — CLI | `v0.4.0` | ✅ Released |
| M4 — FFI + UI Shells | `v0.5.0` | ✅ Released |
| M5 — Providers | `v0.6.0` | ✅ Released |
| M6 — Full Native UI | `v0.7.0` | ✅ Released |
| M7 — Cloud Storage | `v0.8.0` | ✅ Released |
| M8 — Packaging | `v0.9.0` | ✅ Released |
| M9 — Database Export | `v0.10.0` | ✅ Released |
| M10 — Public Release | `v1.0.0` | 🔲 Planned |

> **Note:** The project uses `v0.x.0` pre-release versioning through M9.
> `v1.0.0` is reserved for the first public release at M10.

---

## Platform Version Mapping

### Cargo.toml → Windows MSIX

MSIX uses 4-part versioning (`Major.Minor.Build.Revision`). Pre-release labels are stripped:

| Semver | MSIX |
|--------|------|
| `2.0.0-alpha.1` | `2.0.0.0` |
| `2.0.0-beta.3` | `2.0.0.0` |
| `2.0.0` | `2.0.0.0` |
| `2.1.0` | `2.1.0.0` |

The 4th component (`.0`) is reserved for future use (e.g., build numbers).

### Cargo.toml → macOS Info.plist

- **`CFBundleShortVersionString`**: 3-part version, pre-release stripped (e.g., `2.0.0`)
- **`CFBundleVersion`**: Integer build number, incremented each build (e.g., `1`, `2`, `3`)

Apple requires `CFBundleShortVersionString` to be a valid `X.Y.Z` format for App Store submission.

---

## CI/CD Pipeline Overview

### 8 Workflows

| Workflow | File | Trigger | Purpose |
|----------|------|---------|---------|
| **Rust Core CI** | `ci-rust.yml` | Push/PR to `main` (crates/**) | Format, lint, test, version-sync |
| **macOS CI** | `ci-macos.yml` | Push/PR to `main` (macos/**) | Build SwiftUI app |
| **Windows CI** | `ci-windows.yml` | Push/PR to `main` (windows/**) | Build WinUI 3 app |
| **Linux CI** | `ci-linux.yml` | Push/PR to `main` (crates/mm-gtk/**) | Build GTK4 app under Xvfb |
| **Version Bump** | `version-bump.yml` | Manual (`workflow_dispatch`) | Bump version across all files |
| **Release Build** | `release.yml` | Tag push (`v*`) | Build all platforms, create release |
| **Security Audit** | `audit.yml` | Weekly + push to `main` | `cargo deny` + `cargo audit` |
| **Documentation** | `docs.yml` | Push to `main` (crates/**) | Generate `cargo doc` |

### Release Workflow Details

The release workflow (`release.yml`) runs 5 parallel build jobs + 1 final release job:

```
prepare ──┬── release-macos (arm64)
          ├── release-windows-x64
          ├── release-windows-arm64
          ├── release-linux-x64
          └── release-linux-arm64
                      │
              create-release (draft GitHub Release)
```

**Artifact naming convention:**
```
MeedyaManager-{version}-{platform}-{arch}.tar.gz
MeedyaManager-{version}-{platform}-{arch}.sha256
MeedyaManager-{version}-SHA256SUMS.txt
```

### Code Signing Status

| Platform | Status | Requirement |
|----------|--------|-------------|
| macOS | Pending | Apple Developer ID certificate in `APPLE_CERT_P12` secret |
| Windows | Pending | Code signing certificate in `WINDOWS_CERT_PFX` secret |
| Linux | N/A | Not required for Flatpak/Snap distribution |

---

## Release Binary Hardening

All release and `dist` profile builds include hardening flags that reduce
binary size, improve runtime performance, and remove debug information from
shipped artifacts. This is compliant with all platform store guidelines and
with the GPL-2.0-or-later licence (source code remains fully available).

### Cargo Build Profiles

| Profile | Use case | Key flags |
|---------|----------|-----------|
| `dev` | Local development | `opt-level=0`, `debug=true`, incremental |
| `release` | Release builds | `opt-level=3`, `lto=fat`, `strip=symbols`, `panic=abort` |
| `dist` | Final shipped artifacts | inherits `release` + `strip=debuginfo` |
| `test` | Test runs | `opt-level=1`, `debug=true` |

### What Each Flag Does

| Flag | Effect | Platform compliance |
|------|--------|---------------------|
| `opt-level = 3` | Maximum compiler speed optimisations | All platforms |
| `lto = "fat"` | Cross-crate link-time optimisation — dead code elimination | All platforms |
| `codegen-units = 1` | Single codegen unit for maximum LTO effectiveness | All platforms |
| `strip = "symbols"` | Remove symbol table from binary (~30–60% size reduction) | All platforms |
| `strip = "debuginfo"` | Remove DWARF debug info as well (dist profile only) | All platforms |
| `panic = "abort"` | No unwinding machinery — smaller binary, no stack unwind tables | All platforms |
| `debug = false` | No embedded debug information | All platforms |
| `incremental = false` | Reproducible builds (same input → same output) | All platforms |

### Platform-Specific Hardening

#### macOS
- **Hardened Runtime** — `MeedyaManager.entitlements` enforces:
  - `com.apple.security.app-sandbox = true` — sandboxed execution
  - `com.apple.security.hardened-runtime = true` — JIT disabled, library validation on
- **Notarisation** — all `.dmg` releases notarised via Apple notary service
- **Code signing** — Developer ID certificate required for Gatekeeper

#### Windows
- **MSIX packaging** — authenticode signing via WinAppSDK build pipeline
- **DEP/ASLR** — enforced automatically for all managed (.NET/WinUI 3) code
- **Integrity Level** — MSIX packages run at `Medium IL` by default

#### Linux
- **PIE (Position-Independent Executable)** — Rust enables this by default on Linux
- **RELRO / BIND_NOW** — enabled by default in the Rust linker on ELF targets
- **Strip** — the `cargo build --profile dist` step strips all symbols
- **Flatpak sandboxing** — `strict` confinement via portal permissions

### What We Do NOT Do (and Why)

| Technique | Reason not used |
|-----------|-----------------|
| LLVM obfuscation / obfuscator-llvm | GPL-2.0-or-later requires source availability; obfuscation conflicts with the spirit and legal requirements of the licence |
| Binary packing (UPX) | Triggers antivirus false positives; breaks code signing on macOS/Windows |
| Anti-debugging traps | Not permitted by Apple App Store / Microsoft Store ToS |
| String encryption | Incompatible with GPL source requirements; adds runtime overhead |

### Build Commands

```bash
# Development build (fast, with debug info)
cargo build

# Optimised release build (shipped in CI release workflow)
cargo build --release

# Full distribution build (final shipped artifacts)
cargo build --profile dist

# Check binary size after stripping
size target/release/meedya
file target/release/meedya
```

---

## GitHub Projects Workflow

### Board

We use **GitHub Projects v2** to track all work. The board is at: [MeedyaManager v2.0 — Rust Rewrite](https://github.com/orgs/MWBMPartners/projects/7).

### Issue Lifecycle

1. **Create issue** before starting work (assigned to milestone, labeled, added to project)
2. **Move to In Progress** on the project board when starting
3. **Link PRs** to the issue (`Closes #N` in PR description)
4. **Move to Done** when the PR is merged and verified
5. **Close issue** with a comment noting what was delivered

### Label Conventions

- `milestone:M0` through `milestone:M10` — which milestone
- `platform:core`, `platform:macos`, `platform:windows`, `platform:linux`, `platform:cli`, `platform:all`
- `type:feature`, `type:bug`, `type:chore`, `type:docs`, `type:ci`
- `priority:P0` (critical) through `priority:P3` (low)

### Local Development Quick Reference

```bash
just version          # Show current version
just check            # Run format + lint + tests
just build            # Build all Rust crates
just build-release    # Build in release mode
just release-local    # Build release artifacts locally
just test             # Run all tests
just lint             # Clippy lints
just fmt              # Auto-format code
just audit            # Security + license audit
just docs             # Generate API docs
```
