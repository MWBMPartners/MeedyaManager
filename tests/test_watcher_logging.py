# ============================================================================
# File: /tests/test_watcher_logging.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Validates that /core/watcher.py logs media detection events correctly.
# PII redaction is now handled by the centralized PIIRedactionFilter in
# utils/log_config.py and is tested in tests/test_log_config.py.
# ============================================================================

import logging
import pytest
from unittest.mock import patch

from core import watcher


def test_logging_detection_and_metadata(caplog, tmp_path):
    """Verify handle_file logs file detection and metadata extraction."""
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
        # Capture log output from the MeedyaManager.Watcher logger at INFO level
        with caplog.at_level(logging.INFO, logger="MeedyaManager.Watcher"):
            watcher.handle_file(str(test_media_file))

        # Verify file detection was logged
        assert "Detected file:" in caplog.text, "File detection not logged"

        # Verify metadata extraction was logged
        assert "Extracted metadata:" in caplog.text, "Metadata not logged"
    finally:
        # Restore original simulation setting
        watcher.simulate_enabled = original_sim


def test_watcher_uses_meedyamanager_logger():
    """Verify the watcher module uses the centralized MeedyaManager logger hierarchy."""
    assert watcher.logger.name == "MeedyaManager.Watcher"
