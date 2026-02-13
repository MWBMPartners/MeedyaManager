# ============================================================================
# File: /tests/test_cli_lookup.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the CLI lookup command (cli/commands/lookup.py).
# Uses Click's CliRunner for testing command invocation and output.
# All tests mock the LookupService and extract_metadata to avoid
# making real API calls to external metadata providers.
#
# Test classes:
#   - TestLookupHelp:          --help, --version, no-args error
#   - TestLookupProvidersList: --providers-list display and JSON output
#   - TestLookupSingleFile:    Single file lookup, filters, JSON, errors
#   - TestLookupApply:         --apply, --auto, --dry-run behaviour
#   - TestLookupBatch:         --batch mode with file lists
# ============================================================================

import json                                            # JSON parsing for --json output tests
import pytest                                          # Test framework
from unittest.mock import patch, MagicMock             # Mocking to avoid real API calls
from click.testing import CliRunner                    # CLI testing runner

from cli import cli                                    # Top-level CLI command group
from metadata.providers.base import ProviderResult     # Result dataclass for mock returns


# =============================================================================
# Fixtures — Shared test setup used across all test classes
# =============================================================================

@pytest.fixture
def runner():
    """Provide a Click CliRunner instance for invoking CLI commands.

    The CliRunner captures stdout/stderr and provides exit_code for
    assertion. All tests use this instead of real terminal output.
    """
    return CliRunner()


@pytest.fixture
def mock_lookup_service():
    """Provide a mocked LookupService to prevent real API calls.

    Patches the LookupService class at the import location used by the
    lookup command module (cli.commands.lookup.LookupService). The mock
    instance is pre-configured with sensible default return values:
      - lookup_sync:              Returns one high-confidence result
      - get_available_providers:  Returns one mock provider entry
      - apply_result_sync:        Returns a dict of written tags

    Yields:
        MagicMock: The mock LookupService *instance* (not the class).
    """
    with patch("cli.commands.lookup.LookupService") as MockClass:
        # Create a mock instance that the constructor call will return
        mock_instance = MagicMock()
        MockClass.return_value = mock_instance

        # Default: lookup_sync returns a single high-confidence result
        mock_instance.lookup_sync.return_value = [
            ProviderResult(
                provider_name="mock_provider",         # Identifies the provider source
                title="Test Title",                    # Track/movie title
                artist="Test Artist",                  # Artist/director name
                album="Test Album",                    # Album/show name
                year="2025",                           # Release year
                confidence=0.95,                       # 95% match confidence
                provider_id="mock_123",                # Provider-specific unique ID
            ),
        ]

        # Default: get_available_providers returns one provider status entry
        mock_instance.get_available_providers.return_value = [
            {
                "name": "mock_provider",               # Provider name string
                "category": "music",                   # Provider category
                "requires_auth": False,                # No API key needed
                "available": True,                     # Provider is operational
                "message": "Available",                # Human-readable status
            },
        ]

        # Default: apply_result_sync returns a summary of written tags
        mock_instance.apply_result_sync.return_value = {
            "tags_written": {"title": "Test Title", "artist": "Test Artist"},
            "cover_art_saved": {},                     # No cover art in default mock
            "dry_run": False,                          # Not a dry run by default
        }

        yield mock_instance                            # Yield the instance for test assertions


@pytest.fixture
def mock_extract_metadata():
    """Provide a mocked extract_metadata to return test metadata.

    Patches the extract_metadata function at its import location in the
    lookup command module. Returns a dictionary of standard metadata
    fields that would normally be read from a real media file.

    Yields:
        MagicMock: The patched extract_metadata function.
    """
    with patch("cli.commands.lookup.extract_metadata") as mock:
        # Return a metadata dict matching what the real extractor produces
        mock.return_value = {
            "title": "Test Song",                      # Track title from file tags
            "artist": "Test Artist",                   # Artist from file tags
            "album": "Test Album",                     # Album from file tags
        }
        yield mock                                     # Yield for test assertions


@pytest.fixture
def sample_media_file(tmp_path):
    """Create a dummy media file on disk for lookup tests.

    The lookup command checks os.path.exists() and os.path.isfile()
    before proceeding. This fixture creates a minimal placeholder file
    so those checks pass. The actual metadata extraction is mocked.

    Args:
        tmp_path: Pytest built-in fixture providing a unique temp directory.

    Returns:
        str: Absolute path to the dummy media file.
    """
    filepath = tmp_path / "song.mp3"
    filepath.write_bytes(b"FAKE_MEDIA_DATA")           # Minimal content; extraction is mocked
    return str(filepath)                               # Return as string for CLI argument


# =============================================================================
# TestLookupHelp — --help, --version, and no-args error handling
# =============================================================================

class TestLookupHelp:
    """Tests for help output, version display, and missing-argument errors."""

    def test_help_output(self, runner):
        """--help should show the lookup command usage and all option flags.

        Verifies that the help text includes key options such as --provider,
        --category, --auto, --apply, --dry-run, --no-art, --json, --batch,
        and --providers-list. This ensures Click decorators are wired correctly.
        """
        result = runner.invoke(cli, ["lookup", "--help"])

        # Exit code 0 indicates help was shown successfully
        assert result.exit_code == 0

        # Verify all major options appear in the help output
        assert "--provider" in result.output or "-p" in result.output
        assert "--category" in result.output or "-c" in result.output
        assert "--auto" in result.output
        assert "--apply" in result.output
        assert "--dry-run" in result.output
        assert "--no-art" in result.output
        assert "--json" in result.output
        assert "--batch" in result.output
        assert "--providers-list" in result.output

    def test_version_updated(self, runner):
        """--version should display the current milestone version 1.5-M6.

        The version string is defined in cli/__init__.py via
        @click.version_option(). This test ensures it matches the M5
        milestone version expected after the lookup command was added.
        """
        result = runner.invoke(cli, ["--version"])

        # Exit code 0 indicates version was shown successfully
        assert result.exit_code == 0

        # Verify the version string matches the current milestone
        assert "1.5-M6" in result.output

    def test_no_args_shows_error(self, runner, mock_lookup_service):
        """Invoking lookup with no arguments should show an error message.

        When no file path, --batch, or --providers-list is provided, the
        command should display an error telling the user what is required
        and exit with a non-zero exit code.
        """
        result = runner.invoke(cli, ["lookup"])

        # The command should exit with an error code (SystemExit(1))
        assert result.exit_code != 0

        # The error message should hint at what the user needs to provide
        assert "Error" in result.output or "error" in result.output


# =============================================================================
# TestLookupProvidersList — --providers-list display and JSON output
# =============================================================================

class TestLookupProvidersList:
    """Tests for the --providers-list flag showing registered providers."""

    def test_providers_list(self, runner, mock_lookup_service):
        """--providers-list should display a Rich table with provider names.

        Verifies that the mock provider's name and category appear in the
        output, and that the table header "Registered Metadata Providers"
        or the provider name itself is present.
        """
        result = runner.invoke(cli, ["lookup", "--providers-list"])

        # Exit code 0 indicates the command completed successfully
        assert result.exit_code == 0

        # The mock provider's name should appear in the table output
        assert "mock_provider" in result.output

        # The provider's category should also be visible
        assert "music" in result.output

        # The "Available" status message should be shown
        assert "Available" in result.output

    def test_providers_list_json(self, runner, mock_lookup_service):
        """--providers-list --json should output provider info as JSON array.

        When both flags are combined, the command should produce valid JSON
        containing provider status objects instead of a Rich table.
        """
        result = runner.invoke(cli, ["lookup", "--providers-list", "--json"])

        # Exit code 0 indicates the command completed successfully
        assert result.exit_code == 0

        # Parse the output as JSON to verify it is valid
        data = json.loads(result.output)

        # Should be a list of provider dicts
        assert isinstance(data, list)
        assert len(data) == 1

        # Verify the mock provider's details are in the JSON
        assert data[0]["name"] == "mock_provider"
        assert data[0]["category"] == "music"
        assert data[0]["available"] is True

    def test_providers_list_empty(self, runner, mock_lookup_service):
        """An empty provider registry should show a warning message.

        When get_available_providers() returns an empty list, the command
        should inform the user that no providers are registered and suggest
        checking installation.
        """
        # Override the default mock to return an empty provider list
        mock_lookup_service.get_available_providers.return_value = []

        result = runner.invoke(cli, ["lookup", "--providers-list"])

        # Exit code 0 because this is a clean informational exit
        assert result.exit_code == 0

        # A warning about no providers should be shown
        assert "No providers registered" in result.output


# =============================================================================
# TestLookupSingleFile — Single file lookup with filters and output modes
# =============================================================================

class TestLookupSingleFile:
    """Tests for looking up metadata from a single media file."""

    def test_lookup_shows_results(self, runner, mock_lookup_service,
                                  mock_extract_metadata, sample_media_file):
        """Basic lookup should display a Rich table with result data.

        Invokes the lookup command on a sample file and verifies that the
        mock result's title, artist, and provider name appear in the output.
        """
        result = runner.invoke(cli, ["lookup", sample_media_file])

        # Exit code 0 indicates the lookup completed successfully
        assert result.exit_code == 0

        # The mock result's fields should appear in the Rich table output
        assert "Test Title" in result.output
        assert "Test Artist" in result.output
        assert "mock_provider" in result.output

        # The result count should be displayed
        assert "1 result(s) found" in result.output

    def test_lookup_json_output(self, runner, mock_lookup_service,
                                mock_extract_metadata, sample_media_file):
        """--json should output results as a valid JSON object.

        The JSON output should contain the file path, result count, and
        an array of result objects with provider_name, confidence, title, etc.
        """
        result = runner.invoke(cli, ["lookup", sample_media_file, "--json"])

        # Exit code 0 indicates the lookup completed successfully
        assert result.exit_code == 0

        # Parse the output as JSON
        data = json.loads(result.output)

        # Verify top-level structure
        assert "file" in data
        assert data["result_count"] == 1

        # Verify the result array contains our mock data
        assert len(data["results"]) == 1
        assert data["results"][0]["provider_name"] == "mock_provider"
        assert data["results"][0]["title"] == "Test Title"
        assert data["results"][0]["confidence"] == 0.95

    def test_lookup_no_results(self, runner, mock_lookup_service,
                               mock_extract_metadata, sample_media_file):
        """Empty results should display a warning message.

        When the LookupService returns no matching results, the command
        should show a "No results found" warning with the filename.
        """
        # Override lookup_sync to return an empty list
        mock_lookup_service.lookup_sync.return_value = []

        result = runner.invoke(cli, ["lookup", sample_media_file])

        # Exit code 0 because no-results is informational, not an error
        assert result.exit_code == 0

        # Warning message should mention no results found
        assert "No results found" in result.output

    def test_lookup_with_provider_filter(self, runner, mock_lookup_service,
                                         mock_extract_metadata, sample_media_file):
        """-p spotify should pass the provider filter to LookupService.

        Verifies that the providers keyword argument is correctly forwarded
        to lookup_sync when the user specifies a -p flag.
        """
        result = runner.invoke(cli, ["lookup", sample_media_file, "-p", "spotify"])

        # Exit code 0 indicates the lookup completed
        assert result.exit_code == 0

        # Verify lookup_sync was called with the provider filter
        call_kwargs = mock_lookup_service.lookup_sync.call_args
        assert "providers" in call_kwargs.kwargs or (
            len(call_kwargs.args) > 1                  # Positional arg check as fallback
        )

        # The providers list should contain "spotify"
        if "providers" in call_kwargs.kwargs:
            assert "spotify" in call_kwargs.kwargs["providers"]

    def test_lookup_with_category_filter(self, runner, mock_lookup_service,
                                          mock_extract_metadata, sample_media_file):
        """-c music should pass the category filter to LookupService.

        Verifies that the category keyword argument is correctly forwarded
        to lookup_sync as a ProviderCategory enum value.
        """
        result = runner.invoke(cli, ["lookup", sample_media_file, "-c", "music"])

        # Exit code 0 indicates the lookup completed
        assert result.exit_code == 0

        # Verify lookup_sync was called with the category filter
        call_kwargs = mock_lookup_service.lookup_sync.call_args
        assert "category" in call_kwargs.kwargs

    def test_lookup_nonexistent_file(self, runner, mock_lookup_service, tmp_path):
        """Looking up a non-existent file should show an error message.

        The command should detect that the file does not exist and display
        a "File not found" error without attempting to extract metadata.
        """
        # Construct a path that does not exist on disk
        fake_path = str(tmp_path / "nonexistent_track.mp3")

        result = runner.invoke(cli, ["lookup", fake_path])

        # The command should exit with an error code
        assert result.exit_code != 0

        # Error message should reference the missing file
        assert "not found" in result.output.lower() or "not a regular file" in result.output.lower()


# =============================================================================
# TestLookupApply — --apply, --auto, and --dry-run behaviour
# =============================================================================

class TestLookupApply:
    """Tests for applying lookup results to media files."""

    def test_apply_specific_result(self, runner, mock_lookup_service,
                                   mock_extract_metadata, sample_media_file):
        """--apply 1 should apply the first result to the file.

        Verifies that apply_result_sync is called with the correct result
        and that the output confirms the application.
        """
        result = runner.invoke(cli, ["lookup", sample_media_file, "--apply", "1"])

        # Exit code 0 indicates the apply completed successfully
        assert result.exit_code == 0

        # apply_result_sync should have been called exactly once
        mock_lookup_service.apply_result_sync.assert_called_once()

        # The output should confirm the result was applied
        assert "Applying" in result.output or "Wrote" in result.output

    def test_apply_invalid_index(self, runner, mock_lookup_service,
                                  mock_extract_metadata, sample_media_file):
        """--apply 99 should show an error when the index is out of range.

        The mock returns only one result, so result #99 does not exist.
        The command should display an "Invalid result number" error.
        """
        result = runner.invoke(cli, ["lookup", sample_media_file, "--apply", "99"])

        # Exit code 0 because the command processes lookup first, then reports the error
        assert result.exit_code == 0

        # The output should indicate the result number is invalid
        assert "Invalid result number" in result.output

        # apply_result_sync should NOT have been called
        mock_lookup_service.apply_result_sync.assert_not_called()

    def test_auto_apply_high_confidence(self, runner, mock_lookup_service,
                                         mock_extract_metadata, sample_media_file):
        """--auto should apply the best match when confidence >= 0.8.

        The default mock result has confidence 0.95, which exceeds the
        auto-apply threshold of 0.8. The command should automatically
        apply this result to the file.
        """
        result = runner.invoke(cli, ["lookup", sample_media_file, "--auto"])

        # Exit code 0 indicates auto-apply completed
        assert result.exit_code == 0

        # apply_result_sync should have been called (auto-applied)
        mock_lookup_service.apply_result_sync.assert_called_once()

        # Output should mention auto-applying
        assert "Auto-applying" in result.output

    def test_auto_apply_low_confidence(self, runner, mock_lookup_service,
                                        mock_extract_metadata, sample_media_file):
        """--auto should skip when best confidence < 0.8 threshold.

        Overrides the mock result's confidence to 0.5, which is below the
        default auto-apply threshold. The command should display a "skipped"
        message and NOT call apply_result_sync.
        """
        # Override with a low-confidence result (below 0.8 threshold)
        mock_lookup_service.lookup_sync.return_value = [
            ProviderResult(
                provider_name="mock_provider",
                title="Uncertain Match",               # Title of the low-confidence result
                artist="Maybe Artist",                 # Artist of the low-confidence result
                album="Perhaps Album",                 # Album of the low-confidence result
                confidence=0.5,                        # 50% — below the 80% auto threshold
                provider_id="mock_456",
            ),
        ]

        result = runner.invoke(cli, ["lookup", sample_media_file, "--auto"])

        # Exit code 0 — skipping auto-apply is informational, not an error
        assert result.exit_code == 0

        # apply_result_sync should NOT have been called
        mock_lookup_service.apply_result_sync.assert_not_called()

        # Output should mention that auto-apply was skipped
        assert "skipped" in result.output.lower() or "below threshold" in result.output.lower()

    def test_dry_run_no_write(self, runner, mock_lookup_service,
                              mock_extract_metadata, sample_media_file):
        """--dry-run should show changes without actually writing to the file.

        When --apply 1 and --dry-run are combined, apply_result_sync should
        be called with dry_run=True. The output should indicate it is a
        dry-run preview.
        """
        # Configure apply mock to reflect dry_run mode
        mock_lookup_service.apply_result_sync.return_value = {
            "tags_written": {"title": "Test Title", "artist": "Test Artist"},
            "cover_art_saved": {},
            "dry_run": True,                           # Dry run — no actual writes
        }

        result = runner.invoke(cli, [
            "lookup", sample_media_file, "--apply", "1", "--dry-run"
        ])

        # Exit code 0 indicates the dry-run completed
        assert result.exit_code == 0

        # Verify apply_result_sync was called with dry_run=True
        call_kwargs = mock_lookup_service.apply_result_sync.call_args
        assert call_kwargs.kwargs.get("dry_run") is True or (
            # Check positional args if passed positionally
            True in call_kwargs.args if call_kwargs.args else False
        )

        # Output should mention "dry run" or "Dry run"
        assert "dry run" in result.output.lower() or "Dry run" in result.output


# =============================================================================
# TestLookupBatch — --batch mode for processing multiple files
# =============================================================================

class TestLookupBatch:
    """Tests for batch mode that reads file paths from a text file."""

    def test_batch_mode(self, runner, mock_lookup_service,
                        mock_extract_metadata, tmp_path):
        """--batch should read file paths from a text file and process each.

        Creates a batch file listing two dummy media files, then verifies
        that lookup_sync is called once per file and the batch summary
        is displayed at the end.
        """
        # Create two dummy media files in the temp directory
        file1 = tmp_path / "track_one.mp3"
        file2 = tmp_path / "track_two.mp3"
        file1.write_bytes(b"FAKE_MEDIA_DATA")
        file2.write_bytes(b"FAKE_MEDIA_DATA")

        # Create the batch file listing both paths (one per line)
        batch_file = tmp_path / "batch_list.txt"
        batch_file.write_text(f"{file1}\n{file2}\n")

        result = runner.invoke(cli, ["lookup", "--batch", str(batch_file)])

        # Exit code 0 indicates batch processing completed
        assert result.exit_code == 0

        # lookup_sync should have been called twice (once per file)
        assert mock_lookup_service.lookup_sync.call_count == 2

        # Batch summary should appear in the output
        assert "Batch" in result.output
        assert "2" in result.output                    # Total files count

    def test_batch_nonexistent_file(self, runner, mock_lookup_service, tmp_path):
        """--batch with a nonexistent batch file should show an error.

        The --batch option uses click.Path(exists=True), so Click itself
        should reject a path that does not exist on disk.
        """
        # Reference a batch file that does not exist
        fake_batch = str(tmp_path / "nonexistent_batch.txt")

        result = runner.invoke(cli, ["lookup", "--batch", fake_batch])

        # Click should reject the invalid path with a non-zero exit code
        assert result.exit_code != 0

    def test_batch_empty_file(self, runner, mock_lookup_service, tmp_path):
        """--batch with an empty file should show a warning.

        When the batch file exists but contains no valid file paths
        (empty or only whitespace/comments), the command should display
        a "No file paths found" warning.
        """
        # Create an empty batch file (only whitespace and comments)
        batch_file = tmp_path / "empty_batch.txt"
        batch_file.write_text("# This is a comment\n   \n\n")

        result = runner.invoke(cli, ["lookup", "--batch", str(batch_file)])

        # Exit code 0 because this is a clean informational exit
        assert result.exit_code == 0

        # Warning should mention no file paths were found
        assert "No file paths found" in result.output
