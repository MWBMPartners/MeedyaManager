# Metadata round-trip fixtures

Tiny real media files used by `crates/mm-core/tests/metadata_roundtrip.rs` to
exercise `mm_core::metadata` and `mm_core::integrity` against genuine tag
containers instead of hand-fabricated bytes. Each is ~0.2 seconds of silence —
small enough to commit, large enough for lofty to parse as a real file of its
format.

All generated with `ffmpeg` (8.1.2, Homebrew, `/opt/homebrew/bin/ffmpeg`) from
an `anullsrc` silent source. Regenerate with the commands below, run from this
directory.

| File            | Container / tag format          | Command |
|-----------------|----------------------------------|---------|
| `silence.mp3`   | MP3, ID3v2.4                      | `ffmpeg -y -f lavfi -i anullsrc=r=8000:cl=mono -t 0.2 -c:a libmp3lame -q:a 9 silence.mp3` |
| `silence.flac`  | FLAC, Vorbis comments              | `ffmpeg -y -f lavfi -i anullsrc=r=8000:cl=mono -t 0.2 -c:a flac silence.flac` |
| `silence.m4a`   | MP4/M4A, iTunes atoms (`AAC-LC`)  | `ffmpeg -y -f lavfi -i anullsrc=r=8000:cl=mono -t 0.2 -c:a aac -b:a 32k silence.m4a` |
| `silence.wav`   | WAV, RIFF INFO                     | `ffmpeg -y -f lavfi -i anullsrc=r=8000:cl=mono -t 0.2 -c:a pcm_s16le silence.wav` |
| `cover.png`     | 8x8 solid-blue PNG (cover art test) | `ffmpeg -y -f lavfi -i color=c=blue:s=8x8 -frames:v 1 -update 1 cover.png` |

Total size is well under 1 MB (~28 KB as of writing). Do not replace these
with larger or non-silent audio — the tests only need parseable tag
containers, not audible content.
