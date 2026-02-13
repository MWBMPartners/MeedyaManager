# ============================================================================
# File: /tests/test_provider_tvdb.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the TVDB (TheTVDB) metadata provider.
# Tests JWT token flow, series search with mocked API responses,
# slug URL construction, and provider capabilities/status checks.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP calls
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.video.tvdb import TVDBProvider     # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — TVDB Provider Tests
# =============================================================================

class TestTVDBProvider:
    """Tests for the TVDBProvider class."""

    def test_provider_name(self):
        """Provider name should be 'tvdb'."""
        provider = TVDBProvider()
        assert provider.provider_name == "tvdb"

    def test_category(self):
        """Category should be ProviderCategory.VIDEO."""
        provider = TVDBProvider()
        assert provider.category == ProviderCategory.VIDEO

    def test_requires_auth(self):
        """Provider should require authentication (API key)."""
        provider = TVDBProvider()
        assert provider.requires_auth is True

    def test_capabilities(self):
        """Provider should support shows, episodes, and cover art."""
        provider = TVDBProvider()
        caps = provider.capabilities
        assert caps.can_search_shows is True
        assert caps.can_search_episodes is True
        assert caps.has_static_cover_art is True
        # TVDB doesn't support movies or IMDb lookup
        assert caps.can_search_movies is False
        assert caps.can_lookup_imdb_id is False

    def test_is_available_without_api_key(self):
        """is_available should return False when API key is missing."""
        provider = TVDBProvider()
        # Mock credential manager to return None
        with patch.object(provider._credential_manager, 'get_credential', return_value=None):
            assert provider.is_available() is False

    def test_is_available_with_api_key(self):
        """is_available should return True when API key is present."""
        provider = TVDBProvider()
        # Mock httpx module and credential manager
        mock_httpx = MagicMock()
        with patch.dict('sys.modules', {'httpx': mock_httpx}):
            with patch.object(provider._credential_manager, 'get_credential', return_value='fake_api_key'):
                assert provider.is_available() is True

    def test_jwt_token_flow_with_mocked_response(self):
        """JWT token authentication should work with mocked API response."""
        provider = TVDBProvider()

        # Mock TVDB login response
        mock_login_response_data = {
            "data": {
                "token": "fake_jwt_token_12345"
            }
        }

        # Mock HTTP response
        mock_response = MagicMock()
        mock_response.json.return_value = mock_login_response_data
        mock_response.raise_for_status = MagicMock()

        # Mock HTTP client
        mock_client = AsyncMock()
        mock_client.post = AsyncMock(return_value=mock_response)

        # Create a mock rate limiter
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        # Mock credential manager
        with patch.object(provider, '_get_http_client', return_value=mock_client):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                with patch.object(provider._credential_manager, 'get_credential', return_value='fake_api_key'):
                    # Run async token retrieval
                    token = asyncio.run(provider._get_jwt_token())

                    # Verify token
                    assert token == "fake_jwt_token_12345"
                    assert provider._jwt_token == "fake_jwt_token_12345"

                    # Verify API was called with correct payload
                    mock_client.post.assert_called_once()
                    call_args = mock_client.post.call_args
                    assert call_args[1]["json"] == {"apikey": "fake_api_key"}

    def test_series_search_with_mocked_response(self):
        """Series search should parse TVDB API response correctly."""
        provider = TVDBProvider()

        # Mock TVDB search response
        mock_response_data = {
            "data": [
                {
                    "id": "12345",
                    "tvdb_id": "12345",
                    "name": "The Wire",
                    "overview": "The Wire is an American crime drama...",
                    "year": "2002",
                    "slug": "the-wire",
                    "image": "https://artworks.thetvdb.com/banners/posters/12345.jpg",
                    "status": {"name": "Ended"},
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

        # Create a mock rate limiter
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        # Mock JWT token (bypass authentication)
        provider._jwt_token = "fake_jwt_token"
        provider._token_expires = 9999999999.0              # Far future

        # Mock credential manager
        with patch.object(provider, '_get_http_client', return_value=mock_client):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                with patch.object(provider._credential_manager, 'get_credential', return_value='fake_api_key'):
                    # Run async search
                    query = {"show": "The Wire"}
                    results = asyncio.run(provider.search(query))

                    # Verify results
                    assert len(results) == 1
                    result = results[0]
                    assert result.title == "The Wire"
                    assert result.show == "The Wire"
                    assert result.year == "2002"
                    assert result.provider_id == "12345"
                    assert "thetvdb.com/series/the-wire" in result.provider_url

                    # Verify cover art
                    assert len(result.cover_art) == 1
                    cover = result.cover_art[0]
                    assert cover.asset_type == CoverArtType.STATIC
                    assert "artworks.thetvdb.com" in cover.url
                    assert cover.format == "jpeg"

                    # Verify extra tags
                    assert "custom_tvdb_id" in result.extra_tags
                    assert result.extra_tags["custom_tvdb_id"] == "12345"
                    assert "custom_tvdb_slug" in result.extra_tags
                    assert result.extra_tags["custom_tvdb_slug"] == "the-wire"
                    assert "custom_tvdb_url" in result.extra_tags
                    assert "custom_tvdb_status" in result.extra_tags
                    assert result.extra_tags["custom_tvdb_status"] == "Ended"

    def test_slug_url_construction(self):
        """Series URLs should use the slug for URL construction."""
        provider = TVDBProvider()

        # Create a minimal series item with slug
        item = {
            "id": "999",
            "name": "Test Show",
            "slug": "test-show",
            "overview": "A test show",
            "year": "2024",
            "status": "Continuing",
            "image": "https://example.com/poster.jpg",
        }

        result = provider._parse_series(item)
        assert result is not None
        assert result.provider_url == "https://thetvdb.com/series/test-show"
        assert result.extra_tags["custom_tvdb_url"] == "https://thetvdb.com/series/test-show"

    def test_series_without_slug(self):
        """Series without slug should have empty URL."""
        provider = TVDBProvider()

        # Create a series item without slug
        item = {
            "id": "888",
            "name": "Test Show No Slug",
            "overview": "A test show",
            "year": "2024",
        }

        result = provider._parse_series(item)
        assert result is not None
        assert result.provider_url == ""

    def test_empty_query_returns_empty_results(self):
        """Search with empty query should return empty results."""
        provider = TVDBProvider()
        query = {}
        results = asyncio.run(provider.search(query))
        assert results == []

    def test_authorization_header_in_search(self):
        """Search requests should include JWT token in Authorization header."""
        provider = TVDBProvider()

        # Mock response
        mock_response = MagicMock()
        mock_response.json.return_value = {"data": []}
        mock_response.raise_for_status = MagicMock()

        # Mock HTTP client
        mock_client = AsyncMock()
        mock_client.get = AsyncMock(return_value=mock_response)

        # Create a mock rate limiter
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        # Mock JWT token
        provider._jwt_token = "test_jwt_token"
        provider._token_expires = 9999999999.0

        with patch.object(provider, '_get_http_client', return_value=mock_client):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                with patch.object(provider._credential_manager, 'get_credential', return_value='fake_api_key'):
                    query = {"show": "Test"}
                    asyncio.run(provider.search(query))

                    # Verify Authorization header was sent
                    mock_client.get.assert_called_once()
                    call_args = mock_client.get.call_args
                    headers = call_args[1]["headers"]
                    assert "Authorization" in headers
                    assert headers["Authorization"] == "Bearer test_jwt_token"
