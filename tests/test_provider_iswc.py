# ============================================================================
# File: /tests/test_provider_iswc.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the ISWC identifier provider.
# Tests provider capabilities, ISWC format validation, ISWC normalization,
# MusicBrainz work lookup with mocked responses, and custom tags.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP and credentials
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.identifiers.iswc import ISWCProvider  # Provider under test


# =============================================================================
# Test Class — ISWC Provider Tests
# =============================================================================

class TestISWCProvider:
    """Tests for the ISWCProvider class."""

    def test_provider_name(self):
        """Provider name should be 'iswc'."""
        provider = ISWCProvider()
        assert provider.provider_name == "iswc"

    def test_category(self):
        """Category should be ProviderCategory.IDENTIFIER."""
        provider = ISWCProvider()
        assert provider.category == ProviderCategory.IDENTIFIER

    def test_requires_auth(self):
        """Provider should not require authentication."""
        provider = ISWCProvider()
        assert provider.requires_auth is False

    def test_is_available(self):
        """Provider should always be available."""
        provider = ISWCProvider()
        assert provider.is_available() is True

    def test_iswc_format_validation_valid(self):
        """Valid ISWC codes should pass validation."""
        provider = ISWCProvider()

        # Valid ISWC codes
        valid_iswcs = [
            "T-123.456.789-0",
            "T-000.000.001-5",
            "T-999.999.999-9",
        ]

        for iswc in valid_iswcs:
            assert provider._validate_iswc(iswc) is True

    def test_iswc_format_validation_invalid(self):
        """Invalid ISWC codes should fail validation."""
        provider = ISWCProvider()

        # Invalid ISWC codes
        invalid_iswcs = [
            "INVALID",                                     # Wrong format
            "123.456.789-0",                               # Missing T prefix
            "T-123.456.789",                               # Missing check digit
            "T-123.456-0",                                 # Missing middle segment
            "T-12.456.789-0",                              # Wrong segment length
        ]

        for iswc in invalid_iswcs:
            assert provider._validate_iswc(iswc) is False

    def test_iswc_normalization(self):
        """ISWC codes should be normalized to standard format."""
        provider = ISWCProvider()

        # Test various input formats
        assert provider._normalize_iswc("T1234567890") == "T-123.456.789-0"
        assert provider._normalize_iswc("T-123-456-789-0") == "T-123.456.789-0"
        assert provider._normalize_iswc("t1234567890") == "T-123.456.789-0"

    def test_iswc_lookup_with_mocked_response(self):
        """ISWC lookup should query MusicBrainz works and parse results."""
        provider = ISWCProvider()

        # Mock MusicBrainz work response
        mock_response_data = {
            "works": [
                {
                    "id": "work-iswc-123",
                    "title": "Test Composition",
                    "iswc": "T-123.456.789-0"
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

        # Mock rate limiter
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        with patch.object(provider, '_get_http_client', return_value=mock_client):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                # Run search with ISWC
                query = {"iswc": "T-123.456.789-0"}
                results = asyncio.run(provider.search(query))

                # Verify results
                assert len(results) == 1
                result = results[0]
                assert result.title == "Test Composition"
                assert result.provider_id == "work-iswc-123"

    def test_iswc_custom_tags(self):
        """Results should include custom ISWC tags."""
        provider = ISWCProvider()

        # Mock work
        work = {
            "id": "work-test-456",
            "title": "Test Work Title"
        }

        result = provider._parse_work(work, iswc="T-123.456.789-0")

        # Verify ISWC custom tags
        assert "custom_iswc" in result.extra_tags
        assert result.extra_tags["custom_iswc"] == "T-123.456.789-0"
        assert "custom_iswc_work_title" in result.extra_tags
        assert result.extra_tags["custom_iswc_work_title"] == "Test Work Title"

    def test_musicbrainz_work_id_in_custom_tags(self):
        """Results should include MusicBrainz work ID."""
        provider = ISWCProvider()

        # Mock work with MBID
        work = {
            "id": "work-mbid-789",
            "title": "Work MBID Test"
        }

        result = provider._parse_work(work, iswc="T-123.456.789-0")

        # Verify work MBID in extra tags
        assert "custom_musicbrainz_work_id" in result.extra_tags
        assert result.extra_tags["custom_musicbrainz_work_id"] == "work-mbid-789"

    def test_search_returns_empty_when_invalid_iswc(self):
        """Search should return empty list when ISWC format is invalid."""
        provider = ISWCProvider()

        # Invalid ISWC
        query = {"iswc": "INVALID_ISWC"}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_search_returns_empty_when_no_iswc(self):
        """Search should return empty list when no ISWC provided."""
        provider = ISWCProvider()

        # Empty query
        query = {}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_provider_url_includes_work_id(self):
        """Provider URL should link to MusicBrainz work page."""
        provider = ISWCProvider()

        # Mock work
        work = {
            "id": "work-url-test-123",
            "title": "URL Test Work"
        }

        result = provider._parse_work(work, iswc="T-123.456.789-0")

        # Verify MusicBrainz work URL
        assert "musicbrainz.org/work/work-url-test-123" in result.provider_url

    def test_get_status_info(self):
        """get_status_info should return correct structure."""
        provider = ISWCProvider()

        status = provider.get_status_info()

        assert status["name"] == "iswc"
        assert status["category"] == "identifier"
        assert status["available"] is True
        assert status["requires_auth"] is False
        assert status["message"] == "Available"
