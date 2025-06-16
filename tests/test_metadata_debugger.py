# ============================================================================
# File: /tests/test_metadata_debugger.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Unit tests for /cli/metadata_debugger.py functionality.
# Tests CLI argument parsing, file validation, and JSON output handling.
# ============================================================================

import os
import subprocess
import tempfile
import shutil
import pytest

TEST_MEDIA_FILE = "tests/test_files/sample.flac"

@pytest.fixture(scope="module")
def prepare_test_file():
    """
    Creates a temporary copy of the test media file to use during tests
    """
    temp_dir = tempfile.mkdtemp()
    temp_path = os.path.join(temp_dir, "sample.flac")
    shutil.copy(TEST_MEDIA_FILE, temp_path)
    yield temp_path
    shutil.rmtree(temp_dir)

def test_json_export(prepare_test_file):
    """
    Validates that --json creates a .metadata.json file alongside media
    """
    media_path = prepare_test_file
    result = subprocess.run([
        "python", "cli/metadata_debugger.py", media_path, "--json"
    ], capture_output=True, text=True)

    assert result.returncode == 0
    expected_json = media_path.replace(".flac", ".metadata.json")
    assert os.path.isfile(expected_json)

    # Clean up
    os.remove(expected_json)

def test_json_export_with_out_folder(prepare_test_file):
    """
    Validates that --json with --out saves JSON to the specified folder
    """
    media_path = prepare_test_file
    out_dir = tempfile.mkdtemp()
    result = subprocess.run([
        "python", "cli/metadata_debugger.py", media_path,
        "--json", "--out", out_dir
    ], capture_output=True, text=True)

    base = os.path.splitext(os.path.basename(media_path))[0]
    expected_json = os.path.join(out_dir, base + ".metadata.json")

    assert result.returncode == 0
    assert os.path.isfile(expected_json)

    # Clean up
    shutil.rmtree(out_dir)

def test_file_not_found():
    """
    Validates that invalid file path triggers proper error message
    """
    result = subprocess.run([
        "python", "cli/metadata_debugger.py", "not_a_real_file.mp4"
    ], capture_output=True, text=True)

    assert result.returncode != 0
    assert "File does not exist" in result.stdout