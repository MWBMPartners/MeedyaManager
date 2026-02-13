# ============================================================================
# File: /utils/config_profile.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Configuration export/import for MeedyaManager.
#
# Exports the current settings as a portable .mmprofile ZIP bundle that
# can be transferred between platforms (macOS, Windows, Linux, RPi, etc.).
# On import, platform-specific paths are automatically adapted.
#
# .mmprofile format (ZIP archive):
#   manifest.json  — Version, platform, timestamp, profile name, schema_version
#   settings.json5 — Full config with paths tokenized for portability
#   env.template   — .env key names with blank values (always included)
#   env.secrets    — Actual API key values (only if user explicitly opts in)
#
# Path normalization tokens:
#   {HOME}      → User home directory
#   {DESKTOP}   → Desktop folder
#   {DOWNLOADS} → Downloads folder
#   {MUSIC}     → Music folder
#   {VIDEOS}    → Videos/Movies folder
#
# Usage:
#   from utils.config_profile import export_profile, import_profile
#
#   path = export_profile("/tmp/backup.mmprofile", profile_name="Home Mac")
#   changes = import_profile("/tmp/backup.mmprofile", mode="replace")
# ============================================================================

import os                                                  # Path and env operations
import sys                                                 # Platform detection
import json                                                # Manifest serialization
import copy                                                # Deep copy for merge
import zipfile                                             # .mmprofile ZIP handling
import logging                                             # Structured logging
from pathlib import Path                                   # Path operations
from datetime import datetime                              # Timestamps

logger = logging.getLogger("MeedyaManager.ConfigProfile")


# ============================================================================
# Platform Token Resolution
# ============================================================================

def get_platform_tokens() -> dict[str, str]:
    """
    Get the path token mapping for the current platform.

    Returns a dictionary that maps token names to their platform-specific
    absolute paths. Used during export (to tokenize) and import (to expand).

    Returns:
        dict: Token name → absolute path mapping.
    """
    home = str(Path.home())
    tokens = {
        "{HOME}": home,
        "{DESKTOP}": os.path.join(home, "Desktop"),
        "{DOWNLOADS}": os.path.join(home, "Downloads"),
    }

    # Music directory differs by platform
    if sys.platform == "darwin":
        tokens["{MUSIC}"] = os.path.join(home, "Music")
        tokens["{VIDEOS}"] = os.path.join(home, "Movies")
    elif sys.platform == "win32":
        tokens["{MUSIC}"] = os.path.join(home, "Music")
        tokens["{VIDEOS}"] = os.path.join(home, "Videos")
    else:
        tokens["{MUSIC}"] = os.path.join(home, "Music")
        tokens["{VIDEOS}"] = os.path.join(home, "Videos")

    return tokens


def normalize_paths_for_export(config: dict) -> dict:
    """
    Replace platform-specific paths with portable tokens.

    Walks the config dictionary recursively and replaces absolute paths
    that match known platform directories with their token equivalents.
    For example, ~/Music/Library becomes {MUSIC}/Library.

    Tokens are matched longest-first to prevent partial replacements.

    Args:
        config: The configuration dictionary to normalize.

    Returns:
        dict: A new config dict with paths replaced by tokens.
    """
    tokens = get_platform_tokens()
    # Sort by path length descending so longer paths match first
    sorted_tokens = sorted(tokens.items(), key=lambda t: len(t[1]), reverse=True)

    def _replace_in_value(value):
        if isinstance(value, str):
            for token, path in sorted_tokens:
                if path and path in value:
                    value = value.replace(path, token)
            return value
        elif isinstance(value, list):
            return [_replace_in_value(item) for item in value]
        elif isinstance(value, dict):
            return {k: _replace_in_value(v) for k, v in value.items()}
        return value

    return _replace_in_value(copy.deepcopy(config))


def expand_paths_for_import(config: dict) -> dict:
    """
    Replace portable tokens with platform-specific paths.

    The reverse of normalize_paths_for_export(). Walks the config
    dictionary and replaces tokens with the current platform's paths.

    Args:
        config: The configuration dictionary with tokens.

    Returns:
        dict: A new config dict with tokens expanded to platform paths.
    """
    tokens = get_platform_tokens()

    def _expand_in_value(value):
        if isinstance(value, str):
            for token, path in tokens.items():
                if token in value:
                    value = value.replace(token, path)
            return value
        elif isinstance(value, list):
            return [_expand_in_value(item) for item in value]
        elif isinstance(value, dict):
            return {k: _expand_in_value(v) for k, v in value.items()}
        return value

    return _expand_in_value(copy.deepcopy(config))


# ============================================================================
# Validation
# ============================================================================

def validate_profile(input_path: str | Path) -> list[str]:
    """
    Validate a .mmprofile ZIP bundle.

    Checks that the file is a valid ZIP archive containing the required
    manifest.json and settings.json5 files, and that the manifest is
    parseable.

    Args:
        input_path: Path to the .mmprofile file to validate.

    Returns:
        list[str]: List of validation error messages. Empty list = valid.
    """
    errors = []
    path = Path(input_path)

    if not path.exists():
        errors.append(f"File does not exist: {path}")
        return errors

    if not zipfile.is_zipfile(str(path)):
        errors.append("File is not a valid ZIP archive.")
        return errors

    try:
        with zipfile.ZipFile(str(path), "r") as zf:
            names = zf.namelist()

            if "manifest.json" not in names:
                errors.append("Missing manifest.json in profile.")

            if "settings.json5" not in names:
                errors.append("Missing settings.json5 in profile.")

            # Validate manifest JSON
            if "manifest.json" in names:
                try:
                    manifest_text = zf.read("manifest.json").decode("utf-8")
                    manifest = json.loads(manifest_text)
                    if "schema_version" not in manifest:
                        errors.append("Manifest missing schema_version.")
                except json.JSONDecodeError as e:
                    errors.append(f"Manifest is not valid JSON: {e}")

    except zipfile.BadZipFile as e:
        errors.append(f"Corrupt ZIP file: {e}")

    return errors


# ============================================================================
# Export
# ============================================================================

def export_profile(output_path: str | Path,
                   profile_name: str = "",
                   include_secrets: bool = False) -> str:
    """
    Export the current configuration as a .mmprofile ZIP bundle.

    Creates a portable ZIP archive containing:
      - manifest.json: Version, platform, timestamp, profile name
      - settings.json5: Full configuration with paths tokenized
      - env.template: .env key names with blank values
      - env.secrets: Actual API key values (only if include_secrets=True)

    Args:
        output_path:     Path for the output .mmprofile file.
        profile_name:    Human-readable name for the profile (e.g., "Home Mac").
        include_secrets: Whether to include actual API key values (default False).

    Returns:
        str: Absolute path to the created .mmprofile file.

    Raises:
        FileNotFoundError: If the config file cannot be found.
    """
    import json5 as json5_lib                              # JSON5 parser for config
    import platform                                        # Platform info for manifest

    from utils.config_loader import get_config_path

    output = Path(output_path)

    # Ensure the output has the .mmprofile extension
    if output.suffix != ".mmprofile":
        output = output.with_suffix(".mmprofile")

    # Read the raw config file
    config_path = get_config_path()
    with open(config_path, "r", encoding="utf-8") as f:
        config_text = f.read()

    # Parse the config for path normalization
    config_data = json5_lib.loads(config_text)

    # Normalize paths to portable tokens
    normalized = normalize_paths_for_export(config_data)

    # Build the manifest
    manifest = {
        "schema_version": 1,
        "profile_name": profile_name or "Unnamed Profile",
        "created_at": datetime.now().isoformat(),
        "platform": platform.platform(),
        "python_version": sys.version,
        "meedyamanager_version": _get_app_version(),
    }

    # Build the env.template (key names with blank values)
    env_template_lines = _build_env_template()

    # Build the env.secrets (actual values, if opted in)
    env_secrets_lines = ""
    if include_secrets:
        env_secrets_lines = _build_env_secrets()

    # Write the ZIP bundle
    with zipfile.ZipFile(str(output), "w", zipfile.ZIP_DEFLATED) as zf:
        zf.writestr("manifest.json", json.dumps(manifest, indent=2))
        zf.writestr("settings.json5", json.dumps(normalized, indent=2))
        zf.writestr("env.template", env_template_lines)
        if include_secrets and env_secrets_lines:
            zf.writestr("env.secrets", env_secrets_lines)

    logger.info(f"Exported profile to: {output}")
    return str(output.resolve())


# ============================================================================
# Import
# ============================================================================

def import_profile(input_path: str | Path,
                   mode: str = "replace",
                   dry_run: bool = False) -> dict:
    """
    Import a .mmprofile ZIP bundle into the current configuration.

    Reads the profile, expands platform tokens to local paths, and either
    replaces or merges the current configuration.

    Args:
        input_path: Path to the .mmprofile file to import.
        mode:       Import mode: "replace" (complete replacement) or
                    "merge" (imported values take priority, arrays merged).
        dry_run:    If True, returns the changes without applying them.

    Returns:
        dict with keys:
            - changes: dict of {key: {old: value, new: value}} pairs
            - applied: bool indicating whether changes were written
            - profile_name: Name of the imported profile

    Raises:
        ValueError: If the profile is invalid or the mode is unsupported.
    """
    import json5 as json5_lib

    from utils.config_loader import get_config_path, load_config, reload_config

    path = Path(input_path)

    # Validate the profile
    errors = validate_profile(path)
    if errors:
        raise ValueError(f"Invalid profile: {'; '.join(errors)}")

    # Read the profile contents
    with zipfile.ZipFile(str(path), "r") as zf:
        manifest = json.loads(zf.read("manifest.json").decode("utf-8"))
        imported_text = zf.read("settings.json5").decode("utf-8")
        imported_config = json.loads(imported_text)

    # Expand platform tokens to local paths
    expanded = expand_paths_for_import(imported_config)

    # Load the current config for comparison
    current = copy.deepcopy(load_config())

    # Compute changes
    if mode == "replace":
        new_config = expanded
    elif mode == "merge":
        new_config = _deep_merge(current, expanded)
    else:
        raise ValueError(f"Unsupported import mode: {mode}")

    # Build change summary
    changes = _compute_changes(current, new_config)

    result = {
        "changes": changes,
        "applied": False,
        "profile_name": manifest.get("profile_name", "Unknown"),
    }

    if dry_run:
        return result

    # Apply the changes by writing the new config
    config_path = get_config_path()
    with open(config_path, "w", encoding="utf-8") as f:
        json.dump(new_config, f, indent=2)

    # Reload the config cache
    reload_config()

    result["applied"] = True
    logger.info(
        f"Imported profile '{result['profile_name']}' in {mode} mode "
        f"({len(changes)} changes)"
    )
    return result


def preview_import(input_path: str | Path) -> dict:
    """
    Preview what would change if a profile were imported.

    Convenience function that calls import_profile with dry_run=True.

    Args:
        input_path: Path to the .mmprofile file.

    Returns:
        dict: Same as import_profile return value, with applied=False.
    """
    return import_profile(input_path, dry_run=True)


# ============================================================================
# Internal Helpers
# ============================================================================

def _get_app_version() -> str:
    """Get the MeedyaManager version string."""
    try:
        import importlib
        cli_mod = importlib.import_module("cli")
        return getattr(cli_mod, "__version__", "unknown")
    except Exception:
        return "unknown"


def _build_env_template() -> str:
    """
    Build an env.template file with known API key names and blank values.

    Returns:
        str: .env-style template with blank values for each known key.
    """
    known_keys = [
        "APPLE_MUSIC_TEAM_ID",
        "APPLE_MUSIC_KEY_ID",
        "APPLE_MUSIC_PRIVATE_KEY",
        "SPOTIFY_CLIENT_ID",
        "SPOTIFY_CLIENT_SECRET",
        "TIDAL_CLIENT_ID",
        "TIDAL_CLIENT_SECRET",
        "YOUTUBE_HEADERS_AUTH",
        "TMDB_API_KEY",
        "TVDB_API_KEY",
        "EIDR_CLIENT_ID",
        "EIDR_CLIENT_SECRET",
    ]
    lines = [
        "# MeedyaManager API Keys Template",
        "# Fill in your API keys and rename this file to .env",
        "",
    ]
    for key in known_keys:
        lines.append(f"{key}=")
    return "\n".join(lines)


def _build_env_secrets() -> str:
    """
    Build an env.secrets file with actual API key values from the environment.

    Only includes keys that have non-empty values.

    Returns:
        str: .env-style file with actual key values.
    """
    known_keys = [
        "APPLE_MUSIC_TEAM_ID",
        "APPLE_MUSIC_KEY_ID",
        "APPLE_MUSIC_PRIVATE_KEY",
        "SPOTIFY_CLIENT_ID",
        "SPOTIFY_CLIENT_SECRET",
        "TIDAL_CLIENT_ID",
        "TIDAL_CLIENT_SECRET",
        "YOUTUBE_HEADERS_AUTH",
        "TMDB_API_KEY",
        "TVDB_API_KEY",
        "EIDR_CLIENT_ID",
        "EIDR_CLIENT_SECRET",
    ]
    lines = [
        "# MeedyaManager API Keys (CONFIDENTIAL)",
        "# This file contains actual API key values.",
        "",
    ]
    for key in known_keys:
        value = os.environ.get(key, "")
        if value:
            lines.append(f"{key}={value}")
    return "\n".join(lines)


def _deep_merge(base: dict, overlay: dict) -> dict:
    """
    Deep merge two dictionaries. Overlay values take priority.

    - Dicts are merged recursively
    - Lists are merged by union (overlay items appended if not present)
    - Scalars from overlay replace base values

    Args:
        base:    The base dictionary.
        overlay: The overlay dictionary (takes priority).

    Returns:
        dict: The merged result (new dict, inputs are not modified).
    """
    result = copy.deepcopy(base)

    for key, value in overlay.items():
        if key in result and isinstance(result[key], dict) and isinstance(value, dict):
            result[key] = _deep_merge(result[key], value)
        elif key in result and isinstance(result[key], list) and isinstance(value, list):
            # Union merge for lists: add new items from overlay
            for item in value:
                if item not in result[key]:
                    result[key].append(item)
        else:
            result[key] = copy.deepcopy(value)

    return result


def _compute_changes(old: dict, new: dict, prefix: str = "") -> dict:
    """
    Compute a diff between two config dictionaries.

    Returns a flat dictionary of changed keys with their old and new values.

    Args:
        old:    The old configuration dictionary.
        new:    The new configuration dictionary.
        prefix: Key prefix for nested keys (used in recursion).

    Returns:
        dict: Mapping of key paths to {old: value, new: value} dicts.
    """
    changes = {}
    all_keys = set(list(old.keys()) + list(new.keys()))

    for key in all_keys:
        full_key = f"{prefix}.{key}" if prefix else key
        old_val = old.get(key)
        new_val = new.get(key)

        if old_val == new_val:
            continue

        if isinstance(old_val, dict) and isinstance(new_val, dict):
            # Recurse into nested dicts
            sub_changes = _compute_changes(old_val, new_val, full_key)
            changes.update(sub_changes)
        else:
            changes[full_key] = {"old": old_val, "new": new_val}

    return changes
