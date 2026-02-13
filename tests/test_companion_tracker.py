# ============================================================================
# File: /tests/test_companion_tracker.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for core/companion_tracker.py — companion file detection and
# destination computation. Uses tmp_path fixtures for real filesystem tests.
# ============================================================================

import os                                          # File path operations
import pytest                                      # Test framework
from core.companion_tracker import (
    find_companions,                               # Main detection function
    compute_companion_destinations,                # Destination computation
    get_companion_summary,                         # Human-readable summary
    CompanionFile,                                 # Named tuple type
    COMPANION_EXTENSIONS,                          # Extension categories
    DIRECTORY_COMPANIONS,                          # Directory-level filenames
)


# ============================================================================
# find_companions() tests
# ============================================================================

def test_find_subtitle_companion(tmp_path):
    """Detect .srt subtitle file alongside .mkv video."""
    media = tmp_path / "movie.mkv"
    subtitle = tmp_path / "movie.srt"
    media.write_text("FAKE_VIDEO")
    subtitle.write_text("FAKE_SUBTITLE")

    result = find_companions(str(media))
    assert len(result) == 1
    assert result[0].category == "subtitles"
    assert result[0].path == str(subtitle)


def test_find_lyrics_companion(tmp_path):
    """Detect .lrc lyrics file alongside .mp3 audio."""
    media = tmp_path / "song.mp3"
    lyrics = tmp_path / "song.lrc"
    media.write_text("FAKE_AUDIO")
    lyrics.write_text("[00:01.00]Hello")

    result = find_companions(str(media))
    assert len(result) == 1
    assert result[0].category == "lyrics"
    assert result[0].path == str(lyrics)


def test_find_cue_companion(tmp_path):
    """Detect .cue sheet file alongside .flac audio."""
    media = tmp_path / "album.flac"
    cue = tmp_path / "album.cue"
    media.write_text("FAKE_FLAC")
    cue.write_text("FILE album.flac WAVE")

    result = find_companions(str(media))
    assert len(result) == 1
    assert result[0].category == "cue_sheets"


def test_find_nfo_companion(tmp_path):
    """Detect .nfo metadata file alongside media file."""
    media = tmp_path / "movie.mp4"
    nfo = tmp_path / "movie.nfo"
    media.write_text("FAKE_VIDEO")
    nfo.write_text("<movie><title>Test</title></movie>")

    result = find_companions(str(media))
    assert len(result) == 1
    assert result[0].category == "metadata"


def test_find_multiple_companions(tmp_path):
    """Detect multiple companion types for one media file."""
    media = tmp_path / "movie.mkv"
    srt = tmp_path / "movie.srt"
    nfo = tmp_path / "movie.nfo"
    cover = tmp_path / "cover.jpg"

    media.write_text("FAKE_VIDEO")
    srt.write_text("SUBTITLE")
    nfo.write_text("NFO")
    cover.write_text("JPEG")

    result = find_companions(str(media))
    assert len(result) == 3

    categories = {c.category for c in result}
    assert "subtitles" in categories
    assert "metadata" in categories
    assert "cover_art" in categories


def test_find_directory_companion_cover(tmp_path):
    """Detect cover.jpg directory companion."""
    media = tmp_path / "track01.mp3"
    cover = tmp_path / "cover.jpg"
    media.write_text("FAKE_AUDIO")
    cover.write_text("FAKE_JPEG")

    result = find_companions(str(media))
    assert len(result) == 1
    assert result[0].category == "cover_art"
    assert result[0].path == str(cover)


def test_find_directory_companion_folder_jpg(tmp_path):
    """Detect folder.jpg directory companion."""
    media = tmp_path / "song.flac"
    folder_art = tmp_path / "folder.jpg"
    media.write_text("FAKE_FLAC")
    folder_art.write_text("FAKE_JPEG")

    result = find_companions(str(media))
    assert len(result) == 1
    assert result[0].category == "cover_art"


def test_no_companions(tmp_path):
    """Return empty list when no companions exist."""
    media = tmp_path / "isolated.mp3"
    media.write_text("FAKE_AUDIO")

    result = find_companions(str(media))
    assert result == []


def test_no_companions_only_unrelated_files(tmp_path):
    """Ignore files that don't match companion patterns."""
    media = tmp_path / "song.mp3"
    unrelated = tmp_path / "readme.txt"
    media.write_text("FAKE_AUDIO")
    unrelated.write_text("Not a companion")

    result = find_companions(str(media))
    assert result == []


def test_companion_does_not_include_self(tmp_path):
    """The media file itself is never listed as its own companion."""
    media = tmp_path / "track.mp3"
    media.write_text("FAKE_AUDIO")

    result = find_companions(str(media))
    paths = [c.path for c in result]
    assert str(media) not in paths


def test_find_companions_invalid_path():
    """Return empty list for a non-existent file path."""
    result = find_companions("/nonexistent/path/file.mp3")
    assert result == []


def test_find_companions_none_input():
    """Return empty list for None input."""
    result = find_companions(None)
    assert result == []


def test_find_companions_empty_input():
    """Return empty list for empty string input."""
    result = find_companions("")
    assert result == []


def test_multiple_subtitle_formats(tmp_path):
    """Detect multiple subtitle formats for one media file."""
    media = tmp_path / "movie.mkv"
    srt = tmp_path / "movie.srt"
    ass = tmp_path / "movie.ass"
    vtt = tmp_path / "movie.vtt"

    media.write_text("VIDEO")
    srt.write_text("SRT")
    ass.write_text("ASS")
    vtt.write_text("VTT")

    result = find_companions(str(media))
    subtitle_companions = [c for c in result if c.category == "subtitles"]
    assert len(subtitle_companions) == 3


def test_companion_sorted_by_category(tmp_path):
    """Results are sorted by category then path."""
    media = tmp_path / "movie.mkv"
    srt = tmp_path / "movie.srt"
    lrc = tmp_path / "movie.lrc"
    cover = tmp_path / "cover.jpg"

    media.write_text("VIDEO")
    srt.write_text("SRT")
    lrc.write_text("LRC")
    cover.write_text("JPEG")

    result = find_companions(str(media))
    categories = [c.category for c in result]
    # Sorted alphabetically: cover_art, lyrics, subtitles
    assert categories == sorted(categories)


# ============================================================================
# compute_companion_destinations() tests
# ============================================================================

def test_compute_same_name_destination(tmp_path):
    """Same-name companion follows media file's new name."""
    old_media = str(tmp_path / "song.mp3")
    new_media = str(tmp_path / "output" / "New Song.mp3")
    companions = [CompanionFile(str(tmp_path / "song.lrc"), "lyrics")]

    result = compute_companion_destinations(companions, old_media, new_media)

    expected_path = os.path.join(str(tmp_path / "output"), "New Song.lrc")
    assert result[str(tmp_path / "song.lrc")] == expected_path


def test_compute_cover_art_destination(tmp_path):
    """Cover art companion follows media to new directory, keeps filename."""
    old_media = str(tmp_path / "song.mp3")
    new_media = str(tmp_path / "output" / "artist" / "song.mp3")
    companions = [CompanionFile(str(tmp_path / "cover.jpg"), "cover_art")]

    result = compute_companion_destinations(companions, old_media, new_media)

    expected_path = os.path.join(
        str(tmp_path / "output" / "artist"), "cover.jpg"
    )
    assert result[str(tmp_path / "cover.jpg")] == expected_path


def test_compute_multiple_destinations(tmp_path):
    """Multiple companions each get correct destinations."""
    old_media = str(tmp_path / "movie.mkv")
    new_media = str(tmp_path / "Movies" / "Action" / "Cool Movie.mkv")
    companions = [
        CompanionFile(str(tmp_path / "movie.srt"), "subtitles"),
        CompanionFile(str(tmp_path / "movie.nfo"), "metadata"),
        CompanionFile(str(tmp_path / "cover.jpg"), "cover_art"),
    ]

    result = compute_companion_destinations(companions, old_media, new_media)

    new_dir = str(tmp_path / "Movies" / "Action")
    assert result[str(tmp_path / "movie.srt")] == os.path.join(new_dir, "Cool Movie.srt")
    assert result[str(tmp_path / "movie.nfo")] == os.path.join(new_dir, "Cool Movie.nfo")
    assert result[str(tmp_path / "cover.jpg")] == os.path.join(new_dir, "cover.jpg")


def test_compute_empty_companions():
    """Empty companion list returns empty dict."""
    result = compute_companion_destinations([], "/old/file.mp3", "/new/file.mp3")
    assert result == {}


def test_compute_none_inputs():
    """None/missing inputs return empty dict."""
    assert compute_companion_destinations(None, "/old/f.mp3", "/new/f.mp3") == {}
    assert compute_companion_destinations([], None, "/new/f.mp3") == {}
    assert compute_companion_destinations([], "/old/f.mp3", None) == {}


# ============================================================================
# get_companion_summary() tests
# ============================================================================

def test_summary_no_companions():
    """Empty list returns 'None' string."""
    assert get_companion_summary([]) == "None"


def test_summary_single_category():
    """Single companion returns count and category."""
    companions = [CompanionFile("/path/movie.srt", "subtitles")]
    result = get_companion_summary(companions)
    assert result == "1 subtitles"


def test_summary_multiple_categories():
    """Multiple categories are listed with counts."""
    companions = [
        CompanionFile("/path/movie.srt", "subtitles"),
        CompanionFile("/path/movie.ass", "subtitles"),
        CompanionFile("/path/cover.jpg", "cover_art"),
    ]
    result = get_companion_summary(companions)
    assert "2 subtitles" in result
    assert "1 cover art" in result


def test_summary_humanizes_category_names():
    """Underscores in category names are replaced with spaces."""
    companions = [CompanionFile("/path/album.cue", "cue_sheets")]
    result = get_companion_summary(companions)
    assert "cue sheets" in result


# ============================================================================
# Extension category coverage tests
# ============================================================================

def test_companion_extensions_non_empty():
    """Each category has at least one extension defined."""
    for category, extensions in COMPANION_EXTENSIONS.items():
        assert len(extensions) > 0, f"Category '{category}' has no extensions"


def test_directory_companions_non_empty():
    """The directory companions list has entries."""
    assert len(DIRECTORY_COMPANIONS) > 0
