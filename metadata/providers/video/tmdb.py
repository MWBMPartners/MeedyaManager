# ============================================================================
# File: /metadata/providers/video/tmdb.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# TMDB (The Movie Database) metadata provider for movies and TV shows.
# Uses the TMDB API v3 for searching movies, TV series, and episodes.
# Requires a free API key from https://www.themoviedb.org/settings/api
#
# Returns metadata including title, overview, release dates, ratings,
# poster images, genre information, and external IDs (IMDb).
#
# API: https://api.themoviedb.org/3/
# Rate limit: 40 requests per 10 seconds (4 req/sec sustained)
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

logger = logging.getLogger("MeedyaManager.Provider.TMDB")

# ============================================================================
# TMDB API Constants
# ============================================================================
API_BASE_URL = "https://api.themoviedb.org/3"             # TMDB API v3 base URL
IMAGE_BASE_URL = "https://image.tmdb.org/t/p/original"    # High-res poster images
TMDB_MOVIE_URL = "https://www.themoviedb.org/movie"       # Movie detail page URL
TMDB_TV_URL = "https://www.themoviedb.org/tv"             # TV show detail page URL


@register_provider
class TMDBProvider(BaseProvider):
    """TMDB (The Movie Database) metadata provider for movies and TV shows.

    Requires a free API key from themoviedb.org. Searches for movies and
    TV series, returning metadata including title, overview, release dates,
    ratings, poster images, genres, and external IDs (IMDb).
    """

    provider_name = "tmdb"                                 # Unique provider identifier

    def __init__(self):
        """Initialise the TMDB provider."""
        super().__init__()
        self._credential_manager = CredentialManager()
        self._rate_limiter = get_rate_limiter("tmdb")
        self._http_client = None                           # Lazy httpx client
        self._language = "en-GB"                           # Default language code

        # Load language from config
        try:
            from utils.config_loader import load_config
            config = load_config() or {}
            providers = config.get("providers", {})
            tmdb_config = providers.get("tmdb", {})
            if tmdb_config.get("language"):
                self._language = tmdb_config["language"]
        except Exception:
            pass

    @property
    def category(self) -> ProviderCategory:
        """TMDB is a video provider."""
        return ProviderCategory.VIDEO

    @property
    def capabilities(self) -> ProviderCapabilities:
        """TMDB supports movie, show, and episode search with static cover art."""
        return ProviderCapabilities(
            can_search_movies=True,                        # Search movies
            can_search_shows=True,                         # Search TV series
            can_search_episodes=True,                      # Search TV episodes
            can_lookup_imdb_id=True,                       # Can lookup by IMDb ID
            has_static_cover_art=True,                     # Poster/cover art (JPEG)
            has_cast_crew=True,                            # Cast and crew information available
        )

    @property
    def requires_auth(self) -> bool:
        """TMDB requires an API key for authentication."""
        return True

    def is_available(self) -> bool:
        """Check if TMDB provider is available with valid credentials."""
        try:
            import httpx                                   # Verify httpx is installed
        except ImportError:
            logger.warning("httpx not installed — TMDB provider unavailable")
            return False

        # Check for API key
        api_key = self._credential_manager.get_credential("tmdb", "api_key")
        if not api_key:
            logger.debug("TMDB API key not configured")
            return False

        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search TMDB for matching movies or TV shows.

        Determines the media type from query metadata:
        - If 'show', 'season', or media_class=='TV Show' → search TV
        - If media_class=='Movie' → search movies
        - Otherwise → search both movies and TV shows

        Args:
            query: dict with keys: title, show, season, episode, media_class.

        Returns:
            list[ProviderResult]: Matching movie/TV results.
        """
        search_parts = []
        if query.get("title"):
            search_parts.append(query["title"])
        if query.get("show"):
            search_parts.append(query["show"])

        if not search_parts:
            return []

        search_term = " ".join(search_parts)

        # Determine media type to search
        media_class = query.get("media_class", "").lower()
        has_tv_context = query.get("show") or query.get("season")

        results = []

        # Search TV shows if TV context is present
        if has_tv_context or media_class in ("tv show", "tv episode"):
            tv_results = await self._search_tv(search_term)
            results.extend(tv_results)

        # Search movies if movie context or no specific context
        if media_class == "movie" or not has_tv_context:
            movie_results = await self._search_movies(search_term)
            results.extend(movie_results)

        return results

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific item by TMDB ID.

        Args:
            provider_id: TMDB content ID (numeric string).

        Returns:
            ProviderResult if found, None otherwise.
        """
        # TMDB IDs could be movies or TV shows - try both
        # This is a simplified implementation - a real one would need type hints
        try:
            # Try as movie first
            result = await self._get_movie_details(provider_id)
            if result:
                return result

            # Try as TV show
            result = await self._get_tv_details(provider_id)
            return result

        except Exception as e:
            logger.error(f"TMDB lookup failed for {provider_id}: {e}")
            return None

    async def _search_movies(self, term: str) -> list[ProviderResult]:
        """Search TMDB for movies.

        Args:
            term: Search query string.

        Returns:
            list[ProviderResult]: Matching movie results.
        """
        try:
            api_key = self._credential_manager.get_credential("tmdb", "api_key")
            if not api_key:
                return []

            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            params = {
                "api_key": api_key,
                "query": term,
                "language": self._language,
            }

            url = f"{API_BASE_URL}/search/movie"
            response = await client.get(url, params=params)
            response.raise_for_status()
            data = response.json()

            results = []
            for item in data.get("results", []):
                result = self._parse_movie(item)
                if result:
                    results.append(result)
            return results

        except Exception as e:
            logger.error(f"TMDB movie search failed: {e}")
            return []

    async def _search_tv(self, term: str) -> list[ProviderResult]:
        """Search TMDB for TV shows.

        Args:
            term: Search query string.

        Returns:
            list[ProviderResult]: Matching TV show results.
        """
        try:
            api_key = self._credential_manager.get_credential("tmdb", "api_key")
            if not api_key:
                return []

            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            params = {
                "api_key": api_key,
                "query": term,
                "language": self._language,
            }

            url = f"{API_BASE_URL}/search/tv"
            response = await client.get(url, params=params)
            response.raise_for_status()
            data = response.json()

            results = []
            for item in data.get("results", []):
                result = self._parse_tv(item)
                if result:
                    results.append(result)
            return results

        except Exception as e:
            logger.error(f"TMDB TV search failed: {e}")
            return []

    async def _get_movie_details(self, movie_id: str) -> ProviderResult | None:
        """Get detailed movie information by TMDB ID.

        Args:
            movie_id: TMDB movie ID.

        Returns:
            ProviderResult if found, None otherwise.
        """
        try:
            api_key = self._credential_manager.get_credential("tmdb", "api_key")
            if not api_key:
                return None

            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            params = {
                "api_key": api_key,
                "language": self._language,
                "append_to_response": "external_ids",      # Get IMDb ID
            }

            url = f"{API_BASE_URL}/movie/{movie_id}"
            response = await client.get(url, params=params)
            response.raise_for_status()
            data = response.json()

            return self._parse_movie(data)

        except Exception as e:
            logger.error(f"TMDB movie details failed for {movie_id}: {e}")
            return None

    async def _get_tv_details(self, tv_id: str) -> ProviderResult | None:
        """Get detailed TV show information by TMDB ID.

        Args:
            tv_id: TMDB TV show ID.

        Returns:
            ProviderResult if found, None otherwise.
        """
        try:
            api_key = self._credential_manager.get_credential("tmdb", "api_key")
            if not api_key:
                return None

            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            params = {
                "api_key": api_key,
                "language": self._language,
                "append_to_response": "external_ids",      # Get IMDb ID
            }

            url = f"{API_BASE_URL}/tv/{tv_id}"
            response = await client.get(url, params=params)
            response.raise_for_status()
            data = response.json()

            return self._parse_tv(data)

        except Exception as e:
            logger.error(f"TMDB TV details failed for {tv_id}: {e}")
            return None

    def _parse_movie(self, item: dict) -> ProviderResult | None:
        """Parse a movie item into a ProviderResult.

        Args:
            item: A single movie result object from TMDB API.

        Returns:
            ProviderResult with metadata and cover art.
        """
        try:
            movie_id = str(item.get("id", ""))
            title = item.get("title", "")
            overview = item.get("overview", "")
            release_date = item.get("release_date", "")
            year = release_date[:4] if release_date else ""
            rating = item.get("vote_average", 0.0)
            poster_path = item.get("poster_path", "")

            # Cover art — high resolution poster
            cover_art = []
            if poster_path:
                poster_url = f"{IMAGE_BASE_URL}{poster_path}"
                cover_art.append(CoverArtAsset(
                    url=poster_url,
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                    width=2000,                            # TMDB original size varies
                    height=3000,
                    description="TMDB movie poster",
                ))

            # Get IMDb ID if available from external_ids
            external_ids = item.get("external_ids", {})
            imdb_id = external_ids.get("imdb_id", "")

            # Extra tags
            extra_tags = {
                "custom_tmdb_id": movie_id,
                "custom_tmdb_url": f"{TMDB_MOVIE_URL}/{movie_id}",
            }
            if overview:
                extra_tags["custom_tmdb_overview"] = overview[:500]
            if rating:
                extra_tags["custom_tmdb_rating"] = str(rating)
            if imdb_id:
                extra_tags["custom_tmdb_imdb_id"] = imdb_id

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                year=year,
                provider_id=movie_id,
                provider_url=f"{TMDB_MOVIE_URL}/{movie_id}",
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse TMDB movie item: {e}")
            return None

    def _parse_tv(self, item: dict) -> ProviderResult | None:
        """Parse a TV show item into a ProviderResult.

        Args:
            item: A single TV show result object from TMDB API.

        Returns:
            ProviderResult with metadata and cover art.
        """
        try:
            tv_id = str(item.get("id", ""))
            name = item.get("name", "")
            overview = item.get("overview", "")
            first_air_date = item.get("first_air_date", "")
            year = first_air_date[:4] if first_air_date else ""
            rating = item.get("vote_average", 0.0)
            poster_path = item.get("poster_path", "")

            # Cover art — high resolution poster
            cover_art = []
            if poster_path:
                poster_url = f"{IMAGE_BASE_URL}{poster_path}"
                cover_art.append(CoverArtAsset(
                    url=poster_url,
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                    width=2000,
                    height=3000,
                    description="TMDB TV poster",
                ))

            # Get IMDb ID if available from external_ids
            external_ids = item.get("external_ids", {})
            imdb_id = external_ids.get("imdb_id", "")

            # Extra tags
            extra_tags = {
                "custom_tmdb_id": tv_id,
                "custom_tmdb_url": f"{TMDB_TV_URL}/{tv_id}",
            }
            if overview:
                extra_tags["custom_tmdb_overview"] = overview[:500]
            if rating:
                extra_tags["custom_tmdb_rating"] = str(rating)
            if imdb_id:
                extra_tags["custom_tmdb_imdb_id"] = imdb_id

            return ProviderResult(
                provider_name=self.provider_name,
                title=name,
                show=name,                                 # TV shows use 'name' field
                year=year,
                provider_id=tv_id,
                provider_url=f"{TMDB_TV_URL}/{tv_id}",
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse TMDB TV item: {e}")
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
