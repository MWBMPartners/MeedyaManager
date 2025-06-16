# 🎵🧙‍♂️ MetaMancer

**MetaMancer** is a smart, cross-platform media file organizer and metadata manager — a magical toolbox for taming your music, videos, podcasts, and more using embedded tags and metadata rules.

> *"Taming your media with metadata magic."*

---

## ✨ Features

- ✅ **Cross-platform**: Windows (x64 & ARM), macOS (Apple Silicon), Linux (x86/x64/ARM)
- 📁 **Real-time folder monitoring** with safe file-in-use detection
- 🧠 **Flexible renaming engine** using metadata and custom logic
- 🎶 **Media-aware sorting**: Music, Music Videos, Podcasts, TV, Movies, etc.
- 🪄 **Custom rules**: IF / AND / OR / nested logic for arranging files
- 🔁 **Multi-value tag support** with metadata-safe practices
- 🎛️ **Light & dark UI themes**, automatic or user-controlled
- 🔐 **API key support**: Dev/staging vs user keys with packaging toggle
- 🛠️ **Manual metadata editing**
- 🌐 **Metadata lookup support** for music & video (MusicBrainz, Apple Music, Spotify, etc.)
- 🖼️ **Animated cover art support** (square & portrait .mp4)
- ☁️ **Cloud sync folder support** *(OneDrive, GDrive, Dropbox etc.)*
- 🗃️ **Export metadata to external databases** *(MySQL, SQLite, PostgreSQL, etc.)*
- 🔒 **Optional secure export of media files to intranet/web libraries**

---

## 🧩 Tech Stack

| Component | Stack |
|----------|-------|
| Language | Python 3.11+ |
| GUI      | PySide6 (Qt) |
| File Monitor | watchdog |
| Media Parsing | MediaInfo, Mutagen |
| Rule Config | JSON5 or TOML |
| CLI      | Typer or argparse |
| CI/CD    | GitHub Actions |
| Packaging | PyInstaller, AppImage, Briefcase |
| Themes   | Adaptive light/dark UI |
| Logo     | Animated SVG (dark/light mode support) |

---

## 📦 Supported Formats

- **Audio**: MP3, M4A, FLAC, ALAC, OGG, AC3, EAC3, AC4
- **Video**: MP4, MKV, MKA, M4V, AVI, HEVC, MPG, DIVX
- **Audio Profiles**: Lossless vs Lossy, Dolby, Spatial Audio, multichannel support

---

## 🚧 Roadmap

| Milestone | Description |
|----------|-------------|
| **M1** | File watcher + safe rename engine |
| **M2** | Rule engine (IF/AND/OR logic) |
| **M3** | PySide6 GUI with theme support |
| **M4** | CI builds, packaging, auto-start integration |
| **M5** | Manual metadata editing interface |
| **M6** | Metadata lookups (MusicBrainz, IMDB, TMDB, etc.) |
| **M7** | Cloud sync monitoring support |
| **M8.x** | Metadata export to databases (MySQL → PostgreSQL) |
| **M9** | Secure media export + intranet-access library |

🟢 **Public releases will begin after Milestone 4**, with updates following each completed milestone!

---

## 🔐 API Key Handling

- MetaMancer supports **central and user-provided API keys**.
- Developers can toggle inclusion of each key during packaging (to remain compliant with third-party ToS).
- Users can override API keys via GUI or config file.

---

## 📂 Project Layout
MetaMancer/
├── core/               # Watcher, Renamer, Rule Engine
├── lookup/             # Music/Video Metadata APIs
├── ui/                 # GUI, themes, animated SVG logo
├── cli/                # Optional CLI utility
├── config/             # Rules & user preferences
├── utils/              # Helpers, logger, API manager
├── tests/              # Unit & integration tests
├── .github/            # Workflows, templates, project config
├── README.md
├── LICENSE
└── logo.svg

---

## 🤝 Contributing

We welcome community contributions! Please:

1. Fork this repo
2. Create a feature branch
3. Submit a pull request with detailed notes
4. For feature ideas, open a GitHub issue or proposal ticket

---

## 📣 Credits

Developed and maintained by **MWBM Partners Ltd.**  
Open to community feature requests, forks, and collaboration!

---

## 📜 License

MIT License – see [LICENSE](LICENSE) file for details.

---