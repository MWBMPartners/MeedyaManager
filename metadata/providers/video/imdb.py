# ============================================================================
# File: /metadata/providers/video/imdb.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# IMDb metadata provider for movies and TV shows using the cinemagoer library.
# No authentication required. Uses web scraping via the cinemagoer package
# (formerly IMDbPY) to search and retrieve movie/show metadata.
#
# Returns metadata including title, year, rating, votes, genres, and
# poster images. IMDb IDs are formatted with the 'tt' prefix.
#
# Library: cinemagoer (GPL-2.0 license)
# MeedyaManager: GPL-2.0-or-later (compatible)
# Rate limit: 5 requests per 10 seconds (conservative for scraping)
# ============================================================================

import logging                                             # Standard logging
import asyncio                                             # For running sync code in threads

from metadata.providers import ProviderCategory, register_provider
from metadata.providers.base import (
    BaseProvider,                                           # Provider ABC
    ProviderCapabilities,                                   # Capabilities declaration
    ProviderResult,                                        # Result dataclass
    CoverArtAsset,                                         # Cover art asset
    CoverArtType,                                          # Cover art type enum
)
from metadata.providers.rate_limiter import get_rate_limiter

logger = logging.getLogger("MeedyaManager.Provider.IMDb")

# ============================================================================
# IMDb Constants
# ============================================================================
IMDB_TITLE_URL = "https://www.imdb.com/title"             # IMDb title page URL


@register_provider
class IMDbProvider(BaseProvider):
    """IMDb metadata provider using the cinemagoer library.

    No authentication required. Uses web scraping to search movies and TV
    shows. Returns metadata including title, year, rating, votes, genres,
    and poster images.

    IMPORTANT: Requires the cinemagoer package (GPL-2.0 license).
    MeedyaManager is GPL-2.0-or-later, so this is compatible.
    Always uses lazy import to make cinemagoer an optional dependency.
    """

    provider_name = "imdb"                                 # Unique provider identifier

    def __init__(self):
        """Initialise the IMDb provider."""
        super().__init__()
        self._rate_limiter = get_rate_limiter("imdb")
        self._ia = None                                    # Lazy cinemagoer instance

    @property
    def category(self) -> ProviderCategory:
        """IMDb is a video provider."""
        return ProviderCategory.VIDEO

    @property
    def capabilities(self) -> ProviderCapabilities:
        """IMDb supports movie and show search with static cover art."""
        return ProviderCapabilities(
            can_search_movies=True,                        # Search movies
            can_search_shows=True,                         # Search TV series
            can_lookup_imdb_id=True,                       # Can lookup by IMDb ID (ttXXXXXXX)
            has_static_cover_art=True,                     # Poster/cover art (JPEG)
        )

    @property
    def requires_auth(self) -> bool:
        """IMDb provider does not require authentication (scraping)."""
        return False

    def is_available(self) -> bool:
        """Check if IMDb provider is available (cinemagoer installed)."""
        try:
            import imdb                                    # Lazy import — cinemagoer package
            return True
        except ImportError:
            logger.debug("cinemagoer not installed — IMDb provider unavailable")
            return False

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search IMDb for matching movies or TV shows.

        Uses cinemagoer's search_movie() method which searches both
        movies and TV series. Results are returned in relevance order.

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

        return await self._search_imdb(search_term)

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific title by IMDb ID.

        Args:
            provider_id: IMDb ID (with or without 'tt' prefix).

        Returns:
            ProviderResult if found, None otherwise.
        """
        # Remove 'tt' prefix if present
        movie_id = provider_id.lstrip("tt")

        try:
            ia = self._get_cinemagoer()
            if not ia:
                return None

            await self._rate_limiter.acquire()

            # Run sync operation in thread pool
            movie = await asyncio.to_thread(ia.get_movie, movie_id)

            if movie:
                return self._parse_movie(movie)
            return None

        except Exception as e:
            logger.error(f"IMDb lookup failed for {provider_id}: {e}")
            return None

    async def _search_imdb(self, term: str) -> list[ProviderResult]:
        """Search IMDb using cinemagoer.

        Args:
            term: Search query string.

        Returns:
            list[ProviderResult]: Matching results.
        """
        try:
            ia = self._get_cinemagoer()
            if not ia:
                return []

            await self._rate_limiter.acquire()

            # Run sync search in thread pool (cinemagoer is synchronous)
            search_results = await asyncio.to_thread(ia.search_movie, term)

            results = []
            # Limit to first 10 results to avoid excessive scraping
            for movie in search_results[:10]:
                # Get full movie details (cinemagoer lazy-loads data)
                try:
                    await self._rate_limiter.acquire()
                    await asyncio.to_thread(ia.update, movie)
                    result = self._parse_movie(movie)
                    if result:
                        results.append(result)
                except Exception as e:
                    logger.debug(f"Failed to fetch IMDb details for {movie.movieID}: {e}")
                    continue

            return results

        except Exception as e:
            logger.error(f"IMDb search failed: {e}")
            return []

    def _get_cinemagoer(self):
        """Get or create the cinemagoer instance.

        Returns:
            Cinemagoer instance, or None if not available.
        """
        if self._ia is None:
            try:
                from imdb import Cinemagoer              # Lazy import
                self._ia = Cinemagoer()
            except ImportError:
                logger.warning("cinemagoer package not installed")
                return None
        return self._ia

    def _parse_movie(self, movie) -> ProviderResult | None:
        """Parse a cinemagoer Movie object into a ProviderResult.

        Args:
            movie: A cinemagoer Movie object.

        Returns:
            ProviderResult with metadata and cover art.
        """
        try:
            movie_id = movie.movieID                       # IMDb ID (numeric)
            imdb_id = f"tt{movie_id}"                      # Format with 'tt' prefix
            title = movie.get("title", "") or movie.get("smart canonical title", "")
            year = str(movie.get("year", ""))
            rating = movie.get("rating", 0.0)
            votes = movie.get("votes", 0)
            genres = movie.get("genres", [])
            genre_str = ", ".join(genres) if genres else ""

            # Cover art — IMDb poster images
            cover_art = []
            # Try to get full-size cover URL first, fall back to thumbnail
            cover_url = movie.get("full-size cover url", "") or movie.get("cover url", "")
            if cover_url:
                cover_art.append(CoverArtAsset(
                    url=cover_url,
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                    width=0,                               # IMDb doesn't specify dimensions
                    height=0,
                    description="IMDb poster",
                ))

            # Determine if it's a TV show
            kind = movie.get("kind", "")
            is_tv = kind in ("tv series", "tv mini series")

            # Extra tags
            extra_tags = {
                "custom_imdb_id": imdb_id,
                "custom_imdb_url": f"{IMDB_TITLE_URL}/{imdb_id}/",
            }
            if rating:
                extra_tags["custom_imdb_rating"] = str(rating)
            if votes:
                extra_tags["custom_imdb_votes"] = str(votes)
            if genres:
                extra_tags["custom_imdb_genres"] = genre_str

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                show=title if is_tv else "",               # Use title as show name for TV
                year=year,
                genre=genre_str,
                provider_id=imdb_id,
                provider_url=f"{IMDB_TITLE_URL}/{imdb_id}/",
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse IMDb movie: {e}")
            return None
