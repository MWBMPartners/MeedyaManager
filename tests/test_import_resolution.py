# ============================================================================
# File: /tests/test_import_resolution.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Smoke test to confirm import paths for core and utils modules resolve correctly.
# This ensures CI environments have proper PYTHONPATH or directory context.
# ============================================================================

def test_import_core_module():
    try:
        from core import metadata_extractor
    except ImportError as e:
        assert False, f"❌ Failed to import from core: {e}"


def test_import_utils_module():
    try:
        from utils import env_loader
    except ImportError as e:
        assert False, f"❌ Failed to import from utils: {e}"


def test_import_nested_function():
    try:
        from core.metadata_extractor import extract_metadata
    except ImportError as e:
        assert False, f"❌ Failed to import nested function: {e}"
