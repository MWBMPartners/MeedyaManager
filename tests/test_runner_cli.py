# ============================================================================
# File: /tests/test_runner_cli.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Unit tests for the CLI behavior of /cli/runner.py
# Includes support for dry-run simulation, --json flag, and --out folder
# validation and behavior.
# ============================================================================

import os
import subprocess
import tempfile
import shutil
import pytest

TEST_MEDIA_FILE = "tests/test_files/sample.mkv"

@pytest.fixture(scope="module")
def prepare_test_file():
    temp_dir = tempfile.mkdtemp()
    target_file = os.path.join(temp_dir, "sample.mkv")
    shutil.copy(TEST_MEDIA_FILE, target_file)
    yield target_file
    shutil.rmtree(temp_dir)

def test_runner_dry_run_only(prepare_test_file):
    result = subprocess.run([
        "python", "cli/runner.py"
    ], capture_output=True, text=True)

    assert result.returncode == 0
    assert "Dry-run rename simulation" in result.stdout

def test_runner_json_output(prepare_test_file):
    result = subprocess.run([
        "python", "cli/runner.py", "--json"
    ], capture_output=True, text=True)
    assert result.returncode == 0
    assert "Exported JSON metadata" in result.stdout or "Dry-run rename" in result.stdout


def test_runner_json_with_out_folder(prepare_test_file):
    output_dir = tempfile.mkdtemp()
    result = subprocess.run([
        "python", "cli/runner.py", "--json", "--out", output_dir
    ], capture_output=True, text=True)
    assert result.returncode == 0

    # Check if JSON files were exported
    exported = any(fname.endswith(".metadata.json") for fname in os.listdir(output_dir))
    shutil.rmtree(output_dir)
    assert exported


def test_runner_invalid_out_folder():
    result = subprocess.run([
        "python", "cli/runner.py", "--json", "--out", "/path/does/not/exist"
    ], capture_output=True, text=True)

    assert result.returncode != 0
    assert "does not exist or is not a directory" in result.stdout