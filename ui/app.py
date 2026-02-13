# ============================================================================
# File: /ui/app.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Application entry point for the MeedyaManager GUI.
# Creates the QApplication, applies platform-specific styling and theme,
# and launches the main window.
#
# This module is called by the CLI `gui` command or run directly.
# ============================================================================

import sys                                                  # System exit codes
import os                                                   # Path operations for theme files
import logging                                              # Structured logging

from PySide6.QtWidgets import QApplication                  # Qt application instance
from PySide6.QtCore import Qt                               # Core constants

from ui.main_window import MainWindow                       # Main application window
from ui.platform_style import (
    apply_platform_style,                                   # Native OS styling
    get_system_theme,                                       # Dark/light mode detection
)

logger = logging.getLogger("MeedyaManager.App")

# Path to the themes directory — resolved relative to this file
THEMES_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "themes")


def _load_theme_stylesheet(theme_name: str) -> str:
    """
    Load a Qt stylesheet (.qss) from the themes directory.

    Args:
        theme_name: 'dark' or 'light'

    Returns:
        str: Contents of the .qss file, or empty string on error
    """
    qss_path = os.path.join(THEMES_DIR, f"{theme_name}.qss")
    try:
        with open(qss_path, "r", encoding="utf-8") as f:
            return f.read()
    except FileNotFoundError:
        logger.warning(f"Theme stylesheet not found: {qss_path}")
        return ""
    except Exception as e:
        logger.warning(f"Failed to load theme {theme_name}: {e}")
        return ""


def launch_gui():
    """
    Launch the MeedyaManager GUI application.

    Creates the QApplication, detects system theme, loads the appropriate
    stylesheet, applies platform-specific native styling, and shows
    the main window.

    Returns:
        int: Application exit code (0 = success)
    """
    # Create the Qt application instance
    app = QApplication(sys.argv)
    app.setApplicationName("MeedyaManager")
    app.setOrganizationName("MWBM Partners Ltd")
    app.setOrganizationDomain("mwbmpartners.ltd")
    app.setDesktopFileName("ltd.mwbmpartners.meedyamanager")

    # Detect system theme (dark or light) and load matching stylesheet
    theme = get_system_theme()
    logger.info(f"System theme detected: {theme}")

    stylesheet = _load_theme_stylesheet(theme)
    if stylesheet:
        app.setStyleSheet(stylesheet)
        logger.info(f"Applied {theme} theme stylesheet")

    # Create the main window
    window = MainWindow()

    # Apply platform-specific native styling (Liquid Glass, Mica, Fusion)
    apply_platform_style(app, window)

    # Show the main window
    window.show()

    logger.info("MeedyaManager GUI launched")

    # Enter the Qt event loop — blocks until the application quits
    return app.exec()


# Allow running directly: python -m ui.app
if __name__ == "__main__":
    sys.exit(launch_gui())
