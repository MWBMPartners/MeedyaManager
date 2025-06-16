# ============================================================================
# File: /tests/test_env_loader.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Verifies that environment variables are correctly loaded via utils.env_loader.
# This ensures fallback credentials or environment overrides are supported.
# ============================================================================

import os
import pytest
from utils.env_loader import load_env_variables


def test_env_file_detection(tmp_path):
    """
    Ensures that a .env file is properly detected and loaded.
    """
    # Create mock .env file in temp path
    env_path = tmp_path / ".env"
    env_path.write_text("TEST_VAR_FROM_ENV=12345\n")

    # Clear existing env if present
    os.environ.pop("TEST_VAR_FROM_ENV", None)

    # Load from mock .env
    load_env_variables(dotenv_path=str(env_path))

    # Assert variable was loaded
    assert os.getenv("TEST_VAR_FROM_ENV") == "12345"


def test_env_fallback_silent():
    """
    Verifies that load_env_variables() silently skips if no .env file exists.
    """
    result = load_env_variables(dotenv_path="/nonexistent/.env")
    assert result is None