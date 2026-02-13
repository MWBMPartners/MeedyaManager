# ============================================================================
# File: /tests/test_config_required_key.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for `get_config()` from utils.config_loader:
# - Ensures missing key raises TypeError
# - Ensures known key returns expected value
# - Ensures unknown key raises KeyError
# ============================================================================

import pytest
from utils.config_loader import get_config

def test_get_config_requires_key():
    with pytest.raises(TypeError):
        get_config()

def test_get_config_known_key():
    # Test with a key that actually exists in config/settings.json5
    value = get_config("watch_paths")
    assert isinstance(value, list), "watch_paths should return a list"

def test_get_config_unknown_key():
    with pytest.raises(KeyError):
        get_config("this_key_does_not_exist")
