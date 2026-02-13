# ============================================================================
# File: /tests/test_exception_handler.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the global exception handling module.
# Verifies crash report generation, exception hook installation,
# and thread exception handling.
# ============================================================================

import sys                                             # Exception hooks
import threading                                       # Thread exception hook
import logging                                         # Log capture
import pytest                                          # Test framework
from pathlib import Path                               # Path operations
from unittest.mock import patch, MagicMock             # Mocking utilities
from types import SimpleNamespace                      # For threading.ExceptHookArgs mock

from utils.exception_handler import (
    install_exception_hooks,                           # Hook installer
    reset_exception_hooks,                             # Test cleanup
    _write_crash_report,                               # Crash report writer
    _main_thread_excepthook,                           # Main thread hook
    _threading_excepthook,                             # Thread hook
)


# ============================================================================
# Tests: Crash Report Writer
# ============================================================================

class TestCrashReportWriter:
    """Tests for the _write_crash_report function."""

    def test_writes_crash_report_file(self, tmp_path):
        """Should create a crash_*.txt file in the log directory."""
        with patch("utils.log_config.get_log_directory", return_value=tmp_path):
            try:
                raise RuntimeError("Test crash")
            except RuntimeError as e:
                import traceback
                tb_text = traceback.format_exc()
                report_path = _write_crash_report(type(e), e, tb_text)

        assert report_path is not None
        assert report_path.exists()
        assert report_path.name.startswith("crash_")
        assert report_path.suffix == ".txt"

    def test_crash_report_contains_exception_info(self, tmp_path):
        """Should include exception type, message, and traceback in the report."""
        with patch("utils.log_config.get_log_directory", return_value=tmp_path):
            try:
                raise ValueError("Test value error")
            except ValueError as e:
                import traceback
                tb_text = traceback.format_exc()
                report_path = _write_crash_report(type(e), e, tb_text)

        content = report_path.read_text(encoding="utf-8")
        assert "ValueError" in content
        assert "Test value error" in content
        assert "Traceback" in content

    def test_crash_report_contains_system_info(self, tmp_path):
        """Should include Python version and platform in the report."""
        with patch("utils.log_config.get_log_directory", return_value=tmp_path):
            try:
                raise RuntimeError("test")
            except RuntimeError as e:
                import traceback
                tb_text = traceback.format_exc()
                report_path = _write_crash_report(type(e), e, tb_text)

        content = report_path.read_text(encoding="utf-8")
        assert "Python:" in content
        assert "Platform:" in content
        assert "Architecture:" in content

    def test_crash_report_includes_log_tail(self, tmp_path):
        """Should include recent log lines if the log file exists."""
        # Create a fake log file
        log_file = tmp_path / "meedyamanager.log"
        log_file.write_text("Line 1\nLine 2\nLine 3\n")

        with patch("utils.log_config.get_log_directory", return_value=tmp_path):
            try:
                raise RuntimeError("test")
            except RuntimeError as e:
                import traceback
                tb_text = traceback.format_exc()
                report_path = _write_crash_report(type(e), e, tb_text)

        content = report_path.read_text(encoding="utf-8")
        assert "Line 1" in content
        assert "Recent Log Output" in content

    def test_returns_none_on_unwritable_directory(self):
        """Should return None when the crash report cannot be written."""
        # Patch to return a non-existent, non-creatable directory
        with patch("utils.log_config.get_log_directory",
                   return_value=Path("/nonexistent/path/that/does/not/exist")):
            try:
                raise RuntimeError("test")
            except RuntimeError as e:
                import traceback
                tb_text = traceback.format_exc()
                result = _write_crash_report(type(e), e, tb_text)
                assert result is None


# ============================================================================
# Tests: Main Thread Exception Hook
# ============================================================================

class TestMainThreadExcepthook:
    """Tests for the main thread exception handler."""

    def test_logs_unhandled_exception(self, caplog, tmp_path):
        """Should log the exception through the centralized logging system."""
        with patch("utils.log_config.get_log_directory", return_value=tmp_path), \
             patch("utils.exception_handler._show_crash_dialog_if_gui"):
            with caplog.at_level(logging.CRITICAL, logger="MeedyaManager.CrashHandler"):
                try:
                    raise RuntimeError("Test unhandled exception")
                except RuntimeError:
                    _main_thread_excepthook(*sys.exc_info())

            assert "Unhandled exception" in caplog.text
            assert "RuntimeError" in caplog.text

    def test_passes_through_keyboard_interrupt(self):
        """KeyboardInterrupt should be passed to the default handler."""
        with patch.object(sys, "__excepthook__") as mock_default:
            _main_thread_excepthook(KeyboardInterrupt, KeyboardInterrupt(), None)
            mock_default.assert_called_once()

    def test_writes_crash_report(self, tmp_path):
        """Should write a crash report file for unhandled exceptions."""
        with patch("utils.log_config.get_log_directory", return_value=tmp_path), \
             patch("utils.exception_handler._show_crash_dialog_if_gui"):
            try:
                raise RuntimeError("Test crash report")
            except RuntimeError:
                _main_thread_excepthook(*sys.exc_info())

        # Check that a crash report was created
        crash_files = list(tmp_path.glob("crash_*.txt"))
        assert len(crash_files) >= 1


# ============================================================================
# Tests: Threading Exception Hook
# ============================================================================

class TestThreadingExcepthook:
    """Tests for the Python threading exception handler."""

    def test_logs_thread_exception(self, caplog, tmp_path):
        """Should log the exception with the thread name."""
        mock_thread = MagicMock()
        mock_thread.name = "TestDaemonThread"

        # Use SimpleNamespace to simulate ExceptHookArgs (struct sequence
        # in Python 3.14 doesn't support keyword construction)
        args = SimpleNamespace(
            exc_type=RuntimeError,
            exc_value=RuntimeError("Thread crashed"),
            exc_traceback=None,
            thread=mock_thread,
        )

        with patch("utils.log_config.get_log_directory", return_value=tmp_path), \
             caplog.at_level(logging.CRITICAL, logger="MeedyaManager.CrashHandler"):
            _threading_excepthook(args)

        assert "TestDaemonThread" in caplog.text
        assert "Thread crashed" in caplog.text

    def test_ignores_keyboard_interrupt_in_thread(self):
        """KeyboardInterrupt in threads should be silently ignored."""
        args = SimpleNamespace(
            exc_type=KeyboardInterrupt,
            exc_value=KeyboardInterrupt(),
            exc_traceback=None,
            thread=None,
        )
        # Should not raise or log anything
        _threading_excepthook(args)


# ============================================================================
# Tests: Hook Installation
# ============================================================================

class TestInstallExceptionHooks:
    """Tests for the install_exception_hooks function."""

    def setup_method(self):
        """Reset hooks before each test."""
        reset_exception_hooks()

    def teardown_method(self):
        """Restore default hooks after each test."""
        reset_exception_hooks()

    def test_installs_sys_excepthook(self):
        """Should replace sys.excepthook with the custom handler."""
        install_exception_hooks()
        assert sys.excepthook == _main_thread_excepthook

    def test_installs_threading_excepthook(self):
        """Should replace threading.excepthook with the custom handler."""
        install_exception_hooks()
        assert threading.excepthook == _threading_excepthook

    def test_prevents_double_installation(self):
        """Calling install_exception_hooks() twice should be safe."""
        install_exception_hooks()
        # Second call should be a no-op (no error, no changes)
        install_exception_hooks()
        assert sys.excepthook == _main_thread_excepthook

    def test_reset_restores_defaults(self):
        """reset_exception_hooks() should restore Python's default handlers."""
        install_exception_hooks()
        reset_exception_hooks()
        assert sys.excepthook == sys.__excepthook__
        assert threading.excepthook == threading.__excepthook__
