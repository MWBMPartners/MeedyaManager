# ============================================================================
# File: /metadata/providers/music/tidal.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tidal metadata provider using the Tidal OpenAPI (v2) or legacy API (v1).
# Supports track/album search with rich metadata including ISRC codes,
# audio quality indicators (LOSSLESS, HI_RES, HI_RES_LOSSLESS),
# spatial audio formats (DOLBY_ATMOS, SONY_360RA), and static cover art
# up to 1280x1280 pixels.
#
# Authentication:
# Uses OAuth2.1 Client Credentials flow. Requires TIDAL_CLIENT_ID and
# TIDAL_CLIENT_SECRET from the Tidal Developer program.
# Tokens are cached until expiry (typically 3600 seconds / 1 hour).
#
# Required credentials (via CredentialManager):
# - TIDAL_CLIENT_ID: Tidal app client ID
# - TIDAL_CLIENT_SECRET: Tidal app client secret
#
# API Documentation:
# https://developer.tidal.com/documentation/api/api-overview
# ============================================================================

import time                                                # Token expiry tracking
import logging                                             # Standard logging
from base64 import b64encode                               # For OAuth2 Basic auth

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

logger = logging.getLogger("MeedyaManager.Provider.Tidal")

# ============================================================================
# Tidal API Constants
# ============================================================================
AUTH_URL = "https://auth.tidal.com/v1/oauth2/token"        # OAuth2.1 token endpoint
API_BASE_V1 = "https://api.tidal.com/v1"                   # Legacy API (v1)
API_BASE_V2 = "https://openapi.tidal.com/v2"               # OpenAPI (v2)
SEARCH_ENDPOINT_V1 = "/search"                             # v1 search endpoint
SEARCH_ENDPOINT_V2 = "/searchresults/{query}"              # v2 search endpoint

# OAuth2 token settings
TOKEN_REFRESH_MARGIN = 300                                 # Refresh 5 minutes before expiry

# Audio quality tier mappings
QUALITY_TIERS = {
    "LOW": "Low",
    "HIGH": "High",
    "LOSSLESS": "Lossless",
    "HI_RES": "Hi-Res",
    "HI_RES_LOSSLESS": "Hi-Res Lossless",
}


@register_provider
class TidalProvider(BaseProvider):
    """Tidal metadata provider using Tidal OpenAPI.

    Provides rich music metadata including:
    - Track/album search with title, artist, album, ISRC
    - Static cover art up to 1280x1280 JPEG
    - Audio quality tiers (Lossless, Hi-Res, Hi-Res Lossless)
    - Spatial audio indicators (Dolby Atmos, Sony 360 Reality Audio)
    - Explicit content flags

    Requires Tidal Developer credentials (OAuth2.1 Client Credentials).
    """

    provider_name = "tidal"                                # Unique provider identifier

    def __init__(self):
        """Initialise the Tidal provider with credentials and HTTP client."""
        super().__init__()
        self._credentials = CredentialManager()            # Credential resolution
        self._rate_limiter = get_rate_limiter("tidal")     # Rate limiter
        self._http_client = None                           # Lazy httpx client
        self._access_token: str | None = None              # Cached OAuth2 access token
        self._token_expiry: float = 0.0                    # Token expiry timestamp
        self._country_code = "GB"                          # Default country code

        # Load country code from config if available
        try:
            from utils.config_loader import load_config
            config = load_config() or {}
            providers = config.get("providers", {})
            tidal_config = providers.get("tidal", {})
            if tidal_config.get("country_code"):
                self._country_code = tidal_config["country_code"]
        except Exception:
            pass                                           # Use default if config unavailable

    @property
    def category(self) -> ProviderCategory:
        """Tidal is a music provider."""
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        """Tidal supports track/album search with static cover art."""
        return ProviderCapabilities(
            can_search_tracks=True,                        # Search individual songs
            can_search_albums=True,                        # Search albums
            can_lookup_isrc=True,                          # ISRC available in responses
            has_static_cover_art=True,                     # JPEG cover art up to 1280x1280
        )

    @property
    def requires_auth(self) -> bool:
        """Tidal requires OAuth2.1 Client Credentials authentication."""
        return True

    def is_available(self) -> bool:
        """Check if Tidal API credentials are available.

        Requires client_id and client_secret from Tidal Developer program.
        """
        # Check that both client_id and client_secret are present
        client_id = self._credentials.get_credential("tidal", "client_id")
        client_secret = self._credentials.get_credential("tidal", "client_secret")

        return bool(client_id and client_secret)

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search Tidal catalog for matching tracks.

        Uses the v1 API search endpoint for broader compatibility.
        Constructs a search term from query metadata and returns results
        with metadata, ISRC codes, quality indicators, and cover art.

        Args:
            query: dict with keys: title, artist, album, isrc.

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
            logger.warning("Tidal search: no query terms provided")
            return []

        search_term = " ".join(search_parts)

        # Ensure OAuth2 token is valid
        token = await self._get_or_refresh_token()
        if not token:
            logger.error("Tidal: failed to obtain OAuth2 access token")
            return []

        # Make the API request (using v1 for better stability)
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = API_BASE_V1 + SEARCH_ENDPOINT_V1
            params = {
                "query": search_term,
                "type": "TRACKS",                          # Search tracks only
                "limit": 10,                               # Max 10 results
                "countryCode": self._country_code,         # Country/region code
            }
            headers = {
                "Authorization": f"Bearer {token}",
            }

            response = await client.get(url, params=params, headers=headers)
            response.raise_for_status()
            data = response.json()

            return self._parse_search_results(data)

        except Exception as e:
            logger.error(f"Tidal search failed: {e}")
            return []

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific track by its Tidal ID.

        Args:
            provider_id: Tidal track ID (numeric string).

        Returns:
            ProviderResult if found, None otherwise.
        """
        token = await self._get_or_refresh_token()
        if not token:
            return None

        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = f"{API_BASE_V1}/tracks/{provider_id}"
            params = {
                "countryCode": self._country_code,
            }
            headers = {"Authorization": f"Bearer {token}"}

            response = await client.get(url, params=params, headers=headers)
            response.raise_for_status()
            data = response.json()

            return self._parse_track(data)

        except Exception as e:
            logger.error(f"Tidal lookup failed for {provider_id}: {e}")
            return None

    # ========================================================================
    # OAuth2.1 Client Credentials Flow
    # ========================================================================

    async def _get_or_refresh_token(self) -> str | None:
        """Get a valid OAuth2 access token, requesting a new one if needed.

        OAuth2.1 Client Credentials tokens are cached and reused until expiry.
        The token is obtained by POSTing client_id and client_secret to the
        Tidal token endpoint with Basic authentication.

        Returns:
            Access token string, or None if authentication failed.
        """
        now = time.time()

        # Return cached token if still valid
        if self._access_token and (now + TOKEN_REFRESH_MARGIN) < self._token_expiry:
            return self._access_token

        # Request new token
        try:
            client_id = self._credentials.get_credential("tidal", "client_id")
            client_secret = self._credentials.get_credential("tidal", "client_secret")

            if not client_id or not client_secret:
                logger.error("Tidal: missing client_id or client_secret")
                return None

            # Encode credentials for Basic auth
            credentials = f"{client_id}:{client_secret}"
            credentials_b64 = b64encode(credentials.encode()).decode()

            # Prepare token request
            http_client = await self._get_http_client()
            headers = {
                "Authorization": f"Basic {credentials_b64}",
                "Content-Type": "application/x-www-form-urlencoded",
            }
            data = {
                "grant_type": "client_credentials",
            }

            # Request token
            response = await http_client.post(AUTH_URL, headers=headers, data=data)
            response.raise_for_status()
            token_data = response.json()

            # Extract access token and expiry
            access_token = token_data.get("access_token")
            expires_in = token_data.get("expires_in", 3600)  # Default 1 hour

            if not access_token:
                logger.error("Tidal: no access_token in response")
                return None

            # Cache token
            self._access_token = access_token
            self._token_expiry = now + expires_in
            logger.info("Tidal OAuth2.1 token obtained successfully")
            return access_token

        except Exception as e:
            logger.error(f"Tidal OAuth2 token request failed: {e}")
            return None

    # ========================================================================
    # Response Parsing
    # ========================================================================

    def _parse_search_results(self, data: dict) -> list[ProviderResult]:
        """Parse Tidal search API response into ProviderResult list.

        Args:
            data: Raw JSON response from the search endpoint.

        Returns:
            list[ProviderResult]: Parsed results with cover art assets.
        """
        results = []
        tracks = data.get("tracks", {}).get("items", [])

        for track in tracks:
            result = self._parse_track(track)
            if result:
                results.append(result)

        return results

    def _parse_track(self, track: dict) -> ProviderResult | None:
        """Parse a single Tidal track object into a ProviderResult.

        Extracts standard metadata, ISRC, quality indicators, and cover art.

        Args:
            track: A single track object from the Tidal API.

        Returns:
            ProviderResult with metadata and cover art assets.
        """
        try:
            track_id = str(track.get("id", ""))

            # Extract standard metadata
            title = track.get("title", "")
            artists = track.get("artists", [])
            artist = artists[0].get("name", "") if artists else ""
            album_data = track.get("album", {})
            album = album_data.get("title", "")

            # Extract ISRC
            isrc = track.get("isrc", "")

            # Track/disc numbers
            track_num = str(track.get("trackNumber", ""))
            disc_num = str(track.get("volumeNumber", ""))

            # Release date
            release_date = track.get("streamStartDate", "")
            if not release_date:
                # Try album release date
                release_date = album_data.get("releaseDate", "")
            year = release_date[:4] if release_date else ""

            # URL
            url = track.get("url", "")

            # Explicit flag
            explicit = track.get("explicit", False)

            # Audio quality and modes
            audio_quality = track.get("audioQuality", "")
            audio_modes = track.get("audioModes", [])

            # Build cover art assets
            cover_art = self._extract_cover_art(album_data)

            # Build extra tags
            extra_tags = {}
            if track_id:
                extra_tags["custom_tidal_id"] = track_id
            if url:
                extra_tags["custom_tidal_url"] = url
            if isrc:
                extra_tags["custom_tidal_isrc"] = isrc
            if explicit:
                extra_tags["custom_tidal_explicit"] = "true"

            # Add quality tier
            if audio_quality:
                quality_label = QUALITY_TIERS.get(audio_quality, audio_quality)
                extra_tags["custom_tidal_quality"] = quality_label

            # Add spatial audio modes
            if audio_modes:
                if "DOLBY_ATMOS" in audio_modes:
                    extra_tags["custom_tidal_dolby_atmos"] = "true"
                if "SONY_360RA" in audio_modes:
                    extra_tags["custom_tidal_sony_360ra"] = "true"

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                artist=artist,
                album=album,
                year=year,
                isrc=isrc,
                track_num=track_num,
                disc_num=disc_num,
                provider_id=track_id,
                provider_url=url,
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse Tidal track: {e}")
            return None

    def _extract_cover_art(self, album_data: dict) -> list[CoverArtAsset]:
        """Extract cover art assets from album data.

        Tidal provides cover art via a UUID-based URL template.
        Cover art is available at: https://resources.tidal.com/images/{uuid}/{width}x{height}.jpg
        Common sizes: 160x160, 320x320, 640x640, 1280x1280

        Args:
            album_data: Album object from Tidal API.

        Returns:
            list[CoverArtAsset]: Static cover art assets.
        """
        assets = []
        cover_uuid = album_data.get("cover", "")

        if cover_uuid:
            # Build URL for maximum resolution (1280x1280)
            # Replace dashes in UUID if needed (Tidal UUIDs may have dashes removed)
            cover_uuid_clean = cover_uuid.replace("-", "")
            cover_url = f"https://resources.tidal.com/images/{cover_uuid_clean}/1280x1280.jpg"

            assets.append(CoverArtAsset(
                url=cover_url,
                asset_type=CoverArtType.STATIC,
                format="jpeg",
                width=1280,
                height=1280,
                description="Tidal album cover",
            ))

        return assets

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
