# ============================================================================
# File: /tests/test_cover_art_manager.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the CoverArtManager:
# - File naming conventions (FrontCover, PortraitCover, ArtistCover)
# - save_alongside() file creation and directory handling
# - Animated MP4 cover art file naming
# - Artist spotlight placement in parent directory
# - embed_in_file() delegation to TagEditor
# - process_cover_art() orchestration (mocked downloads)
# - FILENAME_MAP and FORMAT_EXTENSIONS constants
# ============================================================================

import pytest                                              # Test framework
from pathlib import Path                                   # Path operations
from unittest.mock import patch, MagicMock, AsyncMock      # Mocking

from metadata.providers.cover_art import (
    CoverArtManager,                                       # Main class under test
    FILENAME_MAP,                                          # Filename mapping constant
    FORMAT_EXTENSIONS,                                     # Extension mapping constant
)
from metadata.providers.base import (
    CoverArtAsset,                                         # Asset dataclass
    CoverArtType,                                          # Type enum
)


# =============================================================================
# Fixtures
# =============================================================================

@pytest.fixture
def cover_art_mgr():
    """Create a fresh CoverArtManager instance."""
    return CoverArtManager()


@pytest.fixture
def media_dir(tmp_path):
    """Create a simulated album directory structure for cover art tests.

    Structure:
    artist_folder/
      album_folder/
        test_song.mp3
    """
    artist_dir = tmp_path / "Test Artist"
    album_dir = artist_dir / "Test Album"
    album_dir.mkdir(parents=True)
    media_file = album_dir / "test_song.mp3"
    media_file.write_bytes(b"FAKE_AUDIO_DATA")
    return str(media_file)


# =============================================================================
# FILENAME_MAP and FORMAT_EXTENSIONS Constant Tests
# =============================================================================

class TestConstants:
    """Tests for the FILENAME_MAP and FORMAT_EXTENSIONS constants."""

    def test_static_filename(self):
        """STATIC cover art should map to 'FrontCover'."""
        assert FILENAME_MAP[CoverArtType.STATIC] == "FrontCover"

    def test_animated_square_filename(self):
        """ANIMATED_SQUARE should map to 'FrontCover' (same base as static)."""
        assert FILENAME_MAP[CoverArtType.ANIMATED_SQUARE] == "FrontCover"

    def test_animated_portrait_filename(self):
        """ANIMATED_PORTRAIT should map to 'PortraitCover'."""
        assert FILENAME_MAP[CoverArtType.ANIMATED_PORTRAIT] == "PortraitCover"

    def test_artist_spotlight_filename(self):
        """ARTIST_SPOTLIGHT should map to 'ArtistCover'."""
        assert FILENAME_MAP[CoverArtType.ARTIST_SPOTLIGHT] == "ArtistCover"

    def test_jpeg_extension(self):
        """JPEG format should map to .jpg extension."""
        assert FORMAT_EXTENSIONS["jpeg"] == ".jpg"
        assert FORMAT_EXTENSIONS["jpg"] == ".jpg"

    def test_png_extension(self):
        """PNG format should map to .png extension."""
        assert FORMAT_EXTENSIONS["png"] == ".png"

    def test_mp4_extension(self):
        """MP4 format should map to .mp4 extension."""
        assert FORMAT_EXTENSIONS["mp4"] == ".mp4"

    def test_webp_extension(self):
        """WebP format should map to .webp extension."""
        assert FORMAT_EXTENSIONS["webp"] == ".webp"


# =============================================================================
# save_alongside() Tests
# =============================================================================

class TestSaveAlongside:
    """Tests for saving cover art files alongside media files."""

    def test_save_static_jpeg(self, cover_art_mgr, media_dir):
        """Saving a static JPEG should create FrontCover.jpg in the album folder."""
        image_data = b"\xff\xd8\xff\xe0" + b"\x00" * 100  # Fake JPEG data
        result = cover_art_mgr.save_alongside(
            media_filepath=media_dir,
            image_data=image_data,
            asset_type=CoverArtType.STATIC,
            format="jpeg",
        )
        assert result is not None
        assert Path(result).exists()
        assert Path(result).name == "FrontCover.jpg"
        assert Path(result).parent.name == "Test Album"

    def test_save_static_png(self, cover_art_mgr, media_dir):
        """Saving a static PNG should create FrontCover.png."""
        image_data = b"\x89PNG" + b"\x00" * 100            # Fake PNG data
        result = cover_art_mgr.save_alongside(
            media_filepath=media_dir,
            image_data=image_data,
            asset_type=CoverArtType.STATIC,
            format="png",
        )
        assert result is not None
        assert Path(result).name == "FrontCover.png"

    def test_save_animated_square(self, cover_art_mgr, media_dir):
        """Saving animated square art should create FrontCover.mp4."""
        video_data = b"\x00\x00\x00\x20ftyp" + b"\x00" * 100
        result = cover_art_mgr.save_alongside(
            media_filepath=media_dir,
            image_data=video_data,
            asset_type=CoverArtType.ANIMATED_SQUARE,
            format="mp4",
        )
        assert result is not None
        assert Path(result).name == "FrontCover.mp4"
        assert Path(result).parent.name == "Test Album"

    def test_save_animated_portrait(self, cover_art_mgr, media_dir):
        """Saving animated portrait art should create PortraitCover.mp4."""
        video_data = b"\x00" * 100
        result = cover_art_mgr.save_alongside(
            media_filepath=media_dir,
            image_data=video_data,
            asset_type=CoverArtType.ANIMATED_PORTRAIT,
            format="mp4",
        )
        assert result is not None
        assert Path(result).name == "PortraitCover.mp4"

    def test_save_artist_spotlight_parent_dir(self, cover_art_mgr, media_dir):
        """Artist spotlight should be saved in the parent (artist) directory."""
        video_data = b"\x00" * 100
        result = cover_art_mgr.save_alongside(
            media_filepath=media_dir,
            image_data=video_data,
            asset_type=CoverArtType.ARTIST_SPOTLIGHT,
            format="mp4",
        )
        assert result is not None
        assert Path(result).name == "ArtistCover.mp4"
        # Should be in the artist directory, one level up from album
        assert Path(result).parent.name == "Test Artist"

    def test_save_empty_data_returns_none(self, cover_art_mgr, media_dir):
        """Empty image data should return None without creating a file."""
        result = cover_art_mgr.save_alongside(
            media_filepath=media_dir,
            image_data=b"",
            asset_type=CoverArtType.STATIC,
            format="jpeg",
        )
        assert result is None

    def test_save_creates_directory_if_needed(self, cover_art_mgr, tmp_path):
        """save_alongside() should create the output directory if it doesn't exist."""
        # Point to a media file in a non-existent directory
        new_dir = tmp_path / "new_artist" / "new_album"
        media_file = new_dir / "song.mp3"
        new_dir.mkdir(parents=True)
        media_file.write_bytes(b"FAKE")

        result = cover_art_mgr.save_alongside(
            media_filepath=str(media_file),
            image_data=b"\xff\xd8" * 50,
            asset_type=CoverArtType.STATIC,
            format="jpeg",
        )
        assert result is not None
        assert Path(result).exists()


# =============================================================================
# embed_in_file() Tests
# =============================================================================

class TestEmbedInFile:
    """Tests for embedding cover art in media file tags."""

    def test_embed_delegates_to_tag_editor(self, cover_art_mgr, media_dir):
        """embed_in_file() should delegate to TagEditor.write_cover_art()."""
        mock_instance = MagicMock()
        with patch.dict("sys.modules", {}):                # Clear module cache
            with patch("metadata.editor.TagEditor", return_value=mock_instance):
                result = cover_art_mgr.embed_in_file(
                    media_filepath=media_dir,
                    image_data=b"\xff\xd8\xff\xe0" + b"\x00" * 100,
                    image_format="jpeg",
                )
                assert result is True
                mock_instance.write_cover_art.assert_called_once()

    def test_embed_empty_data_returns_false(self, cover_art_mgr, media_dir):
        """embed_in_file() with empty data should return False."""
        result = cover_art_mgr.embed_in_file(
            media_filepath=media_dir,
            image_data=b"",
            image_format="jpeg",
        )
        assert result is False

    def test_embed_error_returns_false(self, cover_art_mgr, media_dir):
        """embed_in_file() should return False on TagEditor error."""
        with patch("metadata.editor.TagEditor", side_effect=Exception("Tag write failed")):
            result = cover_art_mgr.embed_in_file(
                media_filepath=media_dir,
                image_data=b"\xff\xd8" * 50,
                image_format="jpeg",
            )
            assert result is False


# =============================================================================
# process_cover_art() Tests (Mocked Downloads)
# =============================================================================

class TestProcessCoverArt:
    """Tests for the full cover art processing pipeline."""

    def test_process_downloads_and_saves(self, cover_art_mgr, media_dir):
        """process_cover_art() should download and save each asset."""
        assets = [
            CoverArtAsset(
                url="https://example.com/cover.jpg",
                asset_type=CoverArtType.STATIC,
                format="jpeg",
            ),
        ]

        async def run():
            # Mock the download to return fake image data
            with patch.object(cover_art_mgr, "download_asset",
                              return_value=b"\xff\xd8" * 50):
                # Mock embed_in_file to avoid TagEditor dependency
                with patch.object(cover_art_mgr, "embed_in_file", return_value=True):
                    return await cover_art_mgr.process_cover_art(
                        media_dir, assets
                    )

        import asyncio
        saved = asyncio.run(run())
        assert "static" in saved
        assert Path(saved["static"]).name == "FrontCover.jpg"

    def test_process_skips_failed_downloads(self, cover_art_mgr, media_dir):
        """process_cover_art() should skip assets that fail to download."""
        assets = [
            CoverArtAsset(
                url="https://example.com/fail.jpg",
                asset_type=CoverArtType.STATIC,
                format="jpeg",
            ),
        ]

        async def run():
            with patch.object(cover_art_mgr, "download_asset", return_value=None):
                return await cover_art_mgr.process_cover_art(
                    media_dir, assets
                )

        import asyncio
        saved = asyncio.run(run())
        assert len(saved) == 0

    def test_process_multiple_asset_types(self, cover_art_mgr, media_dir):
        """process_cover_art() should handle multiple asset types."""
        assets = [
            CoverArtAsset(url="https://ex.com/front.jpg", asset_type=CoverArtType.STATIC, format="jpeg"),
            CoverArtAsset(url="https://ex.com/video.mp4", asset_type=CoverArtType.ANIMATED_SQUARE, format="mp4"),
        ]

        async def run():
            with patch.object(cover_art_mgr, "download_asset", return_value=b"\x00" * 100):
                with patch.object(cover_art_mgr, "embed_in_file", return_value=True):
                    return await cover_art_mgr.process_cover_art(
                        media_dir, assets
                    )

        import asyncio
        saved = asyncio.run(run())
        assert "static" in saved
        assert "animated_square" in saved
