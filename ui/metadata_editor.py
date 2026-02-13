# ============================================================================
# File: /ui/metadata_editor.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Metadata editor panel for viewing and editing embedded tags on media files.
# Provides a table-based tag editor with:
#   - Two-column display: Tag Name, Current Value (editable)
#   - Cover art thumbnail with Replace/Remove/Extract controls
#   - Batch editing support (select multiple files → edit shared tags)
#   - Save/Revert buttons with change preview
#   - "Add Custom Tag" for user-defined metadata
#
# Uses TagEditor (metadata/editor.py) for reading/writing and the tag
# registry (core/tag_registry.py) for display names and editability.
# ============================================================================

import os                                                   # File path operations
import logging                                              # Structured logging

from PySide6.QtCore import (
    Qt,                                                     # Core Qt constants
    Signal,                                                 # Custom signal declarations
    QModelIndex,                                            # Model index for table
)
from PySide6.QtGui import QPixmap, QImage                   # Image display for cover art
from PySide6.QtWidgets import (
    QWidget,                                                # Base widget class
    QVBoxLayout,                                            # Vertical layout
    QHBoxLayout,                                            # Horizontal layout
    QSplitter,                                              # Resizable split panes
    QPushButton,                                            # Clickable button
    QTableView,                                             # Table display widget
    QLabel,                                                 # Text/image label
    QHeaderView,                                            # Table header config
    QAbstractItemView,                                      # Selection modes
    QFileDialog,                                            # File open/save dialogs
    QMessageBox,                                            # Alert/confirm dialogs
    QInputDialog,                                           # Simple text input dialog
    QGroupBox,                                              # Titled group box
)
from PySide6.QtCore import QAbstractTableModel              # Table model base

from metadata.editor import TagEditor, CoverArt, UnsupportedFormatError, TagWriteError
from core.tag_registry import (
    REVERSE_TAG_MAP,
    is_editable_tag,
    TECHNICAL_TAGS,
)

logger = logging.getLogger("MeedyaManager.MetadataEditor")


# =============================================================================
# Tag Table Model
# =============================================================================

class TagTableModel(QAbstractTableModel):
    """
    Table model for the metadata tag editor.

    Displays two columns: Tag Name (display name) and Value (editable for
    non-technical fields). Loads tag data from TagEditor.read_tags() and
    provides change tracking for the Save operation.

    For batch mode (multiple files), mixed values are shown as '<Multiple>'.
    """

    # Column indices
    COL_TAG = 0
    COL_VALUE = 1

    # Column headers
    HEADERS = ["Tag", "Value"]

    def __init__(self, parent=None):
        """Initialize with empty tag data."""
        super().__init__(parent)
        # Internal data: list of [display_name, internal_key, current_value, new_value]
        self._tags = []
        # Set of internal keys that are editable (not technical)
        self._editable_keys = set()
        # Track original values for revert
        self._original_values = {}
        # Whether we're in batch mode
        self._batch_mode = False

    def rowCount(self, parent=QModelIndex()):
        """Return the number of tag rows."""
        return len(self._tags)

    def columnCount(self, parent=QModelIndex()):
        """Return the number of columns (always 2)."""
        return len(self.HEADERS)

    def data(self, index, role=Qt.ItemDataRole.DisplayRole):
        """
        Provide cell data for the table view.

        Args:
            index: QModelIndex for the cell.
            role: Qt display role.

        Returns:
            str or None: Cell content for the requested role.
        """
        if not index.isValid() or index.row() >= len(self._tags):
            return None

        row_data = self._tags[index.row()]
        display_name, internal_key, current_value, new_value = row_data

        if role == Qt.ItemDataRole.DisplayRole or role == Qt.ItemDataRole.EditRole:
            if index.column() == self.COL_TAG:
                return display_name
            elif index.column() == self.COL_VALUE:
                return new_value if new_value is not None else current_value

        elif role == Qt.ItemDataRole.ToolTipRole:
            if index.column() == self.COL_TAG:
                return f"Internal key: {internal_key}"
            elif index.column() == self.COL_VALUE:
                if new_value is not None and new_value != current_value:
                    return f"Original: {current_value}"

        elif role == Qt.ItemDataRole.ForegroundRole:
            # Highlight modified values in a different color
            if index.column() == self.COL_VALUE:
                if new_value is not None and new_value != current_value:
                    from PySide6.QtGui import QColor
                    return QColor(0, 120, 215)              # Blue for modified values

        return None

    def setData(self, index, value, role=Qt.ItemDataRole.EditRole):
        """
        Handle user edits to the Value column.

        Args:
            index: QModelIndex for the edited cell.
            value: New string value from the editor widget.
            role: Qt edit role.

        Returns:
            bool: True if the edit was accepted.
        """
        if not index.isValid() or role != Qt.ItemDataRole.EditRole:
            return False

        if index.column() != self.COL_VALUE:
            return False

        row = index.row()
        if row >= len(self._tags):
            return False

        # Update the new_value field
        self._tags[row][3] = str(value).strip()
        self.dataChanged.emit(index, index, [role])
        return True

    def flags(self, index):
        """
        Determine cell flags — Value column is editable for non-technical tags.

        Args:
            index: QModelIndex for the cell.

        Returns:
            Qt.ItemFlags: Flags for the cell.
        """
        base_flags = super().flags(index)

        if index.column() == self.COL_VALUE:
            internal_key = self._tags[index.row()][1]
            if internal_key in self._editable_keys:
                return base_flags | Qt.ItemFlag.ItemIsEditable

        return base_flags

    def headerData(self, section, orientation, role=Qt.ItemDataRole.DisplayRole):
        """Provide column header labels."""
        if role == Qt.ItemDataRole.DisplayRole and orientation == Qt.Orientation.Horizontal:
            if 0 <= section < len(self.HEADERS):
                return self.HEADERS[section]
        return None

    def load_file(self, filepath):
        """
        Load tags from a single file and populate the model.

        Reads all embedded tags via TagEditor and presents them sorted
        by display name. Editable and read-only fields are distinguished
        via is_editable_tag().

        Args:
            filepath (str): Path to the media file.
        """
        self._batch_mode = False
        self.beginResetModel()

        editor = TagEditor()
        raw_tags = editor.read_tags(filepath)

        self._tags = []
        self._editable_keys = set()
        self._original_values = {}

        # Add all tags read from the file
        for internal_key, value in sorted(raw_tags.items()):
            display_name = REVERSE_TAG_MAP.get(internal_key, internal_key)
            str_value = str(value) if value else ""
            self._tags.append([display_name, internal_key, str_value, None])
            self._original_values[internal_key] = str_value

            if is_editable_tag(internal_key):
                self._editable_keys.add(internal_key)

        self.endResetModel()

    def load_batch(self, filepaths):
        """
        Load shared tags from multiple files for batch editing.

        Tags that have the same value across all files show that value.
        Tags with differing values show '<Multiple>'.

        Args:
            filepaths (list[str]): List of media file paths.
        """
        self._batch_mode = True
        self.beginResetModel()

        editor = TagEditor()

        # Read tags from all files
        all_tags = []
        for fp in filepaths:
            all_tags.append(editor.read_tags(fp))

        # Collect all unique internal keys across all files
        all_keys = set()
        for tag_dict in all_tags:
            all_keys.update(tag_dict.keys())

        self._tags = []
        self._editable_keys = set()
        self._original_values = {}

        for internal_key in sorted(all_keys):
            display_name = REVERSE_TAG_MAP.get(internal_key, internal_key)

            # Collect values for this key across all files
            values = [str(t.get(internal_key, "")) for t in all_tags]
            unique_values = set(values)

            if len(unique_values) == 1:
                # All files have the same value
                current_value = values[0]
            else:
                # Mixed values across files
                current_value = "<Multiple>"

            self._tags.append([display_name, internal_key, current_value, None])
            self._original_values[internal_key] = current_value

            if is_editable_tag(internal_key):
                self._editable_keys.add(internal_key)

        self.endResetModel()

    def get_changes(self):
        """
        Return a dictionary of modified tags.

        Returns:
            dict: {internal_key: new_value} for all tags where the user
                  entered a different value from the original.
        """
        changes = {}
        for display_name, internal_key, current_value, new_value in self._tags:
            if new_value is not None and new_value != current_value:
                changes[internal_key] = new_value
        return changes

    def add_custom_tag(self, custom_key, initial_value=""):
        """
        Add a new custom tag row to the model.

        Args:
            custom_key (str): The custom tag name (e.g., "spotify_url").
            initial_value (str): Initial value for the tag.
        """
        internal_key = f"custom_{custom_key.lower().replace(' ', '_')}"
        display_name = f"Custom: {custom_key}"

        # Check if this custom tag already exists
        for row in self._tags:
            if row[1] == internal_key:
                return                                       # Already exists

        row = len(self._tags)
        self.beginInsertRows(QModelIndex(), row, row)
        self._tags.append([display_name, internal_key, "", initial_value])
        self._editable_keys.add(internal_key)
        self.endInsertRows()


# =============================================================================
# Cover Art Widget
# =============================================================================

class CoverArtWidget(QWidget):
    """
    Displays cover art thumbnail with Replace, Remove, and Extract buttons.

    Shows the first embedded cover art image (front cover preferred).
    Maximum thumbnail size is 200x200 pixels.
    """

    # Signal emitted when cover art changes (for save tracking)
    cover_changed = Signal()

    def __init__(self, parent=None):
        """Initialize the cover art display widget."""
        super().__init__(parent)

        self._filepath = None                                # Current file path
        self._cover_data = None                              # Raw image bytes
        self._editor = TagEditor()

        self._setup_ui()

    def _setup_ui(self):
        """Create the cover art display and control buttons."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)

        # Cover art thumbnail label (200x200 max)
        self._thumbnail = QLabel("No Cover Art")
        self._thumbnail.setFixedSize(200, 200)
        self._thumbnail.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._thumbnail.setStyleSheet(
            "QLabel { border: 1px solid #555; background-color: #222; color: #888; }"
        )
        layout.addWidget(self._thumbnail)

        # Button row: Replace, Remove, Extract
        btn_layout = QHBoxLayout()

        self._replace_btn = QPushButton("Replace")
        self._replace_btn.setToolTip("Replace cover art with an image file")
        self._replace_btn.clicked.connect(self._on_replace)
        btn_layout.addWidget(self._replace_btn)

        self._remove_btn = QPushButton("Remove")
        self._remove_btn.setToolTip("Remove all cover art from the file")
        self._remove_btn.clicked.connect(self._on_remove)
        btn_layout.addWidget(self._remove_btn)

        self._extract_btn = QPushButton("Extract")
        self._extract_btn.setToolTip("Save the cover art to a file")
        self._extract_btn.clicked.connect(self._on_extract)
        btn_layout.addWidget(self._extract_btn)

        layout.addLayout(btn_layout)

    def load_file(self, filepath):
        """
        Load and display cover art from a media file.

        Args:
            filepath (str): Path to the media file.
        """
        self._filepath = filepath
        self._cover_data = None

        covers = self._editor.read_cover_art(filepath)
        if covers:
            # Display the first cover art image
            cover = covers[0]
            self._cover_data = cover.data
            self._display_image(cover.data)
            self._remove_btn.setEnabled(True)
            self._extract_btn.setEnabled(True)
        else:
            self._thumbnail.setText("No Cover Art")
            self._thumbnail.setPixmap(QPixmap())
            self._remove_btn.setEnabled(True)
            self._extract_btn.setEnabled(False)

    def _display_image(self, image_data):
        """
        Display image bytes as a scaled thumbnail.

        Args:
            image_data (bytes): Raw JPEG or PNG image data.
        """
        pixmap = QPixmap()
        pixmap.loadFromData(image_data)
        if not pixmap.isNull():
            scaled = pixmap.scaled(
                200, 200,
                Qt.AspectRatioMode.KeepAspectRatio,
                Qt.TransformationMode.SmoothTransformation
            )
            self._thumbnail.setPixmap(scaled)
            self._thumbnail.setText("")
        else:
            self._thumbnail.setText("(Invalid Image)")

    def _on_replace(self):
        """Open a file dialog to select a new cover art image."""
        if not self._filepath:
            return

        image_path, _ = QFileDialog.getOpenFileName(
            self, "Select Cover Art Image", "",
            "Images (*.jpg *.jpeg *.png);;All Files (*)"
        )
        if not image_path:
            return

        try:
            with open(image_path, "rb") as f:
                image_data = f.read()

            # Determine format from extension
            ext = os.path.splitext(image_path)[1].lower()
            img_format = "png" if ext == ".png" else "jpeg"

            self._editor.write_cover_art(self._filepath, image_data, img_format)
            self._cover_data = image_data
            self._display_image(image_data)
            self.cover_changed.emit()
            logger.info(f"Cover art replaced in {self._filepath}")

        except (TagWriteError, UnsupportedFormatError) as e:
            QMessageBox.warning(self, "Cover Art Error", str(e))

    def _on_remove(self):
        """Remove all cover art from the current file."""
        if not self._filepath:
            return

        try:
            count = self._editor.remove_cover_art(self._filepath)
            if count > 0:
                self._thumbnail.setText("No Cover Art")
                self._thumbnail.setPixmap(QPixmap())
                self._cover_data = None
                self._extract_btn.setEnabled(False)
                self.cover_changed.emit()
                logger.info(f"Removed {count} cover art image(s) from {self._filepath}")
        except TagWriteError as e:
            QMessageBox.warning(self, "Cover Art Error", str(e))

    def _on_extract(self):
        """Save the current cover art to a file."""
        if not self._cover_data:
            return

        save_path, _ = QFileDialog.getSaveFileName(
            self, "Save Cover Art", "cover.jpg",
            "JPEG (*.jpg);;PNG (*.png);;All Files (*)"
        )
        if save_path:
            with open(save_path, "wb") as f:
                f.write(self._cover_data)
            logger.info(f"Cover art extracted to {save_path}")


# =============================================================================
# Metadata Editor Panel
# =============================================================================

class MetadataEditorPanel(QWidget):
    """
    Main metadata editor panel — added as a tab in MainWindow.

    Provides:
    - File path display showing the currently loaded file
    - Tag table with editable values for embedded tag fields
    - Cover art widget with Replace/Remove/Extract controls
    - Save and Revert buttons with change preview
    - Add Custom Tag button for user-defined metadata
    """

    def __init__(self, parent=None):
        """Initialize the metadata editor panel with all child widgets."""
        super().__init__(parent)

        self._current_filepaths = []                         # Currently loaded file(s)
        self._tag_editor = TagEditor()

        self._setup_ui()

    def _setup_ui(self):
        """Create and arrange all child widgets."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(12, 12, 12, 12)

        # --- File Path Display ---
        self._file_label = QLabel("No file loaded — select a file from Scan/Preview or drag one here")
        self._file_label.setWordWrap(True)
        layout.addWidget(self._file_label)

        # --- Main Content: Tag Table + Cover Art side by side ---
        splitter = QSplitter(Qt.Orientation.Horizontal)

        # Left side: Tag editing table
        tag_group = QGroupBox("Tags")
        tag_layout = QVBoxLayout(tag_group)

        self._tag_model = TagTableModel(self)
        self._tag_table = QTableView()
        self._tag_table.setModel(self._tag_model)
        self._tag_table.setAlternatingRowColors(True)
        self._tag_table.setSelectionBehavior(QAbstractItemView.SelectionBehavior.SelectRows)
        self._tag_table.setSelectionMode(QAbstractItemView.SelectionMode.SingleSelection)

        # Column sizing
        header = self._tag_table.horizontalHeader()
        header.setSectionResizeMode(0, QHeaderView.ResizeMode.ResizeToContents)
        header.setSectionResizeMode(1, QHeaderView.ResizeMode.Stretch)

        tag_layout.addWidget(self._tag_table)
        splitter.addWidget(tag_group)

        # Right side: Cover art widget
        cover_group = QGroupBox("Cover Art")
        cover_layout = QVBoxLayout(cover_group)
        self._cover_widget = CoverArtWidget()
        cover_layout.addWidget(self._cover_widget)
        cover_layout.addStretch()                            # Push cover art to top
        splitter.addWidget(cover_group)

        # Set initial splitter sizes (70% tags, 30% cover art)
        splitter.setSizes([700, 300])

        layout.addWidget(splitter)

        # --- Button Bar ---
        btn_layout = QHBoxLayout()

        self._add_custom_btn = QPushButton("Add Custom Tag")
        self._add_custom_btn.setToolTip("Add a user-defined custom tag")
        self._add_custom_btn.clicked.connect(self._on_add_custom_tag)
        self._add_custom_btn.setEnabled(False)
        btn_layout.addWidget(self._add_custom_btn)

        btn_layout.addStretch()

        self._revert_btn = QPushButton("Revert")
        self._revert_btn.setToolTip("Discard changes and reload from file")
        self._revert_btn.clicked.connect(self._on_revert)
        self._revert_btn.setEnabled(False)
        btn_layout.addWidget(self._revert_btn)

        self._save_btn = QPushButton("Save")
        self._save_btn.setObjectName("primaryButton")
        self._save_btn.setToolTip("Write tag changes to the file")
        self._save_btn.clicked.connect(self._on_save)
        self._save_btn.setEnabled(False)
        btn_layout.addWidget(self._save_btn)

        layout.addLayout(btn_layout)

        # --- Status Label ---
        self._status_label = QLabel("")
        layout.addWidget(self._status_label)

    def load_file(self, filepath):
        """
        Load a single file for tag editing.

        Reads all embedded tags and cover art, populates the tag table
        and cover art widget.

        Args:
            filepath (str): Path to the media file.
        """
        self._current_filepaths = [filepath]
        self._file_label.setText(f"File: {os.path.basename(filepath)}")

        # Load tags into the table model
        self._tag_model.load_file(filepath)

        # Load cover art
        self._cover_widget.load_file(filepath)

        # Enable editing controls
        self._save_btn.setEnabled(True)
        self._revert_btn.setEnabled(True)
        self._add_custom_btn.setEnabled(True)
        self._status_label.setText("Ready to edit")

        logger.info(f"Loaded file for editing: {filepath}")

    def load_files(self, filepaths):
        """
        Load multiple files for batch tag editing.

        Tags with the same value across all files show that value.
        Tags with differing values show '<Multiple>'. Editing a field
        applies the new value to all selected files.

        Args:
            filepaths (list[str]): List of media file paths.
        """
        if len(filepaths) == 1:
            self.load_file(filepaths[0])
            return

        self._current_filepaths = list(filepaths)
        self._file_label.setText(f"Batch edit: {len(filepaths)} files selected")

        # Load batch tags
        self._tag_model.load_batch(filepaths)

        # Disable cover art for batch mode (too complex for initial release)
        self._cover_widget._thumbnail.setText("Cover art not available in batch mode")
        self._cover_widget._thumbnail.setPixmap(QPixmap())
        self._cover_widget._replace_btn.setEnabled(False)
        self._cover_widget._remove_btn.setEnabled(False)
        self._cover_widget._extract_btn.setEnabled(False)

        # Enable editing controls
        self._save_btn.setEnabled(True)
        self._revert_btn.setEnabled(True)
        self._add_custom_btn.setEnabled(True)
        self._status_label.setText(f"Batch mode: {len(filepaths)} files")

        logger.info(f"Loaded {len(filepaths)} files for batch editing")

    def _on_save(self):
        """
        Write tag changes to the file(s).

        Shows a confirmation dialog with the list of changes before writing.
        For batch mode, applies changes to all selected files.
        """
        changes = self._tag_model.get_changes()

        if not changes:
            self._status_label.setText("No changes to save")
            return

        # Build confirmation message
        change_lines = []
        for key, new_value in changes.items():
            display_name = REVERSE_TAG_MAP.get(key, key)
            change_lines.append(f"  {display_name}: {new_value}")

        msg = f"Apply {len(changes)} tag change(s)?\n\n" + "\n".join(change_lines)

        if len(self._current_filepaths) > 1:
            msg += f"\n\nThis will modify {len(self._current_filepaths)} files."

        reply = QMessageBox.question(
            self, "Confirm Save", msg,
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No
        )

        if reply != QMessageBox.StandardButton.Yes:
            return

        # Write changes to each file
        success_count = 0
        error_count = 0

        for filepath in self._current_filepaths:
            try:
                self._tag_editor.write_tags(filepath, changes)
                success_count += 1
            except (TagWriteError, UnsupportedFormatError) as e:
                logger.error(f"Failed to write tags to {filepath}: {e}")
                error_count += 1

        # Update status and reload
        if error_count == 0:
            self._status_label.setText(
                f"Saved {len(changes)} tag(s) to {success_count} file(s)"
            )
        else:
            self._status_label.setText(
                f"Saved to {success_count}, failed on {error_count} file(s)"
            )

        # Reload to show saved values
        if len(self._current_filepaths) == 1:
            self.load_file(self._current_filepaths[0])
        else:
            self.load_files(self._current_filepaths)

    def _on_revert(self):
        """Discard all changes and reload from file."""
        if len(self._current_filepaths) == 1:
            self.load_file(self._current_filepaths[0])
        elif self._current_filepaths:
            self.load_files(self._current_filepaths)
        self._status_label.setText("Changes reverted")

    def _on_add_custom_tag(self):
        """Show input dialog for a custom tag name, then add a row."""
        name, ok = QInputDialog.getText(
            self, "Add Custom Tag",
            "Enter custom tag name (e.g., SpotifyURL, MusicBrainzID):"
        )
        if ok and name.strip():
            self._tag_model.add_custom_tag(name.strip())
            self._status_label.setText(f"Added custom tag: {name.strip()}")
