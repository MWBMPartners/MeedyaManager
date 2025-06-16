# ============================================================================
# File: /tests/test_watcher_simulation_trigger.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
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
    watcher.watch_folders = [temp_watch_folder]
    watcher.valid_extensions = [".mp3"]

    test_file = os.path.join(temp_watch_folder, "trigger.mp3")
    with open(test_file, "w") as f:
        f.write("FAKEAUDIO")

    with patch("cli.runner.simulate_rename") as mock_sim:
        mock_sim.return_value = "/simulated/path/trigger.mp3"
        watcher.handle_file(test_file)
        time.sleep(1)
        mock_sim.assert_called_once()
        args, kwargs = mock_sim.call_args
        assert args[0] == test_file, "Original file path not passed to simulate_rename"
        assert kwargs["dry_run"] is True, "Dry-run flag not set to True"

    print("✅ Rename simulation triggered by watcher.")