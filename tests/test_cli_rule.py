# ============================================================================
# File: /tests/test_cli_rule.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Click-based rule command.
# Validates template expansion, error handling, and sample mode.
# ============================================================================

import pytest
from click.testing import CliRunner
from cli import cli


@pytest.fixture
def runner():
    """Provide a Click CliRunner instance."""
    return CliRunner()


def test_rule_help(runner):
    """Verify the rule command shows help text."""
    result = runner.invoke(cli, ["rule", "--help"])
    assert result.exit_code == 0
    assert "Test a rename template" in result.output


def test_rule_with_sample(runner):
    """Verify rule command works with --sample and default template."""
    result = runner.invoke(cli, ["rule", "--sample"])
    assert result.exit_code == 0
    assert "Result" in result.output


def test_rule_custom_template_with_sample(runner):
    """Verify rule command expands a custom template against sample data."""
    result = runner.invoke(cli, [
        "rule",
        "--sample",
        "--template", "{media_class}/{title}.{ext}",
    ])
    assert result.exit_code == 0
    assert "Music/Sample Track.mp3" in result.output


def test_rule_missing_tag(runner):
    """Verify rule command reports missing tags in templates."""
    result = runner.invoke(cli, [
        "rule",
        "--sample",
        "--template", "{nonexistent_tag}/{title}.{ext}",
    ])
    assert result.exit_code != 0
    assert "Missing tag" in result.output


def test_rule_with_file(runner, tmp_path):
    """Verify rule command works with a real file."""
    test_file = tmp_path / "ruletest.mp3"
    test_file.write_text("FAKE_AUDIO")

    result = runner.invoke(cli, [
        "rule",
        "--file", str(test_file),
        "--template", "{media_class}/{title}.{ext}",
    ])
    assert result.exit_code == 0
    assert "Result" in result.output


def test_rule_no_source(runner):
    """Verify rule command prompts when no file or --sample is provided."""
    result = runner.invoke(cli, ["rule"])
    assert result.exit_code != 0
    assert "Please provide" in result.output or "--file" in result.output
