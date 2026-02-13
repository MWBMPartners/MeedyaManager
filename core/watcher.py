# ============================================================================
# File: /core/watcher.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Real-time or polling-based file watcher for monitored directories.
# Detects new media files and queues them for metadata extraction.
# Supports fallback if watchdog is not available.
# Enhanced with log rotation, metadata redaction, and dry-run simulation.
# Config access now uses safe get_config() wrappers with fallbacks.
# ============================================================================

import os                   # File path operations and existence checks
import time                 # Sleep delays for queue stability and polling
import threading            # Daemon thread for background queue worker
import logging              # Structured logging for watcher events
from queue import Queue     # Thread-safe FIFO queue for detected files
import re                   # Regex patterns for PII redaction

# Core imports — watcher uses metadata extraction and rename simulation
from utils.config_loader import get_config          # Safe config access with defaults
from core.metadata_extractor import extract_metadata # Metadata + classification pipeline
from core.renamer import simulate_rename             # Dry-run rename path computation

# Optional watchdog import with graceful fallback to polling mode
try:
    from watchdog.observers import Observer               # Filesystem event observer
    from watchdog.events import FileSystemEventHandler    # Base class for event handling
    WATCHDOG_AVAILABLE = True
except ImportError:
    WATCHDOG_AVAILABLE = False

from logging.handlers import TimedRotatingFileHandler, RotatingFileHandler

# Load safe config values with defaults (keys match config/settings.json5)
watch_paths = get_config("watch_paths", default=["./watch"])         # Directories to monitor
valid_extensions = get_config("valid_extensions", default=[".mp3", ".flac", ".mp4"])
watch_mode = get_config("watch_mode", default="watchdog")            # "watchdog" or "polling"
simulate_enabled = get_config("simulate_watcher", default=True)      # Enable rename simulation

# Logger setup — writes to logs/watcher_events.log
log_path = os.path.join("logs", "watcher_events.log")
os.makedirs(os.path.dirname(log_path), exist_ok=True)
logger = logging.getLogger("watcher")
logger.setLevel(logging.INFO)

# Timed rotation (daily, 7 backups) + size rotation (5 MB, 5 backups)
timed_handler = TimedRotatingFileHandler(log_path, when="midnight", interval=1, backupCount=7)
size_handler = RotatingFileHandler(log_path, maxBytes=5 * 1024 * 1024, backupCount=5)
formatter = logging.Formatter("[%(asctime)s] %(message)s")
timed_handler.setFormatter(formatter)
size_handler.setFormatter(formatter)
logger.addHandler(timed_handler)
logger.addHandler(size_handler)

# Redaction patterns to remove usernames from log output (PII safety)
REDACT_PATTERNS = [re.compile(r"/Users/\w+"), re.compile(r'C:\\Users\\[^"]+')]


def redact(text):
    """Replace user-identifiable path segments with <user> for log safety."""
    for pattern in REDACT_PATTERNS:
        text = pattern.sub("<user>", text)
    return text


# Thread-safe queue for detected files (processed by queue_worker)
event_queue = Queue()


def handle_file(filepath):
    """
    Process a detected media file through the full pipeline:
    1. Log detection with PII redaction
    2. Extract metadata (includes classification via classify_media)
    3. If simulation enabled, compute and log the proposed rename path
    4. Enqueue the file for further processing

    Args:
        filepath (str): Absolute path to the detected media file
    """
    # Log the detected file with PII-safe path
    logger.info(f"📂 Detected file: {redact(filepath)}")

    # Extract metadata and classification from the media file
    try:
        metadata = extract_metadata(filepath)
        logger.info(f"📋 Extracted metadata: {redact(str(metadata))}")
    except Exception as e:
        logger.error(f"❌ Failed to extract metadata from {redact(filepath)}: {e}")
        return

    # Run dry-run rename simulation if enabled
    if simulate_enabled:
        result = simulate_rename(filepath, metadata)
        if result:
            logger.info(f"🔄 Simulated path: {redact(str(result))}")

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
            logger.warning(f"⚠️ File disappeared before processing: {path}")
            event_queue.task_done()
            continue
        time.sleep(1.5)                                 # Wait for file copy to finish
        logger.info(f"📥 Processing queued file: {redact(path)}")
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
            logger.warning(f"⚠️ Folder not found: {folder}")
    observer.start()
    logger.info(f"👁️ Watching via watchdog: {watch_paths}")
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        observer.stop()
    observer.join()


def start_polling():
    """Placeholder for polling-based file detection (not yet implemented)."""
    logger.info("⚙️ Polling mode is not yet implemented (placeholder)")
    while True:
        time.sleep(5)


def start_watcher():
    """Start the file watcher: spawns queue worker thread, then starts observer."""
    threading.Thread(target=queue_worker, daemon=True).start()
    if WATCHDOG_AVAILABLE and watch_mode == "watchdog":
        start_watchdog()
    else:
        start_polling()
