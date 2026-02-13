# ============================================================================
# File: /tests/test_cli_edit.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the CLI edit command (cli/commands/edit.py).
# Uses Click's CliRunner for testing command invocation and output.
# Tests tag reading, writing, removal, dry-run, JSON export, and cover art.
# ============================================================================

import json                                            # JSON parsing for export tests
import pytest                                          # Test framework
from click.testing import CliRunner                    # CLI testing runner

from cli import cli                                    # Top-level CLI command group
from metadata.editor import TagEditor                  # Tag reading for verification


@pytest.fixture
def runner():
    """Provide a Click CliRunner instance."""
    return CliRunner()


# =============================================================================
# Tag Display Tests (no edit options)
# =============================================================================

class TestEditDisplay:
    """Tests for displaying tags (no --set/--remove options)."""

    def test_display_tags_mp3(self, runner, real_mp3_file):
        """Displaying tags for an MP3 file should show a table with tag names."""
        result = runner.invoke(cli, ["edit", real_mp3_file])
        assert result.exit_code == 0
        assert "Integration Artist" in result.output
        assert "Integration Album" in result.output

    def test_display_tags_flac(self, runner, real_flac_file):
        """Displaying tags for a FLAC file should show Vorbis Comment values."""
        result = runner.invoke(cli, ["edit", real_flac_file])
        assert result.exit_code == 0
        assert "FLAC Integration Artist" in result.output

    def test_json_export(self, runner, real_mp3_file):
        """--json should output tags as valid JSON."""
        result = runner.invoke(cli, ["edit", real_mp3_file, "--json"])
        assert result.exit_code == 0
        tags = json.loads(result.output)
        assert isinstance(tags, dict)
        assert tags["artist"] == "Integration Artist"
        assert tags["album"] == "Integration Album"

    def test_invalid_file_path(self, runner, tmp_path):
        """Non-existent file should produce an error."""
        result = runner.invoke(cli, ["edit", str(tmp_path / "nonexistent.mp3")])
        assert result.exit_code != 0


# =============================================================================
# Tag Writing Tests (--set)
# =============================================================================

class TestEditWrite:
    """Tests for writing tags via --set."""

    def test_set_single_tag(self, runner, real_mp3_file):
        """--set should modify a tag and show the change."""
        result = runner.invoke(cli, [
            "edit", real_mp3_file, "--set", "Artist=CLI Test Artist"
        ])
        assert result.exit_code == 0
        assert "CLI Test Artist" in result.output

        # Verify the write persisted
        tags = TagEditor().read_tags(real_mp3_file)
        assert tags["artist"] == "CLI Test Artist"

    def test_set_multiple_tags(self, runner, real_mp3_file):
        """Multiple --set options should modify multiple tags."""
        result = runner.invoke(cli, [
            "edit", real_mp3_file,
            "--set", "Genre=Jazz",
            "--set", "Year=2030",
        ])
        assert result.exit_code == 0

        tags = TagEditor().read_tags(real_mp3_file)
        assert tags["genre"] == "Jazz"
        assert tags["year"] == "2030"

    def test_set_display_name(self, runner, real_mp3_file):
        """Display names like 'Album Artist' should work as tag keys."""
        result = runner.invoke(cli, [
            "edit", real_mp3_file, "--set", "Album Artist=New Album Artist"
        ])
        assert result.exit_code == 0

        tags = TagEditor().read_tags(real_mp3_file)
        assert tags["album_artist"] == "New Album Artist"

    def test_dry_run_no_write(self, runner, real_mp3_file):
        """--dry-run should show changes but not modify the file."""
        original_tags = TagEditor().read_tags(real_mp3_file)
        original_artist = original_tags["artist"]

        result = runner.invoke(cli, [
            "edit", real_mp3_file, "--dry-run", "--set", "Artist=Should Not Write"
        ])
        assert result.exit_code == 0
        assert "Dry Run" in result.output or "dry run" in result.output

        # Verify original value unchanged
        tags = TagEditor().read_tags(real_mp3_file)
        assert tags["artist"] == original_artist

    def test_set_no_changes(self, runner, real_mp3_file):
        """Setting a tag to its current value should report no changes."""
        result = runner.invoke(cli, [
            "edit", real_mp3_file, "--set", "Artist=Integration Artist"
        ])
        assert result.exit_code == 0
        assert "No changes" in result.output or "already" in result.output

    def test_invalid_set_format(self, runner, real_mp3_file):
        """--set without '=' should produce an error."""
        result = runner.invoke(cli, [
            "edit", real_mp3_file, "--set", "InvalidFormat"
        ])
        assert result.exit_code != 0


# =============================================================================
# Tag Removal Tests (--remove)
# =============================================================================

class TestEditRemove:
    """Tests for removing tags via --remove."""

    def test_remove_tag(self, runner, real_mp3_file):
        """--remove should remove a tag from the file."""
        result = runner.invoke(cli, [
            "edit", real_mp3_file, "--remove", "Genre"
        ])
        assert result.exit_code == 0

        tags = TagEditor().read_tags(real_mp3_file)
        assert "genre" not in tags or tags.get("genre") == ""


# =============================================================================
# Cover Art Tests
# =============================================================================

class TestEditCoverArt:
    """Tests for cover art operations via CLI."""

    def test_set_cover_art(self, runner, real_mp3_file, tmp_path):
        """--cover should embed a JPEG image into the file."""
        # Create a minimal JPEG file
        jpeg_path = str(tmp_path / "cover.jpg")
        # Minimal valid JPEG (just SOI + EOI markers)
        with open(jpeg_path, "wb") as f:
            f.write(
                b'\xff\xd8\xff\xe0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00'
                b'\xff\xdb\x00C\x00\x08\x06\x06\x07\x06\x05\x08\x07\x07\x07\t\t'
                b'\x08\n\x0c\x14\r\x0c\x0b\x0b\x0c\x19\x12\x13\x0f\x14\x1d\x1a'
                b'\x1f\x1e\x1d\x1a\x1c\x1c $.\' ",#\x1c\x1c(7),01444\x1f\'9=82<.342'
                b'\xff\xc0\x00\x0b\x08\x00\x01\x00\x01\x01\x01\x11\x00'
                b'\xff\xc4\x00\x1f\x00\x00\x01\x05\x01\x01\x01\x01\x01\x01\x00'
                b'\x00\x00\x00\x00\x00\x00\x00\x01\x02\x03\x04\x05\x06\x07\x08\t\n\x0b'
                b'\xff\xda\x00\x08\x01\x01\x00\x00?\x00T\xdb\xa1\x8e(\xa0\x02\x80'
                b'\xff\xd9'
            )

        result = runner.invoke(cli, [
            "edit", real_mp3_file, "--cover", jpeg_path
        ])
        assert result.exit_code == 0
        assert "Cover art set" in result.output

        # Verify cover art was embedded
        covers = TagEditor().read_cover_art(real_mp3_file)
        assert len(covers) >= 1

    def test_remove_cover_art(self, runner, real_mp3_file, tmp_path):
        """--remove-cover should remove all embedded cover art."""
        # First embed cover art
        jpeg_path = str(tmp_path / "cover.jpg")
        with open(jpeg_path, "wb") as f:
            f.write(b'\xff\xd8\xff\xd9')              # Minimal JPEG

        editor = TagEditor()
        editor.write_cover_art(real_mp3_file, b'\xff\xd8\xff\xd9', "jpeg")

        result = runner.invoke(cli, [
            "edit", real_mp3_file, "--remove-cover"
        ])
        assert result.exit_code == 0


# =============================================================================
# Help and Version Tests
# =============================================================================

class TestEditHelp:
    """Tests for --help output and version."""

    def test_help_output(self, runner):
        """--help should show command usage and options."""
        result = runner.invoke(cli, ["edit", "--help"])
        assert result.exit_code == 0
        assert "--set" in result.output
        assert "--remove" in result.output
        assert "--cover" in result.output
        assert "--dry-run" in result.output
        assert "--json" in result.output

    def test_version_updated(self, runner):
        """CLI version should be updated to M4."""
        result = runner.invoke(cli, ["--version"])
        assert result.exit_code == 0
        assert "1.3-M4" in result.output
