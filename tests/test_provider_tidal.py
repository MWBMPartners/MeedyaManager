# ============================================================================
# File: /tests/test_provider_tidal.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Tidal metadata provider (Tidal OpenAPI / v1 API).
# Tests OAuth2.1 Client Credentials flow, search functionality with mocked
# API responses, quality tier detection (Lossless, Hi-Res, etc.),
# spatial audio detection (Dolby Atmos, Sony 360RA), ISRC parsing,
# and cover art extraction.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
import time                                                # For token timestamp testing
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP and credentials
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.music.tidal import TidalProvider   # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — Tidal Provider Tests
# =============================================================================

class TestTidalProvider:
    """Tests for the TidalProvider class."""

    def test_provider_name(self):
        """Provider name should be 'tidal'."""
        provider = TidalProvider()
        assert provider.provider_name == "tidal"

    def test_category(self):
        """Category should be ProviderCategory.MUSIC."""
        provider = TidalProvider()
        assert provider.category == ProviderCategory.MUSIC

    def test_capabilities_can_search_tracks(self):
        """Capabilities should include can_search_tracks=True."""
        provider = TidalProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is True

    def test_capabilities_can_search_albums(self):
        """Capabilities should include can_search_albums=True."""
        provider = TidalProvider()
        caps = provider.capabilities
        assert caps.can_search_albums is True

    def test_capabilities_has_static_cover_art(self):
        """Capabilities should include has_static_cover_art=True."""
        provider = TidalProvider()
        caps = provider.capabilities
        assert caps.has_static_cover_art is True

    def test_requires_auth(self):
        """Provider should require authentication (OAuth2.1 Client Credentials)."""
        provider = TidalProvider()
        assert provider.requires_auth is True

    def test_is_available_false_when_no_credentials(self):
        """is_available should return False when credentials are missing."""
        provider = TidalProvider()
        # Mock credentials manager to return None for credentials
        with patch.object(provider._credentials, 'get_credential', return_value=None):
            assert provider.is_available() is False

    def test_is_available_true_when_credentials_present(self):
        """is_available should return True when credentials are present."""
        provider = TidalProvider()
        # Mock credentials manager to return valid credentials
        def credential_side_effect(provider_name, field):
            credentials = {
                ('tidal', 'client_id'): 'test_client_id',
                ('tidal', 'client_secret'): 'test_client_secret',
            }
            return credentials.get((provider_name, field))

        with patch.object(provider._credentials, 'get_credential', side_effect=credential_side_effect):
            assert provider.is_available() is True

    def test_oauth2_token_request(self):
        """OAuth2.1 token request should obtain and cache access token."""
        provider = TidalProvider()

        # Mock credentials
        def credential_side_effect(provider_name, field):
            credentials = {
                ('tidal', 'client_id'): 'test_tidal_id',
                ('tidal', 'client_secret'): 'test_tidal_secret',
            }
            return credentials.get((provider_name, field))

        # Mock HTTP response for token request
        mock_response = MagicMock()
        mock_response.json.return_value = {
            "access_token": "mock_tidal_token_xyz",
            "token_type": "Bearer",
            "expires_in": 3600,
        }
        mock_response.raise_for_status = MagicMock()

        # Mock HTTP client
        mock_client = AsyncMock()
        mock_client.post = AsyncMock(return_value=mock_response)

        with patch.object(provider._credentials, 'get_credential', side_effect=credential_side_effect):
            with patch.object(provider, '_get_http_client', return_value=mock_client):
                # Run async token request
                token = asyncio.run(provider._get_or_refresh_token())

                # Verify token was obtained
                assert token == "mock_tidal_token_xyz"

                # Verify HTTP POST was called with correct parameters
                assert mock_client.post.called
                call_args = mock_client.post.call_args

                # Check URL
                assert call_args[0][0] == "https://auth.tidal.com/v1/oauth2/token"

                # Check headers include Basic auth
                headers = call_args[1]["headers"]
                assert "Authorization" in headers
                assert headers["Authorization"].startswith("Basic ")

    def test_search_with_mocked_response(self):
        """Search should parse API response and return ProviderResult list."""
        provider = TidalProvider()

        # Mock API response data (v1 format)
        mock_response_data = {
            "tracks": {
                "items": [
                    {
                        "id": 123456789,
                        "title": "Test Tidal Track",
                        "artists": [
                            {"name": "Tidal Artist"}
                        ],
                        "album": {
                            "title": "Tidal Album",
                            "cover": "abc123-def456-ghi789",
                            "releaseDate": "2025-03-01"
                        },
                        "isrc": "NOTIDAL123456",
                        "trackNumber": 7,
                        "volumeNumber": 1,
                        "streamStartDate": "2025-03-01",
                        "url": "https://listen.tidal.com/track/123456789",
                        "explicit": True,
                        "audioQuality": "HI_RES_LOSSLESS",
                        "audioModes": ["DOLBY_ATMOS"]
                    }
                ]
            }
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

        # Mock OAuth2 token
        with patch.object(provider, '_get_or_refresh_token', return_value='mock_token'):
            with patch.object(provider, '_get_http_client', return_value=mock_client):
                with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                    # Run async search
                    query = {"title": "Test Tidal Track", "artist": "Tidal Artist"}
                    results = asyncio.run(provider.search(query))

                    # Verify results
                    assert len(results) == 1
                    result = results[0]
                    assert result.title == "Test Tidal Track"
                    assert result.artist == "Tidal Artist"
                    assert result.album == "Tidal Album"
                    assert result.isrc == "NOTIDAL123456"
                    assert result.track_num == "7"
                    assert result.disc_num == "1"
                    assert result.year == "2025"
                    assert result.provider_id == "123456789"

    def test_quality_tier_detection(self):
        """Quality tier should be extracted and labeled correctly."""
        provider = TidalProvider()

        # Mock track with HI_RES_LOSSLESS quality
        track = {
            "id": 111222,
            "title": "High Quality Track",
            "artists": [{"name": "Artist"}],
            "album": {
                "title": "Album",
                "cover": "xyz",
                "releaseDate": "2025-01-01"
            },
            "isrc": "TEST12345",
            "trackNumber": 1,
            "volumeNumber": 1,
            "url": "https://listen.tidal.com/track/111222",
            "explicit": False,
            "audioQuality": "HI_RES_LOSSLESS",
            "audioModes": []
        }

        result = provider._parse_track(track)

        # Verify quality tier was extracted
        assert result.extra_tags.get("custom_tidal_quality") == "Hi-Res Lossless"

    def test_spatial_audio_detection(self):
        """Spatial audio modes (Dolby Atmos, Sony 360RA) should be detected."""
        provider = TidalProvider()

        # Mock track with Dolby Atmos and Sony 360RA
        track = {
            "id": 333444,
            "title": "Spatial Track",
            "artists": [{"name": "Artist"}],
            "album": {
                "title": "Album",
                "cover": "abc",
                "releaseDate": "2025-01-01"
            },
            "isrc": "SPATIAL123",
            "trackNumber": 1,
            "volumeNumber": 1,
            "url": "https://listen.tidal.com/track/333444",
            "explicit": False,
            "audioQuality": "LOSSLESS",
            "audioModes": ["DOLBY_ATMOS", "SONY_360RA"]
        }

        result = provider._parse_track(track)

        # Verify spatial audio tags were added
        assert result.extra_tags.get("custom_tidal_dolby_atmos") == "true"
        assert result.extra_tags.get("custom_tidal_sony_360ra") == "true"

    def test_extract_cover_art(self):
        """_extract_cover_art should build cover URL from UUID."""
        provider = TidalProvider()

        # Mock album data with cover UUID
        album_data = {
            "cover": "abc123-def456-ghi789"
        }

        cover_art = provider._extract_cover_art(album_data)

        # Verify cover art was extracted
        assert len(cover_art) == 1
        # UUID should have dashes removed in URL
        assert "abc123def456ghi789" in cover_art[0].url
        assert cover_art[0].url.endswith("1280x1280.jpg")
        assert cover_art[0].asset_type == CoverArtType.STATIC
        assert cover_art[0].format == "jpeg"
        assert cover_art[0].width == 1280
        assert cover_art[0].height == 1280

    def test_search_returns_empty_list_when_no_query_terms(self):
        """Search should return empty list when no query terms provided."""
        provider = TidalProvider()

        # Empty query
        query = {}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_isrc_extraction(self):
        """ISRC codes should be extracted from search results."""
        provider = TidalProvider()

        # Mock track with ISRC
        track = {
            "id": 555666,
            "title": "Track",
            "artists": [{"name": "Artist"}],
            "album": {
                "title": "Album",
                "cover": "xyz",
                "releaseDate": "2025-01-01"
            },
            "isrc": "TIDAL9876543",
            "trackNumber": 3,
            "volumeNumber": 1,
            "url": "https://listen.tidal.com/track/555666",
            "explicit": False,
            "audioQuality": "LOSSLESS",
            "audioModes": []
        }

        result = provider._parse_track(track)

        # Verify ISRC was extracted
        assert result.isrc == "TIDAL9876543"
        assert result.extra_tags.get("custom_tidal_isrc") == "TIDAL9876543"

    def test_get_status_info(self):
        """get_status_info should return correct structure."""
        provider = TidalProvider()

        # Mock is_available to return True
        with patch.object(provider, 'is_available', return_value=True):
            status = provider.get_status_info()

            assert status["name"] == "tidal"
            assert status["category"] == "music"
            assert status["available"] is True
            assert status["requires_auth"] is True
            assert status["message"] == "Available"
