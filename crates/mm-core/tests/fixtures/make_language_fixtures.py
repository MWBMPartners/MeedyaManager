#!/usr/bin/env python3
# (C) 2025-2026 MWBM Partners Ltd
#
# MeedyaManager — crates/mm-core/tests/fixtures/make_language_fixtures.py
#
# Builds ten language test files from the committed `silence.wav`,
# `silence.flac`, `silence.mp3` and `riff_language.wav`, using nothing but
# Python's standard library. (Three since the third review round; two more
# added for the fourth; four more for Codex's catch-up review of the whole
# language-policy branch — see "The fixtures" at the bottom for what each one
# is for.) Run it from anywhere:
#
#     python3 crates/mm-core/tests/fixtures/make_language_fixtures.py
#
# WHY THESE FILES EXIST
# ---------------------
# MeedyaManager's own writing code keeps every tag in a file in step (that is
# the whole point of it), so it can never produce a file whose tags DISAGREE
# about the language. Real files do disagree — another tool wrote one tag,
# a person or a second tool wrote the other — and the third independent
# review of the language-policy work (issue #251) found MeedyaManager showed
# one of the two answers and silently hid the other. Testing that needs files
# built by something other than MeedyaManager, byte by byte, which is what
# this script does. It deliberately does not use the `lofty` tag library the
# app itself uses, so a fault in how `lofty` writes cannot hide a fault in
# how MeedyaManager reads.
#
# WHAT IT CANNOT DO
# -----------------
# It only knows the few pieces of the WAV, FLAC, ID3 and APE formats these
# files need (one text frame per ID3 tag, UTF-8 text, the Vorbis comment
# block, RIFF INFO entries given as exact bytes, and one APE version 2 tag at
# the end of a file). It is not a general tag writer and should not be grown
# into one.
#
# License: GPL-2.0-or-later

from __future__ import annotations

import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent


# ---------------------------------------------------------------------------
# ID3 version 2.4
# ---------------------------------------------------------------------------


def _syncsafe(n: int) -> bytes:
    """ID3v2.4 writes sizes 7 bits to a byte (the top bit of every byte is
    always zero), so a size can never be mistaken for the start of MP3
    audio. Four bytes, largest part first."""
    return bytes([(n >> 21) & 0x7F, (n >> 14) & 0x7F, (n >> 7) & 0x7F, n & 0x7F])


def id3v24_tag(frames: list[tuple[str, str]]) -> bytes:
    """A complete ID3v2.4 tag holding one text frame per `(frame id, text)`.

    Each frame's text starts with the encoding byte 3, meaning UTF-8 — the
    only encoding this script needs. No padding and no extended header."""
    body = b""
    for frame_id, text in frames:
        data = b"\x03" + text.encode("utf-8")
        body += frame_id.encode("ascii") + _syncsafe(len(data)) + b"\x00\x00" + data
    return b"ID3\x04\x00\x00" + _syncsafe(len(body)) + body


# ---------------------------------------------------------------------------
# WAV (RIFF)
# ---------------------------------------------------------------------------


def _chunk(chunk_id: bytes, payload: bytes) -> bytes:
    """One RIFF chunk. A chunk with an odd length is followed by one padding
    byte that its own length does not count — easy to forget, and a reader
    then misreads every chunk after it."""
    pad = b"\x00" if len(payload) & 1 else b""
    return chunk_id + struct.pack("<I", len(payload)) + payload + pad


def _wav_chunks(data: bytes) -> list[tuple[bytes, bytes]]:
    """Every top-level chunk of a WAV file, in order, as `(id, payload)`."""
    if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        raise ValueError("not a WAV file")
    chunks, pos = [], 12
    while pos + 8 <= len(data):
        chunk_id = data[pos : pos + 4]
        size = struct.unpack("<I", data[pos + 4 : pos + 8])[0]
        chunks.append((chunk_id, data[pos + 8 : pos + 8 + size]))
        pos += 8 + size + (size & 1)
    return chunks


def _wav_from_chunks(chunks: list[tuple[bytes, bytes]]) -> bytes:
    """Reassemble a WAV file, with its overall size recalculated."""
    body = b"WAVE" + b"".join(_chunk(cid, payload) for cid, payload in chunks)
    return b"RIFF" + struct.pack("<I", len(body)) + body


def riff_info(pairs: list[tuple[str, str | bytes]]) -> bytes:
    """The payload of a `LIST` chunk of type `INFO`: one sub-chunk per
    `(four-letter id, text)`, each text ending in a zero byte as RIFF INFO
    expects.

    Text given as `str` is written as UTF-8. Text given as `bytes` is written
    exactly as given — RIFF INFO names no text encoding at all, and older
    Windows tools write the computer's own code page, so a Latin-1 "Café"
    (`43 61 66 E9`) is a real thing to find in a file, and the only way to
    test what happens to one is to write those exact bytes (Codex's catch-up
    review, finding 1)."""
    return b"INFO" + b"".join(
        _chunk(
            cid.encode("ascii"),
            (text if isinstance(text, bytes) else text.encode("utf-8")) + b"\x00",
        )
        for cid, text in pairs
    )


def wav_with(
    source: Path, info: list[tuple[str, str | bytes]] | None, id3: bytes | None
) -> bytes:
    """`source`'s audio with its own `LIST INFO` chunk replaced by `info` (if
    given) and an `ID3 ` chunk holding `id3` added at the end (if given).

    The old INFO chunk is REPLACED, not added to: a WAV with two INFO chunks
    is a different, separate problem (MeedyaManager only updates the first
    one), and these files must not trip over it by accident."""
    kept = [
        (cid, payload)
        for cid, payload in _wav_chunks(source.read_bytes())
        if not (info is not None and cid == b"LIST" and payload[:4] == b"INFO")
    ]
    if info is not None:
        kept.append((b"LIST", riff_info(info)))
    if id3 is not None:
        # "ID3 " (upper case) is what `lofty` itself writes, so a later save
        # by MeedyaManager replaces this chunk rather than adding a second.
        kept.append((b"ID3 ", id3))
    return _wav_from_chunks(kept)


# ---------------------------------------------------------------------------
# FLAC
# ---------------------------------------------------------------------------


def flac_with_vorbis(source: Path, comments: list[tuple[str, str]]) -> bytes:
    """`source` with every Vorbis comment named in `comments` replaced by the
    given values (other comments, and the vendor string, are kept).

    A FLAC file is "fLaC" followed by metadata blocks, each with a four-byte
    header: one bit "this is the last block", seven bits of block type (4 is
    the Vorbis comment block), then three bytes of length."""
    data = source.read_bytes()
    if data[:4] != b"fLaC":
        raise ValueError("not a FLAC file")
    out, pos = bytearray(b"fLaC"), 4
    replaced = {name.upper() for name, _ in comments}
    while True:
        header = data[pos]
        is_last, block_type = header & 0x80, header & 0x7F
        length = int.from_bytes(data[pos + 1 : pos + 4], "big")
        block = data[pos + 4 : pos + 4 + length]
        if block_type == 4:
            vendor_len = struct.unpack("<I", block[:4])[0]
            vendor = block[4 : 4 + vendor_len]
            count = struct.unpack("<I", block[4 + vendor_len : 8 + vendor_len])[0]
            at, existing = 8 + vendor_len, []
            for _ in range(count):
                n = struct.unpack("<I", block[at : at + 4])[0]
                existing.append(block[at + 4 : at + 4 + n])
                at += 4 + n
            kept = [c for c in existing if c.split(b"=", 1)[0].upper() not in replaced]
            kept += [f"{name}={value}".encode("utf-8") for name, value in comments]
            block = (
                struct.pack("<I", len(vendor))
                + vendor
                + struct.pack("<I", len(kept))
                + b"".join(struct.pack("<I", len(c)) + c for c in kept)
            )
        out += bytes([is_last | block_type]) + len(block).to_bytes(3, "big") + block
        pos += 4 + length
        if is_last:
            break
    return bytes(out) + data[pos:]


# ---------------------------------------------------------------------------
# APE version 2
# ---------------------------------------------------------------------------


def apev2_tag(items: list[tuple[str, bytes]]) -> bytes:
    """A complete APE version 2 tag (a header, the items, a footer) holding
    one UTF-8 text item per `(key, value)`, for the END of a file.

    The value is given as exact bytes because the point of the one file that
    uses this is a value with a zero byte inside it: APE stores several
    values in ONE item, separated by zero bytes, where an ID3 tag would keep
    them as separate values (Codex's catch-up review, finding 4).

    Layout, all numbers little-endian: the header and the footer are each 32
    bytes — "APETAGEX", version 2000, the size of the items plus the footer
    (not the header), the item count, flags, and 8 zero bytes. The flags say
    "this tag has a header" (top bit) and, on the header only, "this is the
    header" (bit 29). Each item is its value's length, its own flags (0
    means UTF-8 text), the key, a zero byte, then the value."""
    body = b"".join(
        struct.pack("<II", len(value), 0) + key.encode("ascii") + b"\x00" + value
        for key, value in items
    )
    size = len(body) + 32

    def frame(flags: int) -> bytes:
        return b"APETAGEX" + struct.pack("<IIII", 2000, size, len(items), flags) + bytes(8)

    return frame(0xA000_0000) + body + frame(0x8000_0000)


# ---------------------------------------------------------------------------
# The fixtures
# ---------------------------------------------------------------------------


def main() -> None:
    fixtures = {
        # RIFF INFO says French (in the old three-letter spelling), the
        # embedded ID3 tag says German. MeedyaManager shows "fre" (a full
        # tag is preferred, TRACK-070) and must say the ID3 tag disagrees.
        "lang_riff_fre_id3_ger.wav": wav_with(
            HERE / "riff_language.wav", None, id3v24_tag([("TLAN", "ger")])
        ),
        # The same disagreement in a FLAC that starts with an ID3 tag, as
        # some older tools write them. MeedyaManager can READ such a file,
        # but cannot yet SAVE one (issue opened by the third review round).
        "lang_vorbis_eng_id3_ger.flac": id3v24_tag([("TLAN", "ger")])
        + flac_with_vorbis(HERE / "silence.flac", [("LANGUAGE", "eng")]),
        # RIFF INFO holds a word ("English") that is not a language code,
        # and the embedded ID3 tag holds "eng". Used to prove a program that
        # reads every field and writes them all back unchanged, with only
        # the title altered, is not refused (COMPAT-030).
        "lang_riff_english_id3_eng.wav": wav_with(
            HERE / "silence.wav",
            [("ILNG", "English"), ("INAM", "Old")],
            id3v24_tag([("TLAN", "eng")]),
        ),
        # Both tags hold the same word, "English", which is not a language
        # code. The same text in both is agreement, even though MeedyaManager
        # cannot say which language it means — so no disagreement may be
        # reported (added for the fourth review round, item M6).
        "lang_riff_english_id3_english.wav": wav_with(
            HERE / "silence.wav",
            [("ILNG", "English"), ("INAM", "Old")],
            id3v24_tag([("TLAN", "English")]),
        ),
        # RIFF INFO says "en"; the ID3 tag holds "ger" TWICE (two values,
        # separated by a zero byte, as ID3 version 2.4 allows). The
        # disagreement must name "ger" once, not "ger" and "ger" (fourth
        # review round, item M6).
        "lang_riff_en_id3_ger_twice.wav": wav_with(
            HERE / "silence.wav",
            [("ILNG", "en"), ("INAM", "Old")],
            id3v24_tag([("TLAN", "ger\x00ger")]),
        ),
        # Codex's catch-up review, finding 1. RIFF INFO holds `ILNG` = `fre`
        # and a title written in Latin-1, not UTF-8: "Café" as the bytes
        # 43 61 66 E9. The `lofty` library cannot read that title, so it
        # leaves it out of what it read — and used to leave it out of what
        # it wrote back, too, when the language was changed. A language
        # save must now refuse and leave the file exactly as it was.
        "lang_riff_fre_title_latin1.wav": wav_with(
            HERE / "silence.wav",
            [("ILNG", "fre"), ("INAM", b"Caf\xe9")],
            None,
        ),
        # The other side of the same finding: every RIFF INFO entry is
        # UTF-8, so a language save loses nothing and must go ahead, with
        # every other entry byte for byte the same and in the same order.
        # `ILNG` sits in the middle on purpose (a save may move it; it may
        # not move anything else), and the entries have both odd and even
        # lengths, so padding is covered too.
        "lang_riff_fre_all_utf8.wav": wav_with(
            HERE / "silence.wav",
            [
                ("IART", "Les Élèves"),
                ("INAM", "Café"),
                ("ILNG", "fre"),
                ("ICMT", "Été"),
                ("ISFT", "Lavf62"),
            ],
            None,
        ),
        # Issue #259, and a consequence of finding 1's fix: a WAV with TWO
        # LIST INFO chunks — the first holding only the software name, the
        # second the language and a title. The tag library reads both but
        # rewrites only the first, copying the second's entries into it and
        # leaving the second as it was, so the file ended up with two
        # languages and two titles. A language save must now be refused,
        # leaving the file as it was. Built by appending a second chunk, which
        # `wav_with` deliberately never does.
        "lang_riff_two_info_lists.wav": _wav_from_chunks(
            [
                (cid, payload)
                for cid, payload in _wav_chunks((HERE / "silence.wav").read_bytes())
                if not (cid == b"LIST" and payload[:4] == b"INFO")
            ]
            + [
                (b"LIST", riff_info([("ISFT", "Lavf62")])),
                (b"LIST", riff_info([("ILNG", "fre"), ("INAM", "Old")])),
            ]
        ),
        # Codex's catch-up review, finding 4: an APE tag (at the end of an
        # MP3, as some players write them) whose `Language` item holds two
        # values in one string, English then French, separated by a zero
        # byte. Read the way the ID3 path reads them, the first is the
        # primary language and both are kept.
        "lang_ape_eng_fra.mp3": (HERE / "silence.mp3").read_bytes()
        + apev2_tag([("Title", b"Two languages"), ("Language", b"eng\x00fra")]),
        # Codex's catch-up review, finding 5: a Vorbis `LANGUAGE` comment of
        # a no-break space (U+00A0) followed by "en". The policy trims only
        # four characters (space, tab, line feed, carriage return); anything
        # else, a no-break space included, is part of the value and makes
        # it malformed — so this must NOT be read as English.
        "lang_vorbis_nbsp_en.flac": flac_with_vorbis(
            HERE / "silence.flac", [("LANGUAGE", " en")]
        ),
    }
    for name, data in fixtures.items():
        (HERE / name).write_bytes(data)
        print(f"wrote {name} ({len(data)} bytes)")


if __name__ == "__main__":
    main()
