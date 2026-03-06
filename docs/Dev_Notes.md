# MeedyaManager — Developer Notes

> **(C) 2025-2026 MWBM Partners Ltd**

## Table of Contents

- [Version Management](#version-management)
- [How to Cut a Release](#how-to-cut-a-release)
- [Version Format Conventions](#version-format-conventions)
- [Platform Version Mapping](#platform-version-mapping)
- [CI/CD Pipeline Overview](#cicd-pipeline-overview)
- [GitHub Secrets Configuration](#github-secrets-configuration)
- [Release Binary Hardening](#release-binary-hardening)
- [Dependency Bundling Requirements](#dependency-bundling-requirements)
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
| M10 — Public Release | `v1.0.0` | ✅ Released |

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
| macOS | Implemented | Apple Developer ID cert (`APPLE_CERT_P12` secret) + notarisation |
| Windows | Implemented | Authenticode PFX cert (`WINDOWS_CERT_PFX` secret) via signtool |
| Linux | N/A | Not required for Flatpak/Snap distribution |

---

## GitHub Secrets Configuration

All code signing and release credentials are stored as **GitHub repository
secrets** (Settings → Secrets and variables → Actions → Repository secrets).
The `release.yml` workflow reads these automatically during tag-triggered
release builds. CI builds **do not** require secrets — signing is skipped with
a `::warning::` annotation when a secret is absent.

### How to add a secret

1. Go to the repository on GitHub
2. Click **Settings** → **Secrets and variables** → **Actions**
3. Click **New repository secret**
4. Enter the **Name** and **Value** exactly as shown below
5. Click **Add secret**

---

### Apple Code Signing & Notarisation (macOS)

Apple **requires** all distributed macOS apps to be:
1. **Code-signed** with a Developer ID Application certificate
2. **Notarised** by Apple's notary service
3. **Stapled** — the notarisation ticket attached to the DMG

Without signing and notarisation, Gatekeeper blocks the app on macOS 12+.

#### Required secrets

| Secret name | Description | How to obtain |
|-------------|-------------|---------------|
| `APPLE_DEVELOPER_ID` | Full name string of the Developer ID Application certificate | Keychain Access → find "Developer ID Application: …" — copy the exact name including Team ID in parentheses |
| `APPLE_TEAM_ID` | 10-character Apple Team ID | [developer.apple.com/account](https://developer.apple.com/account) → Membership → Team ID |
| `APPLE_ID` | Apple ID email address used for the Developer Program | The email you use to sign in to developer.apple.com |
| `APPLE_APP_PASSWORD` | App-specific password for `notarytool` | appleid.apple.com → Sign-In and Security → App-Specific Passwords → Generate |
| `APPLE_CERT_P12` | Base64-encoded Developer ID Application certificate + private key (`.p12` / `.pfx`) | Export from Keychain Access → Base64-encode: `base64 -i cert.p12` |
| `APPLE_CERT_PASSWORD` | Password protecting the `.p12` file | The password set when exporting from Keychain Access |

#### Example — exporting and encoding the certificate

```bash
# 1. Open Keychain Access → find "Developer ID Application: MWBM Partners Ltd (XXXXXXXXXX)"
# 2. Right-click → Export → save as cert.p12, set a strong password
# 3. Base64-encode for the secret value:
base64 -i cert.p12 | pbcopy   # macOS — copies to clipboard
base64 -w0 cert.p12            # Linux — prints single-line base64

# 4. Paste the base64 string as the APPLE_CERT_P12 secret value
# 5. Store the export password as APPLE_CERT_PASSWORD
```

#### Example — creating an app-specific password

```
1. Go to appleid.apple.com → Sign-In and Security → App-Specific Passwords
2. Click "+" → name it "MeedyaManager CI Notarisation"
3. Copy the generated password (shown only once)
4. Store it as APPLE_APP_PASSWORD
```

#### What the release workflow does

1. `create-dmg.sh` assembles the `.app` bundle
2. `codesign --deep --options runtime` signs the bundle with the Developer ID certificate
3. `xcrun notarytool submit` uploads the DMG to Apple's notary service and waits for approval
4. `xcrun stapler staple` attaches the notarisation ticket to the DMG
5. The signed, notarised DMG is uploaded as a release artifact

---

### Windows Authenticode Signing

Windows **recommends** (and Microsoft Store **requires**) that MSIX packages
and binaries are signed with an Authenticode certificate. Without signing,
SmartScreen shows a warning on first launch.

#### Required secrets

| Secret name | Description | How to obtain |
|-------------|-------------|---------------|
| `WINDOWS_CERT_PFX` | Base64-encoded code signing certificate + private key (`.pfx` / `.p12`) | Purchase an EV Code Signing certificate from DigiCert, Sectigo, or GlobalSign; export as `.pfx`; Base64-encode: `certutil -encode cert.pfx cert.b64` or `base64 -w0 cert.pfx` |
| `WINDOWS_CERT_PASSWORD` | Password protecting the `.pfx` file | Set when exporting or purchasing the certificate |

#### Example — encoding the certificate

```powershell
# PowerShell — base64-encode the PFX, copy to clipboard
[Convert]::ToBase64String([IO.File]::ReadAllBytes("cert.pfx")) | Set-Clipboard
```

```bash
# bash (Linux/WSL)
base64 -w0 cert.pfx
```

#### What the release workflow does

1. The Base64 value from `WINDOWS_CERT_PFX` is decoded to a temporary `.pfx` file
2. `signtool.exe sign /fd SHA256 /td SHA256 /tr http://timestamp.digicert.com` signs all `.exe` and `.dll` files with a trusted timestamp
3. The temporary `.pfx` is securely deleted from the runner after signing
4. Signed binaries are packaged into the release artifact

#### MSIX identity note

The Windows package identity (`Package.appxmanifest` `Identity.Name`) is
`ltd.MWBMpartners.MeedyaManager`. When submitting to the Microsoft Store,
ensure this name is registered in Partner Center under your Publisher account.

---

### Linux (no signing required)

Flatpak packages distributed via Flathub are signed by Flathub's GPG key,
not by the developer. Snap packages distributed via the Snap Store are signed
by Canonical's infrastructure.

For standalone `.deb` / AppImage / `.tar.gz` releases, SHA256 checksums are
generated and published alongside each artifact — users can verify integrity
without a code signature.

---

### Secrets summary table

| Secret | Required for | Platform |
|--------|-------------|----------|
| `APPLE_DEVELOPER_ID` | Code signing | macOS |
| `APPLE_TEAM_ID` | Notarisation | macOS |
| `APPLE_ID` | Notarisation | macOS |
| `APPLE_APP_PASSWORD` | Notarisation | macOS |
| `APPLE_CERT_P12` | Certificate import (future CI improvement) | macOS |
| `APPLE_CERT_PASSWORD` | Certificate import (future CI improvement) | macOS |
| `WINDOWS_CERT_PFX` | Authenticode signing | Windows |
| `WINDOWS_CERT_PASSWORD` | Authenticode signing | Windows |

> **Security note:** Never commit certificate files or private keys to the
> repository. Store them exclusively as GitHub repository secrets. Rotate
> certificates annually or when a team member with key access leaves.

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

## Dependency Bundling Requirements

MeedyaManager must ship as a self-contained application on all three platforms. Users must not need to install any runtime, SDK, or library separately.

### Overview

| Platform | External Dependency | Bundled How |
| -------- | ------------------- | ----------- |
| All | Rust crate dependencies | **Statically linked** at compile time via Cargo — zero `.dll`/`.dylib`/`.so` from Cargo crates |
| macOS | `libmm_ffi.dylib` (Rust FFI bridge) | Placed in `MeedyaManager.app/Contents/Frameworks/` by `create-dmg.sh` |
| macOS | System frameworks (SwiftUI, Foundation, Security) | Provided by macOS — no bundling required |
| Windows | `mm_ffi.dll` (Rust FFI bridge) | Included via `<Content>` in `MeedyaManager.csproj`; copied to publish output |
| Windows | Windows App SDK runtime | `<WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>` in `.csproj` bundles the runtime in the MSIX |
| Linux | GTK4, Libadwaita | Provided by the `org.gnome.Platform//47` Flatpak runtime — not bundled |
| Linux | All Rust dependencies | Statically compiled into the `mm-gtk` binary by Cargo |

### macOS — Bundling & App Store Compliance

- **`libmm_ffi.dylib`** is signed individually (`codesign --options runtime`) **before** the outer bundle is signed with `--deep`. This is required for Hardened Runtime notarisation.
- **Entitlements** (`macos/MeedyaManager.entitlements`):
  - `app-sandbox = true` — required for Mac App Store submission
  - `files.user-selected.read-write` — grants access to files chosen via open panels
  - `network.client` — outbound network for metadata providers and cloud APIs
  - `keychain-access-groups` — allows the `keyring` crate to read/write API credentials from the macOS Keychain. The `$(AppIdentifierPrefix)` variable is substituted by `codesign`.
- **Mac App Store vs Direct Distribution**: The current build targets **Direct Distribution** via a notarised DMG. For Mac App Store submission, an Xcode project (`.xcodeproj`) is required alongside the SwiftPM package. This is tracked separately.
- **`reqwest` TLS**: Uses the `rustls-tls` feature — OpenSSL is **not** required and **not** linked dynamically.
- **GPL-2.0 licence**: The `LICENSE` file is copied into `Contents/Resources/LICENSE` by `create-dmg.sh`.

### Windows — Bundling & Store Compliance

- **`mm_ffi.dll`** must be built (`cargo build -p mm-ffi --release`) **before** `dotnet publish`. The `.csproj` includes it via a conditional `<Content>` element.
- **Windows App SDK**: `<WindowsAppSDKSelfContained>true</WindowsAppSDKSelfContained>` causes the SDK to be bundled inside the MSIX, eliminating the need for users to install the Windows App Runtime separately.
- **Microsoft Store**: For Store submission, use the **MSIX** package (already configured). The Store manages the Windows App Runtime dependency automatically when `WindowsAppSDKSelfContained` is false. For direct distribution, self-contained is preferred.
- **Authenticode signing**: `signtool.exe` is run in `release.yml` using the `WINDOWS_CERT_PFX` and `WINDOWS_CERT_PASSWORD` secrets. See [GitHub Secrets Configuration](#github-secrets-configuration).
- **GPL-2.0 licence**: The `LICENSE` file is included via `<Content>` in the `.csproj` and deployed alongside the executable.

### Linux — Flatpak & Compliance

- **GNOME Platform runtime** (`org.gnome.Platform//47`) provides GTK4 (4.14), Libadwaita (1.5), and all GNOME libraries. These are **not** bundled inside the Flatpak.
- **Rust dependencies**: All Cargo crates are **statically linked** into the `mm-gtk` binary. The vendor archive (`vendor.tar.gz`) must be regenerated and committed when dependencies change:

  ```bash
  cargo vendor vendor
  tar czf vendor.tar.gz vendor/
  # Update sha256 in the Flatpak YAML
  sha256sum vendor.tar.gz
  ```

- **`libmm_ffi.so`**: Not required for the Linux GTK4 build — `mm-gtk` links directly to `mm-core` as a Cargo workspace dependency without crossing an FFI boundary.
- **GPL-2.0 licence**: Installed to `${FLATPAK_DEST}/share/licenses/ltd.MWBMpartners.MeedyaManager/LICENSE` by the `desktop-integration` Flatpak module.
- **Flathub compliance**: The Flathub submission review checks that:
  - The app ID matches the manifest, `.desktop`, `.metainfo.xml`, and icon filenames.
  - The vendor archive is reproducible and SHA256-pinned.
  - No outbound network access is made during the build.
  - AppStream `<metadata_license>` is FSFAP or CC0; `<project_license>` is GPL-2.0-or-later.

### Snap & AppImage

- **Snap**: `linux/snap/snapcraft.yaml` packages the binary with `confinement: strict`. Rust builds produce statically linked binaries, so no extra stage-packages are needed beyond GTK4 (`libgtk-4-1`, `libadwaita-1-0`).
- **AppImage**: `linux/appimage/build-appimage.sh` uses `appimagetool` to wrap the binary with its GTK4 dependencies into a portable `*.AppImage`. The AppDir includes the GTK4/Libadwaita shared libraries from the build host.

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
