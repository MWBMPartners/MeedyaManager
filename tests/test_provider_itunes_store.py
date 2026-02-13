# ============================================================================
# File: /tests/test_provider_itunes_store.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the iTunes Store metadata provider (iTunes Search API).
# Tests music track search with mocked API responses, artwork URL
# scaling (100x100bb → 3000x3000bb), and provider capabilities/status checks.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP calls
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.video.itunes_store import iTunesStoreProvider  # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — iTunes Store Provider Tests
# =============================================================================

class TestiTunesStoreProvider:
    """Tests for the iTunesStoreProvider class."""

    def test_provider_name(self):
        """Provider name should be 'itunes_store'."""
        provider = iTunesStoreProvider()
        assert provider.provider_name == "itunes_store"

    def test_category(self):
        """Category should be ProviderCategory.VIDEO."""
        provider = iTunesStoreProvider()
        assert provider.category == ProviderCategory.VIDEO

    def test_requires_auth(self):
        """Provider should not require authentication (public API)."""
        provider = iTunesStoreProvider()
        assert provider.requires_auth is False

    def test_is_available(self):
        """is_available should return True when httpx is installed."""
        provider = iTunesStoreProvider()
        # Mock httpx module to simulate it being installed
        mock_httpx = MagicMock()
        with patch.dict('sys.modules', {'httpx': mock_httpx}):
            assert provider.is_available() is True

    def test_search_with_mocked_response(self):
        """Search should parse iTunes Search API response and return ProviderResult list."""
        provider = iTunesStoreProvider()

        # Mock iTunes Search API response data
        mock_response_data = {
            "resultCount": 1,
            "results": [
                {
                    "wrapperType": "track",
                    "kind": "song",
                    "trackId": 987654321,
                    "trackName": "Test Track",
                    "artistName": "Test Artist",
                    "collectionName": "Test Album",
                    "primaryGenreName": "Alternative",
                    "trackViewUrl": "https://music.apple.com/gb/album/test/987654321",
                    "artworkUrl100": "https://example.com/100x100bb.jpg",
                    "releaseDate": "2024-06-15T12:00:00Z",
                    "trackNumber": 5,
                    "discNumber": 1,
                    "trackCount": 12,
                    "collectionId": 111222333,
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
                query = {"title": "Test Track", "artist": "Test Artist"}
                results = asyncio.run(provider.search(query))

                # Verify results
                assert len(results) == 1
                result = results[0]
                assert result.title == "Test Track"
                assert result.artist == "Test Artist"
                assert result.album == "Test Album"
                assert result.genre == "Alternative"
                assert result.year == "2024"
                assert result.track_num == "5"
                assert result.disc_num == "1"
                assert result.total_tracks == "12"
                assert result.provider_id == "987654321"
                assert result.provider_url == "https://music.apple.com/gb/album/test/987654321"

                # Verify cover art scaling
                assert len(result.cover_art) == 1
                cover = result.cover_art[0]
                assert cover.asset_type == CoverArtType.STATIC
                assert "3000x3000bb.jpg" in cover.url  # Should be scaled from 100x100bb
                assert cover.format == "jpeg"
                assert cover.width == 3000
                assert cover.height == 3000

                # Verify extra tags
                assert result.extra_tags["custom_itunes_collection_id"] == "111222333"

    def test_artwork_url_scaling(self):
        """Artwork URL should be scaled from 100x100bb to 3000x3000bb."""
        provider = iTunesStoreProvider()

        # Mock item data with artwork URL
        item = {
            "trackName": "Test Song",
            "artistName": "Test Artist",
            "collectionName": "Test Album",
            "primaryGenreName": "Pop",
            "trackId": 123456,
            "trackViewUrl": "https://example.com/track",
            "artworkUrl100": "https://is1-ssl.mzstatic.com/image/100x100bb.jpg",
            "releaseDate": "2025-01-01",
            "trackNumber": 1,
            "discNumber": 1,
            "trackCount": 10,
        }

        # Parse the item
        result = provider._parse_item(item)

        # Verify URL was scaled correctly
        assert len(result.cover_art) == 1
        assert "3000x3000bb.jpg" in result.cover_art[0].url
        assert "100x100bb" not in result.cover_art[0].url
        assert result.cover_art[0].width == 3000
        assert result.cover_art[0].height == 3000
