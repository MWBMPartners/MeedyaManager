# Release Process

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Step-by-step guide to cutting a MeedyaManager release.

---

## Status: no release has ever been cut under this process

The only GitHub release that exists is *"MetaMancer v1.0-M1"* (2025-06-16, the pre-rename
project name), tagged `v1.0-M1`. The current workspace version is **1.3.0**
(`Cargo.toml` `[workspace.package].version`) and has **not** been tagged or released. Everything
below describes the process as the tooling supports it today, not a history of past releases.

---

## Pre-release Checklist

Before starting the release process:

- [ ] All milestone issues are closed
- [ ] PR Gate (`Gate` check) is green on `main`
- [ ] `cargo deny check` reports no issues (`audit.yml` is green)
- [ ] `docs/changelog.md` is up to date
- [ ] `PROJECT_STATUS.md` reflects the completed milestone
- [ ] `Cargo.toml`, `Package.appxmanifest`, and `Info.plist` all match (`version-check` job in
      `ci-rust.yml`) — note this check does **not** cover the Linux package manifests
      (`snapcraft.yaml`, `linux/deb/control`, `*.metainfo.xml`) or the WinGet manifest, which must
      be synced by hand
- [ ] A `LICENSE` file exists — as of this writing it does not (issue #207)

---

## Step-by-Step Release

### 1. Bump version

```bash
gh workflow run version-bump.yml \
  -f version=1.4.0 \
  -f create_tag=true \
  -f create_pr=true
```

Or manually:

```bash
# Edit Cargo.toml, Info.plist, Package.appxmanifest
# Then commit:
git add Cargo.toml macos/MeedyaManager/Info.plist \
        windows/MeedyaManager/Package.appxmanifest
git commit -m "chore: bump version to v1.4.0"
git tag -a v1.4.0 -m "Release v1.4.0"
git push origin main v1.4.0
```

### 2. Tag triggers release workflow

Pushing a `v*` tag automatically triggers `release.yml`, which:

1. Builds 5 platform targets in parallel (macOS arm64, Windows x64, Windows arm64, Linux x64,
   Linux arm64)
2. Packages each target — macOS: `.dmg`; Windows: MSIX; Linux: `.tar.gz` plus a best-effort
   `.deb` and AppImage (each step falls back to a warning and continues if its packaging tool is
   unavailable on the runner). **There is no Flatpak build step** — `release.yml` contains no
   Flatpak job or action.
3. Generates SHA256 checksums
4. Creates a **draft** GitHub Release with all artifacts attached

### 3. Review the draft release

1. Go to **Releases** on GitHub
2. Review the auto-generated release notes
3. Add any manual highlights or migration notes
4. Download and verify at least one artifact locally

### 4. Publish the release

Click **Publish release** when satisfied. This makes the release public and notifies any
subscribers watching releases. There is no automated WinGet or Flathub submission step in this
repository today — those would need to be filed and submitted manually if desired.

---

## Platform Artifacts

| Platform | Artifact | Notes |
| -------- | -------- | ------ |
| macOS (Apple Silicon) | `MeedyaManager-{v}-macos-arm64.dmg` | Signed + notarised (when Apple secrets are configured) |
| Windows x64 | `MeedyaManager-{v}-windows-x64.msix` | Authenticode signed (when Windows secrets are configured) |
| Windows ARM64 | `MeedyaManager-{v}-windows-arm64.msix` | Authenticode signed (when Windows secrets are configured) |
| Linux x64 | `MeedyaManager-{v}-linux-x64.tar.gz` | Raw `mm-gtk`/`mm-cli`/`libmm_ffi.so` binaries |
| Linux x64 | `meedyamanager_{v}_amd64.deb` | Debian/Ubuntu package, best-effort (skipped if `dpkg-deb` unavailable) |
| Linux x64 | `MeedyaManager-{v}-x86_64.AppImage` | Portable AppImage, best-effort (skipped if `appimagetool` unavailable) |
| Linux ARM64 | `MeedyaManager-{v}-linux-arm64.tar.gz` | Raw binaries, same caveats as Linux x64 |
| Checksums | `SHA256SUMS.txt` | Concatenation of every artifact's individual `.sha256` file |

There is **no Flatpak artifact**. Earlier drafts of this page referenced one; it does not exist in
`release.yml` (`grep -c flatpak .github/workflows/release.yml` → 0).

---

## Release Binary Hardening

All release artifacts are built with the `dist` Cargo profile:

```toml
[profile.dist]
inherits = "release"
opt-level = 3
lto = "fat"
codegen-units = 1
strip = "debuginfo"
panic = "abort"
debug = 0
```

This ensures:

- Maximum performance (O3 + LTO)
- Minimal binary size (stripped debug info + symbols)
- No unwinding tables (`panic = abort`)
- Reproducible builds (`incremental = false`)

See [Developer Notes — Release Binary Hardening](../../Dev_Notes.md#release-binary-hardening)
for full details and platform-specific hardening (Hardened Runtime, MSIX signing, PIE). `Dev_Notes.md`
lives at the **repository root**, not under `docs/wiki/` — a relative link of `Dev_Notes.md` from
this page resolves incorrectly to `docs/wiki/Dev_Notes.md`, which does not exist.

---

## Code Signing Requirements

| Platform | Certificate | Secret Name |
| -------- | ------------ | ------------ |
| macOS | Apple Developer ID Application | `APPLE_CERT_P12` |
| macOS (notarisation) | Apple ID + app-specific password | `APPLE_ID`, `APPLE_PASSWORD` |
| Windows | Code signing certificate (PFX) | `WINDOWS_CERT_PFX` |

Without these secrets, CI will build unsigned artifacts (suitable for local testing).

---

## Hotfix Process

For urgent patches after a release:

```bash
# Branch from the release tag
git checkout -b hotfix/v1.4.1 v1.4.0

# Apply the fix and test
# ...

# Bump patch version
gh workflow run version-bump.yml -f version=1.4.1

# Merge to main
git checkout main
git merge hotfix/v1.4.1

# Tag and push
git tag -a v1.4.1 -m "Hotfix v1.4.1"
git push origin main v1.4.1

# Delete the hotfix branch
git branch -d hotfix/v1.4.1
```

---

*Last updated: 2026-09-03*
