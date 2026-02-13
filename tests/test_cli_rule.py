# ============================================================================
# File: /tests/test_cli_rule.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Click-based rule command.
# Validates template expansion, error handling, sample mode, and validation.
#
# M3 Update: Tests use new <Tag> syntax. Legacy {placeholder} also tested.
# ============================================================================

import pytest                                      # Test framework
from click.testing import CliRunner                # CLI test runner
from cli import cli                                # CLI entry point


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


def test_rule_new_syntax_with_sample(runner):
    """Verify rule command expands a <Tag> template against sample data."""
    result = runner.invoke(cli, [
        "rule",
        "--sample",
        "--template", "<Media Class>/<Title>.<Ext>",
    ])
    assert result.exit_code == 0
    # Rule engine resolves <Media Class> to "Music", <Title> to "Sample Track"
    assert "Music" in result.output
    assert "Sample Track" in result.output


def test_rule_function_in_template(runner):
    """Verify rule command handles $Pad() function in template."""
    result = runner.invoke(cli, [
        "rule",
        "--sample",
        "--template", "<Artist>/<$Pad(<Track #>,2)> - <Title>.<Ext>",
    ])
    assert result.exit_code == 0
    # Track # is "3" → $Pad to 2 digits → "03"
    assert "03" in result.output


def test_rule_validate_valid(runner):
    """Verify --validate reports valid template."""
    result = runner.invoke(cli, [
        "rule",
        "--validate",
        "--template", "<Artist>/<Album>/<Title>.<Ext>",
    ])
    assert result.exit_code == 0
    assert "valid" in result.output.lower()


def test_rule_validate_invalid(runner):
    """Verify --validate reports syntax errors."""
    result = runner.invoke(cli, [
        "rule",
        "--validate",
        "--template", "$If(<Title>=test,yes",
    ])
    assert result.exit_code != 0


def test_rule_with_file(runner, tmp_path):
    """Verify rule command works with a real file."""
    test_file = tmp_path / "ruletest.mp3"
    test_file.write_text("FAKE_AUDIO")

    result = runner.invoke(cli, [
        "rule",
        "--file", str(test_file),
        "--template", "<Media Class>/<Title>.<Ext>",
    ])
    assert result.exit_code == 0
    assert "Result" in result.output


def test_rule_no_source(runner):
    """Verify rule command prompts when no file or --sample is provided."""
    result = runner.invoke(cli, ["rule"])
    assert result.exit_code != 0
    assert "Please provide" in result.output or "--file" in result.output


def test_rule_shows_tag_table(runner):
    """Verify rule command displays available tags table."""
    result = runner.invoke(cli, ["rule", "--sample"])
    assert result.exit_code == 0
    # The tag table should show display names and internal keys
    assert "<Title>" in result.output
    assert "<Artist>" in result.output
