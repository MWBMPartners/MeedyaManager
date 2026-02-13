# ============================================================================
# File: /tests/test_provider_base.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the provider framework base classes:
# - ProviderCategory enum
# - ProviderCapabilities dataclass
# - CoverArtAsset dataclass
# - CoverArtType enum
# - ProviderResult dataclass (including tag extraction methods)
# - BaseProvider ABC
# - ProviderRegistry (register, get, discover, category filter)
# - register_provider decorator
# ============================================================================

import pytest                                              # Test framework

from metadata.providers import (
    ProviderCategory,                                      # Category enum
    ProviderRegistry,                                      # Registry class
    PROVIDER_REGISTRY,                                     # Global singleton
    register_provider,                                     # Decorator
)
from metadata.providers.base import (
    BaseProvider,                                           # Abstract base class
    ProviderCapabilities,                                   # Capabilities dataclass
    ProviderResult,                                        # Result dataclass
    CoverArtAsset,                                         # Cover art asset
    CoverArtType,                                          # Cover art type enum
)


# =============================================================================
# Fixtures — Test-specific providers and registries
# =============================================================================

class _MockProvider(BaseProvider):
    """Concrete implementation of BaseProvider for testing."""
    provider_name = "mock_test"

    @property
    def category(self) -> ProviderCategory:
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        return ProviderCapabilities(
            can_search_tracks=True,
            has_static_cover_art=True,
        )

    @property
    def requires_auth(self) -> bool:
        return False

    def is_available(self) -> bool:
        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        return [ProviderResult(provider_name=self.provider_name, title="Mock")]


class _UnavailableProvider(BaseProvider):
    """A provider that is always unavailable (for testing filters)."""
    provider_name = "unavailable_test"

    @property
    def category(self) -> ProviderCategory:
        return ProviderCategory.VIDEO

    @property
    def capabilities(self) -> ProviderCapabilities:
        return ProviderCapabilities(can_search_movies=True)

    @property
    def requires_auth(self) -> bool:
        return True

    def is_available(self) -> bool:
        return False

    async def search(self, query: dict) -> list[ProviderResult]:
        return []


@pytest.fixture
def fresh_registry():
    """Create a fresh ProviderRegistry (not the global singleton)."""
    registry = ProviderRegistry()
    registry._discovered = True                            # Skip auto-discovery
    return registry


# =============================================================================
# ProviderCategory Tests
# =============================================================================

class TestProviderCategory:
    """Tests for the ProviderCategory enum."""

    def test_music_value(self):
        """MUSIC category should have value 'music'."""
        assert ProviderCategory.MUSIC.value == "music"

    def test_video_value(self):
        """VIDEO category should have value 'video'."""
        assert ProviderCategory.VIDEO.value == "video"

    def test_podcast_value(self):
        """PODCAST category should have value 'podcast'."""
        assert ProviderCategory.PODCAST.value == "podcast"

    def test_identifier_value(self):
        """IDENTIFIER category should have value 'identifier'."""
        assert ProviderCategory.IDENTIFIER.value == "identifier"

    def test_all_categories_exist(self):
        """All expected categories should be defined."""
        expected = {"MUSIC", "VIDEO", "PODCAST", "IDENTIFIER"}
        actual = {c.name for c in ProviderCategory}
        assert actual == expected


# =============================================================================
# CoverArtType Tests
# =============================================================================

class TestCoverArtType:
    """Tests for the CoverArtType enum."""

    def test_static_value(self):
        """STATIC type should have value 'static'."""
        assert CoverArtType.STATIC.value == "static"

    def test_animated_square_value(self):
        """ANIMATED_SQUARE should have value 'animated_square'."""
        assert CoverArtType.ANIMATED_SQUARE.value == "animated_square"

    def test_animated_portrait_value(self):
        """ANIMATED_PORTRAIT should have value 'animated_portrait'."""
        assert CoverArtType.ANIMATED_PORTRAIT.value == "animated_portrait"

    def test_artist_spotlight_value(self):
        """ARTIST_SPOTLIGHT should have value 'artist_spotlight'."""
        assert CoverArtType.ARTIST_SPOTLIGHT.value == "artist_spotlight"


# =============================================================================
# ProviderCapabilities Tests
# =============================================================================

class TestProviderCapabilities:
    """Tests for the ProviderCapabilities dataclass."""

    def test_default_all_false(self):
        """Default capabilities should all be False."""
        caps = ProviderCapabilities()
        assert not caps.can_search_tracks
        assert not caps.can_search_albums
        assert not caps.has_static_cover_art
        assert not caps.has_animated_cover_art
        assert not caps.has_audio_features
        assert not caps.can_fingerprint_audio

    def test_custom_capabilities(self):
        """Setting specific capabilities should work."""
        caps = ProviderCapabilities(
            can_search_tracks=True,
            has_static_cover_art=True,
            has_animated_cover_art=True,
        )
        assert caps.can_search_tracks is True
        assert caps.has_static_cover_art is True
        assert caps.has_animated_cover_art is True
        assert caps.can_search_albums is False             # Other fields still False


# =============================================================================
# CoverArtAsset Tests
# =============================================================================

class TestCoverArtAsset:
    """Tests for the CoverArtAsset dataclass."""

    def test_create_static_asset(self):
        """Creating a static cover art asset should set all fields."""
        asset = CoverArtAsset(
            url="https://example.com/cover.jpg",
            asset_type=CoverArtType.STATIC,
            format="jpeg",
            width=3000,
            height=3000,
        )
        assert asset.url == "https://example.com/cover.jpg"
        assert asset.asset_type == CoverArtType.STATIC
        assert asset.format == "jpeg"
        assert asset.width == 3000
        assert asset.height == 3000

    def test_create_animated_asset(self):
        """Creating an animated cover art asset should set format to mp4."""
        asset = CoverArtAsset(
            url="https://example.com/video.mp4",
            asset_type=CoverArtType.ANIMATED_SQUARE,
            format="mp4",
        )
        assert asset.asset_type == CoverArtType.ANIMATED_SQUARE
        assert asset.format == "mp4"

    def test_default_dimensions(self):
        """Default dimensions should be 0x0 (unknown)."""
        asset = CoverArtAsset(
            url="https://example.com/art.jpg",
            asset_type=CoverArtType.STATIC,
        )
        assert asset.width == 0
        assert asset.height == 0


# =============================================================================
# ProviderResult Tests
# =============================================================================

class TestProviderResult:
    """Tests for the ProviderResult dataclass and its tag methods."""

    def test_create_basic_result(self):
        """Creating a result with basic fields should work."""
        result = ProviderResult(
            provider_name="spotify",
            title="Test Song",
            artist="Test Artist",
            album="Test Album",
            confidence=0.95,
        )
        assert result.provider_name == "spotify"
        assert result.title == "Test Song"
        assert result.artist == "Test Artist"
        assert result.confidence == 0.95

    def test_get_standard_tags_non_empty(self):
        """get_standard_tags() should return only non-empty fields."""
        result = ProviderResult(
            provider_name="spotify",
            title="My Song",
            artist="My Artist",
            album="",                                      # Empty — should be excluded
            year="2025",
        )
        tags = result.get_standard_tags()
        assert tags["title"] == "My Song"
        assert tags["artist"] == "My Artist"
        assert tags["year"] == "2025"
        assert "album" not in tags                         # Empty field excluded

    def test_get_standard_tags_empty_result(self):
        """get_standard_tags() on an empty result should return empty dict."""
        result = ProviderResult(provider_name="test")
        tags = result.get_standard_tags()
        assert tags == {}

    def test_get_custom_tags_with_provider_id(self):
        """get_custom_tags() should include provider ID and URL."""
        result = ProviderResult(
            provider_name="spotify",
            provider_id="abc123",
            provider_url="https://open.spotify.com/track/abc123",
        )
        tags = result.get_custom_tags()
        assert tags["custom_spotify_id"] == "abc123"
        assert tags["custom_spotify_url"] == "https://open.spotify.com/track/abc123"

    def test_get_custom_tags_with_extra(self):
        """get_custom_tags() should include extra_tags dict."""
        result = ProviderResult(
            provider_name="spotify",
            provider_id="abc123",
            extra_tags={
                "custom_spotify_energy": "0.85",
                "custom_spotify_tempo": "120",
            },
        )
        tags = result.get_custom_tags()
        assert tags["custom_spotify_energy"] == "0.85"
        assert tags["custom_spotify_tempo"] == "120"
        assert "custom_spotify_id" in tags

    def test_get_all_tags_combines(self):
        """get_all_tags() should combine standard and custom tags."""
        result = ProviderResult(
            provider_name="spotify",
            title="My Song",
            artist="My Artist",
            provider_id="abc123",
            extra_tags={"custom_spotify_energy": "0.85"},
        )
        all_tags = result.get_all_tags()
        assert all_tags["title"] == "My Song"              # Standard tag
        assert all_tags["custom_spotify_id"] == "abc123"   # Custom tag
        assert all_tags["custom_spotify_energy"] == "0.85" # Extra tag

    def test_cover_art_list_default_empty(self):
        """cover_art should default to an empty list."""
        result = ProviderResult(provider_name="test")
        assert result.cover_art == []

    def test_cover_art_list_with_assets(self):
        """cover_art should hold CoverArtAsset instances."""
        assets = [
            CoverArtAsset(url="https://ex.com/front.jpg", asset_type=CoverArtType.STATIC),
            CoverArtAsset(url="https://ex.com/video.mp4", asset_type=CoverArtType.ANIMATED_SQUARE, format="mp4"),
        ]
        result = ProviderResult(provider_name="apple_music", cover_art=assets)
        assert len(result.cover_art) == 2
        assert result.cover_art[0].asset_type == CoverArtType.STATIC
        assert result.cover_art[1].format == "mp4"


# =============================================================================
# BaseProvider ABC Tests
# =============================================================================

class TestBaseProvider:
    """Tests for the BaseProvider abstract base class."""

    def test_concrete_provider_instantiation(self):
        """A properly implemented provider should instantiate."""
        provider = _MockProvider()
        assert provider.provider_name == "mock_test"
        assert provider.category == ProviderCategory.MUSIC
        assert provider.is_available() is True

    def test_requires_auth_false(self):
        """Mock provider should not require auth."""
        provider = _MockProvider()
        assert provider.requires_auth is False

    def test_capabilities(self):
        """Mock provider capabilities should be correctly set."""
        provider = _MockProvider()
        caps = provider.capabilities
        assert caps.can_search_tracks is True
        assert caps.has_static_cover_art is True
        assert caps.can_search_albums is False

    def test_get_status_info_available(self):
        """get_status_info() for an available provider should show 'Available'."""
        provider = _MockProvider()
        status = provider.get_status_info()
        assert status["name"] == "mock_test"
        assert status["category"] == "music"
        assert status["available"] is True
        assert status["message"] == "Available"

    def test_get_status_info_unavailable(self):
        """get_status_info() for an unavailable provider should show 'Missing credentials'."""
        provider = _UnavailableProvider()
        status = provider.get_status_info()
        assert status["available"] is False
        assert status["requires_auth"] is True
        assert "credentials" in status["message"].lower() or "unavailable" in status["message"].lower()

    def test_lookup_by_id_default_none(self):
        """Default lookup_by_id() should return None."""
        import asyncio
        provider = _MockProvider()
        result = asyncio.run(provider.lookup_by_id("anything"))
        assert result is None


# =============================================================================
# ProviderRegistry Tests
# =============================================================================

class TestProviderRegistry:
    """Tests for the ProviderRegistry class."""

    def test_register_and_get(self, fresh_registry):
        """Registering a provider should make it retrievable by name."""
        fresh_registry.register(_MockProvider)
        provider = fresh_registry.get_provider("mock_test")
        assert provider is not None
        assert provider.provider_name == "mock_test"

    def test_get_nonexistent_returns_none(self, fresh_registry):
        """Getting an unregistered provider should return None."""
        provider = fresh_registry.get_provider("nonexistent")
        assert provider is None

    def test_get_all(self, fresh_registry):
        """get_all() should return instances for all registered providers."""
        fresh_registry.register(_MockProvider)
        fresh_registry.register(_UnavailableProvider)
        all_providers = fresh_registry.get_all()
        names = [p.provider_name for p in all_providers]
        assert "mock_test" in names
        assert "unavailable_test" in names

    def test_get_by_category_music(self, fresh_registry):
        """get_by_category(MUSIC) should return only music providers."""
        fresh_registry.register(_MockProvider)
        fresh_registry.register(_UnavailableProvider)
        music_providers = fresh_registry.get_by_category(ProviderCategory.MUSIC)
        names = [p.provider_name for p in music_providers]
        assert "mock_test" in names
        assert "unavailable_test" not in names             # _UnavailableProvider is VIDEO

    def test_get_available(self, fresh_registry):
        """get_available() should return only providers where is_available() is True."""
        fresh_registry.register(_MockProvider)
        fresh_registry.register(_UnavailableProvider)
        available = fresh_registry.get_available()
        names = [p.provider_name for p in available]
        assert "mock_test" in names
        assert "unavailable_test" not in names

    def test_get_registered_names(self, fresh_registry):
        """get_registered_names() should return sorted list of names."""
        fresh_registry.register(_UnavailableProvider)
        fresh_registry.register(_MockProvider)
        names = fresh_registry.get_registered_names()
        assert names == ["mock_test", "unavailable_test"]  # Sorted alphabetically

    def test_cached_instances(self, fresh_registry):
        """Repeated get_provider() calls should return the same instance."""
        fresh_registry.register(_MockProvider)
        p1 = fresh_registry.get_provider("mock_test")
        p2 = fresh_registry.get_provider("mock_test")
        assert p1 is p2                                    # Same object identity


# =============================================================================
# register_provider Decorator Tests
# =============================================================================

class TestRegisterProviderDecorator:
    """Tests for the @register_provider decorator."""

    def test_decorator_registers_class(self):
        """@register_provider should register the class in the global registry."""
        @register_provider
        class _DecoratorTestProvider(BaseProvider):
            provider_name = "decorator_test"

            @property
            def category(self):
                return ProviderCategory.IDENTIFIER

            @property
            def capabilities(self):
                return ProviderCapabilities()

            @property
            def requires_auth(self):
                return False

            def is_available(self):
                return True

            async def search(self, query):
                return []

        # Should be findable in the global registry
        assert "decorator_test" in PROVIDER_REGISTRY._providers

    def test_decorator_returns_class(self):
        """@register_provider should return the class unchanged."""
        @register_provider
        class _ReturnTestProvider(BaseProvider):
            provider_name = "return_test"

            @property
            def category(self):
                return ProviderCategory.MUSIC

            @property
            def capabilities(self):
                return ProviderCapabilities()

            @property
            def requires_auth(self):
                return False

            def is_available(self):
                return True

            async def search(self, query):
                return []

        assert _ReturnTestProvider.provider_name == "return_test"
