# ============================================================================
# File: /tests/test_cli_version.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the CLI --version flag.
# ============================================================================

from click.testing import CliRunner
from cli import cli


def test_version_flag():
    """Verify --version shows the current version string."""
    runner = CliRunner()
    result = runner.invoke(cli, ["--version"])
    assert result.exit_code == 0
    assert "MeedyaManager" in result.output
    assert "1.5-M6" in result.output
