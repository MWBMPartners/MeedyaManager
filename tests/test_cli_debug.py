# ============================================================================
# File: /tests/test_cli_debug.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Click-based debug command.
# Replaces the legacy test_metadata_debugger.py.
# Uses click.testing.CliRunner instead of subprocess.
# ============================================================================

import os
import json
import pytest
from click.testing import CliRunner
from cli import cli


@pytest.fixture
def runner():
    """Provide a Click CliRunner instance."""
    return CliRunner()


@pytest.fixture
def temp_media_file(tmp_path):
    """Create a temporary media file for debug testing."""
    test_file = tmp_path / "debug_test.mp3"
    test_file.write_text("FAKE_AUDIO_DATA")
    return test_file


def test_debug_help(runner):
    """Verify the debug command shows help text."""
    result = runner.invoke(cli, ["debug", "--help"])
    assert result.exit_code == 0
    assert "Inspect metadata" in result.output


def test_debug_valid_file(runner, temp_media_file):
    """Verify debug command displays metadata for a valid file."""
    result = runner.invoke(cli, ["debug", str(temp_media_file)])
    assert result.exit_code == 0
    # Should contain the metadata table with field names
    assert "Metadata" in result.output
    assert "extension" in result.output


def test_debug_json_export(runner, temp_media_file, tmp_path):
    """Verify debug command exports metadata as JSON."""
    out_dir = tmp_path / "json_out"
    out_dir.mkdir()

    result = runner.invoke(cli, [
        "debug", str(temp_media_file),
        "--json",
        "--out", str(out_dir),
    ])
    assert result.exit_code == 0
    assert "Exported metadata to" in result.output

    # Verify JSON file was created
    expected_json = out_dir / "debug_test.metadata.json"
    assert expected_json.exists(), "JSON metadata file was not created"

    # Verify JSON content is valid
    with open(expected_json) as f:
        data = json.load(f)
    assert isinstance(data, dict)
    assert "extension" in data


def test_debug_file_not_found(runner):
    """Verify debug command fails gracefully for non-existent files."""
    result = runner.invoke(cli, ["debug", "/nonexistent/file.mp3"])
    assert result.exit_code != 0


def test_debug_json_mkdir(runner, temp_media_file, tmp_path):
    """Verify --mkdir creates the output directory for JSON export."""
    out_dir = tmp_path / "new_output"

    result = runner.invoke(cli, [
        "debug", str(temp_media_file),
        "--json",
        "--out", str(out_dir),
        "--mkdir",
    ])
    assert result.exit_code == 0
    assert os.path.isdir(out_dir), "Output directory was not created with --mkdir"
