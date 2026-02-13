# ============================================================================
# File: /tests/test_multi_value.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the multi-value field handling module (metadata/multi_value.py).
# Covers parsing, formatting, and field classification.
# ============================================================================

import pytest                                          # Test framework

from metadata.multi_value import (
    parse_multi_value,
    format_multi_value,
    is_multi_value_field,
    MULTI_VALUE_SEPARATOR,
)


# =============================================================================
# parse_multi_value Tests
# =============================================================================

class TestParseMultiValue:
    """Tests for converting various input formats into lists of strings."""

    def test_parse_semicolon_delimited_string(self):
        """Semicolon-separated string should be split into individual values."""
        result = parse_multi_value("Rock; Pop; Jazz")
        assert result == ["Rock", "Pop", "Jazz"]

    def test_parse_list_input(self):
        """List input should be returned with items stripped of whitespace."""
        result = parse_multi_value(["Rock", " Pop ", "Jazz"])
        assert result == ["Rock", "Pop", "Jazz"]

    def test_parse_single_string(self):
        """Single string without semicolons should become a one-element list."""
        result = parse_multi_value("Rock")
        assert result == ["Rock"]

    def test_parse_none_returns_empty(self):
        """None input should return an empty list."""
        result = parse_multi_value(None)
        assert result == []

    def test_parse_empty_string_returns_empty(self):
        """Empty string should return an empty list."""
        result = parse_multi_value("")
        assert result == []

    def test_parse_whitespace_only_returns_empty(self):
        """Whitespace-only string should return an empty list."""
        result = parse_multi_value("   ")
        assert result == []

    def test_parse_filters_empty_values(self):
        """Empty values between semicolons should be filtered out."""
        result = parse_multi_value("Rock; ; ; Jazz")
        assert result == ["Rock", "Jazz"]

    def test_parse_list_filters_empty_strings(self):
        """Empty strings in list input should be filtered out."""
        result = parse_multi_value(["Rock", "", "  ", "Jazz"])
        assert result == ["Rock", "Jazz"]

    def test_parse_numeric_in_list(self):
        """Numeric values in a list should be converted to strings."""
        result = parse_multi_value([1, 2, 3])
        assert result == ["1", "2", "3"]


# =============================================================================
# format_multi_value Tests
# =============================================================================

class TestFormatMultiValue:
    """Tests for joining lists of values into semicolon-delimited strings."""

    def test_format_multiple_values(self):
        """Multiple values should be joined with semicolons and spaces."""
        result = format_multi_value(["Rock", "Pop", "Jazz"])
        assert result == "Rock ; Pop ; Jazz"

    def test_format_single_value(self):
        """Single value should be returned without separator."""
        result = format_multi_value(["Rock"])
        assert result == "Rock"

    def test_format_empty_list(self):
        """Empty list should return empty string."""
        result = format_multi_value([])
        assert result == ""

    def test_format_none_input(self):
        """None input should return empty string."""
        result = format_multi_value(None)
        assert result == ""

    def test_format_filters_empty_strings(self):
        """Empty strings in the list should be filtered out."""
        result = format_multi_value(["Rock", "", "Jazz"])
        assert result == "Rock ; Jazz"

    def test_format_strips_whitespace(self):
        """Values should be stripped of leading/trailing whitespace."""
        result = format_multi_value(["  Rock  ", " Jazz "])
        assert result == "Rock ; Jazz"


# =============================================================================
# is_multi_value_field Tests
# =============================================================================

class TestIsMultiValueField:
    """Tests for identifying fields that commonly hold multiple values."""

    @pytest.mark.parametrize("field", ["artist", "genre", "composer", "album_artist"])
    def test_known_multi_value_fields(self, field):
        """Known multi-value fields should return True."""
        assert is_multi_value_field(field) is True

    @pytest.mark.parametrize("field", ["title", "album", "year", "track_num", "bpm"])
    def test_single_value_fields(self, field):
        """Fields that are typically single-value should return False."""
        assert is_multi_value_field(field) is False

    def test_unknown_field_returns_false(self):
        """Unknown field names should return False."""
        assert is_multi_value_field("nonexistent_field") is False
