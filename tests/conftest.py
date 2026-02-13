# ============================================================================
# File: /tests/conftest.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Shared pytest fixtures for MeedyaManager test suite.
# Provides CLI runner, temp directories, sample metadata, and config mocking.
# ============================================================================

import os                                          # Path operations
import tempfile                                    # Temporary directories
import shutil                                      # File operations
import pytest                                      # Test framework
from click.testing import CliRunner                # CLI testing runner
from utils.log_config import setup_logging, reset_logging  # Centralized logging


@pytest.fixture(autouse=True, scope="session")
def _initialize_logging():
    """
    Initialize centralized logging once for the entire test session.

    Uses DEBUG level so all log messages are captured by pytest's caplog.
    The autouse=True ensures this runs automatically before any test,
    and scope="session" ensures it only runs once per test session.
    """
    reset_logging()                                    # Clear any previous config
    setup_logging(log_level="DEBUG")
    yield
    reset_logging()                                    # Clean up after all tests


@pytest.fixture
def cli_runner():
    """Provide a Click CliRunner instance for invoking CLI commands."""
    return CliRunner()


@pytest.fixture
def temp_watch_folder(tmp_path):
    """Create a temporary directory with sample media files for testing."""
    # Create dummy media files (not real media, but enough for extension matching)
    test_files = ["test_track.mp3", "album_intro.flac", "movie_clip.mkv"]
    for filename in test_files:
        filepath = tmp_path / filename
        filepath.write_text("FAKE_MEDIA_DATA")
    return tmp_path


@pytest.fixture
def sample_metadata():
    """
    Return a known-good metadata dictionary for testing.
    Includes all standard keys used by the rule engine (M3).
    """
    return {
        "filepath": "/example/media/sample_track.mp3",
        "filename": "sample_track",
        "extension": "mp3",
        "format": "mp3",
        "duration": "245",
        "title": "Sample Track",
        "description": "",
        "audio_channels": "2",
        "is_lossless": "False",
        "media_group": "Audio",
        "format_class": "mp3",
        "media_class": "Music",
        "quality_type": "Lossy",
        "artist": "Test Artist",
        "album": "Test Album",
        "album_artist": "Test Artist",
        "year": "2025",
        "genre": "Rock",
        "track_num": "3",
        "disc_num": "1",
        "total_tracks": "12",
        "codec": "MP3",
        "bitrate": "320",
        "sample_rate": "44100",
    }


# =============================================================================
# Real Media File Fixtures — Created using mutagen for M4 integration tests
# =============================================================================

@pytest.fixture
def real_mp3_file(tmp_path):
    """
    Create a minimal valid MP3 file with standard ID3v2 tags.
    Uses MPEG1 Layer III frames (128kbps, 44100Hz, joint stereo) with
    ID3v2 tags saved via mutagen. Suitable for testing both pymediainfo
    and mutagen-based metadata extraction.
    """
    from mutagen.id3 import (
        ID3, TIT2, TPE1, TALB, TPE2, TCON, TRCK, TPOS, TDRC, TCOM,
    )

    path = str(tmp_path / "real_test.mp3")

    # Write valid MPEG1 Layer III frames (128kbps, 44100Hz, joint stereo)
    frame_header = b'\xff\xfb\x90\x64'
    frame_size = 417                                   # bytes per frame at 128kbps/44100Hz
    single_frame = frame_header + b'\x00' * (frame_size - 4)

    with open(path, "wb") as f:
        f.write(single_frame * 3)                      # 3 frames for reliable sync detection

    # Save ID3v2 tags
    tags = ID3()
    tags.add(TIT2(encoding=3, text=["Integration Test Song"]))
    tags.add(TPE1(encoding=3, text=["Integration Artist"]))
    tags.add(TALB(encoding=3, text=["Integration Album"]))
    tags.add(TPE2(encoding=3, text=["Integration Album Artist"]))
    tags.add(TCON(encoding=3, text=["Electronic"]))
    tags.add(TRCK(encoding=3, text=["7/14"]))
    tags.add(TPOS(encoding=3, text=["2/3"]))
    tags.add(TDRC(encoding=3, text=["2026"]))
    tags.add(TCOM(encoding=3, text=["Integration Composer"]))
    tags.save(path)
    return path


@pytest.fixture
def real_flac_file(tmp_path):
    """
    Create a minimal valid FLAC file with Vorbis Comment tags.
    Writes a binary FLAC file with valid STREAMINFO metadata block,
    then adds tags via mutagen's FLAC class.
    """
    from mutagen.flac import FLAC

    path = str(tmp_path / "real_test.flac")

    # Write FLAC with valid STREAMINFO block (44100Hz, mono, 16-bit)
    streaminfo = bytearray(34)
    streaminfo[0:2] = b'\x10\x00'                     # Min block size = 4096
    streaminfo[2:4] = b'\x10\x00'                     # Max block size = 4096
    streaminfo[10] = 0x0A                              # Sample rate 44100
    streaminfo[11] = 0xC4
    streaminfo[12] = 0x40                              # 1 channel, 16-bit
    streaminfo[13] = 0xF0

    block_header = b'\x80\x00\x00\x22'                 # Last block, STREAMINFO, 34 bytes

    with open(path, "wb") as f:
        f.write(b"fLaC")
        f.write(block_header)
        f.write(bytes(streaminfo))

    # Add Vorbis Comment tags
    audio = FLAC(path)
    audio["TITLE"] = ["FLAC Integration Song"]
    audio["ARTIST"] = ["FLAC Integration Artist"]
    audio["ALBUM"] = ["FLAC Integration Album"]
    audio["ALBUMARTIST"] = ["FLAC Album Artist"]
    audio["GENRE"] = ["Classical"]
    audio["TRACKNUMBER"] = ["5"]
    audio["TOTALTRACKS"] = ["20"]
    audio["DISCNUMBER"] = ["1"]
    audio["TOTALDISCS"] = ["1"]
    audio["DATE"] = ["2025"]
    audio["COMPOSER"] = ["FLAC Composer"]
    audio.save()
    return path
