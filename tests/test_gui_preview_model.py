# ============================================================================
# File: /tests/test_gui_preview_model.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the RenamePreviewModel used in the preview table.
# Validates empty state, data insertion, column headers, row count,
# and data retrieval for the Qt model/view architecture.
# ============================================================================

import os
import sys
import pytest

# Force offscreen rendering for headless testing
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from PySide6.QtWidgets import QApplication                  # Qt application instance
from PySide6.QtCore import Qt, QModelIndex                  # Core constants


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
def model(qapp):
    """Create a fresh RenamePreviewModel instance for each test."""
    from ui.preview_panel import RenamePreviewModel
    return RenamePreviewModel()


@pytest.fixture
def sample_results():
    """Provide a list of sample scan results for testing."""
    return [
        {
            "filepath": "/music/artist/song.mp3",
            "filename": "song.mp3",
            "proposed_path": "Music/Artist/Album/01 - Song.mp3",
            "metadata": {
                "media_class": "Music",
                "format_class": "mp3",
                "quality_type": "Lossy",
            },
        },
        {
            "filepath": "/video/movie.mkv",
            "filename": "movie.mkv",
            "proposed_path": "Movie/Film Title.mkv",
            "metadata": {
                "media_class": "Movie",
                "format_class": "matroska",
                "quality_type": "Lossy",
            },
        },
        {
            "filepath": "/music/lossless.flac",
            "filename": "lossless.flac",
            "proposed_path": None,
            "metadata": {
                "media_class": "Music",
                "format_class": "flac",
                "quality_type": "Lossless",
            },
        },
    ]


def test_empty_model(model):
    """Verify a new model has zero rows and 6 columns."""
    assert model.rowCount() == 0
    assert model.columnCount() == 6


def test_column_headers(model):
    """Verify all 6 column headers are correctly defined."""
    expected = ["Original", "Proposed Path", "Type", "Format", "Quality", "Companions"]
    for i, header in enumerate(expected):
        result = model.headerData(i, Qt.Orientation.Horizontal, Qt.ItemDataRole.DisplayRole)
        assert result == header, f"Column {i}: expected '{header}', got '{result}'"


def test_set_results(model, sample_results):
    """Verify set_results populates the model with the correct row count."""
    model.set_results(sample_results)
    assert model.rowCount() == 3


def test_add_result(model, sample_results):
    """Verify add_result appends a single row."""
    assert model.rowCount() == 0
    model.add_result(sample_results[0])
    assert model.rowCount() == 1
    model.add_result(sample_results[1])
    assert model.rowCount() == 2


def test_data_original_column(model, sample_results):
    """Verify the Original column returns the filename."""
    model.set_results(sample_results)
    index = model.index(0, model.COL_ORIGINAL)
    assert model.data(index) == "song.mp3"


def test_data_proposed_column(model, sample_results):
    """Verify the Proposed Path column returns the proposed path."""
    model.set_results(sample_results)
    # Row 0: has a proposed path
    index = model.index(0, model.COL_PROPOSED)
    assert model.data(index) == "Music/Artist/Album/01 - Song.mp3"
    # Row 2: no proposed path — should show fallback text
    index = model.index(2, model.COL_PROPOSED)
    assert model.data(index) == "(no rename)"


def test_data_type_column(model, sample_results):
    """Verify the Type column returns the media_class."""
    model.set_results(sample_results)
    index = model.index(0, model.COL_TYPE)
    assert model.data(index) == "Music"
    index = model.index(1, model.COL_TYPE)
    assert model.data(index) == "Movie"


def test_data_format_column(model, sample_results):
    """Verify the Format column returns the format_class."""
    model.set_results(sample_results)
    index = model.index(0, model.COL_FORMAT)
    assert model.data(index) == "mp3"
    index = model.index(1, model.COL_FORMAT)
    assert model.data(index) == "matroska"


def test_data_quality_column(model, sample_results):
    """Verify the Quality column returns the quality_type."""
    model.set_results(sample_results)
    index = model.index(0, model.COL_QUALITY)
    assert model.data(index) == "Lossy"
    index = model.index(2, model.COL_QUALITY)
    assert model.data(index) == "Lossless"


def test_tooltip_role(model, sample_results):
    """Verify tooltips show the full file path."""
    model.set_results(sample_results)
    index = model.index(0, model.COL_ORIGINAL)
    tooltip = model.data(index, Qt.ItemDataRole.ToolTipRole)
    assert tooltip == "/music/artist/song.mp3"


def test_invalid_index(model):
    """Verify invalid indexes return None gracefully."""
    invalid = model.index(99, 0)
    assert model.data(invalid) is None


def test_reset_results(model, sample_results):
    """Verify set_results replaces previous data entirely."""
    model.set_results(sample_results)
    assert model.rowCount() == 3
    model.set_results([sample_results[0]])
    assert model.rowCount() == 1
