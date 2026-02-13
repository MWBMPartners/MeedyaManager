# ============================================================================
# File: /metadata/providers/base.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Abstract base class and shared data models for all metadata lookup
# providers. Every provider (Spotify, Apple Music, TMDB, etc.) inherits
# from BaseProvider and returns ProviderResult instances.
#
# Data models:
# - ProviderCapabilities: Declares what a provider can do (search tracks,
#   albums, fingerprint audio, provide animated cover art, etc.)
# - CoverArtAsset: Represents a downloadable cover art resource with URL,
#   type (static/animated), format, and dimensions
# - ProviderResult: A single match result containing metadata fields,
#   provider-specific IDs/URLs, cover art assets, and confidence score
#
# The BaseProvider ABC defines the contract all providers must implement:
# - name, category, capabilities (properties)
# - is_available() — check if provider has valid credentials
# - search(query) — search the provider's catalog
# - lookup_by_id(id) — look up a specific item by provider ID
# ============================================================================

from abc import ABC, abstractmethod                 # Abstract base class support
from dataclasses import dataclass, field            # Structured data containers
from enum import Enum                               # Enum for asset types
import logging                                      # Standard logging

# Import ProviderCategory from the registry module
from metadata.providers import ProviderCategory


# ============================================================================
# Logger — Used by provider implementations for debug/error logging
# ============================================================================
logger = logging.getLogger("MeedyaManager.Provider")


# ============================================================================
# CoverArtType — Classifies different types of cover art assets.
# Used by CoverArtManager to determine file naming and placement.
# ============================================================================
class CoverArtType(Enum):
    """Types of cover art assets that providers can return."""
    STATIC = "static"                               # Standard still image (JPEG/PNG)
    ANIMATED_SQUARE = "animated_square"              # Animated square video (MP4)
    ANIMATED_PORTRAIT = "animated_portrait"          # Animated portrait/tall video (MP4)
    ARTIST_SPOTLIGHT = "artist_spotlight"            # Artist spotlight video (MP4, 16:9)


# ============================================================================
# ProviderCapabilities — Declares what a provider supports.
# Each provider sets these flags to indicate which operations are available.
# Used by the LookupService to filter providers for specific query types.
# ============================================================================
@dataclass
class ProviderCapabilities:
    """Declares the capabilities of a metadata provider.

    Each boolean flag indicates whether the provider supports a specific
    type of search or data retrieval. The LookupService uses these flags
    to determine which providers to query for a given media type.
    """

    # Search capabilities — what types of content can be searched
    can_search_tracks: bool = False                 # Can search individual songs/recordings
    can_search_albums: bool = False                 # Can search albums/releases
    can_search_artists: bool = False                # Can search artists
    can_search_episodes: bool = False               # Can search TV episodes
    can_search_shows: bool = False                  # Can search TV series
    can_search_movies: bool = False                 # Can search movies/films
    can_search_podcasts: bool = False               # Can search podcast shows/episodes

    # Identifier capabilities — lookup by standard identifiers
    can_lookup_isrc: bool = False                   # Can look up by ISRC code
    can_lookup_upc: bool = False                    # Can look up by UPC/GTIN barcode
    can_lookup_imdb_id: bool = False                # Can look up by IMDb ID (tt1234567)
    can_fingerprint_audio: bool = False             # Can identify tracks by audio fingerprint

    # Cover art capabilities — what types of artwork are available
    has_static_cover_art: bool = False              # Can provide still images (JPEG/PNG)
    has_animated_cover_art: bool = False             # Can provide animated cover videos (MP4)
    has_artist_spotlight: bool = False               # Can provide artist spotlight videos

    # Additional data capabilities
    has_audio_features: bool = False                 # Can provide audio analysis (BPM, energy, etc.)
    has_lyrics: bool = False                         # Can provide lyrics text
    has_cast_crew: bool = False                      # Can provide cast/crew information


# ============================================================================
# CoverArtAsset — Represents a single downloadable cover art resource.
# Providers populate these from API responses; the CoverArtManager
# handles downloading and saving them.
# ============================================================================
@dataclass
class CoverArtAsset:
    """A downloadable cover art resource from a provider.

    Each asset has a URL, type classification, format, and optional
    dimension information. The CoverArtManager uses the asset_type to
    determine the output filename and placement.

    Asset type → filename mapping:
    - STATIC → FrontCover.jpg (or .png)
    - ANIMATED_SQUARE → FrontCover.mp4
    - ANIMATED_PORTRAIT → PortraitCover.mp4
    - ARTIST_SPOTLIGHT → ArtistCover.mp4 (in parent/artist directory)
    """
    url: str                                        # Download URL for the cover art
    asset_type: CoverArtType                        # Type classification for naming/placement
    format: str = "jpeg"                            # Image/video format: "jpeg", "png", "mp4"
    width: int = 0                                  # Width in pixels (0 if unknown)
    height: int = 0                                 # Height in pixels (0 if unknown)
    description: str = ""                           # Optional description (e.g., "Front Cover")


# ============================================================================
# ProviderResult — A single match result from a provider search.
#
# Contains standard metadata fields (title, artist, album, etc.),
# provider-specific identification (ID, URL), cover art assets, and
# a confidence score indicating how well this result matches the query.
#
# The extra_tags dict holds provider-specific metadata that doesn't map
# to standard fields (e.g., Spotify audio features, Tidal quality tier).
# These are written to the file as custom tags.
# ============================================================================
@dataclass
class ProviderResult:
    """A single match result from a metadata provider search.

    Standard metadata fields map to TAG_MAP entries. Provider-specific
    data goes in extra_tags, which are stored as Custom: tags in files.
    """

    # Provider identification
    provider_name: str                              # Name of the provider (e.g., "spotify")
    confidence: float = 0.0                         # Match confidence score (0.0-1.0)

    # Standard metadata fields (map to TAG_MAP entries)
    title: str = ""                                 # Track/episode/movie title
    artist: str = ""                                # Artist name(s)
    album: str = ""                                 # Album name
    album_artist: str = ""                          # Album-level artist
    year: str = ""                                  # Release year
    genre: str = ""                                 # Genre(s)
    track_num: str = ""                             # Track number
    total_tracks: str = ""                          # Total tracks on album
    disc_num: str = ""                              # Disc number
    total_discs: str = ""                           # Total discs
    composer: str = ""                              # Composer
    isrc: str = ""                                  # ISRC code
    bpm: str = ""                                   # Beats per minute
    lyrics: str = ""                                # Lyrics text

    # Video-specific fields
    show: str = ""                                  # TV show name
    season: str = ""                                # Season number
    episode: str = ""                               # Episode number
    episode_title: str = ""                         # Episode title
    director: str = ""                              # Director name

    # Provider-specific identification
    provider_id: str = ""                           # Provider's unique ID for this item
    provider_url: str = ""                          # Direct URL to the item on the provider

    # Cover art assets available for download
    cover_art: list[CoverArtAsset] = field(default_factory=list)

    # Provider-specific extra metadata (stored as Custom: tags)
    # Keys should use the format "custom_provider_fieldname"
    # e.g., {"custom_spotify_energy": "0.85", "custom_spotify_popularity": "72"}
    extra_tags: dict[str, str] = field(default_factory=dict)

    def get_standard_tags(self) -> dict[str, str]:
        """Extract standard metadata fields as a dict for tag writing.

        Returns only non-empty fields that map to TAG_MAP entries.
        Used by LookupService.apply_result() to write tags via TagEditor.

        Returns:
            dict: {internal_key: value} for non-empty standard fields.
        """
        tags = {}
        # Map dataclass fields to TAG_MAP internal keys
        field_mapping = {
            "title": "title",
            "artist": "artist",
            "album": "album",
            "album_artist": "album_artist",
            "year": "year",
            "genre": "genre",
            "track_num": "track_num",
            "total_tracks": "total_tracks",
            "disc_num": "disc_num",
            "total_discs": "total_discs",
            "composer": "composer",
            "isrc": "isrc",
            "bpm": "bpm",
            "lyrics": "lyrics",
            "show": "show",
            "season": "season",
            "episode": "episode",
            "episode_title": "episode_title",
            "director": "director",
        }
        for attr_name, internal_key in field_mapping.items():
            value = getattr(self, attr_name, "")
            if value:                               # Only include non-empty values
                tags[internal_key] = str(value)
        return tags

    def get_custom_tags(self) -> dict[str, str]:
        """Get provider-specific custom tags for writing to the file.

        Includes the provider ID and URL as custom tags, plus any
        extra_tags from the provider's API response.

        Returns:
            dict: {custom_key: value} for all provider-specific tags.
        """
        tags = {}
        # Add provider ID and URL as custom tags
        if self.provider_id:
            tags[f"custom_{self.provider_name}_id"] = self.provider_id
        if self.provider_url:
            tags[f"custom_{self.provider_name}_url"] = self.provider_url
        # Add all extra tags from the provider
        tags.update(self.extra_tags)
        return tags

    def get_all_tags(self) -> dict[str, str]:
        """Get all tags (standard + custom) for writing to the file.

        Returns:
            dict: Combined {internal_key: value} dict of all writable tags.
        """
        all_tags = self.get_standard_tags()
        all_tags.update(self.get_custom_tags())
        return all_tags


# ============================================================================
# BaseProvider — Abstract base class that all providers must implement.
#
# Providers inherit from this and implement:
# - provider_name (class attribute): unique identifier string
# - category (property): ProviderCategory enum value
# - capabilities (property): ProviderCapabilities instance
# - requires_auth (property): whether credentials are needed
# - is_available(): check if provider is operational
# - search(query): perform a metadata search
# - lookup_by_id(provider_id): look up a specific item
# ============================================================================
class BaseProvider(ABC):
    """Abstract base class for all metadata lookup providers.

    Each provider wraps a specific API (Spotify, TMDB, MusicBrainz, etc.)
    and normalises its responses into ProviderResult instances that can be
    ranked, compared, and applied to media files.

    Subclasses must set the `provider_name` class attribute and implement
    all abstract methods. Providers are registered with the global
    PROVIDER_REGISTRY via the @register_provider decorator.
    """

    # Class-level attribute that subclasses MUST set to their unique name
    provider_name: str = ""                         # e.g., "spotify", "apple_music", "tmdb"

    def __init__(self):
        """Initialise the provider with a logger instance."""
        self.logger = logging.getLogger(
            f"MeedyaManager.Provider.{self.__class__.__name__}"
        )

    @property
    @abstractmethod
    def category(self) -> ProviderCategory:
        """The content category this provider belongs to.

        Returns:
            ProviderCategory: MUSIC, VIDEO, PODCAST, or IDENTIFIER.
        """
        ...

    @property
    @abstractmethod
    def capabilities(self) -> ProviderCapabilities:
        """Declare what this provider can do.

        Returns:
            ProviderCapabilities: Flags indicating supported operations.
        """
        ...

    @property
    @abstractmethod
    def requires_auth(self) -> bool:
        """Whether this provider requires API credentials to function.

        Returns:
            bool: True if credentials are needed, False for public APIs.
        """
        ...

    @abstractmethod
    def is_available(self) -> bool:
        """Check if this provider is operational.

        Should verify that:
        1. Required credentials are present (if requires_auth is True)
        2. Required libraries are installed (for optional dependencies)
        3. The API endpoint is reachable (optional, avoid slow network checks)

        Returns:
            bool: True if the provider can be used for searches.
        """
        ...

    @abstractmethod
    async def search(self, query: dict) -> list[ProviderResult]:
        """Search the provider's catalog for matching items.

        The query dict contains metadata fields from the file being looked up:
        - title: Track/movie/episode title
        - artist: Artist name (for music)
        - album: Album name (for music)
        - isrc: ISRC code (for music, if available)
        - year: Release year
        - show: TV show name (for TV episodes)
        - season: Season number (for TV episodes)
        - episode: Episode number (for TV episodes)
        - media_class: "Music", "Movie", "TV Show", etc.

        Args:
            query: dict of metadata fields to search by.

        Returns:
            list[ProviderResult]: Matching results (may be empty).
        """
        ...

    async def lookup_by_id(self, provider_id: str) -> ProviderResult | None:
        """Look up a specific item by its provider-specific ID.

        Optional override — not all providers support direct ID lookup.
        Default implementation returns None.

        Args:
            provider_id: The provider's unique identifier for the item.

        Returns:
            ProviderResult if found, None otherwise.
        """
        return None

    def get_status_info(self) -> dict:
        """Get a status summary dict for this provider.

        Used by CLI --providers-list and GUI provider status display.

        Returns:
            dict with keys: name, category, requires_auth, available, message
        """
        available = False
        message = "Unknown status"
        try:
            available = self.is_available()
            if available:
                message = "Available"
            elif self.requires_auth:
                message = "Missing credentials"
            else:
                message = "Unavailable"
        except Exception as e:
            message = f"Error: {e}"

        return {
            "name": self.provider_name,
            "category": self.category.value,
            "requires_auth": self.requires_auth,
            "available": available,
            "message": message,
        }
