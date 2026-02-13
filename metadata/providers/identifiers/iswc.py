# ============================================================================
# File: /metadata/providers/identifiers/iswc.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# ISWC (International Standard Musical Work Code) lookup provider using
# federated lookup via MusicBrainz. ISWC codes uniquely identify musical
# works (compositions) independent of recordings, used for publishing
# rights management and royalty distribution.
#
# ISWC Format:
# - T-nnn.nnn.nnn-n (11 characters with hyphens and dots)
#   - T: Literal prefix indicating musical work
#   - nnn.nnn.nnn: 9-digit work identifier with dot separators
#   - n: Check digit (mod 10)
#
# Authentication:
# No API key required — uses MusicBrainz as backend (free service).
# Inherits MusicBrainz rate limiting (1 req/sec).
#
# Note: ISWC identifies works (compositions), not recordings. A single
# work may have many recordings (covers, remixes, live versions).
# ============================================================================

import re                                                  # Regular expression for ISWC validation
import logging                                             # Standard logging

from metadata.providers import ProviderCategory, register_provider
from metadata.providers.base import (
    BaseProvider,                                           # Provider ABC
    ProviderCapabilities,                                   # Capabilities declaration
    ProviderResult,                                        # Result dataclass
)
from metadata.providers.credentials import CredentialManager
from metadata.providers.rate_limiter import get_rate_limiter

logger = logging.getLogger("MeedyaManager.Provider.ISWC")

# ============================================================================
# MusicBrainz API Constants (used as backend for ISWC lookup)
# ============================================================================
API_BASE = "https://musicbrainz.org/ws/2"                  # MusicBrainz Web Service v2
WORK_SEARCH = "/work"                                      # Search works by ISWC
RECORDING_LOOKUP = "/recording/{mbid}"                     # Get recording with work relations

# User-Agent header (mandatory for MusicBrainz API)
USER_AGENT = "MeedyaManager/1.4 (lance.manasse@mwbmpartners.com)"

# ISWC format validation pattern
# Format: T-nnn.nnn.nnn-n (literal T, 9 digits with dots, check digit)
ISWC_PATTERN = re.compile(r"^T-\d{3}\.\d{3}\.\d{3}-\d$")


@register_provider
class ISWCProvider(BaseProvider):
    """ISWC code lookup provider using federated MusicBrainz backend.

    Provides work (composition) identification by ISWC code with metadata:
    - ISWC validation and formatting
    - Work title and composer information from MusicBrainz
    - Related recordings that use this work

    No authentication required — uses free MusicBrainz service.
    Note: ISWC identifies works (compositions), not specific recordings.
    """

    provider_name = "iswc"                                 # Unique provider identifier

    def __init__(self):
        """Initialise the ISWC provider with HTTP client and rate limiter."""
        super().__init__()
        self._credentials = CredentialManager()            # Not used, consistent with pattern
        self._rate_limiter = get_rate_limiter("iswc")      # Rate limiter (federated via MusicBrainz)
        self._http_client = None                           # Lazy httpx client

    @property
    def category(self) -> ProviderCategory:
        """ISWC is an identifier provider."""
        return ProviderCategory.IDENTIFIER

    @property
    def capabilities(self) -> ProviderCapabilities:
        """ISWC supports minimal identifier lookup only."""
        return ProviderCapabilities()                      # Minimal capabilities

    @property
    def requires_auth(self) -> bool:
        """ISWC provider does not require API credentials."""
        return False

    def is_available(self) -> bool:
        """ISWC provider is always available (uses free MusicBrainz backend).

        Returns:
            bool: Always True.
        """
        return True

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search by ISWC code via MusicBrainz work database.

        Validates and formats the ISWC code, then performs federated lookup
        via MusicBrainz works API. Returns work metadata (composition info).

        Args:
            query: dict with keys: iswc (ISWC code) or recording_mbid (for work relations).

        Returns:
            list[ProviderResult]: Work information with title and ISWC.
        """
        # Check if ISWC code is present
        if query.get("iswc"):
            iswc = self._normalize_iswc(query["iswc"])
            if not self._validate_iswc(iswc):
                logger.warning(f"Invalid ISWC format: {query['iswc']}")
                return []
            return await self._lookup_by_iswc(iswc)

        # Alternative: lookup works by recording MBID (future expansion)
        if query.get("recording_mbid"):
            logger.info("ISWC lookup by recording MBID not yet implemented")
            return []

        logger.warning("ISWC search: no ISWC provided")
        return []

    async def _lookup_by_iswc(self, iswc: str) -> list[ProviderResult]:
        """Look up works by ISWC code via MusicBrainz.

        Args:
            iswc: Normalized ISWC code (T-nnn.nnn.nnn-n format).

        Returns:
            list[ProviderResult]: Works with this ISWC (usually 1, may be 0).
        """
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = API_BASE + WORK_SEARCH
            params = {
                "query": f"iswc:{iswc}",                   # Lucene query for ISWC
                "fmt": "json",
                "limit": 10,
            }
            headers = {"User-Agent": USER_AGENT}

            response = await client.get(url, params=params, headers=headers)
            response.raise_for_status()
            data = response.json()

            return self._parse_work_response(data, iswc)

        except Exception as e:
            logger.error(f"ISWC lookup failed for {iswc}: {e}")
            return []

    # ========================================================================
    # Response Parsing
    # ========================================================================

    def _parse_work_response(self, data: dict, iswc: str) -> list[ProviderResult]:
        """Parse MusicBrainz work search response into ProviderResult list.

        Args:
            data: Raw JSON response from MusicBrainz work search endpoint.
            iswc: The ISWC code that was looked up.

        Returns:
            list[ProviderResult]: Parsed results with work metadata.
        """
        results = []
        works = data.get("works", [])

        for work in works:
            result = self._parse_work(work, iswc)
            if result:
                results.append(result)

        return results

    def _parse_work(self, work: dict, iswc: str) -> ProviderResult | None:
        """Parse a single MusicBrainz work into a ProviderResult.

        Args:
            work: Work object from MusicBrainz API.
            iswc: The ISWC code associated with this work.

        Returns:
            ProviderResult with work title and ISWC metadata.
        """
        try:
            # Extract work MBID and title
            work_id = work.get("id", "")
            work_title = work.get("title", "")

            # Build extra tags with ISWC information
            extra_tags = {
                "custom_iswc": iswc,                       # Store ISWC code
                "custom_iswc_work_title": work_title,      # Store work title
            }
            if work_id:
                extra_tags["custom_musicbrainz_work_id"] = work_id

            return ProviderResult(
                provider_name=self.provider_name,
                title=work_title,                          # Work title (composition)
                provider_id=work_id,                       # MusicBrainz work MBID
                provider_url=f"https://musicbrainz.org/work/{work_id}",
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse ISWC work: {e}")
            return None

    # ========================================================================
    # ISWC Validation and Formatting
    # ========================================================================

    def _normalize_iswc(self, iswc: str) -> str:
        """Normalize ISWC code by ensuring correct format.

        Args:
            iswc: Raw ISWC code (may have inconsistent formatting).

        Returns:
            str: Normalized ISWC code (T-nnn.nnn.nnn-n format).
        """
        # Remove all non-alphanumeric characters
        clean = re.sub(r"[^T0-9]", "", iswc.upper())
        # Extract components: T + 9 digits + check digit
        if len(clean) == 11 and clean.startswith("T"):
            # Reformat as T-nnn.nnn.nnn-n
            return f"T-{clean[1:4]}.{clean[4:7]}.{clean[7:10]}-{clean[10]}"
        return iswc  # Return as-is if format is unclear

    def _validate_iswc(self, iswc: str) -> bool:
        """Validate ISWC code format.

        Format: T-nnn.nnn.nnn-n
        - T: Literal prefix for musical work
        - nnn.nnn.nnn: 9-digit work identifier with dot separators
        - n: Check digit

        Args:
            iswc: Normalized ISWC code.

        Returns:
            bool: True if valid ISWC format, False otherwise.
        """
        return bool(ISWC_PATTERN.match(iswc))

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
