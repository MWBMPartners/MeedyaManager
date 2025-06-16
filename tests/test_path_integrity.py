# ============================================================================
# File: /tests/test_path_integrity.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Smoke test to confirm internal module paths (PYTHONPATH) are respected.
# Ensures modules like core and utils are importable in CI environments.
# ============================================================================

def test_import_core_module():
    try:
        from core import metadata_extractor
    except ImportError as e:
        assert False, f"Failed to import core.metadata_extractor: {e}"


def test_import_utils_module():
    try:
        from utils import env_loader
    except ImportError as e:
        assert False, f"Failed to import utils.env_loader: {e}"
