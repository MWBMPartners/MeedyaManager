# ============================================================================
# File: /metadata/providers/music/iheart.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# iHeartRadio metadata provider using undocumented public endpoints.
# Supports track search with basic metadata and static cover art.
#
# Authentication:
# None required — uses publicly accessible API endpoints.
# Note: These endpoints are undocumented and may change without notice.
#
# API Endpoint:
# GET https://api.iheart.com/api/v3/search/all?keywords={query}&maxRows=10
#
# Response includes songs with: title, artistName, albumName, imageUrl, id, lyrics
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

logger = logging.getLogger("MeedyaManager.Provider.iHeart")

# ============================================================================
# iHeartRadio API Constants
# ============================================================================
API_BASE = "https://api.iheart.com"                        # iHeart API base URL
SEARCH_ENDPOINT = "/api/v3/search/all"                     # Search endpoint (undocumented)


@register_provider
class IHeartProvider(BaseProvider):
    """iHeartRadio metadata provider using undocumented public endpoints.

    Provides basic music metadata including:
    - Track search with title, artist, album
    - Static cover art (JPEG images)
    - Lyrics (when available)

    No authentication required (uses public endpoints).
    Note: API is undocumented and may change without notice.
    """

    provider_name = "iheart"                               # Unique provider identifier

    def __init__(self):
        """Initialise the iHeartRadio provider."""
        super().__init__()
        self._credentials = CredentialManager()            # Credential resolution
        self._rate_limiter = get_rate_limiter("iheart")    # Rate limiter
        self._http_client = None                           # Lazy httpx client

    @property
    def category(self) -> ProviderCategory:
        """iHeartRadio is a music provider."""
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        """iHeartRadio supports track search with static cover art."""
        return ProviderCapabilities(
            can_search_tracks=True,                        # Search individual songs
            has_static_cover_art=True,                     # JPEG cover art
        )

    @property
    def requires_auth(self) -> bool:
        """iHeartRadio does not require authentication."""
        return False

    def is_available(self) -> bool:
        """Check if iHeartRadio provider is available.

        Always returns True since no authentication is required and
        the API endpoints are publicly accessible (best-effort).

        Returns:
            bool: True (no auth needed).
        """
        # No authentication needed — always available
        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search iHeartRadio catalog for matching tracks.

        Uses the undocumented /api/v3/search/all endpoint to search
        for songs by title, artist, and album.

        Args:
            query: dict with keys: title, artist, album.

        Returns:
            list[ProviderResult]: Matching results with cover art assets.
        """
        # Build search term from available metadata
        search_parts = []
        if query.get("title"):
            search_parts.append(query["title"])
        if query.get("artist"):
            search_parts.append(query["artist"])
        if query.get("album"):
            search_parts.append(query["album"])

        if not search_parts:
            logger.warning("iHeart search: no query terms provided")
            return []

        search_term = " ".join(search_parts)

        # Make the API request
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = API_BASE + SEARCH_ENDPOINT
            params = {
                "keywords": search_term,                   # Search query
                "maxRows": 10,                             # Max 10 results
            }

            response = await client.get(url, params=params)
            response.raise_for_status()
            data = response.json()

            return self._parse_search_results(data)

        except Exception as e:
            logger.error(f"iHeart search failed: {e}")
            return []

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific song by its iHeartRadio ID.

        Note: Direct song lookup endpoint is not documented.
        This method returns None as we don't have a reliable lookup endpoint.

        Args:
            provider_id: iHeartRadio song ID.

        Returns:
            None: Direct lookup not supported.
        """
        logger.debug(f"iHeart direct lookup not supported for {provider_id}")
        return None

    # ========================================================================
    # Response Parsing
    # ========================================================================

    def _parse_search_results(self, data: dict) -> list[ProviderResult]:
        """Parse iHeartRadio search API response into ProviderResult list.

        Args:
            data: Raw JSON response from the search endpoint.

        Returns:
            list[ProviderResult]: Parsed results with cover art assets.
        """
        results = []

        # Extract songs from the response
        # The API returns: {results: {songs: [...]}}
        songs = data.get("results", {}).get("songs", [])

        for song in songs:
            result = self._parse_song(song)
            if result:
                results.append(result)

        return results

    def _parse_song(self, song: dict) -> ProviderResult | None:
        """Parse a single iHeartRadio song object into a ProviderResult.

        Extracts standard metadata, song ID, image URL, and lyrics (if available).

        Args:
            song: A single song object from the iHeart API.

        Returns:
            ProviderResult with metadata and cover art assets.
        """
        try:
            # Extract standard metadata
            title = song.get("title", "")
            artist = song.get("artistName", "")
            album = song.get("albumName", "")
            song_id = str(song.get("id", ""))
            lyrics = song.get("lyrics", "")

            # Build iHeartRadio URL (web interface URL format)
            url = f"https://www.iheart.com/artist/{artist}/{song_id}" if song_id and artist else ""

            # Extract cover art from imageUrl
            cover_art = self._extract_cover_art(song)

            # Build extra tags
            extra_tags = {}
            if song_id:
                extra_tags["custom_iheart_id"] = song_id
            if url:
                extra_tags["custom_iheart_url"] = url

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                artist=artist,
                album=album,
                lyrics=lyrics if lyrics else "",
                provider_id=song_id,
                provider_url=url,
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse iHeart song: {e}")
            return None

    def _extract_cover_art(self, song: dict) -> list[CoverArtAsset]:
        """Extract cover art from song imageUrl.

        Args:
            song: Song object from iHeart API.

        Returns:
            list[CoverArtAsset]: Cover art assets (static JPEG).
        """
        assets = []

        # Get image URL
        image_url = song.get("imageUrl", "")
        if image_url:
            assets.append(CoverArtAsset(
                url=image_url,
                asset_type=CoverArtType.STATIC,
                format="jpeg",
                description="iHeartRadio album artwork",
            ))

        return assets

    # ========================================================================
    # HTTP Client
    # ========================================================================

    async def _get_http_client(self):
        """Get or create the httpx async client.

        Returns:
            httpx.AsyncClient instance with timeout and redirect settings.
        """
        if self._http_client is None:
            import httpx
            self._http_client = httpx.AsyncClient(
                timeout=30.0,                              # 30-second timeout
                follow_redirects=True,                     # Follow HTTP redirects
            )
        return self._http_client
