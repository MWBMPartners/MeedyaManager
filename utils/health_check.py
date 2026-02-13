# ============================================================================
# File: /utils/health_check.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Startup health checks for MeedyaManager.
#
# Validates the runtime environment before the application fully launches.
# Checks include: configuration file validity, watch directory accessibility,
# log directory write permissions, MediaInfo library availability,
# disk space sufficiency, and Python version compatibility.
#
# Each check returns a HealthCheckResult with a severity level (OK, WARNING,
# CRITICAL). Critical failures should prevent the application from starting;
# warnings are displayed but do not block startup.
#
# Usage:
#   from utils.health_check import run_startup_checks, Severity
#
#   results = run_startup_checks()
#   critical = [r for r in results if r.severity == Severity.CRITICAL]
#   if critical:
#       # Show error dialog and exit
# ============================================================================

import os                                                  # Path and disk operations
import sys                                                 # Python version info
import shutil                                              # Disk usage
import logging                                             # Structured logging
from enum import Enum                                      # Severity enum
from dataclasses import dataclass                          # Result dataclass
from pathlib import Path                                   # Path operations

logger = logging.getLogger("MeedyaManager.HealthCheck")


class Severity(Enum):
    """Severity levels for health check results."""
    OK = "ok"                                              # Check passed
    WARNING = "warning"                                    # Non-blocking issue
    CRITICAL = "critical"                                  # Blocking — app should not start


@dataclass
class HealthCheckResult:
    """
    Result of a single health check.

    Attributes:
        name:        Short identifier for the check (e.g., "config_valid").
        severity:    Severity level (OK, WARNING, CRITICAL).
        message:     Human-readable description of the result.
        suggestion:  Actionable advice if the check failed (empty if OK).
    """
    name: str
    severity: Severity
    message: str
    suggestion: str = ""


# ============================================================================
# Individual Health Checks
# ============================================================================

def _check_python_version() -> HealthCheckResult:
    """
    Verify that the Python version meets the minimum requirement (3.11+).

    Returns:
        HealthCheckResult with OK if compatible, CRITICAL if not.
    """
    major, minor = sys.version_info[:2]
    if major >= 3 and minor >= 11:
        return HealthCheckResult(
            name="python_version",
            severity=Severity.OK,
            message=f"Python {major}.{minor} is compatible.",
        )
    return HealthCheckResult(
        name="python_version",
        severity=Severity.CRITICAL,
        message=f"Python {major}.{minor} is not supported. Minimum: 3.11.",
        suggestion="Install Python 3.11 or later from python.org.",
    )


def _check_config_valid() -> HealthCheckResult:
    """
    Verify that the configuration file can be loaded without errors.

    Returns:
        HealthCheckResult with OK if config loads, WARNING if it fails.
    """
    try:
        from utils.config_loader import get_config
        # Try to read a known key to verify the config is functional
        get_config("watch_paths", default=[])
        return HealthCheckResult(
            name="config_valid",
            severity=Severity.OK,
            message="Configuration file loaded successfully.",
        )
    except Exception as e:
        return HealthCheckResult(
            name="config_valid",
            severity=Severity.WARNING,
            message=f"Configuration file could not be loaded: {e}",
            suggestion=(
                "Check that config/settings.json5 exists and is valid JSON5. "
                "You can reset to defaults from Settings."
            ),
        )


def _check_watch_dirs() -> HealthCheckResult:
    """
    Verify that configured watch directories exist and are readable.

    Returns:
        HealthCheckResult with OK if all dirs exist, WARNING if any are missing.
    """
    try:
        from utils.config_loader import get_config
        watch_paths = get_config("watch_paths", default=[])

        if not watch_paths:
            return HealthCheckResult(
                name="watch_dirs",
                severity=Severity.WARNING,
                message="No watch directories configured.",
                suggestion="Add watch folders in Settings > General.",
            )

        missing = []
        for path in watch_paths:
            expanded = os.path.expanduser(path)
            if not os.path.isdir(expanded):
                missing.append(path)

        if missing:
            return HealthCheckResult(
                name="watch_dirs",
                severity=Severity.WARNING,
                message=f"Watch directories not found: {', '.join(missing)}",
                suggestion=(
                    "Create the missing directories or update the watch paths "
                    "in Settings > General."
                ),
            )

        return HealthCheckResult(
            name="watch_dirs",
            severity=Severity.OK,
            message=f"All {len(watch_paths)} watch directories are accessible.",
        )

    except Exception as e:
        return HealthCheckResult(
            name="watch_dirs",
            severity=Severity.WARNING,
            message=f"Could not check watch directories: {e}",
        )


def _check_log_dir_writable() -> HealthCheckResult:
    """
    Verify that the log directory is writable.

    Returns:
        HealthCheckResult with OK if writable, CRITICAL if not.
    """
    try:
        from utils.log_config import get_log_directory
        log_dir = get_log_directory()

        # Try writing a test file to verify permissions
        test_file = log_dir / ".health_check_test"
        test_file.write_text("test", encoding="utf-8")
        test_file.unlink()                                 # Clean up test file

        return HealthCheckResult(
            name="log_dir_writable",
            severity=Severity.OK,
            message=f"Log directory is writable: {log_dir}",
        )

    except PermissionError:
        return HealthCheckResult(
            name="log_dir_writable",
            severity=Severity.CRITICAL,
            message="Cannot write to the log directory.",
            suggestion=(
                "Check permissions on the log directory. On macOS, "
                "grant Full Disk Access in System Settings > Privacy & Security."
            ),
        )
    except Exception as e:
        return HealthCheckResult(
            name="log_dir_writable",
            severity=Severity.WARNING,
            message=f"Log directory check failed: {e}",
        )


def _check_mediainfo_available() -> HealthCheckResult:
    """
    Verify that the MediaInfo library is available.

    MeedyaManager uses pymediainfo for media analysis. The library
    requires libmediainfo to be installed or bundled with the app.

    Returns:
        HealthCheckResult with OK if available, WARNING if not.
    """
    try:
        from pymediainfo import MediaInfo
        # Try a simple call to verify the library is functional
        MediaInfo.can_parse()
        return HealthCheckResult(
            name="mediainfo_available",
            severity=Severity.OK,
            message="MediaInfo library is available.",
        )
    except Exception as e:
        return HealthCheckResult(
            name="mediainfo_available",
            severity=Severity.WARNING,
            message=f"MediaInfo library not available: {e}",
            suggestion=(
                "Install MediaInfo: brew install mediainfo (macOS), "
                "sudo apt install mediainfo (Linux), or download from "
                "mediaarea.net (Windows)."
            ),
        )


def _check_disk_space() -> HealthCheckResult:
    """
    Verify that there is sufficient disk space for log files.

    Checks the partition containing the log directory. A warning is
    emitted if less than 100 MB is available; critical if less than 10 MB.

    Returns:
        HealthCheckResult with OK, WARNING, or CRITICAL based on available space.
    """
    try:
        from utils.log_config import get_log_directory
        log_dir = get_log_directory()
        usage = shutil.disk_usage(str(log_dir))
        free_mb = usage.free / (1024 * 1024)

        if free_mb < 10:
            return HealthCheckResult(
                name="disk_space",
                severity=Severity.CRITICAL,
                message=f"Very low disk space: {free_mb:.0f} MB free.",
                suggestion="Free up disk space to prevent log write failures.",
            )
        elif free_mb < 100:
            return HealthCheckResult(
                name="disk_space",
                severity=Severity.WARNING,
                message=f"Low disk space: {free_mb:.0f} MB free.",
                suggestion="Consider freeing up disk space.",
            )
        return HealthCheckResult(
            name="disk_space",
            severity=Severity.OK,
            message=f"Disk space OK: {free_mb:.0f} MB free.",
        )

    except Exception as e:
        return HealthCheckResult(
            name="disk_space",
            severity=Severity.WARNING,
            message=f"Could not check disk space: {e}",
        )


# ============================================================================
# Public API
# ============================================================================

def run_startup_checks() -> list[HealthCheckResult]:
    """
    Run all startup health checks and return the results.

    Checks are run in a fixed order. Each check is independent and
    will not fail if a previous check failed.

    Returns:
        list[HealthCheckResult]: Results for all checks, in order.
    """
    checks = [
        _check_python_version,
        _check_config_valid,
        _check_watch_dirs,
        _check_log_dir_writable,
        _check_mediainfo_available,
        _check_disk_space,
    ]

    results = []
    for check_fn in checks:
        try:
            result = check_fn()
            results.append(result)
            # Log non-OK results
            if result.severity != Severity.OK:
                log_fn = logger.warning if result.severity == Severity.WARNING else logger.critical
                log_fn(f"Health check [{result.name}]: {result.message}")
            else:
                logger.debug(f"Health check [{result.name}]: {result.message}")
        except Exception as e:
            # If a check itself crashes, record it as a warning
            # Get the function name safely (mock objects may not have __name__)
            fn_name = getattr(check_fn, "__name__", "unknown").replace("_check_", "")
            results.append(HealthCheckResult(
                name=fn_name,
                severity=Severity.WARNING,
                message=f"Check failed to execute: {e}",
            ))

    return results


def format_results_for_cli(results: list[HealthCheckResult]) -> str:
    """
    Format health check results for CLI display.

    Args:
        results: List of HealthCheckResult objects.

    Returns:
        str: Formatted multi-line string suitable for terminal output.
    """
    lines = ["MeedyaManager Startup Health Check", "=" * 40]

    severity_symbols = {
        Severity.OK: "  OK ",
        Severity.WARNING: " WARN",
        Severity.CRITICAL: " CRIT",
    }

    for result in results:
        symbol = severity_symbols.get(result.severity, " ??? ")
        lines.append(f"[{symbol}] {result.name}: {result.message}")
        if result.suggestion:
            lines.append(f"         -> {result.suggestion}")

    # Summary line
    ok_count = sum(1 for r in results if r.severity == Severity.OK)
    warn_count = sum(1 for r in results if r.severity == Severity.WARNING)
    crit_count = sum(1 for r in results if r.severity == Severity.CRITICAL)
    lines.append("=" * 40)
    lines.append(f"Results: {ok_count} OK, {warn_count} warnings, {crit_count} critical")

    return "\n".join(lines)
