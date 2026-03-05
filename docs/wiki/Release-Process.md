# Release Process

> **(C) 2025-2026 MWBM Partners Ltd**
>
> Step-by-step guide to cutting a MeedyaManager release.

---

## Pre-release Checklist

Before starting the release process:

- [ ] All milestone issues are closed
- [ ] All CI workflows are green (Rust, macOS, Windows, Linux)
- [ ] `cargo audit` reports no vulnerabilities (`audit.yml` is green)
- [ ] `docs/changelog.md` is up to date
- [ ] `PROJECT_STATUS.md` reflects the completed milestone
- [ ] All three platform version files match `Cargo.toml`

---

## Step-by-Step Release

### 1. Bump version

```bash
gh workflow run version-bump.yml \
  -f version=1.0.0 \
  -f create_tag=true \
  -f create_pr=true
```

Or manually:

```bash
# Edit Cargo.toml, Info.plist, Package.appxmanifest
# Then commit:
git add Cargo.toml macos/MeedyaManager/Info.plist \
        windows/MeedyaManager/Package.appxmanifest
git commit -m "chore: bump version to v1.0.0"
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin main v1.0.0
```

### 2. Tag triggers release workflow

Pushing a `v*` tag automatically triggers `release.yml`, which:

1. Builds all 5 platform targets in parallel
2. Packages each target (`.dmg`, MSIX, `.tar.gz`, Flatpak, AppImage, `.deb`)
3. Generates SHA256 checksums
4. Creates a **draft** GitHub Release with all artifacts attached

### 3. Review the draft release

1. Go to **Releases** on GitHub
2. Review the auto-generated release notes
3. Add any manual highlights or migration notes
4. Download and verify at least one artifact locally

### 4. Publish the release

Click **Publish release** when satisfied. This:
- Makes the release public
- Triggers WinGet / Flathub submission workflows (if configured)
- Notifies any subscribers watching releases

---

## Platform Artifacts

| Platform | Artifact | Notes |
|----------|----------|-------|
| macOS (Apple Silicon) | `MeedyaManager-{v}-macos-arm64.dmg` | Signed + notarised |
| Windows x64 | `MeedyaManager-{v}-windows-x64.msix` | Authenticode signed |
| Windows ARM64 | `MeedyaManager-{v}-windows-arm64.msix` | Authenticode signed |
| Linux x64 | `MeedyaManager-{v}-linux-x64.tar.gz` | GTK4 binary |
| Linux ARM64 | `MeedyaManager-{v}-linux-arm64.tar.gz` | GTK4 binary |
| Linux (all) | `MeedyaManager-{v}-linux-x64.flatpak` | GNOME Flatpak |
| Linux (all) | `MeedyaManager-{v}-linux-x64.deb` | Debian/Ubuntu package |
| Linux (all) | `MeedyaManager-{v}-linux-x64.AppImage` | Portable AppImage |
| Checksums | `MeedyaManager-{v}-SHA256SUMS.txt` | All artifact hashes |

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

See [Developer Notes — Release Binary Hardening](Dev_Notes.md#release-binary-hardening)
for full details and platform-specific hardening (Hardened Runtime, MSIX signing, PIE).

---

## Code Signing Requirements

| Platform | Certificate | Secret Name |
|----------|-------------|-------------|
| macOS | Apple Developer ID Application | `APPLE_CERT_P12` |
| macOS (notarisation) | Apple ID + app-specific password | `APPLE_ID`, `APPLE_PASSWORD` |
| Windows | Code signing certificate (PFX) | `WINDOWS_CERT_PFX` |

Without these secrets, CI will build unsigned artifacts (suitable for local testing).

---

## Hotfix Process

For urgent patches after a release:

```bash
# Branch from the release tag
git checkout -b hotfix/v1.0.1 v1.0.0

# Apply the fix and test
# ...

# Bump patch version
gh workflow run version-bump.yml -f version=1.0.1

# Merge to main
git checkout main
git merge hotfix/v1.0.1

# Tag and push
git tag -a v1.0.1 -m "Hotfix v1.0.1"
git push origin main v1.0.1

# Delete the hotfix branch
git branch -d hotfix/v1.0.1
```

---

*Last updated: 2026-03-05*
