# Custom File Types — MeedyaManager

> **(C) 2025-2026 MWBM Partners Ltd**

MeedyaManager includes a built-in file type registry covering common audio, video, subtitle,
and companion-file formats. If you work with a format that isn't recognised by default, you
can add it to a user override file. This page describes the **real** schema, taken from
`crates/mm-core/src/filetype_registry.rs` and `config/filetypes.json5` — it is not what the
older version of this page (or `custom_filetypes` in `settings.json5`) claimed.

> ⚠️ There is no `meedya debug --list-filetypes` flag — `meedya debug` only inspects a single
> media file (`crates/mm-cli/src/commands/debug.rs`), and no other command lists the registry.
> To see the full built-in list, open `config/filetypes.json5` in your MeedyaManager
> installation, or the same file in the
> [source repository](https://github.com/MWBMPartners/MeedyaManager/blob/main/config/filetypes.json5).

---

## Table of Contents

1. [The Built-In File Type Registry](#the-built-in-file-type-registry)
2. [Overriding the Registry](#overriding-the-registry)
3. [The Real Schema](#the-real-schema)
4. [Examples](#examples)
5. [Disabling a Format](#disabling-a-format)

---

## The Built-In File Type Registry

The built-in registry (`config/filetypes.json5`, embedded into the binary at compile time)
covers four categories:

- **Audio** — MP3, AAC, M4A/M4B/M4R, OGG/OGA, FLAC, ALAC, WAV, AIFF, and others
- **Video** — MP4, MKV, AVI, MOV, WebM, and others
- **Subtitle** — timed subtitles, captions, lyrics, and transcripts
- **Companion** — non-media files that travel alongside a media file when it's renamed (cover
  art, `.nfo`, `.cue`, artist photos, etc.)

There is **no** `custom_filetypes` key in `AppConfig`/`settings.json5`
(`crates/mm-core/src/config/mod.rs:36-59` has no such field) and no `--list-filetypes` CLI
flag. Custom file types are added through a completely separate mechanism: a standalone
`filetypes.json5` override file.

---

## Overriding the Registry

Place a file at:

| Platform | Path |
| -------- | ---- |
| Linux | `~/.config/MeedyaManager/filetypes.json5` |
| macOS | `~/Library/Application Support/MeedyaManager/filetypes.json5` |
| Windows | `%APPDATA%\MeedyaManager\filetypes.json5` |

This is the same single config directory documented in
[configuration.md](configuration.md#configuration-file-location), resolved by
`mm_core::config::app_config_dir()` and overridable with `MM_CONFIG_DIR` (issue #212).

**This file replaces the built-in registry entirely — it does not extend it.** If the file
exists and parses successfully, MeedyaManager uses *only* what's in it; the compiled-in
defaults are not merged in. To add one new format, start by copying the whole built-in
`config/filetypes.json5` into that path, then add your entry to the right array. If your
override file is malformed, MeedyaManager logs a warning and silently falls back to the
built-in defaults (`crates/mm-core/src/filetype_registry.rs:189-206`) — it will not crash on
a bad override.

You must restart MeedyaManager (and the background service, if running) for a changed
override file to be picked up — the registry is parsed once and cached for the life of the
process:

```bash
meedya service stop && meedya service start
```

---

## The Real Schema

The file has four top-level arrays — `audio`, `video`, `subtitle`, `companion` — each holding
objects with **only these fields** (all other field names, including `media_group`,
`format_class`, `media_class`, and `quality_type`, are not part of this schema and are
silently rejected by JSON5 deserialisation, which fails the whole override file):

| Category | Fields |
| -------- | ------ |
| `audio` | `ext`, `mime` (optional), `name`, `lossless` (bool) |
| `video` | `ext`, `mime` (optional), `name` |
| `subtitle` | `ext`, `mime` (optional), `name`, `kind` |
| `companion` | `ext`, `mime` (optional), `name`, `scope` |

All four also accept an optional `enabled` boolean (default `true`) — set it to `false` to
turn off a format without deleting its entry.

`kind` (subtitle only) must be one of: `"subtitle"`, `"caption"`, `"lyrics"`, `"transcript"`.

`scope` (companion only) must be one of: `"track"`, `"album"`, `"artist"` — **not**
`"audio"`/`"video"`/`"any"`. This describes how broadly the companion file travels: with a
single track, with the whole album folder, or with everything by one artist.

---

## Examples

### Register TAK (Tom's Lossless Audio Kompressor) — audio

```json5
{ "ext": "tak", "mime": "audio/x-tak", "name": "TAK", "lossless": true }
```

### Register a video container

```json5
{ "ext": "vob", "mime": "video/dvd", "name": "DVD VOB" }
```

### Register a custom lyrics companion file

```json5
{ "ext": "elrc", "mime": null, "name": "Enhanced LRC", "kind": "lyrics" }
```

(as a `subtitle` entry — lyrics files are a `kind`, not a separate top-level category)

### Register a per-album companion file

```json5
{ "ext": "nfo", "mime": null, "name": "NFO Info File", "scope": "album" }
```

(as a `companion` entry — this travels with the whole album folder, not a single track)

---

## Disabling a Format

Add `"enabled": false` to any entry (built-in or your own) to have MeedyaManager ignore it
everywhere, without deleting the entry:

```json5
{ "ext": "wma", "mime": "audio/x-ms-wma", "name": "WMA", "lossless": false, "enabled": false }
```
