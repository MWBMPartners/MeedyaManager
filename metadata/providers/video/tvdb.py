# ============================================================================
# File: /metadata/providers/video/tvdb.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# TheTVDB metadata provider for TV shows and episodes.
# Uses the TVDB API v4 with JWT token authentication.
# Requires a free API key from https://thetvdb.com
#
# Returns metadata including show name, overview, year, poster images,
# status, and series slugs for URL construction.
#
# API: https://api4.thetvdb.com/v4/
# Rate limit: 1 request per second (conservative estimate)
# ============================================================================

import logging                                             # Standard logging

from metadata.providers import ProviderCategory, register_provider
from metadata.providers.base import (
    BaseProvider,                                           # Provider ABC
    ProviderCapabilities,                                   # Capabilities declaration
    ProviderResult,                                        # Result dataclass
    CoverArtAsset,                                         # Cover art asset
    CoverArtType,                                          # Cover art type enum
)
from metadata.providers.credentials import CredentialManager
from metadata.providers.rate_limiter import get_rate_limiter

logger = logging.getLogger("MeedyaManager.Provider.TVDB")

# ============================================================================
# TVDB API Constants
# ============================================================================
API_BASE_URL = "https://api4.thetvdb.com/v4"              # TVDB API v4 base URL
LOGIN_URL = f"{API_BASE_URL}/login"                       # JWT token endpoint
SEARCH_URL = f"{API_BASE_URL}/search"                     # Search endpoint
TVDB_SERIES_URL = "https://thetvdb.com/series"            # Series detail page URL


@register_provider
class TVDBProvider(BaseProvider):
    """TheTVDB metadata provider for TV shows and episodes.

    Requires a free API key from thetvdb.com. Uses JWT token authentication
    for API v4. Searches for TV series and returns metadata including show
    name, overview, year, status, poster images, and series slugs.
    """

    provider_name = "tvdb"                                 # Unique provider identifier

    def __init__(self):
        """Initialise the TVDB provider."""
        super().__init__()
        self._credential_manager = CredentialManager()
        self._rate_limiter = get_rate_limiter("tvdb")
        self._http_client = None                           # Lazy httpx client
        self._jwt_token = None                             # Cached JWT token
        self._token_expires = 0                            # Token expiry timestamp

    @property
    def category(self) -> ProviderCategory:
        """TVDB is a video provider."""
        return ProviderCategory.VIDEO

    @property
    def capabilities(self) -> ProviderCapabilities:
        """TVDB supports show and episode search with static cover art."""
        return ProviderCapabilities(
            can_search_shows=True,                         # Search TV series
            can_search_episodes=True,                      # Search TV episodes
            has_static_cover_art=True,                     # Poster/cover art (JPEG)
        )

    @property
    def requires_auth(self) -> bool:
        """TVDB requires an API key for authentication."""
        return True

    def is_available(self) -> bool:
        """Check if TVDB provider is available with valid credentials."""
        try:
            import httpx                                   # Verify httpx is installed
        except ImportError:
            logger.warning("httpx not installed — TVDB provider unavailable")
            return False

        # Check for API key
        api_key = self._credential_manager.get_credential("tvdb", "api_key")
        if not api_key:
            logger.debug("TVDB API key not configured")
            return False

        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search TVDB for matching TV shows.

        Args:
            query: dict with keys: title, show, season, episode, media_class.

        Returns:
            list[ProviderResult]: Matching TV show results.
        """
        search_parts = []
        if query.get("title"):
            search_parts.append(query["title"])
        if query.get("show"):
            search_parts.append(query["show"])

        if not search_parts:
            return []

        search_term = " ".join(search_parts)

        return await self._search_series(search_term)

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific TV series by TVDB ID.

        Args:
            provider_id: TVDB series ID (numeric string).

        Returns:
            ProviderResult if found, None otherwise.
        """
        try:
            token = await self._get_jwt_token()
            if not token:
                return None

            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            headers = {"Authorization": f"Bearer {token}"}
            url = f"{API_BASE_URL}/series/{provider_id}/extended"

            response = await client.get(url, headers=headers)
            response.raise_for_status()
            data = response.json()

            series_data = data.get("data", {})
            if series_data:
                return self._parse_series(series_data)
            return None

        except Exception as e:
            logger.error(f"TVDB lookup failed for {provider_id}: {e}")
            return None

    async def _search_series(self, term: str) -> list[ProviderResult]:
        """Search TVDB for TV series.

        Args:
            term: Search query string.

        Returns:
            list[ProviderResult]: Matching series results.
        """
        try:
            token = await self._get_jwt_token()
            if not token:
                return []

            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            headers = {"Authorization": f"Bearer {token}"}
            params = {
                "query": term,
                "type": "series",                          # Search for series only
            }

            response = await client.get(SEARCH_URL, headers=headers, params=params)
            response.raise_for_status()
            data = response.json()

            results = []
            for item in data.get("data", []):
                result = self._parse_series(item)
                if result:
                    results.append(result)
            return results

        except Exception as e:
            logger.error(f"TVDB series search failed: {e}")
            return []

    async def _get_jwt_token(self) -> str | None:
        """Obtain or return cached JWT authentication token.

        TVDB API v4 requires a JWT token obtained by POSTing the API key
        to the /login endpoint. Tokens are cached and reused until expiry.

        Returns:
            JWT token string, or None if authentication fails.
        """
        import time

        # Return cached token if still valid (with 5 minute buffer)
        if self._jwt_token and time.time() < (self._token_expires - 300):
            return self._jwt_token

        # Get fresh token
        try:
            api_key = self._credential_manager.get_credential("tvdb", "api_key")
            if not api_key:
                return None

            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            payload = {"apikey": api_key}
            response = await client.post(LOGIN_URL, json=payload)
            response.raise_for_status()
            data = response.json()

            # Extract token from response
            token = data.get("data", {}).get("token")
            if not token:
                logger.error("TVDB login response missing token")
                return None

            # Cache token (TVDB tokens last 24 hours typically)
            self._jwt_token = token
            self._token_expires = time.time() + 86400      # 24 hours from now

            logger.debug("TVDB JWT token obtained successfully")
            return token

        except Exception as e:
            logger.error(f"TVDB authentication failed: {e}")
            return None

    def _parse_series(self, item: dict) -> ProviderResult | None:
        """Parse a TV series item into a ProviderResult.

        Args:
            item: A single series result object from TVDB API.

        Returns:
            ProviderResult with metadata and cover art.
        """
        try:
            series_id = str(item.get("id", "") or item.get("tvdb_id", ""))
            name = item.get("name", "")
            overview = item.get("overview", "")
            year = str(item.get("year", ""))
            status = item.get("status", {})
            if isinstance(status, dict):
                status = status.get("name", "")
            slug = item.get("slug", "")
            image_url = item.get("image", "") or item.get("image_url", "")

            # Cover art — static JPEG from TVDB
            cover_art = []
            if image_url:
                # TVDB v4 returns full URLs
                cover_art.append(CoverArtAsset(
                    url=image_url,
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                    width=0,                               # TVDB doesn't specify dimensions
                    height=0,
                    description="TVDB series poster",
                ))

            # Extra tags
            extra_tags = {
                "custom_tvdb_id": series_id,
            }
            if slug:
                extra_tags["custom_tvdb_slug"] = slug
                extra_tags["custom_tvdb_url"] = f"{TVDB_SERIES_URL}/{slug}"
            if overview:
                extra_tags["custom_tvdb_overview"] = overview[:500]
            if status:
                extra_tags["custom_tvdb_status"] = status

            return ProviderResult(
                provider_name=self.provider_name,
                title=name,
                show=name,                                 # TV shows use 'name' field
                year=year,
                provider_id=series_id,
                provider_url=f"{TVDB_SERIES_URL}/{slug}" if slug else "",
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse TVDB series item: {e}")
            return None

    async def _get_http_client(self):
        """Get or create the httpx async client."""
        if self._http_client is None:
            import httpx
            self._http_client = httpx.AsyncClient(
                timeout=30.0,
                follow_redirects=True,
            )
        return self._http_client
