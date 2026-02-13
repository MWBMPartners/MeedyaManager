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
