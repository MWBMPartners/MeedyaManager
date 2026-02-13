# ============================================================================
# File: /cli/__init__.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Click-based CLI entry point for MeedyaManager.
# Defines the top-level command group and registers all subcommands.
# Usage: python -m cli [OPTIONS] COMMAND [ARGS]
# ============================================================================

import click                                        # CLI framework for commands and arguments


@click.group()
@click.version_option(
    version="1.1-M2",                              # Current milestone version
    prog_name="MeedyaManager",                       # Application name shown in --version
    message="%(prog)s v%(version)s"                # Output format: "MeedyaManager v1.1-M2"
)
def cli():
    """MeedyaManager — Cross-platform media file manager and auto-organizer.

    Scan, classify, and organize media files using metadata-driven rules.
    """
    pass


# Import and register subcommands after cli group is defined
# (avoids circular imports since commands reference the cli group)
from cli.commands.scan import scan       # noqa: E402  # Batch scan and rename preview
from cli.commands.debug import debug     # noqa: E402  # Single-file metadata inspector
from cli.commands.watch import watch     # noqa: E402  # Real-time folder monitoring
from cli.commands.rule import rule       # noqa: E402  # Rule template testing
from cli.commands.gui import gui         # noqa: E402  # Graphical interface launcher

cli.add_command(scan)
cli.add_command(debug)
cli.add_command(watch)
cli.add_command(rule)
cli.add_command(gui)
