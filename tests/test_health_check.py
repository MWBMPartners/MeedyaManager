# ============================================================================
# File: /tests/test_health_check.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the startup health check module.
# Verifies individual checks, overall runner, severity levels,
# and CLI formatting.
# ============================================================================

import sys                                                 # Python version info
import pytest                                              # Test framework
from pathlib import Path                                   # Path operations
from unittest.mock import patch, MagicMock                 # Mocking utilities

from utils.health_check import (
    run_startup_checks,                                    # Main runner
    format_results_for_cli,                                # CLI formatter
    Severity,                                              # Severity enum
    HealthCheckResult,                                     # Result dataclass
    _check_python_version,                                 # Individual checks
    _check_config_valid,
    _check_watch_dirs,
    _check_log_dir_writable,
    _check_disk_space,
)


# ============================================================================
# Tests: HealthCheckResult Dataclass
# ============================================================================

class TestHealthCheckResult:
    """Tests for the HealthCheckResult dataclass."""

    def test_has_required_fields(self):
        """Should have name, severity, message, and suggestion."""
        result = HealthCheckResult(
            name="test", severity=Severity.OK, message="All good",
        )
        assert result.name == "test"
        assert result.severity == Severity.OK
        assert result.message == "All good"
        assert result.suggestion == ""

    def test_suggestion_defaults_to_empty(self):
        """Suggestion should default to empty string."""
        result = HealthCheckResult(
            name="test", severity=Severity.OK, message="OK",
        )
        assert result.suggestion == ""


# ============================================================================
# Tests: Individual Checks
# ============================================================================

class TestPythonVersionCheck:
    """Tests for the Python version compatibility check."""

    def test_passes_on_current_python(self):
        """Should pass on the current Python version (>=3.11)."""
        result = _check_python_version()
        assert result.severity == Severity.OK

    def test_fails_on_old_python(self):
        """Should fail on Python < 3.11."""
        with patch.object(sys, "version_info", (3, 9, 0)):
            result = _check_python_version()
            assert result.severity == Severity.CRITICAL


class TestConfigCheck:
    """Tests for the configuration file validity check."""

    def test_passes_when_config_loads(self):
        """Should pass when config loads successfully."""
        result = _check_config_valid()
        assert result.severity == Severity.OK

    def test_warns_on_config_failure(self):
        """Should warn when config fails to load."""
        with patch("utils.config_loader.get_config", side_effect=Exception("bad config")):
            result = _check_config_valid()
            assert result.severity == Severity.WARNING


class TestWatchDirsCheck:
    """Tests for the watch directory accessibility check."""

    def test_warns_on_missing_dirs(self):
        """Should warn when watch directories don't exist."""
        with patch("utils.config_loader.get_config",
                   return_value=["/nonexistent/path/that/does/not/exist"]):
            result = _check_watch_dirs()
            assert result.severity == Severity.WARNING

    def test_warns_on_no_dirs_configured(self):
        """Should warn when no watch directories are configured."""
        with patch("utils.config_loader.get_config", return_value=[]):
            result = _check_watch_dirs()
            assert result.severity == Severity.WARNING

    def test_passes_with_existing_dirs(self, tmp_path):
        """Should pass when watch directories exist."""
        with patch("utils.config_loader.get_config", return_value=[str(tmp_path)]):
            result = _check_watch_dirs()
            assert result.severity == Severity.OK


class TestLogDirCheck:
    """Tests for the log directory write permission check."""

    def test_passes_when_writable(self):
        """Should pass when the log directory is writable."""
        result = _check_log_dir_writable()
        assert result.severity == Severity.OK


class TestDiskSpaceCheck:
    """Tests for the disk space check."""

    def test_passes_with_enough_space(self):
        """Should pass when there is sufficient disk space."""
        result = _check_disk_space()
        # On a normal development machine, this should pass
        assert result.severity in (Severity.OK, Severity.WARNING)


# ============================================================================
# Tests: run_startup_checks()
# ============================================================================

class TestRunStartupChecks:
    """Tests for the main health check runner."""

    def test_returns_list_of_results(self):
        """Should return a list of HealthCheckResult objects."""
        results = run_startup_checks()
        assert isinstance(results, list)
        assert all(isinstance(r, HealthCheckResult) for r in results)

    def test_runs_all_checks(self):
        """Should run at least 5 health checks."""
        results = run_startup_checks()
        assert len(results) >= 5

    def test_includes_python_version_check(self):
        """Should include a python_version check."""
        results = run_startup_checks()
        names = [r.name for r in results]
        assert "python_version" in names

    def test_includes_config_check(self):
        """Should include a config_valid check."""
        results = run_startup_checks()
        names = [r.name for r in results]
        assert "config_valid" in names

    def test_handles_check_that_crashes(self):
        """Should handle a check that raises an exception."""
        with patch("utils.health_check._check_python_version",
                   side_effect=Exception("check crashed")):
            results = run_startup_checks()
            # Should still return results for other checks
            assert len(results) >= 5


# ============================================================================
# Tests: format_results_for_cli()
# ============================================================================

class TestFormatResultsForCli:
    """Tests for the CLI result formatter."""

    def test_formats_ok_results(self):
        """Should format OK results with OK symbol."""
        results = [HealthCheckResult(
            name="test", severity=Severity.OK, message="All good",
        )]
        text = format_results_for_cli(results)
        assert "OK" in text
        assert "All good" in text

    def test_formats_warning_results(self):
        """Should format WARNING results with WARN symbol."""
        results = [HealthCheckResult(
            name="test", severity=Severity.WARNING, message="Watch out",
            suggestion="Fix it",
        )]
        text = format_results_for_cli(results)
        assert "WARN" in text
        assert "Watch out" in text
        assert "Fix it" in text

    def test_formats_critical_results(self):
        """Should format CRITICAL results with CRIT symbol."""
        results = [HealthCheckResult(
            name="test", severity=Severity.CRITICAL, message="Fatal",
        )]
        text = format_results_for_cli(results)
        assert "CRIT" in text
        assert "Fatal" in text

    def test_includes_summary_line(self):
        """Should include a summary line with counts."""
        results = [
            HealthCheckResult(name="a", severity=Severity.OK, message="OK"),
            HealthCheckResult(name="b", severity=Severity.WARNING, message="Warn"),
        ]
        text = format_results_for_cli(results)
        assert "1 OK" in text
        assert "1 warnings" in text
