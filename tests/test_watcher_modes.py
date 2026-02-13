# ============================================================================
# File: /tests/test_watcher_modes.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Unit + integration tests for the dual-mode watcher behavior.
# Validates Watchdog and polling modes for detecting files and triggering logic.
# ============================================================================

import os
import tempfile
import time
import shutil
import pytest
from unittest.mock import patch

from core import watcher


@pytest.fixture(scope="function")
def temp_watch_folder():
    path = tempfile.mkdtemp()
    yield path
    shutil.rmtree(path)


def test_polling_detection(temp_watch_folder):
    # Override config
    watcher.watch_paths = [temp_watch_folder]
    watcher.valid_extensions = [".mp3"]
    watcher.watch_mode = "polling"

    # Create test media file
    media_path = os.path.join(temp_watch_folder, "song.mp3")
    with open(media_path, "w") as f:
        f.write("TEST")

    # Run handler manually to simulate detection — handle_file enqueues the file
    watcher.handle_file(media_path)
    time.sleep(1)

    assert not watcher.event_queue.empty(), "Polling failed to detect media file"


def test_watchdog_detection_manual(temp_watch_folder):
    if not watcher.WATCHDOG_AVAILABLE:
        pytest.skip("Watchdog not available on this system")

    # Simulate Watchdog handler directly (class is WatchHandler in watcher.py)
    handler = watcher.WatchHandler()
    file_path = os.path.join(temp_watch_folder, "clip.mp3")

    with open(file_path, "w") as f:
        f.write("FAKE")

    # Create a fake filesystem event and trigger the handler
    event = type("FakeEvent", (object,), {"src_path": file_path, "is_directory": False})()
    handler.on_created(event)
    time.sleep(1)

    assert not watcher.event_queue.empty(), "Watchdog handler failed to queue file"


def test_renamer_triggered_from_handle_file(temp_watch_folder):
    watcher.watch_paths = [temp_watch_folder]
    watcher.valid_extensions = [".mp3"]
    watcher.simulate_enabled = True

    file_path = os.path.join(temp_watch_folder, "autotrigger.mp3")
    with open(file_path, "w") as f:
        f.write("audio")

    # Patch simulate_rename where it's imported in the watcher module
    with patch("core.watcher.simulate_rename") as mock_sim:
        watcher.handle_file(file_path)
        time.sleep(1)
        mock_sim.assert_called_once()

    assert not watcher.event_queue.empty(), "Simulated rename did not enqueue properly"

    print("✅ Watcher mode tests passed.")
