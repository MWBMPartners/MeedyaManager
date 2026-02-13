# ============================================================================
# File: /core/watcher.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Real-time or polling-based file watcher for monitored directories.
# Detects new media files and queues them for metadata extraction.
# Supports fallback if watchdog is not available.
# Enhanced with dry-run simulation support.
# Config access now uses safe get_config() wrappers with fallbacks.
#
# Logging: Uses the centralized MeedyaManager logging system configured
# in utils/log_config.py. PII redaction is handled automatically by the
# global PIIRedactionFilter — no manual redact() calls needed.
# ============================================================================

import os                   # File path operations and existence checks
import time                 # Sleep delays for queue stability and polling
import threading            # Daemon thread for background queue worker
import logging              # Structured logging for watcher events
from queue import Queue     # Thread-safe FIFO queue for detected files

# Core imports — watcher uses metadata extraction and rename simulation
from utils.config_loader import get_config          # Safe config access with defaults
from core.metadata_extractor import extract_metadata # Metadata + classification pipeline
from core.renamer import simulate_rename             # Dry-run rename path computation
from core.companion_tracker import (
    find_companions,                                # Companion file detection
    get_companion_summary,                          # Human-readable companion summary
)

# Optional watchdog import with graceful fallback to polling mode
try:
    from watchdog.observers import Observer               # Filesystem event observer
    from watchdog.events import FileSystemEventHandler    # Base class for event handling
    WATCHDOG_AVAILABLE = True
except ImportError:
    WATCHDOG_AVAILABLE = False

# Load safe config values with defaults (keys match config/settings.json5)
watch_paths = get_config("watch_paths", default=["./watch"])         # Directories to monitor
valid_extensions = get_config("valid_extensions", default=[".mp3", ".flac", ".mp4"])
watch_mode = get_config("watch_mode", default="watchdog")            # "watchdog" or "polling"
simulate_enabled = get_config("simulate_watcher", default=True)      # Enable rename simulation

# Logger — inherits handlers and PII redaction from centralized MeedyaManager logger.
# setup_logging() in utils/log_config.py must be called before this module runs.
logger = logging.getLogger("MeedyaManager.Watcher")


# Thread-safe queue for detected files (processed by queue_worker)
event_queue = Queue()


def handle_file(filepath):
    """
    Process a detected media file through the full pipeline:
    1. Log detection (PII redacted by centralized filter)
    2. Extract metadata (includes classification via classify_media)
    3. If simulation enabled, compute and log the proposed rename path
    4. Enqueue the file for further processing

    Args:
        filepath (str): Absolute path to the detected media file
    """
    # Log the detected file — PII redaction is handled automatically
    # by the PIIRedactionFilter in utils/log_config.py
    logger.info(f"Detected file: {filepath}")

    # Extract metadata and classification from the media file
    try:
        metadata = extract_metadata(filepath)
        logger.info(f"Extracted metadata: {metadata}")
    except Exception as e:
        logger.error(f"Failed to extract metadata from {filepath}: {e}")
        return

    # Detect companion files (subtitles, lyrics, cover art, etc.)
    companions = find_companions(filepath)
    if companions:
        logger.info(
            f"Companions for {os.path.basename(filepath)}: "
            f"{get_companion_summary(companions)}"
        )

    # Run dry-run rename simulation if enabled
    if simulate_enabled:
        result = simulate_rename(filepath, metadata)
        if result:
            logger.info(f"Simulated path: {result}")

    # Enqueue the file for downstream processing (future: actual rename)
    event_queue.put(filepath)


def queue_worker():
    """
    Background worker that processes files from the event queue.
    Runs as a daemon thread — waits 1.5s after dequeue to allow
    file writes to complete before processing.
    """
    while True:
        path = event_queue.get()                        # Block until a file is available
        if not os.path.exists(path):                    # File may have been moved/deleted
            logger.warning(f"File disappeared before processing: {path}")
            event_queue.task_done()
            continue
        time.sleep(1.5)                                 # Wait for file copy to finish
        logger.info(f"Processing queued file: {path}")
        handle_file(path)                               # Run full pipeline on the file
        event_queue.task_done()


def valid_media_file(path):
    """Check if a file's extension is in the configured valid_extensions list."""
    ext = os.path.splitext(path)[1].lower()
    return ext in valid_extensions


class WatchHandler(FileSystemEventHandler):
    """Watchdog event handler that queues newly created media files."""
    def on_created(self, event):
        if not event.is_directory and valid_media_file(event.src_path):
            event_queue.put(event.src_path)


def start_watchdog():
    """Start the watchdog observer on all configured watch paths."""
    observer = Observer()
    handler = WatchHandler()
    for folder in watch_paths:                          # Iterate configured watch directories
        if os.path.exists(folder):
            observer.schedule(handler, folder, recursive=True)
        else:
            logger.warning(f"Folder not found: {folder}")
    observer.start()
    logger.info(f"Watching via watchdog: {watch_paths}")
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        observer.stop()
    observer.join()


def start_polling():
    """Placeholder for polling-based file detection (not yet implemented)."""
    logger.info("Polling mode is not yet implemented (placeholder)")
    while True:
        time.sleep(5)


def start_watcher():
    """Start the file watcher: spawns queue worker thread, then starts observer."""
    threading.Thread(target=queue_worker, daemon=True).start()
    if WATCHDOG_AVAILABLE and watch_mode == "watchdog":
        start_watchdog()
    else:
        start_polling()
