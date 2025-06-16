# ============================================================================
# File: /core/watcher.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Real-time or polling-based file watcher for monitored directories.
# Detects new media files and queues them for metadata extraction.
# Supports fallback if watchdog is not available.
# Enhanced with log rotation (daily + max size) and metadata redaction.
# Triggers dry-run rename simulation for files via runner logic.
# ============================================================================

import os
import time
import threading
import logging
from queue import Queue
from core.metadata_extractor import extract_metadata
from utils.config_loader import get_config
from cli.runner import simulate_rename  # Integration for simulation trigger
import re

try:
    from watchdog.observers import Observer
    from watchdog.events import FileSystemEventHandler
    WATCHDOG_AVAILABLE = True
except ImportError:
    WATCHDOG_AVAILABLE = False

from logging.handlers import TimedRotatingFileHandler, RotatingFileHandler

config = get_config()
watch_folders = config.get("watch_folders", [])
valid_extensions = config.get("valid_extensions", [])
watch_mode = config.get("watch_mode", "watchdog")  # fallback to polling
simulate_enabled = config.get("simulate_watcher", True)  # global override

# Set up rotating logger
log_path = os.path.join("logs", "watcher_events.log")
os.makedirs(os.path.dirname(log_path), exist_ok=True)
logger = logging.getLogger("watcher")
logger.setLevel(logging.INFO)

# Combine Timed + Size-Based rotation handlers
timed_handler = TimedRotatingFileHandler(log_path, when="midnight", interval=1, backupCount=7)
size_handler = RotatingFileHandler(log_path, maxBytes=5 * 1024 * 1024, backupCount=5)
formatter = logging.Formatter("[%(asctime)s] %(message)s")
timed_handler.setFormatter(formatter)
size_handler.setFormatter(formatter)
logger.addHandler(timed_handler)
logger.addHandler(size_handler)

# Shared event queue for CLI/controller
event_queue = Queue()


def redact_metadata(data):
    redacted = {}
    for key, value in data.items():
        if isinstance(value, str):
            value = re.sub(r"/Users/[\w\-_.]+", "/Users/REDACTED", value)
            value = re.sub(r"C:\\Users\\[\w\-_.]+", "C:/Users/REDACTED", value)
        redacted[key] = value
    return redacted


def is_valid_file(path):
    return os.path.isfile(path) and os.path.splitext(path)[1].lower() in valid_extensions


def handle_file(path):
    if not is_valid_file(path):
        return

    try:
        metadata = extract_metadata(path)
        redacted = redact_metadata(metadata)
        logger.info(f"Detected file: {path}")
        logger.info(f"Extracted metadata: {redacted}")
        event_queue.put((path, metadata))

        # Trigger dry-run rename simulation if allowed
        if simulate_enabled:
            logger.info("[WATCHER] Initiating rename simulation for: %s", path)
            simulated_path = simulate_rename(path, metadata, dry_run=True)
            if simulated_path:
                logger.info("[WATCHER] Simulated path: %s", simulated_path)

    except Exception as e:
        logger.error(f"Error processing {path}: {e}")


# ------------------ Watchdog Event Handler ------------------

class MediaHandler(FileSystemEventHandler):
    def on_created(self, event):
        if not event.is_directory:
            time.sleep(1)
            handle_file(event.src_path)


# ------------------ Polling Fallback ------------------

def polling_loop():
    seen = set()
    while True:
        for folder in watch_folders:
            for root, _, files in os.walk(folder):
                for file in files:
                    full_path = os.path.join(root, file)
                    if full_path not in seen and is_valid_file(full_path):
                        seen.add(full_path)
                        handle_file(full_path)
        time.sleep(10)


# ------------------ Watcher Entry ------------------

def start_watcher():
    if WATCHDOG_AVAILABLE and watch_mode == "watchdog":
        observer = Observer()
        for folder in watch_folders:
            observer.schedule(MediaHandler(), folder, recursive=True)
        observer.start()
        logger.info("Started Watchdog watcher.")
        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            observer.stop()
        observer.join()
    else:
        logger.info("Watchdog not available or disabled, using polling fallback.")
        polling_thread = threading.Thread(target=polling_loop, daemon=True)
        polling_thread.start()
        try:
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            pass