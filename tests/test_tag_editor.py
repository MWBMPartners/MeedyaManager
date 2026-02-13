# ============================================================================
# File: /tests/test_tag_editor.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the TagEditor class (metadata/editor.py).
# Creates real minimal media files using mutagen to test tag reading,
# writing, cover art operations, and format detection.
# ============================================================================

import os                                              # File path operations
import pytest                                          # Test framework

from metadata.editor import (
    TagEditor,
    CoverArt,
    UnsupportedFormatError,
    TagWriteError,
)


# =============================================================================
# Fixtures — Create minimal real media files for testing
# =============================================================================

def _create_minimal_mp3(path):
    """
    Create a minimal valid MP3 file with ID3v2 tags.

    Uses mutagen's ID3 class to save tags directly. The MPEG frame data
    consists of multiple valid MPEG1 Layer III frames (128kbps, 44100Hz,
    joint stereo) with zeroed audio data (silence).
    """
    from mutagen.id3 import (
        ID3, TIT2, TPE1, TALB, TPE2, TCON, TRCK, TPOS, TDRC,
    )

    # Write valid MPEG1 Layer III frames to the file first.
    # Frame header bytes: FF FB 90 64
    #   FF FB = sync (0xFFE) + MPEG1 (11) + Layer III (01) + no CRC (1)
    #   90    = bitrate 128kbps (1001) + samplerate 44100 (00) + no padding (0) + private (0)
    #   64    = joint stereo (01) + mode_ext (10) + not copyright (0) + original (1) + no emphasis (00)
    # Frame size for 128kbps 44100Hz = floor(144 * 128000 / 44100) = 417 bytes
    frame_header = b'\xff\xfb\x90\x64'
    frame_size = 417
    single_frame = frame_header + b'\x00' * (frame_size - 4)

    # Write 3 frames for mutagen to reliably detect MPEG sync
    with open(path, "wb") as f:
        f.write(single_frame * 3)

    # Save ID3v2 tags using mutagen's ID3 class (works on any file)
    tags = ID3()
    tags.add(TIT2(encoding=3, text=["Test Title"]))
    tags.add(TPE1(encoding=3, text=["Test Artist"]))
    tags.add(TALB(encoding=3, text=["Test Album"]))
    tags.add(TPE2(encoding=3, text=["Test Album Artist"]))
    tags.add(TCON(encoding=3, text=["Rock"]))
    tags.add(TRCK(encoding=3, text=["3/12"]))
    tags.add(TPOS(encoding=3, text=["1/2"]))
    tags.add(TDRC(encoding=3, text=["2025"]))
    tags.save(path)
    return path


def _create_minimal_flac(path):
    """
    Create a minimal valid FLAC file with Vorbis Comments.

    Writes a binary FLAC file with a valid STREAMINFO metadata block,
    then adds Vorbis Comment tags via mutagen's FLAC class.
    """
    from mutagen.flac import FLAC

    # FLAC file structure: "fLaC" marker + metadata blocks
    # STREAMINFO block is exactly 34 bytes and is required as the first block.
    #
    # STREAMINFO layout (34 bytes):
    #   Bytes 0-1:   Minimum block size (samples) = 4096
    #   Bytes 2-3:   Maximum block size (samples) = 4096
    #   Bytes 4-6:   Minimum frame size (bytes) = 0
    #   Bytes 7-9:   Maximum frame size (bytes) = 0
    #   Bytes 10-13: Sample rate (20 bits) | channels-1 (3 bits) | bps-1 (5 bits) | total_samples_hi (4 bits)
    #   Bytes 14-17: Total samples low 32 bits
    #   Bytes 18-33: MD5 signature (16 bytes, zeroed)
    #
    # For 44100Hz, 1 channel (mono), 16 bits per sample, 0 total samples:
    #   Sample rate = 44100 = 0x0AC44 (20 bits)
    #   channels-1 = 0 (3 bits)
    #   bps-1 = 15 (5 bits)
    #   total_samples_hi = 0 (4 bits)
    #   Bytes 10-13 = 0x0AC440F0
    streaminfo = bytearray(34)
    streaminfo[0:2] = b'\x10\x00'                      # Min block size = 4096
    streaminfo[2:4] = b'\x10\x00'                      # Max block size = 4096
    # Bytes 4-9: min/max frame size = 0 (already zero)
    # Bytes 10-13: sample_rate=44100, channels=1(mono), bps=16
    streaminfo[10] = 0x0A                              # Sample rate bits 19-12
    streaminfo[11] = 0xC4                              # Sample rate bits 11-4
    streaminfo[12] = 0x40                              # SR bits 3-0 (0100) + ch-1 (000) + bps-1 bit4 (0)
    streaminfo[13] = 0xF0                              # bps-1 bits 3-0 (1111) + total_samples_hi (0000)
    # Bytes 14-17: total_samples_lo = 0 (already zero)
    # Bytes 18-33: MD5 = all zeros (already zero)

    # Metadata block header (4 bytes): last=1, type=0 (STREAMINFO), length=34
    block_header = b'\x80\x00\x00\x22'

    with open(path, "wb") as f:
        f.write(b"fLaC")                               # FLAC stream marker
        f.write(block_header)                          # STREAMINFO block header
        f.write(bytes(streaminfo))                     # STREAMINFO data (34 bytes)

    # Open with mutagen and add Vorbis Comment tags
    audio = FLAC(path)
    audio["TITLE"] = ["Test Title"]
    audio["ARTIST"] = ["Test Artist"]
    audio["ALBUM"] = ["Test Album"]
    audio["ALBUMARTIST"] = ["Test Album Artist"]
    audio["GENRE"] = ["Rock"]
    audio["TRACKNUMBER"] = ["3"]
    audio["TOTALTRACKS"] = ["12"]
    audio["DISCNUMBER"] = ["1"]
    audio["TOTALDISCS"] = ["2"]
    audio["DATE"] = ["2025"]
    audio.save()
    return path


@pytest.fixture
def editor():
    """Provide a TagEditor instance."""
    return TagEditor()


@pytest.fixture
def mp3_file(tmp_path):
    """Create a temporary MP3 file with standard tags."""
    return _create_minimal_mp3(str(tmp_path / "test.mp3"))


@pytest.fixture
def flac_file(tmp_path):
    """Create a temporary FLAC file with standard tags."""
    return _create_minimal_flac(str(tmp_path / "test.flac"))


@pytest.fixture
def unsupported_file(tmp_path):
    """Create a file with an unsupported format (plain text)."""
    path = str(tmp_path / "test.txt")
    with open(path, "w") as f:
        f.write("This is not a media file")
    return path


@pytest.fixture
def tiny_jpeg():
    """Return a minimal valid JPEG image (1x1 pixel, red)."""
    # Minimal JPEG: SOI + APP0 + DQT + SOF0 + DHT + SOS + data + EOI
    # This is the smallest valid JPEG that most parsers accept
    return (
        b'\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00'
        b'\xff\xdb\x00C\x00\x08\x06\x06\x07\x06\x05\x08\x07\x07\x07\t\t'
        b'\x08\n\x0c\x14\r\x0c\x0b\x0b\x0c\x19\x12\x13\x0f\x14\x1d\x1a'
        b'\x1f\x1e\x1d\x1a\x1c\x1c $.\' ",#\x1c\x1c(7),01444\x1f\'9=82<.342'
        b'\xff\xc0\x00\x0b\x08\x00\x01\x00\x01\x01\x01\x11\x00'
        b'\xff\xc4\x00\x1f\x00\x00\x01\x05\x01\x01\x01\x01\x01\x01\x00'
        b'\x00\x00\x00\x00\x00\x00\x00\x01\x02\x03\x04\x05\x06\x07\x08\t\n\x0b'
        b'\xff\xc4\x00\xb5\x10\x00\x02\x01\x03\x03\x02\x04\x03\x05\x05\x04'
        b'\x04\x00\x00\x01}\x01\x02\x03\x00\x04\x11\x05\x12!1A\x06\x13Qa\x07'
        b'"q\x142\x81\x91\xa1\x08#B\xb1\xc1\x15R\xd1\xf0$3br\x82\t\n\x16\x17'
        b'\x18\x19\x1a%&\'()*456789:CDEFGHIJSTUVWXYZcdefghijstuvwxyz'
        b'\x83\x84\x85\x86\x87\x88\x89\x8a\x92\x93\x94\x95\x96\x97\x98\x99'
        b'\x9a\xa2\xa3\xa4\xa5\xa6\xa7\xa8\xa9\xaa\xb2\xb3\xb4\xb5\xb6\xb7'
        b'\xb8\xb9\xba\xc2\xc3\xc4\xc5\xc6\xc7\xc8\xc9\xca\xd2\xd3\xd4\xd5'
        b'\xd6\xd7\xd8\xd9\xda\xe1\xe2\xe3\xe4\xe5\xe6\xe7\xe8\xe9\xea\xf1'
        b'\xf2\xf3\xf4\xf5\xf6\xf7\xf8\xf9\xfa'
        b'\xff\xda\x00\x08\x01\x01\x00\x00?\x00T\xdb\xa1\x8e(\xa0\x02\x80'
        b'\xff\xd9'
    )


# =============================================================================
# Format Detection Tests
# =============================================================================

class TestFormatDetection:
    """Tests for get_supported_format() and format auto-detection."""

    def test_mp3_format_detected(self, editor, mp3_file):
        """MP3 files should be detected as supported."""
        assert editor.get_supported_format(mp3_file) == "MP3"

    def test_flac_format_detected(self, editor, flac_file):
        """FLAC files should be detected as supported."""
        assert editor.get_supported_format(flac_file) == "FLAC"

    def test_unsupported_format_returns_none(self, editor, unsupported_file):
        """Non-media files should return None."""
        assert editor.get_supported_format(unsupported_file) is None

    def test_nonexistent_file_returns_none(self, editor):
        """Non-existent files should return None."""
        assert editor.get_supported_format("/nonexistent/file.mp3") is None


# =============================================================================
# ID3 (MP3) Read Tests
# =============================================================================

class TestID3Read:
    """Tests for reading ID3v2 tags from MP3 files."""

    def test_read_standard_tags(self, editor, mp3_file):
        """Standard ID3v2 text frames should be read correctly."""
        tags = editor.read_tags(mp3_file)
        assert tags["title"] == "Test Title"
        assert tags["artist"] == "Test Artist"
        assert tags["album"] == "Test Album"
        assert tags["album_artist"] == "Test Album Artist"
        assert tags["genre"] == "Rock"
        assert tags["year"] == "2025"

    def test_read_track_number_splitting(self, editor, mp3_file):
        """Track number "3/12" should be split into track_num and total_tracks."""
        tags = editor.read_tags(mp3_file)
        assert tags["track_num"] == "3"
        assert tags["total_tracks"] == "12"

    def test_read_disc_number_splitting(self, editor, mp3_file):
        """Disc number "1/2" should be split into disc_num and total_discs."""
        tags = editor.read_tags(mp3_file)
        assert tags["disc_num"] == "1"
        assert tags["total_discs"] == "2"

    def test_read_empty_file_returns_dict(self, editor, tmp_path):
        """MP3 file without tags should return an empty dict."""
        path = str(tmp_path / "no_tags.mp3")
        # Create MP3 with no tags
        frame_header = b'\xff\xfb\x90\x04'
        with open(path, "wb") as f:
            f.write(frame_header + b'\x00' * 413)
        tags = editor.read_tags(path)
        assert isinstance(tags, dict)


# =============================================================================
# ID3 (MP3) Write Tests
# =============================================================================

class TestID3Write:
    """Tests for writing ID3v2 tags to MP3 files."""

    def test_write_standard_tag(self, editor, mp3_file):
        """Writing a standard tag should update the file."""
        changes = editor.write_tags(mp3_file, {"artist": "New Artist"})
        assert "artist" in changes
        assert changes["artist"] == ("Test Artist", "New Artist")

        # Verify the written value
        tags = editor.read_tags(mp3_file)
        assert tags["artist"] == "New Artist"

    def test_write_multiple_tags(self, editor, mp3_file):
        """Writing multiple tags at once should update all of them."""
        editor.write_tags(mp3_file, {
            "title": "New Title",
            "genre": "Jazz",
            "year": "2026",
        })
        tags = editor.read_tags(mp3_file)
        assert tags["title"] == "New Title"
        assert tags["genre"] == "Jazz"
        assert tags["year"] == "2026"

    def test_write_track_with_total(self, editor, mp3_file):
        """Writing track_num and total_tracks should produce "num/total" TRCK."""
        editor.write_tags(mp3_file, {
            "track_num": "5",
            "total_tracks": "15",
        })
        tags = editor.read_tags(mp3_file)
        assert tags["track_num"] == "5"
        assert tags["total_tracks"] == "15"

    def test_write_custom_tag(self, editor, mp3_file):
        """Custom tags should be written as TXXX frames."""
        editor.write_tags(mp3_file, {"custom_my_tag": "Custom Value"})
        tags = editor.read_tags(mp3_file)
        assert tags.get("custom_my_tag") == "Custom Value"

    def test_dry_run_no_changes(self, editor, mp3_file):
        """Dry run should return changes without modifying the file."""
        changes = editor.write_tags(mp3_file, {"artist": "New"}, dry_run=True)
        assert "artist" in changes

        # Verify original value unchanged
        tags = editor.read_tags(mp3_file)
        assert tags["artist"] == "Test Artist"

    def test_write_no_changes_skips(self, editor, mp3_file):
        """Writing the same value should return no changes."""
        changes = editor.write_tags(mp3_file, {"artist": "Test Artist"})
        assert len(changes) == 0

    def test_remove_tag_with_none(self, editor, mp3_file):
        """Setting a tag to None should remove it."""
        editor.write_tags(mp3_file, {"genre": None})
        tags = editor.read_tags(mp3_file)
        assert "genre" not in tags or tags.get("genre") == ""

    def test_remove_tag_with_empty_string(self, editor, mp3_file):
        """Setting a tag to empty string should remove it."""
        editor.write_tags(mp3_file, {"genre": ""})
        tags = editor.read_tags(mp3_file)
        assert "genre" not in tags or tags.get("genre") == ""


# =============================================================================
# Vorbis Comments (FLAC) Read/Write Tests
# =============================================================================

class TestVorbisReadWrite:
    """Tests for reading/writing Vorbis Comments on FLAC files."""

    def test_read_standard_tags(self, editor, flac_file):
        """Standard Vorbis Comments should be read correctly."""
        tags = editor.read_tags(flac_file)
        assert tags["title"] == "Test Title"
        assert tags["artist"] == "Test Artist"
        assert tags["album"] == "Test Album"
        assert tags["album_artist"] == "Test Album Artist"
        assert tags["genre"] == "Rock"
        assert tags["year"] == "2025"

    def test_read_track_and_total(self, editor, flac_file):
        """TRACKNUMBER and TOTALTRACKS should be read separately."""
        tags = editor.read_tags(flac_file)
        assert tags["track_num"] == "3"
        assert tags["total_tracks"] == "12"

    def test_write_tag(self, editor, flac_file):
        """Writing a Vorbis Comment should update the file."""
        editor.write_tags(flac_file, {"artist": "New FLAC Artist"})
        tags = editor.read_tags(flac_file)
        assert tags["artist"] == "New FLAC Artist"

    def test_write_multi_value(self, editor, flac_file):
        """Multi-value fields should be written as multiple Vorbis entries."""
        editor.write_tags(flac_file, {"genre": "Rock; Jazz; Blues"})
        tags = editor.read_tags(flac_file)
        # Should come back as semicolon-delimited (our normalised format)
        assert "Rock" in tags["genre"]
        assert "Jazz" in tags["genre"]

    def test_write_custom_tag(self, editor, flac_file):
        """Custom tags should be written directly as Vorbis Comments."""
        editor.write_tags(flac_file, {"custom_my_custom": "FLAC Custom"})
        tags = editor.read_tags(flac_file)
        assert tags.get("custom_my_custom") == "FLAC Custom"


# =============================================================================
# Cover Art Tests
# =============================================================================

class TestCoverArt:
    """Tests for cover art reading, writing, and removal."""

    def test_write_and_read_cover_mp3(self, editor, mp3_file, tiny_jpeg):
        """Cover art should be embeddable in and readable from MP3 files."""
        editor.write_cover_art(mp3_file, tiny_jpeg, "jpeg", picture_type=3)
        covers = editor.read_cover_art(mp3_file)
        assert len(covers) >= 1
        assert isinstance(covers[0], CoverArt)
        assert covers[0].format == "jpeg"
        assert covers[0].picture_type == 3

    def test_write_and_read_cover_flac(self, editor, flac_file, tiny_jpeg):
        """Cover art should be embeddable in and readable from FLAC files."""
        editor.write_cover_art(flac_file, tiny_jpeg, "jpeg", picture_type=3)
        covers = editor.read_cover_art(flac_file)
        assert len(covers) >= 1
        assert isinstance(covers[0], CoverArt)

    def test_remove_cover_mp3(self, editor, mp3_file, tiny_jpeg):
        """Removing cover art from MP3 should return count removed."""
        editor.write_cover_art(mp3_file, tiny_jpeg)
        removed = editor.remove_cover_art(mp3_file)
        assert removed >= 1
        # Verify no covers remain
        covers = editor.read_cover_art(mp3_file)
        assert len(covers) == 0

    def test_remove_cover_no_art(self, editor, mp3_file):
        """Removing cover art from a file with none should return 0."""
        removed = editor.remove_cover_art(mp3_file)
        assert removed == 0

    def test_read_cover_no_art(self, editor, mp3_file):
        """Reading cover art from file with none should return empty list."""
        covers = editor.read_cover_art(mp3_file)
        assert covers == []

    def test_read_cover_unsupported_format(self, editor, unsupported_file):
        """Reading cover art from unsupported format returns empty list."""
        covers = editor.read_cover_art(unsupported_file)
        assert covers == []


# =============================================================================
# Error Handling Tests
# =============================================================================

class TestErrorHandling:
    """Tests for error handling and edge cases."""

    def test_read_nonexistent_file(self, editor):
        """Reading tags from a nonexistent file should return empty dict."""
        tags = editor.read_tags("/nonexistent/file.mp3")
        assert tags == {}

    def test_write_nonexistent_file_raises(self, editor):
        """Writing to a nonexistent file should raise TagWriteError."""
        with pytest.raises(TagWriteError):
            editor.write_tags("/nonexistent/file.mp3", {"artist": "Test"})

    def test_write_unsupported_format_raises(self, editor, unsupported_file):
        """Writing to an unsupported format should raise UnsupportedFormatError."""
        with pytest.raises(UnsupportedFormatError):
            editor.write_tags(unsupported_file, {"artist": "Test"})

    def test_read_unsupported_format_returns_empty(self, editor, unsupported_file):
        """Reading from unsupported format should return empty dict."""
        tags = editor.read_tags(unsupported_file)
        assert tags == {}

    def test_write_cover_nonexistent_file_raises(self, editor, tiny_jpeg):
        """Writing cover art to nonexistent file should raise TagWriteError."""
        with pytest.raises(TagWriteError):
            editor.write_cover_art("/nonexistent/file.mp3", tiny_jpeg)

    def test_write_cover_unsupported_raises(self, editor, unsupported_file, tiny_jpeg):
        """Writing cover art to unsupported format should raise error."""
        with pytest.raises(UnsupportedFormatError):
            editor.write_cover_art(unsupported_file, tiny_jpeg)
