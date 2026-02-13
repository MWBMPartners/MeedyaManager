# ============================================================================
# File: /tests/test_cli_scan.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Click-based scan command.
# Replaces the legacy test_runner_cli.py and test_runner_dryrun_json.py.
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


def test_scan_help(runner):
    """Verify the scan command shows help text."""
    result = runner.invoke(cli, ["scan", "--help"])
    assert result.exit_code == 0
    assert "Scan watch folders" in result.output


def test_scan_runs_without_error(runner):
    """Verify scan command runs without crashing (even with no matching files)."""
    result = runner.invoke(cli, ["scan"])
    assert result.exit_code == 0
    # Should contain either results or "No matching media files" message
    assert "Summary" in result.output or "No matching" in result.output


def test_scan_with_custom_path(runner, tmp_path):
    """Verify scan accepts --path option for custom watch directories."""
    # Create a dummy media file in the temp path
    test_file = tmp_path / "test.mp3"
    test_file.write_text("FAKE_AUDIO")

    result = runner.invoke(cli, ["scan", "--path", str(tmp_path)])
    assert result.exit_code == 0
    assert "Summary" in result.output


def test_scan_simulate_off(runner):
    """Verify --simulate-off flag is accepted and changes output."""
    result = runner.invoke(cli, ["scan", "--simulate-off"])
    assert result.exit_code == 0
    assert "simulation disabled" in result.output


def test_scan_json_export_with_mkdir(runner, tmp_path):
    """Verify JSON export with --mkdir creates the output directory."""
    out_dir = tmp_path / "json_output"

    # Create a media file to scan
    media_dir = tmp_path / "media"
    media_dir.mkdir()
    test_file = media_dir / "export_test.mp3"
    test_file.write_text("FAKE_AUDIO")

    result = runner.invoke(cli, [
        "scan",
        "--json",
        "--out", str(out_dir),
        "--mkdir",
        "--path", str(media_dir),
    ])
    assert result.exit_code == 0
    assert os.path.isdir(out_dir), "Output directory was not created"


def test_scan_json_export_missing_out_no_mkdir(runner):
    """Verify scan fails gracefully when --out folder doesn't exist without --mkdir."""
    result = runner.invoke(cli, [
        "scan", "--json", "--out", "/nonexistent/path/does/not/exist"
    ])
    assert result.exit_code != 0
    assert "does not exist" in result.output
