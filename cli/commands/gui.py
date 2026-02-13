# ============================================================================
# File: /cli/commands/gui.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# CLI command to launch the MeedyaManager GUI.
# Uses lazy import of PySide6 to avoid import errors when the GUI
# dependencies are not installed (e.g. on headless servers).
# ============================================================================

import sys                                                  # System exit codes
import click                                                # CLI framework


@click.command()
def gui():
    """Launch the MeedyaManager graphical interface."""
    try:
        # Lazy import — only load PySide6 when the gui command is invoked
        from ui.app import launch_gui
    except ImportError as e:
        click.echo("Error: PySide6 is required for the GUI but is not installed.", err=True)
        click.echo("Install it with:  pip install PySide6 darkdetect", err=True)
        click.echo(f"\nDetails: {e}", err=True)
        sys.exit(1)

    # Launch the GUI application — blocks until the window is closed
    exit_code = launch_gui()
    sys.exit(exit_code)
