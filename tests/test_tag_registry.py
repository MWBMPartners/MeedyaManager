# ============================================================================
# File: /tests/test_tag_registry.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for core/tag_registry.py — the bidirectional mapping between
# display tag names and internal metadata keys.
# ============================================================================

import pytest                                      # Test framework
from core.tag_registry import (
    TAG_MAP,                                       # Display → internal mapping
    REVERSE_TAG_MAP,                               # Internal → display mapping
    resolve_tag,                                   # Resolve tag to metadata value
    get_internal_key,                              # Display name → internal key
    get_display_name,                              # Internal key → display name
    get_display_tags,                              # Sorted list of all display names
    is_valid_tag,                                  # Check if tag name is recognized
)


# ============================================================================
# Sample metadata dict for testing tag resolution
# ============================================================================
SAMPLE_METADATA = {
    "title": "Bohemian Rhapsody",
    "artist": "Queen",
    "album": "A Night at the Opera",
    "album_artist": "Queen",
    "year": "1975",
    "genre": "Rock",
    "track_num": "11",
    "media_group": "Audio",
    "format_class": "flac",
    "media_class": "Music",
    "quality_type": "Lossless",
    "extension": "flac",
    "filepath": "/music/queen/song.flac",
    "audio_channels": "2",
    "custom_spotifyurl": "https://open.spotify.com/track/abc123",
}


# ============================================================================
# resolve_tag() tests
# ============================================================================

def test_resolve_known_tag():
    """Resolving a standard tag returns the correct metadata value."""
    assert resolve_tag("Title", SAMPLE_METADATA) == "Bohemian Rhapsody"


def test_resolve_album_artist():
    """Resolving 'Album Artist' maps to 'album_artist' key."""
    assert resolve_tag("Album Artist", SAMPLE_METADATA) == "Queen"


def test_resolve_classification_tag():
    """Classification tags like 'Media Class' resolve correctly."""
    assert resolve_tag("Media Class", SAMPLE_METADATA) == "Music"


def test_resolve_custom_tag():
    """Custom tags with 'Custom:' prefix resolve to custom_ prefixed keys."""
    assert resolve_tag("Custom:SpotifyURL", SAMPLE_METADATA) == "https://open.spotify.com/track/abc123"


def test_resolve_unknown_tag():
    """An unknown tag name returns None."""
    assert resolve_tag("NonExistentTag", SAMPLE_METADATA) is None


def test_resolve_missing_metadata_key():
    """A known tag with no corresponding metadata value returns None."""
    assert resolve_tag("BPM", SAMPLE_METADATA) is None


def test_resolve_case_insensitive_fallback():
    """Tag resolution falls back to case-insensitive match."""
    # "title" instead of "Title" — should still resolve via fallback
    assert resolve_tag("title", SAMPLE_METADATA) == "Bohemian Rhapsody"


# ============================================================================
# get_internal_key() tests
# ============================================================================

def test_get_internal_key_known():
    """Known display name returns the correct internal key."""
    assert get_internal_key("Album Artist") == "album_artist"


def test_get_internal_key_track():
    """Track # maps to track_num."""
    assert get_internal_key("Track #") == "track_num"


def test_get_internal_key_custom():
    """Custom tags generate internal key from the custom name."""
    assert get_internal_key("Custom:MusicBrainzID") == "custom_musicbrainzid"


def test_get_internal_key_unknown():
    """Unknown display name returns None."""
    assert get_internal_key("FakeTag") is None


# ============================================================================
# get_display_name() tests
# ============================================================================

def test_get_display_name_known():
    """Known internal key returns the correct display name."""
    assert get_display_name("album_artist") == "Album Artist"


def test_get_display_name_unknown():
    """Unknown internal key returns None."""
    assert get_display_name("nonexistent_key") is None


# ============================================================================
# get_display_tags() tests
# ============================================================================

def test_display_tags_returns_sorted_list():
    """get_display_tags() returns a sorted list of all known display names."""
    tags = get_display_tags()
    assert isinstance(tags, list)
    assert tags == sorted(tags)                    # Must be sorted alphabetically
    assert len(tags) == len(TAG_MAP)               # One entry per TAG_MAP key


def test_display_tags_contains_expected():
    """The display tags list contains key expected entries."""
    tags = get_display_tags()
    assert "Title" in tags
    assert "Album Artist" in tags
    assert "Track #" in tags
    assert "Media Class" in tags
    assert "Ext" in tags


# ============================================================================
# is_valid_tag() tests
# ============================================================================

def test_valid_known_tag():
    """Known tags are valid."""
    assert is_valid_tag("Title") is True
    assert is_valid_tag("Album Artist") is True
    assert is_valid_tag("Quality Type") is True


def test_valid_custom_tag():
    """Custom tags with 'Custom:' prefix are always valid."""
    assert is_valid_tag("Custom:SpotifyURL") is True
    assert is_valid_tag("Custom:MyRating") is True


def test_invalid_empty_custom_tag():
    """Custom tag with no name after prefix is invalid."""
    assert is_valid_tag("Custom:") is False


def test_invalid_unknown_tag():
    """Unknown tags are not valid."""
    assert is_valid_tag("FakeTag") is False
    assert is_valid_tag("") is False


# ============================================================================
# Reverse map consistency test
# ============================================================================

def test_reverse_map_consistency():
    """Every TAG_MAP entry has a corresponding REVERSE_TAG_MAP entry."""
    for display, internal in TAG_MAP.items():
        assert REVERSE_TAG_MAP[internal] == display
