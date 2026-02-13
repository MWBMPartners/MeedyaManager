# ============================================================================
# File: /metadata/providers/video/apple_tv.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Apple TV metadata provider using the iTunes Search API.
# Searches for movies and TV shows available on Apple TV / iTunes Store.
# No authentication required (public API).
#
# Returns metadata including movie/show title, director, genre,
# release date, episode/season numbers (for TV), and cover art.
#
# API: https://itunes.apple.com/search?media=movie
# API: https://itunes.apple.com/search?media=tvShow
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

logger = logging.getLogger("MeedyaManager.Provider.AppleTV")

# ============================================================================
# iTunes Search API Constants
# ============================================================================
SEARCH_URL = "https://itunes.apple.com/search"             # iTunes Search API endpoint
LOOKUP_URL = "https://itunes.apple.com/lookup"             # iTunes Lookup API endpoint


@register_provider
class AppleTVProvider(BaseProvider):
    """Apple TV metadata provider via iTunes Search API.

    No authentication required. Searches for movies and TV shows.
    Returns metadata including title, director, genre, season/episode
    numbers, release dates, and cover art (JPEG, scalable to 3000x3000).
    """

    provider_name = "apple_tv"                             # Unique provider identifier

    def __init__(self):
        """Initialise the Apple TV provider."""
        super().__init__()
        self._rate_limiter = get_rate_limiter("apple_tv")
        self._http_client = None                           # Lazy httpx client
        self._storefront = "gb"                            # Default country code

        # Load storefront from config
        try:
            from utils.config_loader import load_config
            config = load_config() or {}
            providers = config.get("providers", {})
            atv_config = providers.get("apple_tv", {})
            if atv_config.get("storefront"):
                self._storefront = atv_config["storefront"]
        except Exception:
            pass

    @property
    def category(self) -> ProviderCategory:
        """Apple TV is a video provider."""
        return ProviderCategory.VIDEO

    @property
    def capabilities(self) -> ProviderCapabilities:
        """Apple TV supports movie, show, and episode search with static cover art."""
        return ProviderCapabilities(
            can_search_movies=True,                        # Search movies
            can_search_shows=True,                         # Search TV series
            can_search_episodes=True,                      # Search TV episodes
            has_static_cover_art=True,                     # Poster/cover art (JPEG)
        )

    @property
    def requires_auth(self) -> bool:
        """iTunes Search API is public — no authentication required."""
        return False

    def is_available(self) -> bool:
        """Apple TV is always available (public API)."""
        try:
            import httpx
            return True
        except ImportError:
            logger.warning("httpx not installed — Apple TV provider unavailable")
            return False

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search Apple TV for matching movies or TV shows.

        Determines the media type from query metadata:
        - If 'show' or 'season' is present → search TV episodes
        - If media_class is 'Movie' → search movies
        - Otherwise → search both movies and TV shows

        Args:
            query: dict with keys: title, artist/director, show, season,
                   episode, media_class.

        Returns:
            list[ProviderResult]: Matching movie/TV results.
        """
        search_parts = []
        if query.get("title"):
            search_parts.append(query["title"])
        if query.get("show"):
            search_parts.append(query["show"])
        if query.get("artist"):
            search_parts.append(query["artist"])

        if not search_parts:
            return []

        search_term = " ".join(search_parts)

        # Determine media type to search
        media_class = query.get("media_class", "").lower()
        has_tv_context = query.get("show") or query.get("season")

        results = []

        # Search TV shows if TV context is present
        if has_tv_context or media_class in ("tv show", "tv episode"):
            tv_results = await self._search_media(search_term, "tvShow", "tvEpisode")
            results.extend(tv_results)

        # Search movies if movie context or no specific context
        if media_class == "movie" or not has_tv_context:
            movie_results = await self._search_media(search_term, "movie", "movie")
            results.extend(movie_results)

        return results

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific item by Apple TV / iTunes Store ID.

        Args:
            provider_id: iTunes content ID (numeric string).

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

            items = data.get("results", [])
            if items:
                return self._parse_item(items[0])
            return None

        except Exception as e:
            logger.error(f"Apple TV lookup failed for {provider_id}: {e}")
            return None

    async def _search_media(self, term: str, media: str, entity: str) -> list[ProviderResult]:
        """Search iTunes for a specific media type.

        Args:
            term: Search query string.
            media: iTunes media type ("movie" or "tvShow").
            entity: iTunes entity type ("movie", "tvEpisode", "tvSeason").

        Returns:
            list[ProviderResult]: Matching results.
        """
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            params = {
                "term": term,
                "media": media,
                "entity": entity,
                "country": self._storefront,
                "limit": 10,
            }

            response = await client.get(SEARCH_URL, params=params)
            response.raise_for_status()
            data = response.json()

            results = []
            for item in data.get("results", []):
                result = self._parse_item(item)
                if result:
                    results.append(result)
            return results

        except Exception as e:
            logger.error(f"Apple TV search ({media}/{entity}) failed: {e}")
            return []

    def _parse_item(self, item: dict) -> ProviderResult | None:
        """Parse a single movie or TV episode item into a ProviderResult.

        Handles both movie and TV episode response formats:
        - Movies: trackName, artistName (director), primaryGenreName
        - TV episodes: trackName, artistName (show), collectionName (season)

        Args:
            item: A single result object from the iTunes Search API.

        Returns:
            ProviderResult with metadata and cover art.
        """
        try:
            kind = item.get("kind", "")
            wrapper_type = item.get("wrapperType", "")
            title = item.get("trackName", "") or item.get("collectionName", "")
            artist = item.get("artistName", "")
            content_id = str(item.get("trackId", "") or item.get("collectionId", ""))
            content_url = item.get("trackViewUrl", "") or item.get("collectionViewUrl", "")
            genre = item.get("primaryGenreName", "")
            release_date = item.get("releaseDate", "")
            year = release_date[:4] if release_date else ""

            # TV-specific fields
            show = ""
            season = ""
            episode = ""
            episode_title = ""
            director = ""

            if kind == "tv-episode" or "Episode" in wrapper_type:
                show = item.get("artistName", "")          # Show name is in artistName for episodes
                episode_title = title                       # Track name is the episode title
                season_name = item.get("collectionName", "")
                # Try to extract season number
                if "Season" in season_name:
                    try:
                        season = season_name.split("Season")[-1].strip().split(",")[0].strip()
                    except Exception:
                        pass
                artist = show                               # Use show name as artist
            elif kind == "feature-movie":
                director = item.get("artistName", "")      # Director name for movies

            # Cover art — scale to 3000x3000
            cover_art = []
            artwork_url = item.get("artworkUrl100", "")
            if artwork_url:
                hires_url = artwork_url.replace("100x100bb", "3000x3000bb")
                cover_art.append(CoverArtAsset(
                    url=hires_url,
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                    width=3000,
                    height=3000,
                    description="Apple TV artwork",
                ))

            # Extra tags
            extra_tags = {}
            long_description = item.get("longDescription", "")
            if long_description:
                extra_tags["custom_apple_tv_description"] = long_description[:500]
            content_rating = item.get("contentAdvisoryRating", "")
            if content_rating:
                extra_tags["custom_apple_tv_rating"] = content_rating

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                artist=artist,
                genre=genre,
                year=year,
                show=show,
                season=season,
                episode=episode,
                episode_title=episode_title,
                director=director,
                provider_id=content_id,
                provider_url=content_url,
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse Apple TV item: {e}")
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
