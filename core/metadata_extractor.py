# ============================================================================
# File: /core/metadata_extractor.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Extracts technical and tag metadata from media files using a two-stage
# pipeline:
#   1. pymediainfo — Technical stream analysis (codecs, bitrate, spatial
#      audio, channels, duration, resolution)
#   2. mutagen (via TagEditor) — Embedded music/video tags (artist, album,
#      genre, track number, cover art, custom tags, etc.)
#
# Both libraries are complementary: pymediainfo handles container/stream
# metadata that mutagen cannot read (e.g., MKV, codec details), while
# mutagen handles embedded tag fields that pymediainfo does not expose.
#
# Applies classification logic to determine media_group, format_class,
# media_class, and quality_type. Falls back to settings.json5 defaults if
# required fields are missing.
# ============================================================================

import logging                                         # Structured logging
import os                                              # File path operations

from pymediainfo import MediaInfo                      # Technical stream analysis
from core.classify_media import classify_media         # 4-level classification
from utils.config_loader import get_config             # Settings/fallback loader
from utils.mediainfo_loader import get_mediainfo_parse_kwargs  # Bundled library resolution

logger = logging.getLogger("MeedyaManager.MetadataExtractor")

# Resolve the libmediainfo library path once at module load time.
# Returns {"library_file": "/path/to/lib"} if a bundled library is found,
# or {} to let pymediainfo use its default system detection.
_MEDIAINFO_KWARGS = get_mediainfo_parse_kwargs()


def extract_metadata(filepath):
    """
    Extract all metadata from a media file using a two-stage pipeline.

    Stage 1 (pymediainfo): Technical stream metadata — format, duration,
    codec, bitrate, channels, lossless detection. These fields describe
    the container and audio/video streams.

    Stage 2 (mutagen via TagEditor): Embedded music/video tags — artist,
    album, genre, year, track number, composer, and any custom tags.
    These fields describe the content, not the container.

    The two stages are complementary:
    - pymediainfo fields take priority for technical metadata (format,
      duration, audio_channels, is_lossless, codec, bitrate, etc.)
    - mutagen fields fill in tag metadata that pymediainfo does not
      expose (artist, album, genre, year, track_num, etc.)
    - If both provide a tag field (e.g., title), mutagen's value is
      preferred since it reads the actual embedded tag, while pymediainfo
      may read a different container-level field.

    Args:
        filepath (str): Absolute or relative path to the media file.

    Returns:
        dict: Merged metadata dictionary with all available fields,
              classification tags, and fallback values.

    Raises:
        FileNotFoundError: If the file does not exist.
    """
    if not os.path.isfile(filepath):
        raise FileNotFoundError(f"Metadata extraction failed: File not found: {filepath}")

    # Load fallback values from settings for classification fields
    fallback = get_config("fallback_metadata", {
        "media_group": "Audio",
        "format_class": "unknown",
        "media_class": "Music",
        "quality_type": "Lossy"
    })

    # Base metadata dict with technical fields (pymediainfo stage)
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

    # =========================================================================
    # Stage 1: pymediainfo — Technical stream analysis
    # =========================================================================
    try:
        media_info = MediaInfo.parse(filepath, **_MEDIAINFO_KWARGS)
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
        logger.warning(f"pymediainfo failed for {filepath}: {e}")

    # =========================================================================
    # Stage 2: mutagen (via TagEditor) — Embedded music/video tags
    # =========================================================================
    # Read embedded tags (artist, album, genre, year, track_num, etc.)
    # These fields are not available from pymediainfo for most formats.
    try:
        from metadata.editor import TagEditor            # Import here to avoid circular imports
        tag_editor = TagEditor()
        embedded_tags = tag_editor.read_tags(filepath)

        # Merge strategy: mutagen tag values fill in fields that are empty
        # or missing from the pymediainfo stage. For fields that both stages
        # can provide (like "title" and "description"), mutagen's value is
        # preferred because it reads the actual embedded tag frame/atom.
        for key, value in embedded_tags.items():
            if value:                                    # Only merge non-empty values
                # For title and description, prefer mutagen over pymediainfo
                # because mutagen reads the actual embedded tag (TIT2/©nam)
                # while pymediainfo may read a container-level field
                if key in ("title", "description"):
                    metadata[key] = value
                # For all other tag fields, fill in if not already present
                elif key not in metadata or not metadata[key]:
                    metadata[key] = value

    except Exception as e:
        # Tag reading is optional — pymediainfo metadata is sufficient for
        # classification and basic operations. Log and continue.
        logger.debug(f"Tag reading skipped for {filepath}: {e}")

    # =========================================================================
    # Stage 3: Classification — 4-level media hierarchy
    # =========================================================================
    classified = classify_media(metadata)

    # Merge classification with base metadata (fallback-safe)
    result = {**metadata, **fallback, **classified}
    return result