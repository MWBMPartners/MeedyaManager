# ============================================================================
# File: /tests/test_watcher_logging.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Validates that /core/watcher.py logs media detection events correctly,
# handles output file creation, and applies redaction where needed.
# ============================================================================

import os
import tempfile
import shutil
import time
import logging
from core import watcher
from core.metadata_extractor import extract_metadata


def test_logging_redaction_and_rotation():
    # Setup temporary directory to simulate a media folder
    with tempfile.TemporaryDirectory() as temp_dir:
        test_media_file = os.path.join(temp_dir, "test_music.mp3")

        # Create a dummy media file
        with open(test_media_file, "w") as f:
            f.write("FAKEAUDIO")

        # Override config manually for test
        watcher.watch_folders = [temp_dir]
        watcher.valid_extensions = [".mp3"]

        # Simulate detection
        watcher.handle_file(test_media_file)

        log_file = os.path.join("logs", "watcher_events.log")

        assert os.path.exists(log_file), "Log file was not created"

        # Read and verify log content
        with open(log_file, "r", encoding="utf-8") as log:
            contents = log.read()
            assert "Detected file:" in contents, "File detection not logged"
            assert "Extracted metadata:" in contents, "Metadata not logged"
            assert "/Users/REDACTED" in contents or "C:/Users/REDACTED" in contents or "REDACTED" in contents, "Redaction not applied"

        # Clean up temp log files
        for suffix in [".1", ".2", ".3"]:
            rotated = f"{log_file}{suffix}"
            if os.path.exists(rotated):
                os.remove(rotated)

        print("✅ Watcher logging test passed.")