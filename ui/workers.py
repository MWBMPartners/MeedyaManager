# ============================================================================
# File: /ui/workers.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Background worker threads for the MeedyaManager GUI.
# Uses QThread to perform metadata extraction and rename simulation
# without blocking the main UI thread. Emits progress and result signals.
#
# All workers inherit from SafeWorker, which provides a safety-net
# try/except around the worker's main logic. If an unhandled exception
# escapes the worker's own error handling, SafeWorker catches it,
# logs it, and emits a worker_error signal so the GUI can show an
# ErrorDialog instead of silently crashing the thread.
# ============================================================================

import os                                              # File path operations
import logging                                         # Structured logging
import traceback                                       # Traceback formatting for crash reports

from PySide6.QtCore import QThread, Signal             # Thread and signal infrastructure

from metadata.providers import ProviderCategory        # Provider category enum for lookup filtering

from core.metadata_extractor import extract_metadata   # Core metadata extraction pipeline
from core.renamer import simulate_rename               # Dry-run rename path computation
from utils.config_loader import get_config             # Config access for extensions
from core.companion_tracker import (
    find_companions,                                   # Companion file detection
    get_companion_summary,                             # Human-readable companion summary
)

logger = logging.getLogger("MeedyaManager.Workers")


# ============================================================================
# SafeWorker Base Class
# ============================================================================

class SafeWorker(QThread):
    """
    Base class for all MeedyaManager background workers.

    Provides a safety-net exception handler that catches any unhandled
    exception from the worker's safe_run() method and emits a
    worker_error signal with the error details. This ensures that
    unexpected crashes in background threads are reported to the GUI
    instead of being silently swallowed.

    Subclasses must implement safe_run() instead of run().

    Signals:
        worker_error(str, str): Emits (error_title, error_detail) when an
            unhandled exception escapes the worker's own error handling.
            Connected to the ErrorDialog in the GUI for user-friendly display.
    """

    # Safety-net error signal — emitted for truly unexpected crashes
    worker_error = Signal(str, str)                    # (error_title, error_detail)

    def run(self):
        """
        Wraps safe_run() in a try/except safety net.

        If safe_run() raises an exception that is not caught by the
        worker's own error handling, this method catches it, logs the
        full traceback, and emits worker_error so the GUI can show an
        ErrorDialog.

        Subclasses should NOT override run() — override safe_run() instead.
        """
        try:
            self.safe_run()
        except Exception as e:
            # Format the full traceback for logging and error reporting
            tb_text = traceback.format_exc()
            worker_name = self.__class__.__name__

            # Log the crash through the centralized logging system
            logger.error(
                f"Unhandled exception in {worker_name}:\n{tb_text}"
            )

            # Emit the safety-net error signal for the GUI
            self.worker_error.emit(
                f"{worker_name} Error",
                f"{type(e).__name__}: {e}",
            )

    def safe_run(self):
        """
        Main worker logic — must be implemented by subclasses.

        This method is called by run() inside a safety-net try/except.
        Workers should implement their own error handling within safe_run()
        for expected failure modes (e.g., file not found, network errors).
        The SafeWorker safety net only catches truly unexpected exceptions.

        Raises:
            NotImplementedError: If the subclass does not override this method.
        """
        raise NotImplementedError(
            f"{self.__class__.__name__} must implement safe_run()"
        )


class ScanWorker(SafeWorker):
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

    def safe_run(self):
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


class TagWriteWorker(SafeWorker):
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

    def safe_run(self):
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


class LookupWorker(SafeWorker):
    """
    Background worker for performing metadata lookups against external providers.

    Runs async provider searches in a dedicated thread with its own asyncio
    event loop, keeping the GUI responsive while querying services like
    MusicBrainz, Spotify, Apple Music, etc.

    The worker bridges the async LookupService (which uses asyncio.gather
    for parallel provider searches) into a QThread that emits Qt signals
    for progress updates and results.

    Signals:
        progress(int, int): Emits (current_provider_index, total_providers) as
            each provider is searched. Useful for progress bar updates.
        result_ready(list): Emits the complete list of ProviderResult objects
            once all providers have been searched, scored, and ranked.
        error(str): Emits an error message string if the lookup fails
            (e.g., network timeout, all providers unavailable).
        provider_searched(str, int): Emits (provider_name, result_count)
            after each individual provider completes, for live status updates.
    """

    # Signal definitions — connected by the LookupPanel in the GUI
    progress = Signal(int, int)            # (current, total) for progress bar updates
    result_ready = Signal(list)            # Complete ProviderResult list when lookup finishes
    error = Signal(str)                    # Error message string on failure
    provider_searched = Signal(str, int)   # (provider_name, result_count) per provider

    def __init__(self, metadata, provider_names=None, category=None,
                 min_confidence=0.0, parent=None):
        """
        Initialize the lookup worker with search parameters.

        Args:
            metadata (dict): File metadata used for the lookup query.
                Expected keys include "title", "artist", "album", "isrc", etc.
                The LookupService uses these to search and score results.
            provider_names (list[str] | None): Optional list of specific provider
                names to search (e.g., ["spotify", "musicbrainz"]). If None,
                all available providers are searched.
            category (ProviderCategory | None): Optional category filter to restrict
                searches to a specific content domain (MUSIC, VIDEO, PODCAST, etc.).
            min_confidence (float): Minimum confidence threshold (0.0 to 1.0).
                Results below this score are excluded from the final list.
            parent: Parent QObject for proper Qt object lifecycle management.
        """
        super().__init__(parent)

        # Store lookup parameters as private attributes
        self._metadata = metadata                      # File metadata dict for the search query
        self._provider_names = provider_names          # Optional provider name filter
        self._category = category                      # Optional ProviderCategory filter
        self._min_confidence = min_confidence          # Confidence floor for result filtering

    def safe_run(self):
        """
        Execute the metadata lookup in a background thread.

        Creates a dedicated asyncio event loop for this thread (since Qt
        threads don't have one by default), instantiates the LookupService,
        and runs the async lookup. Results are emitted via the result_ready
        signal; errors are emitted via the error signal.

        The event loop is always closed in the finally block to prevent
        resource leaks, even if the lookup raises an exception.
        """
        import asyncio                                 # Async event loop for this thread

        # Lazy import to avoid circular imports — LookupService depends on
        # provider modules which may reference UI components during discovery
        from metadata.lookup_service import LookupService

        # Create a fresh event loop for this thread (QThreads don't have one)
        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)

        try:
            # Instantiate the lookup service (creates scorer, cover art manager)
            service = LookupService()

            # Run the async lookup coroutine in our dedicated event loop
            results = loop.run_until_complete(
                service.lookup(
                    self._metadata,
                    providers=self._provider_names,
                    category=self._category,
                    min_confidence=self._min_confidence,
                )
            )

            # Emit the complete ranked results list to the GUI
            self.result_ready.emit(results)
            logger.info(
                f"Lookup worker complete: {len(results)} results returned"
            )

        except Exception as e:
            # Emit error signal so the GUI can display the failure message
            logger.error(f"Lookup worker error: {e}")
            self.error.emit(str(e))

        finally:
            # Always close the event loop to release resources, regardless
            # of whether the lookup succeeded or failed
            loop.close()
