# ============================================================================
# File: /ui/lookup_panel.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Lookup panel widget for the MeedyaManager GUI.
# Provides an interactive metadata lookup interface that searches multiple
# online providers (Spotify, Apple Music, TMDB, MusicBrainz, etc.) for
# metadata matches against a loaded media file.
#
# Layout:
#   - Top section: File info label, provider checkboxes (grouped by
#     category), and a Search button
#   - Middle section: QSplitter dividing a results table (left) and a
#     detail panel (right) showing full metadata + cover art placeholder
#   - Bottom section: "Apply Selected" and "Batch Lookup" action buttons
#
# Signals:
#   - lookup_completed(list): Emitted when lookup results arrive
#   - tags_applied(str, dict): Emitted when tags are applied to a file
#
# Uses LookupService (metadata/lookup_service.py) for provider discovery
# and LookupWorker (ui/workers.py) for background threaded lookups.
# ============================================================================

import os                                                       # File path operations
import logging                                                  # Structured logging

from PySide6.QtCore import (
    Qt,                                                         # Core Qt constants (alignment, orientation, etc.)
    Signal,                                                     # Custom signal declarations for inter-widget communication
)
from PySide6.QtWidgets import (
    QWidget,                                                    # Base widget class for all UI elements
    QVBoxLayout,                                                # Vertical layout manager (stack widgets top to bottom)
    QHBoxLayout,                                                # Horizontal layout manager (stack widgets left to right)
    QGridLayout,                                                # Grid-based layout for provider checkboxes
    QGroupBox,                                                  # Titled group container for visual grouping
    QCheckBox,                                                  # Toggleable checkbox for provider selection
    QPushButton,                                                # Clickable button widget for actions
    QTableWidget,                                               # Table widget with built-in item management
    QTableWidgetItem,                                           # Individual cell item within QTableWidget
    QLabel,                                                     # Text or image display label
    QSplitter,                                                  # Resizable split pane divider
    QTextEdit,                                                  # Multi-line rich text display/editor
    QHeaderView,                                                # Table header configuration (column sizing)
    QAbstractItemView,                                          # Abstract base for item view selection modes
    QMessageBox,                                                # Standard alert and confirmation dialogs
    QProgressBar,                                               # Visual progress indicator bar
)

from metadata.lookup_service import LookupService               # Provider discovery and orchestration service
from ui.workers import LookupWorker                             # Background thread for metadata lookups
from ui.error_dialog import show_error                           # User-friendly error dialog

logger = logging.getLogger("MeedyaManager.LookupPanel")


class LookupPanel(QWidget):
    """
    Metadata lookup panel — added as a tab ("Lookup") in the MainWindow.

    Provides an interface for searching online metadata providers for
    information about a loaded media file. Users can:
    - Load a file and view its current metadata
    - Select which providers to search (grouped by category)
    - View ranked results in a table sorted by confidence
    - Inspect full result details in a side panel
    - Apply selected result tags to the file
    - Batch-process multiple files

    Signals:
        lookup_completed (list): Emitted when lookup results arrive from
            the background worker. Payload is a list of ProviderResult objects.
        tags_applied (str, dict): Emitted when tags from a selected result
            are applied to a file. Payload is (filepath, {tag_key: tag_value}).
    """

    # =========================================================================
    # Signal Definitions
    # =========================================================================

    # Emitted when the background LookupWorker returns results.
    # Connected by parent widgets or controllers that need to react to lookups.
    lookup_completed = Signal(list)

    # Emitted when the user applies a selected result's tags to the media file.
    # Carries the file path and a dict of {internal_key: new_value} tag changes.
    tags_applied = Signal(str, dict)

    def __init__(self, parent=None):
        """
        Initialize the lookup panel with all child widgets and internal state.

        Args:
            parent (QWidget, optional): Parent widget for Qt object hierarchy.
        """
        super().__init__(parent)

        # The file path currently loaded for lookup
        self._current_filepath = None                           # str or None

        # List of file paths for batch lookup operations
        self._batch_filepaths = []                              # list[str]

        # Extracted metadata from the current file (populated by load_file)
        self._current_metadata = {}                             # dict of tag key → value

        # The list of ProviderResult objects from the most recent lookup
        self._results = []                                      # list[ProviderResult]

        # Reference to the background lookup worker thread (created on demand)
        self._lookup_worker = None                              # LookupWorker or None

        # LookupService instance for provider discovery (get_available_providers)
        self._lookup_service = LookupService()

        # Dictionary mapping provider name → QCheckBox widget for dynamic toggling
        self._provider_checkboxes = {}                          # {str: QCheckBox}

        # Build all UI components and arrange them in the layout
        self._setup_ui()

    # =========================================================================
    # UI Setup
    # =========================================================================

    def _setup_ui(self):
        """
        Create and arrange all child widgets in the panel layout.

        The layout is structured as:
          [Top]    File info label + provider checkboxes + Search button
          [Middle] QSplitter: results table (left) | detail panel (right)
          [Bottom] Progress bar + Apply Selected / Batch Lookup buttons
        """
        # Root vertical layout — contains all sections from top to bottom
        root_layout = QVBoxLayout(self)
        root_layout.setContentsMargins(12, 12, 12, 12)         # Uniform 12px margins

        # --- Top Section: File Info + Provider Selection + Search ---
        self._setup_top_section(root_layout)

        # --- Middle Section: Results Table + Detail Panel (in a splitter) ---
        self._setup_middle_section(root_layout)

        # --- Progress Bar (hidden until a lookup is in progress) ---
        self._setup_progress_bar(root_layout)

        # --- Bottom Section: Action Buttons ---
        self._setup_bottom_section(root_layout)

        # --- Status Label (informational messages) ---
        self._status_label = QLabel("Ready — load a file to begin lookup")
        root_layout.addWidget(self._status_label)

    def _setup_top_section(self, parent_layout):
        """
        Build the top section containing:
        - File info label showing the loaded file path and basic metadata
        - Provider checkboxes grouped by category (Music, Video, Podcast, Identifier)
        - "Search" button to initiate the lookup

        Args:
            parent_layout (QVBoxLayout): The root layout to add widgets to.
        """
        # --- File Info Label ---
        # Displays the currently loaded file's name and key metadata fields.
        # Updated by load_file() when a file is selected.
        self._file_info_label = QLabel(
            "No file loaded — select a file from Scan/Preview or Metadata tab"
        )
        self._file_info_label.setWordWrap(True)                 # Allow long paths to wrap
        self._file_info_label.setStyleSheet(
            "QLabel { padding: 6px; border: 1px solid #444; border-radius: 4px; }"
        )
        parent_layout.addWidget(self._file_info_label)

        # --- Provider Selection Group ---
        # Contains a grid of checkboxes for each available provider, organised
        # into columns by their ProviderCategory (Music, Video, Podcast, Identifier).
        provider_group = QGroupBox("Providers")
        provider_outer_layout = QHBoxLayout(provider_group)

        # Build category sub-groups dynamically from the LookupService
        self._build_provider_checkboxes(provider_outer_layout)

        # --- Search Button ---
        # Placed to the right of the provider checkboxes. Triggers _start_lookup()
        # which creates a LookupWorker and runs the search in a background thread.
        search_button_layout = QVBoxLayout()
        search_button_layout.addStretch()                       # Push button to vertical centre

        self._search_btn = QPushButton("Search")
        self._search_btn.setObjectName("primaryButton")         # Accent colour via platform_style
        self._search_btn.setToolTip(
            "Search selected providers for metadata matching the loaded file"
        )
        self._search_btn.setMinimumWidth(100)                   # Ensure button is not too narrow
        self._search_btn.setEnabled(False)                      # Disabled until a file is loaded
        self._search_btn.clicked.connect(self._start_lookup)

        search_button_layout.addWidget(self._search_btn)
        search_button_layout.addStretch()

        provider_outer_layout.addLayout(search_button_layout)

        parent_layout.addWidget(provider_group)

    def _build_provider_checkboxes(self, parent_layout):
        """
        Dynamically create provider checkboxes grouped by category.

        Queries LookupService.get_available_providers() to get all registered
        providers and their status. Each provider becomes a QCheckBox placed
        in a grid under its category heading. Available providers are checked
        by default; unavailable ones are unchecked and disabled (greyed out).

        Each checkbox stores the provider name as a dynamic property so that
        _get_selected_providers() can read it when constructing the lookup query.

        Args:
            parent_layout (QHBoxLayout): Layout to add category group boxes to.
        """
        # Fetch all provider status info from the LookupService
        all_providers = self._lookup_service.get_available_providers()

        # Organise providers into category buckets
        # Keys are display names; values are lists of provider status dicts
        category_buckets = {
            "Music": [],
            "Video": [],
            "Podcast": [],
            "Identifier": [],
        }

        # Map the raw category string values to display names
        category_display_map = {
            "music": "Music",
            "video": "Video",
            "podcast": "Podcast",
            "identifier": "Identifier",
        }

        for provider_info in all_providers:
            # provider_info is a dict with keys: name, category, requires_auth,
            # available, message — as returned by BaseProvider.get_status_info()
            display_category = category_display_map.get(
                provider_info.get("category", ""),
                "Identifier"                                    # Fallback to Identifier
            )
            category_buckets[display_category].append(provider_info)

        # Build a QGroupBox with a QGridLayout for each non-empty category
        for category_name, providers in category_buckets.items():
            if not providers:
                # Skip empty categories (no providers registered for this category)
                continue

            # Create a titled group box for this category (e.g., "Music")
            category_group = QGroupBox(category_name)
            grid = QGridLayout(category_group)
            grid.setContentsMargins(6, 6, 6, 6)                # Compact padding inside group

            # Place checkboxes in a grid with a maximum of 2 columns per category
            max_columns = 2                                     # 2 checkboxes per row
            for index, provider_info in enumerate(providers):
                provider_name = provider_info.get("name", "unknown")
                is_available = provider_info.get("available", False)
                status_message = provider_info.get("message", "")

                # Create the checkbox with a human-readable label
                # Convert "apple_music" → "Apple Music" for display
                display_name = provider_name.replace("_", " ").title()
                checkbox = QCheckBox(display_name)

                # Store the internal provider name as a Qt dynamic property
                # so _get_selected_providers() can retrieve it later
                checkbox.setProperty("provider_name", provider_name)

                # Available providers are checked and enabled by default;
                # unavailable providers are unchecked and greyed out with a tooltip
                if is_available:
                    checkbox.setChecked(True)
                    checkbox.setToolTip(f"{display_name} — {status_message}")
                else:
                    checkbox.setChecked(False)
                    checkbox.setEnabled(False)
                    checkbox.setToolTip(
                        f"{display_name} — {status_message} (not available)"
                    )

                # Calculate grid row and column from the linear index
                row = index // max_columns
                col = index % max_columns
                grid.addWidget(checkbox, row, col)

                # Store reference for later access (toggle all, get selection)
                self._provider_checkboxes[provider_name] = checkbox

            parent_layout.addWidget(category_group)

        # If no providers were found at all, show a placeholder label
        if not self._provider_checkboxes:
            no_providers_label = QLabel(
                "No providers registered.\n"
                "Check provider configuration and credentials."
            )
            no_providers_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
            parent_layout.addWidget(no_providers_label)

    def _setup_middle_section(self, parent_layout):
        """
        Build the middle section with a QSplitter dividing:
        - Left: Results QTableWidget showing search results
        - Right: Detail panel with full metadata and cover art placeholder

        The splitter allows the user to resize the two panes by dragging
        the divider handle.

        Args:
            parent_layout (QVBoxLayout): The root layout to add the splitter to.
        """
        # Create a horizontal splitter (left pane | right pane)
        self._splitter = QSplitter(Qt.Orientation.Horizontal)

        # --- Left Pane: Results Table ---
        self._setup_results_table()
        self._splitter.addWidget(self._results_table)

        # --- Right Pane: Detail Panel ---
        self._setup_detail_panel()
        self._splitter.addWidget(self._detail_widget)

        # Set initial splitter proportions: 65% results table, 35% detail panel
        self._splitter.setSizes([650, 350])

        parent_layout.addWidget(self._splitter)

    def _setup_results_table(self):
        """
        Create the results QTableWidget with columns:
        #, Provider, Confidence (%), Title, Artist, Album, Year.

        The table uses single-row selection mode so the user can click a
        result row to see its details in the right-hand detail panel.
        Sorting is enabled on all columns.
        """
        self._results_table = QTableWidget()

        # Define column headers for the results display
        column_headers = [
            "#",                                                # Row number (1-indexed)
            "Provider",                                         # Provider name that returned this result
            "Confidence (%)",                                   # Match confidence as a percentage
            "Title",                                            # Track/movie/episode title
            "Artist",                                           # Artist name(s)
            "Album",                                            # Album or show name
            "Year",                                             # Release year
        ]
        self._results_table.setColumnCount(len(column_headers))
        self._results_table.setHorizontalHeaderLabels(column_headers)

        # Configure table behaviour
        self._results_table.setAlternatingRowColors(True)       # Zebra-striped rows for readability
        self._results_table.setSelectionBehavior(
            QAbstractItemView.SelectionBehavior.SelectRows      # Click selects the entire row
        )
        self._results_table.setSelectionMode(
            QAbstractItemView.SelectionMode.SingleSelection     # Only one result selected at a time
        )
        self._results_table.setSortingEnabled(True)             # Clickable column headers for sorting
        self._results_table.setEditTriggers(
            QAbstractItemView.EditTrigger.NoEditTriggers        # Read-only — no inline editing
        )

        # Column sizing — stretch Title/Artist/Album, fit others to content
        header = self._results_table.horizontalHeader()
        header.setSectionResizeMode(0, QHeaderView.ResizeMode.ResizeToContents)   # #
        header.setSectionResizeMode(1, QHeaderView.ResizeMode.ResizeToContents)   # Provider
        header.setSectionResizeMode(2, QHeaderView.ResizeMode.ResizeToContents)   # Confidence
        header.setSectionResizeMode(3, QHeaderView.ResizeMode.Stretch)            # Title
        header.setSectionResizeMode(4, QHeaderView.ResizeMode.Stretch)            # Artist
        header.setSectionResizeMode(5, QHeaderView.ResizeMode.Stretch)            # Album
        header.setSectionResizeMode(6, QHeaderView.ResizeMode.ResizeToContents)   # Year

        # Connect row selection changes to the detail panel update slot
        self._results_table.itemSelectionChanged.connect(self._on_result_selected)

    def _setup_detail_panel(self):
        """
        Create the right-hand detail panel containing:
        - Cover art thumbnail placeholder (200x200 label)
        - Full metadata display (QTextEdit, read-only)
        - "Apply" button to write the selected result's tags to the file

        The detail panel is wrapped in a QWidget with a vertical layout
        so it can be added to the QSplitter as a single unit.
        """
        # Container widget for the detail panel contents
        self._detail_widget = QWidget()
        detail_layout = QVBoxLayout(self._detail_widget)
        detail_layout.setContentsMargins(8, 8, 8, 8)           # Compact margins inside panel

        # --- Detail Panel Header ---
        detail_header = QLabel("Result Details")
        detail_header.setStyleSheet("QLabel { font-weight: bold; font-size: 13px; }")
        detail_layout.addWidget(detail_header)

        # --- Cover Art Thumbnail Placeholder ---
        # Displays cover art from the selected result (or a placeholder message).
        # Actual image loading is handled in _on_result_selected().
        self._detail_cover_label = QLabel("No Cover Art")
        self._detail_cover_label.setFixedSize(200, 200)
        self._detail_cover_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self._detail_cover_label.setStyleSheet(
            "QLabel { border: 1px solid #555; background-color: #222; color: #888; }"
        )
        detail_layout.addWidget(
            self._detail_cover_label,
            alignment=Qt.AlignmentFlag.AlignHCenter             # Centre the thumbnail horizontally
        )

        # --- Full Metadata Display ---
        # Read-only QTextEdit showing all metadata fields from the selected result.
        # Populated by _on_result_selected() with formatted key-value pairs.
        self._detail_text = QTextEdit()
        self._detail_text.setReadOnly(True)                     # Users cannot edit result details
        self._detail_text.setPlaceholderText(
            "Select a result from the table to view its full metadata here."
        )
        detail_layout.addWidget(self._detail_text)

        # --- Apply Button ---
        # Writes the selected result's tags to the currently loaded file.
        # Disabled until a result is selected.
        self._apply_btn = QPushButton("Apply")
        self._apply_btn.setToolTip(
            "Apply the selected result's metadata tags to the loaded file"
        )
        self._apply_btn.setEnabled(False)                       # Disabled until a result is selected
        self._apply_btn.clicked.connect(self._apply_selected)
        detail_layout.addWidget(self._apply_btn)

    def _setup_progress_bar(self, parent_layout):
        """
        Create a progress bar that is hidden by default and shown
        during active lookup operations.

        Args:
            parent_layout (QVBoxLayout): The root layout to add the bar to.
        """
        self._progress_bar = QProgressBar()
        self._progress_bar.setVisible(False)                    # Hidden until a lookup starts
        self._progress_bar.setTextVisible(True)                 # Show percentage text inside bar
        parent_layout.addWidget(self._progress_bar)

    def _setup_bottom_section(self, parent_layout):
        """
        Build the bottom action bar with:
        - "Apply Selected" button — writes selected result tags to the file
        - "Batch Lookup" button — processes multiple files in sequence

        Args:
            parent_layout (QVBoxLayout): The root layout to add the bar to.
        """
        button_layout = QHBoxLayout()

        # Stretch pushes the action buttons to the right
        button_layout.addStretch()

        # --- Apply Selected Button ---
        # Duplicates the detail panel Apply button for convenience at the bottom.
        self._apply_selected_btn = QPushButton("Apply Selected")
        self._apply_selected_btn.setToolTip(
            "Apply the selected result's metadata tags to the loaded file"
        )
        self._apply_selected_btn.setEnabled(False)              # Disabled until a result is selected
        self._apply_selected_btn.clicked.connect(self._apply_selected)
        button_layout.addWidget(self._apply_selected_btn)

        # --- Batch Lookup Button ---
        # Processes all files in self._batch_filepaths sequentially,
        # running a lookup for each and optionally auto-applying the best match.
        self._batch_lookup_btn = QPushButton("Batch Lookup")
        self._batch_lookup_btn.setToolTip(
            "Search providers for all loaded files in sequence"
        )
        self._batch_lookup_btn.setEnabled(False)                # Disabled until batch files are loaded
        self._batch_lookup_btn.clicked.connect(self._start_batch_lookup)
        button_layout.addWidget(self._batch_lookup_btn)

        parent_layout.addLayout(button_layout)

    # =========================================================================
    # Public Methods — Called by MainWindow and Other Panels
    # =========================================================================

    def load_file(self, filepath):
        """
        Load a single media file for metadata lookup.

        Extracts the file's embedded metadata using the core extractor
        and updates the file info label with the file path and key tags.
        Enables the Search button so the user can start a lookup.

        Args:
            filepath (str): Absolute path to the media file to look up.
        """
        self._current_filepath = filepath                       # Store the file path
        self._batch_filepaths = [filepath]                      # Also populate batch list
        self._results = []                                      # Clear any previous results
        self._results_table.setRowCount(0)                      # Clear the results table display
        self._detail_text.clear()                               # Clear the detail panel
        self._detail_cover_label.setText("No Cover Art")        # Reset cover art placeholder

        # Extract metadata from the file using the core metadata extractor.
        # This provides the query fields (title, artist, album, etc.) that
        # are sent to providers during the lookup search.
        try:
            from core.metadata_extractor import extract_metadata
            self._current_metadata = extract_metadata(filepath)
        except Exception as e:
            # If metadata extraction fails, log the error and show a warning
            # but still allow the user to try a lookup with limited info
            logger.warning(f"Failed to extract metadata from {filepath}: {e}")
            self._current_metadata = {}

        # Build a display string showing the file name and key metadata fields
        filename = os.path.basename(filepath)
        title = self._current_metadata.get("title", "")
        artist = self._current_metadata.get("artist", "")
        album = self._current_metadata.get("album", "")

        # Assemble file info lines for the label
        info_lines = [f"File: {filename}"]
        if title:
            info_lines.append(f"Title: {title}")
        if artist:
            info_lines.append(f"Artist: {artist}")
        if album:
            info_lines.append(f"Album: {album}")
        if not (title or artist or album):
            info_lines.append("(No metadata detected — search will use filename)")

        self._file_info_label.setText("  |  ".join(info_lines))

        # Enable the Search button now that a file is loaded
        self._search_btn.setEnabled(True)

        # Disable batch button (only one file loaded, not a batch)
        self._batch_lookup_btn.setEnabled(False)

        # Disable apply buttons (no result selected yet)
        self._apply_btn.setEnabled(False)
        self._apply_selected_btn.setEnabled(False)

        self._status_label.setText(f"Loaded: {filename}")
        logger.info(f"Loaded file for lookup: {filepath}")

    def load_files(self, filepaths):
        """
        Load multiple media files for batch lookup.

        Loads the first file for immediate lookup display and stores
        the full list for batch processing via the "Batch Lookup" button.

        Args:
            filepaths (list[str]): List of absolute file paths to process.
        """
        if not filepaths:
            return

        # Store the complete list of file paths for batch operations
        self._batch_filepaths = list(filepaths)

        # Load the first file for immediate display and single-file lookup
        self.load_file(filepaths[0])

        # If more than one file, enable the Batch Lookup button
        if len(filepaths) > 1:
            self._batch_lookup_btn.setEnabled(True)
            self._status_label.setText(
                f"Loaded: {os.path.basename(filepaths[0])} "
                f"({len(filepaths)} files for batch lookup)"
            )

    # =========================================================================
    # Lookup Execution Slots
    # =========================================================================

    def _get_selected_providers(self):
        """
        Collect the names of all checked (selected) provider checkboxes.

        Iterates through all dynamically created provider checkboxes and
        returns the internal provider names for those that are both enabled
        and checked by the user.

        Returns:
            list[str]: Provider names to include in the lookup search,
                e.g., ["spotify", "apple_music", "musicbrainz"].
        """
        selected = []
        for provider_name, checkbox in self._provider_checkboxes.items():
            if checkbox.isEnabled() and checkbox.isChecked():
                selected.append(provider_name)
        return selected

    def _start_lookup(self):
        """
        Initiate a background metadata lookup for the currently loaded file.

        Creates a new LookupWorker thread with the current file's metadata
        and the user's selected providers. Connects the worker's signals
        to the appropriate slots for progress updates, result delivery,
        and error handling.

        Connected to: Search button clicked signal.
        """
        # Guard: no file loaded
        if not self._current_filepath or not self._current_metadata:
            self._status_label.setText("No file loaded — load a file first")
            return

        # Guard: prevent starting multiple lookups simultaneously
        if self._lookup_worker and self._lookup_worker.isRunning():
            self._status_label.setText("Lookup already in progress...")
            return

        # Collect the user's selected providers from the checkboxes
        selected_providers = self._get_selected_providers()
        if not selected_providers:
            self._status_label.setText(
                "No providers selected — check at least one provider"
            )
            return

        # Clear previous results from the table and detail panel
        self._results = []
        self._results_table.setRowCount(0)
        self._detail_text.clear()
        self._detail_cover_label.setText("No Cover Art")
        self._apply_btn.setEnabled(False)
        self._apply_selected_btn.setEnabled(False)

        # Update UI state to indicate an active lookup
        self._search_btn.setEnabled(False)                      # Prevent double-clicks
        self._progress_bar.setVisible(True)
        self._progress_bar.setValue(0)
        self._status_label.setText("Searching providers...")

        # Create the background worker with the extracted metadata and selected providers
        self._lookup_worker = LookupWorker(
            metadata=self._current_metadata,
            provider_names=selected_providers,
        )

        # Connect worker signals to UI update slots
        self._lookup_worker.progress.connect(self._on_lookup_progress)
        self._lookup_worker.result_ready.connect(self._on_results_ready)
        self._lookup_worker.error.connect(self._on_lookup_error)
        self._lookup_worker.worker_error.connect(self._on_worker_crash)
        self._lookup_worker.provider_searched.connect(self._on_provider_searched)
        self._lookup_worker.finished.connect(self._on_lookup_finished)

        # Start the background thread
        self._lookup_worker.start()
        logger.info(
            f"Lookup started for {os.path.basename(self._current_filepath)} "
            f"with {len(selected_providers)} provider(s)"
        )

    def _start_batch_lookup(self):
        """
        Initiate a sequential lookup for all files in the batch list.

        For each file in self._batch_filepaths, loads the file and runs
        a lookup. Uses a simple sequential approach: processes the next
        file when the current lookup completes.

        Connected to: Batch Lookup button clicked signal.
        """
        if not self._batch_filepaths:
            self._status_label.setText("No files loaded for batch lookup")
            return

        if len(self._batch_filepaths) <= 1:
            # Single file — just run a normal lookup
            self._start_lookup()
            return

        # Store the batch queue and start processing the first file
        self._batch_queue = list(self._batch_filepaths)         # Copy the list as a processing queue
        self._batch_index = 0                                   # Current position in the queue
        self._batch_results = []                                # Accumulate results across all files

        # Disable batch button during processing to prevent re-entry
        self._batch_lookup_btn.setEnabled(False)

        self._status_label.setText(
            f"Batch lookup: processing file 1 of {len(self._batch_queue)}..."
        )

        # Load and start the first file in the batch
        self.load_file(self._batch_queue[0])
        self._start_lookup()

    # =========================================================================
    # Worker Signal Slots — Receive Updates from LookupWorker
    # =========================================================================

    def _on_lookup_progress(self, current, total):
        """
        Update the progress bar with the current lookup progress.

        Called by the LookupWorker's progress signal as it searches
        through each selected provider.

        Args:
            current (int): Number of providers searched so far.
            total (int): Total number of providers to search.
        """
        self._progress_bar.setMaximum(total)
        self._progress_bar.setValue(current)
        self._status_label.setText(f"Searching: {current}/{total} providers...")

    def _on_provider_searched(self, provider_name, result_count):
        """
        Handle notification that a single provider has been searched.

        Updates the status label with the provider name and result count.
        Can be used to give the user real-time feedback on which providers
        are returning results.

        Args:
            provider_name (str): Name of the provider that was searched.
            result_count (int): Number of results returned by this provider.
        """
        display_name = provider_name.replace("_", " ").title()
        logger.debug(f"Provider {display_name}: {result_count} result(s)")
        self._status_label.setText(
            f"Searched {display_name}: {result_count} result(s)"
        )

    def _on_results_ready(self, results):
        """
        Populate the results table when the LookupWorker delivers results.

        Receives the full list of ProviderResult objects, stores them
        internally, and adds one row per result to the QTableWidget.
        Results are expected to be pre-sorted by confidence (descending).

        Emits the lookup_completed signal for any connected listeners.

        Args:
            results (list): List of ProviderResult objects from the worker.
        """
        self._results = results                                 # Store results for detail display

        # Clear and rebuild the results table
        self._results_table.setRowCount(0)                      # Remove all existing rows
        self._results_table.setSortingEnabled(False)            # Disable sorting during insertion

        for index, result in enumerate(results):
            row = self._results_table.rowCount()
            self._results_table.insertRow(row)

            # Column 0: Row number (1-indexed for human readability)
            num_item = QTableWidgetItem(str(index + 1))
            num_item.setTextAlignment(
                Qt.AlignmentFlag.AlignCenter                    # Centre-align the row number
            )
            self._results_table.setItem(row, 0, num_item)

            # Column 1: Provider name (human-readable format)
            provider_display = result.provider_name.replace("_", " ").title()
            provider_item = QTableWidgetItem(provider_display)
            self._results_table.setItem(row, 1, provider_item)

            # Column 2: Confidence score as a percentage (e.g., 0.85 → "85.0")
            confidence_pct = f"{result.confidence * 100:.1f}"
            confidence_item = QTableWidgetItem(confidence_pct)
            confidence_item.setTextAlignment(
                Qt.AlignmentFlag.AlignRight | Qt.AlignmentFlag.AlignVCenter
            )
            # Store the raw float value as sort data so numeric sorting works correctly
            confidence_item.setData(Qt.ItemDataRole.UserRole, result.confidence)
            self._results_table.setItem(row, 2, confidence_item)

            # Column 3: Title
            title_item = QTableWidgetItem(result.title or "(Unknown)")
            self._results_table.setItem(row, 3, title_item)

            # Column 4: Artist
            artist_item = QTableWidgetItem(result.artist or "")
            self._results_table.setItem(row, 4, artist_item)

            # Column 5: Album
            album_item = QTableWidgetItem(result.album or "")
            self._results_table.setItem(row, 5, album_item)

            # Column 6: Year
            year_item = QTableWidgetItem(result.year or "")
            year_item.setTextAlignment(Qt.AlignmentFlag.AlignCenter)
            self._results_table.setItem(row, 6, year_item)

        # Re-enable sorting now that all rows are inserted
        self._results_table.setSortingEnabled(True)

        # Update status with the total result count
        count = len(results)
        self._status_label.setText(
            f"Lookup complete — {count} result{'s' if count != 1 else ''} found"
        )

        # Emit the lookup_completed signal for any external listeners
        self.lookup_completed.emit(results)

        logger.info(f"Lookup complete: {count} results displayed")

    def _on_lookup_error(self, error_message):
        """
        Handle an error reported by the LookupWorker.

        Displays the error message in the status label and logs it.

        Args:
            error_message (str): Human-readable error description from the worker.
        """
        self._status_label.setText(f"Lookup error: {error_message}")
        logger.error(f"Lookup error: {error_message}")

    def _on_worker_crash(self, title, detail):
        """Handle unexpected worker crash — show ErrorDialog to the user."""
        self._status_label.setText(f"Error: {title}")
        show_error(self, RuntimeError(detail), context="worker")

    def _on_lookup_finished(self):
        """
        Reset UI state when the LookupWorker thread finishes execution.

        Re-enables the Search button, hides the progress bar, and
        handles batch continuation if a batch lookup is in progress.
        """
        # Restore interactive controls
        self._search_btn.setEnabled(True)
        self._progress_bar.setVisible(False)

        # Handle batch continuation — if we're processing a batch queue,
        # advance to the next file when the current lookup completes
        if hasattr(self, '_batch_queue') and self._batch_queue:
            self._batch_index += 1
            if self._batch_index < len(self._batch_queue):
                # More files to process — load and search the next one
                next_file = self._batch_queue[self._batch_index]
                total = len(self._batch_queue)
                self._status_label.setText(
                    f"Batch lookup: processing file "
                    f"{self._batch_index + 1} of {total}..."
                )
                self.load_file(next_file)
                self._start_lookup()
            else:
                # Batch complete — clean up and re-enable the batch button
                self._batch_queue = []
                self._batch_lookup_btn.setEnabled(
                    len(self._batch_filepaths) > 1
                )
                self._status_label.setText(
                    f"Batch lookup complete — "
                    f"processed {self._batch_index} file(s)"
                )
                logger.info(f"Batch lookup complete: {self._batch_index} files processed")

    # =========================================================================
    # Result Selection and Detail Display
    # =========================================================================

    def _on_result_selected(self):
        """
        Update the detail panel when the user selects a result row in the table.

        Reads the ProviderResult for the selected row and populates the
        detail QTextEdit with all metadata fields. Updates the cover art
        placeholder to show the number of available cover art assets.
        Enables the Apply buttons.

        Connected to: results_table.itemSelectionChanged signal.
        """
        # Determine which row is selected
        selected_rows = self._results_table.selectionModel().selectedRows()
        if not selected_rows:
            # No selection — clear the detail panel
            self._detail_text.clear()
            self._detail_cover_label.setText("No Cover Art")
            self._apply_btn.setEnabled(False)
            self._apply_selected_btn.setEnabled(False)
            return

        # Get the row index from the selection model
        row_index = selected_rows[0].row()

        # Guard: ensure the row index maps to a valid result
        if row_index < 0 or row_index >= len(self._results):
            return

        result = self._results[row_index]

        # --- Build the detail text with all metadata fields ---
        detail_lines = []

        # Provider identification
        detail_lines.append(f"<b>Provider:</b> {result.provider_name.replace('_', ' ').title()}")
        detail_lines.append(f"<b>Confidence:</b> {result.confidence * 100:.1f}%")
        detail_lines.append("")                                 # Blank line separator

        # Standard metadata fields — only show non-empty values
        standard_fields = [
            ("Title", result.title),
            ("Artist", result.artist),
            ("Album", result.album),
            ("Album Artist", result.album_artist),
            ("Year", result.year),
            ("Genre", result.genre),
            ("Track #", result.track_num),
            ("Total Tracks", result.total_tracks),
            ("Disc #", result.disc_num),
            ("Total Discs", result.total_discs),
            ("Composer", result.composer),
            ("ISRC", result.isrc),
            ("BPM", result.bpm),
        ]

        for label, value in standard_fields:
            if value:                                           # Only display non-empty fields
                detail_lines.append(f"<b>{label}:</b> {value}")

        # Video-specific fields (show, season, episode, etc.)
        video_fields = [
            ("Show", result.show),
            ("Season", result.season),
            ("Episode", result.episode),
            ("Episode Title", result.episode_title),
            ("Director", result.director),
        ]

        has_video_fields = any(v for _, v in video_fields)
        if has_video_fields:
            detail_lines.append("")                             # Separator
            detail_lines.append("<b>--- Video Details ---</b>")
            for label, value in video_fields:
                if value:
                    detail_lines.append(f"<b>{label}:</b> {value}")

        # Provider-specific identification
        if result.provider_id or result.provider_url:
            detail_lines.append("")                             # Separator
            detail_lines.append("<b>--- Provider Info ---</b>")
            if result.provider_id:
                detail_lines.append(f"<b>Provider ID:</b> {result.provider_id}")
            if result.provider_url:
                detail_lines.append(f"<b>Provider URL:</b> {result.provider_url}")

        # Extra tags (provider-specific custom metadata)
        if result.extra_tags:
            detail_lines.append("")                             # Separator
            detail_lines.append("<b>--- Extra Tags ---</b>")
            for key, value in sorted(result.extra_tags.items()):
                # Format the key for display: "custom_spotify_energy" → "Spotify Energy"
                display_key = key.replace("custom_", "").replace("_", " ").title()
                detail_lines.append(f"<b>{display_key}:</b> {value}")

        # Lyrics preview (truncated if long)
        if result.lyrics:
            detail_lines.append("")                             # Separator
            detail_lines.append("<b>--- Lyrics ---</b>")
            preview = result.lyrics[:300]                       # First 300 characters
            if len(result.lyrics) > 300:
                preview += "..."
            detail_lines.append(preview)

        # Set the assembled HTML content in the detail text widget
        self._detail_text.setHtml("<br>".join(detail_lines))

        # --- Update Cover Art Placeholder ---
        cover_count = len(result.cover_art) if result.cover_art else 0
        if cover_count > 0:
            # Show the number of available cover art assets (actual image loading
            # will be handled by a future enhancement using CoverArtManager)
            asset_types = [asset.asset_type.value for asset in result.cover_art]
            self._detail_cover_label.setText(
                f"Cover Art Available\n"
                f"{cover_count} asset(s)\n"
                f"Types: {', '.join(set(asset_types))}"
            )
        else:
            self._detail_cover_label.setText("No Cover Art")

        # Enable the Apply buttons now that a result is selected
        self._apply_btn.setEnabled(True)
        self._apply_selected_btn.setEnabled(True)

    # =========================================================================
    # Apply Selected Result
    # =========================================================================

    def _apply_selected(self):
        """
        Apply the currently selected result's metadata tags to the loaded file.

        Shows a confirmation dialog listing all tags that will be written,
        then uses the LookupService to apply the result (write tags and
        download cover art). Emits the tags_applied signal on success.

        Connected to: Apply button and Apply Selected button clicked signals.
        """
        # Guard: no file loaded
        if not self._current_filepath:
            self._status_label.setText("No file loaded — cannot apply tags")
            return

        # Guard: no result selected
        selected_rows = self._results_table.selectionModel().selectedRows()
        if not selected_rows:
            self._status_label.setText("No result selected — select a result first")
            return

        row_index = selected_rows[0].row()
        if row_index < 0 or row_index >= len(self._results):
            return

        result = self._results[row_index]

        # Gather all tags that will be written (standard + custom)
        all_tags = result.get_all_tags()

        if not all_tags:
            self._status_label.setText("Selected result has no tags to apply")
            return

        # Build a confirmation message listing all tag changes
        change_lines = []
        for key, value in sorted(all_tags.items()):
            # Format key for display: "album_artist" → "Album Artist"
            display_key = key.replace("_", " ").title()
            # Truncate very long values (e.g., lyrics)
            display_value = value if len(value) <= 80 else value[:77] + "..."
            change_lines.append(f"  {display_key}: {display_value}")

        confirm_message = (
            f"Apply {len(all_tags)} tag(s) from "
            f"{result.provider_name.replace('_', ' ').title()} "
            f"(confidence: {result.confidence * 100:.1f}%)?\n\n"
            + "\n".join(change_lines)
            + f"\n\nFile: {os.path.basename(self._current_filepath)}"
        )

        # Show confirmation dialog — default to "No" for safety
        reply = QMessageBox.question(
            self,
            "Confirm Apply Tags",
            confirm_message,
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No,
        )

        if reply != QMessageBox.StandardButton.Yes:
            self._status_label.setText("Apply cancelled")
            return

        # Apply the result using LookupService.apply_result_sync()
        # This writes tags and downloads cover art synchronously.
        try:
            changes = self._lookup_service.apply_result_sync(
                filepath=self._current_filepath,
                result=result,
                write_tags=True,
                download_art=True,
            )

            # Determine how many tags were actually written
            tags_written = changes.get("tags_written", {})
            cover_saved = changes.get("cover_art_saved", {})

            self._status_label.setText(
                f"Applied {len(tags_written)} tag(s)"
                + (f", {len(cover_saved)} cover art asset(s)" if cover_saved else "")
                + f" from {result.provider_name.replace('_', ' ').title()}"
            )

            # Emit the tags_applied signal so other panels (Metadata Editor)
            # can refresh their display with the newly written tags
            self.tags_applied.emit(self._current_filepath, tags_written)

            logger.info(
                f"Applied {len(tags_written)} tags from {result.provider_name} "
                f"to {self._current_filepath}"
            )

        except Exception as e:
            error_msg = f"Failed to apply tags: {e}"
            self._status_label.setText(error_msg)
            logger.error(error_msg)
            QMessageBox.warning(self, "Apply Error", error_msg)
