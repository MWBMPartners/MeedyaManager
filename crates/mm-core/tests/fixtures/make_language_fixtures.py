#!/usr/bin/env python3
# (C) 2025-2026 MWBM Partners Ltd
#
# MeedyaManager — crates/mm-core/tests/fixtures/make_language_fixtures.py
#
# Builds the three "language in more than one tag at once" test files from
# the committed `silence.wav`, `silence.flac` and `riff_language.wav`, using
# nothing but Python's standard library. Run it from anywhere:
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
# It only knows the few pieces of the WAV, FLAC and ID3 formats these files
# need (one text frame per ID3 tag, UTF-8 text, the Vorbis comment block).
# It is not a general tag writer and should not be grown into one.
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


def riff_info(pairs: list[tuple[str, str]]) -> bytes:
    """The payload of a `LIST` chunk of type `INFO`: one sub-chunk per
    `(four-letter id, text)`, each text ending in a zero byte as RIFF INFO
    expects."""
    return b"INFO" + b"".join(
        _chunk(cid.encode("ascii"), text.encode("utf-8") + b"\x00") for cid, text in pairs
    )


def wav_with(
    source: Path, info: list[tuple[str, str]] | None, id3: bytes | None
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
    }
    for name, data in fixtures.items():
        (HERE / name).write_bytes(data)
        print(f"wrote {name} ({len(data)} bytes)")


if __name__ == "__main__":
    main()
