# ============================================================================
# File: /core/classify_media.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Provides utility functions to classify a media file into:
# - Media Group (e.g., Audio, Video, Image, Book)
# - Format Class (container/codec type)
# - Media Class (intended content type: Music, Movie, Podcast, etc.)
# - Quality Type (Lossy, Lossless)
#
# This logic is used during metadata extraction, renaming, organizing,
# and also for export/tagging. Supports both automatic inference and
# manual override from user metadata (planned in future UI/editor).
#
# Reference:
# Inspired by structures in MusicBrainz, Apple iTunesMediaType, EIDR, etc.
# https://musicbrainz.org/doc/
# ============================================================================

import os

def classify_media(metadata: dict) -> dict:
    """
    Given a parsed metadata dict, return classification dictionary.
    This includes:
      - media_group
      - format_class
      - media_class
      - quality_type
    """
    extension = metadata.get("extension", "").lower()
    format_tag = metadata.get("format", "").lower()
    channels = metadata.get("audio_channels", 0)
    is_lossless = metadata.get("is_lossless", False)
    title = metadata.get("title", "").lower()
    description = metadata.get("description", "").lower()
    duration = metadata.get("duration", 0)

    # === 1. Media Group ===
    if format_tag in ["flac", "mp3", "aac", "m4a", "wav", "ac3", "alac", "ogg"]:
        media_group = "Audio"
    elif format_tag in ["mp4", "mkv", "matroska", "avi", "webm", "mov", "m4v"]:
        media_group = "Video"
    elif extension in ["jpg", "jpeg", "png", "gif", "webp"]:
        media_group = "Image"
    elif extension in ["pdf", "epub", "mobi"]:
        media_group = "Book"
    else:
        media_group = "Unknown"

    # === 2. Format Class ===
    format_class = format_tag if format_tag else extension

    # === 3. Media Class ===
    media_class = "Unknown"
    if media_group == "Audio":
        if "podcast" in title or "podcast" in description:
            media_class = "Podcast"
        elif "radio" in title or "radio" in description:
            media_class = "Radio Show"
        else:
            media_class = "Music"
    elif media_group == "Video":
        if duration < 180:  # < 3 mins
            media_class = "Music Video"
        elif "movie" in title or "film" in title:
            media_class = "Movie"
        elif "tv" in title or "episode" in title:
            media_class = "TV Show"
        else:
            media_class = "Video"
    elif media_group == "Image":
        media_class = "Photo"
    elif media_group == "Book":
        if "booklet" in title:
            media_class = "Booklet"
        else:
            media_class = "eBook"

    # === 4. Quality Type ===
    quality_type = "Lossless" if is_lossless else "Lossy"

    return {
        "media_group": media_group,
        "format_class": format_class,
        "media_class": media_class,
        "quality_type": quality_type,
    }
