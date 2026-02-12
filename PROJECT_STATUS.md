# 📊 MediaMancer — Project Status

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**
>
> 🎧📁 Cross-platform media manager and auto-organizer

---

## 🏁 Quick Status

| Item | Status |
|------|--------|
| **Current Milestone** | M2 — CLI & UI Frontend |
| **Last Completed** | ✅ M1 — Core Engine (June 2025) |
| **Overall Progress** | █░░░░░░░░░ **10%** (1 of 10 milestones) |
| **Latest Version** | `v1.0-M1` |
| **Build Status** | ![CI](https://github.com/MWBMPartners/MediaMancer/actions/workflows/python-app.yml/badge.svg) |

---

## 📈 Milestone Progress

### ✅ M1 — Core Engine *(Complete)*

> 🗓️ Completed: June 2025 | 📦 Release: `v1.0-M1`

**Progress: ██████████ 100%**

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Folder watcher (watchdog + polling) | ✅ Done | Real-time monitoring with retry queue |
| Metadata extraction (pymediainfo) | ✅ Done | Full MediaInfo integration |
| 4-level media classification | ✅ Done | Group → Format → Class → Quality |
| Dry-run rename simulation | ✅ Done | Safe simulation mode by default |
| `settings.json5` configuration | ✅ Done | JSON5 with comments support |
| CLI tools (`runner.py`, `metadata_debugger.py`) | ✅ Done | `--simulate-off`, `--out`, `--mkdir`, `--json` |
| PII-safe logging with rotation | ✅ Done | Daily + size-based rotation |
| `.env` loader for API keys | ✅ Done | Fallback environment variables |
| SHA256 checksum verification | ✅ Done | `verify_checksum.py` |
| GitHub Actions CI matrix | ✅ Done | 3 OS × 2 Python versions |
| Release packaging (ZIP/TAR) | ✅ Done | Auto-build on git tags |
| Unit test suite | ✅ Done | 17 tests, 787 lines |

---

### 🔨 M2 — CLI & UI Frontend *(Next Up)*

> 🗓️ Target: TBD | 📦 Release: `v1.1-M2`

**Progress: ░░░░░░░░░░ 0%**

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Interactive CLI rename preview wizard | 🔲 Not Started | |
| `click`-based CLI framework migration | 🔲 Not Started | |
| Rule builder (AND/OR/nested conditions) | 🔲 Not Started | |
| MusicBee-inspired template syntax parser | 🔲 Not Started | |
| PySide6 (Qt6) cross-platform GUI | 🔲 Not Started | |
| Dark/light theme support | 🔲 Not Started | System-aware |
| Rename preview queue + simulation panel | 🔲 Not Started | |
| Drag-and-drop file import | 🔲 Not Started | |
| Rule validation with error reporting | 🔲 Not Started | |
| Settings dialog | 🔲 Not Started | |

---

### 🔮 Future Milestones

| # | Milestone | Status | Target |
|---|-----------|--------|--------|
| M3 | 🧩 Rule Engine & Companion Files | 🔲 Planned | — |
| M4 | ✏️ Metadata Editor | 🔲 Planned | — |
| M5 | 🎵 Music Metadata Lookup | 🔲 Planned | — |
| M6 | 🎬 TV/Film Metadata Lookup | 🔲 Planned | — |
| M7 | ☁️ Cloud Storage Monitoring | 🔲 Planned | — |
| M8 | 📦 Public Release & Packaging | 🔲 Planned | — |
| M9 | 🗄️ Database Export | 🔲 Planned | — |
| M10 | 🌐 Secure Media Server | 🔲 Planned | — |

---

## 🧪 Test Suite Status

| Category | Tests | Lines | Status |
|----------|-------|-------|--------|
| Metadata extraction | 3 | ~140 | ✅ Passing |
| Watcher functionality | 3 | ~130 | ✅ Passing |
| Rename simulation | 3 | ~120 | ✅ Passing |
| Config loading | 2 | ~80 | ✅ Passing |
| ENV loading | 1 | ~45 | ✅ Passing |
| Checksum verification | 1 | ~50 | ✅ Passing |
| CLI runner | 2 | ~100 | ✅ Passing |
| Path integrity | 1 | ~60 | ✅ Passing |
| Import resolution | 1 | ~60 | ✅ Passing |
| **Total** | **17** | **~787** | ✅ **All Passing** |

---

## 🏗️ Architecture Health

| Component | Files | Lines | Health |
|-----------|-------|-------|--------|
| `core/` | 5 | ~400 | ✅ Stable |
| `cli/` | 2 | ~160 | ✅ Stable |
| `utils/` | 3 | ~140 | ✅ Stable |
| `tests/` | 17 | ~787 | ✅ Good coverage |
| `config/` | 1 | 46 | ✅ Stable |
| **Total** | **~28** | **~1,533** | ✅ **Healthy** |

---

## 📋 Known Issues & Technical Debt

| Issue | Priority | Milestone | Notes |
|-------|----------|-----------|-------|
| ~~Naming inconsistency~~ | ✅ Resolved | — | Standardised to "MediaMancer" across all 14 files |
| Polling mode not yet implemented | 🟡 Medium | M2 | Placeholder in `watcher.py` |
| Rename engine uses `{placeholder}` not `<Tag>` syntax | 🟡 Medium | M3 | Migrate to MusicBee-style template syntax |
| No GUI exists yet | 🔵 Low | M2 | PySide6 GUI planned for M2 |
| No `mutagen` integration for tag writing | 🔵 Low | M4 | Currently read-only via pymediainfo |
| Copyright year not yet automated | 🟡 Medium | M2 | Need build-time year substitution |

---

## 📦 Platform Support Matrix

| Platform | Architecture | CI Tested | Package Built | Service Support |
|----------|-------------|-----------|---------------|-----------------|
| 🪟 Windows | x64 | ✅ | ✅ | 🔲 Planned (pywin32) |
| 🪟 Windows | ARM64 | 🔲 | 🔲 | 🔲 Planned |
| 🍎 macOS | Apple Silicon | ✅ | ✅ | 🔲 Planned (launchd) |
| 🐧 Linux | x86_64 | ✅ | ✅ | 🔲 Planned (systemd) |
| 🐧 Linux | ARM64 | 🔲 | 🔲 | 🔲 Planned |

---

## 📅 Recent Activity

| Date | Activity |
|------|----------|
| 2025-02-12 | Animated SVG logo, enhanced PII redaction patterns |
| 2025-02-12 | Enhanced logging and config handling in watcher |
| 2025-02-12 | Fallback settings generation and coverage report config |
| 2025-02-12 | Config handling for "simulate" mode |
| 2025-02-12 | Import path refactoring for classify_media module |

---

> 📝 *This file is updated with each significant change. For detailed changelog, see [docs/CHANGELOG.md](docs/CHANGELOG.md).*
>
> *Last updated: 2026-02-12*
