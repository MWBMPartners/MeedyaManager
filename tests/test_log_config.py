# ============================================================================
# File: /tests/test_log_config.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the centralized logging configuration module.
# Verifies platform-aware log directory resolution, PII redaction filter,
# log level resolution, old log cleanup, and handler setup.
# ============================================================================

import os                                              # Path operations
import sys                                             # Platform detection
import logging                                         # Logging framework
import pytest                                          # Test framework
from pathlib import Path                               # Path operations
from datetime import datetime, timedelta               # Date calculations
from unittest.mock import patch, MagicMock             # Mocking utilities

from utils.log_config import (
    PIIRedactionFilter,                                # PII redaction filter
    get_log_directory,                                 # Platform-aware log dir
    cleanup_old_logs,                                  # Old log cleanup
    setup_logging,                                     # Main setup function
    reset_logging,                                     # Test cleanup helper
    APP_NAME,                                          # Application name constant
    LOG_FORMAT,                                        # Log format string
)


# ============================================================================
# Tests: PIIRedactionFilter
# ============================================================================

class TestPIIRedactionFilter:
    """Tests for the PII redaction logging filter."""

    def setup_method(self):
        """Create a fresh filter instance for each test."""
        self.filter = PIIRedactionFilter()

    def test_redacts_macos_user_path(self):
        """Should replace /Users/username with /Users/<user>."""
        record = logging.LogRecord(
            name="test", level=logging.INFO, pathname="", lineno=0,
            msg="/Users/johndoe/Music/song.mp3", args=None, exc_info=None,
        )
        self.filter.filter(record)
        assert "/Users/<user>/Music/song.mp3" in record.msg
        assert "johndoe" not in record.msg

    def test_redacts_linux_user_path(self):
        """Should replace /home/username with /home/<user>."""
        record = logging.LogRecord(
            name="test", level=logging.INFO, pathname="", lineno=0,
            msg="/home/alice/Music/song.mp3", args=None, exc_info=None,
        )
        self.filter.filter(record)
        assert "/home/<user>/Music/song.mp3" in record.msg
        assert "alice" not in record.msg

    def test_redacts_windows_user_path(self):
        """Should replace C:\\Users\\username with C:\\Users\\<user>."""
        record = logging.LogRecord(
            name="test", level=logging.INFO, pathname="", lineno=0,
            msg="C:\\Users\\johndoe\\Music\\song.mp3", args=None, exc_info=None,
        )
        self.filter.filter(record)
        assert "C:\\Users\\<user>" in record.msg
        assert "johndoe" not in record.msg

    def test_preserves_non_user_paths(self):
        """Should not modify paths that don't contain user directories."""
        original = "/var/folders/tmp/file.mp3"
        record = logging.LogRecord(
            name="test", level=logging.INFO, pathname="", lineno=0,
            msg=original, args=None, exc_info=None,
        )
        self.filter.filter(record)
        assert record.msg == original

    def test_redacts_multiple_paths_in_message(self):
        """Should redact all user paths when multiple appear in one message."""
        record = logging.LogRecord(
            name="test", level=logging.INFO, pathname="", lineno=0,
            msg="FROM: /Users/alice/a.mp3 TO: /Users/bob/b.mp3",
            args=None, exc_info=None,
        )
        self.filter.filter(record)
        assert "alice" not in record.msg
        assert "bob" not in record.msg
        assert "/Users/<user>" in record.msg

    def test_always_returns_true(self):
        """Filter should never drop records — only sanitize them."""
        record = logging.LogRecord(
            name="test", level=logging.INFO, pathname="", lineno=0,
            msg="test message", args=None, exc_info=None,
        )
        assert self.filter.filter(record) is True

    def test_handles_non_string_msg(self):
        """Should handle non-string messages gracefully."""
        record = logging.LogRecord(
            name="test", level=logging.INFO, pathname="", lineno=0,
            msg=12345, args=None, exc_info=None,
        )
        # Should not raise — non-string messages are left unchanged
        result = self.filter.filter(record)
        assert result is True

    def test_redacts_formatted_args(self):
        """Should redact PII in messages that use % formatting args."""
        record = logging.LogRecord(
            name="test", level=logging.INFO, pathname="", lineno=0,
            msg="Processing file: %s", args=("/Users/johndoe/song.mp3",),
            exc_info=None,
        )
        self.filter.filter(record)
        assert "johndoe" not in record.msg
        assert record.args is None                     # Args cleared after formatting


# ============================================================================
# Tests: get_log_directory()
# ============================================================================

class TestGetLogDirectory:
    """Tests for platform-aware log directory resolution."""

    def test_returns_path_object(self):
        """Should return a Path object."""
        result = get_log_directory()
        assert isinstance(result, Path)

    def test_directory_exists(self):
        """Should create the directory if it doesn't exist."""
        result = get_log_directory()
        assert result.is_dir()

    def test_macos_path(self):
        """On macOS, should use ~/Library/Logs/MeedyaManager/."""
        if sys.platform == "darwin":
            result = get_log_directory()
            assert "Library/Logs/MeedyaManager" in str(result)

    def test_linux_path(self):
        """On Linux, should use ~/.local/state/MeedyaManager/logs/."""
        with patch.object(sys, "platform", "linux"):
            result = get_log_directory()
            assert "MeedyaManager" in str(result)

    def test_windows_path(self):
        """On Windows, should use %LOCALAPPDATA%/MeedyaManager/logs/."""
        with patch.object(sys, "platform", "win32"), \
             patch.dict(os.environ, {"LOCALAPPDATA": "/tmp/test_appdata"}):
            result = get_log_directory()
            assert "MeedyaManager" in str(result)

    def test_returns_consistent_path(self):
        """Calling get_log_directory() twice should return the same path."""
        result1 = get_log_directory()
        result2 = get_log_directory()
        assert result1 == result2


# ============================================================================
# Tests: cleanup_old_logs()
# ============================================================================

class TestCleanupOldLogs:
    """Tests for old log file cleanup."""

    def test_removes_old_rotated_logs(self, tmp_path):
        """Should remove rotated log files older than max_age_days."""
        # Create an old log file (60 days ago)
        old_log = tmp_path / "meedyamanager.log.2025-01-01"
        old_log.write_text("old log content")
        # Set modification time to 60 days ago
        old_time = (datetime.now() - timedelta(days=60)).timestamp()
        os.utime(old_log, (old_time, old_time))

        # Create a recent log file (1 day ago)
        new_log = tmp_path / "meedyamanager.log.2026-02-11"
        new_log.write_text("new log content")

        cleanup_old_logs(tmp_path, max_age_days=30)

        assert not old_log.exists(), "Old log should have been deleted"
        assert new_log.exists(), "Recent log should not be deleted"

    def test_removes_old_crash_reports(self, tmp_path):
        """Should remove crash report files older than max_age_days."""
        old_crash = tmp_path / "crash_2025-01-01_120000.txt"
        old_crash.write_text("old crash report")
        old_time = (datetime.now() - timedelta(days=60)).timestamp()
        os.utime(old_crash, (old_time, old_time))

        cleanup_old_logs(tmp_path, max_age_days=30)
        assert not old_crash.exists()

    def test_preserves_recent_files(self, tmp_path):
        """Should not delete files within the retention period."""
        recent_log = tmp_path / "meedyamanager.log.2026-02-12"
        recent_log.write_text("recent content")

        cleanup_old_logs(tmp_path, max_age_days=30)
        assert recent_log.exists()

    def test_handles_empty_directory(self, tmp_path):
        """Should not raise on an empty directory."""
        cleanup_old_logs(tmp_path, max_age_days=30)   # Should not raise


# ============================================================================
# Tests: setup_logging()
# ============================================================================

class TestSetupLogging:
    """Tests for the main logging setup function."""

    def setup_method(self):
        """Reset logging state before each test."""
        reset_logging()

    def teardown_method(self):
        """Clean up after each test."""
        reset_logging()

    def test_creates_meedyamanager_logger(self):
        """Should create a logger named 'MeedyaManager'."""
        setup_logging()
        logger = logging.getLogger(APP_NAME)
        assert logger.name == APP_NAME

    def test_adds_handlers(self):
        """Should add at least one handler to the MeedyaManager logger."""
        setup_logging()
        logger = logging.getLogger(APP_NAME)
        assert len(logger.handlers) > 0

    def test_respects_log_level_override(self):
        """Should use the explicit log_level parameter when provided."""
        setup_logging(log_level="DEBUG")
        logger = logging.getLogger(APP_NAME)
        assert logger.level == logging.DEBUG

    def test_default_log_level_is_info(self):
        """Should default to INFO when no override is set."""
        with patch.dict(os.environ, {}, clear=False):
            # Remove the env var if it exists
            os.environ.pop("METAMANCER_LOG_LEVEL", None)
            setup_logging()
            logger = logging.getLogger(APP_NAME)
            assert logger.level == logging.INFO

    def test_env_var_overrides_default(self):
        """METAMANCER_LOG_LEVEL env var should override the default level."""
        with patch.dict(os.environ, {"METAMANCER_LOG_LEVEL": "WARNING"}):
            setup_logging()
            logger = logging.getLogger(APP_NAME)
            assert logger.level == logging.WARNING

    def test_prevents_double_setup(self):
        """Calling setup_logging() twice should not add duplicate handlers."""
        setup_logging()
        logger = logging.getLogger(APP_NAME)
        handler_count = len(logger.handlers)

        setup_logging()                                # Second call should be a no-op
        assert len(logger.handlers) == handler_count

    def test_child_loggers_inherit_config(self):
        """Child loggers should inherit handlers from the MeedyaManager root."""
        setup_logging(log_level="DEBUG")
        child = logging.getLogger("MeedyaManager.TestChild")
        # Child logger should be able to log at DEBUG level since parent is DEBUG
        assert child.getEffectiveLevel() == logging.DEBUG

    def test_bridges_legacy_watcher_logger(self):
        """The legacy 'watcher' logger should be bridged to MeedyaManager."""
        setup_logging()
        legacy = logging.getLogger("watcher")
        root = logging.getLogger(APP_NAME)
        assert legacy.parent == root
        assert len(legacy.handlers) == 0               # Handlers should be cleared

    def test_reset_clears_handlers(self):
        """reset_logging() should remove all handlers and allow re-setup."""
        setup_logging()
        logger = logging.getLogger(APP_NAME)
        assert len(logger.handlers) > 0

        reset_logging()
        assert len(logger.handlers) == 0


# ============================================================================
# Tests: Log level resolution
# ============================================================================

class TestLogLevelResolution:
    """Tests for the log level priority chain."""

    def setup_method(self):
        reset_logging()

    def teardown_method(self):
        reset_logging()

    def test_explicit_override_takes_priority(self):
        """Explicit log_level parameter should override env var and config."""
        with patch.dict(os.environ, {"METAMANCER_LOG_LEVEL": "ERROR"}):
            setup_logging(log_level="DEBUG")
            logger = logging.getLogger(APP_NAME)
            assert logger.level == logging.DEBUG       # Override wins


# ============================================================================
# Tests: Integration with existing loggers
# ============================================================================

class TestLoggingIntegration:
    """Tests verifying centralized logging works with existing modules."""

    def test_watcher_logger_uses_correct_name(self):
        """The watcher module should use MeedyaManager.Watcher logger."""
        from core import watcher
        assert watcher.logger.name == "MeedyaManager.Watcher"

    def test_renamer_logger_uses_correct_name(self):
        """The renamer module should use MeedyaManager.Renamer logger."""
        from core import renamer
        assert renamer.logger.name == "MeedyaManager.Renamer"

    def test_app_logger_uses_correct_name(self):
        """The app module should use MeedyaManager.App logger."""
        import ui.app as app_mod
        assert app_mod.logger.name == "MeedyaManager.App"
