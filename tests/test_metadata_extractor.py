# ============================================================================
# File: /tests/test_metadata_extractor.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Test cases for the metadata extraction and classification logic in MeedyaManager.
# These tests validate that files are correctly parsed and assigned the proper
# media_group, format_class, media_class, and quality_type values based on
# their metadata.
# ============================================================================

import pytest
from core.classify_media import classify_media

# Test samples with known metadata scenarios

@pytest.mark.parametrize("metadata, expected", [
    ({"extension": "flac", "format": "FLAC", "is_lossless": True, "audio_channels": 2, "title": "Test Album"},
     {"media_group": "Audio", "format_class": "flac", "media_class": "Music", "quality_type": "Lossless"}),

    ({"extension": "mp3", "format": "MP3", "is_lossless": False, "audio_channels": 2, "title": "My Podcast Episode"},
     {"media_group": "Audio", "format_class": "mp3", "media_class": "Podcast", "quality_type": "Lossy"}),

    ({"extension": "mkv", "format": "Matroska", "duration": 3600, "title": "My Movie Episode"},
     {"media_group": "Video", "format_class": "matroska", "media_class": "Movie", "quality_type": "Lossy"}),

    ({"extension": "mp4", "format": "mp4", "duration": 120, "title": "Short Music Video"},
     {"media_group": "Video", "format_class": "mp4", "media_class": "Music Video", "quality_type": "Lossy"}),

    ({"extension": "jpg", "format": "JPEG", "title": "Cover Art Image"},
     {"media_group": "Image", "format_class": "jpeg", "media_class": "Photo", "quality_type": "Lossy"}),

    ({"extension": "pdf", "format": "PDF", "title": "Deluxe Booklet PDF"},
     {"media_group": "Book", "format_class": "pdf", "media_class": "Booklet", "quality_type": "Lossy"}),
])
def test_classify_media(metadata, expected):
    result = classify_media(metadata)
    assert result == expected, f"Expected {expected} but got {result}"
