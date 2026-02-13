# ============================================================================
# File: /tests/test_provider_youtube_music.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the YouTube Music metadata provider (ytmusicapi).
# Tests provider name/category, availability checks with/without headers file,
# search functionality with mocked ytmusicapi responses, and cover art extraction.
#
# All tests are offline — ytmusicapi calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from pathlib import Path                                   # For temp file path handling
from unittest.mock import MagicMock, AsyncMock, patch      # Mock ytmusicapi and credentials
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.music.youtube_music import YouTubeMusicProvider  # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — YouTube Music Provider Tests
# =============================================================================

class TestYouTubeMusicProvider:
    """Tests for the YouTubeMusicProvider class."""

    def test_provider_name(self):
        """Provider name should be 'youtube_music'."""
        provider = YouTubeMusicProvider()
        assert provider.provider_name == "youtube_music"

    def test_category(self):
        """Category should be ProviderCategory.MUSIC."""
        provider = YouTubeMusicProvider()
        assert provider.category == ProviderCategory.MUSIC

    def test_capabilities_can_search_tracks(self):
        """Capabilities should include can_search_tracks=True."""
        provider = YouTubeMusicProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is True

    def test_capabilities_can_search_albums(self):
        """Capabilities should include can_search_albums=True."""
        provider = YouTubeMusicProvider()
        caps = provider.capabilities
        assert caps.can_search_albums is True

    def test_capabilities_has_static_cover_art(self):
        """Capabilities should include has_static_cover_art=True."""
        provider = YouTubeMusicProvider()
        caps = provider.capabilities
        assert caps.has_static_cover_art is True

    def test_requires_auth(self):
        """Provider should require authentication (headers_auth.json)."""
        provider = YouTubeMusicProvider()
        assert provider.requires_auth is True

    def test_is_available_false_when_ytmusicapi_not_installed(self):
        """is_available should return False when ytmusicapi is not installed."""
        provider = YouTubeMusicProvider()
        # Mock ytmusicapi module to be None in sys.modules to simulate ImportError
        import sys
        # Store original module if it exists
        original_module = sys.modules.get('ytmusicapi')
        try:
            # Set ytmusicapi to None to trigger ImportError on import
            sys.modules['ytmusicapi'] = None
            assert provider.is_available() is False
        finally:
            # Restore original module state
            if original_module is not None:
                sys.modules['ytmusicapi'] = original_module
            elif 'ytmusicapi' in sys.modules:
                del sys.modules['ytmusicapi']

    def test_is_available_false_when_headers_file_missing(self, tmp_path):
        """is_available should return False when headers_auth.json does not exist."""
        provider = YouTubeMusicProvider()

        # Mock ytmusicapi as installed
        mock_ytmusicapi = MagicMock()

        # Mock credentials to return a non-existent file path
        nonexistent_path = str(tmp_path / "does_not_exist.json")

        with patch.dict('sys.modules', {'ytmusicapi': mock_ytmusicapi}):
            with patch.object(provider._credentials, 'get_credential', return_value=nonexistent_path):
                assert provider.is_available() is False

    def test_is_available_true_when_headers_file_exists(self, tmp_path):
        """is_available should return True when ytmusicapi is installed and headers file exists."""
        provider = YouTubeMusicProvider()

        # Create a temporary headers_auth.json file
        headers_file = tmp_path / "headers_auth.json"
        headers_file.write_text('{"Cookie": "test"}')

        # Mock ytmusicapi as installed
        mock_ytmusicapi = MagicMock()

        with patch.dict('sys.modules', {'ytmusicapi': mock_ytmusicapi}):
            with patch.object(provider._credentials, 'get_credential', return_value=str(headers_file)):
                assert provider.is_available() is True

    def test_search_with_mocked_ytmusicapi(self):
        """Search should parse ytmusicapi response and return ProviderResult list."""
        provider = YouTubeMusicProvider()

        # Mock ytmusicapi response data
        mock_search_results = [
            {
                "videoId": "dQw4w9WgXcQ",
                "title": "Test Song",
                "artists": [{"name": "Test Artist"}],
                "album": {"name": "Test Album"},
                "duration_seconds": 213,
                "thumbnails": [
                    {"url": "https://example.com/thumb120.jpg", "width": 120, "height": 120},
                    {"url": "https://example.com/thumb480.jpg", "width": 480, "height": 480},
                ],
            }
        ]

        # Mock YTMusic instance
        mock_ytmusic = MagicMock()
        mock_ytmusic.search = MagicMock(return_value=mock_search_results)

        # Create a mock rate limiter with async acquire
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        # Mock the _get_ytmusic_client method to return our mock
        async def mock_get_ytmusic_client():
            return mock_ytmusic

        with patch.object(provider, '_get_ytmusic_client', side_effect=mock_get_ytmusic_client):
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
                assert result.provider_id == "dQw4w9WgXcQ"
                assert result.provider_url == "https://music.youtube.com/watch?v=dQw4w9WgXcQ"
                assert "custom_youtube_music_id" in result.extra_tags
                assert result.extra_tags["custom_youtube_music_id"] == "dQw4w9WgXcQ"

    def test_search_returns_empty_list_when_no_query_terms(self):
        """Search should return empty list when no query terms provided."""
        provider = YouTubeMusicProvider()

        # Empty query
        query = {}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_extract_cover_art_with_thumbnails(self):
        """_extract_cover_art should extract the largest thumbnail."""
        provider = YouTubeMusicProvider()

        # Mock song with thumbnails
        song = {
            "thumbnails": [
                {"url": "https://example.com/thumb120.jpg", "width": 120, "height": 120},
                {"url": "https://example.com/thumb480.jpg", "width": 480, "height": 480},
                {"url": "https://example.com/thumb1200.jpg", "width": 1200, "height": 1200},
            ]
        }

        cover_art = provider._extract_cover_art(song)

        # Verify the largest thumbnail was selected
        assert len(cover_art) == 1
        assert cover_art[0].url == "https://example.com/thumb1200.jpg"
        assert cover_art[0].asset_type == CoverArtType.STATIC
        assert cover_art[0].format == "jpeg"
        assert cover_art[0].width == 1200
        assert cover_art[0].height == 1200

    def test_extract_cover_art_with_no_thumbnails(self):
        """_extract_cover_art should return empty list when no thumbnails available."""
        provider = YouTubeMusicProvider()

        # Mock song without thumbnails
        song = {"thumbnails": []}

        cover_art = provider._extract_cover_art(song)

        assert cover_art == []
