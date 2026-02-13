# ============================================================================
# File: /utils/exception_handler.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Global exception handling for MeedyaManager.
#
# Installs exception hooks for all three execution contexts:
#   1. Main thread — sys.excepthook for unhandled exceptions
#   2. Python threads — threading.excepthook for daemon/worker threads
#   3. QThreads — SafeWorker base class (in ui/workers.py)
#
# When an unhandled exception is caught:
#   - The full traceback is logged to the centralized log
#   - A crash report file is written to the log directory
#   - In GUI mode, a user-friendly error dialog is shown
#   - KeyboardInterrupt is always passed through (not caught)
#
# Usage:
#   from utils.exception_handler import install_exception_hooks
#   install_exception_hooks()  # Call once at app startup, after setup_logging()
# ============================================================================

import sys                                             # Exception hook installation
import threading                                       # Thread exception hook
import traceback                                       # Traceback formatting
import logging                                         # Structured logging
from datetime import datetime                          # Crash report timestamps
from pathlib import Path                               # File path operations

logger = logging.getLogger("MeedyaManager.CrashHandler")


# ============================================================================
# Crash Report Writer
# ============================================================================

def _write_crash_report(exc_type, exc_value, tb_text: str) -> Path | None:
    """
    Write a timestamped crash report to the log directory.

    The crash report contains:
      - Timestamp of the crash
      - Python version and platform info
      - MeedyaManager version (if available)
      - Exception type and message
      - Full traceback
      - Last 50 lines of the main log file (for context)

    The report is saved as crash_YYYY-MM-DD_HHMMSS.txt in the log directory.

    Args:
        exc_type: The exception class (e.g., RuntimeError).
        exc_value: The exception instance.
        tb_text: Pre-formatted traceback string.

    Returns:
        Path to the crash report file, or None if writing failed.
    """
    try:
        from utils.log_config import get_log_directory  # Lazy import to avoid circular deps
        log_dir = get_log_directory()
    except Exception:
        # If we can't even get the log directory, fall back to current directory
        log_dir = Path(".")

    timestamp = datetime.now().strftime("%Y-%m-%d_%H%M%S")
    report_path = log_dir / f"crash_{timestamp}.txt"

    try:
        # Gather system information
        import platform                                # Platform details
        version_str = "unknown"
        try:
            # Try to read the Click version from the CLI module
            import importlib
            cli_mod = importlib.import_module("cli")
            # The version is in the version_option decorator, extract from source
            version_str = getattr(cli_mod, "__version__", "unknown")
        except Exception:
            pass

        # Read the last 50 lines of the main log for context
        log_tail = ""
        main_log = log_dir / "meedyamanager.log"
        if main_log.exists():
            try:
                lines = main_log.read_text(encoding="utf-8").splitlines()
                log_tail = "\n".join(lines[-50:])
            except Exception:
                log_tail = "(could not read log file)"

        # Write the crash report
        report_content = (
            f"{'=' * 72}\n"
            f"MeedyaManager Crash Report\n"
            f"{'=' * 72}\n"
            f"\n"
            f"Timestamp:     {datetime.now().isoformat()}\n"
            f"Version:       {version_str}\n"
            f"Python:        {sys.version}\n"
            f"Platform:      {platform.platform()}\n"
            f"Architecture:  {platform.machine()}\n"
            f"\n"
            f"{'=' * 72}\n"
            f"Exception\n"
            f"{'=' * 72}\n"
            f"\n"
            f"Type:    {exc_type.__name__ if exc_type else 'Unknown'}\n"
            f"Message: {exc_value}\n"
            f"\n"
            f"{'=' * 72}\n"
            f"Traceback\n"
            f"{'=' * 72}\n"
            f"\n"
            f"{tb_text}\n"
            f"\n"
            f"{'=' * 72}\n"
            f"Recent Log Output (last 50 lines)\n"
            f"{'=' * 72}\n"
            f"\n"
            f"{log_tail}\n"
        )

        report_path.write_text(report_content, encoding="utf-8")
        return report_path

    except Exception as write_err:
        # Last resort: print to stderr if we can't even write a crash report
        print(
            f"CRITICAL: Could not write crash report to {report_path}: {write_err}",
            file=sys.stderr,
        )
        return None


# ============================================================================
# GUI Crash Dialog
# ============================================================================

def _show_crash_dialog_if_gui(exc_type, exc_value, tb_text: str, report_path=None):
    """
    If the Qt application is running, show a crash dialog to the user.

    This function checks whether a QApplication instance exists and is
    still running. If so, it shows a QMessageBox with a user-friendly
    error message and the option to view technical details.

    If the GUI is not running (CLI mode), this is a no-op.

    Args:
        exc_type: The exception class.
        exc_value: The exception instance.
        tb_text: Pre-formatted traceback string.
        report_path: Optional path to the crash report file.
    """
    try:
        from PySide6.QtWidgets import QApplication, QMessageBox
        app = QApplication.instance()
        if app is None:
            return                                     # No GUI running

        # Build user-friendly message
        headline = f"An unexpected error occurred: {exc_type.__name__}"
        detail = str(exc_value) if exc_value else "No additional details available."

        report_note = ""
        if report_path:
            report_note = f"\n\nA crash report has been saved to:\n{report_path}"

        msg = QMessageBox()
        msg.setIcon(QMessageBox.Critical)
        msg.setWindowTitle("MeedyaManager — Unexpected Error")
        msg.setText(headline)
        msg.setInformativeText(
            f"{detail}\n\n"
            f"MeedyaManager encountered an unexpected error. "
            f"You can continue working, but some features may not "
            f"function correctly until the application is restarted."
            f"{report_note}"
        )
        msg.setDetailedText(tb_text)
        msg.setStandardButtons(QMessageBox.Ok)
        msg.exec()

    except Exception:
        pass                                           # Don't crash the crash handler


# ============================================================================
# Exception Hooks
# ============================================================================

def _main_thread_excepthook(exc_type, exc_value, exc_tb):
    """
    Global exception handler for unhandled exceptions in the main thread.

    Installed as sys.excepthook. Catches all unhandled exceptions (except
    KeyboardInterrupt), logs the full traceback, writes a crash report,
    and shows a GUI error dialog if the application is running.

    Args:
        exc_type: The exception class.
        exc_value: The exception instance.
        exc_tb: The traceback object.
    """
    # Always pass through KeyboardInterrupt (Ctrl+C) to the default handler
    if issubclass(exc_type, KeyboardInterrupt):
        sys.__excepthook__(exc_type, exc_value, exc_tb)
        return

    # Format the traceback for logging and reporting
    tb_lines = traceback.format_exception(exc_type, exc_value, exc_tb)
    tb_text = "".join(tb_lines)

    # Log the exception through the centralized logging system
    logger.critical(f"Unhandled exception in main thread:\n{tb_text}")

    # Write a crash report file to the log directory
    report_path = _write_crash_report(exc_type, exc_value, tb_text)

    # Show a user-friendly error dialog if the GUI is running
    _show_crash_dialog_if_gui(exc_type, exc_value, tb_text, report_path)


def _threading_excepthook(args):
    """
    Global exception handler for unhandled exceptions in Python threads.

    Installed as threading.excepthook. Handles crashes in daemon threads
    (e.g., the queue_worker thread in watcher.py) and any other non-QThread
    background threads.

    Args:
        args: A threading.ExceptHookArgs namedtuple containing:
            - exc_type: The exception class
            - exc_value: The exception instance
            - exc_traceback: The traceback object
            - thread: The Thread object that raised the exception
    """
    # Always pass through KeyboardInterrupt
    if issubclass(args.exc_type, KeyboardInterrupt):
        return

    # Format the traceback
    tb_lines = traceback.format_exception(
        args.exc_type, args.exc_value, args.exc_traceback,
    )
    tb_text = "".join(tb_lines)

    # Include the thread name for diagnosis
    thread_name = args.thread.name if args.thread else "unknown"
    logger.critical(
        f"Unhandled exception in thread '{thread_name}':\n{tb_text}"
    )

    # Write a crash report
    _write_crash_report(args.exc_type, args.exc_value, tb_text)


# ============================================================================
# Installation
# ============================================================================

# Track whether hooks have been installed to prevent double installation
_hooks_installed = False


def install_exception_hooks():
    """
    Install global exception hooks for all execution contexts.

    Should be called once at application startup, AFTER setup_logging()
    has been called (so that crash logs go to the right place).

    Installs:
      - sys.excepthook — catches unhandled exceptions in the main thread
      - threading.excepthook — catches exceptions in Python threads

    QThread exception handling is provided by the SafeWorker base class
    in ui/workers.py (not installed here since it's a Qt-specific pattern).
    """
    global _hooks_installed
    if _hooks_installed:
        return                                         # Prevent double installation
    _hooks_installed = True

    # Install main thread exception hook
    sys.excepthook = _main_thread_excepthook

    # Install threading exception hook
    threading.excepthook = _threading_excepthook

    logger.debug("Global exception hooks installed")


def reset_exception_hooks():
    """
    Reset exception hooks to Python defaults.
    Used primarily in tests to ensure clean state between test runs.
    """
    global _hooks_installed
    _hooks_installed = False
    sys.excepthook = sys.__excepthook__
    threading.excepthook = threading.__excepthook__
