# ============================================================================
# File: /cli/commands/rule.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Click command for testing rename rule templates against files or sample
# metadata. Validates template syntax and shows the resulting file path.
# Useful for previewing rule behavior before applying to watch folders.
# ============================================================================

import os                                              # File path operations
import click                                           # CLI framework
from rich.console import Console                       # Rich terminal output
from rich.table import Table                           # Formatted output tables
from rich.panel import Panel                           # Boxed info panels

from utils.config_loader import get_config             # Safe config access
from core.renamer import simulate_rename, sanitize_filename_component


# Shared console instance for rich output
console = Console()

# Built-in sample metadata for testing templates without a real file
SAMPLE_METADATA = {
    "filepath": "/example/media/sample_track.mp3",
    "extension": "mp3",
    "format": "mp3",
    "duration": 245,
    "title": "Sample Track",
    "description": "",
    "audio_channels": 2,
    "is_lossless": False,
    "media_group": "Audio",
    "format_class": "mp3",
    "media_class": "Music",
    "quality_type": "Lossy",
    "artist": "Test Artist",
    "album": "Test Album",
    "track_num": "03",
    "track_number": "03",
}


@click.command()
@click.option(
    "--template", "-t",
    type=str,
    default=None,
    help="Rename template to test (e.g. '{media_class}/{artist}/{title}.{extension}')."
)
@click.option(
    "--file", "-f", "filepath",
    type=click.Path(exists=True),
    default=None,
    help="Media file to test the template against."
)
@click.option(
    "--sample",
    is_flag=True,
    help="Use built-in sample metadata instead of a real file."
)
def rule(template, filepath, sample):
    """Test a rename template against a file or sample data.

    Validates the template syntax and shows what the resulting
    file path would be. Use --sample for quick testing without
    needing a real media file.

    \b
    Available placeholders include:
      {title}, {artist}, {album}, {media_class}, {media_group},
      {format_class}, {quality_type}, {extension}, {ext},
      {track_num}, {duration}, {audio_channels}
    """
    # Load environment variables
    from utils.env_loader import load_env_variables
    load_env_variables()

    # Use config template if none provided via CLI
    if template is None:
        template = get_config("rename_format", "{media_class}/{title}.{extension}")
        console.print(f"[dim]Using template from config:[/dim] {template}\n")

    # Determine metadata source: file or sample
    if filepath:
        from core.metadata_extractor import extract_metadata
        from core.classify_media import classify_media
        metadata = extract_metadata(filepath)
        classified = classify_media(metadata)
        metadata.update(classified)
        source_label = os.path.basename(filepath)
    elif sample:
        metadata = SAMPLE_METADATA.copy()
        filepath = metadata["filepath"]
        source_label = "sample data"
    else:
        # No file or --sample: show help and exit
        console.print("[yellow]Please provide --file or --sample to test the template.[/yellow]")
        console.print("  Example: [cyan]meedyamanager rule --sample -t '{media_class}/{title}.{ext}'[/cyan]")
        raise SystemExit(1)

    # Display the input metadata
    console.print(
        Panel(
            f"[bold]Template:[/bold]  {template}\n"
            f"[bold]Source:[/bold]    {source_label}",
            title="📐 Rule Test",
            style="cyan",
        )
    )

    # Build sanitized metadata for template expansion (mirrors renamer logic)
    fallback = get_config("fallback_metadata", {})
    combined = fallback.copy()
    combined.update(metadata)

    # Add extension aliases
    ext = os.path.splitext(filepath)[1].lstrip('.')
    combined['ext'] = sanitize_filename_component(ext)
    combined['extension'] = combined['ext']

    # Sanitize all values
    sanitized = {}
    for key, value in combined.items():
        if isinstance(value, str):
            sanitized[key] = sanitize_filename_component(value)
        elif isinstance(value, (int, float)):
            sanitized[key] = str(value)
        else:
            sanitized[key] = str(value) if value is not None else "Unknown"

    # Zero-pad track numbers
    if 'track_num' in sanitized:
        sanitized['track_num'] = sanitized['track_num'].zfill(2)
    if 'track_number' in sanitized:
        sanitized['track_number'] = sanitized['track_number'].zfill(2)

    # Attempt template expansion
    try:
        result = template.format(**sanitized)
        console.print(f"\n[green]✅ Result:[/green] {result}")
    except KeyError as e:
        console.print(f"\n[red]❌ Missing tag:[/red] {e}")
        console.print("[dim]Available tags:[/dim]")
        # Show available tags in a table
        tag_table = Table(show_header=True, header_style="bold")
        tag_table.add_column("Tag", style="cyan")
        tag_table.add_column("Value", style="white")
        for key in sorted(sanitized.keys()):
            tag_table.add_row(f"{{{key}}}", sanitized[key])
        console.print(tag_table)
        raise SystemExit(1)

    # Show all available tags for reference
    console.print("\n[dim]Available tags for this source:[/dim]")
    tag_table = Table(show_header=True, header_style="bold dim")
    tag_table.add_column("Tag", style="cyan")
    tag_table.add_column("Value", style="white")
    for key in sorted(sanitized.keys()):
        tag_table.add_row(f"{{{key}}}", sanitized[key])
    console.print(tag_table)
