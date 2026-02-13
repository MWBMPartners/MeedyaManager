# ============================================================================
# File: /cli/commands/lookup.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Click command for looking up metadata from online providers and optionally
# applying matched results back to media files. Uses LookupService
# (metadata/lookup_service.py) to orchestrate searches across all registered
# providers (Spotify, Apple Music, TMDB, MusicBrainz, etc.).
#
# Supports:
#   - Single-file lookup with results displayed in a Rich table
#   - Provider filtering: --provider (-p) for specific providers
#   - Category filtering: --category (-c) for provider category
#   - Auto-apply best match: --auto (confidence >= threshold)
#   - Manual apply: --apply N (1-indexed result number)
#   - Dry-run preview: --dry-run (no files modified)
#   - Skip cover art: --no-art (text metadata only)
#   - JSON output: --json (machine-readable)
#   - Batch mode: --batch file.txt (one path per line)
#   - Provider listing: --providers-list (show all providers + status)
#   - Confidence threshold: --min-confidence (default 0.0 display, 0.8 auto)
#   - Result limit: --max-results (per provider, default 5)
# ============================================================================

import os                                              # File path operations
import json as json_lib                                # JSON serialisation for --json output
import click                                           # CLI framework for command/option definitions
from rich.console import Console                       # Rich terminal output with colour support
from rich.table import Table                           # Formatted table display for results

from metadata.lookup_service import LookupService      # Orchestrates provider searches and apply
from core.metadata_extractor import extract_metadata   # Extracts embedded metadata from media files
from metadata.providers import ProviderCategory        # Enum for provider category validation


# ============================================================================
# Shared console instance — used for all Rich output (tables, messages, etc.)
# ============================================================================
console = Console()

# ============================================================================
# Default confidence threshold for auto-apply mode.
# When --auto is used without --min-confidence, results must meet or exceed
# this score (0.8 = 80% confidence) to be automatically applied to the file.
# ============================================================================
AUTO_APPLY_DEFAULT_CONFIDENCE = 0.8

# ============================================================================
# Valid category names derived from the ProviderCategory enum.
# Used by Click's type=click.Choice() to validate --category input.
# ============================================================================
VALID_CATEGORIES = [c.value for c in ProviderCategory]


def _build_results_table(results, filepath):
    """Build a Rich table displaying lookup results for a single file.

    Creates a numbered table with columns for provider name, confidence
    score, title, artist, and album. Results are shown in the order
    returned by LookupService (sorted by confidence, descending).

    Args:
        results (list[ProviderResult]): Scored results from LookupService.
        filepath (str): The file path being looked up (used in table title).

    Returns:
        Table: A Rich Table object ready for console.print().
    """
    # Create the table with a title showing the filename being looked up
    table = Table(
        title=f"Lookup Results: {os.path.basename(filepath)}",  # Show only filename, not full path
        show_header=True,                              # Display column headers
        header_style="bold cyan",                      # Cyan bold style for header row
    )

    # Column 1: Result number (1-indexed for user reference with --apply N)
    table.add_column("#", style="bold white", no_wrap=True, justify="right")

    # Column 2: Provider name (e.g., "spotify", "apple_music", "tmdb")
    table.add_column("Provider", style="bold magenta", no_wrap=True)

    # Column 3: Confidence score as a percentage (0.0-1.0 → 0%-100%)
    table.add_column("Confidence", style="bold green", no_wrap=True, justify="right")

    # Column 4: Track/movie/episode title from the provider result
    table.add_column("Title", style="white")

    # Column 5: Artist/director name from the provider result
    table.add_column("Artist", style="white")

    # Column 6: Album/show name from the provider result
    table.add_column("Album", style="white")

    # Iterate over results and add each as a numbered row
    for index, result in enumerate(results, start=1):
        # Format confidence as a percentage string (e.g., "85.3%")
        confidence_str = f"{result.confidence * 100:.1f}%"

        # Colour-code confidence: green >= 80%, yellow >= 50%, red < 50%
        if result.confidence >= 0.8:
            confidence_display = f"[green]{confidence_str}[/green]"
        elif result.confidence >= 0.5:
            confidence_display = f"[yellow]{confidence_str}[/yellow]"
        else:
            confidence_display = f"[red]{confidence_str}[/red]"

        # Add the row to the table with all columns populated
        table.add_row(
            str(index),                                # 1-indexed result number
            result.provider_name,                      # Provider name string
            confidence_display,                        # Colour-coded confidence percentage
            result.title or "(unknown)",               # Title, with fallback for empty values
            result.artist or "(unknown)",              # Artist, with fallback for empty values
            result.album or "(unknown)",               # Album, with fallback for empty values
        )

    return table                                       # Return the constructed table


def _build_providers_table(providers_info):
    """Build a Rich table showing all registered providers and their status.

    Displays provider name, category, authentication requirement, current
    availability, and a status message. Used by --providers-list flag.

    Args:
        providers_info (list[dict]): Provider status dicts from
            LookupService.get_available_providers(). Each dict has keys:
            name, category, requires_auth, available, message.

    Returns:
        Table: A Rich Table object ready for console.print().
    """
    # Create the table with a descriptive title
    table = Table(
        title="Registered Metadata Providers",         # Table title
        show_header=True,                              # Display column headers
        header_style="bold cyan",                      # Cyan bold style for header row
    )

    # Column 1: Provider name (e.g., "spotify", "musicbrainz", "tmdb")
    table.add_column("Provider", style="bold white", no_wrap=True)

    # Column 2: Category (e.g., "music", "video", "podcast", "identifier")
    table.add_column("Category", style="magenta", no_wrap=True)

    # Column 3: Whether the provider requires API credentials
    table.add_column("Auth Required", style="dim", no_wrap=True, justify="center")

    # Column 4: Whether the provider is currently available for use
    table.add_column("Available", style="dim", no_wrap=True, justify="center")

    # Column 5: Human-readable status message (e.g., "Available", "Missing credentials")
    table.add_column("Status", style="white")

    # Iterate over provider status dicts and add each as a row
    for info in providers_info:
        # Format auth required as a coloured Yes/No indicator
        auth_str = "[yellow]Yes[/yellow]" if info["requires_auth"] else "[dim]No[/dim]"

        # Format availability as a coloured checkmark or cross
        if info["available"]:
            avail_str = "[green]Yes[/green]"           # Green "Yes" when provider is ready
        else:
            avail_str = "[red]No[/red]"                # Red "No" when provider is unavailable

        # Add the provider row to the table
        table.add_row(
            info["name"],                              # Provider name string
            info["category"],                          # Category string (e.g., "music")
            auth_str,                                  # Coloured auth requirement indicator
            avail_str,                                 # Coloured availability indicator
            info["message"],                           # Status message string
        )

    return table                                       # Return the constructed table


def _result_to_dict(result):
    """Convert a ProviderResult to a plain dict for JSON serialisation.

    Extracts all standard metadata fields, provider identification, and
    confidence score into a flat dictionary. Cover art is serialised as
    a list of URL strings rather than full CoverArtAsset objects.

    Args:
        result (ProviderResult): A single provider result to convert.

    Returns:
        dict: Plain dictionary with all result fields.
    """
    return {
        "provider_name": result.provider_name,         # Source provider name
        "confidence": result.confidence,               # Match confidence (0.0-1.0)
        "title": result.title,                         # Track/movie/episode title
        "artist": result.artist,                       # Artist name(s)
        "album": result.album,                         # Album name
        "genre": result.genre,                         # Genre(s)
        "year": result.year,                           # Release year
        "isrc": result.isrc,                           # ISRC code (music)
        "show": result.show,                           # TV show name (video)
        "season": result.season,                       # Season number (video)
        "episode": result.episode,                     # Episode number (video)
        "episode_title": result.episode_title,         # Episode title (video)
        "director": result.director,                   # Director name (video)
        "track_num": result.track_num,                 # Track number (music)
        "disc_num": result.disc_num,                   # Disc number (music)
        "composer": result.composer,                   # Composer name
        "total_tracks": result.total_tracks,           # Total tracks on album
        "provider_id": result.provider_id,             # Provider-specific ID
        "provider_url": result.provider_url,           # Direct URL to item
        "cover_art": [                                 # List of cover art URLs
            asset.url for asset in (result.cover_art or [])
        ],
        "extra_tags": result.extra_tags,               # Provider-specific extra metadata
    }


def _display_apply_changes(changes, filepath, dry_run):
    """Display the results of applying a provider result to a file.

    Shows a summary of tags written and cover art downloaded/saved.
    Adjusts messaging based on whether this was a dry run or live write.

    Args:
        changes (dict): Result dict from LookupService.apply_result_sync()
            with keys: tags_written, cover_art_saved, dry_run.
        filepath (str): The file path that was (or would be) modified.
        dry_run (bool): Whether this was a dry-run operation.
    """
    # Determine the prefix for messages based on dry-run mode
    prefix = "[cyan]Dry run:[/cyan] Would write" if dry_run else "[green]Wrote[/green]"

    # Display tags written (or that would be written)
    tags_written = changes.get("tags_written", {})     # Dict of {tag_key: value} pairs
    if tags_written:
        console.print(
            f"\n{prefix} {len(tags_written)} tag(s) to {os.path.basename(filepath)}:"
        )
        # Show each tag key-value pair indented for readability
        for key, value in sorted(tags_written.items()):
            console.print(f"  [dim]{key}[/dim] = {value}")
    else:
        console.print(f"\n[yellow]No tags to write for {os.path.basename(filepath)}[/yellow]")

    # Display cover art saved (or that would be saved)
    cover_art_saved = changes.get("cover_art_saved", {})  # Dict of {type: path} pairs
    if cover_art_saved:
        art_prefix = "[cyan]Dry run:[/cyan] Would save" if dry_run else "[green]Saved[/green]"
        console.print(f"\n{art_prefix} {len(cover_art_saved)} cover art asset(s):")
        # Show each cover art type and destination path
        for art_type, art_path in cover_art_saved.items():
            console.print(f"  [dim]{art_type}[/dim]: {art_path}")

    # Display any errors that occurred during the apply operation
    if "error" in changes:
        console.print(f"\n[red]Error during apply:[/red] {changes['error']}")


def _process_single_file(filepath, service, providers, category, min_confidence,
                         max_results, auto_apply, apply_index, dry_run,
                         no_art, export_json):
    """Process a single file through the lookup workflow.

    This is the core logic shared by single-file and batch modes.
    Extracts metadata, searches providers, displays results, and
    optionally applies a selected or auto-selected result.

    Args:
        filepath (str): Path to the media file to look up.
        service (LookupService): The lookup service instance.
        providers (list[str] or None): Specific provider names to search.
        category (ProviderCategory or None): Category filter for providers.
        min_confidence (float): Minimum confidence for result display.
        max_results (int): Maximum results per provider.
        auto_apply (bool): Whether to auto-apply the best match.
        apply_index (int or None): 1-indexed result number to apply.
        dry_run (bool): Whether to preview changes without writing.
        no_art (bool): Whether to skip cover art download.
        export_json (bool): Whether to output results as JSON.

    Returns:
        list[ProviderResult]: The lookup results (for batch summary use).
    """
    # ---- Step 1: Validate the file path ----
    if not os.path.isfile(filepath):
        console.print(f"[red]Not a regular file:[/red] {filepath}")
        return []                                      # Return empty results on validation failure

    # ---- Step 2: Extract current metadata from the file ----
    try:
        metadata = extract_metadata(filepath)          # Returns dict with title, artist, album, etc.
    except Exception as exc:
        # Metadata extraction failed — file may be corrupt or unsupported
        console.print(f"[red]Failed to extract metadata from {filepath}:[/red] {exc}")
        return []                                      # Return empty results on extraction failure

    # ---- Step 3: Perform the lookup across selected providers ----
    # Build keyword arguments for the synchronous lookup call
    lookup_kwargs = {
        "min_confidence": min_confidence,              # Filter threshold for results
        "max_results_per_provider": max_results,       # Limit results per provider
    }

    # Add optional provider name filter (if user specified -p flags)
    if providers:
        lookup_kwargs["providers"] = list(providers)   # Convert tuple to list for the service

    # Add optional category filter (if user specified -c flag)
    if category is not None:
        lookup_kwargs["category"] = category           # ProviderCategory enum value

    # Execute the synchronous lookup (internally runs async providers)
    results = service.lookup_sync(metadata, **lookup_kwargs)

    # ---- Step 4: Handle no results ----
    if not results:
        if export_json:
            # JSON mode: output an empty results array
            click.echo(json_lib.dumps({"file": filepath, "results": []}, indent=2))
        else:
            # Rich mode: show a warning message
            console.print(f"[yellow]No results found for:[/yellow] {os.path.basename(filepath)}")
        return []                                      # Return empty results

    # ---- Step 5: Display results ----
    if export_json:
        # JSON output mode: serialise all results as a JSON object
        output = {
            "file": filepath,                          # The file that was looked up
            "result_count": len(results),              # Total number of results found
            "results": [                               # Array of result objects
                _result_to_dict(r) for r in results
            ],
        }
        click.echo(json_lib.dumps(output, indent=2, ensure_ascii=False))
    else:
        # Rich table output mode: build and display the results table
        table = _build_results_table(results, filepath)
        console.print(table)                           # Print the formatted table to the terminal
        console.print(f"\n[dim]{len(results)} result(s) found[/dim]")

    # ---- Step 6: Auto-apply the best match (if --auto flag is set) ----
    if auto_apply:
        # Use the auto-apply confidence threshold (default 0.8) unless
        # the user explicitly set --min-confidence to a different value
        auto_threshold = max(min_confidence, AUTO_APPLY_DEFAULT_CONFIDENCE)

        # Check if the top result meets the auto-apply threshold
        best_result = results[0]                       # Results are sorted by confidence descending
        if best_result.confidence >= auto_threshold:
            if not export_json:
                # Show which result is being auto-applied
                console.print(
                    f"\n[bold green]Auto-applying[/bold green] result #1 "
                    f"from {best_result.provider_name} "
                    f"(confidence: {best_result.confidence * 100:.1f}%)"
                )

            # Apply the best result to the file
            changes = service.apply_result_sync(
                filepath,                              # Target media file
                best_result,                           # The ProviderResult to apply
                write_tags=True,                       # Write metadata tags to the file
                download_art=not no_art,               # Download cover art unless --no-art
                dry_run=dry_run,                       # Respect --dry-run flag
            )

            # Display what was written (or would be written in dry-run mode)
            if not export_json:
                _display_apply_changes(changes, filepath, dry_run)
        else:
            # Best result didn't meet the threshold — inform the user
            if not export_json:
                console.print(
                    f"\n[yellow]Auto-apply skipped:[/yellow] Best confidence "
                    f"({best_result.confidence * 100:.1f}%) is below threshold "
                    f"({auto_threshold * 100:.1f}%)"
                )

    # ---- Step 7: Apply a specific result by number (if --apply N is set) ----
    if apply_index is not None:
        # Validate the 1-indexed result number is within range
        if apply_index < 1 or apply_index > len(results):
            console.print(
                f"[red]Invalid result number:[/red] {apply_index} "
                f"(valid range: 1-{len(results)})"
            )
            return results                             # Return results but don't apply

        # Convert 1-indexed user input to 0-indexed list access
        selected_result = results[apply_index - 1]

        if not export_json:
            # Show which result is being applied
            console.print(
                f"\n[bold green]Applying[/bold green] result #{apply_index} "
                f"from {selected_result.provider_name} "
                f"(confidence: {selected_result.confidence * 100:.1f}%)"
            )

        # Apply the selected result to the file
        changes = service.apply_result_sync(
            filepath,                                  # Target media file
            selected_result,                           # The ProviderResult to apply
            write_tags=True,                           # Write metadata tags to the file
            download_art=not no_art,                   # Download cover art unless --no-art
            dry_run=dry_run,                           # Respect --dry-run flag
        )

        # Display what was written (or would be written in dry-run mode)
        if not export_json:
            _display_apply_changes(changes, filepath, dry_run)

    return results                                     # Return results for batch summary use


# ============================================================================
# Click command definition
# ============================================================================
@click.command()
@click.argument(
    "filepath",
    required=False,                                    # Optional because --batch and --providers-list don't need it
    default=None,                                      # Default to None when not provided
    type=click.Path(),                                 # Don't require exists=True here (batch mode has no file arg)
)
@click.option(
    "-p", "--provider", "providers",
    multiple=True,                                     # Allow multiple -p flags: -p spotify -p deezer
    help="Search specific provider(s). Can be used multiple times.",
)
@click.option(
    "-c", "--category",
    type=click.Choice(VALID_CATEGORIES, case_sensitive=False),  # Validate against ProviderCategory values
    default=None,                                      # None means search all categories
    help="Filter providers by category (e.g., music, video, podcast, identifier).",
)
@click.option(
    "--auto",
    "auto_apply",                                      # Parameter name in the function signature
    is_flag=True,                                      # Boolean flag, no value needed
    help="Auto-apply the best match if confidence >= threshold (default 0.8).",
)
@click.option(
    "--apply",
    "apply_index",                                     # Parameter name in the function signature
    type=int,                                          # Expects an integer result number
    default=None,                                      # None means don't apply any specific result
    help="Apply result number N (1-indexed) to the file.",
)
@click.option(
    "--dry-run",
    is_flag=True,                                      # Boolean flag, no value needed
    help="Preview changes without actually writing to the file.",
)
@click.option(
    "--no-art",
    is_flag=True,                                      # Boolean flag, no value needed
    help="Skip cover art download when applying results.",
)
@click.option(
    "--json", "export_json",
    is_flag=True,                                      # Boolean flag, no value needed
    help="Output results as JSON instead of a Rich table.",
)
@click.option(
    "--batch",
    type=click.Path(exists=True),                      # Batch file must exist on disk
    default=None,                                      # None means single-file mode
    help="Read file paths from a text file (one per line) for batch processing.",
)
@click.option(
    "--providers-list",
    is_flag=True,                                      # Boolean flag, no value needed
    help="Show all registered providers with their status and availability.",
)
@click.option(
    "--min-confidence",
    type=float,                                        # Floating-point confidence threshold
    default=0.0,                                       # Default 0.0 for display (shows all results)
    help="Minimum confidence threshold (0.0-1.0). Default 0.0 for display, 0.8 for --auto.",
)
@click.option(
    "--max-results",
    type=int,                                          # Integer limit per provider
    default=5,                                         # Default to 5 results per provider
    help="Maximum number of results per provider (default: 5).",
)
def lookup(filepath, providers, category, auto_apply, apply_index, dry_run,
           no_art, export_json, batch, providers_list, min_confidence, max_results):
    """Look up metadata from online providers for a media file.

    Searches registered metadata providers (Spotify, Apple Music, TMDB,
    MusicBrainz, etc.) using the file's existing metadata as a query.
    Results are displayed in a ranked table sorted by match confidence.

    Use --auto to automatically apply the best match, or --apply N to
    apply a specific result. Use --dry-run to preview changes without
    modifying the file.

    \b
    Examples:
        meedyamanager lookup song.mp3
        meedyamanager lookup song.mp3 -p spotify -p deezer
        meedyamanager lookup song.mp3 -c music
        meedyamanager lookup song.mp3 --auto
        meedyamanager lookup song.mp3 --apply 1
        meedyamanager lookup song.mp3 --dry-run
        meedyamanager lookup song.mp3 --no-art
        meedyamanager lookup song.mp3 --json
        meedyamanager lookup --batch files.txt --auto
        meedyamanager lookup --providers-list
    """
    # ---- Handle --providers-list: show all providers and exit ----
    if providers_list:
        # Create a LookupService instance to query the provider registry
        service = LookupService()

        # Retrieve status info for all registered providers
        all_providers = service.get_available_providers()

        if not all_providers:
            # No providers registered at all — likely a setup issue
            console.print("[yellow]No providers registered.[/yellow]")
            console.print("[dim]Check that provider modules are installed correctly.[/dim]")
            raise SystemExit(0)                        # Exit cleanly after showing message

        if export_json:
            # JSON output mode for --providers-list
            click.echo(json_lib.dumps(all_providers, indent=2, ensure_ascii=False))
        else:
            # Rich table output mode for --providers-list
            table = _build_providers_table(all_providers)
            console.print(table)                       # Print the formatted providers table

            # Show summary counts below the table
            available_count = sum(1 for p in all_providers if p["available"])
            total_count = len(all_providers)
            console.print(
                f"\n[dim]{available_count}/{total_count} provider(s) available[/dim]"
            )

        return                                         # Exit after showing providers (no file needed)

    # ---- Resolve the ProviderCategory enum from the string value ----
    resolved_category = None                           # Default: no category filter
    if category is not None:
        # Convert the validated string (e.g., "music") to a ProviderCategory enum value
        resolved_category = ProviderCategory(category)

    # ---- Handle --batch mode: read file paths from a text file ----
    if batch is not None:
        # Read file paths from the batch file (one path per line)
        try:
            with open(batch, "r", encoding="utf-8") as batch_file:
                # Strip whitespace and skip empty lines and comment lines
                file_paths = [
                    line.strip()
                    for line in batch_file.readlines()
                    if line.strip() and not line.strip().startswith("#")
                ]
        except IOError as exc:
            # Failed to read the batch file
            console.print(f"[red]Failed to read batch file {batch}:[/red] {exc}")
            raise SystemExit(1)                        # Exit with error code

        if not file_paths:
            # Batch file exists but contains no valid file paths
            console.print(f"[yellow]No file paths found in batch file:[/yellow] {batch}")
            raise SystemExit(0)                        # Exit cleanly

        # Create a single LookupService instance for all batch operations
        service = LookupService()

        if not export_json:
            # Show batch processing header
            console.print(
                f"\n[bold]Batch lookup:[/bold] {len(file_paths)} file(s) from {batch}\n"
            )

        # Track batch statistics for the summary
        batch_total = len(file_paths)                  # Total files to process
        batch_success = 0                              # Files with at least one result
        batch_applied = 0                              # Files where a result was applied
        batch_json_results = []                        # Collected JSON results for --json mode

        # Process each file in the batch sequentially
        for file_index, file_path in enumerate(file_paths, start=1):
            if not export_json:
                # Show progress indicator for each file
                console.print(
                    f"[bold cyan][{file_index}/{batch_total}][/bold cyan] "
                    f"Processing: {os.path.basename(file_path)}"
                )
                console.print()                        # Blank line for readability between files

            # Process the individual file through the standard workflow
            results = _process_single_file(
                filepath=file_path,
                service=service,
                providers=providers if providers else None,
                category=resolved_category,
                min_confidence=min_confidence,
                max_results=max_results,
                auto_apply=auto_apply,
                apply_index=apply_index,
                dry_run=dry_run,
                no_art=no_art,
                export_json=export_json,
            )

            # Update batch statistics
            if results:
                batch_success += 1                     # At least one result was found
            if (auto_apply or apply_index is not None) and results:
                batch_applied += 1                     # A result was (potentially) applied

            if not export_json and file_index < batch_total:
                # Print a separator between files for readability
                console.print("[dim]" + "-" * 60 + "[/dim]\n")

        # Show batch summary after all files are processed
        if not export_json:
            console.print("\n[bold]Batch Summary[/bold]")
            console.print(f"  Total files:  {batch_total}")
            console.print(f"  With results: {batch_success}")
            if auto_apply or apply_index is not None:
                console.print(f"  Applied:      {batch_applied}")
            if dry_run:
                console.print("  [cyan]Mode: dry run (no files modified)[/cyan]")

        return                                         # Exit after batch processing

    # ---- Single-file mode: require a filepath argument ----
    if filepath is None:
        # No file provided and not in batch/providers-list mode — show usage hint
        console.print(
            "[red]Error:[/red] Please provide a file path, "
            "or use --batch or --providers-list."
        )
        raise SystemExit(1)                            # Exit with error code

    # Verify the file exists on disk before proceeding
    if not os.path.exists(filepath):
        console.print(f"[red]File not found:[/red] {filepath}")
        raise SystemExit(1)                            # Exit with error code

    # Create a LookupService instance for the single-file lookup
    service = LookupService()

    # Process the single file through the standard workflow
    _process_single_file(
        filepath=filepath,
        service=service,
        providers=providers if providers else None,
        category=resolved_category,
        min_confidence=min_confidence,
        max_results=max_results,
        auto_apply=auto_apply,
        apply_index=apply_index,
        dry_run=dry_run,
        no_art=no_art,
        export_json=export_json,
    )
