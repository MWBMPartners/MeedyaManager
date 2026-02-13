# ============================================================================
# File: /metadata/providers/video/itunes_store.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# iTunes Store metadata provider using the iTunes Search API.
# Searches for music tracks/albums available on the iTunes Store.
# No authentication required (public API).
#
# Returns metadata including title, artist, album, genre, release date,
# track/disc numbers, and cover art (JPEG, scalable to 3000x3000).
#
# API: https://itunes.apple.com/search?media=music
# API: https://itunes.apple.com/lookup?id={id}
# Rate limit: ~20 requests per minute (Apple's general limit)
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
from metadata.providers.rate_limiter import get_rate_limiter

logger = logging.getLogger("MeedyaManager.Provider.iTunesStore")

# ============================================================================
# iTunes Search/Lookup API Constants
# ============================================================================
SEARCH_URL = "https://itunes.apple.com/search"             # iTunes Search API endpoint
LOOKUP_URL = "https://itunes.apple.com/lookup"             # iTunes Lookup API endpoint


@register_provider
class iTunesStoreProvider(BaseProvider):
    """iTunes Store metadata provider via iTunes Search/Lookup API.

    No authentication required. Returns music metadata including
    title, artist, album, genre, track/disc numbers, and cover art.
    Cover art can be scaled up to 3000x3000 by modifying the URL template.
    """

    provider_name = "itunes_store"                         # Unique provider identifier

    def __init__(self):
        """Initialise the iTunes Store provider."""
        super().__init__()
        self._rate_limiter = get_rate_limiter("itunes_store")
        self._http_client = None                           # Lazy httpx client
        self._storefront = "gb"                            # Default country code

        # Load storefront from config
        try:
            from utils.config_loader import load_config
            config = load_config() or {}
            providers = config.get("providers", {})
            its_config = providers.get("itunes_store", {})
            if its_config.get("storefront"):
                self._storefront = its_config["storefront"]
        except Exception:
            pass

    @property
    def category(self) -> ProviderCategory:
        """iTunes Store is a video/media provider (covers music and media purchases)."""
        return ProviderCategory.VIDEO

    @property
    def capabilities(self) -> ProviderCapabilities:
        """iTunes Store supports track/album search with static cover art."""
        return ProviderCapabilities(
            can_search_tracks=True,                        # Search individual songs
            can_search_albums=True,                        # Search albums
            has_static_cover_art=True,                     # JPEG cover art (scalable)
        )

    @property
    def requires_auth(self) -> bool:
        """iTunes Search API is public — no authentication required."""
        return False

    def is_available(self) -> bool:
        """iTunes Store is always available (public API)."""
        try:
            import httpx
            return True
        except ImportError:
            logger.warning("httpx not installed — iTunes Store provider unavailable")
            return False

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search the iTunes Store for matching tracks.

        Args:
            query: dict with keys: title, artist, album.

        Returns:
            list[ProviderResult]: Matching results with cover art.
        """
        search_parts = []
        if query.get("title"):
            search_parts.append(query["title"])
        if query.get("artist"):
            search_parts.append(query["artist"])
        if query.get("album"):
            search_parts.append(query["album"])

        if not search_parts:
            return []

        search_term = " ".join(search_parts)

        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            params = {
                "term": search_term,
                "media": "music",
                "entity": "song",
                "country": self._storefront,
                "limit": 10,
            }

            response = await client.get(SEARCH_URL, params=params)
            response.raise_for_status()
            data = response.json()

            return self._parse_results(data)

        except Exception as e:
            logger.error(f"iTunes Store search failed: {e}")
            return []

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific item by iTunes Store ID.

        Args:
            provider_id: iTunes Store track ID (numeric string).

        Returns:
            ProviderResult if found, None otherwise.
        """
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            params = {"id": provider_id}
            response = await client.get(LOOKUP_URL, params=params)
            response.raise_for_status()
            data = response.json()

            results_list = data.get("results", [])
            if results_list:
                return self._parse_item(results_list[0])
            return None

        except Exception as e:
            logger.error(f"iTunes Store lookup failed for {provider_id}: {e}")
            return None

    def _parse_results(self, data: dict) -> list[ProviderResult]:
        """Parse iTunes Search API response into ProviderResult list.

        Args:
            data: Raw JSON response from iTunes Search API.

        Returns:
            list[ProviderResult]: Parsed results.
        """
        results = []
        for item in data.get("results", []):
            result = self._parse_item(item)
            if result:
                results.append(result)
        return results

    def _parse_item(self, item: dict) -> ProviderResult | None:
        """Parse a single iTunes Store item into a ProviderResult.

        Args:
            item: A single result object from the iTunes Search/Lookup API.

        Returns:
            ProviderResult with metadata and cover art.
        """
        try:
            title = item.get("trackName", "")
            artist = item.get("artistName", "")
            album = item.get("collectionName", "")
            genre = item.get("primaryGenreName", "")
            track_id = str(item.get("trackId", ""))
            track_url = item.get("trackViewUrl", "")
            release_date = item.get("releaseDate", "")
            year = release_date[:4] if release_date else ""
            track_num = str(item.get("trackNumber", ""))
            disc_num = str(item.get("discNumber", ""))
            total_tracks = str(item.get("trackCount", ""))

            # Scale artwork URL to maximum resolution (3000x3000)
            cover_art = []
            artwork_url = item.get("artworkUrl100", "")
            if artwork_url:
                # Replace 100x100bb with 3000x3000bb for high-res
                hires_url = artwork_url.replace("100x100bb", "3000x3000bb")
                cover_art.append(CoverArtAsset(
                    url=hires_url,
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                    width=3000,
                    height=3000,
                    description="iTunes Store album artwork",
                ))

            # Extra tags
            extra_tags = {}
            collection_id = item.get("collectionId")
            if collection_id:
                extra_tags["custom_itunes_collection_id"] = str(collection_id)

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                artist=artist,
                album=album,
                genre=genre,
                year=year,
                track_num=track_num,
                disc_num=disc_num,
                total_tracks=total_tracks,
                provider_id=track_id,
                provider_url=track_url,
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse iTunes Store item: {e}")
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
