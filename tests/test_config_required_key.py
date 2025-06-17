# ============================================================================
# File: /tests/test_config_required_key.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
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
    value = get_config("simulate")
    assert isinstance(value, (bool, str, int, float, dict)), "Returned config value is not a supported type"

def test_get_config_unknown_key():
    with pytest.raises(KeyError):
        get_config("this_key_does_not_exist")
