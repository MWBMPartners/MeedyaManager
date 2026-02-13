# ============================================================================
# File: /cli/commands/scan.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Click command for batch scanning watch folders. Extracts metadata from
# all matching media files, runs rename simulation, and displays results
# in a rich formatted table. Supports JSON export of metadata.
# Replaces the legacy cli/runner.py main() function.
# ============================================================================

import os                                              # File path operations
import json                                            # JSON metadata export
import click                                           # CLI framework
from rich.console import Console                       # Rich terminal output
from rich.table import Table                           # Formatted result tables
from rich.panel import Panel                           # Boxed info panels

from utils.config_loader import get_config             # Safe config access
from core.metadata_extractor import extract_metadata   # Metadata + classification pipeline
from core.classify_media import classify_media         # Media classification
from core.renamer import simulate_rename               # Dry-run rename path computation


# Shared console instance for rich output
console = Console()


@click.command()
@click.option(
    "--json", "export_json",                           # Renamed to avoid shadowing json module
    is_flag=True,
    help="Export extracted metadata as JSON files."
)
@click.option(
    "--out",
    type=click.Path(),
    default=None,
    help="Output folder for JSON export."
)
@click.option(
    "--mkdir",
    is_flag=True,
    help="Create the output folder if it does not exist."
)
@click.option(
    "--simulate-off",
    is_flag=True,
    help="Disable rename simulation for this run."
)
@click.option(
    "--path", "-p",
    type=click.Path(exists=True),
    multiple=True,                                     # Allow multiple --path flags
    help="Override watch paths (can be specified multiple times)."
)
def scan(export_json, out, mkdir, simulate_off, path):
    """Scan watch folders and preview rename simulations.

    Walks all configured watch folders (or paths specified with --path),
    extracts metadata from matching media files, classifies them, and
    displays a rename preview table.
    """
    # Load environment variables for API keys
    from utils.env_loader import load_env_variables
    load_env_variables()

    # Use --path overrides if provided, otherwise load from config
    watch_paths = list(path) if path else get_config("watch_paths", default=[])
    extensions = set(get_config("valid_extensions", default=[]))

    # Handle output directory creation for JSON export
    if out:
        if not os.path.exists(out):
            if mkdir:
                try:
                    os.makedirs(out)
                    console.print(f"[green]📁 Created output folder:[/green] {out}")
                except Exception as e:
                    console.print(f"[red]❌ Could not create output folder:[/red] {e}")
                    raise SystemExit(1)
            else:
                console.print(f"[red]❌ Output folder does not exist:[/red] {out}")
                console.print("  Use --mkdir to create it automatically.")
                raise SystemExit(1)

    # Build the results table for displaying scan output
    table = Table(
        title="🔍 MeedyaManager Scan Results",
        show_header=True,
        header_style="bold cyan",
        show_lines=True,
    )
    table.add_column("File", style="white", no_wrap=True, max_width=40)
    table.add_column("Type", style="magenta")
    table.add_column("Format", style="blue")
    table.add_column("Quality", style="green")
    table.add_column("Proposed Path", style="yellow", max_width=50)

    # Track scan statistics
    files_scanned = 0
    files_matched = 0

    # Walk each watch folder and process matching media files
    for folder in watch_paths:
        expanded = os.path.expanduser(folder)          # Expand ~ in paths
        if not os.path.isdir(expanded):
            console.print(f"[yellow]⚠️ Folder not found:[/yellow] {folder}")
            continue

        for root, _, files in os.walk(expanded):
            for file in files:
                full_path = os.path.join(root, file)
                ext = os.path.splitext(file)[1].lower()
                files_scanned += 1

                if ext in extensions and os.path.isfile(full_path):
                    files_matched += 1

                    # Extract metadata (includes classification)
                    metadata = extract_metadata(full_path)
                    classified = classify_media(metadata)
                    metadata.update(classified)

                    # Compute proposed rename path (unless simulation disabled)
                    proposed = ""
                    if not simulate_off:
                        result = simulate_rename(full_path, metadata)
                        proposed = result if result else "[dim]N/A[/dim]"

                    # Add row to results table
                    table.add_row(
                        os.path.basename(full_path),
                        metadata.get("media_class", "Unknown"),
                        metadata.get("format_class", "Unknown"),
                        metadata.get("quality_type", "Unknown"),
                        str(proposed),
                    )

                    # Export metadata to JSON if requested
                    if export_json:
                        _export_metadata_json(full_path, metadata, out)

    # Display results
    if files_matched > 0:
        console.print(table)
    else:
        console.print("[yellow]No matching media files found in watch folders.[/yellow]")

    # Print summary
    console.print(
        Panel(
            f"📊 Scanned [bold]{files_scanned}[/bold] files, "
            f"matched [bold]{files_matched}[/bold] media files"
            + (" (simulation disabled)" if simulate_off else ""),
            title="Summary",
            style="dim",
        )
    )


def _export_metadata_json(filepath, metadata, output_dir=None):
    """
    Export metadata for a single file as a JSON file.

    Args:
        filepath (str): Original media file path
        metadata (dict): Extracted metadata dictionary
        output_dir (str): Optional output directory (defaults to file's directory)
    """
    base = os.path.basename(filepath)                  # Get filename from path
    name, _ = os.path.splitext(base)                   # Strip extension
    json_filename = f"{name}.metadata.json"            # Build JSON output filename

    if output_dir:
        out_path = os.path.join(output_dir, json_filename)
    else:
        out_path = os.path.join(os.path.dirname(filepath), json_filename)

    try:
        with open(out_path, 'w', encoding='utf-8') as f:
            json.dump(metadata, f, indent=4)
        console.print(f"  [green]📄 Exported:[/green] {out_path}")
    except Exception as e:
        console.print(f"  [red]❌ Export failed:[/red] {e}")
