# ============================================================================
# File: /cli/commands/debug.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Click command for inspecting metadata of a single media file.
# Displays all extracted metadata in a rich formatted panel.
# Supports JSON export. Replaces the legacy cli/metadata_debugger.py.
# ============================================================================

import os                                              # File path operations
import json                                            # JSON metadata export
import click                                           # CLI framework
from rich.console import Console                       # Rich terminal output
from rich.table import Table                           # Formatted metadata display
from rich.panel import Panel                           # Boxed info panels

from core.metadata_extractor import extract_metadata   # Metadata + classification pipeline
from core.classify_media import classify_media         # Media classification


# Shared console instance for rich output
console = Console()


@click.command()
@click.argument(
    "filepath",
    type=click.Path(exists=True),                      # File must exist
)
@click.option(
    "--json", "export_json",
    is_flag=True,
    help="Export metadata as a JSON file."
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
def debug(filepath, export_json, out, mkdir):
    """Inspect metadata for a single media file.

    Extracts and displays all available metadata including format,
    duration, classification, and quality information.
    """
    # Load environment variables for API keys
    from utils.env_loader import load_env_variables
    load_env_variables()

    # Verify the path is a file, not a directory
    if not os.path.isfile(filepath):
        console.print(f"[red]❌ Not a regular file:[/red] {filepath}")
        raise SystemExit(1)

    # Handle output directory creation for JSON export
    if out and not os.path.exists(out):
        if mkdir:
            try:
                os.makedirs(out)
                console.print(f"[green]📁 Created output folder:[/green] {out}")
            except Exception as e:
                console.print(f"[red]❌ Failed to create output folder:[/red] {e}")
                raise SystemExit(1)
        else:
            console.print(f"[red]❌ Output folder does not exist:[/red] {out}")
            console.print("  Use --mkdir to create it automatically.")
            raise SystemExit(1)

    # Extract metadata and classification
    metadata = extract_metadata(filepath)
    classified = classify_media(metadata)
    metadata.update(classified)

    # Build metadata display table
    table = Table(
        show_header=True,
        header_style="bold cyan",
        title=f"🔍 Metadata: {os.path.basename(filepath)}",
    )
    table.add_column("Field", style="bold white", no_wrap=True)
    table.add_column("Value", style="green")

    # Classification fields get highlighted styling
    classification_fields = {"media_group", "format_class", "media_class", "quality_type"}

    for key, value in metadata.items():
        style = "magenta bold" if key in classification_fields else "green"
        table.add_row(key, str(value), style=style)

    console.print(table)

    # Export metadata to JSON if requested
    if export_json:
        base = os.path.basename(filepath)
        name, _ = os.path.splitext(base)
        json_filename = f"{name}.metadata.json"

        if out:
            out_path = os.path.join(out, json_filename)
        else:
            out_path = os.path.join(os.path.dirname(filepath), json_filename)

        try:
            with open(out_path, 'w', encoding='utf-8') as f:
                json.dump(metadata, f, indent=4)
            console.print(f"\n[green]✅ Exported metadata to:[/green] {out_path}")
        except Exception as e:
            console.print(f"\n[red]❌ Failed to export JSON:[/red] {e}")
