# ============================================================================
# File: /metadata/providers/music/spotify.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Spotify metadata provider using the Spotify Web API.
# Supports track/album search with rich metadata including ISRC codes,
# audio features (energy, danceability, tempo, valence, key, mode),
# and static cover art in multiple resolutions.
#
# Authentication:
# Uses OAuth2 Client Credentials flow. Requires SPOTIFY_CLIENT_ID and
# SPOTIFY_CLIENT_SECRET from the Spotify Developer Dashboard.
# Tokens are cached until expiry (typically 3600 seconds / 1 hour).
#
# Required credentials (via CredentialManager):
# - SPOTIFY_CLIENT_ID: Spotify app client ID
# - SPOTIFY_CLIENT_SECRET: Spotify app client secret
#
# API Documentation:
# https://developer.spotify.com/documentation/web-api
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

logger = logging.getLogger("MeedyaManager.Provider.Spotify")

# ============================================================================
# Spotify API Constants
# ============================================================================
API_BASE = "https://api.spotify.com/v1"                    # Spotify Web API base URL
AUTH_URL = "https://accounts.spotify.com/api/token"        # OAuth2 token endpoint
SEARCH_ENDPOINT = "/search"                                # Search endpoint
AUDIO_FEATURES_ENDPOINT = "/audio-features/{id}"           # Audio features by track ID

# OAuth2 token settings
TOKEN_REFRESH_MARGIN = 300                                 # Refresh 5 minutes before expiry


@register_provider
class SpotifyProvider(BaseProvider):
    """Spotify metadata provider using Spotify Web API.

    Provides rich music metadata including:
    - Track/album search with title, artist, album, ISRC
    - Static cover art up to 640x640 JPEG
    - Audio features: energy, danceability, tempo, valence, key, mode
    - Popularity scores and explicit content flags

    Requires Spotify Developer credentials (OAuth2 Client Credentials).
    """

    provider_name = "spotify"                              # Unique provider identifier

    def __init__(self):
        """Initialise the Spotify provider with credentials and HTTP client."""
        super().__init__()
        self._credentials = CredentialManager()            # Credential resolution
        self._rate_limiter = get_rate_limiter("spotify")   # Rate limiter
        self._http_client = None                           # Lazy httpx client
        self._access_token: str | None = None              # Cached OAuth2 access token
        self._token_expiry: float = 0.0                    # Token expiry timestamp

    @property
    def category(self) -> ProviderCategory:
        """Spotify is a music provider."""
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        """Spotify supports track/album search with static cover art and audio features."""
        return ProviderCapabilities(
            can_search_tracks=True,                        # Search individual songs
            can_search_albums=True,                        # Search albums
            can_lookup_isrc=True,                          # ISRC available in responses
            has_static_cover_art=True,                     # JPEG cover art up to 640x640
            has_audio_features=True,                       # Audio features available
        )

    @property
    def requires_auth(self) -> bool:
        """Spotify requires OAuth2 Client Credentials authentication."""
        return True

    def is_available(self) -> bool:
        """Check if Spotify API credentials are available.

        Requires client_id and client_secret from Spotify Developer Dashboard.
        """
        # Check that both client_id and client_secret are present
        client_id = self._credentials.get_credential("spotify", "client_id")
        client_secret = self._credentials.get_credential("spotify", "client_secret")

        return bool(client_id and client_secret)

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search Spotify catalog for matching tracks.

        Constructs a search term from query metadata and calls the
        Spotify Search API. Results include metadata, ISRC codes,
        and cover art URLs.

        Args:
            query: dict with keys: title, artist, album, isrc.

        Returns:
            list[ProviderResult]: Matching results with cover art assets.
        """
        # Build search term from available metadata
        search_parts = []
        if query.get("title"):
            search_parts.append(f'track:"{query["title"]}"')
        if query.get("artist"):
            search_parts.append(f'artist:"{query["artist"]}"')
        if query.get("album"):
            search_parts.append(f'album:"{query["album"]}"')

        if not search_parts:
            logger.warning("Spotify search: no query terms provided")
            return []

        search_term = " ".join(search_parts)

        # Ensure OAuth2 token is valid
        token = await self._get_or_refresh_token()
        if not token:
            logger.error("Spotify: failed to obtain OAuth2 access token")
            return []

        # Make the API request
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = API_BASE + SEARCH_ENDPOINT
            params = {
                "q": search_term,
                "type": "track",                           # Search tracks only
                "limit": 10,                               # Max 10 results
            }
            headers = {
                "Authorization": f"Bearer {token}",
            }

            response = await client.get(url, params=params, headers=headers)
            response.raise_for_status()
            data = response.json()

            return await self._parse_search_results(data)

        except Exception as e:
            logger.error(f"Spotify search failed: {e}")
            return []

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific track by its Spotify ID.

        Args:
            provider_id: Spotify track ID (alphanumeric string).

        Returns:
            ProviderResult if found, None otherwise.
        """
        token = await self._get_or_refresh_token()
        if not token:
            return None

        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = f"{API_BASE}/tracks/{provider_id}"
            headers = {"Authorization": f"Bearer {token}"}

            response = await client.get(url, headers=headers)
            response.raise_for_status()
            data = response.json()

            return await self._parse_track(data)

        except Exception as e:
            logger.error(f"Spotify lookup failed for {provider_id}: {e}")
            return None

    # ========================================================================
    # OAuth2 Client Credentials Flow
    # ========================================================================

    async def _get_or_refresh_token(self) -> str | None:
        """Get a valid OAuth2 access token, requesting a new one if needed.

        OAuth2 Client Credentials tokens are cached and reused until expiry.
        The token is obtained by POSTing client_id and client_secret to the
        Spotify token endpoint with Basic authentication.

        Returns:
            Access token string, or None if authentication failed.
        """
        now = time.time()

        # Return cached token if still valid
        if self._access_token and (now + TOKEN_REFRESH_MARGIN) < self._token_expiry:
            return self._access_token

        # Request new token
        try:
            client_id = self._credentials.get_credential("spotify", "client_id")
            client_secret = self._credentials.get_credential("spotify", "client_secret")

            if not client_id or not client_secret:
                logger.error("Spotify: missing client_id or client_secret")
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
                logger.error("Spotify: no access_token in response")
                return None

            # Cache token
            self._access_token = access_token
            self._token_expiry = now + expires_in
            logger.info("Spotify OAuth2 token obtained successfully")
            return access_token

        except Exception as e:
            logger.error(f"Spotify OAuth2 token request failed: {e}")
            return None

    # ========================================================================
    # Response Parsing
    # ========================================================================

    async def _parse_search_results(self, data: dict) -> list[ProviderResult]:
        """Parse Spotify search API response into ProviderResult list.

        Args:
            data: Raw JSON response from the search endpoint.

        Returns:
            list[ProviderResult]: Parsed results with cover art assets.
        """
        results = []
        tracks = data.get("tracks", {}).get("items", [])

        for track in tracks:
            result = await self._parse_track(track)
            if result:
                results.append(result)

        return results

    async def _parse_track(self, track: dict) -> ProviderResult | None:
        """Parse a single Spotify track object into a ProviderResult.

        Extracts standard metadata, ISRC, audio features, and cover art.

        Args:
            track: A single track object from the Spotify API.

        Returns:
            ProviderResult with metadata and cover art assets.
        """
        try:
            track_id = track.get("id", "")

            # Extract standard metadata
            title = track.get("name", "")
            artists = track.get("artists", [])
            artist = artists[0].get("name", "") if artists else ""
            album_data = track.get("album", {})
            album = album_data.get("name", "")
            release_date = album_data.get("release_date", "")
            year = release_date[:4] if release_date else ""

            # Extract ISRC
            external_ids = track.get("external_ids", {})
            isrc = external_ids.get("isrc", "")

            # Track/disc numbers
            track_num = str(track.get("track_number", ""))
            disc_num = str(track.get("disc_number", ""))

            # Explicit flag and popularity
            explicit = track.get("explicit", False)
            popularity = track.get("popularity", 0)

            # URL
            url = track.get("external_urls", {}).get("spotify", "")

            # Build cover art assets
            cover_art = self._extract_cover_art(album_data)

            # Fetch audio features if available
            audio_features = await self._get_audio_features(track_id)

            # Build extra tags
            extra_tags = {}
            if track_id:
                extra_tags["custom_spotify_id"] = track_id
            if url:
                extra_tags["custom_spotify_url"] = url
            if isrc:
                extra_tags["custom_spotify_isrc"] = isrc
            if popularity:
                extra_tags["custom_spotify_popularity"] = str(popularity)
            if explicit:
                extra_tags["custom_spotify_explicit"] = "true"

            # Add audio features to extra tags
            if audio_features:
                for key, value in audio_features.items():
                    extra_tags[f"custom_spotify_{key}"] = str(value)

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
            logger.error(f"Failed to parse Spotify track: {e}")
            return None

    def _extract_cover_art(self, album_data: dict) -> list[CoverArtAsset]:
        """Extract cover art assets from album data.

        Spotify provides images in multiple resolutions. We take the largest
        (640x640) for best quality.

        Args:
            album_data: Album object from Spotify API.

        Returns:
            list[CoverArtAsset]: Static cover art assets.
        """
        assets = []
        images = album_data.get("images", [])

        # Spotify images are sorted by size, largest first
        if images:
            largest = images[0]
            url = largest.get("url", "")
            width = largest.get("width", 640)
            height = largest.get("height", 640)

            if url:
                assets.append(CoverArtAsset(
                    url=url,
                    asset_type=CoverArtType.STATIC,
                    format="jpeg",
                    width=width,
                    height=height,
                    description="Spotify album cover",
                ))

        return assets

    async def _get_audio_features(self, track_id: str) -> dict | None:
        """Fetch audio features for a track.

        Audio features include energy, danceability, tempo, valence, key, mode.

        Args:
            track_id: Spotify track ID.

        Returns:
            dict with audio feature keys/values, or None if unavailable.
        """
        if not track_id:
            return None

        token = await self._get_or_refresh_token()
        if not token:
            return None

        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = API_BASE + AUDIO_FEATURES_ENDPOINT.format(id=track_id)
            headers = {"Authorization": f"Bearer {token}"}

            response = await client.get(url, headers=headers)
            response.raise_for_status()
            data = response.json()

            # Extract relevant audio features
            features = {}
            if data.get("energy") is not None:
                features["energy"] = round(data["energy"], 3)
            if data.get("danceability") is not None:
                features["danceability"] = round(data["danceability"], 3)
            if data.get("tempo") is not None:
                features["tempo"] = round(data["tempo"], 1)
            if data.get("valence") is not None:
                features["valence"] = round(data["valence"], 3)
            if data.get("key") is not None:
                features["key"] = data["key"]
            if data.get("mode") is not None:
                features["mode"] = data["mode"]

            return features if features else None

        except Exception as e:
            logger.debug(f"Failed to fetch audio features for {track_id}: {e}")
            return None

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
