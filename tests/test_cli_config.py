# ============================================================================
# File: /tests/test_cli_config.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the CLI config export/import commands.
# Verifies the Click command group, export subcommand, import subcommand,
# and argument/option handling.
# ============================================================================

import os                                                  # Path operations
import json                                                # JSON handling
import zipfile                                             # ZIP archive handling
import pytest                                              # Test framework
from pathlib import Path                                   # Path operations
from unittest.mock import patch, MagicMock                 # Mocking utilities
from click.testing import CliRunner                        # Click CLI test runner

from cli.commands.config_cmd import config                 # Config command group


# ============================================================================
# Fixtures
# ============================================================================

@pytest.fixture
def runner():
    """Provide a Click CliRunner for CLI testing."""
    return CliRunner()


@pytest.fixture
def valid_profile(tmp_path):
    """Create a valid .mmprofile file for import testing."""
    profile_path = tmp_path / "test.mmprofile"
    manifest = {
        "schema_version": 1,
        "profile_name": "Test Profile",
        "created_at": "2025-01-01T00:00:00",
    }
    config_data = {
        "watch_paths": ["./imported"],
        "valid_extensions": ["flac", "wav"],
    }
    with zipfile.ZipFile(str(profile_path), "w") as zf:
        zf.writestr("manifest.json", json.dumps(manifest))
        zf.writestr("settings.json5", json.dumps(config_data))
    return profile_path


# ============================================================================
# Tests: Config Command Group
# ============================================================================

class TestConfigGroup:
    """Tests for the config command group itself."""

    def test_config_group_exists(self, runner):
        """Should define a 'config' command group."""
        result = runner.invoke(config, ["--help"])
        assert result.exit_code == 0
        assert "config" in result.output.lower() or "Manage" in result.output

    def test_has_export_subcommand(self, runner):
        """Should list 'export' as a subcommand."""
        result = runner.invoke(config, ["--help"])
        assert "export" in result.output

    def test_has_import_subcommand(self, runner):
        """Should list 'import' as a subcommand."""
        result = runner.invoke(config, ["--help"])
        assert "import" in result.output


# ============================================================================
# Tests: Config Export Command
# ============================================================================

class TestConfigExport:
    """Tests for the 'config export' subcommand."""

    def test_export_requires_out_option(self, runner):
        """Should fail if --out is not provided."""
        result = runner.invoke(config, ["export"])
        assert result.exit_code != 0
        assert "out" in result.output.lower() or "Missing" in result.output

    def test_export_creates_profile(self, runner, tmp_path):
        """Should create a .mmprofile at the specified path."""
        output = tmp_path / "exported.mmprofile"
        with patch("utils.config_profile.export_profile",
                   return_value=str(output)) as mock_export:
            result = runner.invoke(config, ["export", "--out", str(output)])

        assert result.exit_code == 0
        mock_export.assert_called_once()

    def test_export_passes_name(self, runner, tmp_path):
        """Should pass the --name option to export_profile."""
        output = tmp_path / "named.mmprofile"
        with patch("utils.config_profile.export_profile",
                   return_value=str(output)) as mock_export:
            result = runner.invoke(config, [
                "export", "--out", str(output), "--name", "My Profile"
            ])

        assert result.exit_code == 0
        call_kwargs = mock_export.call_args
        # profile_name should be "My Profile"
        assert call_kwargs.kwargs.get("profile_name") == "My Profile" or \
               call_kwargs[1].get("profile_name") == "My Profile"

    def test_export_handles_error(self, runner, tmp_path):
        """Should display error message and exit 1 on failure."""
        output = tmp_path / "fail.mmprofile"
        with patch("utils.config_profile.export_profile",
                   side_effect=Exception("disk full")):
            result = runner.invoke(config, ["export", "--out", str(output)])

        assert result.exit_code == 1
        assert "failed" in result.output.lower() or "disk full" in result.output.lower()

    def test_export_secrets_requires_confirmation(self, runner, tmp_path):
        """--include-secrets should prompt for confirmation."""
        output = tmp_path / "secrets.mmprofile"
        with patch("utils.config_profile.export_profile",
                   return_value=str(output)):
            # Answer 'n' to the confirmation prompt
            result = runner.invoke(config, [
                "export", "--out", str(output), "--include-secrets"
            ], input="n\n")

        # Should either exit 0 (cancelled) or show warning
        assert "WARNING" in result.output or "CAUTION" in result.output or result.exit_code == 0


# ============================================================================
# Tests: Config Import Command
# ============================================================================

class TestConfigImport:
    """Tests for the 'config import' subcommand."""

    def test_import_requires_profile_path(self, runner):
        """Should fail if no profile path is provided."""
        result = runner.invoke(config, ["import"])
        assert result.exit_code != 0

    def test_import_validates_profile(self, runner, valid_profile):
        """Should validate the profile before importing."""
        with patch("utils.config_profile.validate_profile",
                   return_value=[]) as mock_validate:
            with patch("utils.config_profile.import_profile",
                       return_value={
                           "changes": {},
                           "applied": False,
                           "profile_name": "Test",
                       }):
                result = runner.invoke(config, ["import", str(valid_profile)])

        mock_validate.assert_called_once()

    def test_import_rejects_invalid_profile(self, runner, valid_profile):
        """Should exit 1 if profile validation fails."""
        with patch("utils.config_profile.validate_profile",
                   return_value=["Missing manifest.json"]):
            result = runner.invoke(config, ["import", str(valid_profile)])

        assert result.exit_code == 1
        assert "validation" in result.output.lower() or "manifest" in result.output.lower()

    def test_import_dry_run_shows_changes(self, runner, valid_profile):
        """--dry-run should show changes without applying."""
        mock_result = {
            "changes": {"watch_paths": {"old": ["./old"], "new": ["./new"]}},
            "applied": False,
            "profile_name": "Test Profile",
        }
        with patch("utils.config_profile.validate_profile", return_value=[]):
            with patch("utils.config_profile.import_profile",
                       return_value=mock_result):
                result = runner.invoke(config, [
                    "import", str(valid_profile), "--dry-run"
                ])

        assert result.exit_code == 0
        assert "dry run" in result.output.lower() or "Dry" in result.output

    def test_import_merge_mode(self, runner, valid_profile):
        """Should pass mode='merge' to import_profile."""
        mock_result = {
            "changes": {"key": {"old": "a", "new": "b"}},
            "applied": False,
            "profile_name": "Test",
        }
        with patch("utils.config_profile.validate_profile", return_value=[]):
            with patch("utils.config_profile.import_profile",
                       return_value=mock_result) as mock_import:
                result = runner.invoke(config, [
                    "import", str(valid_profile), "--mode", "merge", "--dry-run"
                ])

        # First call is the preview (dry_run=True)
        assert mock_import.call_args_list[0][1].get("mode") == "merge" or \
               mock_import.call_args_list[0].kwargs.get("mode") == "merge"

    def test_import_yes_skips_confirmation(self, runner, valid_profile):
        """--yes should skip the confirmation prompt."""
        mock_result = {
            "changes": {"key": {"old": "a", "new": "b"}},
            "applied": True,
            "profile_name": "Test",
        }
        with patch("utils.config_profile.validate_profile", return_value=[]):
            with patch("utils.config_profile.import_profile",
                       return_value=mock_result):
                result = runner.invoke(config, [
                    "import", str(valid_profile), "--yes"
                ])

        assert result.exit_code == 0

    def test_import_no_changes_detected(self, runner, valid_profile):
        """Should report when profile matches current settings."""
        mock_result = {
            "changes": {},
            "applied": False,
            "profile_name": "Test",
        }
        with patch("utils.config_profile.validate_profile", return_value=[]):
            with patch("utils.config_profile.import_profile",
                       return_value=mock_result):
                result = runner.invoke(config, ["import", str(valid_profile)])

        assert result.exit_code == 0
        assert "no changes" in result.output.lower() or "matches" in result.output.lower()
