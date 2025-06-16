# ============================================================================
# File: /core/watcher.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# This module is part of the "MetaMancer" media management application. It is
# responsible for actively monitoring one or more specified directories for any
# filesystem changes related to media files (e.g., new files added, renamed,
# modified, etc.). It uses the cross-platform Python `watchdog` library to
# implement real-time monitoring and queues new file events for later handling
# by the renaming/sorting engine. It also integrates safety logic to ignore
# files that are still in use by another application.
#
# References:
# - Watchdog Library: https://github.com/gorakhargosh/watchdog
# - Watchdog Docs: https://python-watchdog.readthedocs.io/
# ============================================================================

import os
import time
import threading
import logging
from queue import Queue
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler

# Create a logger for this module
logger = logging.getLogger("MetaMancer.Watcher")
logger.setLevel(logging.DEBUG)
handler = logging.StreamHandler()
formatter = logging.Formatter("[%(asctime)s] %(levelname)s - %(message)s")
handler.setFormatter(formatter)
logger.addHandler(handler)

# A thread-safe queue used to pass detected file events to the rename engine
event_queue = Queue()

# Define a set of valid media file extensions that we care about
# These will eventually be made configurable in the user settings
VALID_EXTENSIONS = {'.mp3', '.m4a', '.flac', '.alac', '.ogg', '.wav',
                    '.mp4', '.mkv', '.avi', '.m4v', '.mov',
                    '.ac3', '.eac3', '.ac4', '.mpg', '.divx'}


def is_file_locked(filepath):
    """
    Check whether the given file is still locked (i.e., in use by another app).
    This prevents accidentally trying to move/copy a file while it's being
    written, avoiding corruption.

    This function tries to open the file in append mode.
    If it fails, the file is probably in use.
    """
    try:
        with open(filepath, 'a'):
            return False  # File is not locked
    except OSError:
        return True  # File is locked or inaccessible


class MediaFileHandler(FileSystemEventHandler):
    """
    A handler class derived from watchdog's FileSystemEventHandler.
    This class responds to file creation and movement events in the monitored directories.
    """

    def on_created(self, event):
        """
        Called when a new file or directory is created.
        We only handle file creation events for supported media types.
        """
        if event.is_directory:
            return  # Ignore folders

        filepath = event.src_path
        ext = os.path.splitext(filepath)[1].lower()

        if ext in VALID_EXTENSIONS:
            logger.info(f"Detected new media file: {filepath}")
            
            # Spawn a thread that waits for the file to be unlocked
            def wait_until_unlocked():
                logger.debug(f"Waiting for {filepath} to be available...")
                while is_file_locked(filepath):
                    time.sleep(1)
                logger.info(f"File ready for processing: {filepath}")
                event_queue.put(filepath)

            threading.Thread(target=wait_until_unlocked, daemon=True).start()

    def on_moved(self, event):
        """
        Called when a file is moved into the directory.
        Behaves similar to on_created.
        """
        if event.is_directory:
            return

        filepath = event.dest_path
        ext = os.path.splitext(filepath)[1].lower()

        if ext in VALID_EXTENSIONS:
            logger.info(f"File moved into watch folder: {filepath}")
            
            def wait_until_unlocked():
                logger.debug(f"Waiting for {filepath} to be available...")
                while is_file_locked(filepath):
                    time.sleep(1)
                logger.info(f"File ready for processing: {filepath}")
                event_queue.put(filepath)

            threading.Thread(target=wait_until_unlocked, daemon=True).start()


def start_monitoring(paths):
    """
    Start monitoring one or more folder paths using watchdog.

    Args:
        paths (list of str): The directories to watch
    """
    observer = Observer()
    handler = MediaFileHandler()

    for path in paths:
        abs_path = os.path.abspath(path)
        if not os.path.exists(abs_path):
            logger.warning(f"Watch path does not exist: {abs_path}")
            continue
        logger.info(f"Starting monitoring on: {abs_path}")
        observer.schedule(handler, abs_path, recursive=True)

    observer.start()
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        observer.stop()
    observer.join()


if __name__ == '__main__':
    # Example usage: for development/testing only
    watch_folders = ['./watch_folder']
    start_monitoring(watch_folders)