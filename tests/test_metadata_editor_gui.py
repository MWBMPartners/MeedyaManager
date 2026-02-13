# ============================================================================
# File: /tests/test_metadata_editor_gui.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the metadata editor GUI components (Phase 3).
# Uses QT_QPA_PLATFORM=offscreen for headless testing.
# Tests TagTableModel, CoverArtWidget, MetadataEditorPanel, and
# the TagWriteWorker background thread.
# ============================================================================

import os
import sys
import pytest

# Force offscreen rendering for headless testing
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from PySide6.QtWidgets import QApplication
from PySide6.QtCore import Qt


@pytest.fixture(scope="session")
def qapp():
    """Create a single QApplication instance for the test session."""
    app = QApplication.instance()
    if app is None:
        app = QApplication(sys.argv)
    return app


# =============================================================================
# TagTableModel Tests
# =============================================================================

class TestTagTableModel:
    """Tests for the TagTableModel used in the metadata editor."""

    def test_column_count(self, qapp):
        """Model should have 2 columns (Tag, Value)."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        assert model.columnCount() == 2

    def test_headers(self, qapp):
        """Column headers should be 'Tag' and 'Value'."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        assert model.headerData(0, Qt.Orientation.Horizontal) == "Tag"
        assert model.headerData(1, Qt.Orientation.Horizontal) == "Value"

    def test_empty_model_row_count(self, qapp):
        """Empty model should have 0 rows."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        assert model.rowCount() == 0

    def test_load_file_populates_rows(self, qapp, real_mp3_file):
        """Loading a real MP3 file should populate the model with tag rows."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        model.load_file(real_mp3_file)
        # MP3 fixture has: title, artist, album, album_artist, genre,
        # track_num, total_tracks, disc_num, total_discs, year, composer
        assert model.rowCount() >= 8

    def test_load_file_has_artist(self, qapp, real_mp3_file):
        """Loaded model should contain the artist tag."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        model.load_file(real_mp3_file)
        # Search for artist in the model data
        found = False
        for row in range(model.rowCount()):
            tag_name = model._tags[row][0]         # display_name
            internal_key = model._tags[row][1]      # internal_key
            if internal_key == "artist":
                found = True
                value = model._tags[row][2]         # current_value
                assert value == "Integration Artist"
                break
        assert found, "artist tag not found in model"

    def test_value_column_editable_for_tags(self, qapp, real_mp3_file):
        """Value column should be editable for non-technical tag fields."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        model.load_file(real_mp3_file)
        # Find the artist row
        for row in range(model.rowCount()):
            if model._tags[row][1] == "artist":
                index = model.index(row, 1)
                flags = model.flags(index)
                assert flags & Qt.ItemFlag.ItemIsEditable
                break

    def test_tag_column_not_editable(self, qapp, real_mp3_file):
        """Tag name column (column 0) should never be editable."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        model.load_file(real_mp3_file)
        if model.rowCount() > 0:
            index = model.index(0, 0)               # First row, Tag column
            flags = model.flags(index)
            assert not (flags & Qt.ItemFlag.ItemIsEditable)

    def test_get_changes_empty_initially(self, qapp, real_mp3_file):
        """get_changes should return empty dict when nothing is modified."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        model.load_file(real_mp3_file)
        assert model.get_changes() == {}

    def test_get_changes_after_edit(self, qapp, real_mp3_file):
        """get_changes should return modified values after setData."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        model.load_file(real_mp3_file)
        # Find artist row and modify it
        for row in range(model.rowCount()):
            if model._tags[row][1] == "artist":
                index = model.index(row, 1)
                model.setData(index, "Modified Artist")
                break
        changes = model.get_changes()
        assert "artist" in changes
        assert changes["artist"] == "Modified Artist"

    def test_add_custom_tag(self, qapp, real_mp3_file):
        """add_custom_tag should add a new row to the model."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        model.load_file(real_mp3_file)
        initial_rows = model.rowCount()
        model.add_custom_tag("SpotifyURL", "https://spotify.com/track/123")
        assert model.rowCount() == initial_rows + 1
        # Verify the custom tag is in the model
        last_row = model._tags[-1]
        assert last_row[1] == "custom_spotifyurl"
        assert last_row[3] == "https://spotify.com/track/123"

    def test_load_batch_mixed_values(self, qapp, real_mp3_file, real_flac_file):
        """Batch loading files with different values should show '<Multiple>'."""
        from ui.metadata_editor import TagTableModel
        model = TagTableModel()
        model.load_batch([real_mp3_file, real_flac_file])
        # Artist differs between MP3 ("Integration Artist") and FLAC ("FLAC Integration Artist")
        for row in range(model.rowCount()):
            if model._tags[row][1] == "artist":
                assert model._tags[row][2] == "<Multiple>"
                break


# =============================================================================
# CoverArtWidget Tests
# =============================================================================

class TestCoverArtWidget:
    """Tests for the CoverArtWidget."""

    def test_instantiation(self, qapp):
        """CoverArtWidget should instantiate without crashing."""
        from ui.metadata_editor import CoverArtWidget
        widget = CoverArtWidget()
        assert widget is not None
        assert widget._thumbnail is not None

    def test_initial_state(self, qapp):
        """CoverArtWidget should show 'No Cover Art' initially."""
        from ui.metadata_editor import CoverArtWidget
        widget = CoverArtWidget()
        assert widget._thumbnail.text() == "No Cover Art"


# =============================================================================
# MetadataEditorPanel Tests
# =============================================================================

class TestMetadataEditorPanel:
    """Tests for the MetadataEditorPanel widget."""

    def test_instantiation(self, qapp):
        """MetadataEditorPanel should instantiate without crashing."""
        from ui.metadata_editor import MetadataEditorPanel
        panel = MetadataEditorPanel()
        assert panel is not None

    def test_has_tag_table(self, qapp):
        """Panel should have a tag table view and model."""
        from ui.metadata_editor import MetadataEditorPanel
        panel = MetadataEditorPanel()
        assert panel._tag_model is not None
        assert panel._tag_table is not None

    def test_has_buttons(self, qapp):
        """Panel should have Save, Revert, and Add Custom Tag buttons."""
        from ui.metadata_editor import MetadataEditorPanel
        panel = MetadataEditorPanel()
        assert panel._save_btn is not None
        assert panel._revert_btn is not None
        assert panel._add_custom_btn is not None

    def test_buttons_disabled_initially(self, qapp):
        """Save, Revert, and Add Custom Tag buttons should be disabled initially."""
        from ui.metadata_editor import MetadataEditorPanel
        panel = MetadataEditorPanel()
        assert not panel._save_btn.isEnabled()
        assert not panel._revert_btn.isEnabled()
        assert not panel._add_custom_btn.isEnabled()

    def test_load_file_enables_buttons(self, qapp, real_mp3_file):
        """Loading a file should enable the editing buttons."""
        from ui.metadata_editor import MetadataEditorPanel
        panel = MetadataEditorPanel()
        panel.load_file(real_mp3_file)
        assert panel._save_btn.isEnabled()
        assert panel._revert_btn.isEnabled()
        assert panel._add_custom_btn.isEnabled()

    def test_load_file_updates_label(self, qapp, real_mp3_file):
        """Loading a file should update the file path label."""
        from ui.metadata_editor import MetadataEditorPanel
        panel = MetadataEditorPanel()
        panel.load_file(real_mp3_file)
        assert "real_test.mp3" in panel._file_label.text()

    def test_load_files_batch_mode(self, qapp, real_mp3_file, real_flac_file):
        """Loading multiple files should enter batch mode."""
        from ui.metadata_editor import MetadataEditorPanel
        panel = MetadataEditorPanel()
        panel.load_files([real_mp3_file, real_flac_file])
        assert "2 files" in panel._file_label.text()


# =============================================================================
# TagWriteWorker Tests
# =============================================================================

class TestTagWriteWorker:
    """Tests for the TagWriteWorker background thread."""

    def test_instantiation(self, qapp):
        """TagWriteWorker should instantiate without crashing."""
        from ui.workers import TagWriteWorker
        worker = TagWriteWorker([("/fake/file.mp3", {"artist": "Test"})])
        assert worker is not None

    def test_write_single_file(self, qapp, real_mp3_file):
        """Worker should successfully write tags to a real file."""
        from ui.workers import TagWriteWorker
        worker = TagWriteWorker([
            (real_mp3_file, {"genre": "Pop"})
        ])

        results = {"success": 0, "errors": 0}

        def on_finished(s, e):
            results["success"] = s
            results["errors"] = e

        worker.finished_all.connect(on_finished)
        worker.run()                                     # Run synchronously in test

        assert results["success"] == 1
        assert results["errors"] == 0

        # Verify the tag was written
        from metadata.editor import TagEditor
        tags = TagEditor().read_tags(real_mp3_file)
        assert tags.get("genre") == "Pop"
