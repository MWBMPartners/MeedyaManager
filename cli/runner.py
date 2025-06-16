# ============================================================================
# File: /cli/runner.py
# (C) 2025 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# This is the unified CLI entry point for MetaMancer's early functionality.
# It integrates the file monitoring engine (/core/watcher.py) and the dry-run
# rename engine (/core/renamer.py), simulating how files would be renamed
# according to metadata.
#
# This CLI is for development and testing only. It demonstrates real-time
# interaction between components using a shared event queue and loads settings
# from config/settings.json5.
#
# As of Milestone 1, it:
# - Starts folder monitoring
# - Waits for file events
# - Extracts metadata using pymediainfo
# - Simulates renaming
# - Logs preview results to console and logs/rename_preview.log
# ============================================================================

import os
import time
import threading
import logging
from queue import Queue

# Import core logic
from core.watcher import start_monitoring, event_queue
from core.renamer import simulate_rename
from core.metadata_extractor import extract_metadata
from utils.config_loader import load_config, get_config

# Setup logger
logger = logging.getLogger("MetaMancer.Runner")
logger.setLevel(logging.DEBUG)
handler = logging.StreamHandler()
formatter = logging.Formatter("[%(asctime)s] %(levelname)s - %(message)s")
handler.setFormatter(formatter)
logger.addHandler(handler)


# --- Processing Loop ---
def process_event_queue():
    """
    Continuously process new file paths from the watcher event queue.
    Extracts metadata and simulates renaming.
    """
    while True:
        try:
            filepath = event_queue.get(timeout=1)
            logger.info(f"Processing queued file: {filepath}")
            metadata = extract_metadata(filepath)
            simulate_rename(filepath, metadata)
        except Exception as e:
            # Timeout or queue empty
            time.sleep(1)


# --- Main CLI Entrypoint ---
if __name__ == '__main__':
    print("\nMetaMancer CLI Runner — Milestone 1 Integration Test\n")

    # Load settings
    load_config()
    debug = get_config("debug", False)
    watch_folders = get_config("watch_folders", ['./watch_folder'])

    if debug:
        logger.info("Debug mode is ON")

    # Start processing thread
    processor_thread = threading.Thread(target=process_event_queue, daemon=True)
    processor_thread.start()

    # Start monitoring in the main thread
    try:
        start_monitoring(watch_folders)
    except KeyboardInterrupt:
        print("\n[!] Exiting MetaMancer CLI. Goodbye!")