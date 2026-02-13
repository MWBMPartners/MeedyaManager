# ============================================================================
# File: /tests/test_error_dialog.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Tests for the user-friendly error dialog module.
# Verifies ErrorDialog construction, content population, clipboard copy,
# detail toggle behavior, and the show_error convenience function.
#
# Note: These tests use a headless QApplication (no display required).
# The conftest.py session fixture provides the QApplication instance.
# ============================================================================

import pytest                                              # Test framework
from unittest.mock import patch, MagicMock                 # Mocking utilities

from utils.error_messages import ErrorMessage              # Structured error messages


# ============================================================================
# Fixtures
# ============================================================================

@pytest.fixture(scope="session")
def qapp():
    """Create a QApplication instance for the entire test session.

    PySide6 requires exactly one QApplication per process. This fixture
    creates it once and reuses it across all tests in this module.
    """
    from PySide6.QtWidgets import QApplication
    app = QApplication.instance()
    if app is None:
        app = QApplication([])
    return app


@pytest.fixture
def error_dialog(qapp):
    """Create a fresh ErrorDialog instance for each test."""
    from ui.error_dialog import ErrorDialog
    dialog = ErrorDialog()
    yield dialog
    dialog.close()


# ============================================================================
# Tests: ErrorDialog Construction
# ============================================================================

class TestErrorDialogConstruction:
    """Tests for ErrorDialog widget creation and layout."""

    def test_creates_dialog(self, error_dialog):
        """Should create an ErrorDialog instance."""
        from ui.error_dialog import ErrorDialog
        assert isinstance(error_dialog, ErrorDialog)

    def test_has_window_title(self, error_dialog):
        """Should have a descriptive window title."""
        assert "Error" in error_dialog.windowTitle()

    def test_has_headline_label(self, error_dialog):
        """Should have a headline label widget."""
        assert error_dialog._headline_label is not None

    def test_has_explanation_label(self, error_dialog):
        """Should have an explanation label widget."""
        assert error_dialog._explanation_label is not None

    def test_has_suggestion_label(self, error_dialog):
        """Should have a suggestion label widget."""
        assert error_dialog._suggestion_label is not None

    def test_has_copy_button(self, error_dialog):
        """Should have a 'Copy to Clipboard' button."""
        assert error_dialog._copy_button is not None
        assert "Copy" in error_dialog._copy_button.text()

    def test_has_ok_button(self, error_dialog):
        """Should have an 'OK' button."""
        assert error_dialog._ok_button is not None
        assert "OK" in error_dialog._ok_button.text()

    def test_details_hidden_by_default(self, error_dialog):
        """Technical details should be hidden initially (not shown)."""
        # Use isHidden() which checks the widget's own visibility flag,
        # rather than isVisible() which requires the entire parent chain
        # to be shown (the dialog itself is not shown in tests).
        assert error_dialog._details_text.isHidden()


# ============================================================================
# Tests: set_error() — Catalog-based population
# ============================================================================

class TestSetError:
    """Tests for populating the dialog from an exception via the error catalog."""

    def test_populates_headline(self, error_dialog):
        """Should set the headline from the error catalog."""
        error_dialog.set_error(PermissionError("access denied"))
        assert error_dialog._headline_label.text() != ""
        assert "Permission" in error_dialog._headline_label.text()

    def test_populates_explanation(self, error_dialog):
        """Should set the explanation from the error catalog."""
        error_dialog.set_error(PermissionError("access denied"))
        assert error_dialog._explanation_label.text() != ""

    def test_populates_suggestion(self, error_dialog):
        """Should set the suggestion from the error catalog."""
        error_dialog.set_error(PermissionError("access denied"))
        assert error_dialog._suggestion_label.text() != ""

    def test_populates_technical_details(self, error_dialog):
        """Should populate technical details with exception info."""
        error_dialog.set_error(PermissionError("access denied"))
        detail = error_dialog._details_text.toPlainText()
        assert "PermissionError" in detail

    def test_uses_context_for_lookup(self, error_dialog):
        """Should use the context parameter for catalog lookup."""
        error_dialog.set_error(FileNotFoundError("missing"), context="scan")
        assert "Media file" in error_dialog._headline_label.text()

    def test_uses_explicit_technical_detail(self, error_dialog):
        """Should use the explicit technical_detail when provided."""
        custom_detail = "Custom traceback information here"
        error_dialog.set_error(
            RuntimeError("test"), technical_detail=custom_detail
        )
        detail = error_dialog._details_text.toPlainText()
        assert "Custom traceback information here" in detail

    def test_unknown_exception_uses_fallback(self, error_dialog):
        """Unknown exception types should use the generic fallback message."""
        class VeryCustomError(Exception):
            pass
        error_dialog.set_error(VeryCustomError("something"))
        assert "unexpected" in error_dialog._headline_label.text().lower()


# ============================================================================
# Tests: set_message() — Direct ErrorMessage population
# ============================================================================

class TestSetMessage:
    """Tests for populating the dialog with a custom ErrorMessage directly."""

    def test_sets_custom_headline(self, error_dialog):
        """Should display the custom headline."""
        msg = ErrorMessage(
            headline="Custom Error Title",
            explanation="Something happened.",
            suggestion="Try restarting.",
        )
        error_dialog.set_message(msg)
        assert error_dialog._headline_label.text() == "Custom Error Title"

    def test_sets_custom_explanation(self, error_dialog):
        """Should display the custom explanation."""
        msg = ErrorMessage(
            headline="Title",
            explanation="Detailed explanation here.",
            suggestion="Suggestion here.",
        )
        error_dialog.set_message(msg)
        assert error_dialog._explanation_label.text() == "Detailed explanation here."

    def test_hides_details_toggle_when_no_details(self, error_dialog):
        """Should hide the details toggle when there are no technical details."""
        msg = ErrorMessage(
            headline="Title", explanation="Exp.", suggestion="Sug.",
        )
        error_dialog.set_message(msg)
        assert error_dialog._details_toggle.isHidden()


# ============================================================================
# Tests: Detail Toggle
# ============================================================================

class TestDetailToggle:
    """Tests for the collapsible technical details section."""

    def test_toggle_shows_details(self, error_dialog):
        """Clicking 'Show Details' should reveal the details section."""
        error_dialog.set_error(RuntimeError("test"))
        error_dialog._details_toggle.setChecked(True)
        # Use isHidden() instead of isVisible() — in headless tests the
        # dialog itself is not shown, so isVisible() on children is False
        # even when they are not explicitly hidden.
        assert not error_dialog._details_text.isHidden()

    def test_toggle_hides_details(self, error_dialog):
        """Clicking 'Hide Details' should collapse the details section."""
        error_dialog.set_error(RuntimeError("test"))
        error_dialog._details_toggle.setChecked(True)
        error_dialog._details_toggle.setChecked(False)
        assert error_dialog._details_text.isHidden()

    def test_toggle_updates_button_text(self, error_dialog):
        """Toggle button text should change between 'Show' and 'Hide'."""
        error_dialog.set_error(RuntimeError("test"))
        error_dialog._details_toggle.setChecked(True)
        assert "Hide" in error_dialog._details_toggle.text()
        error_dialog._details_toggle.setChecked(False)
        assert "Show" in error_dialog._details_toggle.text()


# ============================================================================
# Tests: Copy to Clipboard
# ============================================================================

class TestCopyToClipboard:
    """Tests for the clipboard copy functionality."""

    def test_copy_sets_clipboard(self, error_dialog, qapp):
        """Clicking 'Copy to Clipboard' should copy the error text."""
        from PySide6.QtWidgets import QApplication
        error_dialog.set_error(PermissionError("test denial"))
        error_dialog._copy_to_clipboard()
        clipboard_text = QApplication.clipboard().text()
        assert "Permission" in clipboard_text
        assert "test denial" in clipboard_text or "PermissionError" in clipboard_text

    def test_copy_includes_technical_details(self, error_dialog, qapp):
        """Clipboard text should include technical details."""
        from PySide6.QtWidgets import QApplication
        error_dialog.set_error(
            RuntimeError("test"), technical_detail="Full traceback here"
        )
        error_dialog._copy_to_clipboard()
        clipboard_text = QApplication.clipboard().text()
        assert "Full traceback here" in clipboard_text

    def test_copy_changes_button_text(self, error_dialog):
        """Button should briefly change to 'Copied!' after clicking."""
        error_dialog.set_error(RuntimeError("test"))
        error_dialog._copy_to_clipboard()
        assert "Copied" in error_dialog._copy_button.text()


# ============================================================================
# Tests: show_error() convenience function
# ============================================================================

class TestShowErrorFunction:
    """Tests for the show_error() convenience function.

    These tests mock the entire ErrorDialog class to avoid PySide6 segfaults
    that can occur when patching individual methods on QDialog subclasses
    in Python 3.14.
    """

    def test_show_error_creates_dialog(self, qapp):
        """show_error() should create an ErrorDialog and call exec()."""
        mock_dialog = MagicMock()
        mock_cls = MagicMock(return_value=mock_dialog)

        with patch("ui.error_dialog.ErrorDialog", mock_cls):
            from ui.error_dialog import show_error
            show_error(None, PermissionError("test"), context="scan")

        mock_cls.assert_called_once_with(None)
        mock_dialog.set_error.assert_called_once()
        mock_dialog.exec.assert_called_once()

    def test_show_error_passes_context(self, qapp):
        """show_error() should pass the context to set_error()."""
        mock_dialog = MagicMock()
        mock_cls = MagicMock(return_value=mock_dialog)

        with patch("ui.error_dialog.ErrorDialog", mock_cls):
            from ui.error_dialog import show_error
            exc = FileNotFoundError("test")
            show_error(None, exc, context="metadata")

        # Verify set_error was called with the right arguments
        call_args = mock_dialog.set_error.call_args
        assert call_args[0][0] is exc
        assert call_args[0][1] == "metadata"

    def test_show_error_passes_technical_detail(self, qapp):
        """show_error() should pass technical_detail to set_error()."""
        mock_dialog = MagicMock()
        mock_cls = MagicMock(return_value=mock_dialog)

        with patch("ui.error_dialog.ErrorDialog", mock_cls):
            from ui.error_dialog import show_error
            exc = RuntimeError("test")
            show_error(None, exc, technical_detail="custom detail")

        call_args = mock_dialog.set_error.call_args
        assert call_args[0][2] == "custom detail"
