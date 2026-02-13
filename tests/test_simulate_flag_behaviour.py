# ============================================================================
# File: /tests/test_simulate_flag_behavior.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests simulate_watcher flag logic under various conditions.
# Ensures watcher disables rename simulation when overridden or disabled.
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


def test_simulation_disabled_via_flag(temp_watch_folder):
    # Backup + override config
    orig_simulate = watcher.simulate_enabled
    watcher.simulate_enabled = False

    watcher.watch_paths = [temp_watch_folder]
    watcher.valid_extensions = [".mp3"]

    test_file = os.path.join(temp_watch_folder, "disabletest.mp3")
    with open(test_file, "w") as f:
        f.write("TEST")

    with patch("core.watcher.simulate_rename") as mock_sim:
        watcher.handle_file(test_file)
        time.sleep(1)
        mock_sim.assert_not_called()

    watcher.simulate_enabled = orig_simulate
    print("✅ simulate_watcher=False disables rename simulation")


def test_simulation_enabled_by_default(temp_watch_folder):
    watcher.simulate_enabled = True
    watcher.watch_paths = [temp_watch_folder]
    watcher.valid_extensions = [".mp3"]

    test_file = os.path.join(temp_watch_folder, "enabletest.mp3")
    with open(test_file, "w") as f:
        f.write("TEST")

    with patch("core.watcher.simulate_rename") as mock_sim:
        watcher.handle_file(test_file)
        time.sleep(1)
        mock_sim.assert_called_once()

    print("✅ simulate_watcher=True allows rename simulation")


def test_simulation_toggle_at_runtime(temp_watch_folder):
    # Test that toggling simulate_enabled at runtime takes effect
    watcher.watch_paths = [temp_watch_folder]
    watcher.valid_extensions = [".mp3"]

    test_file = os.path.join(temp_watch_folder, "toggletest.mp3")
    with open(test_file, "w") as f:
        f.write("TEST")

    # Start with simulation enabled, then disable
    watcher.simulate_enabled = True

    with patch("core.watcher.simulate_rename") as mock_sim:
        # Disable simulation before calling handle_file
        watcher.simulate_enabled = False
        watcher.handle_file(test_file)
        time.sleep(1)
        mock_sim.assert_not_called()

    # Restore
    watcher.simulate_enabled = True
    print("✅ Runtime toggle of simulate_enabled works correctly")
