# ============================================================================
# File: /metadata/providers/music/musicbrainz.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# MusicBrainz metadata provider using the MusicBrainz Web Service API (v2).
# Supports track/album search with standard metadata including ISRC codes,
# MusicBrainz IDs (MBID), and cover art from the Cover Art Archive.
#
# Authentication:
# No API key required — only a mandatory User-Agent header identifying
# the application and providing contact information for rate limiting.
# Must follow format: "AppName/Version (contact_email)"
#
# Rate limiting:
# MusicBrainz strictly enforces 1 request per second. Exceeding this
# limit may result in temporary IP bans. Always use the rate limiter.
#
# Cover Art Archive:
# Free service providing cover art for MusicBrainz releases via MBID.
# URL pattern: https://coverartarchive.org/release/{mbid}/front-500
# ============================================================================

import logging                                             # Standard logging
from urllib.parse import quote                             # URL encoding for Lucene queries

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

logger = logging.getLogger("MeedyaManager.Provider.MusicBrainz")

# ============================================================================
# MusicBrainz API Constants
# ============================================================================
API_BASE = "https://musicbrainz.org/ws/2"                  # MusicBrainz Web Service v2
RECORDING_SEARCH = "/recording"                            # Search recordings (tracks)
RELEASE_SEARCH = "/release"                                # Search releases (albums)
ISRC_LOOKUP = "/isrc/{isrc}"                               # Look up by ISRC code

# Cover Art Archive (free companion service to MusicBrainz)
COVER_ART_BASE = "https://coverartarchive.org"             # Cover Art Archive base URL
COVER_ART_FRONT = "/release/{mbid}/front-500"              # Front cover (500px) by release MBID

# User-Agent header (mandatory for MusicBrainz API access)
USER_AGENT = "MeedyaManager/1.4 (lance.manasse@mwbmpartners.com)"


@register_provider
class MusicBrainzProvider(BaseProvider):
    """MusicBrainz metadata provider using the Web Service API v2.

    Provides comprehensive music metadata including:
    - Track/album search with title, artist, album matching
    - ISRC code lookup for precise track identification
    - MusicBrainz IDs (recording MBID, release MBID, artist MBID)
    - Cover art from Cover Art Archive (static JPEG, 500px)

    No authentication required — only User-Agent header for rate limiting.
    Strictly enforces 1 request per second rate limit.
    """

    provider_name = "musicbrainz"                          # Unique provider identifier

    def __init__(self):
        """Initialise the MusicBrainz provider with HTTP client and rate limiter."""
        super().__init__()
        self._credentials = CredentialManager()            # Not used, but consistent with pattern
        self._rate_limiter = get_rate_limiter("musicbrainz")  # 1 req/sec rate limiter
        self._http_client = None                           # Lazy httpx client

    @property
    def category(self) -> ProviderCategory:
        """MusicBrainz is a music provider."""
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        """MusicBrainz supports track/album search, ISRC lookup, and static cover art."""
        return ProviderCapabilities(
            can_search_tracks=True,                        # Search individual recordings
            can_search_albums=True,                        # Search releases
            can_lookup_isrc=True,                          # ISRC available in recordings
            has_static_cover_art=True,                     # Cover Art Archive provides JPEGs
        )

    @property
    def requires_auth(self) -> bool:
        """MusicBrainz does not require API credentials (only User-Agent header)."""
        return False

    def is_available(self) -> bool:
        """MusicBrainz is always available — no credentials required.

        Returns:
            bool: Always True (only User-Agent header needed, which is hardcoded).
        """
        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search MusicBrainz catalog for matching recordings.

        Builds a Lucene query from the metadata and searches the recording
        endpoint. If ISRC is present, uses direct ISRC lookup for higher
        accuracy. Otherwise, constructs a text search query.

        Args:
            query: dict with keys: title, artist, album, isrc.

        Returns:
            list[ProviderResult]: Matching results with MBIDs and cover art.
        """
        # If ISRC is available, use direct ISRC lookup (most accurate)
        if query.get("isrc"):
            return await self._lookup_by_isrc(query["isrc"])

        # Build Lucene query from available metadata
        lucene_parts = []
        if query.get("title"):
            # Escape special characters and wrap in quotes for exact phrase
            title = self._escape_lucene(query["title"])
            lucene_parts.append(f'recording:"{title}"')
        if query.get("artist"):
            artist = self._escape_lucene(query["artist"])
            lucene_parts.append(f'artist:"{artist}"')
        if query.get("album"):
            album = self._escape_lucene(query["album"])
            lucene_parts.append(f'release:"{album}"')

        if not lucene_parts:
            logger.warning("MusicBrainz search: no query terms provided")
            return []

        lucene_query = " AND ".join(lucene_parts)

        # Make the API request
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = API_BASE + RECORDING_SEARCH
            params = {
                "query": lucene_query,                     # Lucene query string
                "fmt": "json",                             # JSON response format
                "limit": 10,                               # Maximum 10 results
                "inc": "artist-credits+releases+isrcs",    # Include artist, release, ISRC data
            }
            headers = {"User-Agent": USER_AGENT}

            response = await client.get(url, params=params, headers=headers)
            response.raise_for_status()
            data = response.json()

            return self._parse_recording_search(data, query)

        except Exception as e:
            logger.error(f"MusicBrainz search failed: {e}")
            return []

    async def _lookup_by_isrc(self, isrc: str) -> list[ProviderResult]:
        """Look up a recording by its ISRC code.

        ISRC lookup is more accurate than text search because ISRC codes
        uniquely identify specific recordings. MusicBrainz may have multiple
        recordings linked to the same ISRC (different releases).

        Args:
            isrc: ISRC code (12 alphanumeric characters, e.g., USTEST1234567).

        Returns:
            list[ProviderResult]: Matching recordings (usually 1, may be multiple).
        """
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            # ISRC endpoint: /ws/2/isrc/{isrc}?inc=...
            url = API_BASE + ISRC_LOOKUP.format(isrc=isrc)
            params = {
                "fmt": "json",
                "inc": "artist-credits+releases",          # Include artist and release data
            }
            headers = {"User-Agent": USER_AGENT}

            response = await client.get(url, params=params, headers=headers)
            response.raise_for_status()
            data = response.json()

            # Parse recordings from ISRC lookup response
            recordings = data.get("recordings", [])
            results = []
            for recording in recordings:
                result = self._parse_recording(recording, isrc=isrc)
                if result:
                    results.append(result)

            return results

        except Exception as e:
            logger.error(f"MusicBrainz ISRC lookup failed for {isrc}: {e}")
            return []

    # ========================================================================
    # Response Parsing
    # ========================================================================

    def _parse_recording_search(self, data: dict, query: dict) -> list[ProviderResult]:
        """Parse MusicBrainz recording search response into ProviderResult list.

        Args:
            data: Raw JSON response from recording search endpoint.
            query: Original query dict (used for confidence scoring).

        Returns:
            list[ProviderResult]: Parsed results with MBIDs and cover art.
        """
        results = []
        recordings = data.get("recordings", [])

        for recording in recordings:
            # Extract ISRC if present in the recording
            isrcs = recording.get("isrcs", [])
            isrc = isrcs[0] if isrcs else ""
            result = self._parse_recording(recording, isrc=isrc)
            if result:
                results.append(result)

        return results

    def _parse_recording(self, recording: dict, isrc: str = "") -> ProviderResult | None:
        """Parse a single MusicBrainz recording object into a ProviderResult.

        Extracts standard metadata, MBIDs (recording, release, artist), and
        constructs cover art URL from Cover Art Archive if release MBID present.

        Args:
            recording: A single recording object from MusicBrainz API.
            isrc: ISRC code if known (may come from query or recording data).

        Returns:
            ProviderResult with metadata, MBIDs, and cover art URL.
        """
        try:
            # Extract recording MBID and title
            recording_id = recording.get("id", "")
            title = recording.get("title", "")

            # Extract artist from artist-credit (may be multiple artists)
            artist_credits = recording.get("artist-credit", [])
            if artist_credits:
                # Concatenate all artist names (handles features/collaborations)
                artist = "".join([ac.get("name", "") for ac in artist_credits])
            else:
                artist = ""

            # Extract release information (album) — use first release if multiple
            releases = recording.get("releases", [])
            album = ""
            release_mbid = ""
            release_date = ""
            year = ""

            if releases:
                first_release = releases[0]
                album = first_release.get("title", "")
                release_mbid = first_release.get("id", "")
                release_date = first_release.get("date", "")
                # Extract year from date (format: YYYY-MM-DD or YYYY)
                if release_date:
                    year = release_date.split("-")[0]

            # Build cover art assets from Cover Art Archive
            cover_art = []
            if release_mbid:
                # Cover Art Archive provides front cover at 500px resolution
                cover_url = COVER_ART_BASE + COVER_ART_FRONT.format(mbid=release_mbid)
                cover_art.append(CoverArtAsset(
                    url=cover_url,
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                    width=500,
                    height=500,
                    description="MusicBrainz Cover Art Archive",
                ))

            # Build extra tags with MusicBrainz IDs
            extra_tags = {}
            if recording_id:
                extra_tags["custom_musicbrainz_recording_id"] = recording_id
            if release_mbid:
                extra_tags["custom_musicbrainz_release_id"] = release_mbid
            if artist_credits and artist_credits[0].get("artist", {}).get("id"):
                artist_mbid = artist_credits[0]["artist"]["id"]
                extra_tags["custom_musicbrainz_artist_id"] = artist_mbid
            # MusicBrainz URL for the recording
            if recording_id:
                mb_url = f"https://musicbrainz.org/recording/{recording_id}"
                extra_tags["custom_musicbrainz_url"] = mb_url

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                artist=artist,
                album=album,
                isrc=isrc,
                year=year,
                provider_id=recording_id,
                provider_url=extra_tags.get("custom_musicbrainz_url", ""),
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse MusicBrainz recording: {e}")
            return None

    # ========================================================================
    # Utility Methods
    # ========================================================================

    def _escape_lucene(self, text: str) -> str:
        """Escape special characters in Lucene query syntax.

        Lucene uses special characters: + - && || ! ( ) { } [ ] ^ " ~ * ? : \\
        These must be escaped with backslash for literal matching.

        Args:
            text: Raw search term to escape.

        Returns:
            str: Escaped search term safe for Lucene query.
        """
        # Characters that need escaping in Lucene
        special_chars = ['+', '-', '&', '|', '!', '(', ')', '{', '}',
                        '[', ']', '^', '"', '~', '*', '?', ':', '\\']
        for char in special_chars:
            text = text.replace(char, f"\\{char}")
        return text

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
