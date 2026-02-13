# ============================================================================
# File: /utils/error_reporter.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Error report preparation and email submission for MeedyaManager.
#
# Collects system information, recent logs, and error details into a
# structured report, then opens the user's default email client with
# a pre-composed message. The user sees and reviews the email before
# sending — no automatic transmission occurs.
#
# PII is redacted from all report content before composing the email.
#
# Usage:
#   from utils.error_reporter import prepare_report, open_email_client
#
#   report = prepare_report(error_summary="Crash during scan")
#   open_email_client(report)
# ============================================================================

import sys                                                 # Python version info
import platform                                            # OS platform details
import logging                                             # Structured logging
import webbrowser                                          # Opens mailto: URLs
from pathlib import Path                                   # Path operations
from datetime import datetime                              # Report timestamps
from urllib.parse import quote                             # URL-safe encoding

logger = logging.getLogger("MeedyaManager.ErrorReporter")

# Support email address — reports are sent here by the user
SUPPORT_EMAIL = "support@mwbmpartners.ltd"


def _get_app_version() -> str:
    """
    Retrieve the MeedyaManager version string.

    Reads the version from the CLI module's version_option decorator.
    Returns 'unknown' if the version cannot be determined.

    Returns:
        str: Version string (e.g., '1.4-M5') or 'unknown'.
    """
    try:
        import importlib
        cli_mod = importlib.import_module("cli")
        return getattr(cli_mod, "__version__", "unknown")
    except Exception:
        return "unknown"


def _get_system_info() -> str:
    """
    Collect system information for the error report.

    Returns a multi-line string with Python version, OS platform,
    architecture, and MeedyaManager version.

    Returns:
        str: Formatted system information block.
    """
    return (
        f"MeedyaManager Version: {_get_app_version()}\n"
        f"Python: {sys.version}\n"
        f"Platform: {platform.platform()}\n"
        f"Architecture: {platform.machine()}\n"
        f"OS: {platform.system()} {platform.release()}"
    )


def _get_recent_logs(max_lines: int = 30) -> str:
    """
    Read the last N lines from the main log file.

    If the log file does not exist or cannot be read, returns a
    placeholder message.

    Args:
        max_lines: Maximum number of log lines to include.

    Returns:
        str: Recent log output, or a placeholder if unavailable.
    """
    try:
        from utils.log_config import get_log_directory
        log_file = get_log_directory() / "meedyamanager.log"
        if log_file.exists():
            lines = log_file.read_text(encoding="utf-8").splitlines()
            return "\n".join(lines[-max_lines:])
    except Exception:
        pass
    return "(log file not available)"


def _redact_pii(text: str) -> str:
    """
    Redact personally identifiable information from report text.

    Replaces OS-specific user home directory paths with generic tokens
    to protect user privacy before including text in an email report.

    Args:
        text: The text to redact.

    Returns:
        str: Text with user paths replaced by <user> tokens.
    """
    import re
    # macOS: /Users/username/...
    text = re.sub(r'/Users/[^/\s]+', '/Users/<user>', text)
    # Linux: /home/username/...
    text = re.sub(r'/home/[^/\s]+', '/home/<user>', text)
    # Windows: C:\Users\username\...
    text = re.sub(r'[A-Z]:\\Users\\[^\\]+', r'C:\\Users\\<user>', text)
    return text


def prepare_report(error_summary: str = "",
                   crash_report_path: Path | None = None,
                   include_logs: bool = True,
                   include_system_info: bool = True) -> dict:
    """
    Prepare a structured error report dictionary.

    Collects system information, recent logs, crash report content,
    and the error summary into a dictionary that can be passed to
    open_email_client() to compose an email.

    All text is PII-redacted before inclusion.

    Args:
        error_summary:     Short description of the error (1-2 sentences).
        crash_report_path: Optional path to a crash report file to include.
        include_logs:      Whether to include recent log lines (default True).
        include_system_info: Whether to include system info (default True).

    Returns:
        dict with keys:
            - subject (str): Email subject line
            - body (str): Email body text
            - timestamp (str): ISO timestamp of report generation
    """
    timestamp = datetime.now().isoformat()

    # Build the email subject
    subject = f"MeedyaManager Bug Report — {error_summary[:80]}" if error_summary else \
              "MeedyaManager Bug Report"

    # Build the email body
    sections = []

    # Header
    sections.append(
        "This bug report was generated by MeedyaManager.\n"
        "Please review the information below and add any additional "
        "context before sending."
    )

    # Error summary
    if error_summary:
        sections.append(f"--- Error Summary ---\n{error_summary}")

    # System information
    if include_system_info:
        sys_info = _redact_pii(_get_system_info())
        sections.append(f"--- System Information ---\n{sys_info}")

    # Crash report content
    if crash_report_path and crash_report_path.exists():
        try:
            content = crash_report_path.read_text(encoding="utf-8")
            content = _redact_pii(content)
            # Truncate very long crash reports to keep email manageable
            if len(content) > 5000:
                content = content[:5000] + "\n\n... (truncated, see full crash report file)"
            sections.append(f"--- Crash Report ---\n{content}")
        except Exception as e:
            sections.append(f"--- Crash Report ---\n(could not read: {e})")

    # Recent log output
    if include_logs:
        log_text = _redact_pii(_get_recent_logs())
        sections.append(f"--- Recent Log Output ---\n{log_text}")

    # Timestamp
    sections.append(f"--- Report Generated ---\n{timestamp}")

    body = "\n\n".join(sections)

    return {
        "subject": subject,
        "body": body,
        "timestamp": timestamp,
    }


def open_email_client(report: dict,
                      recipient: str = SUPPORT_EMAIL) -> bool:
    """
    Open the user's default email client with a pre-composed bug report.

    Uses a mailto: URL to open the system's default email application.
    The user sees the email and can review/edit it before sending.
    No automatic email transmission occurs.

    Args:
        report:    Dictionary from prepare_report() with subject and body keys.
        recipient: Email address to send the report to.

    Returns:
        True if the email client was opened successfully, False otherwise.
    """
    try:
        subject = quote(report.get("subject", "MeedyaManager Bug Report"))
        body = quote(report.get("body", ""))

        # Construct the mailto: URL
        # Note: Some email clients truncate very long mailto: URLs.
        # If the body is extremely long, it may be truncated.
        mailto_url = f"mailto:{recipient}?subject={subject}&body={body}"

        # Open the URL in the default email client
        webbrowser.open(mailto_url)
        logger.info(f"Opened email client for bug report to {recipient}")
        return True

    except Exception as e:
        logger.error(f"Failed to open email client: {e}")
        return False


def display_cli_error(error_summary: str, exception: Exception = None):
    """
    Display a formatted error message in the CLI terminal.

    Uses the error catalog for user-friendly messaging if available,
    otherwise falls back to a simple formatted output.

    Args:
        error_summary: Short description of what went wrong.
        exception:     Optional exception instance for additional context.
    """
    try:
        from utils.error_messages import get_user_friendly_message
        if exception:
            msg = get_user_friendly_message(exception)
            print(f"\n  Error: {msg.headline}")
            print(f"  {msg.explanation}")
            print(f"\n  Suggestion: {msg.suggestion}")
            if exception:
                print(f"\n  Details: {type(exception).__name__}: {exception}")
        else:
            print(f"\n  Error: {error_summary}")
    except Exception:
        # Absolute fallback if the error catalog itself fails
        print(f"\n  Error: {error_summary}")
        if exception:
            print(f"  Details: {exception}")
    print()
