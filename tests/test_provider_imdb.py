# ============================================================================
# File: /tests/test_provider_imdb.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the IMDb metadata provider using the cinemagoer library.
# Tests movie search with mocked cinemagoer responses, IMDb ID format,
# and provider capabilities/status checks.
#
# All tests are offline — cinemagoer calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock cinemagoer calls
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.video.imdb import IMDbProvider     # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — IMDb Provider Tests
# =============================================================================

class TestIMDbProvider:
    """Tests for the IMDbProvider class."""

    def test_provider_name(self):
        """Provider name should be 'imdb'."""
        provider = IMDbProvider()
        assert provider.provider_name == "imdb"

    def test_category(self):
        """Category should be ProviderCategory.VIDEO."""
        provider = IMDbProvider()
        assert provider.category == ProviderCategory.VIDEO

    def test_requires_auth(self):
        """Provider should not require authentication (scraping)."""
        provider = IMDbProvider()
        assert provider.requires_auth is False

    def test_capabilities(self):
        """Provider should support movies, shows, IMDb lookup, and cover art."""
        provider = IMDbProvider()
        caps = provider.capabilities
        assert caps.can_search_movies is True
        assert caps.can_search_shows is True
        assert caps.can_lookup_imdb_id is True
        assert caps.has_static_cover_art is True

    def test_is_available_when_cinemagoer_not_installed(self):
        """is_available should return False when cinemagoer is not installed."""
        provider = IMDbProvider()
        # Mock ImportError when importing cinemagoer
        with patch.dict('sys.modules', {'imdb': None}):
            with patch('builtins.__import__', side_effect=ImportError):
                assert provider.is_available() is False

    def test_is_available_when_cinemagoer_installed(self):
        """is_available should return True when cinemagoer is installed."""
        provider = IMDbProvider()
        # Mock successful import of cinemagoer
        mock_imdb = MagicMock()
        with patch.dict('sys.modules', {'imdb': mock_imdb}):
            assert provider.is_available() is True

    def test_search_with_mocked_cinemagoer(self):
        """Search should parse cinemagoer Movie objects correctly."""
        provider = IMDbProvider()

        # Create mock Movie object
        mock_movie = MagicMock()
        mock_movie.movieID = "0137523"
        mock_movie.get = MagicMock(side_effect=lambda key, default="": {
            "title": "Fight Club",
            "year": 1999,
            "rating": 8.8,
            "votes": 2000000,
            "genres": ["Drama", "Thriller"],
            "cover url": "https://m.media-amazon.com/images/M/test.jpg",
            "kind": "movie",
        }.get(key, default))

        # Mock Cinemagoer instance
        mock_ia = MagicMock()
        mock_ia.search_movie = MagicMock(return_value=[mock_movie])
        mock_ia.update = MagicMock()

        # Create a mock rate limiter
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        with patch.object(provider, '_get_cinemagoer', return_value=mock_ia):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                # Run async search
                query = {"title": "Fight Club"}
                results = asyncio.run(provider.search(query))

                # Verify results
                assert len(results) == 1
                result = results[0]
                assert result.title == "Fight Club"
                assert result.year == "1999"
                assert result.provider_id == "tt0137523"  # Should have 'tt' prefix
                assert result.genre == "Drama, Thriller"
                assert "imdb.com/title/tt0137523" in result.provider_url

                # Verify IMDb ID format
                assert result.provider_id.startswith("tt")
                assert "custom_imdb_id" in result.extra_tags
                assert result.extra_tags["custom_imdb_id"] == "tt0137523"
                assert "custom_imdb_rating" in result.extra_tags
                assert result.extra_tags["custom_imdb_rating"] == "8.8"
                assert "custom_imdb_votes" in result.extra_tags
                assert result.extra_tags["custom_imdb_votes"] == "2000000"
                assert "custom_imdb_genres" in result.extra_tags

                # Verify cover art
                assert len(result.cover_art) == 1
                cover = result.cover_art[0]
                assert cover.asset_type == CoverArtType.STATIC
                assert cover.format == "jpeg"

    def test_imdb_id_format_with_tt_prefix(self):
        """IMDb IDs should always be formatted with 'tt' prefix."""
        provider = IMDbProvider()

        # Create mock Movie object with numeric ID
        mock_movie = MagicMock()
        mock_movie.movieID = "1234567"
        mock_movie.get = MagicMock(side_effect=lambda key, default="": {
            "title": "Test Movie",
            "year": 2024,
            "kind": "movie",
        }.get(key, default))

        result = provider._parse_movie(mock_movie)
        assert result is not None
        assert result.provider_id == "tt1234567"
        assert result.extra_tags["custom_imdb_id"] == "tt1234567"
        assert result.provider_url == "https://www.imdb.com/title/tt1234567/"

    def test_tv_series_detection(self):
        """TV series should be properly identified and marked."""
        provider = IMDbProvider()

        # Create mock TV series object
        mock_series = MagicMock()
        mock_series.movieID = "0903747"
        mock_series.get = MagicMock(side_effect=lambda key, default="": {
            "title": "Breaking Bad",
            "year": 2008,
            "kind": "tv series",                           # Identifies as TV series
            "rating": 9.5,
            "genres": ["Crime", "Drama", "Thriller"],
        }.get(key, default))

        result = provider._parse_movie(mock_series)
        assert result is not None
        assert result.show == "Breaking Bad"              # Should populate show field
        assert result.title == "Breaking Bad"

    def test_empty_query_returns_empty_results(self):
        """Search with empty query should return empty results."""
        provider = IMDbProvider()
        query = {}
        results = asyncio.run(provider.search(query))
        assert results == []

    def test_lookup_by_id_with_tt_prefix(self):
        """Lookup should accept IMDb IDs with 'tt' prefix."""
        provider = IMDbProvider()

        # Create mock Movie object
        mock_movie = MagicMock()
        mock_movie.movieID = "0133093"
        mock_movie.get = MagicMock(side_effect=lambda key, default="": {
            "title": "The Matrix",
            "year": 1999,
            "kind": "movie",
        }.get(key, default))

        # Mock Cinemagoer instance
        mock_ia = MagicMock()
        mock_ia.get_movie = MagicMock(return_value=mock_movie)

        # Create a mock rate limiter
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        with patch.object(provider, '_get_cinemagoer', return_value=mock_ia):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                # Test lookup with 'tt' prefix (should be stripped)
                result = asyncio.run(provider.lookup_by_id("tt0133093"))

                # Verify that get_movie was called with numeric ID only
                mock_ia.get_movie.assert_called_once_with("0133093")
                assert result is not None
                assert result.provider_id == "tt0133093"
