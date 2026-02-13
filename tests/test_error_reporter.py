# ============================================================================
# File: /tests/test_error_reporter.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the error report preparation and email submission module.
# Verifies report content, PII redaction, email client opening, and
# CLI error display.
# ============================================================================

import pytest                                              # Test framework
from pathlib import Path                                   # Path operations
from unittest.mock import patch, MagicMock                 # Mocking utilities

from utils.error_reporter import (
    prepare_report,                                        # Report builder
    open_email_client,                                     # Email launcher
    display_cli_error,                                     # CLI error display
    _redact_pii,                                           # PII redaction
    _get_system_info,                                      # System info collector
)


# ============================================================================
# Tests: PII Redaction
# ============================================================================

class TestPIIRedaction:
    """Tests for the _redact_pii helper function."""

    def test_redacts_macos_path(self):
        """Should replace /Users/username with /Users/<user>."""
        text = "Error in /Users/johndoe/Music/file.mp3"
        result = _redact_pii(text)
        assert "johndoe" not in result
        assert "/Users/<user>" in result

    def test_redacts_linux_path(self):
        """Should replace /home/username with /home/<user>."""
        text = "Error in /home/alice/Music/file.mp3"
        result = _redact_pii(text)
        assert "alice" not in result
        assert "/home/<user>" in result

    def test_redacts_windows_path(self):
        """Should replace C:\\Users\\username with C:\\Users\\<user>."""
        text = "Error in C:\\Users\\johndoe\\Music\\file.mp3"
        result = _redact_pii(text)
        assert "johndoe" not in result

    def test_preserves_non_user_paths(self):
        """Should not modify paths without user directories."""
        text = "Error in /var/log/syslog"
        result = _redact_pii(text)
        assert result == text


# ============================================================================
# Tests: System Info
# ============================================================================

class TestSystemInfo:
    """Tests for the _get_system_info helper function."""

    def test_contains_python_version(self):
        """Should include Python version info."""
        info = _get_system_info()
        assert "Python:" in info

    def test_contains_platform(self):
        """Should include platform information."""
        info = _get_system_info()
        assert "Platform:" in info

    def test_contains_architecture(self):
        """Should include architecture information."""
        info = _get_system_info()
        assert "Architecture:" in info


# ============================================================================
# Tests: prepare_report()
# ============================================================================

class TestPrepareReport:
    """Tests for the report preparation function."""

    def test_returns_dict_with_required_keys(self):
        """Should return a dict with subject, body, and timestamp keys."""
        report = prepare_report(error_summary="Test error")
        assert "subject" in report
        assert "body" in report
        assert "timestamp" in report

    def test_includes_error_summary_in_subject(self):
        """Should include the error summary in the email subject."""
        report = prepare_report(error_summary="Crash during scan")
        assert "Crash during scan" in report["subject"]

    def test_includes_error_summary_in_body(self):
        """Should include the error summary in the email body."""
        report = prepare_report(error_summary="Crash during scan")
        assert "Crash during scan" in report["body"]

    def test_includes_system_info_by_default(self):
        """Should include system information by default."""
        report = prepare_report(error_summary="Test")
        assert "Python:" in report["body"]
        assert "Platform:" in report["body"]

    def test_excludes_system_info_when_disabled(self):
        """Should exclude system info when include_system_info=False."""
        report = prepare_report(
            error_summary="Test", include_system_info=False
        )
        assert "System Information" not in report["body"]

    def test_includes_crash_report_content(self, tmp_path):
        """Should include crash report file content when provided."""
        crash_file = tmp_path / "crash_2025-01-01_120000.txt"
        crash_file.write_text("Test crash content here")
        report = prepare_report(
            error_summary="Test",
            crash_report_path=crash_file,
        )
        assert "Test crash content here" in report["body"]

    def test_handles_missing_crash_report(self, tmp_path):
        """Should handle a non-existent crash report path gracefully."""
        missing_file = tmp_path / "nonexistent.txt"
        report = prepare_report(
            error_summary="Test",
            crash_report_path=missing_file,
        )
        assert "subject" in report                         # Should not raise

    def test_redacts_pii_in_logs(self):
        """Should redact PII from log content included in the report."""
        with patch("utils.error_reporter._get_recent_logs",
                   return_value="/Users/johndoe/Music/file.mp3"):
            report = prepare_report(error_summary="Test")
            assert "johndoe" not in report["body"]

    def test_default_subject_without_summary(self):
        """Should use a default subject when no summary is provided."""
        report = prepare_report()
        assert "MeedyaManager Bug Report" in report["subject"]

    def test_truncates_long_subject(self):
        """Should truncate very long error summaries in the subject."""
        long_summary = "A" * 200
        report = prepare_report(error_summary=long_summary)
        assert len(report["subject"]) < 200


# ============================================================================
# Tests: open_email_client()
# ============================================================================

class TestOpenEmailClient:
    """Tests for the email client opener function."""

    def test_opens_mailto_url(self):
        """Should open a mailto: URL via webbrowser."""
        report = {"subject": "Test Report", "body": "Test body"}
        with patch("utils.error_reporter.webbrowser.open") as mock_open:
            result = open_email_client(report)
            assert result is True
            mock_open.assert_called_once()
            call_url = mock_open.call_args[0][0]
            assert call_url.startswith("mailto:")

    def test_returns_false_on_failure(self):
        """Should return False if webbrowser.open raises."""
        report = {"subject": "Test", "body": "Test"}
        with patch("utils.error_reporter.webbrowser.open",
                   side_effect=Exception("no browser")):
            result = open_email_client(report)
            assert result is False

    def test_uses_custom_recipient(self):
        """Should use the custom recipient email address."""
        report = {"subject": "Test", "body": "Test"}
        with patch("utils.error_reporter.webbrowser.open") as mock_open:
            open_email_client(report, recipient="test@example.com")
            call_url = mock_open.call_args[0][0]
            assert "test@example.com" in call_url


# ============================================================================
# Tests: display_cli_error()
# ============================================================================

class TestDisplayCliError:
    """Tests for the CLI error display function."""

    def test_prints_error_summary(self, capsys):
        """Should print the error summary to stdout."""
        display_cli_error("Something went wrong")
        captured = capsys.readouterr()
        assert "Something went wrong" in captured.out

    def test_prints_exception_details(self, capsys):
        """Should print exception type and message when provided."""
        display_cli_error("Scan failed", exception=PermissionError("no access"))
        captured = capsys.readouterr()
        assert "Permission" in captured.out
