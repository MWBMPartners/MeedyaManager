# ============================================================================
# File: /tests/test_lookup_service.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the LookupService orchestrator:
# - Provider selection (by name, category, availability)
# - Multi-provider parallel lookup
# - Result scoring and ranking
# - apply_result() tag writing
# - apply_result() dry-run mode
# - batch_lookup() processing
# - Synchronous wrappers (lookup_sync, apply_result_sync)
# - Error handling for failed providers
# ============================================================================

import asyncio                                             # For running async tests
import pytest                                              # Test framework
from unittest.mock import patch, MagicMock, AsyncMock      # Mocking

from metadata.providers import (
    ProviderCategory,                                      # Category enum
    ProviderRegistry,                                      # Registry class
)
from metadata.providers.base import (
    BaseProvider,                                           # Abstract base class
    ProviderCapabilities,                                   # Capabilities dataclass
    ProviderResult,                                        # Result dataclass
    CoverArtAsset,                                         # Cover art asset
    CoverArtType,                                          # Cover art type enum
)
from metadata.lookup_service import LookupService          # Main class under test


# =============================================================================
# Mock Providers
# =============================================================================

class _MockMusicProvider(BaseProvider):
    """Mock music provider that returns predictable results."""
    provider_name = "mock_music"

    @property
    def category(self) -> ProviderCategory:
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        return ProviderCapabilities(can_search_tracks=True)

    @property
    def requires_auth(self) -> bool:
        return False

    def is_available(self) -> bool:
        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        return [
            ProviderResult(
                provider_name=self.provider_name,
                title=query.get("title", "Mock Title"),
                artist=query.get("artist", "Mock Artist"),
                album="Mock Album",
                provider_id="mock_123",
            ),
        ]


class _MockVideoProvider(BaseProvider):
    """Mock video provider for testing category filtering."""
    provider_name = "mock_video"

    @property
    def category(self) -> ProviderCategory:
        return ProviderCategory.VIDEO

    @property
    def capabilities(self) -> ProviderCapabilities:
        return ProviderCapabilities(can_search_movies=True)

    @property
    def requires_auth(self) -> bool:
        return False

    def is_available(self) -> bool:
        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        return [
            ProviderResult(
                provider_name=self.provider_name,
                title=query.get("title", "Mock Movie"),
                provider_id="video_456",
            ),
        ]


class _MockFailingProvider(BaseProvider):
    """Mock provider that always raises an exception during search."""
    provider_name = "mock_failing"

    @property
    def category(self) -> ProviderCategory:
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        return ProviderCapabilities(can_search_tracks=True)

    @property
    def requires_auth(self) -> bool:
        return False

    def is_available(self) -> bool:
        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        raise ConnectionError("API unavailable")


# =============================================================================
# Fixtures
# =============================================================================

@pytest.fixture
def mock_registry():
    """Create a ProviderRegistry with mock providers pre-registered."""
    registry = ProviderRegistry()
    registry._discovered = True                            # Skip auto-discovery
    registry.register(_MockMusicProvider)
    registry.register(_MockVideoProvider)
    return registry


@pytest.fixture
def service(mock_registry):
    """Create a LookupService with mock registry."""
    svc = LookupService()
    svc._registry = mock_registry
    return svc


@pytest.fixture
def query_metadata():
    """Standard query metadata for testing."""
    return {
        "title": "Test Song",
        "artist": "Test Artist",
        "album": "Test Album",
        "year": "2025",
    }


# =============================================================================
# Provider Selection Tests
# =============================================================================

class TestProviderSelection:
    """Tests for _select_providers() method."""

    def test_select_all_available(self, service):
        """No filters should return all available providers."""
        providers = service._select_providers(names=None, category=None)
        names = [p.provider_name for p in providers]
        assert "mock_music" in names
        assert "mock_video" in names

    def test_select_by_name(self, service):
        """Specific provider names should filter to just those providers."""
        providers = service._select_providers(names=["mock_music"], category=None)
        assert len(providers) == 1
        assert providers[0].provider_name == "mock_music"

    def test_select_nonexistent_name(self, service):
        """Non-existent provider name should be skipped with warning."""
        providers = service._select_providers(names=["nonexistent"], category=None)
        assert len(providers) == 0

    def test_select_by_category(self, service):
        """Category filter should return only matching providers."""
        providers = service._select_providers(names=None, category=ProviderCategory.MUSIC)
        names = [p.provider_name for p in providers]
        assert "mock_music" in names
        assert "mock_video" not in names


# =============================================================================
# Lookup Tests
# =============================================================================

class TestLookup:
    """Tests for the main lookup() method."""

    def test_lookup_returns_results(self, service, query_metadata):
        """lookup() should return scored results from providers."""
        async def run():
            return await service.lookup(query_metadata)

        results = asyncio.run(run())
        assert len(results) >= 1
        # Results should have confidence scores set
        assert all(isinstance(r.confidence, float) for r in results)

    def test_lookup_filters_by_provider(self, service, query_metadata):
        """lookup() with provider filter should only use specified providers."""
        async def run():
            return await service.lookup(
                query_metadata, providers=["mock_music"]
            )

        results = asyncio.run(run())
        provider_names = [r.provider_name for r in results]
        assert all(n == "mock_music" for n in provider_names)

    def test_lookup_min_confidence_filter(self, service, query_metadata):
        """lookup() should filter results below min_confidence."""
        async def run():
            return await service.lookup(
                query_metadata, min_confidence=0.999
            )

        results = asyncio.run(run())
        # With very high threshold, most results should be filtered out
        for r in results:
            assert r.confidence >= 0.999

    def test_lookup_empty_providers(self, service, query_metadata):
        """lookup() with no available providers should return empty list."""
        # Create service with empty registry
        empty_registry = ProviderRegistry()
        empty_registry._discovered = True
        service._registry = empty_registry

        async def run():
            return await service.lookup(query_metadata)

        results = asyncio.run(run())
        assert results == []

    def test_lookup_handles_failing_provider(self, service, query_metadata):
        """lookup() should continue even if one provider fails."""
        # Add a failing provider to the registry
        service._registry.register(_MockFailingProvider)

        async def run():
            return await service.lookup(query_metadata)

        results = asyncio.run(run())
        # Should still get results from non-failing providers
        assert len(results) >= 1


# =============================================================================
# Sync Wrapper Tests
# =============================================================================

class TestLookupSync:
    """Tests for the lookup_sync() synchronous wrapper."""

    def test_lookup_sync_returns_results(self, service, query_metadata):
        """lookup_sync() should return the same results as the async version."""
        results = service.lookup_sync(query_metadata)
        assert isinstance(results, list)
        assert len(results) >= 1

    def test_lookup_sync_with_kwargs(self, service, query_metadata):
        """lookup_sync() should pass kwargs to the async lookup."""
        results = service.lookup_sync(
            query_metadata, providers=["mock_music"]
        )
        provider_names = [r.provider_name for r in results]
        assert all(n == "mock_music" for n in provider_names)


# =============================================================================
# apply_result() Tests
# =============================================================================

class TestApplyResult:
    """Tests for applying a result to a media file."""

    def test_apply_dry_run(self, service, tmp_path):
        """apply_result() in dry-run mode should not write anything."""
        media_file = tmp_path / "test.mp3"
        media_file.write_bytes(b"FAKE_AUDIO")
        result = ProviderResult(
            provider_name="test",
            title="Test Song",
            artist="Test Artist",
            provider_id="test_123",
        )

        async def run():
            return await service.apply_result(
                str(media_file), result, dry_run=True
            )

        changes = asyncio.run(run())
        assert changes["dry_run"] is True
        assert len(changes["tags_written"]) > 0            # Tags computed but not written

    def test_apply_with_tags(self, service, tmp_path):
        """apply_result() should write tags via TagEditor."""
        media_file = tmp_path / "test.mp3"
        media_file.write_bytes(b"FAKE_AUDIO")
        result = ProviderResult(
            provider_name="test",
            title="Test Song",
            artist="Test Artist",
            provider_id="test_123",
        )

        async def run():
            mock_instance = MagicMock()
            mock_instance.write_tags.return_value = True
            with patch("metadata.editor.TagEditor", return_value=mock_instance):
                return await service.apply_result(
                    str(media_file), result, download_art=False
                )

        changes = asyncio.run(run())
        assert changes["dry_run"] is False
        assert "title" in changes["tags_written"]

    def test_apply_no_tags_no_art(self, service, tmp_path):
        """apply_result() with write_tags=False and download_art=False should skip both."""
        media_file = tmp_path / "test.mp3"
        media_file.write_bytes(b"FAKE_AUDIO")
        result = ProviderResult(provider_name="test", title="Test")

        async def run():
            return await service.apply_result(
                str(media_file), result,
                write_tags=False, download_art=False,
            )

        changes = asyncio.run(run())
        assert changes["tags_written"] == {}
        assert changes["cover_art_saved"] == {}

    def test_apply_with_cover_art_dry_run(self, service, tmp_path):
        """apply_result() dry-run should preview cover art downloads."""
        media_file = tmp_path / "test.mp3"
        media_file.write_bytes(b"FAKE_AUDIO")
        result = ProviderResult(
            provider_name="test",
            title="Test",
            cover_art=[
                CoverArtAsset(
                    url="https://example.com/cover.jpg",
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                ),
            ],
        )

        async def run():
            return await service.apply_result(
                str(media_file), result, dry_run=True
            )

        changes = asyncio.run(run())
        assert "static" in changes["cover_art_saved"]
        assert "DRY RUN" in changes["cover_art_saved"]["static"]


# =============================================================================
# apply_result_sync() Tests
# =============================================================================

class TestApplyResultSync:
    """Tests for the apply_result_sync() synchronous wrapper."""

    def test_apply_result_sync_dry_run(self, service, tmp_path):
        """apply_result_sync() should work the same as the async version."""
        media_file = tmp_path / "test.mp3"
        media_file.write_bytes(b"FAKE_AUDIO")
        result = ProviderResult(
            provider_name="test", title="Test Song", provider_id="abc",
        )
        changes = service.apply_result_sync(
            str(media_file), result, dry_run=True
        )
        assert changes["dry_run"] is True


# =============================================================================
# get_available_providers() Tests
# =============================================================================

class TestGetAvailableProviders:
    """Tests for the provider status listing."""

    def test_get_available_providers(self, service):
        """Should return a list of status dicts for all providers."""
        providers = service.get_available_providers()
        assert isinstance(providers, list)
        assert len(providers) >= 2                          # mock_music and mock_video

        # Check structure of first status dict
        status = providers[0]
        assert "name" in status
        assert "category" in status
        assert "available" in status
        assert "message" in status
