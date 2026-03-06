# Custom File Types — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager includes a built-in file type registry covering hundreds of audio, video, image, and companion file formats. If you work with a format that isn't recognised by default, you can register it as a custom file type.

---

## Table of Contents

1. [The Built-In File Type Registry](#the-built-in-file-type-registry)
2. [Registering a Custom File Type](#registering-a-custom-file-type)
3. [Custom File Type Fields](#custom-file-type-fields)
4. [Examples](#examples)
5. [Companion File Scopes](#companion-file-scopes)

---

## The Built-In File Type Registry

MeedyaManager's built-in registry covers:

- **Audio:** MP3, FLAC, ALAC, M4A, AAC, OGG, Opus, WAV, AIFF, WMA, AC3, EAC3, DTS, MKA, and more
- **Video:** MP4, M4V, MKV, AVI, MOV, WMV, WebM, HEVC, TS, MPG, and more
- **Subtitles:** SRT, ASS, SSA, VTT, LRC, SUB, and more
- **Cover art:** JPG, PNG, WebP, TIFF
- **Disc images:** ISO, IMG, BIN/CUE
- **Playlists:** M3U, M3U8, PLS, XSPF

View the full list:

```bash
meedya debug --list-filetypes
```

---

## Registering a Custom File Type

Add custom file types in `settings.json5` under a `custom_filetypes` section (this is stored in the `SettingsBundle` exported via `meedya config export`):

```json5
// In settings.json5 — custom_filetypes extends the built-in registry
custom_filetypes: [
  {
    extension: "tak",
    mime_type: "audio/x-tak",
    media_group: "Audio",
    format_class: "TAK",
    media_class: "Music",
    quality_type: "Lossless",
    companion_scope: null
  },
  {
    extension: "dsf",
    mime_type: "audio/x-dsf",
    media_group: "Audio",
    format_class: "DSF",
    media_class: "Music",
    quality_type: "Lossless",
    companion_scope: null
  }
]
```

After adding custom file types, restart the background service (or restart the app) for the changes to take effect:

```bash
meedya service stop && meedya service start
```

---

## Custom File Type Fields

| Field | Required | Description |
| ----- | -------- | ----------- |
| `extension` | Yes | File extension without the leading dot (e.g. `"tak"`) |
| `mime_type` | Yes | MIME type (e.g. `"audio/x-tak"`) |
| `media_group` | Yes | Top-level group: `"Audio"`, `"Video"`, `"Image"`, `"Document"` |
| `format_class` | Yes | Codec/format name (e.g. `"TAK"`, `"DSF"`, `"DXD"`) |
| `media_class` | Yes | Content class: `"Music"`, `"Movie"`, `"TV Show"`, `"Podcast"`, etc. |
| `quality_type` | Yes | `"Lossless"`, `"Lossy"`, `"Uncompressed"`, `"Unknown"` |
| `companion_scope` | No | Scope for companion file pairing (see below) |

---

## Examples

### Register TAK (Tom's Lossless Audio Kompressor)

```json5
{
  extension: "tak",
  mime_type: "audio/x-tak",
  media_group: "Audio",
  format_class: "TAK",
  media_class: "Music",
  quality_type: "Lossless",
  companion_scope: null
}
```

### Register DXD (Digital eXtreme Definition audio)

```json5
{
  extension: "dxd",
  mime_type: "audio/x-dxd",
  media_group: "Audio",
  format_class: "DXD",
  media_class: "Music",
  quality_type: "Uncompressed",
  companion_scope: null
}
```

### Register a custom subtitle format as a companion file

```json5
{
  extension: "sup",
  mime_type: "application/x-sup",
  media_group: "Document",
  format_class: "SUP",
  media_class: "Subtitle",
  quality_type: "Unknown",
  companion_scope: "video"  // treated as a companion to video files
}
```

---

## Companion File Scopes

When `companion_scope` is set, MeedyaManager treats this file type as a companion to other media files and moves it alongside them when the primary file is renamed.

| Scope | Paired with |
| ----- | ----------- |
| `"audio"` | Audio files (MP3, FLAC, etc.) |
| `"video"` | Video files (MKV, MP4, etc.) |
| `"any"` | Any media file |
| `null` | Not treated as a companion (standalone file) |

For example, an `.lrc` (lyrics) file is a companion to audio, and an `.srt` (subtitle) file is a companion to video. If a companion file is found next to a media file that MeedyaManager renames, the companion is moved to the same destination directory automatically.
