# ============================================================================
# File: /ui/error_dialog.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# User-friendly error dialog for the MeedyaManager GUI.
#
# Replaces raw QMessageBox.critical() calls with a structured dialog that
# provides:
#   - A clear headline describing what went wrong
#   - A human-readable explanation of the problem
#   - Suggested actions the user can take
#   - A collapsible "Technical Details" section (hidden by default)
#   - A "Copy to Clipboard" button for easy sharing
#
# The dialog uses the error catalog in utils/error_messages.py to translate
# exception types into user-friendly language.
#
# Usage:
#   from ui.error_dialog import ErrorDialog, show_error
#
#   # Quick convenience function (recommended):
#   show_error(parent_widget, exception, context="scan")
#
#   # Or create the dialog manually for more control:
#   dialog = ErrorDialog(parent_widget)
#   dialog.set_error(exception, context="lookup", technical_detail=traceback_str)
#   dialog.exec()
# ============================================================================

import logging                                             # Structured logging
import traceback                                           # Traceback formatting

from PySide6.QtCore import Qt                              # Core constants
from PySide6.QtWidgets import (
    QDialog,                                               # Dialog base class
    QVBoxLayout,                                           # Vertical layout
    QHBoxLayout,                                           # Horizontal layout
    QLabel,                                                # Text labels
    QPushButton,                                           # Clickable buttons
    QTextEdit,                                             # Multiline text display
    QApplication,                                          # Clipboard access
    QSizePolicy,                                           # Size policy hints
)

from utils.error_messages import (
    get_user_friendly_message,                             # Error catalog lookup
    ErrorMessage,                                          # Structured message type
)

logger = logging.getLogger("MeedyaManager.ErrorDialog")


class ErrorDialog(QDialog):
    """
    User-friendly error dialog with structured error presentation.

    Shows a headline, explanation, and suggested action. Technical details
    (traceback, exception details) are hidden in a collapsible section that
    can be expanded by clicking "Show Details".

    The dialog provides a "Copy to Clipboard" button that copies all error
    information (including technical details) for easy pasting into bug
    reports or support emails.
    """

    def __init__(self, parent=None):
        """
        Initialize the error dialog.

        Args:
            parent: Parent QWidget. Should be the main window or the panel
                    that encountered the error.
        """
        super().__init__(parent)

        self.setWindowTitle("MeedyaManager — Error")
        self.setMinimumWidth(480)
        self.setMaximumWidth(700)

        # Store the full error text for clipboard copying
        self._full_error_text = ""

        self._setup_ui()

    def _setup_ui(self):
        """Build the dialog layout with headline, body, details, and buttons."""
        layout = QVBoxLayout(self)
        layout.setSpacing(12)
        layout.setContentsMargins(20, 20, 20, 16)

        # --- Headline Label ---
        # Large, bold text describing the error category
        self._headline_label = QLabel()
        self._headline_label.setWordWrap(True)
        self._headline_label.setStyleSheet(
            "font-size: 15px; font-weight: bold; color: #D32F2F;"
        )
        layout.addWidget(self._headline_label)

        # --- Explanation Label ---
        # Human-readable description of what happened
        self._explanation_label = QLabel()
        self._explanation_label.setWordWrap(True)
        self._explanation_label.setStyleSheet("font-size: 13px;")
        layout.addWidget(self._explanation_label)

        # --- Suggestion Label ---
        # Actionable steps the user can take
        self._suggestion_label = QLabel()
        self._suggestion_label.setWordWrap(True)
        self._suggestion_label.setStyleSheet(
            "font-size: 13px; font-style: italic; color: #555;"
        )
        layout.addWidget(self._suggestion_label)

        # --- Technical Details (collapsible) ---
        # Hidden by default; toggled by the "Show Details" / "Hide Details" button
        self._details_toggle = QPushButton("Show Details")
        self._details_toggle.setCheckable(True)
        self._details_toggle.setFlat(True)
        self._details_toggle.setStyleSheet(
            "text-align: left; color: #1976D2; font-size: 12px; "
            "padding: 0; border: none;"
        )
        self._details_toggle.toggled.connect(self._on_toggle_details)
        layout.addWidget(self._details_toggle)

        self._details_text = QTextEdit()
        self._details_text.setReadOnly(True)
        self._details_text.setVisible(False)                # Hidden by default
        self._details_text.setMinimumHeight(120)
        self._details_text.setMaximumHeight(250)
        self._details_text.setSizePolicy(
            QSizePolicy.Expanding, QSizePolicy.Preferred,
        )
        self._details_text.setStyleSheet(
            "font-family: monospace; font-size: 11px; "
            "background-color: #F5F5F5; color: #333; "
            "border: 1px solid #DDD; border-radius: 4px; padding: 6px;"
        )
        layout.addWidget(self._details_text)

        # --- Button Row ---
        button_layout = QHBoxLayout()
        button_layout.setSpacing(8)

        # "Copy to Clipboard" button — copies full error text for bug reports
        self._copy_button = QPushButton("Copy to Clipboard")
        self._copy_button.clicked.connect(self._copy_to_clipboard)
        button_layout.addWidget(self._copy_button)

        button_layout.addStretch()                          # Push OK to the right

        # "OK" button — closes the dialog
        self._ok_button = QPushButton("OK")
        self._ok_button.setDefault(True)
        self._ok_button.clicked.connect(self.accept)
        self._ok_button.setMinimumWidth(80)
        button_layout.addWidget(self._ok_button)

        layout.addLayout(button_layout)

    def set_error(self, exception: Exception, context: str = "",
                  technical_detail: str = ""):
        """
        Populate the dialog with error information.

        Looks up the exception in the error catalog to get a user-friendly
        message, and optionally shows technical details (traceback, etc.)
        in the collapsible section.

        Args:
            exception:        The exception instance that occurred.
            context:          Operation context string (e.g., "scan", "lookup",
                              "metadata", "config", "worker").
            technical_detail: Optional pre-formatted technical detail string
                              (e.g., a traceback). If empty, the exception's
                              repr and traceback are used automatically.
        """
        # Look up user-friendly message from the error catalog
        msg = get_user_friendly_message(exception, context)

        self.set_message(msg, exception, technical_detail)

    def set_message(self, msg: ErrorMessage, exception: Exception = None,
                    technical_detail: str = ""):
        """
        Populate the dialog with an ErrorMessage directly.

        This is useful when you want to override the catalog lookup and
        provide a custom ErrorMessage.

        Args:
            msg:              The ErrorMessage to display.
            exception:        Optional exception instance for technical details.
            technical_detail: Optional pre-formatted technical detail string.
        """
        # Set the visible labels
        self._headline_label.setText(msg.headline)
        self._explanation_label.setText(msg.explanation)
        self._suggestion_label.setText(msg.suggestion)

        # Build technical details text
        detail_parts = []
        if exception is not None:
            detail_parts.append(f"Exception: {type(exception).__name__}: {exception}")
        if technical_detail:
            detail_parts.append(technical_detail)
        elif exception is not None:
            # Auto-format the traceback if no explicit detail was provided
            try:
                tb_text = "".join(traceback.format_exception(
                    type(exception), exception, exception.__traceback__,
                ))
                detail_parts.append(tb_text)
            except Exception:
                pass                                        # Don't fail on traceback formatting

        detail_text = "\n\n".join(detail_parts) if detail_parts else ""
        self._details_text.setPlainText(detail_text)

        # Build the full error text for clipboard copying
        self._full_error_text = (
            f"Error: {msg.headline}\n"
            f"Explanation: {msg.explanation}\n"
            f"Suggestion: {msg.suggestion}\n"
        )
        if detail_text:
            self._full_error_text += f"\nTechnical Details:\n{detail_text}\n"

        # Hide the details toggle if there are no technical details
        has_details = bool(detail_text.strip())
        self._details_toggle.setVisible(has_details)

        # Reset the details section to collapsed state
        self._details_toggle.setChecked(False)
        self._details_text.setVisible(False)

    def _on_toggle_details(self, checked: bool):
        """Toggle visibility of the technical details section."""
        self._details_text.setVisible(checked)
        self._details_toggle.setText(
            "Hide Details" if checked else "Show Details"
        )
        # Resize the dialog to accommodate the details section
        self.adjustSize()

    def _copy_to_clipboard(self):
        """Copy the full error text (including technical details) to the clipboard."""
        clipboard = QApplication.clipboard()
        clipboard.setText(self._full_error_text)

        # Briefly change button text to confirm the copy
        self._copy_button.setText("Copied!")
        # Use a single-shot timer to restore the button text after 2 seconds
        from PySide6.QtCore import QTimer
        QTimer.singleShot(2000, lambda: self._copy_button.setText("Copy to Clipboard"))


# ============================================================================
# Convenience Function
# ============================================================================

def show_error(parent, exception: Exception, context: str = "",
               technical_detail: str = ""):
    """
    Show a user-friendly error dialog for the given exception.

    This is the recommended way to display errors in the MeedyaManager GUI.
    It creates an ErrorDialog, populates it with the appropriate user-friendly
    message from the error catalog, and shows it as a modal dialog.

    Args:
        parent:           Parent QWidget (usually the panel or main window).
        exception:        The exception instance that occurred.
        context:          Operation context string (e.g., "scan", "lookup",
                          "metadata", "config", "worker").
        technical_detail: Optional pre-formatted technical detail string.

    Example:
        try:
            do_something_risky()
        except Exception as e:
            show_error(self, e, context="scan")
    """
    dialog = ErrorDialog(parent)
    dialog.set_error(exception, context, technical_detail)
    dialog.exec()
    logger.debug(f"Error dialog shown: {type(exception).__name__}: {exception}")
