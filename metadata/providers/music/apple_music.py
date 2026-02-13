# ============================================================================
# File: /metadata/providers/music/apple_music.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Apple Music metadata provider using the Apple Music API (MusicKit).
# Supports track/album search with rich metadata including ISRC codes,
# genre data, and cover art in multiple formats:
#
# - Static cover art: JPEG up to 3000x3000 (from artworkUrl template)
# - Animated square: MP4 from editorialVideo.motionSquareVideo1x1
# - Animated portrait: MP4 from editorialVideo.motionDetailTall
# - Artist spotlight: MP4 from editorialVideo.motionArtistWide16x9
#
# Authentication:
# Uses JWT Developer Tokens signed with ES256 (Elliptic Curve).
# Requires Apple Developer Program membership and a MusicKit private key.
# Tokens are cached for 6 months (Apple's maximum lifetime).
#
# Required credentials (via CredentialManager):
# - APPLE_MUSIC_TEAM_ID: 10-character Apple Developer Team ID
# - APPLE_MUSIC_KEY_ID: MusicKit private key identifier
# - APPLE_MUSIC_PRIVATE_KEY: Path to .p8 file or PEM-encoded key string
# ============================================================================

import time                                                # Token expiry tracking
import logging                                             # Standard logging
from pathlib import Path                                   # Private key file handling

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

logger = logging.getLogger("MeedyaManager.Provider.AppleMusic")

# ============================================================================
# Apple Music API Constants
# ============================================================================
API_BASE = "https://api.music.apple.com"                   # Apple Music API base URL
SEARCH_ENDPOINT = "/v1/catalog/{storefront}/search"        # Catalog search endpoint
SONGS_ENDPOINT = "/v1/catalog/{storefront}/songs/{id}"     # Song lookup by ID
ALBUMS_ENDPOINT = "/v1/catalog/{storefront}/albums/{id}"   # Album lookup by ID

# JWT token lifetime — Apple allows up to 6 months (15,768,000 seconds)
TOKEN_LIFETIME = 15_768_000                                # 6 months in seconds
TOKEN_REFRESH_MARGIN = 3600                                # Refresh 1 hour before expiry


@register_provider
class AppleMusicProvider(BaseProvider):
    """Apple Music metadata provider using MusicKit API.

    Provides rich music metadata including:
    - Track/album search with title, artist, album, ISRC
    - Static cover art up to 3000x3000 JPEG
    - Animated cover art (square, portrait, artist spotlight) as MP4
    - Genre classification and release date data

    Requires Apple Developer Program credentials (JWT ES256 auth).
    """

    provider_name = "apple_music"                          # Unique provider identifier

    def __init__(self):
        """Initialise the Apple Music provider with credentials and HTTP client."""
        super().__init__()
        self._credentials = CredentialManager()            # Credential resolution
        self._rate_limiter = get_rate_limiter("apple_music")  # Rate limiter
        self._http_client = None                           # Lazy httpx client
        self._jwt_token: str | None = None                 # Cached JWT token
        self._jwt_expiry: float = 0.0                      # Token expiry timestamp
        self._storefront = "gb"                            # Default storefront (ISO 3166-1)

        # Load storefront from config if available
        try:
            from utils.config_loader import load_config
            config = load_config() or {}
            providers = config.get("providers", {})
            am_config = providers.get("apple_music", {})
            if am_config.get("storefront"):
                self._storefront = am_config["storefront"]
        except Exception:
            pass                                           # Use default if config unavailable

    @property
    def category(self) -> ProviderCategory:
        """Apple Music is a music provider."""
        return ProviderCategory.MUSIC

    @property
    def capabilities(self) -> ProviderCapabilities:
        """Apple Music supports track/album search with static + animated cover art."""
        return ProviderCapabilities(
            can_search_tracks=True,                        # Search individual songs
            can_search_albums=True,                        # Search albums
            can_search_artists=True,                       # Search artists
            can_lookup_isrc=True,                          # ISRC available in responses
            has_static_cover_art=True,                     # JPEG cover art up to 3000x3000
            has_animated_cover_art=True,                   # Animated square + portrait (MP4)
            has_artist_spotlight=True,                     # Artist spotlight video (MP4)
        )

    @property
    def requires_auth(self) -> bool:
        """Apple Music requires JWT Developer Token (ES256 signed)."""
        return True

    def is_available(self) -> bool:
        """Check if Apple Music API credentials are available.

        Requires team_id, key_id, and private_key to generate JWT tokens.
        Also checks that pyjwt and cryptography are installed for ES256 signing.
        """
        # Check required credentials
        if not self._credentials.has_credentials("apple_music"):
            return False

        # Check that JWT library is available for ES256 signing
        try:
            import jwt                                     # pyjwt for token generation
            return True
        except ImportError:
            logger.warning("pyjwt not installed — Apple Music provider unavailable")
            return False

    async def search(self, query: dict) -> list[ProviderResult]:
        """Search Apple Music catalog for matching tracks.

        Constructs a search term from query metadata and calls the
        Apple Music Search API. Results include metadata, ISRC codes,
        and cover art URLs for all available formats.

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
            logger.warning("Apple Music search: no query terms provided")
            return []

        search_term = " ".join(search_parts)

        # Ensure JWT token is valid
        token = self._get_or_refresh_token()
        if not token:
            logger.error("Apple Music: failed to generate JWT token")
            return []

        # Make the API request
        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = API_BASE + SEARCH_ENDPOINT.format(storefront=self._storefront)
            params = {
                "term": search_term,
                "types": "songs",                          # Search songs only
                "limit": 10,                               # Max 10 results per type
                "include[songs]": "artists,albums",        # Include related data
            }
            headers = {
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
            }

            response = await client.get(url, params=params, headers=headers)
            response.raise_for_status()
            data = response.json()

            return self._parse_search_results(data)

        except Exception as e:
            logger.error(f"Apple Music search failed: {e}")
            return []

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific song by its Apple Music catalog ID.

        Args:
            provider_id: Apple Music catalog ID (numeric string).

        Returns:
            ProviderResult if found, None otherwise.
        """
        token = self._get_or_refresh_token()
        if not token:
            return None

        try:
            client = await self._get_http_client()
            await self._rate_limiter.acquire()

            url = API_BASE + SONGS_ENDPOINT.format(
                storefront=self._storefront, id=provider_id
            )
            headers = {"Authorization": f"Bearer {token}"}

            response = await client.get(url, headers=headers)
            response.raise_for_status()
            data = response.json()

            songs = data.get("data", [])
            if songs:
                return self._parse_song(songs[0])
            return None

        except Exception as e:
            logger.error(f"Apple Music lookup failed for {provider_id}: {e}")
            return None

    # ========================================================================
    # JWT Token Generation
    # ========================================================================

    def _get_or_refresh_token(self) -> str | None:
        """Get a valid JWT token, generating a new one if needed.

        JWT tokens are cached and reused until close to expiry.
        The token is signed with ES256 using the Apple Developer private key.

        Returns:
            JWT token string, or None if generation failed.
        """
        now = time.time()

        # Return cached token if still valid
        if self._jwt_token and (now + TOKEN_REFRESH_MARGIN) < self._jwt_expiry:
            return self._jwt_token

        # Generate new token
        try:
            import jwt                                     # pyjwt for token generation

            # Load credentials
            team_id = self._credentials.get_credential("apple_music", "team_id")
            key_id = self._credentials.get_credential("apple_music", "key_id")
            private_key = self._load_private_key()

            if not all([team_id, key_id, private_key]):
                logger.error("Apple Music: missing JWT credentials")
                return None

            # Build JWT payload
            expiry = now + TOKEN_LIFETIME
            payload = {
                "iss": team_id,                            # Issuer: Team ID
                "iat": int(now),                           # Issued at: current time
                "exp": int(expiry),                        # Expires: 6 months from now
            }
            headers = {
                "alg": "ES256",                            # Algorithm: Elliptic Curve
                "kid": key_id,                             # Key ID from Apple Developer
            }

            # Sign with ES256
            token = jwt.encode(payload, private_key, algorithm="ES256", headers=headers)
            self._jwt_token = token
            self._jwt_expiry = expiry
            logger.info("Apple Music JWT token generated successfully")
            return token

        except ImportError:
            logger.error("pyjwt not installed — cannot generate Apple Music JWT")
            return None
        except Exception as e:
            logger.error(f"Apple Music JWT generation failed: {e}")
            return None

    def _load_private_key(self) -> str | None:
        """Load the ES256 private key from file path or PEM string.

        The private key can be specified as:
        1. A file path to a .p8 file (Apple's download format)
        2. A PEM-encoded key string directly

        Returns:
            PEM-encoded private key string, or None if not found.
        """
        key_value = self._credentials.get_credential("apple_music", "private_key")
        if not key_value:
            return None

        # Check if it's a file path
        key_path = Path(key_value)
        if key_path.exists() and key_path.is_file():
            try:
                key_data = key_path.read_text().strip()
                logger.debug(f"Loaded Apple Music private key from {key_path}")
                return key_data
            except Exception as e:
                logger.error(f"Failed to read private key file {key_path}: {e}")
                return None

        # Treat as PEM string directly
        if "BEGIN" in key_value:
            return key_value

        logger.error("Apple Music private key: not a valid file path or PEM string")
        return None

    # ========================================================================
    # Response Parsing
    # ========================================================================

    def _parse_search_results(self, data: dict) -> list[ProviderResult]:
        """Parse Apple Music search API response into ProviderResult list.

        Args:
            data: Raw JSON response from the search endpoint.

        Returns:
            list[ProviderResult]: Parsed results with cover art assets.
        """
        results = []
        songs = data.get("results", {}).get("songs", {}).get("data", [])

        for song in songs:
            result = self._parse_song(song)
            if result:
                results.append(result)

        return results

    def _parse_song(self, song: dict) -> ProviderResult | None:
        """Parse a single Apple Music song object into a ProviderResult.

        Extracts standard metadata, ISRC, and all available cover art
        formats (static, animated square, animated portrait, artist spotlight).

        Args:
            song: A single song object from the Apple Music API.

        Returns:
            ProviderResult with metadata and cover art assets.
        """
        try:
            attrs = song.get("attributes", {})
            song_id = song.get("id", "")

            # Extract standard metadata
            title = attrs.get("name", "")
            artist = attrs.get("artistName", "")
            album = attrs.get("albumName", "")
            genre = ", ".join(attrs.get("genreNames", []))
            isrc = attrs.get("isrc", "")
            track_num = str(attrs.get("trackNumber", ""))
            disc_num = str(attrs.get("discNumber", ""))
            composer = attrs.get("composerName", "")
            release_date = attrs.get("releaseDate", "")
            year = release_date[:4] if release_date else ""
            url = attrs.get("url", "")

            # Build cover art assets
            cover_art = self._extract_cover_art(attrs)

            # Build extra tags
            extra_tags = {}
            if isrc:
                extra_tags["custom_apple_music_isrc"] = isrc
            content_rating = attrs.get("contentRating", "")
            if content_rating:
                extra_tags["custom_apple_music_content_rating"] = content_rating
            duration_ms = attrs.get("durationInMillis", 0)
            if duration_ms:
                extra_tags["custom_apple_music_duration_ms"] = str(duration_ms)

            return ProviderResult(
                provider_name=self.provider_name,
                title=title,
                artist=artist,
                album=album,
                genre=genre,
                isrc=isrc,
                year=year,
                track_num=track_num,
                disc_num=disc_num,
                composer=composer,
                provider_id=str(song_id),
                provider_url=url,
                cover_art=cover_art,
                extra_tags=extra_tags,
            )

        except Exception as e:
            logger.error(f"Failed to parse Apple Music song: {e}")
            return None

    def _extract_cover_art(self, attrs: dict) -> list[CoverArtAsset]:
        """Extract all cover art assets from song/album attributes.

        Apple Music provides:
        - artwork.url: Template URL with {w}x{h} placeholders for static art
        - editorialVideo.motionSquareVideo1x1: Animated square cover (MP4)
        - editorialVideo.motionDetailTall: Animated portrait cover (MP4)
        - editorialVideo.motionArtistWide16x9: Artist spotlight video (MP4)

        Args:
            attrs: Song or album attributes dict from Apple Music API.

        Returns:
            list[CoverArtAsset]: All available cover art assets.
        """
        assets = []

        # Static cover art (JPEG)
        artwork = attrs.get("artwork", {})
        artwork_url = artwork.get("url", "")
        if artwork_url:
            # Replace {w}x{h} template with maximum resolution
            width = artwork.get("width", 3000)
            height = artwork.get("height", 3000)
            static_url = artwork_url.replace("{w}", str(width)).replace("{h}", str(height))
            assets.append(CoverArtAsset(
                url=static_url,
                asset_type=CoverArtType.STATIC,
                format="jpeg",
                width=width,
                height=height,
                description="Apple Music album artwork",
            ))

        # Animated cover art from editorialVideo
        editorial = attrs.get("editorialVideo", {})

        # Animated square (1:1 aspect ratio)
        square_video = editorial.get("motionSquareVideo1x1", {})
        square_url = square_video.get("video", "")
        if square_url:
            assets.append(CoverArtAsset(
                url=square_url,
                asset_type=CoverArtType.ANIMATED_SQUARE,
                format="mp4",
                description="Apple Music animated square cover",
            ))

        # Animated portrait (tall aspect ratio)
        portrait_video = editorial.get("motionDetailTall", {})
        portrait_url = portrait_video.get("video", "")
        if portrait_url:
            assets.append(CoverArtAsset(
                url=portrait_url,
                asset_type=CoverArtType.ANIMATED_PORTRAIT,
                format="mp4",
                description="Apple Music animated portrait cover",
            ))

        # Artist spotlight (16:9 wide)
        artist_video = editorial.get("motionArtistWide16x9", {})
        artist_url = artist_video.get("video", "")
        if artist_url:
            assets.append(CoverArtAsset(
                url=artist_url,
                asset_type=CoverArtType.ARTIST_SPOTLIGHT,
                format="mp4",
                description="Apple Music artist spotlight video",
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
