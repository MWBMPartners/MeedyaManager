# 📊 MeedyaManager — Project Status

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**
>
> 🎧📁 Cross-platform media manager and auto-organizer

---

## 🏁 Quick Status

| Item | Status |
|------|--------|
| **Current Milestone** | M3 — Rule Engine & Companion Files |
| **Last Completed** | ✅ M2 — CLI & UI Frontend (Feb 2026) |
| **Overall Progress** | ██░░░░░░░░ **20%** (2 of 10 milestones) |
| **Latest Version** | `v1.1-M2` |
| **Build Status** | ![CI](https://github.com/MWBMPartners/MeedyaManager/actions/workflows/python-app.yml/badge.svg) |

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

### ✅ M2 — CLI & UI Frontend *(Complete)*

> 🗓️ Completed: February 2026 | 📦 Release: `v1.1-M2`

**Progress: ██████████ 100%**

| Deliverable | Status | Notes |
|-------------|--------|-------|
| Click-based CLI framework | ✅ Done | 5 subcommands: scan, debug, watch, rule, gui |
| Scan command with rich output | ✅ Done | `--json`, `--out`, `--mkdir`, `--simulate-off`, `--path` |
| Debug command (metadata inspector) | ✅ Done | Rich tables, JSON export |
| Watch command (folder monitoring) | ✅ Done | `--mode`, `--simulate/--no-simulate`, `--path` |
| Rule template tester | ✅ Done | `--sample`, `--file`, `--template` |
| PySide6 6.10+ cross-platform GUI | ✅ Done | Tabbed interface, drag-and-drop |
| macOS Liquid Glass support | ✅ Done | PyObjC → NSGlassEffectView with vibrancy fallback |
| Windows Mica/Acrylic styling | ✅ Done | DWM API via ctypes |
| Dark/light theme (system-aware) | ✅ Done | darkdetect + QSS stylesheets |
| Rename preview panel | ✅ Done | Table model, progress bar, search filter |
| Settings dialog | ✅ Done | 5 tabs: folders, extensions, template, fallback, replacements |
| Rule builder with syntax highlighting | ✅ Done | Token highlighting, tag insertion, live preview |
| System tray icon | ✅ Done | Show/hide, scan, watch toggle, quit |
| Drag-and-drop file import | ✅ Done | Drop files onto main window |
| GUI test suite | ✅ Done | 23 tests (smoke + model) |
| CLI test suite (CliRunner) | ✅ Done | 18 tests replacing subprocess tests |
| Foundation bug fixes | ✅ Done | Config keys, circular dep, handle_file, classify_media |

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

| Category | Tests | Status |
|----------|-------|--------|
| CLI: scan command | 6 | ✅ Passing |
| CLI: debug command | 5 | ✅ Passing |
| CLI: rule command | 6 | ✅ Passing |
| CLI: version flag | 1 | ✅ Passing |
| GUI: smoke tests | 11 | ✅ Passing |
| GUI: preview model | 12 | ✅ Passing |
| Metadata extraction | 6 | ✅ Passing |
| Media classification | 2 | ✅ Passing |
| Watcher functionality | 6 | ✅ Passing |
| Rename simulation | 1 | ✅ Passing |
| Config & ENV loading | 5 | ✅ Passing |
| Checksum verification | 3 | ✅ Passing |
| Path & import integrity | 5 | ✅ Passing |
| Watcher logging & redaction | 2 | ✅ Passing |
| Simulation log output | 1 | ✅ Passing |
| Batch rename simulation | 1 | ✅ Passing |
| **Total** | **73** | ✅ **All Passing** |

---

## 🏗️ Architecture Health

| Component | Files | Health |
|-----------|-------|--------|
| `core/` | 5 | ✅ Stable |
| `cli/` | 7 | ✅ Stable (Click framework) |
| `cli/commands/` | 5 | ✅ New (scan, debug, watch, rule, gui) |
| `ui/` | 8 | ✅ New (PySide6 GUI) |
| `ui/themes/` | 2 | ✅ New (dark.qss, light.qss) |
| `utils/` | 3 | ✅ Stable |
| `tests/` | 20 | ✅ 73 tests, all passing |
| `config/` | 1 | ✅ Stable |
| **Total** | **~51** | ✅ **Healthy** |

---

## 📋 Known Issues & Technical Debt

| Issue | Priority | Milestone | Notes |
|-------|----------|-----------|-------|
| Polling mode not yet implemented | 🟡 Medium | M3 | Placeholder in `watcher.py` |
| Rename engine uses `{placeholder}` not `<Tag>` syntax | 🟡 Medium | M3 | Migrate to MusicBee-style template syntax |
| No `mutagen` integration for tag writing | 🔵 Low | M4 | Currently read-only via pymediainfo |
| Watcher not integrated with GUI toggle | 🟡 Medium | M3 | GUI button state tracked, needs core watcher connection |
| Rule builder text-only (no visual $If/$And/$Or) | 🔵 Low | M3 | Visual condition builder planned for M3 |

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
| 2026-02-13 | **M2 Complete** — Click CLI, PySide6 GUI, 73 tests passing |
| 2026-02-13 | GUI: Main window, preview panel, settings, rule builder, system tray |
| 2026-02-13 | Platform styling: macOS Liquid Glass, Windows Mica, Linux Fusion |
| 2026-02-13 | Click CLI: scan, debug, watch, rule, gui commands |
| 2026-02-13 | Foundation fixes: config keys, circular deps, classify_media |
| 2025-02-12 | Animated SVG logo, enhanced PII redaction patterns |

---

> 📝 *This file is updated with each significant change. For detailed changelog, see [docs/CHANGELOG.md](docs/CHANGELOG.md).*
>
> *Last updated: 2026-02-13*
