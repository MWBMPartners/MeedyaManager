# ============================================================================
# File: /tests/test_classify_media_sanity.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Smoke test to ensure the `core.classify_media` module is resolvable
# and that `classify_media()` can be invoked with minimal metadata.
# ============================================================================

def test_classify_media_import():
    try:
        from core.classify_media import classify_media
    except ImportError as e:
        assert False, f"❌ Failed to import core.classify_media: {e}"


def test_classify_media_minimal():
    from core.classify_media import classify_media

    dummy_metadata = {
        "container": "mp4",
        "codec": "aac",
        "duration": 120,
        "bitrate": 320,
        "channels": 2,
    }

    result = classify_media(dummy_metadata)
    assert isinstance(result, dict), "❌ classify_media did not return a dictionary"
    assert "media_group" in result, "❌ Missing media_group in classification"
    assert "format_class" in result, "❌ Missing format_class in classification"