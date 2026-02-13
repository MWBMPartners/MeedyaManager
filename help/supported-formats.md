# 🎵 Supported Formats — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd (d/b/a MW Services)**

MeedyaManager supports a wide range of audio, video, and companion file formats.

---

## 🎵 Audio Formats

| Extension | Format Name | Lossy/Lossless | Notes |
| --------- | ----------- | -------------- | ----- |
| `.mp3` | MPEG Audio Layer 3 | Lossy | Most common audio format |
| `.flac` | Free Lossless Audio Codec | Lossless | Open-source lossless |
| `.alac` | Apple Lossless Audio Codec | Lossless | Typically in `.m4a` container |
| `.m4a` | MPEG-4 Audio | Both | AAC (lossy) or ALAC (lossless) |
| `.ogg` | Ogg Vorbis | Lossy | Open-source alternative to MP3 |
| `.opus` | Opus Audio | Lossy | Modern, efficient codec |
| `.wav` | Waveform Audio | Lossless (PCM) | Uncompressed audio |
| `.aiff` | Audio Interchange File Format | Lossless (PCM) | Apple's uncompressed format |
| `.aac` | Advanced Audio Coding | Lossy | Common in streaming |
| `.wma` | Windows Media Audio | Both | Microsoft format |
| `.ac3` | Dolby Digital (AC-3) | Lossy | 5.1 surround sound |
| `.eac3` | Dolby Digital Plus (E-AC-3) | Lossy | Enhanced Dolby Digital |
| `.ac4` | Dolby AC-4 | Lossy | Next-gen Dolby codec |
| `.mka` | Matroska Audio | Both | Audio-only Matroska container |
| `.dts` | DTS Digital Surround | Lossy | Surround sound codec |

---

## 🎬 Video Formats

| Extension | Format Name | Container | Notes |
| --------- | ----------- | --------- | ----- |
| `.mp4` | MPEG-4 Part 14 | MP4 | Most common video container |
| `.m4v` | MPEG-4 Video | MP4 | Apple's MP4 variant |
| `.mkv` | Matroska Video | Matroska | Flexible open container |
| `.avi` | Audio Video Interleave | AVI | Legacy Microsoft format |
| `.divx` | DivX Video | AVI/MKV | DivX-encoded video |
| `.mpg` / `.mpeg` | MPEG Video | MPEG | Legacy MPEG-1/2 |
| `.hevc` | High Efficiency Video Coding | Various | H.265 video codec |
| `.mov` | QuickTime Movie | QuickTime | Apple's native video format |
| `.wmv` | Windows Media Video | ASF | Microsoft video format |
| `.webm` | WebM Video | WebM | Google's open web format |
| `.ts` | MPEG Transport Stream | MPEG-TS | Broadcast/streaming format |

---

## 🔊 Audio Characteristics Detection

MeedyaManager can detect and classify these audio properties:

### Lossy vs Lossless

| Quality Type | Codecs | Detection Method |
| ------------ | ------ | ---------------- |
| **Lossless** | FLAC, ALAC, WAV, AIFF, WMA Lossless | Codec identification via MediaInfo |
| **Lossy** | MP3, AAC, OGG Vorbis, Opus, WMA, AC3, EAC3 | Codec identification via MediaInfo |

### Multichannel Formats

| Format | Codec | Typical Channels |
| ------ | ----- | ---------------- |
| **Dolby Digital** | AC-3 | 5.1 |
| **Dolby Digital Plus** | E-AC-3 | 5.1, 7.1 |
| **Dolby AC-4** | AC-4 | 5.1, 7.1, Atmos |
| **DTS** | DTS | 5.1 |
| **DTS-HD** | DTS-HD MA | 5.1, 7.1 |

### Spatial Audio

| Format | Detection | Notes |
| ------ | --------- | ----- |
| **Dolby Atmos** | Extended codec metadata in E-AC-3/AC-4 | Object-based spatial |
| **Sony 360 Reality Audio** | MPEG-H 3D Audio markers | Sony's spatial format |
| **Apple Spatial Audio** | Dolby Atmos in MP4/M4A container | Apple's implementation |

### Dolby Vision (Video)

| Profile | Description | Detection |
| ------- | ----------- | --------- |
| Profile 5 | Single-layer HDR | HDR metadata flags |
| Profile 7 | Dual-layer (with HDR10 base) | HDR metadata flags |
| Profile 8 | Single-layer (with HDR10/SDR base) | HDR metadata flags |

---

## 📝 Companion File Formats

These files are tracked alongside media files and moved together:

### Subtitles

| Extension | Format | Notes |
| --------- | ------ | ----- |
| `.srt` | SubRip | Most common subtitle format |
| `.lrc` | LRC Lyrics | Timed lyrics for music |
| `.sub` | MicroDVD / SubViewer | Legacy subtitle format |
| `.ass` / `.ssa` | Advanced SubStation Alpha | Styled subtitles |
| `.vtt` | WebVTT | Web-standard subtitles |

### Cover Art & Images

| Extension | Format | Notes |
| --------- | ------ | ----- |
| `.jpg` / `.jpeg` | JPEG | Most common cover art |
| `.png` | PNG | Higher quality, transparency |
| `.bmp` | Bitmap | Legacy format |
| `cover.mp4` | Animated Cover Art | Animated album art (square/portrait) |

### Disc Images

| Extension | Format | Notes |
| --------- | ------ | ----- |
| `.iso` | ISO 9660 | Standard disc image |
| `.nrg` | Nero Image | Nero burning format |

### Other

| Extension | Format | Notes |
| --------- | ------ | ----- |
| `.cue` | Cue Sheet | Track listing for disc images |
| `.nfo` | Info File | Release information |
| `.pdf` | PDF Booklet | Album booklets, liner notes |
| `.log` | EAC/XLD Log | Ripping verification log |

---

## 🔧 Companion File Behaviour

When MeedyaManager moves media files, companion files in the same directory are handled as follows:

1. **All media files moved** — All companion files are also moved to the destination
2. **Some media files moved** — Companion files stay with the remaining media
3. **Companion file types** are configured in `settings.json5` and can be customised

This ensures subtitles, cover art, and other associated files always stay with their media.

---

> 📝 *Format support is continuously expanded. If you need a format not listed here, please [open an issue](https://github.com/MWBMPartners/MeedyaManager/issues/new?template=feature_request.md).*
