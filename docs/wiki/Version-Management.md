# Version Management

> **(C) 2025-2026 MWBM Partners Ltd**
>
> This page covers how versions are managed across the MeedyaManager
> multi-platform codebase.

---

## Single Source of Truth

The canonical version lives in the **root `Cargo.toml`** under
`[workspace.package].version`. The current value is **`1.4.0-alpha.1`** — the project's first
semver pre-release, previously `1.3.0` (issue #214). Other platform files are meant to be derived
from it; as of issues #197/#204/#214, four are actually kept in sync by CI:

| File | Field | Format | Kept in sync by CI? |
| ---- | ----- | ------ | -------------------- |
| `Cargo.toml` | `[workspace.package].version` | Semver `X.Y.Z[-pre]` | — (source of truth) |
| `macos/MeedyaManager/Info.plist` | `CFBundleShortVersionString` | 3-part `X.Y.Z`, pre-release stripped | Yes (`ci-rust.yml` `version-check`) |
| `windows/MeedyaManager/Package.appxmanifest` | `Identity Version` | 4-part `X.Y.Z.0`, pre-release stripped | Yes (`ci-rust.yml` `version-check`) |
| `crates/mm-gtk/Cargo.toml` | `version` | Semver, **exact match including pre-release** | Yes — `mm-gtk` is `exclude`d from `[workspace] members` (#199) and cannot inherit, so it carries a literal copy that CI now checks |
| `linux/snap/snapcraft.yaml` | `version` | Semver, **exact match including pre-release** | Yes — same rationale as `mm-gtk` |
| `linux/deb/control` | `Version` | Debian-mapped (`-` → `~`), e.g. `1.4.0~alpha.1` | **No** — updated by hand for the `1.4.0-alpha.1` cut |
| Flatpak `*.metainfo.xml` `<releases>` | `version` attribute | Semver | **No** — updated by hand; previously carried two fabricated `0.9.0`/`0.8.0` entries dated 2026-03-05 for releases that never happened |
| Flatpak manifest `tag:`/`commit:` | — | — | **No** — still pinned at `v1.0.0` / a literal `placeholder-pin-to-actual-commit-sha`; not touched by the `1.4.0-alpha.1` cut |
| WinGet manifest | `PackageVersion` | Semver | **No** — still `1.0.0`/`0.9.0`, unsynced; not touched by the `1.4.0-alpha.1` cut |

The `version-check` job in `ci-rust.yml` compares `Cargo.toml` against `Info.plist`,
`Package.appxmanifest`, `crates/mm-gtk/Cargo.toml` and `linux/snap/snapcraft.yaml`. It still does
not touch `linux/deb/control`, the Flatpak metainfo/manifest, or the WinGet manifest, so those
can and do drift silently.

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
| M5 — Providers | #73-#84 | Mixed (open + closed) | Partial — `meedya lookup` still prints a not-implemented notice and now exits `3` (`NOT_IMPLEMENTED`) instead of `0` |
| M6 — Full Native UI | #85-#93 | Closed | Done |
| M7 — Cloud Storage | #94-#102 | Reopened this session | **Status: not yet implemented** — scaffolding only |
| M8 — Packaging | #103-#111 | Closed | Partial — see Release Process wiki |
| M9 — Database Export | #112-#119 | Reopened this session | **Status: not yet implemented** — no live DB pool ever created |
| M10 — Secure Media Server | #120-#127 | Reopened this session | **Status: not yet implemented** — no axum router, zero `.html` files in the repo |

**No `v1.0.0` or any other public release has been cut.** The only GitHub release is
*"MetaMancer v1.0-M1"* (2025-06-16, pre-rename), tagged `v1.0-M1`. The current version,
`1.4.0-alpha.1` — previously `1.3.0` — has never been tagged either, though `release.yml` is now
(issue #202) capable of actually producing a usable build from it. Versions 1.3.1 and 1.3.2
appear in `docs/changelog.md` but were **never actually set in `Cargo.toml`** — both are folded
into the `[v1.4.0-alpha.1]` changelog entry, the version the project actually reached.

The historical `v0.x.0`-per-milestone numbering scheme this page previously described does not
match what is in `Cargo.toml`'s git history; the table above reports milestone-to-issue-range and
completion state instead of a version-per-milestone mapping, since no such mapping was ever
actually applied.

---

## How to Bump the Version

### Manual bump (during development)

Edit `Cargo.toml` workspace version, then update the derived files — as of issues #197/#204/#214
this now includes two Linux files that must match **exactly**, pre-release suffix included.
Example: bumping from `1.4.0-alpha.1` to `1.5.0`:

```bash
# Update Cargo.toml
sed -i 's/^version = "1.4.0-alpha.1"/version = "1.5.0"/' Cargo.toml

# Update Info.plist (macOS) — pre-release suffix stripped, so this only changes at all
# when the base X.Y.Z changes, not on a pure pre-release increment
sed -i 's/<string>1.4.0<\/string>/<string>1.5.0<\/string>/' \
    macos/MeedyaManager/Info.plist

# Update Package.appxmanifest (Windows) — same rule
sed -i 's/Version="1.4.0.0"/Version="1.5.0.0"/' \
    windows/MeedyaManager/Package.appxmanifest

# Update mm-gtk's own Cargo.toml — EXACT match, including any pre-release suffix
sed -i 's/^version = "1.4.0-alpha.1"/version = "1.5.0"/' crates/mm-gtk/Cargo.toml

# Update snapcraft.yaml — EXACT match, including any pre-release suffix
sed -i 's/^version: "1.4.0-alpha.1"/version: "1.5.0"/' linux/snap/snapcraft.yaml
```

Remember this still does **not** cover `linux/deb/control` (needs the `-` → `~` Debian remap),
the Flatpak AppStream `*.metainfo.xml`, the Flatpak manifest's pinned `tag:`/`commit:`, or the
WinGet manifest — those need separate manual edits if you want them in sync.

### Automated bump via GitHub Actions

```bash
# Bump to explicit version
gh workflow run version-bump.yml -f version=1.5.0

# Bump by increment type
gh workflow run version-bump.yml -f bump_type=minor   # 1.4.0-alpha.1 → 1.5.0
gh workflow run version-bump.yml -f bump_type=major   # 1.4.0-alpha.1 → 2.0.0

# Pre-release increments are also supported
gh workflow run version-bump.yml -f bump_type=pre-alpha  # 1.4.0-alpha.1 → 1.4.0-alpha.2
gh workflow run version-bump.yml -f bump_type=pre-beta
gh workflow run version-bump.yml -f bump_type=pre-rc

# Bump + create a tag
gh workflow run version-bump.yml -f version=1.5.0 -f create_tag=true
```

The workflow updates `Cargo.toml`, `Package.appxmanifest`, `Info.plist`, `crates/mm-gtk/Cargo.toml`,
`linux/snap/snapcraft.yaml` (the last two exact-match), and inserts a new `docs/changelog.md`
section, then either opens a PR (`create_pr: true`, the default) or commits directly to the
triggering branch. It still does not touch `linux/deb/control`, the Flatpak metainfo/manifest,
or the WinGet manifest.

---

## Version Sync CI Check

The `ci-rust.yml` workflow runs a `version-check` job that:

1. Reads `Cargo.toml` workspace version
2. Reads `CFBundleShortVersionString` from `Info.plist`
3. Reads `Identity.Version` from `Package.appxmanifest`
4. Fails the build if either derived value doesn't match the base (pre-release-stripped) version
5. As of issues #197/#204/#214: reads `crates/mm-gtk/Cargo.toml` and `linux/snap/snapcraft.yaml`
   and fails the build if either does not match `Cargo.toml` **exactly**, pre-release suffix
   included — unlike steps 2/3, no stripping happens here, because both files are meant to carry
   the literal same version string as `Cargo.toml`

This prevents drift between `Cargo.toml`, `Info.plist`, `Package.appxmanifest`,
`crates/mm-gtk/Cargo.toml` and `linux/snap/snapcraft.yaml` — proven by `snapcraft.yaml` itself,
which had silently drifted to `0.9.0` before this check existed and failed the check the moment
it was added. It still does **not** check `CFBundleVersion`, `linux/deb/control`, any
`.metainfo.xml`, the Flatpak manifest's pinned `tag:`/`commit:`, or the WinGet manifest.

---

## Platform Format Mapping

### Cargo.toml → macOS Info.plist

- `CFBundleShortVersionString` = 3-part semver, pre-release suffix stripped
- `CFBundleVersion` = a plain integer build number. **This is not auto-incremented by any
  workflow** — `macos/MeedyaManager/Info.plist` currently hardcodes it to `1`, and no CI job (nor
  `version-bump.yml`) ever touches this field. Treat it as a manual field if it needs to move.

### Cargo.toml → Windows MSIX

- MSIX uses `Major.Minor.Build.Revision` (4-part, no pre-release)
- `1.4.0-alpha.1` → `1.4.0.0`
- `1.5.0` → `1.5.0.0`
- **Consequence worth noting:** because MSIX (and `Info.plist`) strip the pre-release suffix,
  `1.4.0-alpha.1` and the eventual final `1.4.0` map to the *same* platform version number on
  Windows and macOS. No MSIX package is produced by `release.yml` today (see the Release Process
  wiki), so this has not yet caused a real installer conflict — but it will need resolving before
  the first MSIX package exists, since MSIX refuses to install an older-or-equal version over an
  installed one.

### Cargo.toml → Linux packaging

Unlike macOS/Windows, two Linux carriers must match `Cargo.toml` **exactly**, and one remaps the
separator rather than stripping it:

| Carrier | Rule | Example |
| ------- | ---- | ------- |
| `crates/mm-gtk/Cargo.toml` | Exact match, verbatim | `1.4.0-alpha.1` |
| `linux/snap/snapcraft.yaml` | Exact match, verbatim | `1.4.0-alpha.1` |
| `linux/deb/control` | `-` → `~` (Debian ordering: `~` sorts *before* the bare version, `-` would sort *after* it) | `1.4.0~alpha.1` |
| Flatpak `*.metainfo.xml` `<releases>` | Verbatim | `1.4.0-alpha.1` |

---

*Last updated: 2026-09-03*
