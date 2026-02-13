# ============================================================================
# File: /utils/error_messages.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Error message catalog for MeedyaManager.
# Maps exception types and context strings to user-friendly messages
# with a headline, explanation, and suggested action.
#
# This module is used by ui/error_dialog.py to present errors in a way
# that non-technical users can understand and act on.
#
# Usage:
#   from utils.error_messages import get_user_friendly_message
#   headline, explanation, suggestion = get_user_friendly_message(exception, "scan")
# ============================================================================

from dataclasses import dataclass                          # Structured error message tuples


@dataclass(frozen=True)
class ErrorMessage:
    """
    A user-friendly error message with three components.

    Attributes:
        headline:    Short title describing what went wrong (shown prominently).
        explanation: Human-readable explanation of the problem.
        suggestion:  Actionable steps the user can take to resolve the issue.
    """
    headline: str
    explanation: str
    suggestion: str


# ============================================================================
# Error Catalog
# ============================================================================
# Each entry maps (exception_type_name, context) to a user-friendly message.
# The context string narrows the message to the specific operation that failed.
# If no context-specific match is found, a type-only match is tried.
# If no type match is found, a generic fallback is returned.
# ============================================================================

_ERROR_CATALOG: dict[tuple[str, str], ErrorMessage] = {

    # --- File System Errors ---

    ("FileNotFoundError", "scan"): ErrorMessage(
        headline="Media file not found",
        explanation=(
            "One or more files could not be found during the scan. "
            "The file may have been moved, renamed, or deleted by another program."
        ),
        suggestion="Check that the watch folders still exist and try scanning again.",
    ),

    ("FileNotFoundError", "metadata"): ErrorMessage(
        headline="File not found",
        explanation=(
            "The file could not be found when trying to read its metadata. "
            "It may have been moved or deleted since the last scan."
        ),
        suggestion="Rescan the watch folders to refresh the file list.",
    ),

    ("PermissionError", ""): ErrorMessage(
        headline="Permission denied",
        explanation=(
            "MeedyaManager does not have permission to access one or more files "
            "or directories. This often happens with protected system folders "
            "or files owned by another user."
        ),
        suggestion=(
            "Check the file/folder permissions in your operating system's settings. "
            "On macOS, you may need to grant Full Disk Access in System Settings > "
            "Privacy & Security."
        ),
    ),

    ("OSError", ""): ErrorMessage(
        headline="File system error",
        explanation=(
            "An error occurred while accessing the file system. This could be "
            "caused by a full disk, a disconnected drive, or a network share "
            "that is no longer available."
        ),
        suggestion=(
            "Check that the drive has available space and that any network "
            "drives are connected, then try again."
        ),
    ),

    # --- Metadata Errors ---

    ("TagWriteError", ""): ErrorMessage(
        headline="Could not write metadata tags",
        explanation=(
            "MeedyaManager was unable to write the updated tags to the file. "
            "The file may be read-only, in use by another program, or the "
            "file format may not support the tags you are trying to write."
        ),
        suggestion=(
            "Check that the file is not open in another application (e.g., a "
            "media player) and that the file is not read-only."
        ),
    ),

    ("UnsupportedFormatError", ""): ErrorMessage(
        headline="Unsupported file format",
        explanation=(
            "This file's format does not support embedded metadata editing. "
            "Some container formats (like MKV or AVI) have limited tag support."
        ),
        suggestion=(
            "Try converting the file to a format with full tag support (such as "
            "MP3, FLAC, or M4A for audio, or MP4/MKV with Matroska tags for video)."
        ),
    ),

    # --- Template / Rule Errors ---

    ("TemplateSyntaxError", ""): ErrorMessage(
        headline="Invalid rule template",
        explanation=(
            "The rename rule template contains a syntax error. This usually "
            "means there is an unclosed tag, a misspelled function name, or "
            "mismatched brackets."
        ),
        suggestion=(
            "Open the Rules tab and check your template for typos. "
            "Refer to Help > Rule Syntax for the correct format."
        ),
    ),

    ("TemplateEvalError", ""): ErrorMessage(
        headline="Rule evaluation failed",
        explanation=(
            "The rename rule could not be applied to this file. A required "
            "metadata tag may be missing, or a function received an invalid argument."
        ),
        suggestion=(
            "Check that the file has the metadata tags your rule expects "
            "(artist, album, track number, etc.). You can inspect a file's "
            "tags in the Metadata tab."
        ),
    ),

    # --- Network / Lookup Errors ---

    ("ConnectionError", "lookup"): ErrorMessage(
        headline="Could not connect to metadata provider",
        explanation=(
            "MeedyaManager was unable to reach one or more online metadata "
            "providers. This is usually caused by a network connectivity issue."
        ),
        suggestion=(
            "Check your internet connection and try again. If you are behind "
            "a firewall or proxy, ensure that outgoing HTTPS connections are allowed."
        ),
    ),

    ("TimeoutError", "lookup"): ErrorMessage(
        headline="Metadata lookup timed out",
        explanation=(
            "The request to one or more metadata providers took too long to "
            "respond. The service may be temporarily overloaded or unreachable."
        ),
        suggestion="Wait a moment and try the lookup again.",
    ),

    ("ConnectionError", ""): ErrorMessage(
        headline="Network connection error",
        explanation=(
            "A network request failed. This could be caused by a lost internet "
            "connection, DNS failure, or a firewall blocking the request."
        ),
        suggestion="Check your internet connection and try again.",
    ),

    # --- Configuration Errors ---

    ("KeyError", "config"): ErrorMessage(
        headline="Missing configuration value",
        explanation=(
            "A required configuration setting could not be found. The settings "
            "file may be incomplete or corrupted."
        ),
        suggestion=(
            "Open Settings and verify all fields are filled in correctly. "
            "If the problem persists, try resetting settings to defaults."
        ),
    ),

    ("ValueError", "config"): ErrorMessage(
        headline="Invalid configuration value",
        explanation=(
            "A configuration setting has an invalid value. This can happen if "
            "the settings file was edited manually with an incorrect format."
        ),
        suggestion=(
            "Open Settings and correct any invalid values. Numbers must be "
            "numeric, paths must be valid directories, and extensions must "
            "start with a dot (e.g., .mp3)."
        ),
    ),

    # --- Worker / Thread Errors ---

    ("RuntimeError", "worker"): ErrorMessage(
        headline="Background task failed",
        explanation=(
            "A background operation encountered an unexpected error and could "
            "not complete. This is usually caused by an internal issue."
        ),
        suggestion=(
            "Try the operation again. If the problem persists, check the "
            "application log files for more details."
        ),
    ),
}

# Type-only entries (no context) serve as fallbacks when a specific
# (type, context) pair is not found. Built from entries where context is "".
_TYPE_FALLBACKS: dict[str, ErrorMessage] = {
    exc_type: msg
    for (exc_type, context), msg in _ERROR_CATALOG.items()
    if context == ""
}

# Generic fallback message for completely unknown exceptions
_GENERIC_FALLBACK = ErrorMessage(
    headline="An unexpected error occurred",
    explanation=(
        "MeedyaManager encountered an error it did not expect. This may "
        "be caused by a bug in the application or an unusual system "
        "configuration."
    ),
    suggestion=(
        "Try the operation again. If the problem persists, you can help "
        "us fix it by submitting an error report from Help > Report Bug."
    ),
)


# ============================================================================
# Public API
# ============================================================================

def get_user_friendly_message(exception: Exception,
                               context: str = "") -> ErrorMessage:
    """
    Look up a user-friendly error message for an exception.

    Resolution order:
      1. Exact match on (exception_type_name, context)
      2. Walk the MRO (parent classes) with the same context
      3. Exact match on (exception_type_name, "") — type-only fallback
      4. Walk the MRO with empty context
      5. Generic fallback message

    Args:
        exception: The exception instance to look up.
        context:   A string describing the operation that failed
                   (e.g., "scan", "lookup", "metadata", "config", "worker").

    Returns:
        An ErrorMessage dataclass with headline, explanation, and suggestion.
    """
    exc_type = type(exception)

    # Walk the MRO for context-specific match
    if context:
        for cls in exc_type.__mro__:
            key = (cls.__name__, context)
            if key in _ERROR_CATALOG:
                return _ERROR_CATALOG[key]

    # Walk the MRO for type-only match
    for cls in exc_type.__mro__:
        name = cls.__name__
        if name in _TYPE_FALLBACKS:
            return _TYPE_FALLBACKS[name]

    return _GENERIC_FALLBACK
