# ============================================================================
# File: /cli/commands/rule.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Click command for testing rename rule templates against files or sample
# metadata. Validates template syntax and shows the resulting file path.
# Useful for previewing rule behavior before applying to watch folders.
#
# M3 Update: Supports MusicBee-style <Tag> and $Function() syntax.
# Legacy {placeholder} syntax is still accepted with deprecation warning.
# ============================================================================

import os                                          # File path operations
import sys                                         # Exit codes
import click                                       # CLI framework
from rich.console import Console                   # Rich terminal output
from rich.table import Table                       # Formatted output tables
from rich.panel import Panel                       # Boxed info panels

from utils.config_loader import get_config         # Safe config access
from core.rule_engine import (
    RuleEngine,                                    # Template evaluator
    TemplateSyntaxError,                           # Syntax error type
    TemplateEvalError,                             # Evaluation error type
)
from core.tag_registry import get_display_tags, TAG_MAP  # Tag name reference
from utils.char_replacer import sanitize_path      # Path sanitization


# Shared console instance for rich output
console = Console()

# Shared rule engine instance
_engine = RuleEngine()

# Built-in sample metadata for testing templates without a real file
# Uses internal snake_case keys (what the rule engine resolves tags to)
SAMPLE_METADATA = {
    "filepath": "/example/media/sample_track.mp3",
    "filename": "sample_track",
    "extension": "mp3",
    "format": "mp3",
    "duration": "245",
    "title": "Sample Track",
    "description": "",
    "audio_channels": "2",
    "is_lossless": "False",
    "media_group": "Audio",
    "format_class": "mp3",
    "media_class": "Music",
    "quality_type": "Lossy",
    "artist": "Test Artist",
    "album": "Test Album",
    "album_artist": "Test Artist",
    "year": "2025",
    "genre": "Rock; Alternative",
    "track_num": "3",
    "track_number": "3",
    "disc_num": "1",
    "total_tracks": "12",
    "codec": "MP3",
    "bitrate": "320",
    "sample_rate": "44100",
    "bit_depth": "16",
    "date_added": "2025-06-15",
}


@click.command()
@click.option(
    "--template", "-t",
    type=str,
    default=None,
    help="Rename template to test (e.g. '<Media Class>/<Artist>/<Title>.<Ext>')."
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
@click.option(
    "--validate",
    is_flag=True,
    help="Validate template syntax without evaluating."
)
def rule(template, filepath, sample, validate):
    """Test a rename template against a file or sample data.

    Validates the template syntax and shows what the resulting
    file path would be. Use --sample for quick testing without
    needing a real media file.

    \b
    Available tags (use <Tag Name> syntax):
      <Title>, <Artist>, <Album>, <Album Artist>, <Year>, <Genre>,
      <Track #>, <Disc #>, <Media Class>, <Media Group>,
      <Format Class>, <Quality Type>, <Ext>, <Filename>,
      <Channels>, <Codec>, <Bitrate>, <Date Added>

    \b
    Available functions:
      $If(), $And(), $Or(), $IsNull(), $Contains(), $IsMatch(),
      $Replace(), $RxReplace(), $Left(), $Right(), $Upper(), $Lower(),
      $Trim(), $Split(), $RSplit(), $First(), $Pad(), $Date(),
      $Sort(), $Group()
    """
    # Load environment variables
    from utils.env_loader import load_env_variables
    load_env_variables()

    # Use config template if none provided via CLI
    if template is None:
        template = get_config(
            "rename_format",
            "<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>"
        )
        console.print(f"[dim]Using template from config:[/dim] {template}\n")

    # Validate-only mode: check syntax and exit
    if validate:
        errors = _engine.validate(template)
        if errors:
            console.print("[red]❌ Template syntax errors:[/red]")
            for err in errors:
                console.print(f"  [red]• {err}[/red]")
            raise SystemExit(1)
        else:
            console.print("[green]✅ Template syntax is valid.[/green]")
            return

    # Determine metadata source: file or sample
    if filepath:
        from core.metadata_extractor import extract_metadata
        from core.classify_media import classify_media
        metadata = extract_metadata(filepath)
        classified = classify_media(metadata)
        metadata.update(classified)
        # Add filename and extension
        metadata['filename'] = os.path.splitext(os.path.basename(filepath))[0]
        metadata['extension'] = os.path.splitext(filepath)[1].lstrip('.')
        source_label = os.path.basename(filepath)
    elif sample:
        metadata = SAMPLE_METADATA.copy()
        filepath = metadata["filepath"]
        source_label = "sample data"
    else:
        # No file or --sample: show help and exit
        console.print("[yellow]Please provide --file or --sample to test the template.[/yellow]")
        console.print("  Example: [cyan]meedyamanager rule --sample -t '<Media Class>/<Title>.<Ext>'[/cyan]")
        raise SystemExit(1)

    # Display the input
    console.print(
        Panel(
            f"[bold]Template:[/bold]  {template}\n"
            f"[bold]Source:[/bold]    {source_label}",
            title="📐 Rule Test",
            style="cyan",
        )
    )

    # Merge with fallback metadata
    fallback = get_config("fallback_metadata", {})
    combined = fallback.copy()
    combined.update(metadata)

    # Evaluate the template
    try:
        result = _engine.evaluate(template, combined)
        # Apply character replacement/sanitization
        sanitized_result = sanitize_path(result)
        console.print(f"\n[green]✅ Result:[/green] {sanitized_result}")
    except TemplateSyntaxError as e:
        console.print(f"\n[red]❌ Template syntax error:[/red] {e}")
        raise SystemExit(1)
    except TemplateEvalError as e:
        console.print(f"\n[red]❌ Template evaluation error:[/red] {e}")
        raise SystemExit(1)

    # Show available tags for reference
    console.print("\n[dim]Available tags for this source:[/dim]")
    tag_table = Table(show_header=True, header_style="bold dim")
    tag_table.add_column("Tag", style="cyan")
    tag_table.add_column("Internal Key", style="dim")
    tag_table.add_column("Value", style="white")

    # Show standard tags from the registry
    for display_name in get_display_tags():
        internal_key = TAG_MAP.get(display_name, "")
        value = combined.get(internal_key, "[dim]—[/dim]")
        tag_table.add_row(f"<{display_name}>", internal_key, str(value))

    console.print(tag_table)
