# ============================================================================
# File: /cli/commands/report_bug.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Click command for submitting bug reports via the user's email client.
# Collects system information, recent logs, and optional crash report
# content, then opens a pre-composed email for the user to review and send.
#
# Usage:
#   meedyamanager report-bug [--include-logs] [--no-system-info]
# ============================================================================

import click                                               # CLI framework
from pathlib import Path                                   # Path operations
from rich.console import Console                           # Rich terminal output
from rich.panel import Panel                               # Boxed info panels

from utils.error_reporter import (
    prepare_report,                                        # Report builder
    open_email_client,                                     # Email launcher
)
from utils.log_config import get_log_directory             # Log directory resolution

# Shared console instance for rich output
console = Console()


@click.command(name="report-bug")
@click.option(
    "--include-logs/--no-logs",
    default=True,
    help="Include recent log lines in the report (default: yes).",
)
@click.option(
    "--no-system-info",
    is_flag=True,
    default=False,
    help="Exclude system information (Python version, OS, etc.) from the report.",
)
@click.option(
    "--crash-report",
    type=click.Path(exists=True, path_type=Path),
    default=None,
    help="Path to a specific crash report file to include.",
)
@click.option(
    "--summary",
    type=str,
    default="",
    help="Short description of the bug or error.",
)
def report_bug(include_logs, no_system_info, crash_report, summary):
    """Submit a bug report via your default email client.

    Collects system information and recent log output, composes an email,
    and opens your email client for review before sending. No data is
    transmitted automatically — you see the email before it is sent.
    """
    # If no summary provided, prompt interactively
    if not summary:
        summary = click.prompt(
            "Please describe the bug briefly",
            default="General bug report",
        )

    # If no crash report specified, check for the most recent one
    if crash_report is None:
        log_dir = get_log_directory()
        crash_files = sorted(log_dir.glob("crash_*.txt"), reverse=True)
        if crash_files:
            latest = crash_files[0]
            if click.confirm(f"Include most recent crash report ({latest.name})?", default=True):
                crash_report = latest

    # Show what will be included in the report
    console.print()
    console.print(Panel(
        f"[bold]Bug Report Summary[/bold]\n\n"
        f"Description: {summary}\n"
        f"System info: {'Yes' if not no_system_info else 'No'}\n"
        f"Recent logs: {'Yes' if include_logs else 'No'}\n"
        f"Crash report: {crash_report.name if crash_report else 'None'}",
        title="MeedyaManager Bug Report",
        border_style="blue",
    ))

    # Prepare the report
    report = prepare_report(
        error_summary=summary,
        crash_report_path=crash_report,
        include_logs=include_logs,
        include_system_info=not no_system_info,
    )

    # Try to open the email client
    console.print()
    console.print("Opening your email client...")

    success = open_email_client(report)

    if success:
        console.print(
            "[green]Email client opened.[/green] "
            "Please review the report and click Send when ready."
        )
    else:
        # Fallback: print the report to the console
        console.print(
            "[yellow]Could not open email client.[/yellow] "
            "The report has been printed below — you can copy and "
            "email it manually."
        )
        console.print()
        console.print(Panel(
            report["body"],
            title=report["subject"],
            border_style="red",
        ))

    # Show the log directory for reference
    log_dir = get_log_directory()
    console.print(f"\nLog files: {log_dir}")
