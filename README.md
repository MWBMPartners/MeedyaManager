# MetaMancer

<p align="center">
  <img src="branding/mediamancer-logo.svg" alt="MetaMancer Logo" width="200" height="200" />
</p>

🎧📁 **MetaMancer** is a smart, cross-platform media manager and auto-organizer for audio, video, images, and books — inspired by the flexibility of MusicBee but built to run on Windows, macOS (Apple Silicon), and Linux (x86/ARM).

(C) 2025 MWBM Partners Ltd (d/b/a MW Services)

# MetaMancer

![CI Tests](https://github.com/MWBMPartners/MediaMancer/actions/workflows/python-app.yml/badge.svg)

---

## 🚀 Features (Milestone Overview)

### ✅ Milestone 1: Core Engine
- Folder watcher using `watchdog` (with polling fallback)
- Real metadata extraction using MediaInfo
- Dry-run rename engine with simulated path output
- Configurable CLI tool with `settings.json5` fallback
- Output as `.json` (optional) with folder overrides
- `.env`-based environment fallback support for credentials and region
- Simulation log output and rotating log cleanup with PII redaction
- Full GitHub Actions CI matrix (Linux, macOS, Windows — Py 3.10/3.11)
- Post-install checksum verification tool (`verify_checksum.py`)
- Checksum `.sha256` file generation in CI packaging
- Unit tests for `.env` loading, rename logic, and checksum validation

### 🧠 Metadata Hierarchy
MetaMancer classifies media using a multi-level structure:

| Level | Field          | Example                          |
|-------|----------------|----------------------------------|
| 1️⃣    | `media_group`   | Audio, Video, Image, Book         |
| 2️⃣    | `format_class`  | MP3, FLAC, MP4, PDF, JPEG         |
| 3️⃣    | `media_class`   | Music, Movie, TV Show, Podcast    |
| 4️⃣    | `quality_type`  | Lossy, Lossless                  |

This enables flexible rule matching, file routing, and future UI metadata overrides.

---

## ✅ Development Usage

### 🧪 Testing Locally
```bash
pip install -r requirements.txt
pytest tests/
```

### 🔐 .env Support
Create a `.env` file or copy `.env.example` to define API keys and region overrides:
```env
SPOTIFY_CLIENT_ID=...
METAMANCER_REGION_DEFAULT=GB
```

### 📦 Post-install Integrity Check
After downloading a ZIP or TAR archive from the [Releases](https://github.com/mwbm-partners/mediamancer/releases):
```bash
python utils/verify_checksum.py dist/MediaMancer-macos-latest.tar.gz dist/MediaMancer-macos-latest.tar.gz.sha256
```
This will compare SHA256 hashes and ensure the file is untampered.

---

## 📦 Release Builds
- ✅ GitHub Actions builds packages per platform on tag (v1.0-M1, v1.0-M2, ...)
- ✅ Includes auto-generated `.sha256` files for each platform archive
- ✅ Final packages contain `core/`, `cli/`, `settings.json5`, `branding/`, `README.md`

---

## 🔭 Coming in Milestone 2
- UI & CLI Wizard for configuring rename rules
- Metadata editing tools with manual overrides
- Light/dark UI modes and search/test preview panes

---

(C) 2025 MWBM Partners Ltd (d/b/a MW Services)