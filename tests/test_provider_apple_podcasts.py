# ============================================================================
# File: /tests/test_provider_apple_podcasts.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Apple Podcasts metadata provider (iTunes Search API).
# Tests podcast episode search with mocked API responses, cover art
# extraction, and provider capabilities/status checks.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP calls
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.podcasts.apple_podcasts import ApplePodcastsProvider  # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — Apple Podcasts Provider Tests
# =============================================================================

class TestApplePodcastsProvider:
    """Tests for the ApplePodcastsProvider class."""

    def test_provider_name(self):
        """Provider name should be 'apple_podcasts'."""
        provider = ApplePodcastsProvider()
        assert provider.provider_name == "apple_podcasts"

    def test_category(self):
        """Category should be ProviderCategory.PODCAST."""
        provider = ApplePodcastsProvider()
        assert provider.category == ProviderCategory.PODCAST

    def test_requires_auth(self):
        """Provider should not require authentication (public API)."""
        provider = ApplePodcastsProvider()
        assert provider.requires_auth is False

    def test_is_available(self):
        """is_available should return True when httpx is installed."""
        provider = ApplePodcastsProvider()
        # Mock httpx module to simulate it being installed
        mock_httpx = MagicMock()
        with patch.dict('sys.modules', {'httpx': mock_httpx}):
            assert provider.is_available() is True

    def test_search_with_mocked_response(self):
        """Search should parse iTunes Search API response and return ProviderResult list."""
        provider = ApplePodcastsProvider()

        # Mock iTunes Search API response data
        mock_response_data = {
            "resultCount": 1,
            "results": [
                {
                    "wrapperType": "podcastEpisode",
                    "kind": "podcast-episode",
                    "trackId": 1234567890,
                    "trackName": "Test Episode Title",
                    "collectionName": "Test Podcast Show",
                    "artistName": "Test Podcast Author",
                    "artworkUrl600": "https://example.com/artwork600x600.jpg",
                    "artworkUrl100": "https://example.com/artwork100x100.jpg",
                    "trackViewUrl": "https://podcasts.apple.com/gb/podcast/test/id1234567890",
                    "feedUrl": "https://example.com/feed.rss",
                    "releaseDate": "2025-02-10T08:00:00Z",
                    "trackTimeMillis": 3600000,
                    "primaryGenreName": "Technology",
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
                query = {"title": "Test Episode", "show": "Test Podcast"}
                results = asyncio.run(provider.search(query))

                # Verify results
                assert len(results) == 1
                result = results[0]
                assert result.title == "Test Episode Title"
                assert result.show == "Test Podcast Show"
                assert result.artist == "Test Podcast Author"
                assert result.genre == "Technology"
                assert result.year == "2025"
                assert result.provider_id == "1234567890"
                assert result.provider_url == "https://podcasts.apple.com/gb/podcast/test/id1234567890"

                # Verify cover art
                assert len(result.cover_art) == 1
                cover = result.cover_art[0]
                assert cover.asset_type == CoverArtType.STATIC
                assert cover.url == "https://example.com/artwork600x600.jpg"
                assert cover.format == "jpeg"
                assert cover.width == 600
                assert cover.height == 600

                # Verify extra tags
                assert result.extra_tags["custom_apple_podcast_feed_url"] == "https://example.com/feed.rss"
                assert result.extra_tags["custom_apple_podcast_duration_ms"] == "3600000"

    def test_search_returns_empty_list_when_no_query(self):
        """Search should return empty list when no query terms provided."""
        provider = ApplePodcastsProvider()

        # Empty query
        query = {}
        results = asyncio.run(provider.search(query))

        assert results == []
