# ============================================================================
# File: /core/tag_registry.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Bidirectional mapping between MusicBee-style display tag names (e.g.,
# "Album Artist", "Track #") and internal snake_case metadata keys
# (e.g., "album_artist", "track_num") used throughout the codebase.
#
# This registry is the foundation for the M3 rule engine template syntax,
# where tags are referenced as <Album Artist> in templates but stored
# internally as "album_artist" in metadata dictionaries.
#
# Also supports unlimited custom tags via the <Custom:AnyName> prefix,
# removing MusicBee's 16-20 custom tag limit.
# ============================================================================

# ============================================================================
# Tag Map — Maps friendly display names to internal metadata keys.
#
# Display names use Title Case with spaces (e.g., "Album Artist") and are
# the names users see in templates, UI dropdowns, and documentation.
#
# Internal keys use snake_case (e.g., "album_artist") and are the keys
# used in metadata dictionaries throughout the codebase.
# ============================================================================

TAG_MAP = {
    # -------------------------------------------------------------------------
    # Standard Audio Tags — Common music metadata fields
    # -------------------------------------------------------------------------
    "Title": "title",                   # Track title
    "Artist": "artist",                 # Track artist(s)
    "Album": "album",                   # Album name
    "Album Artist": "album_artist",     # Album-level artist (compilation support)
    "Year": "year",                     # Release year
    "Genre": "genre",                   # Genre(s), may be multi-value
    "Track #": "track_num",             # Track number on disc
    "Disc #": "disc_num",               # Disc number in set
    "Total Tracks": "total_tracks",     # Total tracks on disc
    "Total Discs": "total_discs",       # Total discs in set
    "Composer": "composer",             # Composer name
    "Publisher": "publisher",           # Record label / publisher
    "Comment": "description",           # Comment / description field
    "BPM": "bpm",                       # Beats per minute

    # -------------------------------------------------------------------------
    # Standard Video Tags — TV show and movie metadata
    # -------------------------------------------------------------------------
    "Show": "show",                     # TV show name
    "Season": "season",                 # Season number
    "Episode": "episode",               # Episode number
    "Episode Title": "episode_title",   # Episode title
    "Director": "director",             # Director name
    "Resolution": "resolution",         # Video resolution (e.g., "1080p")

    # -------------------------------------------------------------------------
    # Classification Tags — MeedyaManager's 4-level hierarchy
    # Populated by core/classify_media.py during metadata extraction
    # -------------------------------------------------------------------------
    "Media Group": "media_group",       # Level 1: Audio, Video, Image, Book
    "Format Class": "format_class",     # Level 2: MP3, FLAC, MP4, MKV, etc.
    "Media Class": "media_class",       # Level 3: Music, Movie, TV Show, etc.
    "Quality Type": "quality_type",     # Level 4: Lossy, Lossless

    # -------------------------------------------------------------------------
    # Audio Property Tags — Technical audio characteristics
    # Populated by core/metadata_extractor.py using pymediainfo
    # -------------------------------------------------------------------------
    "Codec": "codec",                   # Audio codec (AAC, FLAC, Vorbis, etc.)
    "Bitrate": "bitrate",              # Bitrate in kbps
    "Sample Rate": "sample_rate",       # Sample rate in Hz
    "Channels": "audio_channels",       # Number of audio channels
    "Channel Layout": "channel_layout", # Channel layout (Stereo, 5.1, 7.1)
    "Spatial Format": "spatial_format", # Spatial audio (Dolby Atmos, 360 RA)
    "Multichannel": "multichannel",     # Multichannel format (DD, DD+, DTS)
    "Bit Depth": "bit_depth",          # Bit depth (16, 24, 32)

    # -------------------------------------------------------------------------
    # File Tags — Filesystem-level metadata
    # -------------------------------------------------------------------------
    "Filename": "filename",             # Original filename without extension
    "Ext": "extension",                 # File extension without dot
    "Path": "filepath",                 # Full original file path
    "File Size": "file_size",          # File size in bytes
    "Date Added": "date_added",        # Date file was first detected
}

# ============================================================================
# Reverse Map — Internal key → display name (auto-generated from TAG_MAP)
# Used for displaying metadata in UI and reports
# ============================================================================
REVERSE_TAG_MAP = {v: k for k, v in TAG_MAP.items()}

# ============================================================================
# Custom Tag Prefix — Tags starting with this prefix are user-defined
# and have no predefined internal key. The portion after the prefix
# is used directly as the metadata key.
# Example: <Custom:SpotifyURL> → metadata key "custom_spotifyurl"
# ============================================================================
CUSTOM_TAG_PREFIX = "Custom:"


def resolve_tag(display_name, metadata):
    """
    Resolve a display tag name to its value from a metadata dictionary.

    Handles three cases:
    1. Known tag: looks up internal key via TAG_MAP, returns metadata value
    2. Custom tag: strips "Custom:" prefix, lowercases, returns metadata value
    3. Unknown tag: returns None (caller decides how to handle)

    Args:
        display_name (str): The friendly tag name (e.g., "Album Artist",
                            "Custom:SpotifyURL")
        metadata (dict): The metadata dictionary with internal keys

    Returns:
        str or None: The tag value, or None if not found in metadata
    """
    # Case 1: Custom tag with "Custom:" prefix
    if display_name.startswith(CUSTOM_TAG_PREFIX):
        # Extract the custom key portion after "Custom:"
        custom_key = display_name[len(CUSTOM_TAG_PREFIX):]
        # Convert to internal format: lowercase with underscores
        internal_key = "custom_" + custom_key.lower().replace(" ", "_")
        return metadata.get(internal_key)

    # Case 2: Known tag — look up in TAG_MAP
    internal_key = TAG_MAP.get(display_name)
    if internal_key is not None:
        return metadata.get(internal_key)

    # Case 3: Unknown tag — try case-insensitive match as fallback
    display_lower = display_name.lower()
    for tag_display, tag_key in TAG_MAP.items():
        if tag_display.lower() == display_lower:
            return metadata.get(tag_key)

    # No match found at all
    return None


def get_internal_key(display_name):
    """
    Convert a display tag name to its internal snake_case key.

    Args:
        display_name (str): The friendly tag name (e.g., "Album Artist")

    Returns:
        str or None: The internal key (e.g., "album_artist"), or None if unknown
    """
    # Handle custom tags
    if display_name.startswith(CUSTOM_TAG_PREFIX):
        custom_key = display_name[len(CUSTOM_TAG_PREFIX):]
        return "custom_" + custom_key.lower().replace(" ", "_")

    # Look up in TAG_MAP
    return TAG_MAP.get(display_name)


def get_display_name(internal_key):
    """
    Convert an internal snake_case key to its friendly display name.

    Args:
        internal_key (str): The internal key (e.g., "album_artist")

    Returns:
        str or None: The display name (e.g., "Album Artist"), or None if unknown
    """
    return REVERSE_TAG_MAP.get(internal_key)


def get_display_tags():
    """
    Return a sorted list of all known display tag names.
    Useful for populating UI dropdowns and tag selectors.

    Returns:
        list[str]: Sorted list of display names (e.g., ["Album", "Album Artist", ...])
    """
    return sorted(TAG_MAP.keys())


def is_valid_tag(display_name):
    """
    Check whether a display tag name is recognized.

    Returns True for:
    - Any tag in TAG_MAP (e.g., "Album Artist", "Track #")
    - Any tag with the "Custom:" prefix (unlimited custom tags)

    Args:
        display_name (str): The tag name to validate

    Returns:
        bool: True if the tag is known or a valid custom tag
    """
    # Custom tags are always valid (unlimited user-defined)
    if display_name.startswith(CUSTOM_TAG_PREFIX):
        # Must have at least one character after the prefix
        return len(display_name) > len(CUSTOM_TAG_PREFIX)

    # Check exact match in TAG_MAP
    return display_name in TAG_MAP
