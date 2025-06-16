# ============================================================================
# File: /tests/test_runner_dryrun_json.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests dry-run behavior of runner.py when using --json and --out with --mkdir.
# Validates metadata export, output directory creation, and file validation handling.
# ============================================================================

import os
import subprocess
import tempfile
import shutil
import pytest
import json

TEST_MEDIA_FILE = "tests/test_files/sample.mkv"

@pytest.fixture(scope="function")
def setup_temp_environment():
    temp_dir = tempfile.mkdtemp()
    target_file = os.path.join(temp_dir, "sample.mkv")
    shutil.copy(TEST_MEDIA_FILE, target_file)
    yield temp_dir, target_file
    shutil.rmtree(temp_dir)

def test_dry_run_json_output_with_mkdir(setup_temp_environment):
    temp_dir, test_file = setup_temp_environment
    export_dir = os.path.join(temp_dir, "json_output")

    cmd = [
        "python", "cli/runner.py",
        "--json",
        "--out", export_dir,
        "--mkdir"
    ]

    result = subprocess.run(cmd, capture_output=True, text=True)

    assert result.returncode == 0
    assert os.path.isdir(export_dir)

    json_files = [f for f in os.listdir(export_dir) if f.endswith(".metadata.json")]
    assert len(json_files) > 0
    assert "Exported metadata to" in result.stdout

    for json_file in json_files:
        with open(os.path.join(export_dir, json_file), 'r', encoding='utf-8') as f:
            data = json.load(f)
            assert isinstance(data, dict)
            assert "title" in data or "format" in data

def test_dry_run_json_output_missing_out(setup_temp_environment):
    temp_dir, test_file = setup_temp_environment
    fake_output = os.path.join(temp_dir, "does_not_exist")

    cmd = [
        "python", "cli/runner.py",
        "--json",
        "--out", fake_output
    ]

    result = subprocess.run(cmd, capture_output=True, text=True)

    assert result.returncode != 0
    assert "does not exist" in result.stdout or result.stderr

def test_dry_run_invalid_file():
    cmd = [
        "python", "cli/runner.py",
        "--json",
        "--out", "/tmp"
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    assert result.returncode == 0  # Runner should not crash even if no valid files
    assert "Dry-run rename" in result.stdout or "[SIMULATION]" in result.stdout