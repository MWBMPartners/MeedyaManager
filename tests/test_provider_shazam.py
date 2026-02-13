# ============================================================================
# File: /tests/test_provider_shazam.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Shazam metadata provider.
# Tests provider capabilities, shazamio availability check, text search,
# audio fingerprinting, result parsing, cover art extraction, and custom tags.
#
# All tests are offline — HTTP calls and shazamio library are mocked.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP and libraries
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.music.shazam import ShazamProvider  # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — Shazam Provider Tests
# =============================================================================

class TestShazamProvider:
    """Tests for the ShazamProvider class."""

    def test_provider_name(self):
        """Provider name should be 'shazam'."""
        provider = ShazamProvider()
        assert provider.provider_name == "shazam"

    def test_category(self):
        """Category should be ProviderCategory.MUSIC."""
        provider = ShazamProvider()
        assert provider.category == ProviderCategory.MUSIC

    def test_capabilities_can_search_tracks(self):
        """Capabilities should include can_search_tracks=True."""
        provider = ShazamProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is True

    def test_capabilities_can_fingerprint_audio(self):
        """Capabilities should include can_fingerprint_audio=True."""
        provider = ShazamProvider()
        caps = provider.capabilities
        assert caps.can_fingerprint_audio is True

    def test_capabilities_has_static_cover_art(self):
        """Capabilities should include has_static_cover_art=True."""
        provider = ShazamProvider()
        caps = provider.capabilities
        assert caps.has_static_cover_art is True

    def test_requires_auth(self):
        """Provider should not require authentication."""
        provider = ShazamProvider()
        assert provider.requires_auth is False

    def test_is_available_false_when_shazamio_not_installed(self):
        """is_available should return False when shazamio is not installed."""
        provider = ShazamProvider()

        # Mock ImportError when importing shazamio
        with patch.dict('sys.modules', {'shazamio': None}):
            with patch('builtins.__import__', side_effect=ImportError):
                assert provider.is_available() is False

    def test_is_available_true_when_shazamio_installed(self):
        """is_available should return True when shazamio is installed."""
        provider = ShazamProvider()

        # Mock successful shazamio import
        mock_shazamio = MagicMock()
        with patch.dict('sys.modules', {'shazamio': mock_shazamio}):
            assert provider.is_available() is True

    def test_text_search_with_mocked_response(self):
        """Text search should query shazamio and parse results."""
        provider = ShazamProvider()

        # Mock shazamio search response
        mock_search_data = {
            "tracks": {
                "hits": [
                    {
                        "track": {
                            "key": "shazam-track-123",
                            "title": "Shazam Test Song",
                            "subtitle": "Shazam Test Artist",
                            "genres": {"primary": "Pop"},
                            "url": "https://www.shazam.com/track/123",
                            "images": {
                                "coverart": "https://cdn.shazam.com/cover123.jpg"
                            },
                            "key": "C Major"
                        }
                    }
                ]
            }
        }

        # Mock Shazam client
        mock_shazam_client = AsyncMock()
        mock_shazam_client.search_track = AsyncMock(return_value=mock_search_data)

        # Mock rate limiter
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        # Mock shazamio module
        mock_shazamio = MagicMock()
        mock_shazamio.Shazam = MagicMock(return_value=mock_shazam_client)

        with patch.dict('sys.modules', {'shazamio': mock_shazamio}):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                with patch.object(provider, 'is_available', return_value=True):
                    # Run search
                    query = {"title": "Test Song", "artist": "Test Artist"}
                    results = asyncio.run(provider.search(query))

                    # Verify results
                    assert len(results) == 1
                    result = results[0]
                    assert result.title == "Shazam Test Song"
                    assert result.artist == "Shazam Test Artist"
                    assert result.genre == "Pop"

    def test_audio_fingerprinting_with_mocked_response(self):
        """Audio fingerprinting should use recognize() method."""
        provider = ShazamProvider()

        # Mock shazamio recognize response
        mock_recognize_data = {
            "track": {
                "key": "fingerprint-track-456",
                "title": "Fingerprinted Song",
                "subtitle": "Fingerprinted Artist",
                "genres": {"primary": "Rock"},
                "url": "https://www.shazam.com/track/456",
                "images": {
                    "coverart": "https://cdn.shazam.com/cover456.jpg"
                }
            }
        }

        # Mock Shazam client
        mock_shazam_client = AsyncMock()
        mock_shazam_client.recognize = AsyncMock(return_value=mock_recognize_data)

        # Mock rate limiter
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        # Mock shazamio module
        mock_shazamio = MagicMock()
        mock_shazamio.Shazam = MagicMock(return_value=mock_shazam_client)

        with patch.dict('sys.modules', {'shazamio': mock_shazamio}):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                with patch.object(provider, 'is_available', return_value=True):
                    # Run search with file path (triggers fingerprinting)
                    query = {"file_path": "/path/to/audio.mp3"}
                    results = asyncio.run(provider.search(query))

                    # Verify recognize was called
                    assert mock_shazam_client.recognize.called

                    # Verify results
                    assert len(results) == 1
                    result = results[0]
                    assert result.title == "Fingerprinted Song"
                    assert result.artist == "Fingerprinted Artist"

    def test_cover_art_extraction(self):
        """Should extract cover art URL from Shazam response."""
        provider = ShazamProvider()

        # Mock track with cover art
        track = {
            "key": "cover-test-789",
            "title": "Cover Test",
            "subtitle": "Cover Artist",
            "images": {
                "coverart": "https://cdn.shazam.com/cover789.jpg"
            },
            "url": ""
        }

        result = provider._parse_track(track)

        # Verify cover art
        assert len(result.cover_art) == 1
        cover = result.cover_art[0]
        assert cover.asset_type == CoverArtType.STATIC
        assert cover.url == "https://cdn.shazam.com/cover789.jpg"
        assert cover.format == "jpeg"

    def test_shazam_custom_tags(self):
        """Results should include Shazam-specific custom tags."""
        provider = ShazamProvider()

        # Mock track with all metadata (note: Shazam uses 'key' for both track ID and musical key)
        # In real API responses, track ID is in 'key' field, musical key in 'sections'
        track = {
            "key": "custom-tags-123",
            "title": "Custom Tags Test",
            "subtitle": "Tags Artist",
            "url": "https://www.shazam.com/track/123",
            "sections": [{"type": "SONG", "metapage": {"caption": "D Minor"}}],  # Musical key in sections
            "genres": {"primary": "Jazz"},
            "images": {}
        }

        result = provider._parse_track(track)

        # Verify custom tags
        assert "custom_shazam_id" in result.extra_tags
        assert result.extra_tags["custom_shazam_id"] == "custom-tags-123"
        assert "custom_shazam_url" in result.extra_tags
        assert result.extra_tags["custom_shazam_url"] == "https://www.shazam.com/track/123"

    def test_search_returns_empty_when_unavailable(self):
        """Search should return empty list when shazamio is not available."""
        provider = ShazamProvider()

        with patch.object(provider, 'is_available', return_value=False):
            query = {"title": "Test Song"}
            results = asyncio.run(provider.search(query))

            assert results == []

    def test_search_returns_empty_when_no_query_terms(self):
        """Search should return empty list when no query terms or file path provided."""
        provider = ShazamProvider()

        with patch.object(provider, 'is_available', return_value=True):
            # Empty query
            query = {}
            results = asyncio.run(provider.search(query))

            assert results == []

    def test_genre_extraction_from_primary(self):
        """Should extract genre from genres.primary field."""
        provider = ShazamProvider()

        track = {
            "key": "genre-test-1",
            "title": "Genre Test",
            "subtitle": "Genre Artist",
            "genres": {"primary": "Electronic"},
            "url": "",
            "images": {}
        }

        result = provider._parse_track(track)
        assert result.genre == "Electronic"

    def test_genre_extraction_from_list(self):
        """Should extract genre from genres list if primary not available."""
        provider = ShazamProvider()

        track = {
            "key": "genre-test-2",
            "title": "Genre Test 2",
            "subtitle": "Genre Artist 2",
            "genres": ["Alternative", "Indie"],           # List format
            "url": "",
            "images": {}
        }

        result = provider._parse_track(track)
        assert result.genre == "Alternative"

    def test_get_status_info(self):
        """get_status_info should return correct structure."""
        provider = ShazamProvider()

        with patch.object(provider, 'is_available', return_value=True):
            status = provider.get_status_info()

            assert status["name"] == "shazam"
            assert status["category"] == "music"
            assert status["available"] is True
            assert status["requires_auth"] is False
            assert status["message"] == "Available"
