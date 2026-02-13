# ============================================================================
# File: /tests/test_provider_tmdb.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the TMDB (The Movie Database) metadata provider.
# Tests movie and TV show search with mocked API responses,
# poster URL construction, and provider capabilities/status checks.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP calls
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.video.tmdb import TMDBProvider     # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — TMDB Provider Tests
# =============================================================================

class TestTMDBProvider:
    """Tests for the TMDBProvider class."""

    def test_provider_name(self):
        """Provider name should be 'tmdb'."""
        provider = TMDBProvider()
        assert provider.provider_name == "tmdb"

    def test_category(self):
        """Category should be ProviderCategory.VIDEO."""
        provider = TMDBProvider()
        assert provider.category == ProviderCategory.VIDEO

    def test_requires_auth(self):
        """Provider should require authentication (API key)."""
        provider = TMDBProvider()
        assert provider.requires_auth is True

    def test_capabilities(self):
        """Provider should support movies, shows, episodes, IMDb lookup, and cover art."""
        provider = TMDBProvider()
        caps = provider.capabilities
        assert caps.can_search_movies is True
        assert caps.can_search_shows is True
        assert caps.can_search_episodes is True
        assert caps.can_lookup_imdb_id is True
        assert caps.has_static_cover_art is True
        assert caps.has_cast_crew is True

    def test_is_available_without_api_key(self):
        """is_available should return False when API key is missing."""
        provider = TMDBProvider()
        # Mock credential manager to return None
        with patch.object(provider._credential_manager, 'get_credential', return_value=None):
            assert provider.is_available() is False

    def test_is_available_with_api_key(self):
        """is_available should return True when API key is present."""
        provider = TMDBProvider()
        # Mock httpx module and credential manager
        mock_httpx = MagicMock()
        with patch.dict('sys.modules', {'httpx': mock_httpx}):
            with patch.object(provider._credential_manager, 'get_credential', return_value='fake_api_key'):
                assert provider.is_available() is True

    def test_movie_search_with_mocked_response(self):
        """Movie search should parse TMDB API response correctly."""
        provider = TMDBProvider()

        # Mock TMDB API response for a movie
        mock_response_data = {
            "results": [
                {
                    "id": 550,
                    "title": "Fight Club",
                    "overview": "A ticking-time-bomb insomniac and a slippery soap salesman...",
                    "release_date": "1999-10-15",
                    "vote_average": 8.4,
                    "poster_path": "/pB8BM7pdSp6B6Ih7QZ4DrQ3PmJK.jpg",
                    "genre_ids": [18, 53],
                }
            ]
        }

        # Mock HTTP response
        mock_response = MagicMock()
        mock_response.json.return_value = mock_response_data
        mock_response.raise_for_status = MagicMock()

        # Mock HTTP client - use AsyncMock for async context manager
        mock_client = AsyncMock()
        mock_client.get = AsyncMock(return_value=mock_response)

        # Create a mock rate limiter with async acquire
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        # Mock credential manager
        with patch.object(provider, '_get_http_client', return_value=mock_client):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                with patch.object(provider._credential_manager, 'get_credential', return_value='fake_api_key'):
                    # Run async search
                    query = {"title": "Fight Club", "media_class": "Movie"}
                    results = asyncio.run(provider.search(query))

                    # Verify results
                    assert len(results) == 1
                    result = results[0]
                    assert result.title == "Fight Club"
                    assert result.year == "1999"
                    assert result.provider_id == "550"
                    assert "themoviedb.org/movie/550" in result.provider_url

                    # Verify cover art URL construction
                    assert len(result.cover_art) == 1
                    cover = result.cover_art[0]
                    assert cover.asset_type == CoverArtType.STATIC
                    assert "image.tmdb.org/t/p/original" in cover.url
                    assert "/pB8BM7pdSp6B6Ih7QZ4DrQ3PmJK.jpg" in cover.url
                    assert cover.format == "jpeg"

                    # Verify extra tags
                    assert "custom_tmdb_id" in result.extra_tags
                    assert result.extra_tags["custom_tmdb_id"] == "550"
                    assert "custom_tmdb_url" in result.extra_tags
                    assert "custom_tmdb_overview" in result.extra_tags
                    assert "custom_tmdb_rating" in result.extra_tags
                    assert result.extra_tags["custom_tmdb_rating"] == "8.4"

    def test_tv_search_with_mocked_response(self):
        """TV search should parse TMDB API response correctly."""
        provider = TMDBProvider()

        # Mock TMDB API response for a TV show
        mock_response_data = {
            "results": [
                {
                    "id": 1396,
                    "name": "Breaking Bad",
                    "overview": "A high school chemistry teacher diagnosed with cancer...",
                    "first_air_date": "2008-01-20",
                    "vote_average": 8.9,
                    "poster_path": "/ggFHVNu6YYI5L9pCfOacjizRGt.jpg",
                    "genre_ids": [18, 80],
                }
            ]
        }

        # Mock HTTP response
        mock_response = MagicMock()
        mock_response.json.return_value = mock_response_data
        mock_response.raise_for_status = MagicMock()

        # Mock HTTP client - use AsyncMock for async context manager
        mock_client = AsyncMock()
        mock_client.get = AsyncMock(return_value=mock_response)

        # Create a mock rate limiter with async acquire
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        # Mock credential manager
        with patch.object(provider, '_get_http_client', return_value=mock_client):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                with patch.object(provider._credential_manager, 'get_credential', return_value='fake_api_key'):
                    # Run async search with TV context
                    query = {"title": "Breaking Bad", "show": "Breaking Bad"}
                    results = asyncio.run(provider.search(query))

                    # Verify results
                    assert len(results) == 1
                    result = results[0]
                    assert result.title == "Breaking Bad"
                    assert result.show == "Breaking Bad"
                    assert result.year == "2008"
                    assert result.provider_id == "1396"
                    assert "themoviedb.org/tv/1396" in result.provider_url

                    # Verify cover art
                    assert len(result.cover_art) == 1
                    cover = result.cover_art[0]
                    assert cover.asset_type == CoverArtType.STATIC
                    assert "image.tmdb.org/t/p/original" in cover.url

                    # Verify extra tags
                    assert result.extra_tags["custom_tmdb_id"] == "1396"
                    assert "themoviedb.org/tv/1396" in result.extra_tags["custom_tmdb_url"]

    def test_poster_url_construction(self):
        """Poster URLs should use TMDB original image size."""
        provider = TMDBProvider()

        # Create a minimal movie item with poster_path
        item = {
            "id": 123,
            "title": "Test Movie",
            "poster_path": "/test123.jpg",
            "release_date": "2023-01-01",
            "overview": "Test",
            "vote_average": 7.5,
        }

        result = provider._parse_movie(item)
        assert result is not None
        assert len(result.cover_art) == 1
        assert result.cover_art[0].url == "https://image.tmdb.org/t/p/original/test123.jpg"

    def test_empty_query_returns_empty_results(self):
        """Search with empty query should return empty results."""
        provider = TMDBProvider()
        query = {}
        results = asyncio.run(provider.search(query))
        assert results == []
