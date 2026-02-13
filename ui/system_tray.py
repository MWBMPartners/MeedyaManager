# ============================================================================
# File: /ui/system_tray.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# System tray icon for the MeedyaManager GUI.
# Provides a persistent tray icon with context menu for quick access to:
#   - Show/Hide main window
#   - Start/Stop file watcher
#   - Trigger manual scan
#   - Quit the application
#
# Supports minimize-to-tray behavior (configurable).
# ============================================================================

import os                                                   # Path operations for icon
import logging                                              # Structured logging

from PySide6.QtCore import Signal                           # Custom signal definitions
from PySide6.QtGui import QIcon, QAction                    # Icon and menu action
from PySide6.QtWidgets import (
    QSystemTrayIcon,                                        # System tray icon widget
    QMenu,                                                  # Context menu
)

logger = logging.getLogger("MeedyaManager.SystemTray")

# Path to the application icon — resolved relative to the project root
ICON_PATH = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "assets", "icon.png"
)


class SystemTrayIcon(QSystemTrayIcon):
    """
    System tray icon with context menu for MeedyaManager.
    Allows the user to control the application from the system tray
    without the main window being visible.

    Signals:
        show_requested: Emitted when user clicks "Show/Hide" in tray menu
        scan_requested: Emitted when user clicks "Scan Now" in tray menu
        watch_toggled(bool): Emitted when user toggles the watcher state
        quit_requested: Emitted when user clicks "Quit" in tray menu
    """

    # Custom signals for tray menu actions
    show_requested = Signal()
    scan_requested = Signal()
    watch_toggled = Signal(bool)
    quit_requested = Signal()

    def __init__(self, parent=None):
        """
        Initialize the system tray icon with context menu.

        Args:
            parent: Parent QObject (typically the main window)
        """
        super().__init__(parent)

        # Set the tray icon (falls back to a default if icon file not found)
        if os.path.exists(ICON_PATH):
            self.setIcon(QIcon(ICON_PATH))
        else:
            # Use a built-in Qt icon as fallback
            from PySide6.QtWidgets import QApplication
            self.setIcon(QApplication.style().standardIcon(
                QApplication.style().StandardPixmap.SP_MediaPlay
            ))
            logger.info(f"Tray icon not found at {ICON_PATH}, using default")

        self.setToolTip("MeedyaManager")

        # Track watcher state for toggle action
        self._watcher_running = False

        # Build the context menu
        self._setup_menu()

        # Double-click on tray icon shows the main window
        self.activated.connect(self._on_activated)

    def _setup_menu(self):
        """Create the system tray context menu with all actions."""
        menu = QMenu()

        # Show/Hide main window action
        self._show_action = QAction("Show MeedyaManager", self)
        self._show_action.triggered.connect(self.show_requested.emit)
        menu.addAction(self._show_action)

        menu.addSeparator()

        # Scan Now action — triggers a manual scan
        scan_action = QAction("Scan Now", self)
        scan_action.triggered.connect(self.scan_requested.emit)
        menu.addAction(scan_action)

        # Start/Stop Watcher toggle action
        self._watch_action = QAction("Start Watcher", self)
        self._watch_action.triggered.connect(self._toggle_watcher)
        menu.addAction(self._watch_action)

        menu.addSeparator()

        # Quit action — exits the application entirely
        quit_action = QAction("Quit", self)
        quit_action.triggered.connect(self.quit_requested.emit)
        menu.addAction(quit_action)

        self.setContextMenu(menu)

    def _on_activated(self, reason):
        """
        Handle tray icon activation events (click, double-click).
        Double-click toggles the main window visibility.

        Args:
            reason: QSystemTrayIcon.ActivationReason enum value
        """
        if reason == QSystemTrayIcon.ActivationReason.DoubleClick:
            self.show_requested.emit()

    def _toggle_watcher(self):
        """Toggle the watcher running state and update the menu text."""
        self._watcher_running = not self._watcher_running
        if self._watcher_running:
            self._watch_action.setText("Stop Watcher")
        else:
            self._watch_action.setText("Start Watcher")
        self.watch_toggled.emit(self._watcher_running)

    def set_watcher_state(self, running: bool):
        """
        Externally set the watcher state (e.g. from the main window).
        Updates the menu text to match the actual state.

        Args:
            running (bool): True if the watcher is currently running
        """
        self._watcher_running = running
        self._watch_action.setText("Stop Watcher" if running else "Start Watcher")
