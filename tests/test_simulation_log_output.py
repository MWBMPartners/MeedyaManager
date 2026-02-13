# ============================================================================
# File: /tests/test_simulation_log_output.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Validates that simulated rename paths are logged during watcher processing.
# Uses caplog for reliable log capture instead of reading from log files.
# ============================================================================

import logging
import pytest
from unittest.mock import patch

from core import watcher


def test_simulated_path_logged(caplog, tmp_path):
    """Verify that simulate_rename output is recorded in watcher log."""
    # Override config for test isolation
    watcher.watch_paths = [str(tmp_path)]
    watcher.valid_extensions = [".mp3"]
    watcher.simulate_enabled = True

    # Create a dummy media file
    test_file = tmp_path / "simlogtest.mp3"
    test_file.write_text("AUDIO")

    # Patch simulate_rename where it's imported in the watcher module
    with patch("core.watcher.simulate_rename", return_value="/simulated/path/output.mp3"):
        with caplog.at_level(logging.INFO, logger="watcher"):
            watcher.handle_file(str(test_file))

    # Verify the simulated path was logged
    assert "Simulated path" in caplog.text, "Simulated path not logged"
    assert "/simulated/path/output.mp3" in caplog.text, "Simulated path output missing"
