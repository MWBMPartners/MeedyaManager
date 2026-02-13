# ============================================================================
# File: /core/companion_tracker.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Detects companion files associated with media files — subtitles, lyrics,
# cue sheets, metadata, disc images, and cover art. When a media file is
# renamed/moved, its companions should follow to maintain file associations.
#
# Two types of companions are tracked:
#   1. Same-name companions: Same base name with a different extension
#      (e.g., "song.lrc" alongside "song.mp3")
#   2. Directory companions: Well-known filenames that belong to the same
#      folder (e.g., "cover.jpg", "folder.jpg")
#
# This module does NOT move files — it identifies companions and computes
# where they should go if the media file moves.
# ============================================================================

import os                                          # File path operations
import logging                                     # Structured logging
from collections import namedtuple                 # Lightweight companion record

logger = logging.getLogger("MeedyaManager.CompanionTracker")

# Named tuple representing a detected companion file
CompanionFile = namedtuple("CompanionFile", ["path", "category"])

# ============================================================================
# Companion file extension categories
# Each category maps to a list of extensions (lowercase, with leading dot)
# that are considered companions to a media file when they share the same
# base name (e.g., "movie.srt" is a subtitle companion to "movie.mkv")
# ============================================================================
COMPANION_EXTENSIONS = {
    "subtitles": [
        ".srt",                                    # SubRip subtitle format
        ".sub",                                    # MicroDVD / SubViewer format
        ".ass",                                    # Advanced SubStation Alpha
        ".ssa",                                    # SubStation Alpha
        ".vtt",                                    # WebVTT (web subtitles)
        ".idx",                                    # VobSub index file
    ],
    "lyrics": [
        ".lrc",                                    # Synced lyrics format
    ],
    "cue_sheets": [
        ".cue",                                    # CD cue sheet
    ],
    "metadata": [
        ".nfo",                                    # Media info / metadata file
    ],
    "disc_images": [
        ".iso",                                    # ISO disc image
        ".img",                                    # Raw disc image
        ".bin",                                    # Binary disc image
    ],
}

# ============================================================================
# Directory-level companion filenames
# These are files that live in the same directory as media files and are
# associated with the album/folder rather than a specific track. They should
# follow when the entire folder moves.
# ============================================================================
DIRECTORY_COMPANIONS = [
    "cover.jpg",                                   # Primary album art
    "cover.png",
    "cover.bmp",
    "folder.jpg",                                  # Windows Media Player art
    "folder.png",
    "artwork.jpg",                                 # Generic art name
    "artwork.png",
    "front.jpg",                                   # Front cover variant
    "front.png",
    "album.jpg",                                   # Album art variant
    "album.png",
]

# Flat set of all companion extensions for fast lookup
_ALL_COMPANION_EXTENSIONS = set()
for _exts in COMPANION_EXTENSIONS.values():
    _ALL_COMPANION_EXTENSIONS.update(_exts)

# Lowercase set of directory companion filenames for fast lookup
_DIRECTORY_COMPANION_SET = {name.lower() for name in DIRECTORY_COMPANIONS}


def find_companions(filepath):
    """
    Find all companion files associated with a media file.

    Checks two sources:
    1. Same-name companions: Files in the same directory that share the base
       name but have a recognized companion extension (subtitles, lyrics, etc.)
    2. Directory companions: Well-known filenames (cover.jpg, folder.jpg, etc.)
       in the same directory.

    Args:
        filepath (str): Absolute path to the media file

    Returns:
        list[CompanionFile]: List of CompanionFile(path, category) named tuples,
                            sorted by category then path. Empty list if no
                            companions found or filepath is invalid.
    """
    if not filepath or not os.path.exists(filepath):
        return []

    directory = os.path.dirname(filepath)
    base_name = os.path.splitext(os.path.basename(filepath))[0]
    companions = []

    # Avoid scanning if the directory doesn't exist
    if not os.path.isdir(directory):
        return []

    try:
        dir_contents = os.listdir(directory)
    except OSError as e:
        logger.warning(f"Cannot list directory {directory}: {e}")
        return []

    for entry in dir_contents:
        entry_path = os.path.join(directory, entry)

        # Skip the media file itself and directories
        if entry_path == filepath or os.path.isdir(entry_path):
            continue

        entry_lower = entry.lower()
        entry_base = os.path.splitext(entry)[0]
        entry_ext = os.path.splitext(entry)[1].lower()

        # Check 1: Same-name companion (same base name, companion extension)
        if entry_base == base_name and entry_ext in _ALL_COMPANION_EXTENSIONS:
            # Determine which category this extension belongs to
            category = _get_category_for_extension(entry_ext)
            companions.append(CompanionFile(entry_path, category))

        # Check 2: Directory-level companion (well-known filename)
        elif entry_lower in _DIRECTORY_COMPANION_SET:
            companions.append(CompanionFile(entry_path, "cover_art"))

    # Sort by category then path for consistent ordering
    companions.sort(key=lambda c: (c.category, c.path))

    if companions:
        logger.debug(
            f"Found {len(companions)} companion(s) for "
            f"{os.path.basename(filepath)}: "
            f"{', '.join(os.path.basename(c.path) for c in companions)}"
        )

    return companions


def _get_category_for_extension(ext):
    """
    Look up which companion category an extension belongs to.

    Args:
        ext (str): File extension (lowercase, with leading dot)

    Returns:
        str: Category name (e.g., "subtitles", "lyrics") or "unknown"
    """
    for category, extensions in COMPANION_EXTENSIONS.items():
        if ext in extensions:
            return category
    return "unknown"


def compute_companion_destinations(companions, old_media_path, new_media_path):
    """
    Compute where companion files should be moved when their associated
    media file is renamed/moved.

    Rules:
    - Same-name companions (subtitles, lyrics, cue, metadata, disc_images):
      Follow the media file's new base name + keep their original extension.
      Example: "song.srt" follows "song.mp3" → "new_song.srt"
    - Directory companions (cover_art): Follow the media file's new directory
      but keep their original filename.
      Example: "cover.jpg" stays as "cover.jpg" in the new directory.

    Args:
        companions (list[CompanionFile]): List from find_companions()
        old_media_path (str): Original media file path
        new_media_path (str): Proposed new media file path

    Returns:
        dict: Mapping of old companion path → new companion path
    """
    if not companions or not old_media_path or not new_media_path:
        return {}

    # Extract components from the new media path
    new_dir = os.path.dirname(new_media_path)
    new_base = os.path.splitext(os.path.basename(new_media_path))[0]

    destinations = {}

    for companion in companions:
        companion_filename = os.path.basename(companion.path)
        companion_ext = os.path.splitext(companion_filename)[1]

        if companion.category == "cover_art":
            # Directory companions: keep filename, change directory
            new_companion_path = os.path.join(new_dir, companion_filename)
        else:
            # Same-name companions: follow new base name + keep extension
            new_companion_path = os.path.join(
                new_dir, new_base + companion_ext
            )

        destinations[companion.path] = new_companion_path

    return destinations


def get_companion_summary(companions):
    """
    Generate a human-readable summary of companion files.
    Useful for UI tooltips and log messages.

    Args:
        companions (list[CompanionFile]): List from find_companions()

    Returns:
        str: Summary string, e.g., "2 subtitles, 1 cover art"
             Returns "None" if no companions.
    """
    if not companions:
        return "None"

    # Count companions per category
    counts = {}
    for companion in companions:
        counts[companion.category] = counts.get(companion.category, 0) + 1

    # Build readable summary parts
    parts = []
    for category, count in sorted(counts.items()):
        # Humanize category name: "cue_sheets" → "cue sheets"
        display_name = category.replace("_", " ")
        parts.append(f"{count} {display_name}")

    return ", ".join(parts)
