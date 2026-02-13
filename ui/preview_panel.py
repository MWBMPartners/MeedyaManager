# ============================================================================
# File: /ui/preview_panel.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Preview panel widget for the MeedyaManager GUI.
# Displays a table of scanned media files with their proposed rename paths.
# Includes scan controls, progress indicator, and search/filter functionality.
# Uses QAbstractTableModel for efficient data handling with large file lists.
# ============================================================================

import os                                                   # File path operations
import logging                                              # Structured logging

from PySide6.QtCore import (
    Qt,                                                     # Core Qt constants
    Signal,                                                 # Custom signal declarations
    QSortFilterProxyModel,                                  # Filter/sort proxy for search
)
from PySide6.QtGui import QAction                           # Context menu actions
from PySide6.QtWidgets import (
    QWidget,                                                # Base widget class
    QVBoxLayout,                                            # Vertical layout manager
    QHBoxLayout,                                            # Horizontal layout manager
    QPushButton,                                            # Clickable button widget
    QTableView,                                             # Table display widget
    QProgressBar,                                           # Progress indicator
    QLineEdit,                                              # Single-line text input
    QLabel,                                                 # Text label widget
    QHeaderView,                                            # Table header configuration
    QAbstractItemView,                                      # Abstract view base (selection modes)
    QMenu,                                                  # Context menu widget
)
from PySide6.QtCore import QAbstractTableModel, QModelIndex  # Table model base class

from ui.workers import ScanWorker                           # Background scanning thread

logger = logging.getLogger("MeedyaManager.PreviewPanel")


class RenamePreviewModel(QAbstractTableModel):
    """
    Table model for displaying rename preview results.
    Provides data for 5 columns: Original filename, Proposed path,
    Media Type (class), Format, and Quality.

    Uses Qt's model/view architecture for memory-efficient display
    of potentially thousands of scanned files.
    """

    # Column indices for readability
    COL_ORIGINAL = 0
    COL_PROPOSED = 1
    COL_TYPE = 2
    COL_FORMAT = 3
    COL_QUALITY = 4
    COL_COMPANIONS = 5

    # Column headers displayed in the table
    HEADERS = ["Original", "Proposed Path", "Type", "Format", "Quality", "Companions"]

    def __init__(self, parent=None):
        """Initialize the model with an empty results list."""
        super().__init__(parent)
        self._results = []             # List of scan result dictionaries

    def rowCount(self, parent=QModelIndex()):
        """Return the number of scanned files."""
        return len(self._results)

    def columnCount(self, parent=QModelIndex()):
        """Return the number of display columns (always 5)."""
        return len(self.HEADERS)

    def data(self, index, role=Qt.ItemDataRole.DisplayRole):
        """
        Provide data for a specific cell in the table.

        Args:
            index: QModelIndex specifying row and column
            role: Qt data role (DisplayRole for text, ToolTipRole for hover text)

        Returns:
            str or None: Cell text content, or None for unsupported roles
        """
        if not index.isValid() or index.row() >= len(self._results):
            return None

        result = self._results[index.row()]
        metadata = result.get("metadata", {})

        if role == Qt.ItemDataRole.DisplayRole:
            col = index.column()
            if col == self.COL_ORIGINAL:
                return result.get("filename", "")
            elif col == self.COL_PROPOSED:
                proposed = result.get("proposed_path")
                return proposed if proposed else "(no rename)"
            elif col == self.COL_TYPE:
                return metadata.get("media_class", "Unknown")
            elif col == self.COL_FORMAT:
                return metadata.get("format_class", "Unknown")
            elif col == self.COL_QUALITY:
                return metadata.get("quality_type", "Unknown")
            elif col == self.COL_COMPANIONS:
                return result.get("companion_summary", "None")

        elif role == Qt.ItemDataRole.ToolTipRole:
            col = index.column()
            if col == self.COL_COMPANIONS:
                # Show companion file names as tooltip
                companions = result.get("companions", [])
                if companions:
                    return "\n".join(
                        os.path.basename(c.path) for c in companions
                    )
                return "No companion files"
            # Default: show full file path on hover
            return result.get("filepath", "")

        return None

    def headerData(self, section, orientation, role=Qt.ItemDataRole.DisplayRole):
        """Provide column header labels."""
        if role == Qt.ItemDataRole.DisplayRole and orientation == Qt.Orientation.Horizontal:
            if 0 <= section < len(self.HEADERS):
                return self.HEADERS[section]
        return None

    def set_results(self, results):
        """
        Replace the entire results dataset and refresh the view.

        Args:
            results (list): List of scan result dictionaries from ScanWorker
        """
        self.beginResetModel()        # Notify views that data is about to change
        self._results = results
        self.endResetModel()          # Notify views that data has been updated

    def add_result(self, result):
        """
        Append a single result row (used for live updates during scanning).

        Args:
            result (dict): Single scan result dictionary
        """
        row = len(self._results)
        self.beginInsertRows(QModelIndex(), row, row)     # Notify views of new row
        self._results.append(result)
        self.endInsertRows()


class PreviewPanel(QWidget):
    """
    Main preview panel widget containing:
    - Scan button to trigger background scanning
    - Cancel button to stop an in-progress scan
    - Search field for filtering results
    - Progress bar for scan progress feedback
    - Table view displaying rename preview results with sorting

    This is the primary tab in the MeedyaManager main window.

    Signals:
        files_selected(list): Emitted when the user selects file(s) in the table.
            Contains a list of file path strings for the selected rows.
    """

    # Signal emitted when user selects files in the table (for metadata editor)
    files_selected = Signal(list)

    def __init__(self, parent=None):
        """Initialize the preview panel with all child widgets and layout."""
        super().__init__(parent)

        # Data model for the rename preview table
        self._model = RenamePreviewModel(self)

        # Sort/filter proxy model for search functionality
        self._proxy_model = QSortFilterProxyModel(self)
        self._proxy_model.setSourceModel(self._model)
        self._proxy_model.setFilterCaseSensitivity(Qt.CaseSensitivity.CaseInsensitive)
        self._proxy_model.setFilterKeyColumn(-1)              # Search across all columns

        # Background scan worker (created on demand)
        self._scan_worker = None

        # Build the widget layout
        self._setup_ui()

    def _setup_ui(self):
        """Create and arrange all child widgets in the panel layout."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(12, 12, 12, 12)

        # --- Top Controls Bar ---
        controls_layout = QHBoxLayout()

        # Scan button — triggers background scan of watch folders
        self._scan_btn = QPushButton("Scan")
        self._scan_btn.setObjectName("primaryButton")        # Uses accent-colour styling
        self._scan_btn.setToolTip("Scan watch folders for media files")
        self._scan_btn.clicked.connect(self._start_scan)
        controls_layout.addWidget(self._scan_btn)

        # Cancel button — stops an in-progress scan
        self._cancel_btn = QPushButton("Cancel")
        self._cancel_btn.setToolTip("Cancel the current scan")
        self._cancel_btn.setEnabled(False)                    # Disabled until scan starts
        self._cancel_btn.clicked.connect(self._cancel_scan)
        controls_layout.addWidget(self._cancel_btn)

        # Spacer between buttons and search field
        controls_layout.addStretch()

        # Search field — filters table results by text
        self._search_input = QLineEdit()
        self._search_input.setPlaceholderText("Search files...")
        self._search_input.setMaximumWidth(250)
        self._search_input.textChanged.connect(self._proxy_model.setFilterFixedString)
        controls_layout.addWidget(QLabel("Filter:"))
        controls_layout.addWidget(self._search_input)

        layout.addLayout(controls_layout)

        # --- Progress Bar ---
        self._progress_bar = QProgressBar()
        self._progress_bar.setVisible(False)                  # Hidden until scan starts
        self._progress_bar.setTextVisible(True)
        layout.addWidget(self._progress_bar)

        # --- Results Table ---
        self._table_view = QTableView()
        self._table_view.setModel(self._proxy_model)
        self._table_view.setAlternatingRowColors(True)        # Zebra-striped rows
        self._table_view.setSelectionBehavior(QAbstractItemView.SelectionBehavior.SelectRows)
        # ExtendedSelection allows multi-select (Ctrl+click, Shift+click) for batch editing
        self._table_view.setSelectionMode(QAbstractItemView.SelectionMode.ExtendedSelection)
        self._table_view.setSortingEnabled(True)              # Clickable column headers

        # Context menu for right-click actions
        self._table_view.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self._table_view.customContextMenuRequested.connect(self._show_context_menu)

        # Connect selection changes to emit files_selected signal
        self._table_view.selectionModel().selectionChanged.connect(self._on_selection_changed)

        # Double-click opens the metadata editor for the selected file
        self._table_view.doubleClicked.connect(self._on_double_click)

        # Configure column sizing — stretch first two columns, fit others
        header = self._table_view.horizontalHeader()
        header.setSectionResizeMode(0, QHeaderView.ResizeMode.Stretch)     # Original
        header.setSectionResizeMode(1, QHeaderView.ResizeMode.Stretch)     # Proposed
        header.setSectionResizeMode(2, QHeaderView.ResizeMode.ResizeToContents)  # Type
        header.setSectionResizeMode(3, QHeaderView.ResizeMode.ResizeToContents)  # Format
        header.setSectionResizeMode(4, QHeaderView.ResizeMode.ResizeToContents)  # Quality
        header.setSectionResizeMode(5, QHeaderView.ResizeMode.ResizeToContents)  # Companions

        layout.addWidget(self._table_view)

        # --- Status Label ---
        self._status_label = QLabel("Ready — click Scan to begin")
        layout.addWidget(self._status_label)

    def _start_scan(self, scan_paths=None):
        """
        Start a background scan of watch folders.
        Creates a new ScanWorker thread and connects its signals.

        Args:
            scan_paths (list, optional): Override paths to scan. Uses config if None.
        """
        # Prevent starting multiple scans simultaneously
        if self._scan_worker and self._scan_worker.isRunning():
            return

        # Update UI state for scanning mode
        self._scan_btn.setEnabled(False)
        self._cancel_btn.setEnabled(True)
        self._progress_bar.setVisible(True)
        self._progress_bar.setValue(0)
        self._status_label.setText("Scanning...")

        # Clear previous results
        self._model.set_results([])

        # Create and configure the background worker
        self._scan_worker = ScanWorker(scan_paths=scan_paths, parent=self)
        self._scan_worker.progress.connect(self._on_progress)
        self._scan_worker.file_scanned.connect(self._on_file_scanned)
        self._scan_worker.result_ready.connect(self._on_scan_complete)
        self._scan_worker.error.connect(self._on_scan_error)
        self._scan_worker.finished.connect(self._on_worker_finished)

        # Start the background thread
        self._scan_worker.start()
        logger.info("Scan started")

    def _cancel_scan(self):
        """Request cancellation of the running scan."""
        if self._scan_worker:
            self._scan_worker.cancel()
            self._status_label.setText("Cancelling...")

    def _on_progress(self, current, total):
        """Update the progress bar with current scan progress."""
        self._progress_bar.setMaximum(total)
        self._progress_bar.setValue(current)
        self._status_label.setText(f"Scanning: {current}/{total} files")

    def _on_file_scanned(self, result):
        """Add a single scanned file to the table (live update)."""
        self._model.add_result(result)

    def _on_scan_complete(self, results):
        """Handle completed scan — update status with final count."""
        count = len(results)
        self._status_label.setText(f"Scan complete — {count} file{'s' if count != 1 else ''} found")
        logger.info(f"Scan complete: {count} files")

    def _on_scan_error(self, error_msg):
        """Handle scan error — display error message in status bar."""
        self._status_label.setText(f"Scan error: {error_msg}")
        logger.error(f"Scan error: {error_msg}")

    def _on_worker_finished(self):
        """Reset UI state when the scan worker thread finishes."""
        self._scan_btn.setEnabled(True)
        self._cancel_btn.setEnabled(False)
        self._progress_bar.setVisible(False)

    def _get_selected_filepaths(self):
        """
        Return file paths for all selected rows in the table.

        Returns:
            list[str]: List of file paths for the selected rows.
        """
        filepaths = []
        indexes = self._table_view.selectionModel().selectedRows()
        for proxy_index in indexes:
            source_index = self._proxy_model.mapToSource(proxy_index)
            row = source_index.row()
            if row < len(self._model._results):
                filepath = self._model._results[row].get("filepath", "")
                if filepath:
                    filepaths.append(filepath)
        return filepaths

    def _on_selection_changed(self, selected, deselected):
        """Emit files_selected signal when the table selection changes."""
        filepaths = self._get_selected_filepaths()
        if filepaths:
            self.files_selected.emit(filepaths)

    def _on_double_click(self, proxy_index):
        """
        Handle double-click on a table row — emit files_selected for the
        single clicked file (metadata editor will load it).
        """
        source_index = self._proxy_model.mapToSource(proxy_index)
        row = source_index.row()
        if row < len(self._model._results):
            filepath = self._model._results[row].get("filepath", "")
            if filepath:
                self.files_selected.emit([filepath])

    def _show_context_menu(self, pos):
        """
        Show a right-click context menu at the given position.
        Provides quick actions: Edit Metadata, Copy Path.
        """
        menu = QMenu(self)

        # Edit Metadata action
        edit_action = QAction("Edit Metadata", self)
        edit_action.triggered.connect(self._context_edit_metadata)
        menu.addAction(edit_action)

        menu.addSeparator()

        # Copy Path action
        copy_action = QAction("Copy Path", self)
        copy_action.triggered.connect(self._context_copy_path)
        menu.addAction(copy_action)

        menu.exec(self._table_view.viewport().mapToGlobal(pos))

    def _context_edit_metadata(self):
        """Context menu: emit selected files for metadata editing."""
        filepaths = self._get_selected_filepaths()
        if filepaths:
            self.files_selected.emit(filepaths)

    def _context_copy_path(self):
        """Context menu: copy selected file path to clipboard."""
        from PySide6.QtWidgets import QApplication
        filepaths = self._get_selected_filepaths()
        if filepaths:
            QApplication.clipboard().setText(filepaths[0])
