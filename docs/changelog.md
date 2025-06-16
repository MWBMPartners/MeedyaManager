# 📦 CHANGELOG.md

(C) 2025 MWBM Partners Ltd (d/b/a MW Services)

This changelog tracks major milestone-based releases and architectural changes for the MetaMancer project.

---

## 🧱 Metadata Hierarchy Reference
As of `v1.0-M1`, all media are classified using a standardized internal structure:

| Level         | Field           | Description                        | Example Values                         |
|---------------|------------------|------------------------------------|----------------------------------------|
| 1️⃣           | `media_group`   | High-level category                | Audio, Video, Image, Book              |
| 2️⃣           | `format_class`  | Container/codec format             | MP3, FLAC, MP4, PDF, Matroska          |
| 3️⃣           | `media_class`   | Functional purpose/content type    | Music, Movie, TV Show, Podcast, Booklet|
| 4️⃣           | `quality_type`  | Fidelity or compression class      | Lossy, Lossless                        |

This hierarchy powers all classification, rename rules, metadata editing, and UI grouping.

Additional design notes:
- Booklets (PDF) and Animated Album Art (MP4 square/portrait) can be attached to albums
- Albums or tracks can be linked to movies, TV shows, or episodes for contextual reference

---

## ✅ v1.0-M1 — Core Engine & Simulation Framework
**Release Date:** 2025-06-XX

### 🚀 Core Features
- 📂 Folder watcher with `watchdog` (fallback to polling mode)
- 🧠 Metadata parsing via `MediaInfo`
- 📊 Auto classification (media group, format class, etc)
- 🔄 Dry-run rename simulation
- 🔧 Configurable rename logic via `settings.json5`
- 🗃️ File existence checks and retry queuing for locked media
- 🔒 Redacted logging (paths like `/Users/YourName` → `REDACTED`)
- 📝 Logging system with daily + size-based rotation
- 📤 Dry-run metadata export to `.json` (with `--out` and `--mkdir`)
- ⚙️ CLI toggle `--simulate-off` to suppress rename simulations

### 🧪 Testing Coverage
- `test_metadata_extractor.py`: Format and classification
- `test_simulate_flag_behavior.py`: Toggle simulation on/off
- `test_watcher_simulation_trigger.py`: Simulate from watcher
- `test_simulation_log_output.py`: Check log content and redaction
- `test_batch_rename_simulation.py`: Multi-file integration

### 🏗️ Build & CI
- ✅ Full GitHub Actions CI matrix (Windows/macOS/Linux)
- ✅ Python 3.10 & 3.11 testing with log upload on failure
- ✅ Build pipeline auto-packages ZIP/TAR on tagged release
- ✅ Assets auto-attached to GitHub Releases via `softprops/action-gh-release`

---

## 🔜 Next Release: `v1.1-M2` — UI + CLI Wizard
Planned for July 2025

- 🎛️ Interactive CLI rename rule wizard
- 🧙 Light GUI for batch file review and rename preview
- 🎨 Light/dark UI theme support
- 🧪 Live simulation/rename toggle via GUI
- 📥 Drag-and-drop file import testing

---

## 🗂 Historical Notes
Initial scaffolding and modular classification work began in early June 2025.
All logic is platform-agnostic and modularized for future metadata editing, API lookups, and media export capabilities.

Future milestones (M2–M8+) will include advanced lookup, sync, export, and cloud-safe archiving options.