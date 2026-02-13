# ============================================================================
# File: /tests/test_state_manager.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the crash recovery and state persistence module.
# Verifies WatcherState file tracking, atomic saves, crash recovery,
# and AppLockFile single-instance detection.
# ============================================================================

import os                                                  # PID operations
import pytest                                              # Test framework
from pathlib import Path                                   # Path operations
from unittest.mock import patch                            # Mocking utilities

from core.state_manager import (
    WatcherState,                                          # File state tracker
    AppLockFile,                                           # Lock file manager
    get_state_directory,                                   # State directory resolver
)


# ============================================================================
# Tests: get_state_directory()
# ============================================================================

class TestGetStateDirectory:
    """Tests for the platform-aware state directory resolution."""

    def test_returns_path_object(self):
        """Should return a Path object."""
        result = get_state_directory()
        assert isinstance(result, Path)

    def test_creates_directory(self):
        """Should create the directory if it doesn't exist."""
        result = get_state_directory()
        assert result.is_dir()


# ============================================================================
# Tests: WatcherState
# ============================================================================

class TestWatcherState:
    """Tests for the file processing state tracker."""

    def test_creates_state_in_custom_dir(self, tmp_path):
        """Should create a state file in the specified directory."""
        state = WatcherState(state_dir=tmp_path)
        state.mark_in_progress("/test/file.mp3")
        assert (tmp_path / "watcher_state.json").exists()

    def test_mark_in_progress(self, tmp_path):
        """Should track a file as in-progress."""
        state = WatcherState(state_dir=tmp_path)
        state.mark_in_progress("/test/file.mp3")
        pending = state.get_pending_recovery()
        assert "/test/file.mp3" in pending

    def test_mark_completed_removes_from_in_progress(self, tmp_path):
        """Should remove a file from in-progress when completed."""
        state = WatcherState(state_dir=tmp_path)
        state.mark_in_progress("/test/file.mp3")
        state.mark_completed("/test/file.mp3")
        pending = state.get_pending_recovery()
        assert "/test/file.mp3" not in pending

    def test_mark_completed_increments_count(self, tmp_path):
        """Should increment the completed count."""
        state = WatcherState(state_dir=tmp_path)
        state.mark_in_progress("/test/file.mp3")
        state.mark_completed("/test/file.mp3")
        assert state.get_completed_count() == 1

    def test_mark_deferred_removes_from_in_progress(self, tmp_path):
        """Should remove a file from in-progress when deferred."""
        state = WatcherState(state_dir=tmp_path)
        state.mark_in_progress("/test/file.mp3")
        state.mark_deferred("/test/file.mp3", reason="file locked")
        pending = state.get_pending_recovery()
        assert "/test/file.mp3" not in pending

    def test_mark_deferred_tracks_reason(self, tmp_path):
        """Should store the deferral reason."""
        state = WatcherState(state_dir=tmp_path)
        state.mark_deferred("/test/file.mp3", reason="file locked")
        deferred = state.get_deferred()
        assert "/test/file.mp3" in deferred
        assert deferred["/test/file.mp3"]["reason"] == "file locked"

    def test_persists_across_instances(self, tmp_path):
        """State should persist and be loadable by a new instance."""
        state1 = WatcherState(state_dir=tmp_path)
        state1.mark_in_progress("/test/file.mp3")

        state2 = WatcherState(state_dir=tmp_path)
        pending = state2.get_pending_recovery()
        assert "/test/file.mp3" in pending

    def test_clear_resets_all_state(self, tmp_path):
        """clear() should remove all tracked files."""
        state = WatcherState(state_dir=tmp_path)
        state.mark_in_progress("/test/a.mp3")
        state.mark_deferred("/test/b.mp3", reason="locked")
        state.mark_completed("/test/c.mp3")
        state.clear()
        assert state.get_pending_recovery() == []
        assert state.get_deferred() == {}
        assert state.get_completed_count() == 0

    def test_handles_corrupted_state_file(self, tmp_path):
        """Should recover gracefully from a corrupted state file."""
        state_file = tmp_path / "watcher_state.json"
        state_file.write_text("NOT VALID JSON{{{")
        state = WatcherState(state_dir=tmp_path)
        assert state.get_pending_recovery() == []

    def test_handles_empty_state_file(self, tmp_path):
        """Should handle an empty state file gracefully."""
        state_file = tmp_path / "watcher_state.json"
        state_file.write_text("")
        state = WatcherState(state_dir=tmp_path)
        assert state.get_pending_recovery() == []

    def test_multiple_files_tracked(self, tmp_path):
        """Should track multiple files simultaneously."""
        state = WatcherState(state_dir=tmp_path)
        state.mark_in_progress("/test/a.mp3")
        state.mark_in_progress("/test/b.mp3")
        state.mark_deferred("/test/c.mp3", reason="error")
        pending = state.get_pending_recovery()
        assert len(pending) == 2
        assert len(state.get_deferred()) == 1


# ============================================================================
# Tests: AppLockFile
# ============================================================================

class TestAppLockFile:
    """Tests for the application lock file manager."""

    def test_acquire_creates_lock_file(self, tmp_path):
        """Should create a lock file containing the current PID."""
        lock = AppLockFile(state_dir=tmp_path)
        result = lock.acquire()
        assert result is True
        lock_file = tmp_path / "meedyamanager.lock"
        assert lock_file.exists()
        assert str(os.getpid()) in lock_file.read_text()
        lock.release()

    def test_release_removes_lock_file(self, tmp_path):
        """Should remove the lock file on release."""
        lock = AppLockFile(state_dir=tmp_path)
        lock.acquire()
        lock.release()
        assert not (tmp_path / "meedyamanager.lock").exists()

    def test_is_stale_detects_dead_process(self, tmp_path):
        """Should detect a stale lock file from a dead process."""
        lock_file = tmp_path / "meedyamanager.lock"
        # Write a PID that definitely doesn't exist (very large number)
        lock_file.write_text("9999999")
        lock = AppLockFile(state_dir=tmp_path)
        assert lock.is_stale() is True

    def test_is_stale_false_when_no_lock(self, tmp_path):
        """Should return False when no lock file exists."""
        lock = AppLockFile(state_dir=tmp_path)
        assert lock.is_stale() is False

    def test_is_stale_false_when_running(self, tmp_path):
        """Should return False when the lock file PID is still running."""
        lock_file = tmp_path / "meedyamanager.lock"
        lock_file.write_text(str(os.getpid()))             # Our PID is running
        lock = AppLockFile(state_dir=tmp_path)
        assert lock.is_stale() is False

    def test_acquire_detects_running_instance(self, tmp_path):
        """Should return False if another instance is running."""
        lock_file = tmp_path / "meedyamanager.lock"
        lock_file.write_text(str(os.getpid()))             # Our own PID simulates another instance
        lock = AppLockFile(state_dir=tmp_path)
        result = lock.acquire()
        assert result is False

    def test_acquire_overrides_stale_lock(self, tmp_path):
        """Should acquire the lock when the previous instance crashed."""
        lock_file = tmp_path / "meedyamanager.lock"
        lock_file.write_text("9999999")                    # Dead PID
        lock = AppLockFile(state_dir=tmp_path)
        result = lock.acquire()
        assert result is True
        assert str(os.getpid()) in lock_file.read_text()
        lock.release()

    def test_handles_corrupted_lock_file(self, tmp_path):
        """Should treat a corrupted lock file as stale."""
        lock_file = tmp_path / "meedyamanager.lock"
        lock_file.write_text("not_a_number")
        lock = AppLockFile(state_dir=tmp_path)
        assert lock.is_stale() is True
