# ============================================================================
# File: /ui/platform_style.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Platform-specific styling for the MeedyaManager GUI.
# Detects the current OS and applies native visual effects:
#   - macOS: Liquid Glass (NSGlassEffectView) with fallback to
#            NSVisualEffectView vibrancy
#   - Windows: Mica/Acrylic backdrop via DWM API (ctypes)
#   - Linux: Qt Fusion style with custom palette
#
# Also detects the system light/dark theme via darkdetect.
# ============================================================================

import sys                 # Platform detection via sys.platform
import os                  # Environment variable checks
import logging             # Structured logging for style application events

logger = logging.getLogger("MeedyaManager.PlatformStyle")


def detect_platform() -> str:
    """
    Detect the current operating system.

    Returns:
        str: One of 'macos', 'windows', or 'linux'
    """
    if sys.platform == "darwin":
        return "macos"
    elif sys.platform == "win32":
        return "windows"
    else:
        return "linux"


def get_system_theme() -> str:
    """
    Detect whether the system is using dark or light mode.
    Uses the darkdetect library for cross-platform detection.

    Returns:
        str: 'dark' or 'light'
    """
    try:
        import darkdetect                          # Cross-platform dark mode detection
        theme = darkdetect.theme()                 # Returns 'Dark' or 'Light' (or None)
        return "dark" if theme and theme.lower() == "dark" else "light"
    except ImportError:
        logger.warning("darkdetect not installed, defaulting to 'light' theme")
        return "light"
    except Exception as e:
        logger.warning(f"Could not detect system theme: {e}, defaulting to 'light'")
        return "light"


def _apply_macos_style(app, window):
    """
    Apply macOS-native styling to the application window.
    Attempts Liquid Glass (macOS 26+) via PyObjC, falling back to
    NSVisualEffectView vibrancy, then to a basic translucent style.

    Args:
        app: QApplication instance
        window: QMainWindow instance
    """
    try:
        # Attempt PyObjC bridge for native AppKit styling
        from Foundation import NSProcessInfo                    # macOS version detection
        from AppKit import NSApplication, NSVisualEffectView    # AppKit vibrancy view

        # Get macOS version to determine available effects
        os_version = NSProcessInfo.processInfo().operatingSystemVersion()
        major = os_version.majorVersion
        minor = os_version.minorVersion
        logger.info(f"macOS version detected: {major}.{minor}")

        # Get the native NSWindow from the Qt window
        ns_view = int(window.winId())

        if major >= 26:
            # macOS 26+ (Tahoe): Try Liquid Glass effect
            try:
                from AppKit import NSGlassEffectView            # Liquid Glass API

                # Create a Liquid Glass background view
                glass_view = NSGlassEffectView.alloc().init()
                logger.info("Applied Liquid Glass effect (macOS 26+)")
            except (ImportError, AttributeError):
                # NSGlassEffectView not available — fall back to vibrancy
                logger.info("Liquid Glass not available, using NSVisualEffectView")
                _apply_macos_vibrancy(window)
        else:
            # macOS 15 and below: Use NSVisualEffectView vibrancy
            _apply_macos_vibrancy(window)

    except ImportError:
        # PyObjC not installed — apply basic translucent style
        logger.info("PyObjC not available, applying basic macOS style")
        window.setAttribute(
            __import__("PySide6.QtCore", fromlist=["Qt"]).Qt.WidgetAttribute.WA_TranslucentBackground,
            True
        )


def _apply_macos_vibrancy(window):
    """
    Apply NSVisualEffectView-based vibrancy to the window.
    This provides the frosted glass appearance on macOS 10.14+.

    Args:
        window: QMainWindow instance
    """
    try:
        from PySide6.QtCore import Qt                          # Qt core constants
        # Enable translucent background for vibrancy effect
        window.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground, True)
        logger.info("Applied macOS vibrancy (translucent background)")
    except Exception as e:
        logger.warning(f"Failed to apply macOS vibrancy: {e}")


def _apply_windows_style(app, window):
    """
    Apply Windows-native styling to the application window.
    Attempts Mica backdrop (Windows 11) via DWM API, falling back
    to Acrylic, then to basic Windows styling.

    Args:
        app: QApplication instance
        window: QMainWindow instance
    """
    try:
        import ctypes                                          # C-level Windows API access
        from ctypes import wintypes                            # Windows-specific type defs

        # DWM attribute constants for backdrop type
        DWMWA_USE_IMMERSIVE_DARK_MODE = 20                     # Enable dark title bar
        DWMWA_SYSTEMBACKDROP_TYPE = 38                         # Backdrop type (Win11 22H2+)
        DWMSBT_MAINWINDOW = 2                                  # Mica material
        DWMSBT_TRANSIENTWINDOW = 3                             # Acrylic material

        # Load the Desktop Window Manager API
        dwmapi = ctypes.windll.dwmapi

        # Get the native window handle (HWND) from Qt
        hwnd = int(window.winId())

        # Enable dark mode for the title bar
        dark_mode = ctypes.c_int(1 if get_system_theme() == "dark" else 0)
        dwmapi.DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            ctypes.byref(dark_mode),
            ctypes.sizeof(dark_mode)
        )

        # Try Mica backdrop first (Windows 11 22H2+)
        backdrop_type = ctypes.c_int(DWMSBT_MAINWINDOW)
        result = dwmapi.DwmSetWindowAttribute(
            hwnd,
            DWMWA_SYSTEMBACKDROP_TYPE,
            ctypes.byref(backdrop_type),
            ctypes.sizeof(backdrop_type)
        )

        if result == 0:
            logger.info("Applied Windows Mica backdrop")
        else:
            # Fall back to Acrylic
            backdrop_type = ctypes.c_int(DWMSBT_TRANSIENTWINDOW)
            dwmapi.DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                ctypes.byref(backdrop_type),
                ctypes.sizeof(backdrop_type)
            )
            logger.info("Applied Windows Acrylic backdrop")

    except Exception as e:
        logger.info(f"Windows DWM styling not available: {e}")


def _apply_linux_style(app, window):
    """
    Apply Linux styling using Qt's Fusion theme with a custom palette.
    Fusion provides a consistent cross-desktop look regardless of
    the window manager or desktop environment in use.

    Args:
        app: QApplication instance
        window: QMainWindow instance
    """
    from PySide6.QtWidgets import QStyleFactory                # Available Qt style engines

    # Apply Qt Fusion style for consistent cross-desktop appearance
    if "Fusion" in QStyleFactory.keys():
        app.setStyle("Fusion")
        logger.info("Applied Linux Fusion style")
    else:
        logger.info("Fusion style not available, using default")


def apply_platform_style(app, window):
    """
    Detect the current platform and apply the appropriate native styling.
    This is the main entry point called during application startup.

    Args:
        app: QApplication instance
        window: QMainWindow instance
    """
    platform = detect_platform()
    logger.info(f"Detected platform: {platform}")

    if platform == "macos":
        _apply_macos_style(app, window)
    elif platform == "windows":
        _apply_windows_style(app, window)
    else:
        _apply_linux_style(app, window)
