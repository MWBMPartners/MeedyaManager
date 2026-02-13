# ============================================================================
# File: /cli/commands/edit.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Click command for reading and editing metadata tags on media files.
# Uses TagEditor (metadata/editor.py) to read/write embedded tags via
# the mutagen library. Supports:
#   - Viewing all tags in a Rich table (default, no options)
#   - Setting tags: --set "Artist=New Artist"
#   - Removing tags: --remove Artist
#   - Cover art management: --cover image.jpg, --remove-cover
#   - Dry-run preview: --dry-run
#   - JSON export: --json
# ============================================================================

import os                                              # File path operations
import json                                            # JSON metadata export
import click                                           # CLI framework
from rich.console import Console                       # Rich terminal output
from rich.table import Table                           # Formatted tag display

from metadata.editor import (
    TagEditor,                                         # Tag read/write engine
    UnsupportedFormatError,                            # Format not supported error
    TagWriteError,                                     # Write operation error
)
from core.tag_registry import (
    REVERSE_TAG_MAP,                                   # Internal key → display name
    TAG_MAP,                                           # Display name → internal key
    is_editable_tag,                                   # Check if tag is writable
)


# Shared console instance for rich output
console = Console()


def _resolve_key(user_key):
    """
    Resolve a user-provided tag name to the internal snake_case key.

    Accepts display names ("Artist", "Album Artist"), internal keys
    ("artist", "album_artist"), or custom tag names ("Custom:SpotifyURL").

    Args:
        user_key (str): The tag name as provided by the user.

    Returns:
        str: The resolved internal key, or the lowered/underscored version
             as a best-effort fallback.
    """
    # Check if it's already an internal key
    if user_key in REVERSE_TAG_MAP:
        return user_key

    # Check if it's a display name in TAG_MAP
    if user_key in TAG_MAP:
        return TAG_MAP[user_key]

    # Case-insensitive display name match
    user_lower = user_key.lower()
    for display, internal in TAG_MAP.items():
        if display.lower() == user_lower:
            return internal

    # Custom tag: "Custom:SpotifyURL" → "custom_spotifyurl"
    if user_key.lower().startswith("custom:"):
        suffix = user_key[7:]                          # Strip "Custom:" prefix
        return "custom_" + suffix.lower().replace(" ", "_")

    # Fallback: convert to snake_case internal key format
    return user_key.lower().replace(" ", "_")


@click.command()
@click.argument(
    "filepath",
    type=click.Path(exists=True),                      # File must exist
)
@click.option(
    "--set", "tag_pairs",
    multiple=True,
    help='Set a tag value: --set "Artist=New Artist". Can be used multiple times.',
)
@click.option(
    "--remove", "remove_tags",
    multiple=True,
    help='Remove a tag: --remove Artist. Can be used multiple times.',
)
@click.option(
    "--cover",
    type=click.Path(exists=True),
    default=None,
    help="Set cover art from an image file (JPEG or PNG).",
)
@click.option(
    "--remove-cover",
    is_flag=True,
    help="Remove all embedded cover art from the file.",
)
@click.option(
    "--dry-run",
    is_flag=True,
    help="Preview changes without actually writing to the file.",
)
@click.option(
    "--json", "export_json",
    is_flag=True,
    help="Output current tags as JSON instead of a Rich table.",
)
def edit(filepath, tag_pairs, remove_tags, cover, remove_cover, dry_run, export_json):
    """Read and edit metadata tags on a media file.

    When called with no --set or --remove options, displays the current
    tags in a formatted table. Use --set and --remove to modify tags.

    Tag names can be display names (e.g., "Album Artist"), internal keys
    (e.g., "album_artist"), or custom tags (e.g., "Custom:SpotifyURL").

    \b
    Examples:
        meedyamanager edit song.mp3
        meedyamanager edit song.mp3 --set "Artist=New Artist"
        meedyamanager edit song.mp3 --set "Genre=Rock" --set "Year=2026"
        meedyamanager edit song.mp3 --remove Genre
        meedyamanager edit song.mp3 --cover artwork.jpg
        meedyamanager edit song.mp3 --remove-cover
        meedyamanager edit song.mp3 --dry-run --set "Title=New Title"
        meedyamanager edit song.mp3 --json
    """
    # Verify file is a regular file
    if not os.path.isfile(filepath):
        console.print(f"[red]Not a regular file:[/red] {filepath}")
        raise SystemExit(1)

    editor = TagEditor()

    # Check if format is supported
    fmt = editor.get_supported_format(filepath)
    if fmt is None:
        console.print(f"[yellow]Warning:[/yellow] Format not supported by mutagen: {filepath}")
        console.print("Tag reading/writing may not be available for this file type.")

    # --- Handle cover art operations ---
    if cover:
        try:
            with open(cover, "rb") as f:
                image_data = f.read()
            ext = os.path.splitext(cover)[1].lower()
            img_format = "png" if ext == ".png" else "jpeg"

            if dry_run:
                console.print(f"[cyan]Dry run:[/cyan] Would set cover art from {cover}")
            else:
                editor.write_cover_art(filepath, image_data, img_format)
                console.print(f"[green]Cover art set from:[/green] {cover}")
        except (TagWriteError, UnsupportedFormatError) as e:
            console.print(f"[red]Cover art error:[/red] {e}")
            raise SystemExit(1)

    if remove_cover:
        try:
            if dry_run:
                covers = editor.read_cover_art(filepath)
                console.print(f"[cyan]Dry run:[/cyan] Would remove {len(covers)} cover art image(s)")
            else:
                count = editor.remove_cover_art(filepath)
                if count > 0:
                    console.print(f"[green]Removed {count} cover art image(s)[/green]")
                else:
                    console.print("[yellow]No cover art to remove[/yellow]")
        except TagWriteError as e:
            console.print(f"[red]Cover art error:[/red] {e}")
            raise SystemExit(1)

    # --- Handle tag edits ---
    has_edits = bool(tag_pairs or remove_tags)

    if has_edits:
        # Build the tags dict for write_tags()
        tags_to_write = {}

        # Parse --set pairs: "Key=Value"
        for pair in tag_pairs:
            if "=" not in pair:
                console.print(f"[red]Invalid --set format:[/red] '{pair}' (expected Key=Value)")
                raise SystemExit(1)
            key, value = pair.split("=", 1)
            internal_key = _resolve_key(key.strip())
            tags_to_write[internal_key] = value.strip()

        # Parse --remove tags (set to None to remove)
        for tag_name in remove_tags:
            internal_key = _resolve_key(tag_name.strip())
            tags_to_write[internal_key] = None

        # Write (or dry-run)
        try:
            changes = editor.write_tags(filepath, tags_to_write, dry_run=dry_run)

            if changes:
                # Display changes table
                table = Table(
                    title=f"{'[cyan]Dry Run[/cyan] ' if dry_run else ''}Tag Changes: {os.path.basename(filepath)}",
                    show_header=True,
                    header_style="bold cyan",
                )
                table.add_column("Tag", style="bold white", no_wrap=True)
                table.add_column("Old Value", style="red")
                table.add_column("New Value", style="green")

                for key, (old, new) in changes.items():
                    display = REVERSE_TAG_MAP.get(key, key)
                    new_display = new if new else "(removed)"
                    table.add_row(display, old, new_display)

                console.print(table)

                if dry_run:
                    console.print("\n[cyan]No changes written (dry run mode)[/cyan]")
                else:
                    console.print(f"\n[green]Wrote {len(changes)} tag change(s)[/green]")
            else:
                console.print("[yellow]No changes detected — tags already have the specified values[/yellow]")

        except UnsupportedFormatError as e:
            console.print(f"[red]Unsupported format:[/red] {e}")
            raise SystemExit(1)
        except TagWriteError as e:
            console.print(f"[red]Write error:[/red] {e}")
            raise SystemExit(1)

        return                                             # Done — don't show the read table

    # --- Display current tags (no edit options given) ---
    tags = editor.read_tags(filepath)

    if export_json:
        # JSON output mode
        click.echo(json.dumps(tags, indent=2, ensure_ascii=False))
        return

    if not tags:
        console.print(f"[yellow]No tags found in:[/yellow] {filepath}")
        return

    # Rich table display of all tags
    table = Table(
        title=f"Tags: {os.path.basename(filepath)}",
        show_header=True,
        header_style="bold cyan",
    )
    table.add_column("Tag", style="bold white", no_wrap=True)
    table.add_column("Value", style="green")
    table.add_column("Editable", style="dim", no_wrap=True)

    for internal_key in sorted(tags.keys()):
        display_name = REVERSE_TAG_MAP.get(internal_key, internal_key)
        value = str(tags[internal_key])
        editable = "Yes" if is_editable_tag(internal_key) else "No"
        table.add_row(display_name, value, editable)

    console.print(table)

    # Show format info
    if fmt:
        console.print(f"\n[dim]Format: {fmt} | Tags: {len(tags)} | "
                       f"Editable: {sum(1 for k in tags if is_editable_tag(k))}[/dim]")
