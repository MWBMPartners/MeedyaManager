# 🎵 Supported Formats — MeedyaManager

> **(C) 2025–2026 MWBM Partners Ltd**

MeedyaManager recognises the audio, video, subtitle, and companion file formats listed below.
This page reflects the actual registry in `config/filetypes.json5`
(`crates/mm-core/src/filetype_registry.rs`) — see [Custom File Types](custom-filetypes.md) if
you want to add a format that isn't here.

---

## 🎵 Audio Formats

### Lossy compressed

| Extension | Format Name |
| --------- | ----------- |
| `.mp3` | MP3 |
| `.aac` | AAC |
| `.m4a` | M4A (AAC or ALAC container) |
| `.m4b` | M4B (Audiobook) |
| `.m4r` | M4R (iPhone Ringtone) |
| `.ogg`, `.oga` | Ogg Vorbis / Ogg Audio |
| `.opus` | Opus |
| `.wma` | WMA |
| `.amr` | AMR |
| `.3gp` | 3GPP Audio |
| `.mp2` | MP2 |
| `.ra` | RealAudio |
| `.mpc` | Musepack |
| `.spx` | Speex |
| `.snd` | NeXT/Sun Audio |

### Lossless compressed

| Extension | Format Name |
| --------- | ----------- |
| `.flac` | FLAC |
| `.alac` | ALAC |
| `.ape` | Monkey's Audio (APE) |
| `.wv` | WavPack |
| `.tta` | True Audio (TTA) |

### Uncompressed / PCM

| Extension | Format Name |
| --------- | ----------- |
| `.wav` | WAV (PCM) |
| `.aiff`, `.aif` | AIFF |
| `.aifc` | AIFF-C |
| `.au` | Sun AU |
| `.caf` | Core Audio Format |

### Tracker / chiptune / MIDI

| Extension | Format Name |
| --------- | ----------- |
| `.mod` | MOD Tracker |
| `.xm` | XM Tracker |
| `.it` | Impulse Tracker |
| `.s3m` | ScreamTracker 3 |
| `.mid`, `.midi` | MIDI |

There is **no** support for Dolby Digital (`.ac3`), Dolby Digital Plus (`.eac3`), Dolby AC-4
(`.ac4`), DTS (`.dts`), or Matroska Audio (`.mka`) — none of these extensions appear in the
registry.

---

## 🎬 Video Formats

| Extension | Format Name |
| --------- | ----------- |
| `.mp4` | MPEG-4 Video |
| `.m4v` | M4V (iTunes Video) |
| `.mkv` | Matroska |
| `.webm` | WebM |
| `.avi` | AVI |
| `.mov` | QuickTime MOV |
| `.wmv` | Windows Media Video |
| `.flv` | Flash Video |
| `.f4v` | Flash MP4 Video |
| `.mpg`, `.mpeg` | MPEG Video |
| `.ts` | MPEG Transport Stream |
| `.m2ts`, `.mts` | AVCHD / MPEG-2 Transport Stream |
| `.vob` | DVD Video Object |
| `.rmvb`, `.rm` | RealMedia |
| `.3gp` | 3GPP Video |
| `.ogv` | Ogg Video |
| `.divx`, `.xvid` | DivX / Xvid Video |
| `.dv` | Digital Video (DV) |
| `.hevc` | HEVC / H.265 |
| `.heic` | HEIC (Apple video still) |
| `.avif` | AVIF |

---

## 🔊 Audio Quality Classification

MeedyaManager classifies quality using the `lossless` flag recorded per audio format in the
registry, combined with the file's decoded bitrate (`crates/mm-core/src/classify/mod.rs`):
`Lossless`, `HiRes` (24-bit/88.2kHz+ or DSD), and lossy tiers `Lossy320`/`Lossy256`/`Lossy192`/
`Lossy128`/`LossyLow`. This is a straightforward registry lookup plus a bitrate comparison —
there is **no MediaInfo integration** (issue
[#130](https://github.com/MWBMPartners/MeedyaManager/issues/130), open) and no codec-name
detection beyond the file extension.

**Not implemented** (all open issues — do not expect these today):

- **Multichannel format detection** (Dolby Digital, Dolby Digital Plus, DTS, DTS-HD) — there
  is no channel-layout naming anywhere in the codebase.
- **Spatial audio detection** (Dolby Atmos, Sony 360 Reality Audio, Apple Spatial Audio) — see
  issue [#131](https://github.com/MWBMPartners/MeedyaManager/issues/131), open.
- **Dolby Vision profile detection** (Profile 5/7/8) for video — see issue
  [#164](https://github.com/MWBMPartners/MeedyaManager/issues/164), open.

---

## 📝 Companion File Formats

These files travel alongside a media file when it's renamed or moved. Full details on scope
(`track`/`album`/`artist`) are in [Custom File Types](custom-filetypes.md#the-real-schema).

### Subtitles, Captions, Lyrics, Transcripts

| Extension | Format | Kind |
| --------- | ------ | ---- |
| `.srt` | SubRip | subtitle |
| `.sub` | MicroDVD / SubViewer | subtitle |
| `.ass` / `.ssa` | (Advanced) SubStation Alpha | subtitle |
| `.vtt` | WebVTT | subtitle |
| `.idx` | VobSub Index | subtitle |
| `.smi` | SAMI | subtitle |
| `.ttml` / `.dfxp` | Timed Text / DFXP | subtitle |
| `.sbv`, `.srv1`, `.srv2`, `.srv3` | YouTube caption formats | caption |
| `.cap` | Caption | caption |
| `.lrc` | Timed Lyrics | lyrics |
| `.elrc` | Enhanced LRC | lyrics |
| `.txt` | Plain Text Transcript | transcript |

### Disc Images & Archives

| Extension | Format |
| --------- | ------ |
| `.iso`, `.bin`, `.img`, `.nrg`, `.mdf`, `.mds`, `.daa`, `.udf` | Disc/optical images |
| `.zip`, `.rar`, `.7z`, `.tar`, `.gz` | Archives (often full album release packages) |
| `.itlp`, `.itmsp`, `.itms` | Apple iTunes LP / Music Store packages |

### Info, Logs, and Playlists

| Extension | Format |
| --------- | ------ |
| `.cue` | Cue Sheet |
| `.nfo` | Release Info |
| `.sfv`, `.md5` | Checksum files |
| `.log` | Rip log (EAC/XLD/dBpoweramp) |
| `.accurip`, `.crc` | Accuracy-check files |
| `.m3u`, `.m3u8`, `.pls`, `.xspf`, `.wpl`, `.asx` | Playlists |

There is **no** `.pdf` (booklet) or `.nrg`-only-style entry beyond what's listed above.

### Cover Art (Detected Separately)

Cover art is **not** part of the extension registry above — it's detected by filename pattern
in `crates/mm-core/src/companion/mod.rs`. A file qualifies if its extension is one of `.jpg`,
`.jpeg`, `.png`, `.gif`, `.bmp`, `.webp`, `.tiff`/`.tif` **and** its filename stem
(case-insensitive) is one of: `cover`, `folder`, `album`, `front`, `back`, `inlay`, `booklet`,
`artwork`, `thumb`, `thumbnail`, or `poster` — e.g. `cover.jpg`, `Folder.PNG`, `front.webp`. A
file like `vacation.jpg` or `photo.png` is **not** treated as cover art. There is no animated
cover art (`.mp4`) support of any kind.

---

## 🔧 Companion File Behaviour

When MeedyaManager moves a media file, companion files found alongside it in the same
directory are classified and moved based on the `scope` described above — a `track`-scoped
file follows a single renamed file, while `album`/`artist`-scoped files apply more broadly.
There is no `settings.json5` key for customising this behaviour directly — to change which
extensions are recognised as companions, edit your `filetypes.json5` override (see
[Custom File Types](custom-filetypes.md)).

---

> 📝 *Format support is continuously expanded. If you need a format not listed here, please [open an issue](https://github.com/MWBMPartners/MeedyaManager/issues/new?template=feature_request.md).*
