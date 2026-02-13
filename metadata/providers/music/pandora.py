# ============================================================================
# File: /metadata/providers/music/pandora.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Pandora metadata provider stub.
# Pandora does not provide a public API for metadata lookup or search.
# This provider is a framework stub that gracefully handles the absence
# of an API by always returning unavailable status and empty results.
#
# Authentication:
# Not applicable — Pandora has no public API.
#
# URL construction is included for manual reference to the Pandora
# web interface search.
# ============================================================================

import logging                                             # Standard logging

from metadata.providers import ProviderCategory, register_provider
from metadata.providers.base import (
    BaseProvider,                                           # Provider ABC
    ProviderCapabilities,                                   # Capabilities declaration
    ProviderResult,                                        # Result dataclass
)
from metadata.providers.credentials import CredentialManager
from metadata.providers.rate_limiter import get_rate_limiter

logger = logging.getLogger("MeedyaManager.Provider.Pandora")


@register_provider
class PandoraProvider(BaseProvider):
    """Pandora metadata provider stub (no public API).

    Pandora does not provide a public API for music metadata lookup.
    This provider exists as a placeholder in the framework and will
    always return unavailable status with an explanatory message.
    """

    provider_name = "pandora"                              # Unique provider identifier

    def __init__(self):
        """Initialise the Pandora provider."""
        super().__init__()
        self._credentials = CredentialManager()            # Credential resolution
        self._rate_limiter = get_rate_limiter("pandora")   # Rate limiter

    @property
    def category(self) -> ProviderCategory:
        """Pandora is a music provider."""
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        """Pandora has no capabilities (no public API)."""
        return ProviderCapabilities(
            # All capabilities set to False — no API access
            can_search_tracks=False,
            can_search_albums=False,
            can_search_artists=False,
            has_static_cover_art=False,
        )

    @property
    def requires_auth(self) -> bool:
        """Pandora does not require authentication (no API available)."""
        return False

    def is_available(self) -> bool:
        """Check if Pandora API is available.

        Always returns False since Pandora does not provide a public API.

        Returns:
            bool: False (no public API).
        """
        logger.debug("Pandora does not provide a public API")
        return False

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search Pandora catalog for matching tracks.

        Always returns empty list since Pandora has no public API.

        Args:
            query: dict with keys: title, artist, album.

        Returns:
            list[ProviderResult]: Empty list (no API available).
        """
        # Log informational message about API status
        logger.info("Pandora has no public API — search skipped")
        return []

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific track by Pandora ID.

        Always returns None since Pandora has no public API.

        Args:
            provider_id: Pandora track ID.

        Returns:
            None: No API available.
        """
        logger.info(f"Pandora has no public API — lookup for {provider_id} skipped")
        return None

    def get_status_info(self) -> dict:
        """Get status information for Pandora provider.

        Overrides the default implementation to provide a clear message
        about the lack of a public API.

        Returns:
            dict: Status info with message about no public API.
        """
        return {
            "name": self.provider_name,
            "category": self.category.value,
            "requires_auth": self.requires_auth,
            "available": False,
            "message": "Pandora does not provide a public API",
        }

    # ========================================================================
    # URL Construction (for manual reference)
    # ========================================================================

    def _construct_search_url(self, query: str) -> str:
        """Construct a manual search URL for Pandora web interface.

        This is for reference only — no API is available for programmatic access.

        Args:
            query: Search query string.

        Returns:
            str: Pandora search URL.
        """
        # URL encode the query for safe inclusion in URL
        import urllib.parse
        encoded_query = urllib.parse.quote(query)
        return f"https://www.pandora.com/search/{encoded_query}"
