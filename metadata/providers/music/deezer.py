# ============================================================================
# File: /metadata/providers/music/deezer.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Deezer metadata provider using the Deezer Public API.
# Supports track/album search with metadata including ISRC codes
# and static cover art up to 1000x1000 pixels.
#
# Authentication:
# No authentication required for public search endpoints.
# The Deezer API provides free access to catalog search and metadata.
#
# API Documentation:
# https://developers.deezer.com/api
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

logger = logging.getLogger("MeedyaManager.Provider.Deezer")

# ============================================================================
# Deezer API Constants
# ============================================================================
API_BASE = "https://api.deezer.com"                        # Deezer API base URL
SEARCH_ENDPOINT = "/search"                                # Search endpoint (tracks)


@register_provider
class DeezerProvider(BaseProvider):
    """Deezer metadata provider using Deezer Public API.

    Provides music metadata including:
    - Track/album search with title, artist, album, ISRC
    - Static cover art up to 1000x1000 JPEG
    - Duration and track position data
    - Public API (no authentication required)

    No authentication required for search endpoints.
    """

    provider_name = "deezer"                               # Unique provider identifier

    def __init__(self):
        """Initialise the Deezer provider with HTTP client."""
        super().__init__()
        self._credentials = CredentialManager()            # Credential resolution (unused)
        self._rate_limiter = get_rate_limiter("deezer")    # Rate limiter
        self._http_client = None                           # Lazy httpx client

    @property
    def category(self) -> ProviderCategory:
        """Deezer is a music provider."""
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        """Deezer supports track/album search with static cover art."""
        return ProviderCapabilities(
            can_search_tracks=True,                        # Search individual songs
            can_search_albums=True,                        # Search albums
            can_lookup_isrc=True,                          # ISRC available in responses
            has_static_cover_art=True,                     # JPEG cover art up to 1000x1000
        )

    @property
    def requires_auth(self) -> bool:
        """Deezer public API does not require authentication."""
        return False

    def is_available(self) -> bool:
        """Deezer is always available (no credentials required).

        Returns:
            True (Deezer public API is always available).
        """
        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search Deezer catalog for matching tracks.

        Constructs a search term from query metadata and calls the
        Deezer Search API. Results include metadata, ISRC codes,
        and cover art URLs.

        Args:
            query: dict with keys: title, artist, album, isrc.

        Returns:
            list[ProviderResult]: Matching results with cover art assets.
        """
        # Build search term from available metadata
        search_parts = []
        if query.get("title"):
            search_parts.append(f'track:"{query["title"]}"')
        if query.get("artist"):
            search_parts.append(f'artist:"{query["artist"]}"')
        if query.get("album"):
            search_parts.append(f'album:"{query["album"]}"')

        if not search_parts:
            logger.warning("Deezer search: no query terms provided")
            return []

        search_term = " ".join(search_parts)

        # Make the API request
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = API_BASE + SEARCH_ENDPOINT
            params = {
                "q": search_term,
                "limit": 10,                               # Max 10 results
            }

            response = await client.get(url, params=params)
            response.raise_for_status()
            data = response.json()

            return self._parse_search_results(data)

        except Exception as e:
            logger.error(f"Deezer search failed: {e}")
            return []

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific track by its Deezer ID.

        Args:
            provider_id: Deezer track ID (numeric string).

        Returns:
            ProviderResult if found, None otherwise.
        """
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = f"{API_BASE}/track/{provider_id}"

            response = await client.get(url)
            response.raise_for_status()
            data = response.json()

            return self._parse_track(data)

        except Exception as e:
            logger.error(f"Deezer lookup failed for {provider_id}: {e}")
            return None

    # ========================================================================
    # Response Parsing
    # ========================================================================

    def _parse_search_results(self, data: dict) -> list[ProviderResult]:
        """Parse Deezer search API response into ProviderResult list.

        Args:
            data: Raw JSON response from the search endpoint.

        Returns:
            list[ProviderResult]: Parsed results with cover art assets.
        """
        results = []
        tracks = data.get("data", [])

        for track in tracks:
            result = self._parse_track(track)
            if result:
                results.append(result)

        return results

    def _parse_track(self, track: dict) -> ProviderResult | None:
        """Parse a single Deezer track object into a ProviderResult.

        Extracts standard metadata, ISRC, and cover art.

        Args:
            track: A single track object from the Deezer API.

        Returns:
            ProviderResult with metadata and cover art assets.
        """
        try:
            track_id = str(track.get("id", ""))

            # Extract standard metadata
            title = track.get("title", "")
            artist_data = track.get("artist", {})
            artist = artist_data.get("name", "")
            album_data = track.get("album", {})
            album = album_data.get("title", "")

            # Extract ISRC
            isrc = track.get("isrc", "")

            # Track/disc numbers
            track_num = str(track.get("track_position", ""))
            disc_num = str(track.get("disk_number", ""))

            # Duration (in seconds)
            duration = track.get("duration", 0)

            # Release date
            release_date = track.get("release_date", "")
            if not release_date:
                # Try album release date
                release_date = album_data.get("release_date", "")
            year = release_date[:4] if release_date else ""

            # URL
            url = track.get("link", "")

            # Build cover art assets
            cover_art = self._extract_cover_art(album_data)

            # Build extra tags
            extra_tags = {}
            if track_id:
                extra_tags["custom_deezer_id"] = track_id
            if url:
                extra_tags["custom_deezer_url"] = url
            if isrc:
                extra_tags["custom_deezer_isrc"] = isrc
            if duration:
                extra_tags["custom_deezer_duration"] = str(duration)

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                artist=artist,
                album=album,
                year=year,
                isrc=isrc,
                track_num=track_num,
                disc_num=disc_num,
                provider_id=track_id,
                provider_url=url,
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse Deezer track: {e}")
            return None

    def _extract_cover_art(self, album_data: dict) -> list[CoverArtAsset]:
        """Extract cover art assets from album data.

        Deezer provides cover art in multiple resolutions:
        - cover_small: 56x56
        - cover_medium: 250x250
        - cover_big: 500x500
        - cover_xl: 1000x1000

        We use cover_xl for best quality.

        Args:
            album_data: Album object from Deezer API.

        Returns:
            list[CoverArtAsset]: Static cover art assets.
        """
        assets = []
        cover_xl_url = album_data.get("cover_xl", "")

        if cover_xl_url:
            assets.append(CoverArtAsset(
                url=cover_xl_url,
                asset_type=CoverArtType.STATIC,
                format="jpeg",
                width=1000,
                height=1000,
                description="Deezer album cover (XL)",
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
