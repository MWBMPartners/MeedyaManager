# ============================================================================
# File: /tests/test_provider_apple_music.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Apple Music metadata provider (MusicKit API).
# Tests JWT token generation with ES256 signing, private key loading,
# search functionality with mocked API responses, cover art extraction
# (static, animated square, animated portrait, artist spotlight),
# and provider capabilities/status checks.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
import time                                                # For JWT timestamp testing
from pathlib import Path                                   # For temp file path handling
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP and credentials
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.music.apple_music import AppleMusicProvider  # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — Apple Music Provider Tests
# =============================================================================

class TestAppleMusicProvider:
    """Tests for the AppleMusicProvider class."""

    def test_provider_name(self):
        """Provider name should be 'apple_music'."""
        provider = AppleMusicProvider()
        assert provider.provider_name == "apple_music"

    def test_category(self):
        """Category should be ProviderCategory.MUSIC."""
        provider = AppleMusicProvider()
        assert provider.category == ProviderCategory.MUSIC

    def test_capabilities_can_search_tracks(self):
        """Capabilities should include can_search_tracks=True."""
        provider = AppleMusicProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is True

    def test_capabilities_has_static_cover_art(self):
        """Capabilities should include has_static_cover_art=True."""
        provider = AppleMusicProvider()
        caps = provider.capabilities
        assert caps.has_static_cover_art is True

    def test_capabilities_has_animated_cover_art(self):
        """Capabilities should include has_animated_cover_art=True."""
        provider = AppleMusicProvider()
        caps = provider.capabilities
        assert caps.has_animated_cover_art is True

    def test_capabilities_has_artist_spotlight(self):
        """Capabilities should include has_artist_spotlight=True."""
        provider = AppleMusicProvider()
        caps = provider.capabilities
        assert caps.has_artist_spotlight is True

    def test_requires_auth(self):
        """Provider should require authentication (JWT token)."""
        provider = AppleMusicProvider()
        assert provider.requires_auth is True

    def test_is_available_false_when_no_credentials(self):
        """is_available should return False when credentials are missing."""
        provider = AppleMusicProvider()
        # Mock credentials manager to return False for has_credentials
        with patch.object(provider._credentials, 'has_credentials', return_value=False):
            assert provider.is_available() is False

    def test_is_available_true_when_credentials_present(self):
        """is_available should return True when credentials are present and pyjwt is installed."""
        provider = AppleMusicProvider()
        # Mock credentials manager to return True for has_credentials
        # Mock jwt module in sys.modules to simulate pyjwt being installed
        mock_jwt = MagicMock()
        with patch.object(provider._credentials, 'has_credentials', return_value=True):
            with patch.dict('sys.modules', {'jwt': mock_jwt}):
                assert provider.is_available() is True

    def test_jwt_token_generation(self):
        """JWT token generation should create a valid ES256-signed token."""
        provider = AppleMusicProvider()

        # Create a mock jwt module
        mock_jwt_module = MagicMock()
        mock_jwt_module.encode.return_value = 'mock_jwt_token'

        # Mock credentials
        with patch.object(provider._credentials, 'get_credential') as mock_get:
            # Mock credential responses
            def credential_side_effect(provider_name, field):
                credentials = {
                    ('apple_music', 'team_id'): 'TEST123456',
                    ('apple_music', 'key_id'): 'TESTKEY123',
                }
                return credentials.get((provider_name, field))

            mock_get.side_effect = credential_side_effect

            # Mock private key loading
            with patch.object(provider, '_load_private_key', return_value='mock_private_key'):
                # Mock jwt module import inside the method
                with patch.dict('sys.modules', {'jwt': mock_jwt_module}):
                    token = provider._get_or_refresh_token()

                    # Verify token was generated
                    assert token == 'mock_jwt_token'

                    # Verify jwt.encode was called with correct parameters
                    assert mock_jwt_module.encode.called
                    call_args = mock_jwt_module.encode.call_args

                    # Check payload structure
                    payload = call_args[0][0]
                    assert payload['iss'] == 'TEST123456'
                    assert 'iat' in payload
                    assert 'exp' in payload

                    # Check headers
                    headers = call_args[1]['headers']
                    assert headers['alg'] == 'ES256'
                    assert headers['kid'] == 'TESTKEY123'

    def test_load_private_key_from_pem_string(self):
        """_load_private_key should handle PEM-encoded key strings."""
        provider = AppleMusicProvider()

        # Mock PEM key string
        pem_key = """-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgTest
-----END PRIVATE KEY-----"""

        with patch.object(provider._credentials, 'get_credential', return_value=pem_key):
            result = provider._load_private_key()
            assert result == pem_key

    def test_load_private_key_from_file(self, tmp_path):
        """_load_private_key should read key from file path."""
        provider = AppleMusicProvider()

        # Create temporary key file
        key_file = tmp_path / "test_key.p8"
        pem_content = """-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgTest
-----END PRIVATE KEY-----"""
        key_file.write_text(pem_content)

        # Mock credential to return file path
        with patch.object(provider._credentials, 'get_credential', return_value=str(key_file)):
            result = provider._load_private_key()
            assert result == pem_content

    def test_search_with_mocked_response(self):
        """Search should parse API response and return ProviderResult list."""
        provider = AppleMusicProvider()

        # Mock API response data
        mock_response_data = {
            "results": {
                "songs": {
                    "data": [
                        {
                            "id": "1234567890",
                            "type": "songs",
                            "attributes": {
                                "name": "Test Song",
                                "artistName": "Test Artist",
                                "albumName": "Test Album",
                                "genreNames": ["Pop", "Rock"],
                                "isrc": "USTEST1234567",
                                "trackNumber": 1,
                                "discNumber": 1,
                                "releaseDate": "2025-01-15",
                                "url": "https://music.apple.com/gb/song/test/1234567890",
                                "artwork": {
                                    "url": "https://is1-ssl.mzstatic.com/image/{w}x{h}bb.jpg",
                                    "width": 3000,
                                    "height": 3000,
                                },
                                "editorialVideo": {
                                    "motionSquareVideo1x1": {
                                        "video": "https://example.com/square.mp4"
                                    },
                                    "motionDetailTall": {
                                        "video": "https://example.com/portrait.mp4"
                                    },
                                    "motionArtistWide16x9": {
                                        "video": "https://example.com/spotlight.mp4"
                                    }
                                }
                            }
                        }
                    ]
                }
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

        # Mock JWT token generation
        with patch.object(provider, '_get_or_refresh_token', return_value='mock_token'):
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
                    assert result.genre == "Pop, Rock"
                    assert result.isrc == "USTEST1234567"
                    assert result.track_num == "1"
                    assert result.disc_num == "1"
                    assert result.year == "2025"
                    assert result.provider_id == "1234567890"

    def test_extract_cover_art_with_all_types(self):
        """_extract_cover_art should extract all available cover art types."""
        provider = AppleMusicProvider()

        # Mock song attributes with all cover art types
        attrs = {
            "artwork": {
                "url": "https://example.com/{w}x{h}bb.jpg",
                "width": 3000,
                "height": 3000,
            },
            "editorialVideo": {
                "motionSquareVideo1x1": {
                    "video": "https://example.com/square.mp4"
                },
                "motionDetailTall": {
                    "video": "https://example.com/portrait.mp4"
                },
                "motionArtistWide16x9": {
                    "video": "https://example.com/spotlight.mp4"
                }
            }
        }

        cover_art = provider._extract_cover_art(attrs)

        # Verify all 4 cover art types are present
        assert len(cover_art) == 4

        # Check static cover art
        static_art = [a for a in cover_art if a.asset_type == CoverArtType.STATIC][0]
        assert "3000x3000bb.jpg" in static_art.url
        assert static_art.format == "jpeg"
        assert static_art.width == 3000
        assert static_art.height == 3000

        # Check animated square
        square_art = [a for a in cover_art if a.asset_type == CoverArtType.ANIMATED_SQUARE][0]
        assert square_art.url == "https://example.com/square.mp4"
        assert square_art.format == "mp4"

        # Check animated portrait
        portrait_art = [a for a in cover_art if a.asset_type == CoverArtType.ANIMATED_PORTRAIT][0]
        assert portrait_art.url == "https://example.com/portrait.mp4"
        assert portrait_art.format == "mp4"

        # Check artist spotlight
        spotlight_art = [a for a in cover_art if a.asset_type == CoverArtType.ARTIST_SPOTLIGHT][0]
        assert spotlight_art.url == "https://example.com/spotlight.mp4"
        assert spotlight_art.format == "mp4"

    def test_search_returns_empty_list_when_no_query_terms(self):
        """Search should return empty list when no query terms provided."""
        provider = AppleMusicProvider()

        # Empty query
        query = {}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_get_status_info(self):
        """get_status_info should return correct structure."""
        provider = AppleMusicProvider()

        # Mock is_available to return True
        with patch.object(provider, 'is_available', return_value=True):
            status = provider.get_status_info()

            assert status["name"] == "apple_music"
            assert status["category"] == "music"
            assert status["available"] is True
            assert status["requires_auth"] is True
            assert status["message"] == "Available"
