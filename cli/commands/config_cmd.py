# ============================================================================
# File: /cli/commands/config_cmd.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Click commands for configuration export and import.
# Provides `meedyamanager config export` and `meedyamanager config import`
# subcommands for portable settings migration between platforms.
#
# Usage:
#   meedyamanager config export --out ~/backup.mmprofile [--name "Home Mac"]
#   meedyamanager config import ~/backup.mmprofile [--mode replace|merge] [--dry-run]
# ============================================================================

import click                                               # CLI framework
from pathlib import Path                                   # Path operations
from rich.console import Console                           # Rich terminal output
from rich.table import Table                               # Formatted tables
from rich.panel import Panel                               # Boxed info panels

# Shared console instance for rich output
console = Console()


@click.group(name="config")
def config():
    """Manage MeedyaManager configuration profiles.

    Export and import settings for migration between platforms.
    """
    pass


@config.command()
@click.option(
    "--out", "-o",
    type=click.Path(path_type=Path),
    required=True,
    help="Output path for the .mmprofile file.",
)
@click.option(
    "--name", "-n",
    type=str,
    default="",
    help="Human-readable profile name (e.g., 'Home Mac').",
)
@click.option(
    "--include-secrets",
    is_flag=True,
    default=False,
    help="Include actual API key values from .env (CAUTION: sensitive data).",
)
def export(out, name, include_secrets):
    """Export current settings as a portable .mmprofile bundle.

    Creates a ZIP archive containing your settings with platform-specific
    paths converted to portable tokens. Can be imported on any platform.
    """
    from utils.config_profile import export_profile

    if include_secrets:
        console.print(
            "[bold yellow]WARNING:[/bold yellow] Including API key values in the profile. "
            "Treat this file as confidential."
        )
        if not click.confirm("Continue?"):
            raise SystemExit(0)

    try:
        result_path = export_profile(
            output_path=out,
            profile_name=name,
            include_secrets=include_secrets,
        )
        console.print(f"\n[green]Profile exported successfully:[/green] {result_path}")

    except Exception as e:
        console.print(f"\n[red]Export failed:[/red] {e}")
        raise SystemExit(1)


@config.command(name="import")
@click.argument(
    "profile_path",
    type=click.Path(exists=True, path_type=Path),
)
@click.option(
    "--mode", "-m",
    type=click.Choice(["replace", "merge"], case_sensitive=False),
    default="replace",
    help="Import mode: 'replace' (full replacement) or 'merge' (additive).",
)
@click.option(
    "--dry-run",
    is_flag=True,
    default=False,
    help="Preview changes without applying them.",
)
@click.option(
    "--yes", "-y",
    is_flag=True,
    default=False,
    help="Skip confirmation prompt.",
)
def import_cmd(profile_path, mode, dry_run, yes):
    """Import a .mmprofile bundle into the current configuration.

    Reads the profile, adapts paths for the current platform, and either
    replaces or merges the settings. Use --dry-run to preview changes first.
    """
    from utils.config_profile import import_profile, validate_profile

    # Validate first
    errors = validate_profile(profile_path)
    if errors:
        console.print("[red]Profile validation failed:[/red]")
        for error in errors:
            console.print(f"  - {error}")
        raise SystemExit(1)

    # Preview changes
    try:
        result = import_profile(profile_path, mode=mode, dry_run=True)
    except Exception as e:
        console.print(f"\n[red]Import preview failed:[/red] {e}")
        raise SystemExit(1)

    # Display change summary
    changes = result["changes"]
    profile_name = result["profile_name"]

    console.print(f"\n[bold]Profile:[/bold] {profile_name}")
    console.print(f"[bold]Mode:[/bold] {mode}")
    console.print(f"[bold]Changes:[/bold] {len(changes)}")

    if changes:
        table = Table(title="Configuration Changes")
        table.add_column("Setting", style="cyan")
        table.add_column("Current", style="red")
        table.add_column("New", style="green")

        for key, diff in sorted(changes.items()):
            old_str = str(diff["old"])[:50] if diff["old"] is not None else "(not set)"
            new_str = str(diff["new"])[:50] if diff["new"] is not None else "(removed)"
            table.add_row(key, old_str, new_str)

        console.print(table)
    else:
        console.print("\n[dim]No changes detected — profile matches current settings.[/dim]")

    if dry_run:
        console.print("\n[dim](Dry run — no changes applied)[/dim]")
        return

    if not changes:
        return

    # Confirm before applying
    if not yes and not click.confirm("\nApply these changes?"):
        console.print("[dim]Import cancelled.[/dim]")
        return

    # Apply the changes
    try:
        result = import_profile(profile_path, mode=mode, dry_run=False)
        console.print(f"\n[green]Profile '{profile_name}' imported successfully.[/green]")
    except Exception as e:
        console.print(f"\n[red]Import failed:[/red] {e}")
        raise SystemExit(1)
