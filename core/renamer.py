# ============================================================================
# File: /core/renamer.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# This module is part of MetaMancer's Milestone 1 deliverables. It processes
# file paths received from the `watcher.py` module, performs dry-run evaluations
# of their new names/locations based on rule templates, and logs the results.
# This module does not actually move/rename files in this milestone — it
# evaluates and simulates the renaming process for testing and validation.
#
# Future versions will support applying these changes, rollback, and history.
#
# This dry-run engine is designed to test:
# - Rule token parsing (e.g., {artist}, {title}, {format})
# - Character sanitization
# - File extension preservation
#
# References:
# - https://docs.python.org/3/library/os.path.html
# - https://docs.python.org/3/library/re.html
# ============================================================================

import os
import re
import logging
from datetime import datetime

# Create a logger specifically for the renamer
logger = logging.getLogger("MetaMancer.Renamer")
logger.setLevel(logging.DEBUG)
handler = logging.StreamHandler()
formatter = logging.Formatter("[%(asctime)s] %(levelname)s - %(message)s")
handler.setFormatter(formatter)
logger.addHandler(handler)

# Placeholder: In future this will be passed from the rule engine or loaded config
DEFAULT_RENAME_TEMPLATE = "{media_type}/{artist}/{album}/{track_number} - {title}.{ext}"

# Set of characters that are unsafe in file/folder names across platforms
# Based on Windows, macOS, Linux restrictions
UNSAFE_CHARS_PATTERN = re.compile(r'[<>:"/\\|?*\x00-\x1F]')


def sanitize_filename_component(name):
    """
    Remove or replace characters in a string that are not safe for filenames.
    This helps prevent filesystem errors across platforms.
    """
    # Replace unsafe characters with underscores
    safe = UNSAFE_CHARS_PATTERN.sub('_', name)
    # Strip leading/trailing whitespace or dots
    return safe.strip().strip('.')


def simulate_rename(filepath, metadata):
    """
    Simulates renaming a file based on a metadata dictionary and a rule template.
    Returns the new proposed path without actually touching the filesystem.

    Args:
        filepath (str): Original file path
        metadata (dict): Extracted metadata tags

    Returns:
        str: Proposed new file path (simulated)
    """
    ext = os.path.splitext(filepath)[1].lstrip('.')  # Preserve original extension

    # Build the output path by replacing tokens in the template
    try:
        relative_path = DEFAULT_RENAME_TEMPLATE.format(
            media_type=sanitize_filename_component(metadata.get('media_type', 'Unknown')),
            artist=sanitize_filename_component(metadata.get('artist', 'Unknown Artist')),
            album=sanitize_filename_component(metadata.get('album', 'Unknown Album')),
            track_number=str(metadata.get('track_number', '00')).zfill(2),
            title=sanitize_filename_component(metadata.get('title', 'Untitled')),
            ext=sanitize_filename_component(ext)
        )
    except KeyError as e:
        logger.error(f"Missing required metadata tag: {e}")
        return None

    # Join with root base (later configurable)
    base_dir = os.path.dirname(filepath)
    new_path = os.path.normpath(os.path.join(base_dir, relative_path))
    logger.info(f"Simulated rename: \n  FROM: {filepath}\n    TO: {new_path}")
    return new_path


if __name__ == '__main__':
    # Example standalone test for dry-run renamer logic
    dummy_file = "/media/inbox/01_Track.mp3"
    dummy_metadata = {
        'media_type': 'Music',
        'artist': 'Hans Zimmer',
        'album': 'Dune OST',
        'track_number': 1,
        'title': 'Dream of Arrakis'
    }

    simulate_rename(dummy_file, dummy_metadata)