# ============================================================================
# File: /tests/test_provider_musicbrainz.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the MusicBrainz metadata provider.
# Tests provider capabilities, User-Agent header, search functionality,
# ISRC lookup, MBID extraction, Cover Art Archive URL construction,
# and Lucene query escaping.
#
# All tests are offline — HTTP calls are mocked using unittest.mock.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import MagicMock, AsyncMock, patch      # Mock HTTP and credentials
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.music.musicbrainz import MusicBrainzProvider  # Provider under test
from metadata.providers.base import CoverArtType           # Cover art type enum


# =============================================================================
# Test Class — MusicBrainz Provider Tests
# =============================================================================

class TestMusicBrainzProvider:
    """Tests for the MusicBrainzProvider class."""

    def test_provider_name(self):
        """Provider name should be 'musicbrainz'."""
        provider = MusicBrainzProvider()
        assert provider.provider_name == "musicbrainz"

    def test_category(self):
        """Category should be ProviderCategory.MUSIC."""
        provider = MusicBrainzProvider()
        assert provider.category == ProviderCategory.MUSIC

    def test_capabilities_can_search_tracks(self):
        """Capabilities should include can_search_tracks=True."""
        provider = MusicBrainzProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is True

    def test_capabilities_can_search_albums(self):
        """Capabilities should include can_search_albums=True."""
        provider = MusicBrainzProvider()
        caps = provider.capabilities
        assert caps.can_search_albums is True

    def test_capabilities_can_lookup_isrc(self):
        """Capabilities should include can_lookup_isrc=True."""
        provider = MusicBrainzProvider()
        caps = provider.capabilities
        assert caps.can_lookup_isrc is True

    def test_capabilities_has_static_cover_art(self):
        """Capabilities should include has_static_cover_art=True."""
        provider = MusicBrainzProvider()
        caps = provider.capabilities
        assert caps.has_static_cover_art is True

    def test_requires_auth(self):
        """Provider should not require authentication."""
        provider = MusicBrainzProvider()
        assert provider.requires_auth is False

    def test_is_available(self):
        """Provider should always be available (no credentials needed)."""
        provider = MusicBrainzProvider()
        assert provider.is_available() is True

    def test_user_agent_header_in_request(self):
        """HTTP requests should include mandatory User-Agent header."""
        provider = MusicBrainzProvider()

        # Mock response
        mock_response = MagicMock()
        mock_response.json.return_value = {"recordings": []}
        mock_response.raise_for_status = MagicMock()

        # Mock HTTP client
        mock_client = AsyncMock()
        mock_client.get = AsyncMock(return_value=mock_response)

        # Mock rate limiter
        mock_rate_limiter = AsyncMock()
        mock_rate_limiter.acquire = AsyncMock()

        with patch.object(provider, '_get_http_client', return_value=mock_client):
            with patch.object(provider, '_rate_limiter', mock_rate_limiter):
                # Run search
                query = {"title": "Test Song"}
                asyncio.run(provider.search(query))

                # Verify User-Agent header was sent
                assert mock_client.get.called
                call_args = mock_client.get.call_args
                headers = call_args[1]['headers']
                assert 'User-Agent' in headers
                assert 'MeedyaManager' in headers['User-Agent']

    def test_search_with_text_query(self):
        """Text search should build Lucene query and return results."""
        provider = MusicBrainzProvider()

        # Mock API response
        mock_response_data = {
            "recordings": [
                {
                    "id": "abc123-def456",
                    "title": "Test Song",
                    "artist-credit": [
                        {"name": "Test Artist"}
                    ],
                    "releases": [
                        {
                            "id": "release-mbid-123",
                            "title": "Test Album",
                            "date": "2025-01-15"
                        }
                    ],
                    "isrcs": ["USTEST1234567"]
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
                # Run search
                query = {"title": "Test Song", "artist": "Test Artist"}
                results = asyncio.run(provider.search(query))

                # Verify results
                assert len(results) == 1
                result = results[0]
                assert result.title == "Test Song"
                assert result.artist == "Test Artist"
                assert result.album == "Test Album"
                assert result.isrc == "USTEST1234567"
                assert result.year == "2025"

    def test_isrc_lookup(self):
        """ISRC lookup should use ISRC endpoint and return results."""
        provider = MusicBrainzProvider()

        # Mock API response for ISRC lookup
        mock_response_data = {
            "recordings": [
                {
                    "id": "recording-mbid-789",
                    "title": "ISRC Test Song",
                    "artist-credit": [
                        {"name": "ISRC Artist"}
                    ],
                    "releases": [
                        {
                            "id": "release-mbid-789",
                            "title": "ISRC Album",
                            "date": "2024-06-10"
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
                query = {"isrc": "USTEST7654321"}
                results = asyncio.run(provider.search(query))

                # Verify ISRC endpoint was called
                assert mock_client.get.called
                call_args = mock_client.get.call_args
                url = call_args[0][0]
                assert "/isrc/" in url

                # Verify results
                assert len(results) == 1
                result = results[0]
                assert result.title == "ISRC Test Song"
                assert result.isrc == "USTEST7654321"

    def test_mbid_extraction(self):
        """Should extract recording MBID, release MBID, and artist MBID."""
        provider = MusicBrainzProvider()

        # Mock recording with all MBIDs
        recording = {
            "id": "recording-mbid-abc",
            "title": "MBID Test",
            "artist-credit": [
                {
                    "name": "MBID Artist",
                    "artist": {"id": "artist-mbid-xyz"}
                }
            ],
            "releases": [
                {
                    "id": "release-mbid-def",
                    "title": "MBID Album",
                    "date": "2025"
                }
            ]
        }

        result = provider._parse_recording(recording, isrc="")

        # Verify MBIDs in extra tags
        assert result.extra_tags["custom_musicbrainz_recording_id"] == "recording-mbid-abc"
        assert result.extra_tags["custom_musicbrainz_release_id"] == "release-mbid-def"
        assert result.extra_tags["custom_musicbrainz_artist_id"] == "artist-mbid-xyz"

    def test_cover_art_archive_url(self):
        """Should construct Cover Art Archive URL from release MBID."""
        provider = MusicBrainzProvider()

        # Mock recording with release MBID
        recording = {
            "id": "recording-123",
            "title": "Cover Art Test",
            "artist-credit": [{"name": "Test Artist"}],
            "releases": [
                {
                    "id": "release-mbid-cover",
                    "title": "Test Album"
                }
            ]
        }

        result = provider._parse_recording(recording, isrc="")

        # Verify cover art URL
        assert len(result.cover_art) == 1
        cover = result.cover_art[0]
        assert cover.asset_type == CoverArtType.STATIC
        assert "coverartarchive.org" in cover.url
        assert "release-mbid-cover" in cover.url
        assert "/front-500" in cover.url

    def test_lucene_query_escaping(self):
        """Special characters in query should be escaped for Lucene."""
        provider = MusicBrainzProvider()

        # Test string with special Lucene characters
        test_string = "Test: Song (Live) [2025]"
        escaped = provider._escape_lucene(test_string)

        # Verify special characters are escaped
        assert "\\:" in escaped
        assert "\\(" in escaped
        assert "\\)" in escaped
        assert "\\[" in escaped
        assert "\\]" in escaped

    def test_search_returns_empty_list_when_no_query_terms(self):
        """Search should return empty list when no query terms provided."""
        provider = MusicBrainzProvider()

        # Empty query
        query = {}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_multiple_artist_credits(self):
        """Should concatenate multiple artist names for collaborations."""
        provider = MusicBrainzProvider()

        # Mock recording with multiple artists
        recording = {
            "id": "collab-123",
            "title": "Collab Song",
            "artist-credit": [
                {"name": "Artist One"},
                {"name": " feat. "},
                {"name": "Artist Two"}
            ],
            "releases": []
        }

        result = provider._parse_recording(recording, isrc="")

        # Verify artists are concatenated
        assert result.artist == "Artist One feat. Artist Two"

    def test_musicbrainz_url_in_extra_tags(self):
        """Should include MusicBrainz URL in extra tags."""
        provider = MusicBrainzProvider()

        recording = {
            "id": "url-test-123",
            "title": "URL Test",
            "artist-credit": [{"name": "URL Artist"}],
            "releases": []
        }

        result = provider._parse_recording(recording, isrc="")

        # Verify MusicBrainz URL
        assert "custom_musicbrainz_url" in result.extra_tags
        assert "musicbrainz.org/recording/url-test-123" in result.extra_tags["custom_musicbrainz_url"]

    def test_get_status_info(self):
        """get_status_info should return correct structure."""
        provider = MusicBrainzProvider()

        status = provider.get_status_info()

        assert status["name"] == "musicbrainz"
        assert status["category"] == "music"
        assert status["available"] is True
        assert status["requires_auth"] is False
        assert status["message"] == "Available"
