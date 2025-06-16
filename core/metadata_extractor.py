# ============================================================================
# File: /core/metadata_extractor.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Extracts technical and tag metadata from media files using pymediainfo.
# Applies classification logic to determine media_group, format_class,
# media_class, and quality_type. Falls back to settings.json5 defaults if
# required fields are missing.
# ============================================================================

from pymediainfo import MediaInfo
from core.classify_media import classify_media
from utils.config_loader import get_config
import os


def extract_metadata(filepath):
    """
    Extracts metadata from a media file using MediaInfo and returns a dictionary
    that includes basic technical metadata and classification tags.
    """
    if not os.path.isfile(filepath):
        raise FileNotFoundError(f"Metadata extraction failed: File not found: {filepath}")

    fallback = get_config("fallback_metadata", {
        "media_group": "Audio",
        "format_class": "unknown",
        "media_class": "Music",
        "quality_type": "Lossy"
    })

    metadata = {
        "filepath": filepath,
        "extension": os.path.splitext(filepath)[1][1:].lower(),
        "format": "",
        "duration": 0,
        "title": "",
        "description": "",
        "audio_channels": 0,
        "is_lossless": False,
    }

    try:
        media_info = MediaInfo.parse(filepath)
        for track in media_info.tracks:
            if track.track_type == "General":
                metadata["format"] = (track.format or metadata["extension"]).lower()
                metadata["duration"] = int(track.duration / 1000) if track.duration else 0
                metadata["title"] = (track.title or "").strip()
                metadata["description"] = (track.comment or track.description or "").strip()

            if track.track_type == "Audio":
                metadata["audio_channels"] = int(track.channel_s or 0)
                metadata["is_lossless"] = track.codec_id in ["FLAC", "ALAC"] or \
                                           track.format in ["FLAC", "ALAC"]
    except Exception as e:
        print(f"Warning: Failed to parse {filepath}: {e}")

    # Apply classification based on available metadata or fallback
    classified = classify_media(metadata)

    # Merge classification with base metadata (fallback-safe)
    result = {**metadata, **fallback, **classified}
    return result