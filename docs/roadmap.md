# 📍 MetaMancer Development Roadmap
> (C) 2025 MWBM Partners Ltd (d/b/a MW Services)

This document outlines the official milestone-based roadmap for **MetaMancer**: a smart, cross-platform metadata-driven media organizer.

## ✅ Released
- **M1** – Real-time File Watcher
  - Monitor folders
  - Detect file creation/move
  - Avoid locked/in-use files
  - Queue events for renaming

## 🚧 In Progress
- **M2** – Rule Engine
  - Logic-based renaming templates
  - Support for IF, AND, OR, nested expressions
  - Configurable via JSON5/TOML

## 🔜 Upcoming Milestones

### **M3 – GUI + Theming**
- PySide6 desktop interface
- Light/dark mode support (system & manual)
- Rule editor & file preview interface

### **M4 – CI/CD + Cross-Platform Packaging**
- Auto-builds (Windows x64/ARM, macOS Apple Silicon, Linux)
- GitHub Actions
- Versioned releases w/ artifacts

### **M5 – Metadata Editing**
- Full tag editor UI
- Multi-value tag handling
- Format-safe editing (MP3, MP4/M4A, FLAC, MKV, etc.)

### **M6 – Music & Video Metadata Lookup**
- 🎵 MusicBrainz, Apple Music, Spotify, Tidal, Amazon Music, Shazam
- 🎬 TheMovieDB, TheTVDB, Apple TV, IMDb, EIDR
- Save service URLs + IDs in custom tags
- Download & embed animated cover art (MP4 square/portrait)

### **M7 – Cloud Sync Support**
- OneDrive, SharePoint, Dropbox, Google Drive, Mega.nz, iCloud
- Monitor cloud folders
- Sync-safe sorting behavior

### **M8 – Metadata Export to External DB**
- MySQL → MariaDB → SQL Server → SQLite → PostgreSQL
- Structured export of full library + custom tags

### **M9 – Secure Web Access + Media Copy**
- Optional media export
- Highly secure with user access control
- Multi-format export: FLAC, ALAC, MP3/M4A

---

## 📘 Notes
- Milestone numbering indicates development sequence.
- Public release artifacts will be tagged: `v1.0-M1`, `v1.0-M2`, etc.
- Feedback-driven enhancements may appear mid-milestone.

---

_Last updated: 2025-06-16_