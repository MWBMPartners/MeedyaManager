# ============================================================================
# File: /ui/main_window.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Main application window for the MeedyaManager GUI.
# Provides:
#   - Tabbed interface: Scan/Preview, Rules, Settings
#   - Menu bar: File, Edit, View, Help
#   - Toolbar: Quick access to Scan, Watch toggle, Settings
#   - Status bar: Watcher status, file count, simulation mode
#   - System tray integration for background operation
#   - Drag-and-drop support for media files
#
# Uses PySide6/Qt6 for cross-platform native rendering.
# ============================================================================

import os                                                   # File path operations
import logging                                              # Structured logging

from PySide6.QtCore import Qt, QUrl                         # Core constants, URL handling
from PySide6.QtGui import QAction, QIcon, QDragEnterEvent, QDropEvent  # GUI actions/events
from PySide6.QtWidgets import (
    QMainWindow,                                            # Main window base class
    QTabWidget,                                             # Tabbed container widget
    QToolBar,                                               # Toolbar widget
    QStatusBar,                                             # Status bar widget
    QLabel,                                                 # Text label widget
    QMessageBox,                                            # Alert/confirm dialogs
    QApplication,                                           # Application instance
)

from ui.preview_panel import PreviewPanel                   # Scan/Preview tab
from ui.rule_builder import RuleBuilder                     # Rule builder tab
from ui.settings_dialog import SettingsDialog               # Settings dialog
from ui.system_tray import SystemTrayIcon                   # System tray icon

logger = logging.getLogger("MeedyaManager.MainWindow")


class MainWindow(QMainWindow):
    """
    MeedyaManager main application window.

    Contains tabbed interface with Scan/Preview, Rules, and access to
    Settings dialog. Integrates with system tray for background operation
    and supports drag-and-drop of media files for quick inspection.
    """

    def __init__(self):
        """Initialize the main window with all UI components."""
        super().__init__()

        # Window properties
        self.setWindowTitle("MeedyaManager")
        self.setMinimumSize(900, 600)
        self.resize(1100, 700)

        # Accept drag-and-drop events on the main window
        self.setAcceptDrops(True)

        # Track watcher state
        self._watcher_running = False

        # Build all UI components
        self._setup_menu_bar()
        self._setup_toolbar()
        self._setup_tabs()
        self._setup_status_bar()
        self._setup_system_tray()

        logger.info("Main window initialized")

    # =========================================================================
    # UI Setup Methods
    # =========================================================================

    def _setup_menu_bar(self):
        """Create the application menu bar with File, Edit, View, and Help menus."""
        menu_bar = self.menuBar()

        # --- File Menu ---
        file_menu = menu_bar.addMenu("&File")

        # Scan action — triggers a media file scan
        scan_action = QAction("&Scan", self)
        scan_action.setShortcut("Ctrl+Shift+S")
        scan_action.setStatusTip("Scan watch folders for media files")
        scan_action.triggered.connect(self._on_scan)
        file_menu.addAction(scan_action)

        file_menu.addSeparator()

        # Settings action — opens the settings dialog
        settings_action = QAction("&Settings...", self)
        settings_action.setShortcut("Ctrl+,")
        settings_action.setStatusTip("Open application settings")
        settings_action.triggered.connect(self._open_settings)
        file_menu.addAction(settings_action)

        file_menu.addSeparator()

        # Quit action — exits the application
        quit_action = QAction("&Quit", self)
        quit_action.setShortcut("Ctrl+Q")
        quit_action.setStatusTip("Quit MeedyaManager")
        quit_action.triggered.connect(self._on_quit)
        file_menu.addAction(quit_action)

        # --- Edit Menu ---
        edit_menu = menu_bar.addMenu("&Edit")

        # Placeholder for future edit actions (copy path, select all, etc.)
        copy_path_action = QAction("Copy &Path", self)
        copy_path_action.setShortcut("Ctrl+Shift+C")
        copy_path_action.setStatusTip("Copy selected file path to clipboard")
        copy_path_action.triggered.connect(self._copy_selected_path)
        edit_menu.addAction(copy_path_action)

        # --- View Menu ---
        view_menu = menu_bar.addMenu("&View")

        # Toggle watcher from menu
        self._watch_menu_action = QAction("Start &Watcher", self)
        self._watch_menu_action.setShortcut("Ctrl+W")
        self._watch_menu_action.setStatusTip("Start or stop the file watcher")
        self._watch_menu_action.triggered.connect(self._toggle_watcher)
        view_menu.addAction(self._watch_menu_action)

        # --- Help Menu ---
        help_menu = menu_bar.addMenu("&Help")

        about_action = QAction("&About MeedyaManager", self)
        about_action.setStatusTip("About this application")
        about_action.triggered.connect(self._show_about)
        help_menu.addAction(about_action)

    def _setup_toolbar(self):
        """Create the main toolbar with quick-access buttons."""
        toolbar = QToolBar("Main Toolbar")
        toolbar.setMovable(False)                            # Fixed position toolbar
        self.addToolBar(toolbar)

        # Scan button
        scan_action = QAction("Scan", self)
        scan_action.setStatusTip("Scan watch folders")
        scan_action.triggered.connect(self._on_scan)
        toolbar.addAction(scan_action)

        # Watch toggle button
        self._watch_toolbar_action = QAction("Start Watch", self)
        self._watch_toolbar_action.setStatusTip("Toggle file watcher")
        self._watch_toolbar_action.triggered.connect(self._toggle_watcher)
        toolbar.addAction(self._watch_toolbar_action)

        toolbar.addSeparator()

        # Settings button
        settings_action = QAction("Settings", self)
        settings_action.setStatusTip("Open settings")
        settings_action.triggered.connect(self._open_settings)
        toolbar.addAction(settings_action)

    def _setup_tabs(self):
        """Create the central tabbed widget with Scan/Preview and Rules tabs."""
        self._tab_widget = QTabWidget()
        self.setCentralWidget(self._tab_widget)

        # Scan/Preview tab — main scanning and rename preview interface
        self._preview_panel = PreviewPanel()
        self._tab_widget.addTab(self._preview_panel, "Scan / Preview")

        # Rules tab — template editor with syntax highlighting
        self._rule_builder = RuleBuilder()
        self._tab_widget.addTab(self._rule_builder, "Rules")

    def _setup_status_bar(self):
        """Create the status bar with watcher status and file count labels."""
        status_bar = QStatusBar()
        self.setStatusBar(status_bar)

        # Watcher status indicator (left side)
        self._watcher_status_label = QLabel("Watcher: Stopped")
        status_bar.addWidget(self._watcher_status_label)

        # File count (right side)
        self._file_count_label = QLabel("Files: 0")
        status_bar.addPermanentWidget(self._file_count_label)

    def _setup_system_tray(self):
        """Create the system tray icon and connect its signals."""
        self._tray_icon = SystemTrayIcon(self)

        # Connect tray signals to main window actions
        self._tray_icon.show_requested.connect(self._toggle_visibility)
        self._tray_icon.scan_requested.connect(self._on_scan)
        self._tray_icon.watch_toggled.connect(self._on_tray_watch_toggled)
        self._tray_icon.quit_requested.connect(self._on_quit)

        # Show the tray icon
        self._tray_icon.show()

    # =========================================================================
    # Action Handlers
    # =========================================================================

    def _on_scan(self):
        """Trigger a scan from the preview panel."""
        self._tab_widget.setCurrentWidget(self._preview_panel)
        self._preview_panel._start_scan()

    def _open_settings(self):
        """Open the settings dialog as a modal window."""
        dialog = SettingsDialog(self)
        dialog.exec()

    def _toggle_watcher(self):
        """Toggle the file watcher on/off."""
        self._watcher_running = not self._watcher_running

        if self._watcher_running:
            self._watcher_status_label.setText("Watcher: Running")
            self._watch_menu_action.setText("Stop &Watcher")
            self._watch_toolbar_action.setText("Stop Watch")
            self._tray_icon.set_watcher_state(True)
            logger.info("Watcher started")
        else:
            self._watcher_status_label.setText("Watcher: Stopped")
            self._watch_menu_action.setText("Start &Watcher")
            self._watch_toolbar_action.setText("Start Watch")
            self._tray_icon.set_watcher_state(False)
            logger.info("Watcher stopped")

    def _on_tray_watch_toggled(self, running):
        """Handle watcher toggle from the tray icon."""
        self._watcher_running = running
        if running:
            self._watcher_status_label.setText("Watcher: Running")
            self._watch_menu_action.setText("Stop &Watcher")
            self._watch_toolbar_action.setText("Stop Watch")
        else:
            self._watcher_status_label.setText("Watcher: Stopped")
            self._watch_menu_action.setText("Start &Watcher")
            self._watch_toolbar_action.setText("Start Watch")

    def _toggle_visibility(self):
        """Toggle main window visibility (used by tray icon)."""
        if self.isVisible():
            self.hide()
        else:
            self.show()
            self.activateWindow()
            self.raise_()

    def _copy_selected_path(self):
        """Copy the selected file's path to the system clipboard."""
        # Get the selected row from the preview table
        table = self._preview_panel._table_view
        indexes = table.selectionModel().selectedRows()
        if indexes:
            # Get the tooltip (full file path) from the first column
            source_index = self._preview_panel._proxy_model.mapToSource(indexes[0])
            filepath = self._preview_panel._model._results[source_index.row()].get("filepath", "")
            if filepath:
                QApplication.clipboard().setText(filepath)
                self.statusBar().showMessage(f"Copied: {filepath}", 3000)

    def _show_about(self):
        """Show the About dialog with application info."""
        QMessageBox.about(
            self,
            "About MeedyaManager",
            "MeedyaManager v1.1-M2\n\n"
            "Cross-platform media file manager and auto-organizer.\n\n"
            "(C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)\n\n"
            "Built with Python, PySide6, and pymediainfo."
        )

    def _on_quit(self):
        """Quit the application cleanly."""
        self._tray_icon.hide()
        QApplication.quit()

    # =========================================================================
    # Drag and Drop Support
    # =========================================================================

    def dragEnterEvent(self, event: QDragEnterEvent):
        """Accept drag events that contain file URLs."""
        if event.mimeData().hasUrls():
            event.acceptProposedAction()

    def dropEvent(self, event: QDropEvent):
        """Handle dropped files — scan them individually."""
        urls = event.mimeData().urls()
        files = [url.toLocalFile() for url in urls if url.isLocalFile()]
        if files:
            logger.info(f"Files dropped: {len(files)}")
            self._preview_panel._start_scan(scan_paths=[os.path.dirname(f) for f in files])

    # =========================================================================
    # Window Close Behavior
    # =========================================================================

    def closeEvent(self, event):
        """
        Override close to minimize to tray instead of quitting.
        The application continues running in the background.
        """
        if self._tray_icon.isVisible():
            # Hide to tray instead of closing
            self.hide()
            self._tray_icon.showMessage(
                "MeedyaManager",
                "Running in the background. Double-click the tray icon to show.",
                QSystemTrayIcon.MessageIcon.Information,
                2000
            )
            event.ignore()                                   # Don't actually close
        else:
            # No tray icon — quit normally
            event.accept()
