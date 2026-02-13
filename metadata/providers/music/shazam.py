# ============================================================================
# File: /metadata/providers/music/shazam.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Shazam metadata provider using the unofficial shazamio Python library.
# Supports both text-based track search and audio fingerprinting for
# precise track identification from audio files.
#
# Authentication:
# No API key required — uses reverse-engineered Shazam API via shazamio.
# This is an unofficial integration and may break if Shazam changes their API.
#
# Dependencies:
# - shazamio: Python library for Shazam API (pip install shazamio)
#   Lazy import — provider is unavailable if shazamio is not installed.
#
# Features:
# - Text search: Search Shazam catalog by title/artist/album
# - Audio fingerprinting: Identify tracks from audio files (most accurate)
# - Cover art: Static cover images from Shazam's CDN
# - Metadata: Title, artist, genres, Shazam URL
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

logger = logging.getLogger("MeedyaManager.Provider.Shazam")


@register_provider
class ShazamProvider(BaseProvider):
    """Shazam metadata provider using unofficial shazamio library.

    Provides music metadata and audio fingerprinting:
    - Text search: Search by title, artist, album
    - Audio fingerprinting: Identify tracks from audio files
    - Static cover art from Shazam CDN
    - Genre classification
    - Shazam track URLs for sharing/linking

    No authentication required — uses reverse-engineered API.
    Requires shazamio library (lazy import, optional dependency).
    """

    provider_name = "shazam"                               # Unique provider identifier

    def __init__(self):
        """Initialise the Shazam provider with optional shazamio import."""
        super().__init__()
        self._credentials = CredentialManager()            # Not used, consistent with pattern
        self._rate_limiter = get_rate_limiter("shazam")    # Rate limiter (conservative)
        self._shazam_client = None                         # Lazy shazamio.Shazam instance

    @property
    def category(self) -> ProviderCategory:
        """Shazam is a music provider."""
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        """Shazam supports track search, audio fingerprinting, and static cover art."""
        return ProviderCapabilities(
            can_search_tracks=True,                        # Text-based track search
            can_fingerprint_audio=True,                    # Audio fingerprint recognition
            has_static_cover_art=True,                     # Cover art from Shazam CDN
        )

    @property
    def requires_auth(self) -> bool:
        """Shazam does not require API credentials (unofficial API)."""
        return False

    def is_available(self) -> bool:
        """Check if shazamio library is installed and available.

        Returns:
            bool: True if shazamio can be imported, False otherwise.
        """
        try:
            import shazamio                                # Lazy import check
            return True
        except ImportError:
            logger.warning("shazamio not installed — Shazam provider unavailable")
            return False

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search Shazam catalog for matching tracks.

        If file_path is present, uses audio fingerprinting for precise
        identification. Otherwise, constructs text search from metadata.

        Args:
            query: dict with keys: title, artist, album, file_path.

        Returns:
            list[ProviderResult]: Matching results with cover art and metadata.
        """
        # Check if shazamio is available
        if not self.is_available():
            logger.error("Shazam provider unavailable — shazamio not installed")
            return []

        # If file path is present, use audio fingerprinting (most accurate)
        if query.get("file_path"):
            return await self._fingerprint_audio(query["file_path"])

        # Otherwise, use text-based search
        search_parts = []
        if query.get("title"):
            search_parts.append(query["title"])
        if query.get("artist"):
            search_parts.append(query["artist"])

        if not search_parts:
            logger.warning("Shazam search: no query terms or file path provided")
            return []

        search_term = " ".join(search_parts)
        return await self._text_search(search_term)

    async def _text_search(self, query: str) -> list[ProviderResult]:
        """Search Shazam catalog by text query.

        Args:
            query: Search term (title, artist, or combination).

        Returns:
            list[ProviderResult]: Matching tracks from Shazam.
        """
        try:
            # Import shazamio lazily
            from shazamio import Shazam

            await self._rate_limiter.acquire()

            # Create Shazam client if needed
            if self._shazam_client is None:
                self._shazam_client = Shazam()

            # Perform text search
            results = await self._shazam_client.search_track(query=query, limit=10)

            # Parse results
            return self._parse_search_results(results)

        except Exception as e:
            logger.error(f"Shazam text search failed: {e}")
            return []

    async def _fingerprint_audio(self, file_path: str) -> list[ProviderResult]:
        """Identify a track using audio fingerprinting.

        Reads the audio file and uses Shazam's fingerprinting algorithm
        to identify the track. This is the most accurate method.

        Args:
            file_path: Path to audio file to fingerprint.

        Returns:
            list[ProviderResult]: Identified track (single result or empty).
        """
        try:
            # Import shazamio lazily
            from shazamio import Shazam

            await self._rate_limiter.acquire()

            # Create Shazam client if needed
            if self._shazam_client is None:
                self._shazam_client = Shazam()

            # Recognize track from file
            result = await self._shazam_client.recognize(file_path)

            # Parse single result
            if result and "track" in result:
                parsed = self._parse_track(result["track"])
                return [parsed] if parsed else []
            return []

        except Exception as e:
            logger.error(f"Shazam audio fingerprinting failed: {e}")
            return []

    # ========================================================================
    # Response Parsing
    # ========================================================================

    def _parse_search_results(self, data: dict) -> list[ProviderResult]:
        """Parse Shazam text search response into ProviderResult list.

        Args:
            data: Raw JSON response from shazamio search_track().

        Returns:
            list[ProviderResult]: Parsed results with metadata and cover art.
        """
        results = []
        tracks = data.get("tracks", {}).get("hits", [])

        for hit in tracks:
            track = hit.get("track", {})
            result = self._parse_track(track)
            if result:
                results.append(result)

        return results

    def _parse_track(self, track: dict) -> ProviderResult | None:
        """Parse a single Shazam track object into a ProviderResult.

        Extracts title, artist (subtitle), cover art, genres,
        and Shazam URL for sharing.

        Args:
            track: A single track object from Shazam API.

        Returns:
            ProviderResult with metadata and cover art.
        """
        try:
            # Extract basic metadata
            track_id = track.get("key", "")                # Shazam track ID
            title = track.get("title", "")
            artist = track.get("subtitle", "")             # Artist name in subtitle field

            # Extract genres (may be multiple, different formats)
            genre_data = track.get("genres", {})
            genres = ""
            if isinstance(genre_data, dict):
                # Dict format: {"primary": "Pop"}
                genres = genre_data.get("primary", "")
            elif isinstance(genre_data, list) and genre_data:
                # List format: ["Alternative", "Indie"]
                genres = genre_data[0]

            # Extract Shazam URL
            shazam_url = track.get("url", "")

            # Extract cover art
            cover_art = []
            images = track.get("images", {})
            cover_url = images.get("coverart", "")         # Primary cover art URL
            if cover_url:
                cover_art.append(CoverArtAsset(
                    url=cover_url,
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                    description="Shazam cover art",
                ))

            # Build extra tags
            extra_tags = {}
            if track_id:
                extra_tags["custom_shazam_id"] = track_id
            if shazam_url:
                extra_tags["custom_shazam_url"] = shazam_url

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                artist=artist,
                genre=genres,
                provider_id=str(track_id),
                provider_url=shazam_url,
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse Shazam track: {e}")
            return None
