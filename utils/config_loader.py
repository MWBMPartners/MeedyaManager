# ============================================================================
# File: /utils/config_loader.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Loads and caches the settings.json5 configuration file.
# Searches multiple standard locations for the config file.
# Includes optional fallback/default value support for missing keys.
# ============================================================================

import json5    # JSON5 parser that supports comments and trailing commas
import os       # File path resolution and existence checks

# Cached config dictionary (loaded once, reused on subsequent calls)
_config_cache = None

# Config search order: env override → root → config/ subdirectory
_CONFIG_SEARCH_PATHS = [
    os.environ.get("MEEDYAMANAGER_CONFIG", ""),       # User/CI override via env var
    "settings.json5",                                # Root directory (CI fallback)
    os.path.join("config", "settings.json5"),        # Standard config directory
]


def _find_config_file():
    """
    Locate the settings.json5 config file by searching standard locations.
    Returns the first path that exists, or raises FileNotFoundError.
    """
    for path in _CONFIG_SEARCH_PATHS:
        if path and os.path.isfile(path):
            return path
    raise FileNotFoundError(
        f"❌ Config file not found. Searched: {[p for p in _CONFIG_SEARCH_PATHS if p]}"
    )


def load_config():
    """
    Load and cache the JSON5 configuration file. Searches standard locations
    on first call, then returns the cached dictionary on subsequent calls.

    Returns:
        dict: The parsed configuration dictionary
    """
    global _config_cache
    if _config_cache is None:
        config_path = _find_config_file()
        with open(config_path, "r", encoding="utf-8") as f:
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
