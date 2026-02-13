# ============================================================================
# File: /metadata/editor.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Unified tag reader/writer engine built on mutagen. Provides a format-aware
# facade that normalizes different tag systems (ID3v2, MP4 atoms, Vorbis
# Comments) into TAG_MAP's internal snake_case keys from core/tag_registry.py.
#
# Supports reading and writing tags for: MP3, FLAC, M4A/MP4, OGG Vorbis,
# OGG Opus, AIFF, and WMA/ASF. MKV/MKA reading is deferred to pymediainfo.
#
# Key design decisions:
#   - mutagen.File() for format auto-detection
#   - ID3v2 "3/12" track numbers → split into track_num + total_tracks
#   - MP4 trkn tuples (3, 12) → same split
#   - Custom tags via TXXX (ID3), freeform atoms (MP4), any key (Vorbis)
#   - All string values stripped of leading/trailing whitespace
#   - Multi-value fields returned as semicolon-delimited strings by default
# ============================================================================

import os                                              # File path operations
import logging                                         # Structured logging
from dataclasses import dataclass, field               # Data class for CoverArt
from typing import Optional                            # Type hints

import mutagen                                         # Core mutagen library for format detection
from mutagen.id3 import (                              # ID3v2 tag frame types (MP3, AIFF)
    ID3,
    TIT2, TPE1, TPE2, TALB, TDRC, TCON, TRCK, TPOS,
    TCOM, TPUB, TBPM, COMM, TXXX, APIC, USLT,
)
from mutagen.mp3 import MP3                            # MP3 format handler
from mutagen.flac import FLAC, Picture                 # FLAC format handler with picture support
from mutagen.mp4 import MP4, MP4Cover                  # MP4/M4A format handler
from mutagen.oggvorbis import OggVorbis                # OGG Vorbis format handler
from mutagen.oggopus import OggOpus                    # OGG Opus format handler
from mutagen.aiff import AIFF                          # AIFF format handler
from mutagen.asf import ASF                            # WMA/WMV/ASF format handler

from metadata.multi_value import (                     # Multi-value field utilities
    parse_multi_value,
    format_multi_value,
)

logger = logging.getLogger("MeedyaManager.TagEditor")


# =============================================================================
# Custom Exceptions
# =============================================================================

class UnsupportedFormatError(Exception):
    """Raised when a file format does not support tag writing via mutagen."""
    pass


class TagWriteError(Exception):
    """Raised when writing tags to a file fails."""
    pass


# =============================================================================
# Cover Art Data Class
# =============================================================================

@dataclass
class CoverArt:
    """
    Represents a single cover art image extracted from or to be embedded
    into a media file.

    Attributes:
        data: Raw image bytes (JPEG or PNG)
        format: Image format string — "jpeg" or "png"
        picture_type: ID3/FLAC picture type enum (3 = front cover, 4 = back cover)
        description: Optional text description of the image
        width: Image width in pixels (0 if unknown)
        height: Image height in pixels (0 if unknown)
    """
    data: bytes
    format: str = "jpeg"
    picture_type: int = 3
    description: str = ""
    width: int = 0
    height: int = 0


# =============================================================================
# Format-Specific Tag Mappings
# =============================================================================
# Each dict maps format-specific tag keys → TAG_MAP internal snake_case keys.
# These are the "Rosetta Stones" that allow unified tag access across formats.

# ID3v2 frame IDs → internal keys (MP3, AIFF, TTA)
ID3_TAG_MAP = {
    "TIT2": "title",               # Song title
    "TPE1": "artist",              # Lead artist/performer
    "TALB": "album",               # Album name
    "TPE2": "album_artist",        # Album artist / band
    "TDRC": "year",                # Recording date (ID3v2.4)
    "TCON": "genre",               # Genre
    "TRCK": "track_num",           # Track number (may be "3/12")
    "TPOS": "disc_num",            # Disc number (may be "1/2")
    "TCOM": "composer",            # Composer
    "TPUB": "publisher",           # Publisher / label
    "TBPM": "bpm",                 # Beats per minute
}

# ID3v2 frames that need special handling (not simple text)
ID3_SPECIAL_FRAMES = {"COMM", "TXXX", "APIC", "USLT"}

# MP4/M4A atom keys → internal keys
MP4_TAG_MAP = {
    "\xa9nam": "title",            # Song title
    "\xa9ART": "artist",           # Artist
    "\xa9alb": "album",            # Album
    "aART": "album_artist",        # Album artist
    "\xa9day": "year",             # Year / release date
    "\xa9gen": "genre",            # Genre
    "trkn": "track_num",           # Track number tuple: (track, total)
    "disk": "disc_num",            # Disc number tuple: (disc, total)
    "\xa9wrt": "composer",         # Composer / writer
    "\xa9cmt": "description",      # Comment
    "tmpo": "bpm",                 # Tempo (beats per minute)
    "tvsh": "show",                # TV show name
    "tvsn": "season",              # TV season number
    "tves": "episode",             # TV episode number
    "\xa9lyr": "lyrics",           # Lyrics
}

# MP4 freeform prefix for custom tags (used by MusicBrainz Picard, MP3tag, etc.)
MP4_FREEFORM_PREFIX = "----:com.apple.iTunes:"

# Vorbis Comment keys → internal keys (FLAC, OGG Vorbis, OGG Opus)
VORBIS_TAG_MAP = {
    "TITLE": "title",              # Song title
    "ARTIST": "artist",            # Artist
    "ALBUM": "album",              # Album
    "ALBUMARTIST": "album_artist", # Album artist
    "DATE": "year",                # Year / date
    "GENRE": "genre",              # Genre
    "TRACKNUMBER": "track_num",    # Track number
    "DISCNUMBER": "disc_num",      # Disc number
    "TOTALTRACKS": "total_tracks", # Total tracks in album
    "TOTALDISCS": "total_discs",   # Total discs in set
    "COMPOSER": "composer",        # Composer
    "PUBLISHER": "publisher",      # Publisher / label
    "COMMENT": "description",      # Comment
    "BPM": "bpm",                  # Beats per minute
    "ISRC": "isrc",                # International Standard Recording Code
    "LYRICS": "lyrics",            # Lyrics
}

# Reverse mappings: internal key → format-specific key (for writing)
ID3_REVERSE_MAP = {v: k for k, v in ID3_TAG_MAP.items()}
MP4_REVERSE_MAP = {v: k for k, v in MP4_TAG_MAP.items()}
VORBIS_REVERSE_MAP = {v: k for k, v in VORBIS_TAG_MAP.items()}

# ID3v2 frame class lookup for writing (frame ID → frame class constructor)
ID3_FRAME_CLASSES = {
    "TIT2": TIT2, "TPE1": TPE1, "TPE2": TPE2, "TALB": TALB,
    "TDRC": TDRC, "TCON": TCON, "TRCK": TRCK, "TPOS": TPOS,
    "TCOM": TCOM, "TPUB": TPUB, "TBPM": TBPM,
}

# Custom tag prefix matching core/tag_registry.py CUSTOM_TAG_PREFIX
CUSTOM_TAG_PREFIX = "custom_"


# =============================================================================
# TagEditor Class
# =============================================================================

class TagEditor:
    """
    Unified tag reader/writer that normalizes format-specific tags to the
    TAG_MAP internal keys defined in core/tag_registry.py.

    Supports reading and writing for: MP3, FLAC, M4A/MP4, OGG Vorbis,
    OGG Opus, AIFF, and ASF/WMA. MKV/MKA are not supported by mutagen
    and will raise UnsupportedFormatError on write attempts.

    Usage:
        editor = TagEditor()
        tags = editor.read_tags("/path/to/song.mp3")
        # tags = {"title": "My Song", "artist": "Band Name", ...}

        editor.write_tags("/path/to/song.mp3", {"artist": "New Artist"})
    """

    def read_tags(self, filepath):
        """
        Read all embedded tags from a media file and return a dictionary
        with TAG_MAP internal snake_case keys.

        Uses mutagen.File() for auto-detection, then dispatches to the
        appropriate format-specific reader.

        Args:
            filepath (str): Absolute or relative path to the media file.

        Returns:
            dict: Tag dictionary with internal keys. Empty dict if format
                  is not supported or file has no tags.
        """
        if not os.path.isfile(filepath):
            logger.warning(f"Tag read skipped — file not found: {filepath}")
            return {}

        try:
            audio = mutagen.File(filepath)
        except Exception as e:
            logger.debug(f"mutagen could not open {filepath}: {e}")
            return {}

        # mutagen returns None for unsupported formats
        if audio is None:
            logger.debug(f"Unsupported format for tag reading: {filepath}")
            return {}

        # Dispatch to format-specific reader based on mutagen file type
        if isinstance(audio, MP3) or isinstance(audio, AIFF):
            return self._read_id3(audio)
        elif isinstance(audio, MP4):
            return self._read_mp4(audio)
        elif isinstance(audio, (FLAC, OggVorbis, OggOpus)):
            return self._read_vorbis(audio)
        elif isinstance(audio, ASF):
            return self._read_asf(audio)
        else:
            logger.debug(f"No specific reader for type: {type(audio).__name__}")
            return {}

    def write_tags(self, filepath, tags, dry_run=False):
        """
        Write tags to a media file. Returns a dictionary of changes made.

        Args:
            filepath (str): Path to the media file.
            tags (dict): Dictionary of {internal_key: new_value} to write.
                         Pass None as value to remove a tag.
            dry_run (bool): If True, compute and return changes without
                            actually writing to the file.

        Returns:
            dict: Mapping of {internal_key: (old_value, new_value)} for
                  all tags that were changed.

        Raises:
            UnsupportedFormatError: If the file format cannot be written by mutagen.
            TagWriteError: If the write operation fails.
        """
        if not os.path.isfile(filepath):
            raise TagWriteError(f"File not found: {filepath}")

        try:
            audio = mutagen.File(filepath)
        except Exception as e:
            raise TagWriteError(f"Could not open file: {e}")

        if audio is None:
            raise UnsupportedFormatError(
                f"Format not supported for tag writing: {filepath}"
            )

        # Read current tags to compute diff
        current_tags = self.read_tags(filepath)

        # Compute the changes to be made
        changes = {}
        for key, new_value in tags.items():
            old_value = current_tags.get(key, "")
            # Normalize both values to strings for comparison
            old_str = str(old_value) if old_value else ""
            new_str = str(new_value) if new_value is not None else ""
            if old_str != new_str:
                changes[key] = (old_str, new_str)

        # If dry_run, return computed changes without writing
        if dry_run:
            logger.info(f"Dry run: {len(changes)} change(s) for {filepath}")
            return changes

        # If no changes detected, skip writing
        if not changes:
            logger.debug(f"No tag changes for {filepath}")
            return changes

        # Dispatch to format-specific writer
        try:
            if isinstance(audio, MP3) or isinstance(audio, AIFF):
                self._write_id3(audio, tags)
            elif isinstance(audio, MP4):
                self._write_mp4(audio, tags)
            elif isinstance(audio, (FLAC, OggVorbis, OggOpus)):
                self._write_vorbis(audio, tags)
            elif isinstance(audio, ASF):
                raise UnsupportedFormatError(
                    "ASF/WMA tag writing is not yet implemented"
                )
            else:
                raise UnsupportedFormatError(
                    f"Tag writing not supported for: {type(audio).__name__}"
                )

            # Save the file
            audio.save()
            logger.info(f"Wrote {len(changes)} tag(s) to {filepath}")

        except UnsupportedFormatError:
            raise
        except Exception as e:
            raise TagWriteError(f"Failed to write tags to {filepath}: {e}")

        return changes

    def read_cover_art(self, filepath):
        """
        Extract all cover art images from a media file.

        Args:
            filepath (str): Path to the media file.

        Returns:
            list[CoverArt]: List of CoverArt objects found in the file.
                            Empty list if no cover art or unsupported format.
        """
        if not os.path.isfile(filepath):
            return []

        try:
            audio = mutagen.File(filepath)
        except Exception:
            return []

        if audio is None:
            return []

        # Dispatch to format-specific cover art reader
        if isinstance(audio, MP3) or isinstance(audio, AIFF):
            return self._read_cover_id3(audio)
        elif isinstance(audio, MP4):
            return self._read_cover_mp4(audio)
        elif isinstance(audio, FLAC):
            return self._read_cover_flac(audio)
        elif isinstance(audio, (OggVorbis, OggOpus)):
            return self._read_cover_vorbis(audio)
        return []

    def write_cover_art(self, filepath, image_data, image_format="jpeg",
                        picture_type=3):
        """
        Embed cover art into a media file.

        Args:
            filepath (str): Path to the media file.
            image_data (bytes): Raw image bytes (JPEG or PNG).
            image_format (str): "jpeg" or "png".
            picture_type (int): Picture type enum — 3 = front cover (default),
                                4 = back cover, 0 = other.

        Raises:
            UnsupportedFormatError: If cover art embedding is not supported.
            TagWriteError: If the write operation fails.
        """
        if not os.path.isfile(filepath):
            raise TagWriteError(f"File not found: {filepath}")

        try:
            audio = mutagen.File(filepath)
        except Exception as e:
            raise TagWriteError(f"Could not open file: {e}")

        if audio is None:
            raise UnsupportedFormatError(
                f"Cover art not supported for: {filepath}"
            )

        # Determine MIME type from image format
        mime_type = f"image/{image_format}"

        try:
            if isinstance(audio, MP3) or isinstance(audio, AIFF):
                # ID3v2: Add APIC frame
                if audio.tags is None:
                    audio.add_tags()
                # Remove existing APIC frames of same type
                audio.tags.delall("APIC")
                audio.tags.add(APIC(
                    encoding=3,                            # UTF-8 encoding
                    mime=mime_type,
                    type=picture_type,
                    desc="Cover",
                    data=image_data,
                ))

            elif isinstance(audio, MP4):
                # MP4: Set covr atom
                fmt = (MP4Cover.FORMAT_JPEG if image_format == "jpeg"
                       else MP4Cover.FORMAT_PNG)
                audio["covr"] = [MP4Cover(image_data, imageformat=fmt)]

            elif isinstance(audio, FLAC):
                # FLAC: Add Picture metadata block
                picture = Picture()
                picture.type = picture_type
                picture.mime = mime_type
                picture.desc = "Cover"
                picture.data = image_data
                audio.clear_pictures()                     # Remove existing pictures
                audio.add_picture(picture)

            elif isinstance(audio, (OggVorbis, OggOpus)):
                # OGG: Encode picture as base64 METADATA_BLOCK_PICTURE
                import base64
                picture = Picture()
                picture.type = picture_type
                picture.mime = mime_type
                picture.desc = "Cover"
                picture.data = image_data
                encoded = base64.b64encode(picture.write()).decode("ascii")
                audio["metadata_block_picture"] = [encoded]

            else:
                raise UnsupportedFormatError(
                    f"Cover art not supported for: {type(audio).__name__}"
                )

            audio.save()
            logger.info(f"Cover art written to {filepath}")

        except UnsupportedFormatError:
            raise
        except Exception as e:
            raise TagWriteError(f"Failed to write cover art: {e}")

    def remove_cover_art(self, filepath):
        """
        Remove all cover art from a media file.

        Args:
            filepath (str): Path to the media file.

        Returns:
            int: Number of cover art images removed.

        Raises:
            TagWriteError: If the removal fails.
        """
        if not os.path.isfile(filepath):
            raise TagWriteError(f"File not found: {filepath}")

        try:
            audio = mutagen.File(filepath)
        except Exception as e:
            raise TagWriteError(f"Could not open file: {e}")

        if audio is None:
            return 0

        count = 0

        try:
            if isinstance(audio, MP3) or isinstance(audio, AIFF):
                # Count and remove all APIC frames
                if audio.tags:
                    apic_frames = audio.tags.getall("APIC")
                    count = len(apic_frames)
                    audio.tags.delall("APIC")

            elif isinstance(audio, MP4):
                # Remove covr atom
                if "covr" in audio:
                    count = len(audio["covr"])
                    del audio["covr"]

            elif isinstance(audio, FLAC):
                # Remove all picture blocks
                count = len(audio.pictures)
                audio.clear_pictures()

            elif isinstance(audio, (OggVorbis, OggOpus)):
                # Remove METADATA_BLOCK_PICTURE entries
                if "metadata_block_picture" in audio:
                    count = len(audio["metadata_block_picture"])
                    del audio["metadata_block_picture"]

            if count > 0:
                audio.save()
                logger.info(f"Removed {count} cover art image(s) from {filepath}")

        except Exception as e:
            raise TagWriteError(f"Failed to remove cover art: {e}")

        return count

    def get_supported_format(self, filepath):
        """
        Check if a file format is supported for tag writing.

        Args:
            filepath (str): Path to the media file.

        Returns:
            str or None: Format name (e.g., "MP3", "FLAC", "MP4") if supported,
                         None if not supported for writing.
        """
        if not os.path.isfile(filepath):
            return None

        try:
            audio = mutagen.File(filepath)
        except Exception:
            return None

        if audio is None:
            return None

        # Map mutagen types to friendly format names
        format_names = {
            MP3: "MP3",
            FLAC: "FLAC",
            MP4: "MP4",
            OggVorbis: "OGG Vorbis",
            OggOpus: "OGG Opus",
            AIFF: "AIFF",
            ASF: "ASF",
        }

        for fmt_class, name in format_names.items():
            if isinstance(audio, fmt_class):
                return name

        return None

    # =========================================================================
    # ID3v2 Reader/Writer (MP3, AIFF)
    # =========================================================================

    def _read_id3(self, audio):
        """
        Read ID3v2 tags from an MP3 or AIFF file and return a normalized dict.

        ID3v2 uses frame-based tags (TIT2 for title, TPE1 for artist, etc.).
        Track numbers like "3/12" are split into track_num and total_tracks.
        Custom TXXX frames are read as custom_ prefixed keys.

        Args:
            audio: mutagen MP3 or AIFF file object.

        Returns:
            dict: Normalized tag dictionary with internal keys.
        """
        tags = {}

        if audio.tags is None:
            return tags

        for frame_id, internal_key in ID3_TAG_MAP.items():
            frame = audio.tags.get(frame_id)
            if frame is not None:
                # ID3 text frames store values as a list of strings
                value = str(frame)

                # Handle track/disc number splitting: "3/12" → num + total
                if frame_id == "TRCK" and "/" in value:
                    parts = value.split("/", 1)
                    tags["track_num"] = parts[0].strip()
                    tags["total_tracks"] = parts[1].strip()
                    continue
                elif frame_id == "TPOS" and "/" in value:
                    parts = value.split("/", 1)
                    tags["disc_num"] = parts[0].strip()
                    tags["total_discs"] = parts[1].strip()
                    continue

                tags[internal_key] = value.strip()

        # Read comment frames (COMM::'eng' or any language)
        for key in audio.tags:
            if key.startswith("COMM"):
                frame = audio.tags[key]
                tags["description"] = str(frame).strip()
                break

        # Read lyrics (USLT frames)
        for key in audio.tags:
            if key.startswith("USLT"):
                frame = audio.tags[key]
                tags["lyrics"] = str(frame).strip()
                break

        # Read custom TXXX frames → custom_ prefixed keys
        for key in audio.tags:
            if key.startswith("TXXX:"):
                frame = audio.tags[key]
                custom_name = key.replace("TXXX:", "").strip()
                # Convert to internal custom key format
                internal = CUSTOM_TAG_PREFIX + custom_name.lower().replace(" ", "_")
                tags[internal] = str(frame).strip()

        return tags

    def _write_id3(self, audio, tags):
        """
        Write tags to an ID3v2 file (MP3 or AIFF).

        Creates or updates ID3v2 text frames. Handles track/disc number
        reassembly (track_num + total_tracks → "3/12" TRCK frame).
        Custom tags are written as TXXX frames.

        Args:
            audio: mutagen MP3 or AIFF file object (modified in-place).
            tags (dict): Dictionary of {internal_key: value} to write.
        """
        # Ensure ID3 tags exist
        if audio.tags is None:
            audio.add_tags()

        for internal_key, value in tags.items():
            # Handle tag removal (value is None or empty string)
            if value is None or value == "":
                frame_id = ID3_REVERSE_MAP.get(internal_key)
                if frame_id and frame_id in audio.tags:
                    del audio.tags[frame_id]
                continue

            # Handle custom tags → TXXX frames
            if internal_key.startswith(CUSTOM_TAG_PREFIX):
                custom_name = internal_key[len(CUSTOM_TAG_PREFIX):].replace("_", " ").title()
                audio.tags.add(TXXX(
                    encoding=3,                            # UTF-8
                    desc=custom_name,
                    text=[str(value)],
                ))
                continue

            # Handle track_num with total_tracks → "num/total" TRCK
            if internal_key == "track_num":
                total = tags.get("total_tracks", "")
                trck_value = str(value)
                if total:
                    trck_value = f"{value}/{total}"
                audio.tags.add(TRCK(encoding=3, text=[trck_value]))
                continue

            # Handle disc_num with total_discs → "num/total" TPOS
            if internal_key == "disc_num":
                total = tags.get("total_discs", "")
                tpos_value = str(value)
                if total:
                    tpos_value = f"{value}/{total}"
                audio.tags.add(TPOS(encoding=3, text=[tpos_value]))
                continue

            # Skip total_tracks/total_discs — handled above with track_num/disc_num
            if internal_key in ("total_tracks", "total_discs"):
                continue

            # Handle description → COMM frame
            if internal_key == "description":
                # Remove existing comment frames
                for key in list(audio.tags):
                    if key.startswith("COMM"):
                        del audio.tags[key]
                audio.tags.add(COMM(
                    encoding=3,
                    lang="eng",
                    desc="",
                    text=[str(value)],
                ))
                continue

            # Handle lyrics → USLT frame
            if internal_key == "lyrics":
                for key in list(audio.tags):
                    if key.startswith("USLT"):
                        del audio.tags[key]
                audio.tags.add(USLT(
                    encoding=3,
                    lang="eng",
                    desc="",
                    text=str(value),
                ))
                continue

            # Standard text frames — look up frame ID and class
            frame_id = ID3_REVERSE_MAP.get(internal_key)
            if frame_id and frame_id in ID3_FRAME_CLASSES:
                frame_class = ID3_FRAME_CLASSES[frame_id]
                audio.tags.add(frame_class(encoding=3, text=[str(value)]))

    # =========================================================================
    # MP4/M4A Reader/Writer
    # =========================================================================

    def _read_mp4(self, audio):
        """
        Read MP4/M4A atom tags and return a normalized dict.

        MP4 uses atom keys like \\xa9nam for title, trkn as (track, total) tuples.
        Custom tags use the freeform ----:com.apple.iTunes: prefix.

        Args:
            audio: mutagen MP4 file object.

        Returns:
            dict: Normalized tag dictionary with internal keys.
        """
        tags = {}

        if audio.tags is None:
            return tags

        for atom_key, internal_key in MP4_TAG_MAP.items():
            if atom_key in audio.tags:
                value = audio.tags[atom_key]

                # Handle tuple-based track/disc numbers: [(3, 12)]
                if atom_key in ("trkn", "disk") and isinstance(value, list):
                    if value and isinstance(value[0], tuple):
                        num, total = value[0]
                        tags[internal_key] = str(num)
                        if atom_key == "trkn":
                            tags["total_tracks"] = str(total) if total else ""
                        elif atom_key == "disk":
                            tags["total_discs"] = str(total) if total else ""
                        continue

                # Handle integer values (tmpo for BPM, tvsn/tves for season/episode)
                if atom_key in ("tmpo", "tvsn", "tves") and isinstance(value, list):
                    tags[internal_key] = str(value[0]) if value else ""
                    continue

                # Handle string list values (most atoms store as list of strings)
                if isinstance(value, list):
                    # Multi-value: join with semicolons
                    str_values = [str(v).strip() for v in value if str(v).strip()]
                    tags[internal_key] = format_multi_value(str_values) if len(str_values) > 1 else (str_values[0] if str_values else "")
                else:
                    tags[internal_key] = str(value).strip()

        # Read custom freeform tags (----:com.apple.iTunes:KEY)
        if audio.tags:
            for key in audio.tags:
                if isinstance(key, str) and key.startswith(MP4_FREEFORM_PREFIX):
                    custom_name = key[len(MP4_FREEFORM_PREFIX):]
                    value = audio.tags[key]
                    if isinstance(value, list):
                        # Freeform values are bytes objects
                        str_values = []
                        for v in value:
                            if isinstance(v, bytes):
                                str_values.append(v.decode("utf-8", errors="replace").strip())
                            else:
                                str_values.append(str(v).strip())
                        internal = CUSTOM_TAG_PREFIX + custom_name.lower().replace(" ", "_")
                        tags[internal] = format_multi_value(str_values) if len(str_values) > 1 else (str_values[0] if str_values else "")

        return tags

    def _write_mp4(self, audio, tags):
        """
        Write tags to an MP4/M4A file.

        Handles tuple-based track/disc numbers, integer fields, and custom
        freeform atoms for user-defined tags.

        Args:
            audio: mutagen MP4 file object (modified in-place).
            tags (dict): Dictionary of {internal_key: value} to write.
        """
        if audio.tags is None:
            audio.add_tags()

        for internal_key, value in tags.items():
            # Handle tag removal
            if value is None or value == "":
                atom_key = MP4_REVERSE_MAP.get(internal_key)
                if atom_key and atom_key in audio.tags:
                    del audio.tags[atom_key]
                continue

            # Handle custom tags → freeform atoms
            if internal_key.startswith(CUSTOM_TAG_PREFIX):
                custom_name = internal_key[len(CUSTOM_TAG_PREFIX):].replace("_", " ").title()
                freeform_key = f"{MP4_FREEFORM_PREFIX}{custom_name}"
                audio.tags[freeform_key] = [str(value).encode("utf-8")]
                continue

            # Handle track_num → trkn tuple
            if internal_key == "track_num":
                total = tags.get("total_tracks", "")
                try:
                    num = int(value)
                    tot = int(total) if total else 0
                    audio.tags["trkn"] = [(num, tot)]
                except (ValueError, TypeError):
                    pass
                continue

            # Handle disc_num → disk tuple
            if internal_key == "disc_num":
                total = tags.get("total_discs", "")
                try:
                    num = int(value)
                    tot = int(total) if total else 0
                    audio.tags["disk"] = [(num, tot)]
                except (ValueError, TypeError):
                    pass
                continue

            # Skip total_tracks/total_discs — handled with track_num/disc_num
            if internal_key in ("total_tracks", "total_discs"):
                continue

            # Handle integer fields (BPM, season, episode)
            atom_key = MP4_REVERSE_MAP.get(internal_key)
            if atom_key in ("tmpo", "tvsn", "tves"):
                try:
                    audio.tags[atom_key] = [int(value)]
                except (ValueError, TypeError):
                    pass
                continue

            # Standard string atoms
            if atom_key:
                audio.tags[atom_key] = [str(value)]

    # =========================================================================
    # Vorbis Comments Reader/Writer (FLAC, OGG Vorbis, OGG Opus)
    # =========================================================================

    def _read_vorbis(self, audio):
        """
        Read Vorbis Comments from a FLAC or OGG file and return a normalized dict.

        Vorbis Comments are flat key-value pairs where any key is valid and
        multi-value is natively supported (same key appearing multiple times).

        Args:
            audio: mutagen FLAC, OggVorbis, or OggOpus file object.

        Returns:
            dict: Normalized tag dictionary with internal keys.
        """
        tags = {}

        if audio.tags is None:
            return tags

        for vorbis_key, internal_key in VORBIS_TAG_MAP.items():
            if vorbis_key in audio.tags:
                values = audio.tags[vorbis_key]
                # Vorbis Comments always return lists
                if isinstance(values, list):
                    stripped = [v.strip() for v in values if v.strip()]
                    tags[internal_key] = format_multi_value(stripped) if len(stripped) > 1 else (stripped[0] if stripped else "")
                else:
                    tags[internal_key] = str(values).strip()

        # Read custom tags (any key not in VORBIS_TAG_MAP)
        known_vorbis_keys = set(k.upper() for k in VORBIS_TAG_MAP.keys())
        # Also skip cover art metadata key
        known_vorbis_keys.add("METADATA_BLOCK_PICTURE")
        # NOTE: Use audio.tags.keys() instead of iterating audio.tags directly.
        # For FLAC VCFLACDict, __iter__ yields (key, value) tuples, but
        # keys() returns just the string key names.
        for key in audio.tags.keys():
            if key.upper() not in known_vorbis_keys:
                values = audio.tags[key]
                internal = CUSTOM_TAG_PREFIX + key.lower().replace(" ", "_")
                if isinstance(values, list):
                    stripped = [str(v).strip() for v in values if str(v).strip()]
                    tags[internal] = format_multi_value(stripped) if len(stripped) > 1 else (stripped[0] if stripped else "")
                else:
                    tags[internal] = str(values).strip()

        return tags

    def _write_vorbis(self, audio, tags):
        """
        Write Vorbis Comments to a FLAC or OGG file.

        Vorbis Comments support any key name natively, making custom tag
        writing straightforward. Multi-value fields are written as multiple
        entries for the same key.

        Args:
            audio: mutagen FLAC, OggVorbis, or OggOpus file object (modified in-place).
            tags (dict): Dictionary of {internal_key: value} to write.
        """
        if audio.tags is None:
            # For FLAC, tags are always present; for OGG, add if missing
            if hasattr(audio, "add_tags"):
                audio.add_tags()

        for internal_key, value in tags.items():
            # Handle tag removal
            if value is None or value == "":
                vorbis_key = VORBIS_REVERSE_MAP.get(internal_key)
                if vorbis_key and vorbis_key in audio.tags:
                    del audio.tags[vorbis_key]
                continue

            # Handle custom tags
            if internal_key.startswith(CUSTOM_TAG_PREFIX):
                custom_name = internal_key[len(CUSTOM_TAG_PREFIX):].upper().replace("_", " ")
                # Parse multi-value: semicolons → multiple Vorbis entries
                values = parse_multi_value(value)
                audio.tags[custom_name] = values
                continue

            # Standard Vorbis Comment keys
            vorbis_key = VORBIS_REVERSE_MAP.get(internal_key)
            if vorbis_key:
                # Parse multi-value for fields that support it
                values = parse_multi_value(value)
                audio.tags[vorbis_key] = values

    # =========================================================================
    # ASF Reader (WMA/WMV — read-only for now)
    # =========================================================================

    def _read_asf(self, audio):
        """
        Read ASF (WMA/WMV) tags and return a normalized dict.
        Write support for ASF is not yet implemented.

        Args:
            audio: mutagen ASF file object.

        Returns:
            dict: Normalized tag dictionary with internal keys.
        """
        tags = {}

        # ASF standard attribute mappings
        asf_map = {
            "Title": "title",
            "Author": "artist",
            "WM/AlbumTitle": "album",
            "WM/AlbumArtist": "album_artist",
            "WM/Year": "year",
            "WM/Genre": "genre",
            "WM/TrackNumber": "track_num",
            "WM/PartOfSet": "disc_num",
            "WM/Composer": "composer",
            "WM/Publisher": "publisher",
            "Description": "description",
            "WM/BeatsPerMinute": "bpm",
        }

        if audio.tags is None:
            return tags

        for asf_key, internal_key in asf_map.items():
            if asf_key in audio.tags:
                values = audio.tags[asf_key]
                if values:
                    tags[internal_key] = str(values[0]).strip()

        return tags

    # =========================================================================
    # Cover Art Readers (format-specific)
    # =========================================================================

    def _read_cover_id3(self, audio):
        """
        Extract APIC (cover art) frames from an ID3v2 file.

        Args:
            audio: mutagen MP3 or AIFF file object.

        Returns:
            list[CoverArt]: List of cover art images.
        """
        covers = []
        if audio.tags is None:
            return covers

        for key in audio.tags:
            if key.startswith("APIC"):
                frame = audio.tags[key]
                img_format = "jpeg" if "jpeg" in frame.mime.lower() else "png"
                covers.append(CoverArt(
                    data=frame.data,
                    format=img_format,
                    picture_type=frame.type,
                    description=frame.desc or "",
                ))
        return covers

    def _read_cover_mp4(self, audio):
        """
        Extract cover art from MP4 covr atom.

        Args:
            audio: mutagen MP4 file object.

        Returns:
            list[CoverArt]: List of cover art images.
        """
        covers = []
        if audio.tags and "covr" in audio.tags:
            for cover in audio.tags["covr"]:
                img_format = "png" if cover.imageformat == MP4Cover.FORMAT_PNG else "jpeg"
                covers.append(CoverArt(
                    data=bytes(cover),
                    format=img_format,
                    picture_type=3,                        # MP4 doesn't have type enum
                    description="",
                ))
        return covers

    def _read_cover_flac(self, audio):
        """
        Extract Picture metadata blocks from a FLAC file.

        Args:
            audio: mutagen FLAC file object.

        Returns:
            list[CoverArt]: List of cover art images.
        """
        covers = []
        for picture in audio.pictures:
            img_format = "jpeg" if "jpeg" in picture.mime.lower() else "png"
            covers.append(CoverArt(
                data=picture.data,
                format=img_format,
                picture_type=picture.type,
                description=picture.desc or "",
                width=picture.width,
                height=picture.height,
            ))
        return covers

    def _read_cover_vorbis(self, audio):
        """
        Extract cover art from OGG METADATA_BLOCK_PICTURE entries.

        OGG files store cover art as base64-encoded FLAC Picture blocks
        in a Vorbis Comment field.

        Args:
            audio: mutagen OggVorbis or OggOpus file object.

        Returns:
            list[CoverArt]: List of cover art images.
        """
        import base64
        covers = []

        if audio.tags and "metadata_block_picture" in audio.tags:
            for encoded in audio.tags["metadata_block_picture"]:
                try:
                    raw = base64.b64decode(encoded)
                    picture = Picture(raw)
                    img_format = "jpeg" if "jpeg" in picture.mime.lower() else "png"
                    covers.append(CoverArt(
                        data=picture.data,
                        format=img_format,
                        picture_type=picture.type,
                        description=picture.desc or "",
                        width=picture.width,
                        height=picture.height,
                    ))
                except Exception as e:
                    logger.debug(f"Failed to decode OGG cover art: {e}")

        return covers
