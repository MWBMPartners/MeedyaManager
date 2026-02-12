# ============================================================================
# File: /core/renamer.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# This module is part of MediaMancer's Milestone 1 deliverables. It processes
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
# - Logging to `logs/rename_preview.log` for dry-run review
#
# References:
# - https://docs.python.org/3/library/os.path.html
# - https://docs.python.org/3/library/re.html
# ============================================================================

import os
import re
import logging
from datetime import datetime
from utils.config_loader import get_config

# Setup log file directory
LOG_DIR = os.path.join("logs")
os.makedirs(LOG_DIR, exist_ok=True)
LOG_FILE = os.path.join(LOG_DIR, "rename_preview.log")

# Logger for console output
logger = logging.getLogger("MediaMancer.Renamer")
logger.setLevel(logging.DEBUG)
handler = logging.StreamHandler()
formatter = logging.Formatter("[%(asctime)s] %(levelname)s - %(message)s")
handler.setFormatter(formatter)
logger.addHandler(handler)

# Logger for file logging
file_logger = logging.getLogger("MediaMancer.RenamerFile")
file_handler = logging.FileHandler(LOG_FILE, mode='a', encoding='utf-8')
file_handler.setFormatter(logging.Formatter("[%(asctime)s] FROM: %(message)s"))
file_logger.addHandler(file_handler)
file_logger.setLevel(logging.INFO)

# Set of characters that are unsafe in file/folder names across platforms
UNSAFE_CHARS_PATTERN = re.compile(r'[<>:"/\\|?*\x00-\x1F]')


def sanitize_filename_component(name):
    """
    Remove or replace characters in a string that are not safe for filenames.
    This helps prevent filesystem errors across platforms.
    """
    replacement = get_config("replacement_char", "_")
    safe = UNSAFE_CHARS_PATTERN.sub(replacement, name)
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
    # Load template and fallback defaults from config
    template = get_config("rename_template")
    fallback = get_config("default_metadata", {})

    # Merge metadata with defaults
    combined = fallback.copy()
    combined.update(metadata)

    ext = os.path.splitext(filepath)[1].lstrip('.')
    combined['ext'] = sanitize_filename_component(ext)

    try:
        relative_path = template.format(
            media_type=sanitize_filename_component(combined.get('media_type')),
            artist=sanitize_filename_component(combined.get('artist')),
            album=sanitize_filename_component(combined.get('album')),
            track_number=str(combined.get('track_number')).zfill(2),
            title=sanitize_filename_component(combined.get('title')),
            ext=combined.get('ext')
        )
    except KeyError as e:
        logger.error(f"Missing required metadata tag: {e}")
        return None

    base_dir = os.path.dirname(filepath)
    new_path = os.path.normpath(os.path.join(base_dir, relative_path))
    logger.info(f"Simulated rename: \n  FROM: {filepath}\n    TO: {new_path}")

    # Write to dry-run rename log
    file_logger.info(f"{filepath}\n    TO: {new_path}")

    return new_path


if __name__ == '__main__':
    from core.metadata_extractor import extract_metadata

    dummy_file = "./watch_folder/test.mp3"
    dummy_metadata = extract_metadata(dummy_file)
    simulate_rename(dummy_file, dummy_metadata)