# ============================================================================
# File: /ui/workers.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Background worker threads for the MeedyaManager GUI.
# Uses QThread to perform metadata extraction and rename simulation
# without blocking the main UI thread. Emits progress and result signals.
# ============================================================================

import os                                              # File path operations
import logging                                         # Structured logging

from PySide6.QtCore import QThread, Signal             # Thread and signal infrastructure

from core.metadata_extractor import extract_metadata   # Core metadata extraction pipeline
from core.renamer import simulate_rename               # Dry-run rename path computation
from utils.config_loader import get_config             # Config access for extensions
from core.companion_tracker import (
    find_companions,                                   # Companion file detection
    get_companion_summary,                             # Human-readable companion summary
)

logger = logging.getLogger("MeedyaManager.Workers")


class ScanWorker(QThread):
    """
    Background worker that scans directories for media files,
    extracts metadata, and computes rename previews.

    Runs in a separate thread to keep the GUI responsive during
    potentially long scanning operations.

    Signals:
        progress(int, int): Emits (current_file_index, total_files) for progress tracking
        result_ready(list): Emits the complete list of scan results when finished
        error(str): Emits error message if scanning fails
        file_scanned(dict): Emits individual file result as it's processed
    """

    # Signal definitions — connected by the PreviewPanel
    progress = Signal(int, int)         # (current, total) for progress bar updates
    result_ready = Signal(list)         # Complete results list when scan finishes
    error = Signal(str)                 # Error message string on failure
    file_scanned = Signal(dict)         # Individual file result for live updates

    def __init__(self, scan_paths=None, parent=None):
        """
        Initialize the scan worker with target directories.

        Args:
            scan_paths (list): Directories to scan. If None, uses config watch_paths.
            parent: Parent QObject for proper Qt object lifecycle management.
        """
        super().__init__(parent)

        # Use provided paths or fall back to configured watch directories
        self.scan_paths = scan_paths or get_config("watch_paths", default=["./watch"])

        # Load valid extensions from config (lowercase, with leading dot)
        raw_extensions = get_config("valid_extensions", default=["mp3", "flac", "mp4"])
        self.valid_extensions = [
            ext if ext.startswith(".") else f".{ext}"
            for ext in raw_extensions
        ]

        # Flag to allow graceful cancellation from the UI
        self._cancelled = False

    def cancel(self):
        """
        Request cancellation of the current scan operation.
        The worker checks this flag between file processing steps.
        """
        self._cancelled = True

    def run(self):
        """
        Main scanning loop — runs in a background thread.

        Walks all configured watch directories, extracts metadata from
        files with valid extensions, computes rename previews, and
        emits results progressively.
        """
        results = []

        try:
            # Phase 1: Collect all matching media files across all watch paths
            all_files = []
            for scan_path in self.scan_paths:
                expanded_path = os.path.expanduser(scan_path)   # Expand ~ to home dir
                if not os.path.isdir(expanded_path):
                    logger.warning(f"Scan path does not exist: {expanded_path}")
                    continue

                # Walk the directory tree to find media files
                for root, dirs, files in os.walk(expanded_path):
                    for filename in files:
                        ext = os.path.splitext(filename)[1].lower()
                        if ext in self.valid_extensions:
                            all_files.append(os.path.join(root, filename))

            total = len(all_files)
            logger.info(f"Found {total} media files to scan")

            # Phase 2: Process each file — extract metadata, compute rename
            for index, filepath in enumerate(all_files):
                # Check for cancellation between each file
                if self._cancelled:
                    logger.info("Scan cancelled by user")
                    break

                try:
                    # Extract metadata (includes classification)
                    metadata = extract_metadata(filepath)

                    # Compute proposed rename path
                    proposed_path = None
                    try:
                        proposed_path = simulate_rename(filepath, metadata)
                    except Exception as rename_err:
                        logger.debug(f"Rename simulation failed for {filepath}: {rename_err}")

                    # Detect companion files (subtitles, lyrics, cover art, etc.)
                    companions = find_companions(filepath)

                    # Build the result entry for this file
                    result_entry = {
                        "filepath": filepath,
                        "filename": os.path.basename(filepath),
                        "proposed_path": proposed_path,
                        "metadata": metadata,
                        "companions": companions,
                        "companion_summary": get_companion_summary(companions),
                    }

                    results.append(result_entry)

                    # Emit individual file result for live UI updates
                    self.file_scanned.emit(result_entry)

                except Exception as file_err:
                    logger.warning(f"Failed to process {filepath}: {file_err}")
                    results.append({
                        "filepath": filepath,
                        "filename": os.path.basename(filepath),
                        "proposed_path": None,
                        "metadata": {},
                        "error": str(file_err),
                    })

                # Emit progress update (1-indexed for display)
                self.progress.emit(index + 1, total)

            # Emit final results
            self.result_ready.emit(results)

        except Exception as e:
            logger.error(f"Scan worker error: {e}")
            self.error.emit(str(e))


class TagWriteWorker(QThread):
    """
    Background worker for writing metadata tags to multiple files.

    Runs tag writing in a separate thread to keep the GUI responsive
    during batch editing operations.

    Signals:
        progress(int, int): Emits (current_file_index, total_files)
        file_written(str, dict): Emits (filepath, changes_dict) per successful write
        error(str, str): Emits (filepath, error_message) per failed write
        finished_all(int, int): Emits (success_count, error_count) when complete
    """

    # Signal definitions
    progress = Signal(int, int)         # (current, total) for progress tracking
    file_written = Signal(str, dict)    # (filepath, changes) per file written
    error = Signal(str, str)            # (filepath, error_message) per file error
    finished_all = Signal(int, int)     # (success_count, error_count) final summary

    def __init__(self, file_tags, parent=None):
        """
        Initialize the tag write worker.

        Args:
            file_tags (list[tuple]): List of (filepath, {key: new_value}) tuples.
                Each tuple specifies one file and the tags to write to it.
            parent: Parent QObject for proper lifecycle management.
        """
        super().__init__(parent)
        self._file_tags = file_tags

    def run(self):
        """
        Write tags to each file using TagEditor.
        Emits progress and result signals for each file processed.
        """
        from metadata.editor import TagEditor, TagWriteError, UnsupportedFormatError

        editor = TagEditor()
        total = len(self._file_tags)
        success_count = 0
        error_count = 0

        for index, (filepath, tags) in enumerate(self._file_tags):
            try:
                changes = editor.write_tags(filepath, tags)
                self.file_written.emit(filepath, changes)
                success_count += 1
            except (TagWriteError, UnsupportedFormatError) as e:
                self.error.emit(filepath, str(e))
                error_count += 1
                logger.warning(f"Tag write failed for {filepath}: {e}")
            except Exception as e:
                self.error.emit(filepath, str(e))
                error_count += 1
                logger.error(f"Unexpected error writing tags to {filepath}: {e}")

            # Emit progress update (1-indexed)
            self.progress.emit(index + 1, total)

        # Emit final summary
        self.finished_all.emit(success_count, error_count)
        logger.info(f"Tag write complete: {success_count} succeeded, {error_count} failed")
