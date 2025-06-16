# ============================================================================
# File: /tests/test_batch_rename_simulation.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Integration-style test that simulates batch file handling and rename simulation.
# Ensures runner + watcher handle multiple files correctly and consistently.
# ============================================================================

import os
import tempfile
import shutil
import pytest
from unittest.mock import patch
from core import watcher


@pytest.fixture(scope="function")
def batch_folder():
    path = tempfile.mkdtemp()
    yield path
    shutil.rmtree(path)


def test_batch_rename_simulation(batch_folder):
    watcher.watch_folders = [batch_folder]
    watcher.valid_extensions = [".mp3", ".flac", ".mkv"]
    watcher.simulate_enabled = True

    # Create batch of mixed files
    files = ["01_test_track.mp3", "album_intro.flac", "movie_scene.mkv"]
    for filename in files:
        with open(os.path.join(batch_folder, filename), "w") as f:
            f.write("FAKE")

    with patch("cli.runner.simulate_rename") as mock_sim:
        mock_sim.side_effect = lambda path, metadata, dry_run=True: f"/simulated/output/{os.path.basename(path)}"

        for filename in files:
            full_path = os.path.join(batch_folder, filename)
            watcher.handle_file(full_path)

    assert mock_sim.call_count == len(files), f"Expected {len(files)} simulated calls, got {mock_sim.call_count}"

    print("✅ Batch rename simulation called correctly for each file.")