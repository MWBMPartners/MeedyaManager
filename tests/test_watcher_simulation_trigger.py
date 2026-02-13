# ============================================================================
# File: /tests/test_watcher_simulation_trigger.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests that /core/watcher.py invokes simulate_rename() when simulate_watcher is enabled.
# Ensures rename simulation logic is triggered correctly from watcher flow.
# ============================================================================

import os
import tempfile
import shutil
import time
import pytest
from unittest.mock import patch

from core import watcher


@pytest.fixture(scope="function")
def temp_watch_folder():
    path = tempfile.mkdtemp()
    yield path
    shutil.rmtree(path)


def test_simulate_rename_triggered(temp_watch_folder):
    # Set up watcher config
    watcher.watch_paths = [temp_watch_folder]
    watcher.valid_extensions = [".mp3"]
    watcher.simulate_enabled = True

    test_file = os.path.join(temp_watch_folder, "trigger.mp3")
    with open(test_file, "w") as f:
        f.write("FAKEAUDIO")

    # Patch the simulate_rename function where it was imported into the watcher module
    with patch("core.watcher.simulate_rename") as mock_sim:
        mock_sim.return_value = "/simulated/path/trigger.mp3"
        watcher.handle_file(test_file)
        time.sleep(1)
        mock_sim.assert_called_once()
        # Verify the original filepath was passed as the first positional arg
        args, kwargs = mock_sim.call_args
        assert args[0] == test_file, "Original file path not passed to simulate_rename"

    print("✅ Rename simulation triggered by watcher.")
