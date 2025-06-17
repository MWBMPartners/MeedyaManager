# ============================================================================
# File: /utils/config_loader.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Loads and caches the settings.json5 configuration file.
# Now includes optional fallback/default value support for missing keys.
# ============================================================================

import json5
import os

_config_cache = None

CONFIG_FILE_PATH = os.environ.get("MEDIAMANCER_CONFIG", "settings.json5")


def load_config():
    global _config_cache
    if _config_cache is None:
        if not os.path.isfile(CONFIG_FILE_PATH):
            raise FileNotFoundError(f"❌ Config file not found: {CONFIG_FILE_PATH}")
        with open(CONFIG_FILE_PATH, "r", encoding="utf-8") as f:
            _config_cache = json5.load(f)
    return _config_cache


def get_config(key, default=None):
    """
    Retrieves a config value by key. Returns `default` if provided and key is missing.

    Args:
        key (str): The config key to look up.
        default (Any, optional): A default value to return if the key is missing.

    Returns:
        Any: The config value

    Raises:
        KeyError: If the key is missing and no default is provided
    """
    config = load_config()
    if key in config:
        return config[key]
    if default is not None:
        return default
    raise KeyError(f"❌ Key '{key}' not found in config and no default provided")