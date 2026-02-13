# ============================================================================
# File: /utils/mediainfo_loader.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Locates the libmediainfo native library for use with pymediainfo.
#
# pymediainfo is a Python wrapper that requires the libmediainfo shared
# library (.dylib on macOS, .dll on Windows, .so on Linux). This module
# resolves the library path across three scenarios:
#
#   1. Nuitka frozen build — looks alongside the compiled executable and
#      in a lib/ subdirectory next to it.
#   2. pip-installed pymediainfo wheel — modern wheels bundle the native
#      library inside the pymediainfo package directory (all platforms).
#   3. System-installed library — falls back to the OS search path
#      (e.g., brew install mediainfo, apt install libmediainfo-dev).
#
# The resolved path (or None for system fallback) is passed to
# MediaInfo.parse(library_file=...) in the metadata extractor.
# ============================================================================

import os                                              # Path operations
import sys                                             # Frozen/Nuitka detection
import logging                                         # Structured logging

logger = logging.getLogger("MeedyaManager.MediaInfoLoader")

# ============================================================================
# Platform-specific library filenames
# ============================================================================
# These are the filenames that pymediainfo and libmediainfo use by convention.
# Listed in order of preference (versioned names first for ABI stability).

_LIBRARY_FILENAMES = {
    "darwin": [                                        # macOS
        "libmediainfo.0.dylib",                        # Versioned (preferred)
        "libmediainfo.dylib",                          # Unversioned symlink
    ],
    "win32": [                                         # Windows (all editions)
        "MediaInfo.dll",                               # Standard DLL name
    ],
    "linux": [                                         # Linux (Debian, RHEL, Arch, etc.)
        "libmediainfo.so.0",                           # Versioned (preferred)
        "libmediainfo.so",                             # Unversioned symlink
    ],
}


def _is_frozen() -> bool:
    """Check whether we are running inside a Nuitka-compiled (frozen) build.

    Nuitka sets __nuitka_binary_dir on the main module and/or marks
    sys.frozen. PyInstaller also sets sys.frozen, so this covers both
    bundling tools.

    Returns:
        True if running from a compiled/frozen executable, False otherwise.
    """
    # Nuitka: sets __compiled__ attribute on the __main__ module
    if hasattr(sys, "frozen"):                         # PyInstaller / Nuitka --onefile
        return True
    if hasattr(sys.modules.get("__main__"), "__nuitka_binary_dir"):
        return True                                    # Nuitka --standalone
    return False


def _get_executable_dir() -> str:
    """Get the directory containing the running executable.

    For frozen builds, this is where the compiled binary lives.
    For regular Python, this returns the directory of the Python interpreter
    (not typically useful, but included for completeness).

    Returns:
        Absolute path to the directory containing the executable.
    """
    if hasattr(sys, "_MEIPASS"):                       # PyInstaller temp dir
        return sys._MEIPASS
    if hasattr(sys, "frozen"):                         # Nuitka / PyInstaller
        return os.path.dirname(sys.executable)
    # Nuitka --standalone: check __main__ for binary dir
    main_mod = sys.modules.get("__main__")
    if hasattr(main_mod, "__nuitka_binary_dir"):
        return main_mod.__nuitka_binary_dir
    return os.path.dirname(sys.executable)


def _get_platform_filenames() -> list[str]:
    """Get the list of library filenames for the current platform.

    Returns:
        List of candidate library filenames in preference order.
        Falls back to the Linux filenames for unknown platforms.
    """
    return _LIBRARY_FILENAMES.get(sys.platform, _LIBRARY_FILENAMES["linux"])


def _find_in_directory(directory: str, filenames: list[str]) -> str | None:
    """Search a directory for any of the candidate library filenames.

    Args:
        directory: Absolute path to the directory to search.
        filenames: List of candidate library filenames to look for.

    Returns:
        Absolute path to the first matching library file, or None.
    """
    for filename in filenames:
        candidate = os.path.join(directory, filename)
        if os.path.isfile(candidate):
            return candidate
    return None


def find_mediainfo_library() -> str | None:
    """Locate the libmediainfo native library.

    Searches in the following order:
      1. Nuitka/frozen executable directory (and lib/ subdirectory)
      2. pymediainfo pip package directory (wheels bundle the library)
      3. Returns None to let pymediainfo fall back to system paths

    The resolved path is intended to be passed to MediaInfo.parse() via
    the library_file parameter. A return value of None means "let
    pymediainfo use its own auto-detection" (which checks the package
    directory and then system paths).

    Returns:
        Absolute path to the library file, or None for system fallback.
    """
    filenames = _get_platform_filenames()

    # -----------------------------------------------------------------
    # Priority 1: Nuitka / PyInstaller frozen executable directory
    # -----------------------------------------------------------------
    # When packaged with Nuitka, the native library should be placed
    # alongside the executable or in a lib/ subdirectory. This takes
    # top priority because the bundled version is guaranteed to be
    # compatible with the bundled pymediainfo.
    if _is_frozen():
        exe_dir = _get_executable_dir()
        logger.debug(f"Frozen build detected — searching {exe_dir}")

        # Check directly alongside the executable
        found = _find_in_directory(exe_dir, filenames)
        if found:
            logger.info(f"Found bundled libmediainfo: {found}")
            return found

        # Check in a lib/ subdirectory (common Nuitka layout)
        lib_dir = os.path.join(exe_dir, "lib")
        found = _find_in_directory(lib_dir, filenames)
        if found:
            logger.info(f"Found bundled libmediainfo in lib/: {found}")
            return found

        logger.debug("Bundled library not found next to executable")

    # -----------------------------------------------------------------
    # Priority 2: pymediainfo pip package directory
    # -----------------------------------------------------------------
    # Modern pymediainfo wheels (especially on Windows and macOS) include
    # the native library inside the Python package. If pymediainfo is
    # installed via pip, the library may already be in its package dir.
    try:
        import pymediainfo as _pmi
        pkg_dir = os.path.dirname(_pmi.__file__)
        found = _find_in_directory(pkg_dir, filenames)
        if found:
            logger.debug(f"Found pip-bundled libmediainfo: {found}")
            return found
    except ImportError:
        logger.warning("pymediainfo package not installed")

    # -----------------------------------------------------------------
    # Priority 3: System fallback (return None)
    # -----------------------------------------------------------------
    # Returning None tells the caller to omit the library_file parameter,
    # letting pymediainfo use its built-in auto-detection which checks
    # system library paths (e.g., /usr/lib, Homebrew prefix, Windows PATH).
    logger.debug("No bundled library found — will use system libmediainfo")
    return None


def get_mediainfo_parse_kwargs() -> dict:
    """Get keyword arguments for MediaInfo.parse() with the resolved library.

    Convenience function that returns a dict suitable for unpacking into
    MediaInfo.parse(**kwargs). If a bundled library is found, the dict
    contains {"library_file": "/path/to/lib"}. Otherwise, returns an
    empty dict so the call proceeds with pymediainfo's default detection.

    Returns:
        Dict with "library_file" key if a bundled library was found,
        or empty dict for system fallback.

    Example:
        >>> from pymediainfo import MediaInfo
        >>> from utils.mediainfo_loader import get_mediainfo_parse_kwargs
        >>> mi = MediaInfo.parse("song.mp3", **get_mediainfo_parse_kwargs())
    """
    library_path = find_mediainfo_library()
    if library_path:
        return {"library_file": library_path}
    return {}
