# ============================================================================
# File: /tests/test_safe_worker.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the SafeWorker base class in ui/workers.py.
# Verifies that the safety-net exception handler catches unhandled
# exceptions in background workers and emits the worker_error signal.
# ============================================================================

import time                                                # Thread synchronization
import logging                                             # Log capture
import pytest                                              # Test framework

from PySide6.QtCore import QCoreApplication                # Event loop for signal delivery


# ============================================================================
# Fixtures
# ============================================================================

@pytest.fixture(scope="session")
def qapp():
    """Create a QApplication instance for signal delivery."""
    from PySide6.QtWidgets import QApplication
    app = QApplication.instance()
    if app is None:
        app = QApplication([])
    return app


# ============================================================================
# Tests: SafeWorker Base Class
# ============================================================================

class TestSafeWorker:
    """Tests for the SafeWorker base class exception handling."""

    def test_safe_run_raises_not_implemented(self, qapp):
        """SafeWorker.safe_run() should raise NotImplementedError."""
        from ui.workers import SafeWorker
        worker = SafeWorker()
        with pytest.raises(NotImplementedError):
            worker.safe_run()

    def test_emits_worker_error_on_crash(self, qapp):
        """Should emit worker_error when safe_run() raises an exception."""
        from ui.workers import SafeWorker

        class CrashingWorker(SafeWorker):
            def safe_run(self):
                raise RuntimeError("deliberate crash")

        worker = CrashingWorker()
        received = []
        worker.worker_error.connect(lambda title, detail: received.append((title, detail)))

        # Run the worker and wait for completion
        worker.start()
        worker.wait(5000)                                  # 5 second timeout

        # Process pending events to deliver signals
        QCoreApplication.processEvents()

        assert len(received) == 1
        title, detail = received[0]
        assert "CrashingWorker" in title
        assert "deliberate crash" in detail

    def test_logs_crash_on_unhandled_exception(self, qapp, caplog):
        """Should log the full traceback when safe_run() crashes."""
        from ui.workers import SafeWorker

        class CrashingWorker(SafeWorker):
            def safe_run(self):
                raise ValueError("logged crash")

        worker = CrashingWorker()

        with caplog.at_level(logging.ERROR, logger="MeedyaManager.Workers"):
            worker.start()
            worker.wait(5000)
            QCoreApplication.processEvents()

        assert "CrashingWorker" in caplog.text
        assert "logged crash" in caplog.text

    def test_successful_worker_no_error(self, qapp):
        """Should not emit worker_error when safe_run() completes normally."""
        from ui.workers import SafeWorker

        class GoodWorker(SafeWorker):
            def safe_run(self):
                pass                                       # No error

        worker = GoodWorker()
        received = []
        worker.worker_error.connect(lambda title, detail: received.append((title, detail)))

        worker.start()
        worker.wait(5000)
        QCoreApplication.processEvents()

        assert len(received) == 0

    def test_worker_error_includes_exception_type(self, qapp):
        """worker_error detail should include the exception type name."""
        from ui.workers import SafeWorker

        class TypedCrashWorker(SafeWorker):
            def safe_run(self):
                raise TypeError("wrong type")

        worker = TypedCrashWorker()
        received = []
        worker.worker_error.connect(lambda title, detail: received.append((title, detail)))

        worker.start()
        worker.wait(5000)
        QCoreApplication.processEvents()

        assert len(received) == 1
        assert "TypeError" in received[0][1]


# ============================================================================
# Tests: Existing Workers Inherit SafeWorker
# ============================================================================

class TestWorkerInheritance:
    """Verify that all workers inherit from SafeWorker."""

    def test_scan_worker_inherits_safe_worker(self):
        """ScanWorker should be a subclass of SafeWorker."""
        from ui.workers import ScanWorker, SafeWorker
        assert issubclass(ScanWorker, SafeWorker)

    def test_tag_write_worker_inherits_safe_worker(self):
        """TagWriteWorker should be a subclass of SafeWorker."""
        from ui.workers import TagWriteWorker, SafeWorker
        assert issubclass(TagWriteWorker, SafeWorker)

    def test_lookup_worker_inherits_safe_worker(self):
        """LookupWorker should be a subclass of SafeWorker."""
        from ui.workers import LookupWorker, SafeWorker
        assert issubclass(LookupWorker, SafeWorker)

    def test_scan_worker_has_worker_error_signal(self):
        """ScanWorker should have the worker_error signal from SafeWorker."""
        from ui.workers import ScanWorker
        # Check that the class has the signal attribute
        assert hasattr(ScanWorker, "worker_error")

    def test_tag_write_worker_has_worker_error_signal(self):
        """TagWriteWorker should have the worker_error signal from SafeWorker."""
        from ui.workers import TagWriteWorker
        assert hasattr(TagWriteWorker, "worker_error")

    def test_lookup_worker_has_worker_error_signal(self):
        """LookupWorker should have the worker_error signal from SafeWorker."""
        from ui.workers import LookupWorker
        assert hasattr(LookupWorker, "worker_error")
