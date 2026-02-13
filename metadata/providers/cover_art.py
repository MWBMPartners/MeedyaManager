# ============================================================================
# File: /metadata/providers/cover_art.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Cover art management system for metadata lookup providers.
# Handles downloading, saving, and embedding cover art from provider
# search results. Supports both static images (JPEG/PNG) and animated
# video artwork (MP4).
#
# File naming convention:
# - FrontCover.jpg / FrontCover.png — Static album cover (same dir as media)
# - FrontCover.mp4 — Animated square cover art (same dir as media)
# - PortraitCover.mp4 — Animated portrait/tall cover art (same dir as media)
# - ArtistCover.mp4 — Artist spotlight video (parent/artist directory)
#
# Static cover art is also embedded in the media file's tags using
# TagEditor.write_cover_art() (APIC for ID3, covr for MP4, Picture for FLAC).
# ============================================================================

import logging                                      # Standard logging
from pathlib import Path                            # Cross-platform path handling

from metadata.providers.base import CoverArtAsset, CoverArtType

logger = logging.getLogger("MeedyaManager.CoverArtManager")


# ============================================================================
# FILENAME_MAP — Maps CoverArtType to output filename (without extension).
# The extension comes from the asset's format field.
# ============================================================================
FILENAME_MAP = {
    CoverArtType.STATIC: "FrontCover",              # Static album cover image
    CoverArtType.ANIMATED_SQUARE: "FrontCover",      # Animated square (MP4)
    CoverArtType.ANIMATED_PORTRAIT: "PortraitCover", # Animated portrait (MP4)
    CoverArtType.ARTIST_SPOTLIGHT: "ArtistCover",    # Artist spotlight video (MP4)
}

# ============================================================================
# FORMAT_EXTENSIONS — Maps format identifiers to file extensions.
# ============================================================================
FORMAT_EXTENSIONS = {
    "jpeg": ".jpg",
    "jpg": ".jpg",
    "png": ".png",
    "mp4": ".mp4",
    "webp": ".webp",
}


class CoverArtManager:
    """Downloads, saves, and embeds cover art from provider results.

    Manages the full lifecycle of cover art assets:
    1. Download from provider URL (via httpx)
    2. Save as a file alongside the media file (FrontCover.jpg, etc.)
    3. Embed static art into the media file's tags (via TagEditor)

    Animated cover art (MP4) is only saved as files — it cannot be
    embedded in audio file tags.
    """

    def __init__(self):
        """Initialise the cover art manager."""
        self._http_client = None                    # Lazy httpx client instance

    async def download_asset(self, asset: CoverArtAsset) -> bytes | None:
        """Download cover art data from a URL.

        Uses httpx for async HTTP download with timeout and error handling.

        Args:
            asset: CoverArtAsset with the download URL.

        Returns:
            Raw bytes of the downloaded image/video, or None on error.
        """
        if not asset.url:
            logger.warning("Cannot download asset: empty URL")
            return None

        try:
            import httpx                            # Lazy import — async HTTP client

            # Create or reuse HTTP client
            if self._http_client is None:
                self._http_client = httpx.AsyncClient(
                    timeout=60.0,                   # 60-second timeout for large files
                    follow_redirects=True,          # Follow HTTP redirects
                )

            logger.debug(f"Downloading cover art: {asset.url[:80]}...")
            response = await self._http_client.get(asset.url)
            response.raise_for_status()

            data = response.content
            logger.debug(f"Downloaded {len(data)} bytes ({asset.asset_type.value})")
            return data

        except ImportError:
            logger.warning("httpx not installed — cannot download cover art")
            return None
        except Exception as e:
            logger.error(f"Failed to download cover art: {e}")
            return None

    def save_alongside(self, media_filepath: str, image_data: bytes,
                       asset_type: CoverArtType,
                       format: str = "jpeg") -> str | None:
        """Save cover art as a file alongside the media file.

        Creates the output file in the same directory as the media file,
        except for ARTIST_SPOTLIGHT which goes in the parent (artist) directory.

        Args:
            media_filepath: Path to the media file.
            image_data: Raw bytes of the image/video data.
            asset_type: Type of cover art (determines filename).
            format: Image/video format ("jpeg", "png", "mp4").

        Returns:
            Path to the saved file as a string, or None on error.
        """
        if not image_data:
            logger.warning("Cannot save cover art: empty data")
            return None

        try:
            media_path = Path(media_filepath)
            base_name = FILENAME_MAP.get(asset_type, "CoverArt")
            extension = FORMAT_EXTENSIONS.get(format, f".{format}")

            # Determine output directory
            if asset_type == CoverArtType.ARTIST_SPOTLIGHT:
                # Artist spotlight goes in the parent (artist) directory
                output_dir = media_path.parent.parent
            else:
                # All other types go in the same directory as the media file
                output_dir = media_path.parent

            # Ensure the output directory exists
            output_dir.mkdir(parents=True, exist_ok=True)

            # Build the full output path
            output_path = output_dir / f"{base_name}{extension}"

            # Write the file
            output_path.write_bytes(image_data)
            logger.info(f"Saved cover art: {output_path}")
            return str(output_path)

        except Exception as e:
            logger.error(f"Failed to save cover art alongside {media_filepath}: {e}")
            return None

    def embed_in_file(self, media_filepath: str, image_data: bytes,
                      image_format: str = "jpeg") -> bool:
        """Embed static cover art into the media file's tags.

        Delegates to TagEditor.write_cover_art() which handles format-specific
        embedding (APIC frame for ID3, covr atom for MP4, Picture for FLAC).

        Only static images (JPEG/PNG) can be embedded. Animated MP4 cover art
        cannot be embedded in audio file tags.

        Args:
            media_filepath: Path to the media file.
            image_data: Raw bytes of the JPEG/PNG image.
            image_format: Image format ("jpeg" or "png").

        Returns:
            True if embedded successfully, False on error.
        """
        if not image_data:
            logger.warning("Cannot embed cover art: empty data")
            return False

        try:
            from metadata.editor import TagEditor   # Lazy import to avoid circular deps

            editor = TagEditor()
            editor.write_cover_art(
                media_filepath,
                image_data=image_data,
                image_format=image_format,
                picture_type=3,                     # 3 = Front Cover (APIC standard)
            )
            logger.info(f"Embedded cover art in: {media_filepath}")
            return True

        except Exception as e:
            logger.error(f"Failed to embed cover art in {media_filepath}: {e}")
            return False

    async def process_cover_art(self, media_filepath: str,
                                 assets: list[CoverArtAsset],
                                 embed_static: bool = True) -> dict[str, str]:
        """Download all cover art assets, save alongside, and embed static art.

        Orchestrates the full cover art workflow:
        1. Download each asset from its URL
        2. Save as a file alongside the media file
        3. Embed static images in the file's tags (if embed_static=True)

        Args:
            media_filepath: Path to the media file.
            assets: List of CoverArtAsset objects from provider results.
            embed_static: Whether to embed static art in file tags.

        Returns:
            dict: {asset_type_name: saved_path} for each successfully saved asset.
        """
        saved_paths: dict[str, str] = {}

        for asset in assets:
            # Download the asset data
            data = await self.download_asset(asset)
            if data is None:
                logger.warning(f"Skipping {asset.asset_type.value}: download failed")
                continue

            # Save alongside the media file
            saved_path = self.save_alongside(
                media_filepath=media_filepath,
                image_data=data,
                asset_type=asset.asset_type,
                format=asset.format,
            )

            if saved_path:
                saved_paths[asset.asset_type.value] = saved_path

            # Embed static images in the file's tags
            if (embed_static
                    and asset.asset_type == CoverArtType.STATIC
                    and asset.format in ("jpeg", "jpg", "png")):
                self.embed_in_file(media_filepath, data, asset.format)

        logger.info(
            f"Processed {len(saved_paths)}/{len(assets)} cover art assets "
            f"for {Path(media_filepath).name}"
        )
        return saved_paths

    async def close(self) -> None:
        """Close the HTTP client connection (cleanup)."""
        if self._http_client is not None:
            await self._http_client.aclose()
            self._http_client = None
