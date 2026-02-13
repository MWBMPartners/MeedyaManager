# ============================================================================
# File: /core/renamer.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Processes file paths received from the watcher module, performs dry-run
# evaluations of their new names/locations based on rule templates, and
# logs the results. This module does not actually move/rename files — it
# evaluates and simulates the renaming process for testing and validation.
#
# M3 Update: Now supports two template syntaxes:
#   - New: MusicBee-style <Tag> and $Function() syntax (via rule_engine.py)
#   - Legacy: Simple {placeholder} syntax (auto-detected, deprecated)
#
# Future versions will support applying these changes, rollback, and history.
# ============================================================================

import os                                          # File path operations
import re                                          # Regex for legacy sanitization
import logging                                     # Structured logging
from utils.config_loader import get_config         # Config access
from utils.char_replacer import (
    sanitize_component,                            # Filename component sanitizer
    sanitize_path,                                 # Full path sanitizer
)
from core.rule_engine import (
    RuleEngine,                                    # MusicBee-style template engine
    TemplateSyntaxError,                           # Template syntax errors
    TemplateEvalError,                             # Template evaluation errors
)

# Setup log file directory
LOG_DIR = os.path.join("logs")                     # Log output directory
os.makedirs(LOG_DIR, exist_ok=True)                # Create if it doesn't exist
LOG_FILE = os.path.join(LOG_DIR, "rename_preview.log")

# Logger for console output
logger = logging.getLogger("MeedyaManager.Renamer")
logger.setLevel(logging.DEBUG)
handler = logging.StreamHandler()
formatter = logging.Formatter("[%(asctime)s] %(levelname)s - %(message)s")
handler.setFormatter(formatter)
logger.addHandler(handler)

# Logger for file logging (dry-run rename preview log)
file_logger = logging.getLogger("MeedyaManager.RenamerFile")
file_handler = logging.FileHandler(LOG_FILE, mode='a', encoding='utf-8')
file_handler.setFormatter(logging.Formatter("[%(asctime)s] FROM: %(message)s"))
file_logger.addHandler(file_handler)
file_logger.setLevel(logging.INFO)

# Set of characters that are unsafe in file/folder names across platforms
# Kept for backward compatibility with legacy {placeholder} syntax
UNSAFE_CHARS_PATTERN = re.compile(r'[<>:"/\\|?*\x00-\x1F]')

# Singleton rule engine instance (reused across calls for performance)
_rule_engine = RuleEngine()


def _is_legacy_template(template):
    """
    Detect whether a template uses legacy {placeholder} syntax or the new
    MusicBee-style <Tag>/$Function() syntax.

    Legacy templates contain '{' and '}' but no '<' or '$' function calls.
    If mixed, treat as new syntax (legacy placeholders won't be resolved).

    Args:
        template (str): The template string to check

    Returns:
        bool: True if the template uses legacy {placeholder} syntax
    """
    has_curly = '{' in template and '}' in template
    has_angle = '<' in template and '>' in template
    has_func = '$' in template

    # Legacy: has {placeholder} but no <Tag> or $Function()
    return has_curly and not has_angle and not has_func


def sanitize_filename_component(name):
    """
    Remove or replace characters in a string that are not safe for filenames.
    This helps prevent filesystem errors across platforms.
    Returns "Unknown" if the input is None or empty.

    Note: This function delegates to utils/char_replacer.sanitize_component()
    for configurable replacement. Kept as a public function for backward
    compatibility with other modules that import it.
    """
    return sanitize_component(name)


def simulate_rename(filepath, metadata):
    """
    Simulates renaming a file based on a metadata dictionary and a rule template.
    Returns the new proposed path without actually touching the filesystem.

    Supports two template syntaxes:
    - New (M3): MusicBee-style <Tag> and $Function() syntax
    - Legacy (M1): Simple {placeholder} syntax (auto-detected, deprecated)

    Args:
        filepath (str): Original file path
        metadata (dict): Extracted metadata tags (internal snake_case keys)

    Returns:
        str or None: Proposed new file path (simulated), or None on error
    """
    # Load template and fallback defaults from config
    template = get_config("rename_format")
    fallback = get_config("fallback_metadata", {})

    # Merge metadata with fallback defaults (metadata takes priority)
    combined = fallback.copy()
    combined.update(metadata)

    # Add file extension to metadata for use in template
    ext = os.path.splitext(filepath)[1].lstrip('.')
    combined['extension'] = ext                    # Used by both <Ext> and {extension}
    combined['ext'] = ext                          # Alias for backward compatibility

    # Add filename without extension (useful for <Filename> tag)
    if 'filename' not in combined:
        combined['filename'] = os.path.splitext(os.path.basename(filepath))[0]

    # Detect template syntax and evaluate accordingly
    if _is_legacy_template(template):
        # === LEGACY {placeholder} SYNTAX (deprecated) ===
        logger.warning(
            "Using deprecated {placeholder} template syntax. "
            "Please migrate to <Tag> syntax (see help/rule-syntax.md)."
        )
        relative_path = _evaluate_legacy_template(template, combined)
    else:
        # === NEW MusicBee-style <Tag>/$Function() SYNTAX ===
        relative_path = _evaluate_new_template(template, combined)

    # If evaluation failed, return None
    if relative_path is None:
        return None

    # Apply filename character replacement to the evaluated path
    relative_path = sanitize_path(relative_path)

    # Compute absolute path relative to original file's directory
    base_dir = os.path.dirname(filepath)
    new_path = os.path.normpath(os.path.join(base_dir, relative_path))

    # Log the simulated rename
    logger.info(f"Simulated rename: \n  FROM: {filepath}\n    TO: {new_path}")
    file_logger.info(f"{filepath}\n    TO: {new_path}")

    return new_path


def _evaluate_new_template(template, metadata):
    """
    Evaluate a MusicBee-style template using the rule engine.

    Args:
        template (str): Template with <Tag> and $Function() syntax
        metadata (dict): Metadata dictionary with internal snake_case keys

    Returns:
        str or None: Evaluated path string, or None on error
    """
    try:
        return _rule_engine.evaluate(template, metadata)
    except TemplateSyntaxError as e:
        logger.error(f"Template syntax error: {e}")
        return None
    except TemplateEvalError as e:
        logger.error(f"Template evaluation error: {e}")
        return None


def _evaluate_legacy_template(template, metadata):
    """
    Evaluate a legacy {placeholder} template using Python str.format().
    This preserves backward compatibility with M1/M2 templates.

    Args:
        template (str): Template with {placeholder} syntax
        metadata (dict): Metadata dictionary

    Returns:
        str or None: Evaluated path string, or None on error
    """
    # Sanitize all string values for safe filename use (legacy behavior)
    sanitized = {}
    for key, value in metadata.items():
        if isinstance(value, str):
            sanitized[key] = sanitize_filename_component(value)
        elif isinstance(value, (int, float)):
            sanitized[key] = str(value)            # Convert numbers to strings
        else:
            sanitized[key] = str(value) if value is not None else "Unknown"

    # Zero-pad track numbers (legacy behavior — in new syntax use $Pad)
    if 'track_num' in sanitized:
        sanitized['track_num'] = sanitized['track_num'].zfill(2)
    if 'track_number' in sanitized:
        sanitized['track_number'] = sanitized['track_number'].zfill(2)

    try:
        return template.format(**sanitized)
    except KeyError as e:
        logger.error(f"Missing required metadata tag: {e}")
        return None


if __name__ == '__main__':
    from core.metadata_extractor import extract_metadata

    dummy_file = "./watch_folder/test.mp3"
    dummy_metadata = extract_metadata(dummy_file)
    simulate_rename(dummy_file, dummy_metadata)
