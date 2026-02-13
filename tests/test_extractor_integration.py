# ============================================================================
# File: /tests/test_extractor_integration.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Integration tests for the enhanced metadata extractor (Phase 2).
# Verifies that extract_metadata() returns mutagen-read tag fields
# (artist, album, genre, etc.) alongside pymediainfo technical metadata.
#
# Also tests the tag_registry additions (TECHNICAL_TAGS, is_editable_tag).
# ============================================================================

import pytest                                          # Test framework

from core.tag_registry import (
    TECHNICAL_TAGS,
    is_editable_tag,
    TAG_MAP,
    REVERSE_TAG_MAP,
)


# =============================================================================
# Tag Registry — TECHNICAL_TAGS and is_editable_tag() Tests
# =============================================================================

class TestTechnicalTags:
    """Tests for the TECHNICAL_TAGS set and is_editable_tag() function."""

    def test_technical_tags_contains_filepath(self):
        """filepath is a filesystem field, not an embedded tag."""
        assert "filepath" in TECHNICAL_TAGS

    def test_technical_tags_contains_format(self):
        """format is a technical stream property."""
        assert "format" in TECHNICAL_TAGS

    def test_technical_tags_contains_duration(self):
        """duration is a technical stream property."""
        assert "duration" in TECHNICAL_TAGS

    def test_technical_tags_contains_audio_channels(self):
        """audio_channels is a technical stream property."""
        assert "audio_channels" in TECHNICAL_TAGS

    def test_technical_tags_contains_codec(self):
        """codec is a technical stream property."""
        assert "codec" in TECHNICAL_TAGS

    def test_technical_tags_contains_classification(self):
        """Classification fields are computed, not embedded tags."""
        for key in ("media_group", "format_class", "media_class", "quality_type"):
            assert key in TECHNICAL_TAGS, f"{key} should be in TECHNICAL_TAGS"

    def test_editable_artist(self):
        """artist is an embedded tag, should be editable."""
        assert is_editable_tag("artist") is True

    def test_editable_album(self):
        """album is an embedded tag, should be editable."""
        assert is_editable_tag("album") is True

    def test_editable_genre(self):
        """genre is an embedded tag, should be editable."""
        assert is_editable_tag("genre") is True

    def test_editable_title(self):
        """title is an embedded tag, should be editable."""
        assert is_editable_tag("title") is True

    def test_editable_custom_tag(self):
        """Custom tags should always be editable."""
        assert is_editable_tag("custom_spotify_url") is True

    def test_not_editable_codec(self):
        """codec is technical, should NOT be editable."""
        assert is_editable_tag("codec") is False

    def test_not_editable_filepath(self):
        """filepath is filesystem, should NOT be editable."""
        assert is_editable_tag("filepath") is False

    def test_not_editable_media_group(self):
        """media_group is computed, should NOT be editable."""
        assert is_editable_tag("media_group") is False


class TestTagRegistryNewTags:
    """Tests for the new ISRC and Lyrics tags added to TAG_MAP."""

    def test_isrc_in_tag_map(self):
        """ISRC should be in TAG_MAP."""
        assert "ISRC" in TAG_MAP
        assert TAG_MAP["ISRC"] == "isrc"

    def test_lyrics_in_tag_map(self):
        """Lyrics should be in TAG_MAP."""
        assert "Lyrics" in TAG_MAP
        assert TAG_MAP["Lyrics"] == "lyrics"

    def test_isrc_in_reverse_map(self):
        """isrc should resolve back to ISRC display name."""
        assert REVERSE_TAG_MAP.get("isrc") == "ISRC"

    def test_lyrics_in_reverse_map(self):
        """lyrics should resolve back to Lyrics display name."""
        assert REVERSE_TAG_MAP.get("lyrics") == "Lyrics"

    def test_isrc_is_editable(self):
        """ISRC is an embedded tag, should be editable."""
        assert is_editable_tag("isrc") is True

    def test_lyrics_is_editable(self):
        """Lyrics is an embedded tag, should be editable."""
        assert is_editable_tag("lyrics") is True


# =============================================================================
# Metadata Extractor Integration — Tag Enrichment Tests
# =============================================================================

class TestExtractorTagEnrichment:
    """
    Integration tests verifying that extract_metadata() returns mutagen-read
    tag fields when given real media files with embedded tags.

    These tests use the real_mp3_file and real_flac_file fixtures from
    conftest.py, which create actual valid media files with mutagen.
    """

    def test_mp3_has_artist_tag(self, real_mp3_file):
        """extract_metadata should include artist from ID3v2 tags."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_mp3_file)
        assert meta.get("artist") == "Integration Artist"

    def test_mp3_has_album_tag(self, real_mp3_file):
        """extract_metadata should include album from ID3v2 tags."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_mp3_file)
        assert meta.get("album") == "Integration Album"

    def test_mp3_has_genre_tag(self, real_mp3_file):
        """extract_metadata should include genre from ID3v2 tags."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_mp3_file)
        assert meta.get("genre") == "Electronic"

    def test_mp3_has_year_tag(self, real_mp3_file):
        """extract_metadata should include year from ID3v2 tags."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_mp3_file)
        assert meta.get("year") == "2026"

    def test_mp3_has_track_num(self, real_mp3_file):
        """extract_metadata should include track number from ID3v2 tags."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_mp3_file)
        assert meta.get("track_num") == "7"

    def test_mp3_has_total_tracks(self, real_mp3_file):
        """extract_metadata should include total tracks from ID3v2 TRCK "7/14"."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_mp3_file)
        assert meta.get("total_tracks") == "14"

    def test_mp3_has_composer(self, real_mp3_file):
        """extract_metadata should include composer from ID3v2 tags."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_mp3_file)
        assert meta.get("composer") == "Integration Composer"

    def test_mp3_title_from_mutagen(self, real_mp3_file):
        """Title should come from mutagen's ID3v2 TIT2 frame."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_mp3_file)
        assert meta.get("title") == "Integration Test Song"

    def test_mp3_still_has_technical_metadata(self, real_mp3_file):
        """Technical fields from pymediainfo should still be present."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_mp3_file)
        assert "extension" in meta
        assert meta["extension"] == "mp3"
        assert "filepath" in meta

    def test_mp3_still_has_classification(self, real_mp3_file):
        """Classification fields should still be populated."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_mp3_file)
        assert "media_group" in meta
        assert "format_class" in meta
        assert "media_class" in meta
        assert "quality_type" in meta

    def test_flac_has_artist_tag(self, real_flac_file):
        """extract_metadata should include artist from Vorbis Comments."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_flac_file)
        assert meta.get("artist") == "FLAC Integration Artist"

    def test_flac_has_album_tag(self, real_flac_file):
        """extract_metadata should include album from Vorbis Comments."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_flac_file)
        assert meta.get("album") == "FLAC Integration Album"

    def test_flac_has_genre_tag(self, real_flac_file):
        """extract_metadata should include genre from Vorbis Comments."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_flac_file)
        assert meta.get("genre") == "Classical"

    def test_flac_title_from_mutagen(self, real_flac_file):
        """Title should come from mutagen's Vorbis Comment TITLE field."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_flac_file)
        assert meta.get("title") == "FLAC Integration Song"

    def test_flac_has_composer(self, real_flac_file):
        """extract_metadata should include composer from Vorbis Comments."""
        from core.metadata_extractor import extract_metadata
        meta = extract_metadata(real_flac_file)
        assert meta.get("composer") == "FLAC Composer"
