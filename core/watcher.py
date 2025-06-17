# ============================================================================
# File: /core/watcher.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Real-time or polling-based file watcher for monitored directories.
# Detects new media files and queues them for metadata extraction.
# Supports fallback if watchdog is not available.
# Enhanced with log rotation, metadata redaction, and dry-run simulation.
# Config access now uses safe get_config() wrappers with fallbacks.
# ============================================================================

import os
import time
import threading
import logging
from queue import Queue
import re

from utils.config_loader import get_config
from core.metadata_extractor import extract_metadata
from cli.runner import simulate_rename

try:
    from watchdog.observers import Observer
    from watchdog.events import FileSystemEventHandler
    WATCHDOG_AVAILABLE = True
except ImportError:
    WATCHDOG_AVAILABLE = False

from logging.handlers import TimedRotatingFileHandler, RotatingFileHandler

# Load safe config values with defaults
watch_folders = get_config("watch_folders", default=["./watch"])
valid_extensions = get_config("valid_extensions", default=[".mp3", ".flac", ".mp4"])
watch_mode = get_config("watch_mode", default="watchdog")
simulate_enabled = get_config("simulate_watcher", default=True)

# Logger setup
log_path = os.path.join("logs", "watcher_events.log")
os.makedirs(os.path.dirname(log_path), exist_ok=True)
logger = logging.getLogger("watcher")
logger.setLevel(logging.INFO)

# Timed + size-based rotation
timed_handler = TimedRotatingFileHandler(log_path, when="midnight", interval=1, backupCount=7)
size_handler = RotatingFileHandler(log_path, maxBytes=5 * 1024 * 1024, backupCount=5)
formatter = logging.Formatter("[%(asctime)s] %(message)s")
timed_handler.setFormatter(formatter)
size_handler.setFormatter(formatter)
logger.addHandler(timed_handler)
logger.addHandler(size_handler)

# Redaction utility for PII
REDACT_PATTERNS = [re.compile(r"/Users/\w+"), re.compile(r'C:\\Users\\[^"]+')]

def redact(text):
    for pattern in REDACT_PATTERNS:
        text = pattern.sub("<user>", text)
    return text

# Queue and delay for stability
event_queue = Queue()

def queue_worker():
    while True:
        path = event_queue.get()
        if not os.path.exists(path):
            logger.warning(f"⚠️ File disappeared before processing: {path}")
            event_queue.task_done()
            continue
        time.sleep(1.5)  # wait for file to finish copying
        logger.info(f"📥 Queued file: {redact(path)}")
        if simulate_enabled:
            simulate_rename(path)
        event_queue.task_done()


def valid_media_file(path):
    ext = os.path.splitext(path)[1].lower()
    return ext in valid_extensions


class WatchHandler(FileSystemEventHandler):
    def on_created(self, event):
        if not event.is_directory and valid_media_file(event.src_path):
            event_queue.put(event.src_path)


def start_watchdog():
    observer = Observer()
    handler = WatchHandler()
    for folder in watch_folders:
        if os.path.exists(folder):
            observer.schedule(handler, folder, recursive=True)
        else:
            logger.warning(f"⚠️ Folder not found: {folder}")
    observer.start()
    logger.info(f"👁️ Watching via watchdog: {watch_folders}")
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        observer.stop()
    observer.join()


def start_polling():
    logger.info("⚙️ Polling mode is not yet implemented (placeholder)")
    while True:
        time.sleep(5)


def start_watcher():
    threading.Thread(target=queue_worker, daemon=True).start()
    if WATCHDOG_AVAILABLE and watch_mode == "watchdog":
        start_watchdog()
    else:
        start_polling()