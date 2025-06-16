# 📝 CHANGELOG
> (C) 2025 MWBM Partners Ltd (d/b/a MW Services)

This document tracks versioned releases for the MetaMancer media management platform, aligned with its milestone-based development cycle.

All releases follow the format: `v1.0-M{milestone}`.

---

## 🔰 [v1.0-M1] – Initial Watcher & Dry-Run Renamer (2025-06-16)
### ✅ Features
- Real-time folder monitoring via `watchdog`
- Cross-platform support (Windows/macOS/Linux)
- Auto-detection of new and moved media files
- File extension filtering (audio/video containers)
- File lock safety logic with background retry
- Thread-safe event queue for renaming engine
- Development CLI entry point for quick testing
- Simulated renaming engine using token-based templates
- Character sanitization for file/folder names

### 🔒 Improvements
- Avoids processing in-use/locked files to prevent corruption
- Lightweight background threads per event (non-blocking)
- Modular design between watcher and renamer modules
- Full logging for both detection and renaming simulations

### 📁 Affected Modules
- `core/watcher.py`
- `core/renamer.py`

---

## 🚧 [v1.0-M2] – Rule Engine (In Progress)
Will include:
- IF / AND / OR logic evaluation
- Renaming token parser
- Configurable rules via JSON5/TOML
- Test ruleset builder for dry-run previews

---

## 🔮 Upcoming
- [v1.0-M3] – GUI + Theme Support
- [v1.0-M4] – CI Packaging, GitHub Actions, and Auto Releases
- [v1.0-M5] – Metadata Editing Engine
- [v1.0-M6] – Lookup APIs + Animated Cover Art
- [v1.0-M7+] – Cloud Sync, DB Export, Web Secure Access

---

_Last updated: 2025-06-16_