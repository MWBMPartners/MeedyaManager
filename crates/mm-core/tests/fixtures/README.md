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
| `riff_language.wav` | WAV, a genuine RIFF INFO `ILNG` chunk carrying `fre` — used for the "keep every tag container consistent" test (`write_tags`, TRACK-070, item 5 of the language-policy review). `write_tags` on a `.wav` puts a brand new `language` value into an embedded ID3v2 tag (see the note on `wav_write_tags_uses_embedded_id3v2_not_riff_info` below) rather than RIFF INFO, so this fixture is the only way to test a file that has RIFF INFO's own `ILNG` from the start. | `ffmpeg -y -f lavfi -i anullsrc=r=8000:cl=mono -t 0.2 -c:a pcm_s16le -metadata language=fre riff_language.wav` |
| `cover.png`     | 8x8 solid-blue PNG (cover art test) | `ffmpeg -y -f lavfi -i color=c=blue:s=8x8 -frames:v 1 -update 1 cover.png` |

### Files whose tags disagree about the language

MeedyaManager's own writing code keeps every tag in a file in step, so it can
never make a file whose tags disagree about the language — but real files do
(two different tools each wrote one tag). The third independent review of the
language-policy work (issue #251) found MeedyaManager showed one answer and
hid the other. These five files test that, and are built byte by byte from
the files above by `make_language_fixtures.py` in this folder, using only
Python's standard library — deliberately not `lofty`, the library the app
itself uses, so a fault in `lofty` cannot hide a fault in the app. Regenerate
all nine (these five and the four in the next table) with
`python3 make_language_fixtures.py`; the output is the same every time (the
first three were checked to come out byte for byte the same when the next two
were added, for the fourth review round, and all five again when the last
four were added, for Codex's catch-up review).

| File | What is in it | Used for |
|------|---------------|----------|
| `lang_riff_fre_id3_ger.wav` | `riff_language.wav` (RIFF INFO `ILNG` = `fre`) plus an embedded ID3v2.4 tag with `TLAN` = `ger` | The tags disagree: `fre` is shown, `ger` must be reported, and resending `fre` must leave the ID3 tag alone |
| `lang_vorbis_eng_id3_ger.flac` | `silence.flac` with its Vorbis `LANGUAGE` set to `eng`, and an ID3v2.4 tag with `TLAN` = `ger` placed before the FLAC data, as some older tools write them | The same disagreement in a FLAC. MeedyaManager can read this file but cannot yet save one like it (its own issue) |
| `lang_riff_english_id3_eng.wav` | `silence.wav` with its RIFF INFO replaced by `ILNG` = `English` (a word, not a code) and `INAM` = `Old`, plus an ID3v2.4 tag with `TLAN` = `eng` | A value nothing recognises beside a real code; also a program that reads every field and writes them all back with only the title changed must not be refused |
| `lang_riff_english_id3_english.wav` | `silence.wav` with `ILNG` = `English` and `INAM` = `Old`, plus an ID3v2.4 tag with `TLAN` = `English` | The same unrecognised word in both tags is agreement: no disagreement may be reported |
| `lang_riff_en_id3_ger_twice.wav` | `silence.wav` with `ILNG` = `en` and `INAM` = `Old`, plus an ID3v2.4 tag whose `TLAN` holds `ger` twice (two values, separated by a zero byte) | The disagreement must name `ger` once, not twice |

### Files for Codex's catch-up review of the language-policy branch

Built by the same script (with one more, `lang_riff_two_info_lists.wav`, at the end of
this table). Each of the first four reproduced a finding with the `meedya`
binary built from `a150926`, before the fix (the evidence is in the commit
that added the files).

| File | What is in it | Used for |
|------|---------------|----------|
| `lang_riff_fre_title_latin1.wav` | `silence.wav` with RIFF INFO `ILNG` = `fre` and `INAM` (the title) = "Café" written in Latin-1, the bytes `43 61 66 E9`, not UTF-8 | Finding 1: changing or clearing the language used to delete the title, because the tag library cannot read it and wrote the list back without it. It must now be refused, with the file left exactly as it was |
| `lang_riff_fre_all_utf8.wav` | `silence.wav` with RIFF INFO `IART`, `INAM`, `ILNG` = `fre`, `ICMT` and `ISFT`, all UTF-8, with odd and even lengths | Finding 1's other side: nothing is lost, so the save goes ahead, and every other entry stays byte for byte the same and in the same order |
| `lang_ape_eng_fra.mp3` | `silence.mp3` with an APE version 2 tag at the end whose `Language` item holds `eng`, a zero byte, then `fra` — two values in one item, as APE stores them | Finding 4: the two values must be read as two, in order, the way an ID3 tag's two values are |
| `lang_vorbis_nbsp_en.flac` | `silence.flac` with its Vorbis `LANGUAGE` set to a no-break space (U+00A0) followed by `en` | Finding 5: only the policy's own four whitespace characters are trimmed, so this value is malformed and must not be read as English |
| `lang_riff_two_info_lists.wav` | `silence.wav` with two `LIST INFO` chunks: the first holds only `ISFT`, the second `ILNG` = `fre` and `INAM` = `Old` | Issue #259: the tag library rewrites only the first chunk and copies the second's entries into it. Since finding 1's fix, a language save is refused instead, leaving the file as it was |

Total size is well under 1 MB (~28 KB as of writing). Do not replace these
with larger or non-silent audio — the tests only need parseable tag
containers, not audible content.
