# Version Management

> **(C) 2025-2026 MWBM Partners Ltd**
>
> This page covers how versions are managed across the MeedyaManager
> multi-platform codebase.

---

## Single Source of Truth

The canonical version lives in the **root `Cargo.toml`** under
`[workspace.package].version`. The current value is **1.3.0**. Other platform files are meant to
be derived from it, but only two are actually kept in sync by CI:

| File | Field | Format | Kept in sync by CI? |
| ---- | ----- | ------ | -------------------- |
| `Cargo.toml` | `[workspace.package].version` | Semver `X.Y.Z` | — (source of truth) |
| `macos/MeedyaManager/Info.plist` | `CFBundleShortVersionString` | 3-part `X.Y.Z` | Yes (`ci-rust.yml` `version-check`) |
| `windows/MeedyaManager/Package.appxmanifest` | `Identity Version` | 4-part `X.Y.Z.0` | Yes (`ci-rust.yml` `version-check`) |
| `snapcraft.yaml`, `linux/deb/control`, `*.metainfo.xml` | package version | varies | **No** — currently `0.9.0`, unsynced |
| WinGet manifest | `PackageVersion` | Semver | **No** — currently `1.0.0`, unsynced |

The `version-check` job in `ci-rust.yml` only compares `Cargo.toml` against `Info.plist` and
`Package.appxmanifest`. It does not touch the Linux package manifests or the WinGet manifest, so
those can and do drift silently.

---

## Milestone Versioning — status as actually observed

Every milestone below has a closed GitHub milestone/issue range, but **closed does not mean the
feature is real** — see the completion-reality notes in `.claude/HANDOFF.md` and the per-domain
audits. In particular, M7 (cloud), M9 (database export), and M10 (secure media server) are
architectural scaffolding: no real network calls, no live database connections, and `mm-server`
never builds an axum router (`crates/mm-cli/src/commands/serve.rs:337-342` prints
*"Server stub: exiting cleanly"*).

| Milestone | Issues | GitHub state | Functional state |
| --------- | ------ | ------------- | ----------------- |
| M0 — Repository Setup | #19-#31 | Closed | Done |
| M1 — Core Engine | — | Closed | Done (217+ tests at the time, since grown) |
| M2 — Rule Engine | — | Closed | Done |
| M3 — CLI | — | Closed | Done |
| M4 — FFI & Shells | #63-#72 | Closed | Done |
| M5 — Providers | #73-#84 | Mixed (open + closed) | Partial — `meedya lookup` still prints "coming in M5" |
| M6 — Full Native UI | #85-#93 | Closed | Done |
| M7 — Cloud Storage | #94-#102 | Reopened this session | **Status: not yet implemented** — scaffolding only |
| M8 — Packaging | #103-#111 | Closed | Partial — see Release Process wiki |
| M9 — Database Export | #112-#119 | Reopened this session | **Status: not yet implemented** — no live DB pool ever created |
| M10 — Secure Media Server | #120-#127 | Reopened this session | **Status: not yet implemented** — no axum router, zero `.html` files in the repo |

**No `v1.0.0` or any other public release has been cut.** The only GitHub release is
*"MetaMancer v1.0-M1"* (2025-06-16, pre-rename), tagged `v1.0-M1`. The current version, 1.3.0,
has never been tagged. Versions 1.3.1 and 1.3.2 appear in `docs/changelog.md` but were **never
actually set in `Cargo.toml`** — see the changelog itself for the correction.

The historical `v0.x.0`-per-milestone numbering scheme this page previously described does not
match what is in `Cargo.toml`'s git history; the table above reports milestone-to-issue-range and
completion state instead of a version-per-milestone mapping, since no such mapping was ever
actually applied.

---

## How to Bump the Version

### Manual bump (during development)

Edit `Cargo.toml` workspace version, then update the derived files:

```bash
# Update Cargo.toml
sed -i 's/^version = "1.3.0"/version = "1.4.0"/' Cargo.toml

# Update Info.plist (macOS)
sed -i 's/<string>1.3.0<\/string>/<string>1.4.0<\/string>/' \
    macos/MeedyaManager/Info.plist

# Update Package.appxmanifest (Windows)
sed -i 's/Version="1.3.0.0"/Version="1.4.0.0"/' \
    windows/MeedyaManager/Package.appxmanifest
```

Remember this does **not** cover the Linux package manifests or the WinGet manifest — those need
separate manual edits if you want them in sync.

### Automated bump via GitHub Actions

```bash
# Bump to explicit version
gh workflow run version-bump.yml -f version=1.4.0

# Bump by increment type
gh workflow run version-bump.yml -f bump_type=minor   # 1.3.0 → 1.4.0
gh workflow run version-bump.yml -f bump_type=major   # 1.3.0 → 2.0.0

# Pre-release increments are also supported
gh workflow run version-bump.yml -f bump_type=pre-alpha  # 1.3.0 → 1.3.0-alpha.1 (or -alpha.N+1)
gh workflow run version-bump.yml -f bump_type=pre-beta
gh workflow run version-bump.yml -f bump_type=pre-rc

# Bump + create a tag
gh workflow run version-bump.yml -f version=1.4.0 -f create_tag=true
```

The workflow updates `Cargo.toml`, `Package.appxmanifest`, `Info.plist`, and inserts a new
`docs/changelog.md` section, then either opens a PR (`create_pr: true`, the default) or commits
directly to the triggering branch.

---

## Version Sync CI Check

The `ci-rust.yml` workflow runs a `version-check` job that:

1. Reads `Cargo.toml` workspace version
2. Reads `CFBundleShortVersionString` from `Info.plist`
3. Reads `Identity.Version` from `Package.appxmanifest`
4. Fails the build if either derived value doesn't match the base (pre-release-stripped) version

This prevents drift between `Cargo.toml`, `Info.plist`, and `Package.appxmanifest` — but it does
**not** check `CFBundleVersion`, `snapcraft.yaml`, `linux/deb/control`, any `.metainfo.xml`, or
the WinGet manifest.

---

## Platform Format Mapping

### Cargo.toml → macOS Info.plist

- `CFBundleShortVersionString` = 3-part semver, pre-release suffix stripped
- `CFBundleVersion` = a plain integer build number. **This is not auto-incremented by any
  workflow** — `macos/MeedyaManager/Info.plist` currently hardcodes it to `1`, and no CI job (nor
  `version-bump.yml`) ever touches this field. Treat it as a manual field if it needs to move.

### Cargo.toml → Windows MSIX

- MSIX uses `Major.Minor.Build.Revision` (4-part, no pre-release)
- `1.3.0` → `1.3.0.0`
- `1.4.0` → `1.4.0.0`

---

*Last updated: 2026-09-03*
