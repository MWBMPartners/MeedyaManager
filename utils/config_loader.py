# ============================================================================
# File: /utils/config_loader.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Loads and parses the `settings.json5` configuration file for use across the
# MetaMancer application. This utility provides a centralized way to read global
# settings and apply defaults where necessary.
#
# The config file supports JSON5 for readability and may be extended later
# with schema validation or GUI editing.
#
# Dependencies:
# - json5: pip install json5
#
# References:
# https://pypi.org/project/json5/
# https://json5.org/
# ============================================================================

import json5
import os
import logging

CONFIG_PATH = os.path.join("config", "settings.json5")
logger = logging.getLogger("MetaMancer.Config")

# --- Global Config Dictionary ---
CONFIG = {}


def load_config():
    """
    Loads the settings.json5 configuration into the global CONFIG dictionary.

    Returns:
        dict: Parsed configuration
    """
    global CONFIG

    if not os.path.exists(CONFIG_PATH):
        logger.error(f"Configuration file not found at: {CONFIG_PATH}")
        raise FileNotFoundError(CONFIG_PATH)

    try:
        with open(CONFIG_PATH, "r", encoding="utf-8") as f:
            CONFIG = json5.load(f)
            logger.info(f"Loaded configuration from {CONFIG_PATH}")
    except Exception as e:
        logger.exception(f"Failed to load config: {e}")
        raise e

    return CONFIG


def get_config(key, default=None):
    """
    Retrieves a config value from the global CONFIG dictionary.

    Args:
        key (str): The key to retrieve (dot-notation not yet supported)
        default (Any): Fallback value if key is missing

    Returns:
        value or default
    """
    return CONFIG.get(key, default)


if __name__ == '__main__':
    cfg = load_config()
    print("\nLoaded Config Keys:")
    for k in cfg:
        print(f" - {k}: {cfg[k]}")