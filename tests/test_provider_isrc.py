# ============================================================================
# File: /tests/test_provider_isrc.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the ISRC identifier provider.
# Tests provider capabilities, ISRC format validation, ISRC normalization,
# MusicBrainz lookup with mocked responses, result parsing, and custom tags.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP and credentials
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.identifiers.isrc import ISRCProvider  # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — ISRC Provider Tests
# =============================================================================

class TestISRCProvider:
    """Tests for the ISRCProvider class."""

    def test_provider_name(self):
        """Provider name should be 'isrc'."""
        provider = ISRCProvider()
        assert provider.provider_name == "isrc"

    def test_category(self):
        """Category should be ProviderCategory.IDENTIFIER."""
        provider = ISRCProvider()
        assert provider.category == ProviderCategory.IDENTIFIER

    def test_capabilities_can_lookup_isrc(self):
        """Capabilities should include can_lookup_isrc=True."""
        provider = ISRCProvider()
        caps = provider.capabilities
        assert caps.can_lookup_isrc is True

    def test_capabilities_can_lookup_upc(self):
        """Capabilities should include can_lookup_upc=True."""
        provider = ISRCProvider()
        caps = provider.capabilities
        assert caps.can_lookup_upc is True

    def test_requires_auth(self):
        """Provider should not require authentication."""
        provider = ISRCProvider()
        assert provider.requires_auth is False

    def test_is_available(self):
        """Provider should always be available."""
        provider = ISRCProvider()
        assert provider.is_available() is True

    def test_isrc_format_validation_valid(self):
        """Valid ISRC codes should pass validation."""
        provider = ISRCProvider()

        # Valid ISRC codes (12 chars: 2 letter country + 3 alphanumeric registrant + 2 digit year + 5 digit designation)
        valid_isrcs = [
            "USRC11234567",                                # US, RC1 (registrant), 12 (year), 34567 (designation)
            "GBXYZ9912345",                                # GB, XYZ, 99, 12345
            "JPABC0512345",                                # JP, ABC, 05, 12345
        ]

        for isrc in valid_isrcs:
            normalized = provider._normalize_isrc(isrc)
            assert provider._validate_isrc(normalized) is True

    def test_isrc_format_validation_invalid(self):
        """Invalid ISRC codes should fail validation."""
        provider = ISRCProvider()

        # Invalid ISRC codes
        invalid_isrcs = [
            "INVALID",                                     # Too short
            "USRC1123456",                                 # Missing one digit (11 chars)
            "USRC112345678",                               # Too long (13 chars)
            "12RC11234567",                                # Numbers in country code
            "USRC1A34567",                                 # Letter in year position
        ]

        for isrc in invalid_isrcs:
            normalized = provider._normalize_isrc(isrc)
            assert provider._validate_isrc(normalized) is False

    def test_isrc_normalization(self):
        """ISRC codes should be normalized (uppercase, no hyphens)."""
        provider = ISRCProvider()

        # Test various input formats
        assert provider._normalize_isrc("US-RC1-12-34567") == "USRC11234567"
        assert provider._normalize_isrc("us-rc1-12-34567") == "USRC11234567"
        assert provider._normalize_isrc("usrc11234567") == "USRC11234567"

    def test_isrc_lookup_with_mocked_response(self):
        """ISRC lookup should query MusicBrainz and parse results."""
        provider = ISRCProvider()

        # Mock MusicBrainz ISRC response
        mock_response_data = {
            "recordings": [
                {
                    "id": "recording-isrc-123",
                    "title": "ISRC Lookup Test",
                    "artist-credit": [
                        {"name": "ISRC Test Artist"}
                    ],
                    "releases": [
                        {
                            "id": "release-isrc-456",
                            "title": "ISRC Test Album",
                            "date": "2025-02-10"
                        }
                    ]
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
                # Run search with ISRC
                query = {"isrc": "USRC11234567"}
                results = asyncio.run(provider.search(query))

                # Verify results
                assert len(results) == 1
                result = results[0]
                assert result.title == "ISRC Lookup Test"
                assert result.artist == "ISRC Test Artist"
                assert result.album == "ISRC Test Album"
                assert result.isrc == "USRC11234567"
                assert result.year == "2025"

    def test_isrc_source_custom_tag(self):
        """Results should include custom_isrc_source tag."""
        provider = ISRCProvider()

        # Mock recording
        recording = {
            "id": "test-123",
            "title": "Source Test",
            "artist-credit": [{"name": "Test Artist"}],
            "releases": []
        }

        result = provider._parse_recording(recording, isrc="USRC11234567")

        # Verify ISRC source tag
        assert "custom_isrc_source" in result.extra_tags
        assert result.extra_tags["custom_isrc_source"] == "musicbrainz"

    def test_musicbrainz_ids_in_custom_tags(self):
        """Results should include MusicBrainz recording and release IDs."""
        provider = ISRCProvider()

        # Mock recording with MBIDs
        recording = {
            "id": "recording-mbid-789",
            "title": "MBID Test",
            "artist-credit": [{"name": "MBID Artist"}],
            "releases": [
                {
                    "id": "release-mbid-012",
                    "title": "MBID Album"
                }
            ]
        }

        result = provider._parse_recording(recording, isrc="USRC11234567")

        # Verify MBIDs in extra tags
        assert "custom_musicbrainz_recording_id" in result.extra_tags
        assert result.extra_tags["custom_musicbrainz_recording_id"] == "recording-mbid-789"
        assert "custom_musicbrainz_release_id" in result.extra_tags
        assert result.extra_tags["custom_musicbrainz_release_id"] == "release-mbid-012"

    def test_cover_art_from_cover_art_archive(self):
        """Results should include Cover Art Archive URL if release MBID present."""
        provider = ISRCProvider()

        # Mock recording with release MBID
        recording = {
            "id": "recording-345",
            "title": "Cover Test",
            "artist-credit": [{"name": "Cover Artist"}],
            "releases": [
                {
                    "id": "release-cover-678",
                    "title": "Cover Album"
                }
            ]
        }

        result = provider._parse_recording(recording, isrc="USRC11234567")

        # Verify cover art
        assert len(result.cover_art) == 1
        cover = result.cover_art[0]
        assert cover.asset_type == CoverArtType.STATIC
        assert "coverartarchive.org" in cover.url
        assert "release-cover-678" in cover.url

    def test_search_returns_empty_when_invalid_isrc(self):
        """Search should return empty list when ISRC format is invalid."""
        provider = ISRCProvider()

        # Invalid ISRC
        query = {"isrc": "INVALID_ISRC"}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_search_returns_empty_when_no_isrc(self):
        """Search should return empty list when no ISRC provided."""
        provider = ISRCProvider()

        # Empty query
        query = {}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_get_status_info(self):
        """get_status_info should return correct structure."""
        provider = ISRCProvider()

        status = provider.get_status_info()

        assert status["name"] == "isrc"
        assert status["category"] == "identifier"
        assert status["available"] is True
        assert status["requires_auth"] is False
        assert status["message"] == "Available"
