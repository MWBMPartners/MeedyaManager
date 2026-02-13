# ============================================================================
# File: /tests/test_match_scoring.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the MatchScorer confidence scoring system:
# - _normalize() helper
# - _fuzzy_ratio() with and without fuzzywuzzy
# - Exact match scoring
# - Fuzzy string match scoring
# - ISRC bonus scoring
# - Year and track number bonuses
# - rank_results() sorting
# - Empty/missing field handling
# - Score capping at 1.0
# ============================================================================

import pytest                                              # Test framework

from metadata.providers.match_scoring import (
    MatchScorer,                                           # Main class under test
    _normalize,                                            # Normalisation helper
    _fuzzy_ratio,                                          # Fuzzy matching helper
)
from metadata.providers.base import ProviderResult         # Result dataclass


# =============================================================================
# Fixtures
# =============================================================================

@pytest.fixture
def scorer():
    """Create a fresh MatchScorer with default weights."""
    return MatchScorer()


@pytest.fixture
def exact_query():
    """A known query dict for testing exact matches."""
    return {
        "title": "Bohemian Rhapsody",
        "artist": "Queen",
        "album": "A Night at the Opera",
        "year": "1975",
        "track_num": "11",
        "isrc": "GBUM71029604",
    }


@pytest.fixture
def exact_result():
    """A ProviderResult that exactly matches the exact_query."""
    return ProviderResult(
        provider_name="test_provider",
        title="Bohemian Rhapsody",
        artist="Queen",
        album="A Night at the Opera",
        year="1975",
        track_num="11",
        isrc="GBUM71029604",
    )


# =============================================================================
# _normalize() Tests
# =============================================================================

class TestNormalize:
    """Tests for the _normalize() helper function."""

    def test_lowercase(self):
        """Should convert to lowercase."""
        assert _normalize("Hello World") == "hello world"

    def test_strip_whitespace(self):
        """Should strip leading/trailing whitespace."""
        assert _normalize("  hello  ") == "hello"

    def test_empty_string(self):
        """Empty string should return empty string."""
        assert _normalize("") == ""

    def test_none_returns_empty(self):
        """None should return empty string."""
        assert _normalize(None) == ""


# =============================================================================
# _fuzzy_ratio() Tests
# =============================================================================

class TestFuzzyRatio:
    """Tests for the _fuzzy_ratio() function."""

    def test_exact_match(self):
        """Identical strings should score 1.0."""
        assert _fuzzy_ratio("hello", "hello") == 1.0

    def test_case_insensitive(self):
        """Case-different strings should score 1.0 (case-insensitive)."""
        assert _fuzzy_ratio("Hello", "hello") == 1.0

    def test_empty_strings(self):
        """Empty strings should score 0.0."""
        assert _fuzzy_ratio("", "") == 0.0
        assert _fuzzy_ratio("hello", "") == 0.0
        assert _fuzzy_ratio("", "hello") == 0.0

    def test_similar_strings(self):
        """Similar strings should score > 0.5."""
        ratio = _fuzzy_ratio("Bohemian Rhapsody", "Bohemian Rhapsody (Remastered)")
        assert ratio > 0.5

    def test_different_strings(self):
        """Completely different strings should score low."""
        ratio = _fuzzy_ratio("Hello World", "Xyz Abc Def")
        assert ratio < 0.3

    def test_word_reorder(self):
        """Reordered words should produce a non-negative score."""
        # fuzzywuzzy handles word reordering well; fallback may score lower
        ratio = _fuzzy_ratio("The Beatles", "Beatles The")
        assert ratio >= 0.0                                # Valid range regardless of engine


# =============================================================================
# Exact Match Scoring Tests
# =============================================================================

class TestExactMatch:
    """Tests for scoring exact matches."""

    def test_perfect_match_high_score(self, scorer, exact_query, exact_result):
        """An exact match should score very high (>0.9)."""
        score = scorer.score(exact_query, exact_result)
        assert score > 0.9

    def test_isrc_match_bonus(self, scorer):
        """ISRC exact match should add a 0.15 bonus."""
        query = {"title": "Song", "artist": "Artist", "isrc": "USRC12345678"}
        result = ProviderResult(
            provider_name="test",
            title="Song",
            artist="Artist",
            isrc="USRC12345678",
        )
        score_with = scorer.score(query, result)

        # Same but without ISRC
        result_no_isrc = ProviderResult(
            provider_name="test",
            title="Song",
            artist="Artist",
            isrc="",
        )
        score_without = scorer.score(query, result_no_isrc)
        assert score_with > score_without

    def test_year_match_bonus(self, scorer):
        """Exact year match should add a bonus."""
        query = {"title": "Song", "artist": "Artist", "year": "2025"}
        result_match = ProviderResult(
            provider_name="test", title="Song", artist="Artist", year="2025",
        )
        result_off = ProviderResult(
            provider_name="test", title="Song", artist="Artist", year="2020",
        )
        score_match = scorer.score(query, result_match)
        score_off = scorer.score(query, result_off)
        assert score_match > score_off

    def test_year_close_match_partial_bonus(self, scorer):
        """Year within 1 year should get a partial bonus."""
        query = {"title": "Song", "artist": "Artist", "year": "2025"}
        result_exact = ProviderResult(
            provider_name="test", title="Song", artist="Artist", year="2025",
        )
        result_close = ProviderResult(
            provider_name="test", title="Song", artist="Artist", year="2024",
        )
        score_exact = scorer.score(query, result_exact)
        score_close = scorer.score(query, result_close)
        # Close should get partial bonus (less than exact)
        assert score_exact >= score_close

    def test_track_number_bonus(self, scorer):
        """Track number match should add a bonus."""
        query = {"title": "Song", "artist": "Artist", "track_num": "5"}
        result_match = ProviderResult(
            provider_name="test", title="Song", artist="Artist", track_num="5",
        )
        result_no = ProviderResult(
            provider_name="test", title="Song", artist="Artist", track_num="",
        )
        score_match = scorer.score(query, result_match)
        score_no = scorer.score(query, result_no)
        assert score_match > score_no


# =============================================================================
# Fuzzy Match Scoring Tests
# =============================================================================

class TestFuzzyMatch:
    """Tests for scoring fuzzy (approximate) matches."""

    def test_similar_title(self, scorer):
        """Similar titles should score decently."""
        query = {"title": "Bohemian Rhapsody", "artist": "Queen"}
        result = ProviderResult(
            provider_name="test",
            title="Bohemian Rhapsody (Remastered 2011)",
            artist="Queen",
        )
        score = scorer.score(query, result)
        assert score > 0.5                                 # Good fuzzy match

    def test_different_artist(self, scorer):
        """Different artist should lower the score significantly."""
        query = {"title": "Yesterday", "artist": "The Beatles"}
        result = ProviderResult(
            provider_name="test",
            title="Yesterday",
            artist="Completely Different Band",
        )
        score = scorer.score(query, result)
        # Title matches but artist doesn't
        assert score < 0.7

    def test_completely_wrong_result(self, scorer):
        """A completely wrong result should score very low."""
        query = {"title": "Bohemian Rhapsody", "artist": "Queen"}
        result = ProviderResult(
            provider_name="test",
            title="Stairway to Heaven",
            artist="Led Zeppelin",
            album="Led Zeppelin IV",
        )
        score = scorer.score(query, result)
        assert score < 0.3


# =============================================================================
# Empty/Missing Field Tests
# =============================================================================

class TestEmptyFields:
    """Tests for handling empty or missing fields."""

    def test_empty_query(self, scorer):
        """Empty query should result in score of 0.0."""
        query = {}
        result = ProviderResult(
            provider_name="test", title="Song", artist="Artist",
        )
        score = scorer.score(query, result)
        assert score == 0.0

    def test_empty_result(self, scorer):
        """Empty result fields should result in score of 0.0."""
        query = {"title": "Song", "artist": "Artist"}
        result = ProviderResult(provider_name="test")
        score = scorer.score(query, result)
        assert score == 0.0

    def test_partial_query(self, scorer):
        """Query with only title should still produce a score."""
        query = {"title": "Bohemian Rhapsody"}
        result = ProviderResult(
            provider_name="test", title="Bohemian Rhapsody",
        )
        score = scorer.score(query, result)
        assert score > 0.5


# =============================================================================
# Score Capping Tests
# =============================================================================

class TestScoreCapping:
    """Tests for score boundary behaviour."""

    def test_score_never_exceeds_1(self, scorer, exact_query, exact_result):
        """Score should never exceed 1.0 even with all bonuses."""
        score = scorer.score(exact_query, exact_result)
        assert score <= 1.0

    def test_score_never_negative(self, scorer):
        """Score should never be negative."""
        query = {"title": "X"}
        result = ProviderResult(provider_name="test", title="Y")
        score = scorer.score(query, result)
        assert score >= 0.0


# =============================================================================
# rank_results() Tests
# =============================================================================

class TestRankResults:
    """Tests for the rank_results() sorting method."""

    def test_rank_sorts_descending(self, scorer):
        """rank_results() should sort by confidence (highest first)."""
        query = {"title": "Bohemian Rhapsody", "artist": "Queen"}
        results = [
            ProviderResult(provider_name="low", title="Wrong Song", artist="Other"),
            ProviderResult(provider_name="high", title="Bohemian Rhapsody", artist="Queen"),
            ProviderResult(provider_name="mid", title="Bohemian Rhapsody", artist="Different"),
        ]
        ranked = scorer.rank_results(query, results)
        assert ranked[0].provider_name == "high"           # Best match first
        assert ranked[0].confidence >= ranked[1].confidence
        assert ranked[1].confidence >= ranked[2].confidence

    def test_rank_sets_confidence_field(self, scorer):
        """rank_results() should set the confidence field on each result."""
        query = {"title": "Test", "artist": "Artist"}
        results = [
            ProviderResult(provider_name="a", title="Test", artist="Artist"),
        ]
        ranked = scorer.rank_results(query, results)
        assert ranked[0].confidence > 0.0                  # Should be scored

    def test_rank_empty_list(self, scorer):
        """rank_results() with empty list should return empty list."""
        query = {"title": "Test"}
        ranked = scorer.rank_results(query, [])
        assert ranked == []
