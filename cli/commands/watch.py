# ============================================================================
# File: /cli/commands/watch.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Click command for starting real-time folder monitoring.
# Watches configured directories for new media files and processes them
# through the metadata extraction and rename simulation pipeline.
# Supports both watchdog and polling modes with clean shutdown.
# ============================================================================

import os                                              # File path operations
import sys                                             # System exit handling
import click                                           # CLI framework
from rich.console import Console                       # Rich terminal output
from rich.panel import Panel                           # Boxed info panels

from utils.config_loader import get_config             # Safe config access


# Shared console instance for rich output
console = Console()


@click.command()
@click.option(
    "--mode",
    type=click.Choice(["watchdog", "polling"], case_sensitive=False),
    default="watchdog",
    help="File detection mode: watchdog (real-time) or polling (interval-based)."
)
@click.option(
    "--simulate/--no-simulate",
    default=True,
    help="Enable or disable rename simulation during monitoring."
)
@click.option(
    "--path", "-p",
    type=click.Path(exists=True),
    multiple=True,                                     # Allow multiple --path flags
    help="Override watch paths (can be specified multiple times)."
)
def watch(mode, simulate, path):
    """Start watching folders for new media files.

    Monitors configured directories in real-time and processes new
    media files through the metadata extraction and rename simulation
    pipeline. Press Ctrl+C to stop.
    """
    # Load environment variables for API keys
    from utils.env_loader import load_env_variables
    load_env_variables()

    # Import watcher module (deferred to avoid module-level side effects)
    from core import watcher

    # Apply CLI overrides to watcher module-level config
    if path:
        watcher.watch_paths = list(path)               # Override configured watch paths
    watcher.watch_mode = mode                          # Set detection mode
    watcher.simulate_enabled = simulate                # Enable/disable simulation

    # Display startup information
    watch_paths = watcher.watch_paths
    console.print(
        Panel(
            f"[bold]Mode:[/bold] {mode}\n"
            f"[bold]Simulation:[/bold] {'enabled' if simulate else 'disabled'}\n"
            f"[bold]Watching:[/bold]\n"
            + "\n".join(f"  📂 {p}" for p in watch_paths),
            title="👁️ MeedyaManager Watcher",
            style="cyan",
        )
    )
    console.print("[dim]Press Ctrl+C to stop watching.[/dim]\n")

    # Start the watcher (blocks until interrupted)
    try:
        watcher.start_watcher()
    except KeyboardInterrupt:
        console.print("\n[yellow]🛑 Watcher stopped by user.[/yellow]")
        sys.exit(0)
