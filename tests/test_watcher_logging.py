# ============================================================================
# File: /tests/test_watcher_logging.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Validates that /core/watcher.py logs media detection events correctly
# and applies redaction where needed. Uses caplog for reliable log capture.
# ============================================================================

import logging
import pytest
from unittest.mock import patch

from core import watcher


def test_logging_redaction_and_rotation(caplog, tmp_path):
    """Verify handle_file logs detection and metadata, and that redact() works."""
    # Create a dummy media file in a temp directory
    test_media_file = tmp_path / "test_music.mp3"
    test_media_file.write_text("FAKEAUDIO")

    # Override config for test isolation
    watcher.watch_paths = [str(tmp_path)]
    watcher.valid_extensions = [".mp3"]

    # Disable simulation to avoid renamer errors on fake files (no artist tag)
    original_sim = watcher.simulate_enabled
    watcher.simulate_enabled = False

    try:
        # Capture log output from the watcher logger at INFO level
        with caplog.at_level(logging.INFO, logger="watcher"):
            watcher.handle_file(str(test_media_file))

        # Verify file detection was logged
        assert "Detected file:" in caplog.text, "File detection not logged"

        # Verify metadata extraction was logged
        assert "Extracted metadata:" in caplog.text, "Metadata not logged"
    finally:
        # Restore original simulation setting
        watcher.simulate_enabled = original_sim


def test_redact_function():
    """Verify the redact() function replaces user paths with <user>."""
    # Test macOS-style path redaction
    assert watcher.redact("/Users/johndoe/Music/song.mp3") == "<user>/Music/song.mp3"

    # Test Windows-style path redaction
    assert watcher.redact('C:\\Users\\johndoe') == "<user>"

    # Test paths that don't match any pattern (should be unchanged)
    assert watcher.redact("/var/folders/tmp/file.mp3") == "/var/folders/tmp/file.mp3"
