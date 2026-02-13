# MeedyaManager — Claude Code Project Instructions

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

## Project Identity

- **Name:** MeedyaManager
- **Type:** Cross-platform media file manager and auto-organizer
- **Language:** Python 3.14+ (bundled & sandboxed via Nuitka — never impacts host Python)
- **Licence:** GPL-2.0-or-later
- **Copyright:** MWBM Partners Ltd (d/b/a MW Services)
- **Platforms:** Windows (x64/ARM), macOS (Apple Silicon only), Linux (x64/ARM)

## Key Architecture Decisions

- **Modular structure:** `core/`, `cli/`, `ui/`, `metadata/`, `cloud/`, `export/`, `utils/`, `service/`
- **Config format:** JSON5 (`settings.json5`) with `.env` fallback for secrets
- **Metadata extraction:** `pymediainfo` (MediaInfo library)
- **Metadata writing:** `mutagen` (M4+)
- **GUI framework:** PySide6 6.10+ / Qt6 with native platform styling (M2+)
- **macOS Liquid Glass:** PyObjC bridge → `NSGlassEffectView` (falls back to `NSVisualEffectView` on older macOS)
- **CLI framework:** `click` (M2+)
- **File watching:** `watchdog` with polling fallback
- **Rule engine:** MusicBee-inspired template syntax with `<Tag>`, `$If()`, `$And()`, `$Or()` (M3)

## Coding Standards

- **ALL code** must include detailed comments/annotations (every line where possible)
- **Copyright header** in every source file:

  ```python
  # (C) 2025-{current_year} MWBM Partners Ltd (d/b/a MW Services)
  ```

- **Copyright year** must be automated (start 2025, end current year)
- Code formatting: proper line breaks, indentation, full code (no shortened annotations)
- Emojis are welcome in documentation and UI
- Use the canonical name **MeedyaManager** (not MetaMancer) everywhere

## Documentation Requirements

- **Project_Plan.md** — Full project plan (root)
- **PROJECT_STATUS.md** — Current status tracker (root)
- **README.md** — Project overview with quick start (root)
- **docs/CHANGELOG.md** — Detailed change log with dates
- **docs/ROADMAP.md** — Milestone timeline
- **help/** — User documentation (getting-started, config, rules, formats, troubleshooting, FAQ)
- **.claude/** — This file + project brief for session continuity
- **All .md files** must be updated after every code change

## Milestone Order

1. ✅ M1 — Core Engine (Complete)
2. ✅ M2 — CLI & UI Frontend (Complete)
3. ✅ M3 — Rule Engine & Companion Files (Complete)
4. ✅ M4 — Metadata Editor (Complete)
5. ✅ M5 — Metadata Lookup (19 providers: music, video, podcasts, identifiers) (Complete)
6. ✅ M6 — Packaging, Error Handling & Config Profiles (Complete)
7. M7 — Cloud Storage Monitoring (OneDrive, Google Drive, Dropbox, MEGA, iCloud)
8. M8 — Public Release & Packaging
9. M9 — Database Export (MySQL, MariaDB, SQL Server, SQLite, PostgreSQL)
10. M10 — Secure Media Server

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

## Packaging & Distribution

- **Nuitka** compiles Python to native C/machine code with embedded runtime
- App is fully self-contained — users need ZERO pre-installed software
- Bundled Python 3.14 runtime is sandboxed (isolated from any host Python)
- GUI uses PySide6 6.10+ with native platform styling (Cocoa on macOS, Win11 on Windows)

## Git & CI/CD

- GitHub Actions: CI test matrix (3 OS × Python 3.14)
- Release builds via Nuitka on git tags (`v*`)
- Platform packages: Windows (MSI/ZIP), macOS (DMG/TAR.GZ), Linux (AppImage/DEB/TAR.GZ)
- All packages include SHA256 checksums
- `.gitignore` covers OS files, Python, IDEs, secrets, build artifacts, Nuitka outputs
