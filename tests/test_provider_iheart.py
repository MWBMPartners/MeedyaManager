# ============================================================================
# File: /tests/test_provider_iheart.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the iHeartRadio metadata provider (undocumented API).
# Tests provider name/category, availability (always True, best-effort),
# search functionality with mocked HTTP responses, and cover art extraction.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP and responses
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.music.iheart import IHeartProvider  # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — iHeartRadio Provider Tests
# =============================================================================

class TestIHeartProvider:
    """Tests for the IHeartProvider class."""

    def test_provider_name(self):
        """Provider name should be 'iheart'."""
        provider = IHeartProvider()
        assert provider.provider_name == "iheart"

    def test_category(self):
        """Category should be ProviderCategory.MUSIC."""
        provider = IHeartProvider()
        assert provider.category == ProviderCategory.MUSIC

    def test_capabilities_can_search_tracks(self):
        """Capabilities should include can_search_tracks=True."""
        provider = IHeartProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is True

    def test_capabilities_has_static_cover_art(self):
        """Capabilities should include has_static_cover_art=True."""
        provider = IHeartProvider()
        caps = provider.capabilities
        assert caps.has_static_cover_art is True

    def test_requires_auth(self):
        """Provider should not require authentication."""
        provider = IHeartProvider()
        assert provider.requires_auth is False

    def test_is_available_returns_true(self):
        """is_available should return True (no auth needed, best-effort)."""
        provider = IHeartProvider()
        assert provider.is_available() is True

    def test_search_with_mocked_response(self):
        """Search should parse API response and return ProviderResult list."""
        provider = IHeartProvider()

        # Mock API response data
        mock_response_data = {
            "results": {
                "songs": [
                    {
                        "id": 123456,
                        "title": "Test Song",
                        "artistName": "Test Artist",
                        "albumName": "Test Album",
                        "imageUrl": "https://example.com/cover.jpg",
                        "lyrics": "Test lyrics content",
                    }
                ]
            }
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
                query = {"title": "Test Song", "artist": "Test Artist"}
                results = asyncio.run(provider.search(query))

                # Verify results
                assert len(results) == 1
                result = results[0]
                assert result.title == "Test Song"
                assert result.artist == "Test Artist"
                assert result.album == "Test Album"
                assert result.lyrics == "Test lyrics content"
                assert result.provider_id == "123456"
                assert "custom_iheart_id" in result.extra_tags
                assert result.extra_tags["custom_iheart_id"] == "123456"

    def test_search_returns_empty_list_when_no_query_terms(self):
        """Search should return empty list when no query terms provided."""
        provider = IHeartProvider()

        # Empty query
        query = {}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_extract_cover_art_with_image_url(self):
        """_extract_cover_art should extract cover art from imageUrl."""
        provider = IHeartProvider()

        # Mock song with imageUrl
        song = {
            "imageUrl": "https://example.com/album_cover.jpg"
        }

        cover_art = provider._extract_cover_art(song)

        # Verify cover art was extracted
        assert len(cover_art) == 1
        assert cover_art[0].url == "https://example.com/album_cover.jpg"
        assert cover_art[0].asset_type == CoverArtType.STATIC
        assert cover_art[0].format == "jpeg"

    def test_extract_cover_art_with_no_image_url(self):
        """_extract_cover_art should return empty list when no imageUrl available."""
        provider = IHeartProvider()

        # Mock song without imageUrl
        song = {}

        cover_art = provider._extract_cover_art(song)

        assert cover_art == []

    def test_lookup_by_id_returns_none(self):
        """lookup_by_id should return None (direct lookup not supported)."""
        provider = IHeartProvider()

        # Lookup with valid ID
        result = asyncio.run(provider.lookup_by_id("123456"))

        assert result is None
