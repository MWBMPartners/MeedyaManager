# ============================================================================
# File: /core/watcher.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Watches directories for file creation events and queues new media files
# into an event queue. Designed to run as part of the MetaMancer CLI/daemon.
#
# Uses the `watchdog` library to provide cross-platform support for file
# system event monitoring with low resource overhead.
#
# This module supports real-time file detection, filtering by extension,
# and skip/retry logic for in-use files. Extensions and paths are fully
# configurable via settings.json5.
# ============================================================================

import os
import time
import logging
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler
from utils.config_loader import get_config

# Shared queue for runner.py
from queue import Queue

# Setup logger
logger = logging.getLogger("MetaMancer.Watcher")
logger.setLevel(logging.DEBUG)
handler = logging.StreamHandler()
formatter = logging.Formatter("[%(asctime)s] %(levelname)s - %(message)s")
handler.setFormatter(formatter)
logger.addHandler(handler)

# Queue shared with runner
event_queue = Queue()

# Load file type filters from config
valid_exts = get_config("valid_extensions", ["mp3", "mkv", "flac", "m4a", "mp4"])  # fallback


class MediaFileHandler(FileSystemEventHandler):
    def on_created(self, event):
        if event.is_directory:
            return

        _, ext = os.path.splitext(event.src_path)
        ext = ext.lower().lstrip('.')

        if ext not in valid_exts:
            logger.debug(f"Ignored file with unsupported extension: {event.src_path}")
            return

        # Check if file is locked (in use)
        if is_file_locked(event.src_path):
            logger.warning(f"File locked, queuing retry later: {event.src_path}")
            time.sleep(3)
        else:
            logger.info(f"Queued file for processing: {event.src_path}")
            event_queue.put(event.src_path)


def is_file_locked(filepath):
    """Attempt to open the file in append mode to detect lock state."""
    try:
        with open(filepath, 'a'):
            return False
    except Exception:
        return True


def start_monitoring(paths=None):
    """
    Start monitoring each configured path in a blocking loop. If no paths
    are passed, falls back to those defined in settings.json5.

    Args:
        paths (list[str]): Optional list of folders to monitor
    """
    if paths is None:
        paths = get_config("watch_folders", ["./watch_folder"])

    observer = Observer()
    handler = MediaFileHandler()

    for path in paths:
        abs_path = os.path.abspath(path)
        if not os.path.exists(abs_path):
            logger.warning(f"Path does not exist: {abs_path}")
            continue
        observer.schedule(handler, abs_path, recursive=True)
        logger.info(f"Watching path: {abs_path}")

    observer.start()
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        observer.stop()
    observer.join()