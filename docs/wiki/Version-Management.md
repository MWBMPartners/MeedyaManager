# Version Management

> **(C) 2025-2026 MWBM Partners Ltd**
>
> This page covers how versions are managed across the MeedyaManager
> multi-platform codebase.

---

## Single Source of Truth

The canonical version lives in the **root `Cargo.toml`** under
`[workspace.package].version`. Every other platform file is derived from it.

| File | Field | Format |
|------|-------|--------|
| `Cargo.toml` | `[workspace.package].version` | Semver `X.Y.Z` |
| `macos/MeedyaManager/Info.plist` | `CFBundleShortVersionString` | 3-part `X.Y.Z` |
| `windows/MeedyaManager/Package.appxmanifest` | `Identity Version` | 4-part `X.Y.Z.0` |

---

## Milestone Versioning

MeedyaManager uses sequential `v0.x.0` versioning during the pre-release phase:

| Milestone | Version | Status |
|-----------|---------|--------|
| M0 — Repository Setup | `v0.1.0` | ✅ Complete |
| M1 — Core Engine | `v0.2.0` | ✅ Complete |
| M2 — Rule Engine | `v0.3.0` | ✅ Complete |
| M3 — CLI | `v0.4.0` | ✅ Complete |
| M4 — FFI & Shells | `v0.5.0` | ✅ Complete |
| M5 — Providers | `v0.6.0` | ✅ Complete |
| M6 — Full Native UI | `v0.7.0` | ✅ Complete |
| M7 — Cloud Storage | `v0.8.0` | ✅ Complete |
| M8 — Packaging | `v0.9.0` | ✅ Complete |
| M9 — Database Export | `v0.10.0` | ✅ Complete |
| M10 — Public Release | `v1.0.0` | 🔲 Planned |

`v1.0.0` is reserved for the first public release after M10 completes.

---

## How to Bump the Version

### Manual bump (during development)

Edit `Cargo.toml` workspace version, then update all derived files:

```bash
# Update Cargo.toml
sed -i 's/^version = "0.10.0"/version = "1.0.0"/' Cargo.toml

# Update Info.plist (macOS)
sed -i 's/<string>0.10.0<\/string>/<string>1.0.0<\/string>/' \
    macos/MeedyaManager/Info.plist

# Update Package.appxmanifest (Windows)
sed -i 's/Version="0.10.0.0"/Version="1.0.0.0"/' \
    windows/MeedyaManager/Package.appxmanifest
```

### Automated bump via GitHub Actions

```bash
# Bump to explicit version
gh workflow run version-bump.yml -f version=1.0.0

# Bump by increment type
gh workflow run version-bump.yml -f bump_type=minor   # 0.10.0 → 0.11.0
gh workflow run version-bump.yml -f bump_type=major   # 0.10.0 → 1.0.0

# Bump + create a tag
gh workflow run version-bump.yml -f version=1.0.0 -f create_tag=true
```

---

## Version Sync CI Check

The `ci-rust.yml` workflow runs a `version-check` job that:

1. Reads `Cargo.toml` workspace version
2. Reads `CFBundleShortVersionString` from `Info.plist`
3. Reads `Identity.Version` from `Package.appxmanifest`
4. Fails the build if any value doesn't match

This prevents version drift across platforms.

---

## Platform Format Mapping

### Cargo.toml → macOS Info.plist

- `CFBundleShortVersionString` = 3-part semver, pre-release stripped
- `CFBundleVersion` = integer build number (incremented per build in CI)

### Cargo.toml → Windows MSIX

- MSIX uses `Major.Minor.Build.Revision` (4-part, no pre-release)
- `0.10.0` → `0.10.0.0`
- `1.0.0` → `1.0.0.0`

---

*Last updated: 2026-03-05*
