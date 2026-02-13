# ============================================================================
# File: /metadata/providers/music/youtube_music.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# YouTube Music metadata provider using the ytmusicapi library.
# Supports track/album search with browser cookie authentication.
#
# Authentication:
# Uses browser cookies via ytmusicapi's headers_auth.json file.
# This file contains the Cookie and X-Goog-AuthUser headers extracted
# from a logged-in YouTube Music browser session.
#
# Required credentials (via CredentialManager):
# - YOUTUBE_MUSIC_HEADERS_AUTH: Path to headers_auth.json file
#
# The headers_auth.json file can be generated using:
# ytmusicapi oauth
#
# Note: ytmusicapi is synchronous, so we wrap calls in asyncio.to_thread()
# for async compatibility with the BaseProvider interface.
# ============================================================================

import asyncio                                             # For wrapping sync ytmusicapi calls
import logging                                             # Standard logging
from pathlib import Path                                   # File path handling

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

logger = logging.getLogger("MeedyaManager.Provider.YouTubeMusic")


@register_provider
class YouTubeMusicProvider(BaseProvider):
    """YouTube Music metadata provider using ytmusicapi.

    Provides music metadata including:
    - Track/album search with title, artist, album, duration
    - Static cover art (JPEG thumbnails)
    - Video IDs for direct YouTube Music links

    Requires browser authentication via headers_auth.json file.
    """

    provider_name = "youtube_music"                        # Unique provider identifier

    def __init__(self):
        """Initialise the YouTube Music provider with credentials."""
        super().__init__()
        self._credentials = CredentialManager()            # Credential resolution
        self._rate_limiter = get_rate_limiter("youtube_music")  # Rate limiter
        self._ytmusic = None                               # Lazy ytmusicapi client

    @property
    def category(self) -> ProviderCategory:
        """YouTube Music is a music provider."""
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        """YouTube Music supports track/album search with static cover art."""
        return ProviderCapabilities(
            can_search_tracks=True,                        # Search individual songs
            can_search_albums=True,                        # Search albums
            has_static_cover_art=True,                     # JPEG thumbnails
        )

    @property
    def requires_auth(self) -> bool:
        """YouTube Music requires headers_auth.json for authentication."""
        return True

    def is_available(self) -> bool:
        """Check if YouTube Music is available.

        Requires:
        1. ytmusicapi library to be installed
        2. headers_auth.json file to exist at the configured path

        Returns:
            bool: True if ytmusicapi is installed and headers file exists.
        """
        # Check if ytmusicapi is installed
        try:
            import ytmusicapi                              # Check ytmusicapi availability
        except ImportError:
            logger.warning("ytmusicapi not installed — YouTube Music provider unavailable")
            return False

        # Check if headers_auth.json file exists
        headers_path = self._get_headers_path()
        if not headers_path or not Path(headers_path).exists():
            logger.debug("YouTube Music headers_auth.json not found")
            return False

        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search YouTube Music catalog for matching tracks.

        Constructs a search query from metadata and uses ytmusicapi to
        search for songs. Results include title, artist, album, duration,
        video ID, and thumbnail URLs.

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
            logger.warning("YouTube Music search: no query terms provided")
            return []

        search_term = " ".join(search_parts)

        # Initialize ytmusicapi client if needed
        ytmusic = await self._get_ytmusic_client()
        if not ytmusic:
            logger.error("YouTube Music: failed to initialize client")
            return []

        # Perform search (wrapped in thread since ytmusicapi is synchronous)
        try:
            await self._rate_limiter.acquire()

            # Run synchronous ytmusicapi call in thread pool
            search_results = await asyncio.to_thread(
                ytmusic.search,
                query=search_term,
                filter="songs",                            # Search songs only
                limit=10,                                  # Max 10 results
            )

            return self._parse_search_results(search_results)

        except Exception as e:
            logger.error(f"YouTube Music search failed: {e}")
            return []

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific song by its YouTube Music video ID.

        Args:
            provider_id: YouTube Music video ID (videoId).

        Returns:
            ProviderResult if found, None otherwise.
        """
        ytmusic = await self._get_ytmusic_client()
        if not ytmusic:
            return None

        try:
            await self._rate_limiter.acquire()

            # Get song details by video ID
            song_data = await asyncio.to_thread(
                ytmusic.get_song,
                videoId=provider_id,
            )

            if song_data:
                return self._parse_song(song_data)
            return None

        except Exception as e:
            logger.error(f"YouTube Music lookup failed for {provider_id}: {e}")
            return None

    # ========================================================================
    # YTMusic Client Management
    # ========================================================================

    async def _get_ytmusic_client(self):
        """Get or create the ytmusicapi client.

        Returns:
            YTMusic instance authenticated with headers_auth.json.
        """
        if self._ytmusic is None:
            try:
                import ytmusicapi                          # Import ytmusicapi
                headers_path = self._get_headers_path()
                if not headers_path:
                    logger.error("YouTube Music: headers_auth.json path not configured")
                    return None

                # Create YTMusic client with authentication file
                self._ytmusic = ytmusicapi.YTMusic(auth=headers_path)
                logger.info(f"YouTube Music client initialized with {headers_path}")
            except ImportError:
                logger.error("ytmusicapi not installed")
                return None
            except Exception as e:
                logger.error(f"Failed to initialize YouTube Music client: {e}")
                return None

        return self._ytmusic

    def _get_headers_path(self) -> str | None:
        """Get the path to the headers_auth.json file.

        Returns:
            str: Path to headers_auth.json, or None if not configured.
        """
        return self._credentials.get_credential("youtube_music", "headers_auth_path")

    # ========================================================================
    # Response Parsing
    # ========================================================================

    def _parse_search_results(self, results: list) -> list[ProviderResult]:
        """Parse YouTube Music search API response into ProviderResult list.

        Args:
            results: List of song dictionaries from ytmusicapi.search().

        Returns:
            list[ProviderResult]: Parsed results with cover art assets.
        """
        parsed_results = []

        for song in results:
            result = self._parse_song(song)
            if result:
                parsed_results.append(result)

        return parsed_results

    def _parse_song(self, song: dict) -> ProviderResult | None:
        """Parse a single YouTube Music song object into a ProviderResult.

        Extracts standard metadata, video ID, and thumbnail URLs.

        Args:
            song: A single song object from ytmusicapi.

        Returns:
            ProviderResult with metadata and cover art assets.
        """
        try:
            # Extract standard metadata
            title = song.get("title", "")
            video_id = song.get("videoId", "")

            # Extract artist (first artist if multiple)
            artists = song.get("artists", [])
            artist = artists[0].get("name", "") if artists else ""

            # Extract album (may be None for singles)
            album_obj = song.get("album", {})
            album = album_obj.get("name", "") if album_obj else ""

            # Extract duration (in seconds, convert to string)
            duration_seconds = song.get("duration_seconds", 0)

            # Build YouTube Music URL
            url = f"https://music.youtube.com/watch?v={video_id}" if video_id else ""

            # Extract cover art from thumbnails
            cover_art = self._extract_cover_art(song)

            # Build extra tags
            extra_tags = {}
            if video_id:
                extra_tags["custom_youtube_music_id"] = video_id
            if url:
                extra_tags["custom_youtube_music_url"] = url
            if duration_seconds:
                extra_tags["custom_youtube_music_duration"] = str(duration_seconds)

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                artist=artist,
                album=album,
                provider_id=video_id,
                provider_url=url,
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse YouTube Music song: {e}")
            return None

    def _extract_cover_art(self, song: dict) -> list[CoverArtAsset]:
        """Extract cover art from song thumbnails.

        YouTube Music provides thumbnails in multiple resolutions.
        We select the largest (last in the list).

        Args:
            song: Song object from ytmusicapi.

        Returns:
            list[CoverArtAsset]: Cover art assets (static JPEG).
        """
        assets = []

        # Get thumbnails array
        thumbnails = song.get("thumbnails", [])
        if not thumbnails:
            return assets

        # Use the last (largest) thumbnail
        largest_thumbnail = thumbnails[-1]
        thumbnail_url = largest_thumbnail.get("url", "")
        width = largest_thumbnail.get("width", 0)
        height = largest_thumbnail.get("height", 0)

        if thumbnail_url:
            assets.append(CoverArtAsset(
                url=thumbnail_url,
                asset_type=CoverArtType.STATIC,
                format="jpeg",
                width=width,
                height=height,
                description="YouTube Music thumbnail",
            ))

        return assets
