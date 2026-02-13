# ============================================================================
# File: /core/state_manager.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Crash recovery and state persistence for MeedyaManager.
#
# Provides two key classes:
#   1. WatcherState — Tracks in-progress and deferred file operations,
#      enabling recovery after a crash. State is persisted to a JSON file
#      in the platform-appropriate state directory.
#   2. AppLockFile — Detects stale lock files from previous crashes and
#      supports single-instance enforcement.
#
# State directory locations:
#   - macOS:   ~/Library/Application Support/MeedyaManager/
#   - Windows: %LOCALAPPDATA%/MeedyaManager/
#   - Linux:   ~/.local/share/MeedyaManager/
#
# Usage:
#   from core.state_manager import WatcherState, AppLockFile
#
#   state = WatcherState()
#   state.mark_in_progress("/path/to/file.mp3")
#   state.mark_completed("/path/to/file.mp3")
#
#   lock = AppLockFile()
#   if lock.is_stale():
#       files = state.get_pending_recovery()
# ============================================================================

import os                                                  # Path and PID operations
import sys                                                 # Platform detection
import json                                                # State serialization
import logging                                             # Structured logging
from pathlib import Path                                   # Path operations
from datetime import datetime                              # Timestamps

logger = logging.getLogger("MeedyaManager.StateManager")


# ============================================================================
# State Directory Resolution
# ============================================================================

def get_state_directory() -> Path:
    """
    Get the platform-appropriate state directory for MeedyaManager.

    Creates the directory if it doesn't exist.

    Returns:
        Path: The state directory path.
    """
    if sys.platform == "darwin":
        # macOS: ~/Library/Application Support/MeedyaManager/
        state_dir = Path.home() / "Library" / "Application Support" / "MeedyaManager"
    elif sys.platform == "win32":
        # Windows: %LOCALAPPDATA%/MeedyaManager/
        local_appdata = os.environ.get("LOCALAPPDATA", str(Path.home()))
        state_dir = Path(local_appdata) / "MeedyaManager"
    else:
        # Linux / other: ~/.local/share/MeedyaManager/
        state_dir = Path.home() / ".local" / "share" / "MeedyaManager"

    state_dir.mkdir(parents=True, exist_ok=True)
    return state_dir


# ============================================================================
# WatcherState — File Processing State Tracker
# ============================================================================

class WatcherState:
    """
    Tracks the state of file processing operations for crash recovery.

    Files can be in three states:
      - in_progress: Currently being processed (metadata extraction, rename)
      - completed: Successfully processed
      - deferred: Skipped due to an issue (file locked, error) with a reason

    State is persisted to a JSON file so that after a crash, the application
    can identify which files were interrupted mid-processing and offer to
    retry them.

    The state file uses atomic writes (write to .tmp, then rename) to
    prevent corruption if the application crashes during a save.
    """

    def __init__(self, state_dir: Path | None = None):
        """
        Initialize the watcher state tracker.

        Args:
            state_dir: Directory to store the state file. If None, uses
                       the platform-appropriate default.
        """
        self._state_dir = state_dir or get_state_directory()
        self._state_file = self._state_dir / "watcher_state.json"
        self._state = self._load()

    def _load(self) -> dict:
        """
        Load state from the JSON file.

        Returns:
            dict: The loaded state, or a fresh empty state if the file
                  doesn't exist or is corrupted.
        """
        if self._state_file.exists():
            try:
                text = self._state_file.read_text(encoding="utf-8")
                data = json.loads(text)
                if isinstance(data, dict):
                    return data
            except (json.JSONDecodeError, OSError) as e:
                logger.warning(f"Could not load watcher state: {e}")
        return {"in_progress": {}, "deferred": {}, "completed_count": 0}

    def _save(self):
        """
        Persist the current state to disk using atomic write.

        Writes to a temporary file first, then renames to the final path.
        This prevents corruption if the application crashes mid-write.
        """
        tmp_path = self._state_file.with_suffix(".tmp")
        try:
            tmp_path.write_text(
                json.dumps(self._state, indent=2, default=str),
                encoding="utf-8",
            )
            tmp_path.replace(self._state_file)
        except OSError as e:
            logger.error(f"Could not save watcher state: {e}")

    def mark_in_progress(self, filepath: str):
        """
        Mark a file as currently being processed.

        Args:
            filepath: Absolute path to the file.
        """
        self._state["in_progress"][filepath] = {
            "started_at": datetime.now().isoformat(),
        }
        self._save()
        logger.debug(f"Marked in-progress: {filepath}")

    def mark_completed(self, filepath: str):
        """
        Mark a file as successfully processed.

        Removes it from in_progress and increments the completed count.

        Args:
            filepath: Absolute path to the file.
        """
        self._state["in_progress"].pop(filepath, None)
        self._state["deferred"].pop(filepath, None)
        self._state["completed_count"] = self._state.get("completed_count", 0) + 1
        self._save()
        logger.debug(f"Marked completed: {filepath}")

    def mark_deferred(self, filepath: str, reason: str):
        """
        Mark a file as deferred (skipped for now, retry later).

        Args:
            filepath: Absolute path to the file.
            reason:   Human-readable reason for deferral (e.g., "file locked").
        """
        self._state["in_progress"].pop(filepath, None)
        self._state["deferred"][filepath] = {
            "reason": reason,
            "deferred_at": datetime.now().isoformat(),
        }
        self._save()
        logger.debug(f"Marked deferred: {filepath} — {reason}")

    def get_pending_recovery(self) -> list[str]:
        """
        Get files that were in-progress when the app last crashed.

        These are files that were marked in_progress but never marked
        completed or deferred.

        Returns:
            list[str]: File paths that need recovery processing.
        """
        return list(self._state.get("in_progress", {}).keys())

    def get_deferred(self) -> dict[str, dict]:
        """
        Get all deferred files and their deferral reasons.

        Returns:
            dict: Mapping of filepath -> {reason, deferred_at}.
        """
        return dict(self._state.get("deferred", {}))

    def get_completed_count(self) -> int:
        """
        Get the total number of files processed in this session.

        Returns:
            int: Number of files marked as completed.
        """
        return self._state.get("completed_count", 0)

    def clear(self):
        """
        Clear all state (reset to empty).

        Used after successful recovery or when the user wants to
        reset the tracking state.
        """
        self._state = {"in_progress": {}, "deferred": {}, "completed_count": 0}
        self._save()
        logger.debug("Watcher state cleared")


# ============================================================================
# AppLockFile — Single Instance & Crash Detection
# ============================================================================

class AppLockFile:
    """
    Application lock file for single-instance enforcement and crash detection.

    On startup, MeedyaManager writes a lock file containing its PID.
    If a lock file already exists:
      - If the PID is still running → another instance is active
      - If the PID is dead → previous instance crashed

    This allows the application to detect crashes and offer recovery.
    """

    def __init__(self, state_dir: Path | None = None):
        """
        Initialize the lock file manager.

        Args:
            state_dir: Directory to store the lock file. If None, uses
                       the platform-appropriate default.
        """
        self._state_dir = state_dir or get_state_directory()
        self._lock_file = self._state_dir / "meedyamanager.lock"

    def acquire(self) -> bool:
        """
        Acquire the application lock.

        Writes the current PID to the lock file. If a lock file already
        exists with a running PID, returns False (another instance is active).

        Returns:
            True if the lock was acquired, False if another instance is running.
        """
        if self._lock_file.exists():
            if self._is_pid_running(self._read_pid()):
                logger.warning("Another MeedyaManager instance is already running")
                return False
            else:
                # Stale lock file — previous instance crashed
                logger.info("Detected stale lock file from previous crash")

        # Write our PID
        try:
            self._lock_file.write_text(str(os.getpid()), encoding="utf-8")
            return True
        except OSError as e:
            logger.error(f"Could not write lock file: {e}")
            return True                                    # Proceed anyway

    def release(self):
        """
        Release the application lock.

        Removes the lock file. Should be called during clean shutdown.
        """
        try:
            if self._lock_file.exists():
                self._lock_file.unlink()
        except OSError as e:
            logger.warning(f"Could not remove lock file: {e}")

    def is_stale(self) -> bool:
        """
        Check if a stale lock file exists (indicating a previous crash).

        A lock file is stale if it exists and the PID it contains is
        no longer running.

        Returns:
            True if a stale lock file was found, False otherwise.
        """
        if not self._lock_file.exists():
            return False
        pid = self._read_pid()
        if pid is None:
            return True                                    # Corrupted lock file
        return not self._is_pid_running(pid)

    def _read_pid(self) -> int | None:
        """
        Read the PID from the lock file.

        Returns:
            int: The PID, or None if the file cannot be read.
        """
        try:
            text = self._lock_file.read_text(encoding="utf-8").strip()
            return int(text)
        except (OSError, ValueError):
            return None

    @staticmethod
    def _is_pid_running(pid: int | None) -> bool:
        """
        Check if a process with the given PID is currently running.

        Args:
            pid: Process ID to check.

        Returns:
            True if the process is running, False otherwise.
        """
        if pid is None:
            return False
        try:
            # os.kill with signal 0 checks if the process exists
            # without actually sending a signal
            os.kill(pid, 0)
            return True
        except (OSError, ProcessLookupError):
            return False
