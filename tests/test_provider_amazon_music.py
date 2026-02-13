# ============================================================================
# File: /tests/test_provider_amazon_music.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Amazon Music metadata provider (closed beta API stub).
# Tests provider name/category, availability (always False), search (returns empty),
# and URL construction for manual reference.
#
# All tests verify that the provider gracefully degrades when the API
# is not available.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.music.amazon_music import AmazonMusicProvider  # Provider under test


# =============================================================================
# Test Class — Amazon Music Provider Tests
# =============================================================================

class TestAmazonMusicProvider:
    """Tests for the AmazonMusicProvider class."""

    def test_provider_name(self):
        """Provider name should be 'amazon_music'."""
        provider = AmazonMusicProvider()
        assert provider.provider_name == "amazon_music"

    def test_category(self):
        """Category should be ProviderCategory.MUSIC."""
        provider = AmazonMusicProvider()
        assert provider.category == ProviderCategory.MUSIC

    def test_capabilities_can_search_tracks(self):
        """Capabilities should include can_search_tracks=True (when available)."""
        provider = AmazonMusicProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is True

    def test_capabilities_has_static_cover_art(self):
        """Capabilities should include has_static_cover_art=True (when available)."""
        provider = AmazonMusicProvider()
        caps = provider.capabilities
        assert caps.has_static_cover_art is True

    def test_requires_auth(self):
        """Provider should require authentication (when API becomes available)."""
        provider = AmazonMusicProvider()
        assert provider.requires_auth is True

    def test_is_available_returns_false(self):
        """is_available should return False since API is in closed beta."""
        provider = AmazonMusicProvider()
        assert provider.is_available() is False

    def test_search_returns_empty_list(self):
        """Search should return empty list since API is not available."""
        provider = AmazonMusicProvider()

        # Search with valid query
        query = {"title": "Test Song", "artist": "Test Artist"}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_lookup_by_id_returns_none(self):
        """lookup_by_id should return None since API is not available."""
        provider = AmazonMusicProvider()

        # Lookup with valid ID
        result = asyncio.run(provider.lookup_by_id("B08XYZZZZZ"))

        assert result is None

    def test_get_status_info_returns_helpful_message(self):
        """get_status_info should return a helpful message about closed beta."""
        provider = AmazonMusicProvider()
        status = provider.get_status_info()

        assert status["name"] == "amazon_music"
        assert status["category"] == "music"
        assert status["available"] is False
        assert status["requires_auth"] is True
        assert "closed beta" in status["message"].lower()

    def test_construct_search_url(self):
        """_construct_search_url should build a valid Amazon Music search URL."""
        provider = AmazonMusicProvider()

        # Construct URL for a search query
        url = provider._construct_search_url("Test Artist Test Song")

        assert url.startswith("https://music.amazon.com/search/")
        assert "Test" in url
