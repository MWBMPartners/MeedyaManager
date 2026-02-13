# ============================================================================
# File: /metadata/providers/identifiers/eidr.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# EIDR (Entertainment Identifier Registry) metadata provider.
# Provides standardized identifiers for movies, TV shows, and other
# audiovisual content. Requires paid membership and HTTP Basic Auth.
#
# EIDR IDs use DOI format: 10.5240/XXXX-XXXX-XXXX-XXXX-XXXX-C
#
# Most users will not have EIDR credentials. This provider gracefully
# reports as unavailable when credentials are missing.
#
# API: https://resolve.eidr.org/ and https://registry.eidr.org/
# Rate limit: 10 requests per 10 seconds (conservative estimate)
# ============================================================================

import logging                                             # Standard logging
import base64                                              # For HTTP Basic Auth encoding

from metadata.providers import ProviderCategory, register_provider
from metadata.providers.base import (
    BaseProvider,                                           # Provider ABC
    ProviderCapabilities,                                   # Capabilities declaration
    ProviderResult,                                        # Result dataclass
)
from metadata.providers.credentials import CredentialManager
from metadata.providers.rate_limiter import get_rate_limiter

logger = logging.getLogger("MeedyaManager.Provider.EIDR")

# ============================================================================
# EIDR API Constants
# ============================================================================
RESOLVE_URL = "https://resolve.eidr.org/EIDR/object"      # EIDR resolution endpoint
REGISTRY_URL = "https://registry.eidr.org/EIDR"           # EIDR registry endpoint


@register_provider
class EIDRProvider(BaseProvider):
    """EIDR (Entertainment Identifier Registry) metadata provider.

    Provides standardized identifiers for audiovisual content. Requires
    paid EIDR membership with client ID and client secret credentials.

    Most users will not have EIDR access. This provider gracefully reports
    as unavailable when credentials are missing.
    """

    provider_name = "eidr"                                 # Unique provider identifier

    def __init__(self):
        """Initialise the EIDR provider."""
        super().__init__()
        self._credential_manager = CredentialManager()
        self._rate_limiter = get_rate_limiter("eidr")
        self._http_client = None                           # Lazy httpx client

    @property
    def category(self) -> ProviderCategory:
        """EIDR is an identifier provider."""
        return ProviderCategory.IDENTIFIER

    @property
    def capabilities(self) -> ProviderCapabilities:
        """EIDR supports movie and show identification."""
        return ProviderCapabilities(
            can_search_movies=True,                        # Can identify movies
            can_search_shows=True,                         # Can identify TV shows
        )

    @property
    def requires_auth(self) -> bool:
        """EIDR requires HTTP Basic Auth credentials."""
        return True

    def is_available(self) -> bool:
        """Check if EIDR provider is available with valid credentials."""
        try:
            import httpx                                   # Verify httpx is installed
        except ImportError:
            logger.warning("httpx not installed — EIDR provider unavailable")
            return False

        # Check for credentials (most users won't have these)
        client_id = self._credential_manager.get_credential("eidr", "client_id")
        client_secret = self._credential_manager.get_credential("eidr", "client_secret")

        if not client_id or not client_secret:
            logger.debug("EIDR credentials not configured (EIDR membership required)")
            return False

        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search EIDR for matching identifiers.

        This is a simplified implementation that returns empty results
        gracefully when credentials are not available. A full implementation
        would query the EIDR registry with XML payloads.

        Args:
            query: dict with keys: title, show, year, media_class.

        Returns:
            list[ProviderResult]: Matching EIDR results (empty if no access).
        """
        # Gracefully return empty results if no credentials
        if not self.is_available():
            return []

        search_parts = []
        if query.get("title"):
            search_parts.append(query["title"])
        if query.get("show"):
            search_parts.append(query["show"])

        if not search_parts:
            return []

        search_term = " ".join(search_parts)
        year = query.get("year", "")

        return await self._search_eidr(search_term, year)

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific EIDR ID.

        Args:
            provider_id: EIDR ID in format 10.5240/XXXX-XXXX-XXXX-XXXX-XXXX-C

        Returns:
            ProviderResult if found, None otherwise.
        """
        try:
            if not self.is_available():
                return None

            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            # Get HTTP Basic Auth credentials
            auth = self._get_basic_auth()
            if not auth:
                return None

            # Resolve EIDR ID
            url = f"{RESOLVE_URL}/{provider_id}"
            headers = {
                "Authorization": f"Basic {auth}",
                "Accept": "application/json",
            }

            response = await client.get(url, headers=headers)
            response.raise_for_status()
            data = response.json()

            return self._parse_eidr_record(data)

        except Exception as e:
            logger.error(f"EIDR lookup failed for {provider_id}: {e}")
            return None

    async def _search_eidr(self, term: str, year: str = "") -> list[ProviderResult]:
        """Search EIDR registry for matching records.

        This is a simplified implementation. A full implementation would
        construct XML query payloads according to EIDR specifications.

        Args:
            term: Search query string.
            year: Release year (optional).

        Returns:
            list[ProviderResult]: Matching EIDR results.
        """
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            # Get HTTP Basic Auth credentials
            auth = self._get_basic_auth()
            if not auth:
                return []

            # Simplified search implementation
            # Real implementation would use XML POST to /EIDR/query
            headers = {
                "Authorization": f"Basic {auth}",
                "Content-Type": "application/xml",
                "Accept": "application/json",
            }

            # Construct simple XML query (simplified)
            xml_query = f"""<?xml version="1.0" encoding="UTF-8"?>
<SimpleQuery xmlns="http://www.eidr.org/schema">
    <Expression>
        <Title>{term}</Title>
    </Expression>
</SimpleQuery>"""

            url = f"{REGISTRY_URL}/query"
            response = await client.post(url, headers=headers, content=xml_query)
            response.raise_for_status()
            data = response.json()

            results = []
            # Parse response (format varies - simplified)
            for item in data.get("results", []):
                result = self._parse_eidr_record(item)
                if result:
                    results.append(result)

            return results

        except Exception as e:
            logger.error(f"EIDR search failed: {e}")
            return []

    def _get_basic_auth(self) -> str | None:
        """Get HTTP Basic Auth header value.

        Returns:
            Base64-encoded "client_id:client_secret" string, or None.
        """
        client_id = self._credential_manager.get_credential("eidr", "client_id")
        client_secret = self._credential_manager.get_credential("eidr", "client_secret")

        if not client_id or not client_secret:
            return None

        credentials = f"{client_id}:{client_secret}"
        encoded = base64.b64encode(credentials.encode()).decode()
        return encoded

    def _parse_eidr_record(self, item: dict) -> ProviderResult | None:
        """Parse an EIDR record into a ProviderResult.

        Args:
            item: A single EIDR record from the API.

        Returns:
            ProviderResult with EIDR ID.
        """
        try:
            eidr_id = item.get("id", "") or item.get("EIDR", "")
            title = item.get("title", "") or item.get("ReferentName", "")
            year = str(item.get("year", "") or item.get("ReleaseDate", "")[:4])

            # Extra tags
            extra_tags = {
                "custom_eidr_id": eidr_id,
            }

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                year=year,
                provider_id=eidr_id,
                provider_url="",                           # EIDR has no public detail pages
                cover_art=[],                              # EIDR doesn't provide cover art
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse EIDR record: {e}")
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
