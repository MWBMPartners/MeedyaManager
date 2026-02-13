# ============================================================================
# File: /metadata/multi_value.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Handles multi-value metadata fields consistently across formats.
# Different tag systems represent multi-value fields differently:
#   - Vorbis Comments: native multi-value (same key appears multiple times)
#   - ID3v2: multiple frames or delimiter-separated within a frame
#   - MP4: list of values in a single atom
#
# This module normalizes all representations to a consistent interface:
#   - Internal representation: list of strings
#   - Display/storage representation: semicolon-delimited string
# ============================================================================


# Separator used for display and string-based storage of multi-value fields
MULTI_VALUE_SEPARATOR = ";"

# Fields that commonly hold multiple values across music tagging conventions
_MULTI_VALUE_FIELDS = {
    "artist",              # Multiple performing artists
    "album_artist",        # Multiple album artists
    "genre",               # Multiple genres
    "composer",            # Multiple composers
}


def parse_multi_value(value):
    """
    Convert any tag value representation into a list of individual strings.

    Handles the different ways multi-value data can arrive:
    - list (from Vorbis Comments or MP4 atoms): returned as-is after stripping
    - str with semicolons: split on ';' separator
    - str without semicolons: wrapped in a single-element list
    - None or empty: returns empty list

    Args:
        value: The tag value to parse (str, list, or None).

    Returns:
        list[str]: List of individual value strings, stripped of whitespace.
                   Empty strings are filtered out.
    """
    if value is None:
        return []

    if isinstance(value, list):
        # Already a list — strip whitespace from each element and filter empties
        return [str(v).strip() for v in value if str(v).strip()]

    # Convert to string and split on semicolons
    text = str(value).strip()
    if not text:
        return []

    if MULTI_VALUE_SEPARATOR in text:
        # Split on semicolons, strip each part, filter empties
        return [part.strip() for part in text.split(MULTI_VALUE_SEPARATOR) if part.strip()]

    # Single value — return as one-element list
    return [text]


def format_multi_value(values):
    """
    Join a list of values into a semicolon-delimited string for display
    or storage.

    Args:
        values (list[str]): List of individual value strings.

    Returns:
        str: Semicolon-separated string, or empty string if no values.
    """
    if not values:
        return ""

    # Filter empty strings and strip whitespace
    cleaned = [str(v).strip() for v in values if str(v).strip()]
    return f" {MULTI_VALUE_SEPARATOR} ".join(cleaned)


def is_multi_value_field(internal_key):
    """
    Check whether a given metadata field commonly holds multiple values.

    This helps the UI and CLI determine when to present multi-value editing
    controls or split/join behaviour.

    Args:
        internal_key (str): TAG_MAP internal snake_case key (e.g., "artist", "genre").

    Returns:
        bool: True if the field commonly holds multiple values.
    """
    return internal_key in _MULTI_VALUE_FIELDS
