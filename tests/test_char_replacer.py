# ============================================================================
# File: /tests/test_char_replacer.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for utils/char_replacer.py — configurable filename character
# replacement and sanitization.
# ============================================================================

import pytest                                      # Test framework
from unittest.mock import patch                    # Mock config access
from utils.char_replacer import (
    sanitize_component,                            # Single component sanitizer
    sanitize_path,                                 # Full path sanitizer
)


# ============================================================================
# Replacement map used across tests — simulates config/settings.json5
# ============================================================================
MOCK_REPLACEMENTS = {
    "/": "-",
    "\\": "-",
    ":": "-",
    "*": "",
    "?": "",
    "\"": "'",
    "<": "",
    ">": "",
    "|": "",
}


def _mock_get_config(key, default=None):
    """
    Mock config loader that returns sensible defaults for each key.
    Used as side_effect for patch("utils.char_replacer.get_config").
    """
    if key == "replacement_char":
        return default if default is not None else "_"
    if key == "filename_replacements":
        return default if default is not None else {}
    return default


# ============================================================================
# sanitize_component() tests
# ============================================================================

@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_basic_text(mock_config):
    """Clean text passes through unchanged."""
    assert sanitize_component("Queen", {}) == "Queen"


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_colon_replacement(mock_config):
    """Colons are replaced per configured mapping."""
    result = sanitize_component("AC:DC", MOCK_REPLACEMENTS)
    assert result == "AC-DC"                       # Colon replaced with dash


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_multiple_replacements(mock_config):
    """Multiple unsafe characters are each replaced according to mapping."""
    result = sanitize_component('File: "Test" <v2>', MOCK_REPLACEMENTS)
    # : → -, " → ', < → "", > → ""
    assert result == "File- 'Test' v2"


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_none_input(mock_config):
    """None input returns 'Unknown'."""
    assert sanitize_component(None, {}) == "Unknown"


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_empty_input(mock_config):
    """Empty string input returns 'Unknown'."""
    assert sanitize_component("", {}) == "Unknown"


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_all_stripped(mock_config):
    """If all characters are unsafe, returns 'Unknown'."""
    # All chars in this string are stripped by the replacements + unsafe pattern
    result = sanitize_component("???", MOCK_REPLACEMENTS)
    # ? → "" per replacements, so result is empty → "Unknown"
    assert result == "Unknown"


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_strips_dots(mock_config):
    """Leading and trailing dots are stripped (unsafe on Windows)."""
    result = sanitize_component("..hidden..", {})
    assert result == "hidden"


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_control_chars(mock_config):
    """Control characters (0x00-0x1F) are stripped by fallback pattern."""
    result = sanitize_component("Hello\x00World\x1F", {})
    assert result == "Hello_World_"


# ============================================================================
# sanitize_path() tests
# ============================================================================

@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_path_simple(mock_config):
    """Simple path with clean components passes through."""
    result = sanitize_path("Music/Queen/Album", {})
    assert result == "Music/Queen/Album"


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_path_with_replacements(mock_config):
    """Each path component is sanitized independently."""
    result = sanitize_path("Rock/AC:DC/Back in Black", MOCK_REPLACEMENTS)
    assert result == "Rock/AC-DC/Back in Black"


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_path_none(mock_config):
    """None path returns 'Unknown'."""
    assert sanitize_path(None, {}) == "Unknown"


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_path_empty(mock_config):
    """Empty path returns 'Unknown'."""
    assert sanitize_path("", {}) == "Unknown"


@patch("utils.char_replacer.get_config", side_effect=_mock_get_config)
def test_sanitize_path_preserves_structure(mock_config):
    """Path structure (separators) is preserved after sanitization."""
    result = sanitize_path("A/B/C/D.mp3", {})
    # Each component sanitized, slashes preserved
    assert result == "A/B/C/D.mp3"
    assert result.count("/") == 3                  # Three separators preserved
