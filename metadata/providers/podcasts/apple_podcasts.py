# ============================================================================
# File: /metadata/providers/podcasts/apple_podcasts.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Apple Podcasts metadata provider using the iTunes Search API.
# Searches for podcast shows and episodes. No authentication required
# (public API). Returns metadata including show name, episode title,
# artist/author, genre, artwork, and feed URL.
#
# API: https://itunes.apple.com/search?media=podcast
# Rate limit: ~20 requests per minute (Apple's general Search API limit)
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

logger = logging.getLogger("MeedyaManager.Provider.ApplePodcasts")

# ============================================================================
# iTunes Search API Constants
# ============================================================================
SEARCH_URL = "https://itunes.apple.com/search"             # iTunes Search API endpoint


@register_provider
class ApplePodcastsProvider(BaseProvider):
    """Apple Podcasts metadata provider via iTunes Search API.

    No authentication required. Returns podcast show/episode metadata
    including artwork, genre, artist/author, and feed URLs.
    """

    provider_name = "apple_podcasts"                       # Unique provider identifier

    def __init__(self):
        """Initialise the Apple Podcasts provider."""
        super().__init__()
        self._rate_limiter = get_rate_limiter("apple_podcasts")
        self._http_client = None                           # Lazy httpx client

    @property
    def category(self) -> ProviderCategory:
        """Apple Podcasts is a podcast provider."""
        return ProviderCategory.PODCAST

    @property
    def capabilities(self) -> ProviderCapabilities:
        """Apple Podcasts supports show and episode search with static cover art."""
        return ProviderCapabilities(
            can_search_podcasts=True,                      # Search podcast shows/episodes
            has_static_cover_art=True,                     # Podcast artwork (JPEG)
        )

    @property
    def requires_auth(self) -> bool:
        """iTunes Search API is public — no authentication required."""
        return False

    def is_available(self) -> bool:
        """Apple Podcasts is always available (public API, no auth)."""
        try:
            import httpx                                   # Check httpx is installed
            return True
        except ImportError:
            logger.warning("httpx not installed — Apple Podcasts provider unavailable")
            return False

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search Apple Podcasts for matching shows or episodes.

        Args:
            query: dict with keys: title, artist (author), show.

        Returns:
            list[ProviderResult]: Matching podcast results.
        """
        # Build search term
        search_parts = []
        if query.get("title"):
            search_parts.append(query["title"])
        if query.get("artist"):
            search_parts.append(query["artist"])
        if query.get("show"):
            search_parts.append(query["show"])

        if not search_parts:
            return []

        search_term = " ".join(search_parts)

        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            params = {
                "term": search_term,
                "media": "podcast",
                "entity": "podcastEpisode",                # Search episodes
                "limit": 10,
            }

            response = await client.get(SEARCH_URL, params=params)
            response.raise_for_status()
            data = response.json()

            return self._parse_results(data)

        except Exception as e:
            logger.error(f"Apple Podcasts search failed: {e}")
            return []

    def _parse_results(self, data: dict) -> list[ProviderResult]:
        """Parse iTunes Search API response for podcast results.

        Args:
            data: Raw JSON response from iTunes Search API.

        Returns:
            list[ProviderResult]: Parsed podcast results.
        """
        results = []
        for item in data.get("results", []):
            try:
                # Episode metadata
                title = item.get("trackName", "")
                show = item.get("collectionName", "")
                artist = item.get("artistName", "")
                genre = item.get("primaryGenreName", "")
                episode_id = str(item.get("trackId", ""))
                episode_url = item.get("trackViewUrl", "")
                release_date = item.get("releaseDate", "")
                year = release_date[:4] if release_date else ""

                # Cover art — scale artwork URL to 600x600
                cover_art = []
                artwork_url = item.get("artworkUrl600", "")
                if not artwork_url:
                    artwork_url = item.get("artworkUrl100", "")
                    if artwork_url:
                        artwork_url = artwork_url.replace("100x100", "600x600")
                if artwork_url:
                    cover_art.append(CoverArtAsset(
                        url=artwork_url,
                        asset_type=CoverArtType.STATIC,
                        format="jpeg",
                        width=600,
                        height=600,
                        description="Podcast artwork",
                    ))

                # Extra tags
                extra_tags = {}
                feed_url = item.get("feedUrl", "")
                if feed_url:
                    extra_tags["custom_apple_podcast_feed_url"] = feed_url
                duration_ms = item.get("trackTimeMillis", 0)
                if duration_ms:
                    extra_tags["custom_apple_podcast_duration_ms"] = str(duration_ms)

                results.append(ProviderResult(
                    provider_name=self.provider_name,
                    title=title,
                    artist=artist,
                    show=show,
                    genre=genre,
                    year=year,
                    provider_id=episode_id,
                    provider_url=episode_url,
                    cover_art=cover_art,
                    extra_tags=extra_tags,
                ))

            except Exception as e:
                logger.error(f"Failed to parse podcast result: {e}")
                continue

        return results

    async def _get_http_client(self):
        """Get or create the httpx async client."""
        if self._http_client is None:
            import httpx
            self._http_client = httpx.AsyncClient(
                timeout=30.0,
                follow_redirects=True,
            )
        return self._http_client
