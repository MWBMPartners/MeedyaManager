# ============================================================================
# File: /core/metadata_extractor.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# This module extracts metadata from media files using pymediainfo, which is a
# Python binding to the MediaInfo CLI/API. It is used to populate structured
# tags for use by the renamer, rule engine, or display logic.
#
# Supported fields include artist, album, title, track number, duration, format,
# codec, channel layout, and more. It also returns media_type based on heuristics.
#
# Requires: pymediainfo
# pip install pymediainfo
#
# References:
# https://github.com/sbraz/pymediainfo
# https://mediaarea.net/en/MediaInfo
# ============================================================================

from pymediainfo import MediaInfo
import os
import logging
from utils.config_loader import get_config

logger = logging.getLogger("MetaMancer.Metadata")


def extract_metadata(filepath):
    """
    Extracts key media metadata tags from the given file, applying fallbacks
    from settings.json5 when tags are missing.

    Args:
        filepath (str): Absolute or relative path to a media file

    Returns:
        dict: Structured metadata tags used by the renamer
    """
    # Load fallback/defaults from config
    fallback = get_config("default_metadata", {})

    metadata = {
        'media_type': fallback.get('media_type', 'Unknown'),
        'artist': fallback.get('artist', 'Unknown Artist'),
        'album': fallback.get('album', 'Unknown Album'),
        'track_number': fallback.get('track_number', '00'),
        'title': os.path.splitext(os.path.basename(filepath))[0],
        'ext': os.path.splitext(filepath)[1].lstrip('.')
    }

    try:
        media_info = MediaInfo.parse(filepath)
        for track in media_info.tracks:
            if track.track_type == 'Audio':
                if track.performer:
                    metadata['artist'] = track.performer
                if track.album:
                    metadata['album'] = track.album
                if track.track_name:
                    metadata['title'] = track.track_name
                if track.track_position:
                    metadata['track_number'] = str(track.track_position).zfill(2)

            if track.track_type == 'General':
                # Determine media_type by format + extension
                if track.format:
                    fmt = track.format.lower()
                    ext = metadata['ext'].lower()
                    if any(k in fmt for k in ['mkv', 'mp4', 'mov', 'avi']) or ext in ['mkv', 'mp4', 'm4v', 'avi']:
                        metadata['media_type'] = 'Video'
                    elif any(k in fmt for k in ['mp3', 'aac', 'flac', 'alac', 'ac3']) or ext in ['mp3', 'm4a', 'flac', 'ogg']:
                        metadata['media_type'] = 'Music'
    
    except Exception as e:
        logger.warning(f"[MetaData] Failed to parse {filepath}: {e}")

    return metadata


if __name__ == '__main__':
    # Example test run
    test_file = "./watch_folder/sample.mp3"
    result = extract_metadata(test_file)
    print("\nExtracted Metadata:")
    for key, value in result.items():
        print(f"  {key}: {value}")