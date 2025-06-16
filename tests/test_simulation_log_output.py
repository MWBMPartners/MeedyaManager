# ============================================================================
# File: /tests/test_simulation_log_output.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Validates that simulated rename paths are logged during watcher processing.
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


@pytest.fixture(scope="function")
def temp_log_folder():
    log_path = os.path.join(os.getcwd(), "logs")
    os.makedirs(log_path, exist_ok=True)
    test_log = os.path.join(log_path, "watcher_events.log")
    if os.path.exists(test_log):
        os.remove(test_log)
    yield test_log
    if os.path.exists(test_log):
        os.remove(test_log)


def test_simulated_path_logged(temp_watch_folder, temp_log_folder):
    watcher.watch_folders = [temp_watch_folder]
    watcher.valid_extensions = [".mp3"]
    watcher.simulate_enabled = True

    test_file = os.path.join(temp_watch_folder, "simlogtest.mp3")
    with open(test_file, "w") as f:
        f.write("AUDIO")

    with patch("cli.runner.simulate_rename", return_value="/simulated/path/output.mp3"):
        watcher.handle_file(test_file)
        time.sleep(1)

    assert os.path.exists(temp_log_folder), "Simulation log not created"

    with open(temp_log_folder, "r") as log:
        contents = log.read()
        assert "Simulated path" in contents, "Simulated path not logged"
        assert "/simulated/path/output.mp3" in contents, "Simulated path output missing"

    print("✅ Simulated rename path is recorded in watcher log.")