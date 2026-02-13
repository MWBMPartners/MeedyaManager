# ============================================================================
# File: /tests/test_provider_spotify.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Spotify metadata provider (Spotify Web API).
# Tests OAuth2 Client Credentials flow, search functionality with mocked
# API responses, audio features extraction, ISRC parsing, and cover art
# extraction. Also tests provider capabilities and availability checks.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
import time                                                # For token timestamp testing
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP and credentials
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.music.spotify import SpotifyProvider  # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — Spotify Provider Tests
# =============================================================================

class TestSpotifyProvider:
    """Tests for the SpotifyProvider class."""

    def test_provider_name(self):
        """Provider name should be 'spotify'."""
        provider = SpotifyProvider()
        assert provider.provider_name == "spotify"

    def test_category(self):
        """Category should be ProviderCategory.MUSIC."""
        provider = SpotifyProvider()
        assert provider.category == ProviderCategory.MUSIC

    def test_capabilities_can_search_tracks(self):
        """Capabilities should include can_search_tracks=True."""
        provider = SpotifyProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is True

    def test_capabilities_can_search_albums(self):
        """Capabilities should include can_search_albums=True."""
        provider = SpotifyProvider()
        caps = provider.capabilities
        assert caps.can_search_albums is True

    def test_capabilities_has_static_cover_art(self):
        """Capabilities should include has_static_cover_art=True."""
        provider = SpotifyProvider()
        caps = provider.capabilities
        assert caps.has_static_cover_art is True

    def test_capabilities_has_audio_features(self):
        """Capabilities should include has_audio_features=True."""
        provider = SpotifyProvider()
        caps = provider.capabilities
        assert caps.has_audio_features is True

    def test_requires_auth(self):
        """Provider should require authentication (OAuth2 Client Credentials)."""
        provider = SpotifyProvider()
        assert provider.requires_auth is True

    def test_is_available_false_when_no_credentials(self):
        """is_available should return False when credentials are missing."""
        provider = SpotifyProvider()
        # Mock credentials manager to return None for credentials
        with patch.object(provider._credentials, 'get_credential', return_value=None):
            assert provider.is_available() is False

    def test_is_available_true_when_credentials_present(self):
        """is_available should return True when credentials are present."""
        provider = SpotifyProvider()
        # Mock credentials manager to return valid credentials
        def credential_side_effect(provider_name, field):
            credentials = {
                ('spotify', 'client_id'): 'test_client_id',
                ('spotify', 'client_secret'): 'test_client_secret',
            }
            return credentials.get((provider_name, field))

        with patch.object(provider._credentials, 'get_credential', side_effect=credential_side_effect):
            assert provider.is_available() is True

    def test_oauth2_token_request(self):
        """OAuth2 token request should obtain and cache access token."""
        provider = SpotifyProvider()

        # Mock credentials
        def credential_side_effect(provider_name, field):
            credentials = {
                ('spotify', 'client_id'): 'test_client_id',
                ('spotify', 'client_secret'): 'test_client_secret',
            }
            return credentials.get((provider_name, field))

        # Mock HTTP response for token request
        mock_response = MagicMock()
        mock_response.json.return_value = {
            "access_token": "mock_access_token_abc123",
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
                assert token == "mock_access_token_abc123"

                # Verify HTTP POST was called with correct parameters
                assert mock_client.post.called
                call_args = mock_client.post.call_args

                # Check URL
                assert call_args[0][0] == "https://accounts.spotify.com/api/token"

                # Check headers include Basic auth
                headers = call_args[1]["headers"]
                assert "Authorization" in headers
                assert headers["Authorization"].startswith("Basic ")

    def test_search_with_mocked_response(self):
        """Search should parse API response and return ProviderResult list."""
        provider = SpotifyProvider()

        # Mock API response data
        mock_response_data = {
            "tracks": {
                "items": [
                    {
                        "id": "abc123xyz789",
                        "name": "Test Track",
                        "artists": [
                            {"name": "Test Artist"}
                        ],
                        "album": {
                            "name": "Test Album",
                            "release_date": "2025-01-15",
                            "images": [
                                {
                                    "url": "https://i.scdn.co/image/abc123",
                                    "width": 640,
                                    "height": 640,
                                }
                            ]
                        },
                        "external_ids": {
                            "isrc": "USTEST1234567"
                        },
                        "track_number": 3,
                        "disc_number": 1,
                        "explicit": False,
                        "popularity": 78,
                        "external_urls": {
                            "spotify": "https://open.spotify.com/track/abc123xyz789"
                        }
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
                    # Mock audio features to return None (simplify test)
                    with patch.object(provider, '_get_audio_features', return_value=None):
                        # Run async search
                        query = {"title": "Test Track", "artist": "Test Artist"}
                        results = asyncio.run(provider.search(query))

                        # Verify results
                        assert len(results) == 1
                        result = results[0]
                        assert result.title == "Test Track"
                        assert result.artist == "Test Artist"
                        assert result.album == "Test Album"
                        assert result.isrc == "USTEST1234567"
                        assert result.track_num == "3"
                        assert result.disc_num == "1"
                        assert result.year == "2025"
                        assert result.provider_id == "abc123xyz789"

    def test_extract_cover_art(self):
        """_extract_cover_art should extract largest image from album data."""
        provider = SpotifyProvider()

        # Mock album data with images
        album_data = {
            "images": [
                {
                    "url": "https://i.scdn.co/image/large",
                    "width": 640,
                    "height": 640,
                },
                {
                    "url": "https://i.scdn.co/image/small",
                    "width": 300,
                    "height": 300,
                }
            ]
        }

        cover_art = provider._extract_cover_art(album_data)

        # Verify cover art was extracted
        assert len(cover_art) == 1
        assert cover_art[0].url == "https://i.scdn.co/image/large"
        assert cover_art[0].asset_type == CoverArtType.STATIC
        assert cover_art[0].format == "jpeg"
        assert cover_art[0].width == 640
        assert cover_art[0].height == 640

    def test_audio_features_extraction(self):
        """_get_audio_features should fetch and parse audio features."""
        provider = SpotifyProvider()

        # Mock audio features response
        mock_response_data = {
            "energy": 0.842,
            "danceability": 0.735,
            "tempo": 128.045,
            "valence": 0.654,
            "key": 7,
            "mode": 1,
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
                    # Run async audio features fetch
                    features = asyncio.run(provider._get_audio_features("test_track_id"))

                    # Verify features
                    assert features is not None
                    assert features["energy"] == 0.842
                    assert features["danceability"] == 0.735
                    assert features["tempo"] == 128.0
                    assert features["valence"] == 0.654
                    assert features["key"] == 7
                    assert features["mode"] == 1

    def test_search_returns_empty_list_when_no_query_terms(self):
        """Search should return empty list when no query terms provided."""
        provider = SpotifyProvider()

        # Empty query
        query = {}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_isrc_extraction(self):
        """ISRC codes should be extracted from search results."""
        provider = SpotifyProvider()

        # Mock track with ISRC
        track = {
            "id": "test123",
            "name": "Test",
            "artists": [{"name": "Artist"}],
            "album": {
                "name": "Album",
                "release_date": "2025-01-01",
                "images": []
            },
            "external_ids": {
                "isrc": "GBUM71234567"
            },
            "track_number": 1,
            "disc_number": 1,
            "explicit": False,
            "popularity": 50,
            "external_urls": {
                "spotify": "https://open.spotify.com/track/test123"
            }
        }

        # Mock audio features to return None
        with patch.object(provider, '_get_audio_features', return_value=None):
            result = asyncio.run(provider._parse_track(track))

            # Verify ISRC was extracted
            assert result.isrc == "GBUM71234567"
            assert result.extra_tags.get("custom_spotify_isrc") == "GBUM71234567"

    def test_get_status_info(self):
        """get_status_info should return correct structure."""
        provider = SpotifyProvider()

        # Mock is_available to return True
        with patch.object(provider, 'is_available', return_value=True):
            status = provider.get_status_info()

            assert status["name"] == "spotify"
            assert status["category"] == "music"
            assert status["available"] is True
            assert status["requires_auth"] is True
            assert status["message"] == "Available"
