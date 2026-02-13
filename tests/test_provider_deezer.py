# ============================================================================
# File: /tests/test_provider_deezer.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Deezer metadata provider (Deezer Public API).
# Tests search functionality with mocked API responses, ISRC parsing,
# cover art extraction, and provider capabilities. Since Deezer uses
# a public API, no authentication tests are needed.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP and credentials
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.music.deezer import DeezerProvider # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — Deezer Provider Tests
# =============================================================================

class TestDeezerProvider:
    """Tests for the DeezerProvider class."""

    def test_provider_name(self):
        """Provider name should be 'deezer'."""
        provider = DeezerProvider()
        assert provider.provider_name == "deezer"

    def test_category(self):
        """Category should be ProviderCategory.MUSIC."""
        provider = DeezerProvider()
        assert provider.category == ProviderCategory.MUSIC

    def test_capabilities_can_search_tracks(self):
        """Capabilities should include can_search_tracks=True."""
        provider = DeezerProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is True

    def test_capabilities_can_search_albums(self):
        """Capabilities should include can_search_albums=True."""
        provider = DeezerProvider()
        caps = provider.capabilities
        assert caps.can_search_albums is True

    def test_capabilities_has_static_cover_art(self):
        """Capabilities should include has_static_cover_art=True."""
        provider = DeezerProvider()
        caps = provider.capabilities
        assert caps.has_static_cover_art is True

    def test_requires_auth(self):
        """Provider should not require authentication (public API)."""
        provider = DeezerProvider()
        assert provider.requires_auth is False

    def test_is_available(self):
        """is_available should always return True (no auth required)."""
        provider = DeezerProvider()
        assert provider.is_available() is True

    def test_search_with_mocked_response(self):
        """Search should parse API response and return ProviderResult list."""
        provider = DeezerProvider()

        # Mock API response data
        mock_response_data = {
            "data": [
                {
                    "id": 987654321,
                    "title": "Test Song",
                    "artist": {
                        "name": "Test Artist"
                    },
                    "album": {
                        "title": "Test Album",
                        "cover_xl": "https://e-cdns-images.dzcdn.net/images/cover/abc/1000x1000.jpg",
                        "release_date": "2025-02-01"
                    },
                    "isrc": "FRTEST1234567",
                    "track_position": 5,
                    "disk_number": 1,
                    "duration": 245,
                    "link": "https://www.deezer.com/track/987654321"
                }
            ]
        }

        # Mock HTTP response
        mock_response = MagicMock()
        mock_response.json.return_value = mock_response_data
        mock_response.raise_for_status = MagicMock()

        # Mock HTTP client
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
                assert result.isrc == "FRTEST1234567"
                assert result.track_num == "5"
                assert result.disc_num == "1"
                assert result.year == "2025"
                assert result.provider_id == "987654321"

    def test_extract_cover_art(self):
        """_extract_cover_art should extract XL cover from album data."""
        provider = DeezerProvider()

        # Mock album data with cover_xl
        album_data = {
            "cover_xl": "https://e-cdns-images.dzcdn.net/images/cover/xyz/1000x1000.jpg"
        }

        cover_art = provider._extract_cover_art(album_data)

        # Verify cover art was extracted
        assert len(cover_art) == 1
        assert cover_art[0].url == "https://e-cdns-images.dzcdn.net/images/cover/xyz/1000x1000.jpg"
        assert cover_art[0].asset_type == CoverArtType.STATIC
        assert cover_art[0].format == "jpeg"
        assert cover_art[0].width == 1000
        assert cover_art[0].height == 1000

    def test_search_returns_empty_list_when_no_query_terms(self):
        """Search should return empty list when no query terms provided."""
        provider = DeezerProvider()

        # Empty query
        query = {}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_isrc_extraction(self):
        """ISRC codes should be extracted from search results."""
        provider = DeezerProvider()

        # Mock track with ISRC
        track = {
            "id": 123456,
            "title": "Test Track",
            "artist": {
                "name": "Artist Name"
            },
            "album": {
                "title": "Album Name",
                "cover_xl": "https://example.com/cover.jpg",
                "release_date": "2025-01-01"
            },
            "isrc": "DETEST7654321",
            "track_position": 2,
            "disk_number": 1,
            "duration": 180,
            "link": "https://www.deezer.com/track/123456"
        }

        result = provider._parse_track(track)

        # Verify ISRC was extracted
        assert result.isrc == "DETEST7654321"
        assert result.extra_tags.get("custom_deezer_isrc") == "DETEST7654321"

    def test_parse_track_with_missing_fields(self):
        """_parse_track should handle tracks with missing optional fields."""
        provider = DeezerProvider()

        # Mock track with minimal data
        track = {
            "id": 999,
            "title": "Minimal Track",
            "artist": {
                "name": "Minimal Artist"
            },
            "album": {
                "title": "Minimal Album"
            }
        }

        result = provider._parse_track(track)

        # Verify result was created with defaults
        assert result is not None
        assert result.title == "Minimal Track"
        assert result.artist == "Minimal Artist"
        assert result.album == "Minimal Album"
        assert result.isrc == ""
        assert result.year == ""

    def test_get_status_info(self):
        """get_status_info should return correct structure."""
        provider = DeezerProvider()

        status = provider.get_status_info()

        assert status["name"] == "deezer"
        assert status["category"] == "music"
        assert status["available"] is True
        assert status["requires_auth"] is False
        assert status["message"] == "Available"
