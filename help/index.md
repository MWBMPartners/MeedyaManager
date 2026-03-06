# MeedyaManager Help Centre

> **(C) 2025-2026 MWBM Partners Ltd**

Welcome to the MeedyaManager documentation. Choose a topic below to get started.

---

## Getting Started

| Guide | Description |
| ----- | ----------- |
| [getting-started.md](getting-started.md) | Install MeedyaManager, run your first scan, and configure watch folders |
| [configuration.md](configuration.md) | Full reference for all settings in `settings.json5` |
| [cli-reference.md](cli-reference.md) | Every `meedya` command, subcommand, and flag |

---

## Core Features

| Guide | Description |
| ----- | ----------- |
| [rule-syntax.md](rule-syntax.md) | Template language reference — `<Tag>`, `$If`, `$Replace`, `$Pad`, and more |
| [supported-formats.md](supported-formats.md) | All recognised audio, video, image, and companion file formats |
| [background-service.md](background-service.md) | Run MeedyaManager as a system service (systemd, launchd, Windows Service) |
| [file-integrity.md](file-integrity.md) | SHA256 integrity checking and atomic rename safety |
| [settings-export-import.md](settings-export-import.md) | Export and import your configuration as a portable `.mmprofile` bundle |

---

## Customisation

| Guide | Description |
| ----- | ----------- |
| [custom-tags.md](custom-tags.md) | Define and use unlimited custom metadata tags |
| [custom-filetypes.md](custom-filetypes.md) | Register custom file extensions with their MIME type and classification |

---

## Metadata Providers

| Guide | Description |
| ----- | ----------- |
| [providers/musicbrainz.md](providers/musicbrainz.md) | MusicBrainz — free open music database (no key required) |
| [providers/spotify.md](providers/spotify.md) | Spotify — music metadata and cover art |
| [providers/apple-music.md](providers/apple-music.md) | Apple Music — music metadata and artwork |
| [providers/tidal.md](providers/tidal.md) | Tidal — high-fidelity music metadata |
| [providers/deezer.md](providers/deezer.md) | Deezer — music search (no key required) |
| [providers/amazon-music.md](providers/amazon-music.md) | Amazon Music — music catalogue |
| [providers/youtube-music.md](providers/youtube-music.md) | YouTube Music — music search |
| [providers/iheart.md](providers/iheart.md) | iHeart — music and podcast metadata |
| [providers/pandora.md](providers/pandora.md) | Pandora — music metadata |
| [providers/shazam.md](providers/shazam.md) | Shazam — acoustic recognition |
| [providers/isrc.md](providers/isrc.md) | ISRC — International Standard Recording Code |
| [providers/iswc.md](providers/iswc.md) | ISWC — International Standard Musical Work Code |
| [providers/tmdb.md](providers/tmdb.md) | TMDb — movie and TV show metadata |
| [providers/tvdb.md](providers/tvdb.md) | TVDB — TV series metadata |
| [providers/imdb.md](providers/imdb.md) | IMDb — movie and TV metadata |
| [providers/apple-tv.md](providers/apple-tv.md) | Apple TV — film and TV catalogue |
| [providers/itunes-store.md](providers/itunes-store.md) | iTunes Store — digital media catalogue |
| [providers/eidr.md](providers/eidr.md) | EIDR — Entertainment Identifier Registry |
| [providers/apple-podcasts.md](providers/apple-podcasts.md) | Apple Podcasts — podcast metadata |

---

## Help and Support

| Guide | Description |
| ----- | ----------- |
| [troubleshooting.md](troubleshooting.md) | Common issues and their solutions |
| [faq.md](faq.md) | Frequently asked questions |

---

## Quick Reference

```bash
# Inspect a file
meedya debug ~/Music/song.mp3

# Preview renames for a directory
meedya scan ~/Music --dry-run

# Watch folders live
meedya watch

# Look up metadata online
meedya lookup ~/Music/song.mp3

# Edit tags
meedya edit ~/Music/song.mp3 --tag "Artist=My Artist"

# Manage background service
meedya service install
meedya service start
meedya service status

# Export/import configuration
meedya config export --out ~/my-settings.mmprofile
meedya config import ~/my-settings.mmprofile

# Generate a bug report
meedya report-bug
```
