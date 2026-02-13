# ============================================================================
# File: /tests/test_provider_eidr.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the EIDR (Entertainment Identifier Registry) metadata provider.
# Tests EIDR ID format, resolve endpoint with mocked responses,
# and provider capabilities/status checks.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP calls
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.identifiers.eidr import EIDRProvider  # Provider under test


# =============================================================================
# Test Class — EIDR Provider Tests
# =============================================================================

class TestEIDRProvider:
    """Tests for the EIDRProvider class."""

    def test_provider_name(self):
        """Provider name should be 'eidr'."""
        provider = EIDRProvider()
        assert provider.provider_name == "eidr"

    def test_category(self):
        """Category should be ProviderCategory.IDENTIFIER."""
        provider = EIDRProvider()
        assert provider.category == ProviderCategory.IDENTIFIER

    def test_requires_auth(self):
        """Provider should require authentication (HTTP Basic Auth)."""
        provider = EIDRProvider()
        assert provider.requires_auth is True

    def test_capabilities(self):
        """Provider should support movie and show identification."""
        provider = EIDRProvider()
        caps = provider.capabilities
        assert caps.can_search_movies is True
        assert caps.can_search_shows is True
        # EIDR doesn't provide cover art
        assert caps.has_static_cover_art is False

    def test_is_available_without_credentials(self):
        """is_available should return False when credentials are missing."""
        provider = EIDRProvider()
        # Mock credential manager to return None for both credentials
        with patch.object(provider._credential_manager, 'get_credential', return_value=None):
            assert provider.is_available() is False

    def test_is_available_with_credentials(self):
        """is_available should return True when both credentials are present."""
        provider = EIDRProvider()
        # Mock httpx module
        mock_httpx = MagicMock()
        with patch.dict('sys.modules', {'httpx': mock_httpx}):
            # Mock credential manager to return fake credentials
            def get_cred(provider_name, field_name):
                if field_name == "client_id":
                    return "fake_client_id"
                elif field_name == "client_secret":
                    return "fake_client_secret"
                return None

            with patch.object(provider._credential_manager, 'get_credential', side_effect=get_cred):
                assert provider.is_available() is True

    def test_eidr_id_format(self):
        """EIDR IDs should use DOI format 10.5240/XXXX-XXXX-XXXX-XXXX-XXXX-C."""
        provider = EIDRProvider()

        # Parse an EIDR record with standard DOI format
        item = {
            "id": "10.5240/ABCD-1234-5678-9012-3456-K",
            "title": "Test Movie",
            "year": "2024",
        }

        result = provider._parse_eidr_record(item)
        assert result is not None
        assert result.provider_id == "10.5240/ABCD-1234-5678-9012-3456-K"
        assert result.extra_tags["custom_eidr_id"] == "10.5240/ABCD-1234-5678-9012-3456-K"

    def test_resolve_endpoint_with_mocked_response(self):
        """Resolve endpoint should fetch EIDR record by ID."""
        provider = EIDRProvider()

        # Mock EIDR resolve response
        mock_response_data = {
            "id": "10.5240/1234-5678-9ABC-DEF0-1234-L",
            "title": "The Shawshank Redemption",
            "ReferentName": "The Shawshank Redemption",
            "year": "1994",
            "ReleaseDate": "1994-09-23",
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

        # Mock credentials
        def get_cred(provider_name, field_name):
            if field_name == "client_id":
                return "test_client_id"
            elif field_name == "client_secret":
                return "test_client_secret"
            return None

        # Mock httpx module to make is_available() return True
        mock_httpx = MagicMock()

        with patch.dict('sys.modules', {'httpx': mock_httpx}):
            with patch.object(provider, '_get_http_client', return_value=mock_client):
                with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                    with patch.object(provider._credential_manager, 'get_credential', side_effect=get_cred):
                        # Run async lookup
                        eidr_id = "10.5240/1234-5678-9ABC-DEF0-1234-L"
                        result = asyncio.run(provider.lookup_by_id(eidr_id))

                        # Verify result
                        assert result is not None
                        assert result.title == "The Shawshank Redemption"
                        assert result.year == "1994"
                        assert result.provider_id == "10.5240/1234-5678-9ABC-DEF0-1234-L"
                        assert result.extra_tags["custom_eidr_id"] == eidr_id

                        # Verify HTTP request had Basic Auth header
                        mock_client.get.assert_called_once()
                        call_args = mock_client.get.call_args
                        headers = call_args[1]["headers"]
                        assert "Authorization" in headers
                        assert headers["Authorization"].startswith("Basic ")

    def test_basic_auth_encoding(self):
        """HTTP Basic Auth should be properly base64-encoded."""
        provider = EIDRProvider()

        # Mock credentials
        def get_cred(provider_name, field_name):
            if field_name == "client_id":
                return "user123"
            elif field_name == "client_secret":
                return "pass456"
            return None

        with patch.object(provider._credential_manager, 'get_credential', side_effect=get_cred):
            auth_header = provider._get_basic_auth()

            # Verify it's base64 encoded
            assert auth_header is not None
            # Decode to verify format (should be "user123:pass456")
            import base64
            decoded = base64.b64decode(auth_header).decode()
            assert decoded == "user123:pass456"

    def test_search_returns_empty_without_credentials(self):
        """Search should gracefully return empty results without credentials."""
        provider = EIDRProvider()

        # Mock no credentials
        with patch.object(provider._credential_manager, 'get_credential', return_value=None):
            query = {"title": "Test Movie"}
            results = asyncio.run(provider.search(query))

            # Should return empty list gracefully
            assert results == []

    def test_eidr_has_no_cover_art(self):
        """EIDR provider should not provide cover art."""
        provider = EIDRProvider()

        item = {
            "id": "10.5240/TEST-TEST-TEST-TEST-TEST-T",
            "title": "Test",
        }

        result = provider._parse_eidr_record(item)
        assert result is not None
        assert result.cover_art == []                     # No cover art from EIDR
        assert result.provider_url == ""                  # EIDR has no public detail pages
