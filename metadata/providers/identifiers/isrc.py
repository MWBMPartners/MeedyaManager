# ============================================================================
# File: /metadata/providers/identifiers/isrc.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# ISRC (International Standard Recording Code) lookup provider using
# federated lookup via MusicBrainz. ISRC codes uniquely identify specific
# recordings and are used globally by music industry for rights management.
#
# ISRC Format:
# - 12 alphanumeric characters: CC-XXX-YY-NNNNN
#   - CC: 2-letter country code (e.g., US, GB, JP)
#   - XXX: 3-character registrant code (assigned to record label)
#   - YY: 2-digit year (last 2 digits of registration year)
#   - NNNNN: 5-digit designation code (unique to this recording)
#
# Authentication:
# No API key required — uses MusicBrainz as backend (free service).
# Inherits MusicBrainz rate limiting (1 req/sec).
#
# Also supports UPC/GTIN (barcode) lookup if available in future releases.
# ============================================================================

import re                                                  # Regular expression for ISRC validation
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

logger = logging.getLogger("MeedyaManager.Provider.ISRC")

# ============================================================================
# MusicBrainz API Constants (used as backend for ISRC lookup)
# ============================================================================
API_BASE = "https://musicbrainz.org/ws/2"                  # MusicBrainz Web Service v2
ISRC_LOOKUP = "/isrc/{isrc}"                               # Look up by ISRC code

# Cover Art Archive
COVER_ART_BASE = "https://coverartarchive.org"
COVER_ART_FRONT = "/release/{mbid}/front-500"

# User-Agent header (mandatory for MusicBrainz API)
USER_AGENT = "MeedyaManager/1.4 (lance.manasse@mwbmpartners.com)"

# ISRC format validation pattern
# Format: CC-XXX-YY-NNNNN (hyphens optional)
ISRC_PATTERN = re.compile(r"^[A-Z]{2}[A-Z0-9]{3}\d{2}\d{5}$")


@register_provider
class ISRCProvider(BaseProvider):
    """ISRC code lookup provider using federated MusicBrainz backend.

    Provides precise track identification by ISRC code with metadata:
    - ISRC validation and formatting
    - Track metadata from MusicBrainz recordings linked to ISRC
    - Release information and cover art
    - UPC/GTIN (barcode) support for future expansion

    No authentication required — uses free MusicBrainz service.
    """

    provider_name = "isrc"                                 # Unique provider identifier

    def __init__(self):
        """Initialise the ISRC provider with HTTP client and rate limiter."""
        super().__init__()
        self._credentials = CredentialManager()            # Not used, consistent with pattern
        self._rate_limiter = get_rate_limiter("isrc")      # Rate limiter (federated)
        self._http_client = None                           # Lazy httpx client

    @property
    def category(self) -> ProviderCategory:
        """ISRC is an identifier provider."""
        return ProviderCategory.IDENTIFIER

    @property
    def capabilities(self) -> ProviderCapabilities:
        """ISRC supports ISRC and UPC/GTIN lookup."""
        return ProviderCapabilities(
            can_lookup_isrc=True,                          # Primary function
            can_lookup_upc=True,                           # UPC/GTIN barcode lookup
        )

    @property
    def requires_auth(self) -> bool:
        """ISRC provider does not require API credentials."""
        return False

    def is_available(self) -> bool:
        """ISRC provider is always available (uses free MusicBrainz backend).

        Returns:
            bool: Always True.
        """
        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search by ISRC code or UPC/GTIN barcode.

        Validates and formats the ISRC code, then performs federated lookup
        via MusicBrainz. Returns all recordings linked to that ISRC.

        Args:
            query: dict with keys: isrc (ISRC code) or upc (UPC/GTIN barcode).

        Returns:
            list[ProviderResult]: Matching recordings with metadata and cover art.
        """
        # Check if ISRC code is present
        if query.get("isrc"):
            isrc = self._normalize_isrc(query["isrc"])
            if not self._validate_isrc(isrc):
                logger.warning(f"Invalid ISRC format: {query['isrc']}")
                return []
            return await self._lookup_by_isrc(isrc)

        # UPC/GTIN lookup (future expansion)
        if query.get("upc"):
            logger.info("UPC/GTIN lookup not yet implemented")
            return []

        logger.warning("ISRC search: no ISRC or UPC provided")
        return []

    async def _lookup_by_isrc(self, isrc: str) -> list[ProviderResult]:
        """Look up recordings by ISRC code via MusicBrainz.

        Args:
            isrc: Normalized ISRC code (12 alphanumeric, no hyphens).

        Returns:
            list[ProviderResult]: Recordings linked to this ISRC.
        """
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = API_BASE + ISRC_LOOKUP.format(isrc=isrc)
            params = {
                "fmt": "json",
                "inc": "artist-credits+releases",          # Include artist and release data
            }
            headers = {"User-Agent": USER_AGENT}

            response = await client.get(url, params=params, headers=headers)
            response.raise_for_status()
            data = response.json()

            return self._parse_isrc_response(data, isrc)

        except Exception as e:
            logger.error(f"ISRC lookup failed for {isrc}: {e}")
            return []

    # ========================================================================
    # Response Parsing
    # ========================================================================

    def _parse_isrc_response(self, data: dict, isrc: str) -> list[ProviderResult]:
        """Parse MusicBrainz ISRC lookup response into ProviderResult list.

        Args:
            data: Raw JSON response from MusicBrainz ISRC endpoint.
            isrc: The ISRC code that was looked up.

        Returns:
            list[ProviderResult]: Parsed results with metadata and cover art.
        """
        results = []
        recordings = data.get("recordings", [])

        for recording in recordings:
            result = self._parse_recording(recording, isrc)
            if result:
                results.append(result)

        return results

    def _parse_recording(self, recording: dict, isrc: str) -> ProviderResult | None:
        """Parse a single MusicBrainz recording into a ProviderResult.

        Args:
            recording: Recording object from MusicBrainz API.
            isrc: The ISRC code associated with this recording.

        Returns:
            ProviderResult with metadata, ISRC, and cover art.
        """
        try:
            # Extract recording MBID and title
            recording_id = recording.get("id", "")
            title = recording.get("title", "")

            # Extract artist from artist-credit
            artist_credits = recording.get("artist-credit", [])
            if artist_credits:
                artist = "".join([ac.get("name", "") for ac in artist_credits])
            else:
                artist = ""

            # Extract release information
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
                if release_date:
                    year = release_date.split("-")[0]

            # Build cover art from Cover Art Archive
            cover_art = []
            if release_mbid:
                cover_url = COVER_ART_BASE + COVER_ART_FRONT.format(mbid=release_mbid)
                cover_art.append(CoverArtAsset(
                    url=cover_url,
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                    width=500,
                    height=500,
                    description="ISRC lookup via MusicBrainz Cover Art Archive",
                ))

            # Build extra tags with ISRC source information
            extra_tags = {
                "custom_isrc_source": "musicbrainz",       # Which provider confirmed the ISRC
            }
            if recording_id:
                extra_tags["custom_musicbrainz_recording_id"] = recording_id
            if release_mbid:
                extra_tags["custom_musicbrainz_release_id"] = release_mbid

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                artist=artist,
                album=album,
                isrc=isrc,
                year=year,
                provider_id=recording_id,                  # Use MusicBrainz recording MBID
                provider_url=f"https://musicbrainz.org/recording/{recording_id}",
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse ISRC recording: {e}")
            return None

    # ========================================================================
    # ISRC Validation and Formatting
    # ========================================================================

    def _normalize_isrc(self, isrc: str) -> str:
        """Normalize ISRC code by removing hyphens and converting to uppercase.

        Args:
            isrc: Raw ISRC code (may contain hyphens, mixed case).

        Returns:
            str: Normalized ISRC code (uppercase, no hyphens).
        """
        # Remove all hyphens and convert to uppercase
        return isrc.replace("-", "").upper()

    def _validate_isrc(self, isrc: str) -> bool:
        """Validate ISRC code format.

        Format: 12 alphanumeric characters (CCXXXYYNNNN)
        - CC: 2 letters (country code)
        - XXX: 3 alphanumeric (registrant code)
        - YY: 2 digits (year)
        - NNNNN: 5 digits (designation code)

        Args:
            isrc: Normalized ISRC code (no hyphens, uppercase).

        Returns:
            bool: True if valid ISRC format, False otherwise.
        """
        return bool(ISRC_PATTERN.match(isrc))

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
