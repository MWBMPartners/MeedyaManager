# ============================================================================
# File: /tests/test_provider_pandora.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the Pandora metadata provider stub (no public API).
# Tests provider name/category, availability (always False), search (returns empty),
# and URL construction for manual reference.
#
# All tests verify that the provider gracefully handles the absence of
# a public API.
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from metadata.providers import ProviderCategory            # Provider category enum
from metadata.providers.music.pandora import PandoraProvider  # Provider under test


# =============================================================================
# Test Class — Pandora Provider Tests
# =============================================================================

class TestPandoraProvider:
    """Tests for the PandoraProvider class."""

    def test_provider_name(self):
        """Provider name should be 'pandora'."""
        provider = PandoraProvider()
        assert provider.provider_name == "pandora"

    def test_category(self):
        """Category should be ProviderCategory.MUSIC."""
        provider = PandoraProvider()
        assert provider.category == ProviderCategory.MUSIC

    def test_capabilities_all_false(self):
        """All capabilities should be False since there is no API."""
        provider = PandoraProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is False
        assert caps.can_search_albums is False
        assert caps.can_search_artists is False
        assert caps.has_static_cover_art is False

    def test_requires_auth(self):
        """Provider should not require authentication (no API available)."""
        provider = PandoraProvider()
        assert provider.requires_auth is False

    def test_is_available_returns_false(self):
        """is_available should return False since Pandora has no public API."""
        provider = PandoraProvider()
        assert provider.is_available() is False

    def test_search_returns_empty_list(self):
        """Search should return empty list since no API is available."""
        provider = PandoraProvider()

        # Search with valid query
        query = {"title": "Test Song", "artist": "Test Artist"}
        results = asyncio.run(provider.search(query))

        assert results == []

    def test_lookup_by_id_returns_none(self):
        """lookup_by_id should return None since no API is available."""
        provider = PandoraProvider()

        # Lookup with valid ID
        result = asyncio.run(provider.lookup_by_id("12345"))

        assert result is None

    def test_get_status_info_returns_helpful_message(self):
        """get_status_info should return a message about no public API."""
        provider = PandoraProvider()
        status = provider.get_status_info()

        assert status["name"] == "pandora"
        assert status["category"] == "music"
        assert status["available"] is False
        assert status["requires_auth"] is False
        assert "does not provide a public api" in status["message"].lower()

    def test_construct_search_url(self):
        """_construct_search_url should build a valid Pandora search URL."""
        provider = PandoraProvider()

        # Construct URL for a search query
        url = provider._construct_search_url("Test Artist Test Song")

        assert url.startswith("https://www.pandora.com/search/")
        assert "Test" in url
