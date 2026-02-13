# ============================================================================
# File: /utils/char_replacer.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Configurable filename character replacement and sanitization module.
# Replaces the fixed-pattern approach in renamer.py with a two-stage
# process:
#   1. Apply user-configured character replacements from settings.json5
#      (the "filename_replacements" key)
#   2. Strip any remaining OS-unsafe characters that weren't handled
#      by the configured replacements
#
# This activates the filename_replacements config key that was defined
# in settings.json5 since M1 but not yet integrated into the pipeline.
# ============================================================================

import re                                          # Regex for fallback unsafe char stripping
from utils.config_loader import get_config         # Config access for replacement rules


# ============================================================================
# Regex pattern matching characters unsafe for filenames across all platforms.
# Windows is the most restrictive: < > : " / \ | ? * and control chars 0x00-0x1F
# This is the fallback — applied AFTER user-configured replacements.
# ============================================================================
UNSAFE_CHARS_PATTERN = re.compile(r'[<>:"/\\|?*\x00-\x1F]')


def sanitize_component(text, replacements=None):
    """
    Sanitize a single filename/folder component for safe filesystem use.

    Two-stage process:
    1. Apply user-configured character replacements (e.g., ":" → "-")
    2. Strip any remaining unsafe characters not covered by replacements

    Args:
        text (str): The text to sanitize (e.g., a metadata value like artist name)
        replacements (dict, optional): Character replacement mapping.
            If None, loads from config "filename_replacements" key.
            Pass an empty dict {} to skip configured replacements.

    Returns:
        str: Sanitized text safe for use in filenames. Returns "Unknown" if
             the input is None, empty, or becomes empty after sanitization.
    """
    # Handle None or empty input
    if not text:
        return "Unknown"

    # Load replacements from config if not provided
    if replacements is None:
        replacements = get_config("filename_replacements", {})

    # Stage 1: Apply user-configured character replacements
    # Process each replacement rule from the config mapping
    result = text
    for find_char, replace_char in replacements.items():
        result = result.replace(find_char, replace_char)

    # Stage 2: Strip any remaining unsafe characters with underscore fallback
    # This catches anything not explicitly handled by user replacements
    fallback_char = get_config("replacement_char", "_")
    result = UNSAFE_CHARS_PATTERN.sub(fallback_char, result)

    # Clean up: strip leading/trailing whitespace and dots (unsafe on Windows)
    result = result.strip().strip(".")

    # Return "Unknown" if everything was stripped away
    return result if result else "Unknown"


def sanitize_path(path_string, replacements=None):
    """
    Sanitize a full path string by sanitizing each component independently.

    Splits on "/" (the template path separator), sanitizes each folder/file
    component, then rejoins with os-appropriate separator. Leading/trailing
    slashes are preserved if present.

    Args:
        path_string (str): The path to sanitize (e.g., "Rock/AC:DC/Album/song.mp3")
        replacements (dict, optional): Character replacement mapping.
            If None, loads from config.

    Returns:
        str: Sanitized path with each component individually cleaned.
             Returns "Unknown" if input is None or empty.
    """
    # Handle None or empty input
    if not path_string:
        return "Unknown"

    # Load replacements from config once (avoid re-loading per component)
    if replacements is None:
        replacements = get_config("filename_replacements", {})

    # Split on "/" — the standard template path separator
    # (MeedyaManager converts to OS separator later in renamer.py)
    components = path_string.split("/")

    # Sanitize each component independently
    # Empty components (from leading/trailing/double slashes) are preserved
    sanitized_parts = []
    for component in components:
        if component:                                  # Non-empty component
            sanitized_parts.append(sanitize_component(component, replacements))
        else:
            sanitized_parts.append("")                 # Preserve empty for leading "/"

    # Rejoin with "/" — os.path.normpath in renamer.py handles OS conversion
    return "/".join(sanitized_parts)
