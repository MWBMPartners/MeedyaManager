# ============================================================================
# File: /tests/test_gui_lookup.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the GUI Lookup panel (ui/lookup_panel.py) and LookupWorker
# (ui/workers.py). Verifies widget instantiation, provider checkbox
# creation, results table structure, signal definitions, and worker
# parameter handling.
#
# Uses QT_QPA_PLATFORM=offscreen for headless/CI execution.
# All provider interactions are mocked — no real API calls.
# ============================================================================

import os                                                  # Environment variable setup
import sys                                                 # System argv for QApplication
import pytest                                              # Test framework
from unittest.mock import patch, MagicMock                 # Mocking for provider isolation

# Force offscreen rendering for headless testing (CI, SSH, etc.)
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from PySide6.QtWidgets import QApplication                 # Qt application instance
from PySide6.QtCore import Signal                          # Signal type checking


@pytest.fixture(scope="session")
def qapp():
    """
    Create a single QApplication instance for the entire test session.
    Qt requires exactly one QApplication — reuse it across all GUI tests.
    """
    app = QApplication.instance()
    if app is None:
        app = QApplication(sys.argv)
    return app


@pytest.fixture
def mock_lookup_service():
    """
    Mock the LookupService used by LookupPanel to discover providers.

    Patches LookupService at the import location used by lookup_panel.py
    so provider discovery returns controlled test data without hitting
    any real API endpoints.

    Yields:
        MagicMock: The mocked LookupService class.
    """
    with patch("ui.lookup_panel.LookupService") as MockService:
        # Configure the instance returned by LookupService()
        mock_instance = MagicMock()

        # Return a representative set of providers for checkbox creation
        mock_instance.get_available_providers.return_value = [
            {
                "name": "spotify",                         # Provider internal name
                "category": "music",                       # ProviderCategory value
                "requires_auth": True,                     # Needs API credentials
                "available": True,                         # Credentials are configured
                "message": "Ready",                        # Status message
            },
            {
                "name": "musicbrainz",                     # Provider internal name
                "category": "music",                       # ProviderCategory value
                "requires_auth": False,                    # Public API
                "available": True,                         # Always available
                "message": "Ready",                        # Status message
            },
            {
                "name": "tmdb",                            # Provider internal name
                "category": "video",                       # ProviderCategory value
                "requires_auth": True,                     # Needs API key
                "available": False,                        # No credentials configured
                "message": "API key not set",              # Explains why unavailable
            },
        ]

        MockService.return_value = mock_instance
        yield MockService


# =============================================================================
# LookupPanel Instantiation Tests
# =============================================================================

class TestLookupPanelCreation:
    """Tests for LookupPanel widget instantiation and basic structure."""

    def test_lookup_panel_instantiates(self, qapp, mock_lookup_service):
        """LookupPanel should instantiate without crashing.

        Verifies that the widget can be created and is not None.
        The mock prevents any real provider discovery during __init__.
        """
        from ui.lookup_panel import LookupPanel
        panel = LookupPanel()
        assert panel is not None

    def test_lookup_panel_has_signals(self, qapp, mock_lookup_service):
        """LookupPanel should define lookup_completed and tags_applied signals.

        These signals are used for inter-widget communication:
        - lookup_completed(list): Emitted when results arrive
        - tags_applied(str, dict): Emitted when tags are written to a file
        """
        from ui.lookup_panel import LookupPanel
        panel = LookupPanel()

        # Verify signal attributes exist on the class
        assert hasattr(LookupPanel, "lookup_completed")
        assert hasattr(LookupPanel, "tags_applied")

    def test_lookup_panel_has_search_button(self, qapp, mock_lookup_service):
        """LookupPanel should have a Search button that is initially disabled.

        The button is disabled until a file is loaded via load_file().
        """
        from ui.lookup_panel import LookupPanel
        panel = LookupPanel()

        assert panel._search_btn is not None
        assert panel._search_btn.text() == "Search"
        assert panel._search_btn.isEnabled() is False      # Disabled until file loaded

    def test_lookup_panel_has_results_table(self, qapp, mock_lookup_service):
        """LookupPanel should have a results QTableWidget with correct columns.

        The table should have columns for: #, Provider, Confidence,
        Title, Artist, Album, Year.
        """
        from ui.lookup_panel import LookupPanel
        panel = LookupPanel()

        assert panel._results_table is not None
        assert panel._results_table.columnCount() >= 6     # At least 6 data columns

    def test_lookup_panel_has_file_info_label(self, qapp, mock_lookup_service):
        """LookupPanel should have a file info label showing initial placeholder text."""
        from ui.lookup_panel import LookupPanel
        panel = LookupPanel()

        assert panel._file_info_label is not None
        assert "No file loaded" in panel._file_info_label.text()

    def test_lookup_panel_has_status_label(self, qapp, mock_lookup_service):
        """LookupPanel should have a status label with initial ready message."""
        from ui.lookup_panel import LookupPanel
        panel = LookupPanel()

        assert panel._status_label is not None
        assert "Ready" in panel._status_label.text()


# =============================================================================
# Provider Checkbox Tests
# =============================================================================

class TestLookupPanelProviders:
    """Tests for provider checkbox creation and state management."""

    def test_provider_checkboxes_created(self, qapp, mock_lookup_service):
        """Provider checkboxes should be created from LookupService discovery.

        The mock returns 3 providers (spotify, musicbrainz, tmdb).
        Each should have a corresponding checkbox in _provider_checkboxes.
        """
        from ui.lookup_panel import LookupPanel
        panel = LookupPanel()

        # Should have 3 checkboxes — one per mock provider
        assert len(panel._provider_checkboxes) == 3
        assert "spotify" in panel._provider_checkboxes
        assert "musicbrainz" in panel._provider_checkboxes
        assert "tmdb" in panel._provider_checkboxes

    def test_available_provider_checked(self, qapp, mock_lookup_service):
        """Available providers should be checked (enabled) by default.

        Spotify and MusicBrainz are configured as available in the mock,
        so their checkboxes should be both checked and enabled.
        """
        from ui.lookup_panel import LookupPanel
        panel = LookupPanel()

        # Spotify: available=True → should be checked and enabled
        spotify_cb = panel._provider_checkboxes["spotify"]
        assert spotify_cb.isChecked() is True
        assert spotify_cb.isEnabled() is True

        # MusicBrainz: available=True → should be checked and enabled
        mb_cb = panel._provider_checkboxes["musicbrainz"]
        assert mb_cb.isChecked() is True
        assert mb_cb.isEnabled() is True

    def test_unavailable_provider_unchecked_disabled(self, qapp, mock_lookup_service):
        """Unavailable providers should be unchecked and disabled (greyed out).

        TMDB is configured as available=False in the mock, so its checkbox
        should be unchecked and disabled to prevent selection.
        """
        from ui.lookup_panel import LookupPanel
        panel = LookupPanel()

        # TMDB: available=False → should be unchecked and disabled
        tmdb_cb = panel._provider_checkboxes["tmdb"]
        assert tmdb_cb.isChecked() is False
        assert tmdb_cb.isEnabled() is False


# =============================================================================
# LookupWorker Tests
# =============================================================================

class TestLookupWorker:
    """Tests for the LookupWorker QThread background worker."""

    def test_worker_instantiates(self, qapp):
        """LookupWorker should instantiate with metadata and optional filters."""
        from ui.workers import LookupWorker

        metadata = {"title": "Test", "artist": "Artist"}
        worker = LookupWorker(
            metadata=metadata,
            provider_names=["spotify"],
            min_confidence=0.5,
        )

        assert worker is not None
        assert worker._metadata == metadata
        assert worker._provider_names == ["spotify"]
        assert worker._min_confidence == 0.5

    def test_worker_has_signals(self, qapp):
        """LookupWorker should define progress, result_ready, error, and
        provider_searched signals for GUI communication."""
        from ui.workers import LookupWorker

        # Verify signal attributes exist on the class
        assert hasattr(LookupWorker, "progress")
        assert hasattr(LookupWorker, "result_ready")
        assert hasattr(LookupWorker, "error")
        assert hasattr(LookupWorker, "provider_searched")

    def test_worker_default_params(self, qapp):
        """LookupWorker should accept minimal metadata with defaults for
        optional parameters (provider_names=None, category=None, min_confidence=0.0)."""
        from ui.workers import LookupWorker

        worker = LookupWorker(metadata={"title": "Test"})

        assert worker._provider_names is None              # No provider filter
        assert worker._category is None                    # No category filter
        assert worker._min_confidence == 0.0               # Accept all confidence levels
