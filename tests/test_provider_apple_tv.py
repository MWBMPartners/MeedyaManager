# ============================================================================
# File: /tests/test_provider_apple_tv.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Apple TV metadata provider (iTunes Search API).
# Tests movie and TV episode search with mocked API responses,
# artwork URL scaling, and provider capabilities/status checks.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP calls
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.video.apple_tv import AppleTVProvider  # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — Apple TV Provider Tests
# =============================================================================

class TestAppleTVProvider:
    """Tests for the AppleTVProvider class."""

    def test_provider_name(self):
        """Provider name should be 'apple_tv'."""
        provider = AppleTVProvider()
        assert provider.provider_name == "apple_tv"

    def test_category(self):
        """Category should be ProviderCategory.VIDEO."""
        provider = AppleTVProvider()
        assert provider.category == ProviderCategory.VIDEO

    def test_requires_auth(self):
        """Provider should not require authentication (public API)."""
        provider = AppleTVProvider()
        assert provider.requires_auth is False

    def test_is_available(self):
        """is_available should return True when httpx is installed."""
        provider = AppleTVProvider()
        # Mock httpx module to simulate it being installed
        mock_httpx = MagicMock()
        with patch.dict('sys.modules', {'httpx': mock_httpx}):
            assert provider.is_available() is True

    def test_movie_search_with_mocked_response(self):
        """Movie search should parse iTunes Search API response correctly."""
        provider = AppleTVProvider()

        # Mock iTunes Search API response for a movie
        mock_response_data = {
            "resultCount": 1,
            "results": [
                {
                    "wrapperType": "track",
                    "kind": "feature-movie",
                    "trackId": 555666777,
                    "trackName": "Test Movie Title",
                    "artistName": "Test Director",
                    "primaryGenreName": "Action & Adventure",
                    "trackViewUrl": "https://tv.apple.com/gb/movie/test/555666777",
                    "artworkUrl100": "https://example.com/100x100bb.jpg",
                    "releaseDate": "2023-11-20T00:00:00Z",
                    "longDescription": "A thrilling action movie about testing.",
                    "contentAdvisoryRating": "PG-13",
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

        with patch.object(provider, '_get_http_client', return_value=mock_client):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                # Run async search
                query = {"title": "Test Movie", "media_class": "Movie"}
                results = asyncio.run(provider.search(query))

                # Verify results
                assert len(results) == 1
                result = results[0]
                assert result.title == "Test Movie Title"
                assert result.director == "Test Director"
                assert result.genre == "Action & Adventure"
                assert result.year == "2023"
                assert result.provider_id == "555666777"
                assert result.provider_url == "https://tv.apple.com/gb/movie/test/555666777"

                # Verify cover art (should be scaled to 3000x3000)
                assert len(result.cover_art) == 1
                cover = result.cover_art[0]
                assert cover.asset_type == CoverArtType.STATIC
                assert "3000x3000bb.jpg" in cover.url
                assert cover.format == "jpeg"
                assert cover.width == 3000
                assert cover.height == 3000

                # Verify extra tags
                assert "custom_apple_tv_description" in result.extra_tags
                assert "thrilling action" in result.extra_tags["custom_apple_tv_description"]
                assert result.extra_tags["custom_apple_tv_rating"] == "PG-13"

    def test_tv_episode_search_with_mocked_response(self):
        """TV episode search should parse iTunes Search API response correctly."""
        provider = AppleTVProvider()

        # Mock iTunes Search API response for a TV episode
        mock_response_data = {
            "resultCount": 1,
            "results": [
                {
                    "wrapperType": "track",
                    "kind": "tv-episode",
                    "trackId": 888999000,
                    "trackName": "Test Episode Title",
                    "artistName": "Test TV Show Name",
                    "collectionName": "Test TV Show Name, Season 1",
                    "primaryGenreName": "Drama",
                    "trackViewUrl": "https://tv.apple.com/gb/episode/test/888999000",
                    "artworkUrl100": "https://example.com/100x100bb.jpg",
                    "releaseDate": "2024-03-15T20:00:00Z",
                    "longDescription": "An exciting episode of the test show.",
                    "contentAdvisoryRating": "TV-14",
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

        with patch.object(provider, '_get_http_client', return_value=mock_client):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                # Run async search with TV context
                query = {"title": "Test Episode", "show": "Test TV Show"}
                results = asyncio.run(provider.search(query))

                # Verify results
                assert len(results) == 1
                result = results[0]
                assert result.episode_title == "Test Episode Title"
                assert result.show == "Test TV Show Name"
                assert result.artist == "Test TV Show Name"  # Artist is show name for episodes
                assert result.season == "1"  # Extracted from "Season 1"
                assert result.genre == "Drama"
                assert result.year == "2024"
                assert result.provider_id == "888999000"
                assert result.provider_url == "https://tv.apple.com/gb/episode/test/888999000"

                # Verify cover art (should be scaled to 3000x3000)
                assert len(result.cover_art) == 1
                cover = result.cover_art[0]
                assert cover.asset_type == CoverArtType.STATIC
                assert "3000x3000bb.jpg" in cover.url
                assert cover.format == "jpeg"

                # Verify extra tags
                assert "custom_apple_tv_description" in result.extra_tags
                assert result.extra_tags["custom_apple_tv_rating"] == "TV-14"
