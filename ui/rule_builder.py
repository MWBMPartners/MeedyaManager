# ============================================================================
# File: /ui/rule_builder.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Rule builder widget for the MeedyaManager GUI.
# Provides a text-based template editor with:
#   - Syntax highlighting for {placeholder} tokens
#   - Tag dropdown for quick insertion of available metadata keys
#   - Test button to preview template expansion with sample data
#   - Live result display showing the expanded rename path
#
# Note: Full visual builder with $If/$And/$Or condition support
# is planned for M3 (Conditional Rules milestone).
# ============================================================================

import re                                                   # Regex for token highlighting
import logging                                              # Structured logging

from PySide6.QtCore import Qt, QRegularExpression           # Core constants, regex
from PySide6.QtGui import (
    QSyntaxHighlighter,                                     # Base class for highlighting
    QTextCharFormat,                                        # Text formatting (colour, bold)
    QColor,                                                 # Colour definition
    QFont,                                                  # Font properties
)
from PySide6.QtWidgets import (
    QWidget,                                                # Base widget class
    QVBoxLayout,                                            # Vertical layout
    QHBoxLayout,                                            # Horizontal layout
    QPlainTextEdit,                                         # Multi-line text editor
    QPushButton,                                            # Button widget
    QLabel,                                                 # Text label
    QComboBox,                                              # Drop-down selector
    QGroupBox,                                              # Framed group container
)

logger = logging.getLogger("MeedyaManager.RuleBuilder")

# Available metadata tag names for template insertion
AVAILABLE_TAGS = [
    "media_class", "media_group", "format_class", "quality_type",
    "artist", "album", "track_num", "title", "extension",
    "duration", "audio_channels", "is_lossless", "description",
    "filepath",
]

# Sample metadata for template testing — matches the CLI rule command's sample data
SAMPLE_METADATA = {
    "filepath": "/example/media/sample_track.mp3",
    "extension": "mp3",
    "format": "mp3",
    "duration": "245",
    "title": "Sample Track",
    "description": "",
    "audio_channels": "2",
    "is_lossless": "False",
    "media_group": "Audio",
    "format_class": "mp3",
    "media_class": "Music",
    "quality_type": "Lossy",
    "artist": "Test Artist",
    "album": "Test Album",
    "track_num": "01",
}


class TemplateHighlighter(QSyntaxHighlighter):
    """
    Syntax highlighter for rename template text.
    Highlights {placeholder} tokens in a distinct colour to make them
    visually distinguishable from literal path text.
    """

    def __init__(self, parent=None):
        """Initialize with highlighting rules for placeholder tokens."""
        super().__init__(parent)

        # Format for valid {placeholder} tokens — blue/cyan, bold
        self._tag_format = QTextCharFormat()
        self._tag_format.setForeground(QColor("#4fc3f7"))
        self._tag_format.setFontWeight(QFont.Weight.Bold)

        # Format for unrecognised {tokens} — orange/red warning colour
        self._unknown_tag_format = QTextCharFormat()
        self._unknown_tag_format.setForeground(QColor("#ef5350"))
        self._unknown_tag_format.setFontWeight(QFont.Weight.Bold)

        # Regex pattern to match any {word} token in the template
        self._pattern = QRegularExpression(r"\{(\w+)\}")

    def highlightBlock(self, text):
        """
        Apply syntax highlighting to a single block (line) of text.
        Called automatically by Qt whenever the text content changes.

        Args:
            text (str): The line of text to highlight
        """
        # Find all {token} matches in the text
        match_iterator = self._pattern.globalMatch(text)
        while match_iterator.hasNext():
            match = match_iterator.next()
            start = match.capturedStart()
            length = match.capturedLength()
            tag_name = match.captured(1)                     # The word inside {}

            # Use valid format for known tags, warning format for unknown tags
            if tag_name in AVAILABLE_TAGS:
                self.setFormat(start, length, self._tag_format)
            else:
                self.setFormat(start, length, self._unknown_tag_format)


class RuleBuilder(QWidget):
    """
    Rule builder panel containing:
    - Template text editor with syntax highlighting
    - Tag dropdown for quick tag insertion
    - Test button to expand template with sample data
    - Result display showing the expanded path

    This provides a visual way to build and test rename templates
    before applying them in the configuration.
    """

    def __init__(self, parent=None):
        """Initialize the rule builder with editor, controls, and result display."""
        super().__init__(parent)
        self._setup_ui()

    def _setup_ui(self):
        """Create and arrange all child widgets in the panel layout."""
        layout = QVBoxLayout(self)
        layout.setContentsMargins(12, 12, 12, 12)

        # --- Template Editor Section ---
        editor_group = QGroupBox("Rename Template")
        editor_layout = QVBoxLayout(editor_group)

        editor_layout.addWidget(QLabel(
            "Enter your rename template below. Use {tag} syntax for metadata placeholders.\n"
            "Tags shown in blue are valid; tags shown in red are unrecognised."
        ))

        # Multi-line template editor with syntax highlighting
        self._editor = QPlainTextEdit()
        self._editor.setPlaceholderText("{media_class}/{artist}/{album}/{track_num} - {title}.{extension}")
        self._editor.setMaximumHeight(100)

        # Attach the syntax highlighter to the editor's document
        self._highlighter = TemplateHighlighter(self._editor.document())

        editor_layout.addWidget(self._editor)

        # Tag insertion controls (dropdown + insert button)
        tag_layout = QHBoxLayout()

        tag_layout.addWidget(QLabel("Insert tag:"))

        self._tag_combo = QComboBox()
        self._tag_combo.addItems(AVAILABLE_TAGS)
        tag_layout.addWidget(self._tag_combo)

        insert_btn = QPushButton("Insert")
        insert_btn.setToolTip("Insert the selected tag at the cursor position")
        insert_btn.clicked.connect(self._insert_tag)
        tag_layout.addWidget(insert_btn)

        tag_layout.addStretch()

        # Test button to preview template expansion
        test_btn = QPushButton("Test Template")
        test_btn.setObjectName("primaryButton")
        test_btn.setToolTip("Expand the template using sample metadata")
        test_btn.clicked.connect(self._test_template)
        tag_layout.addWidget(test_btn)

        editor_layout.addLayout(tag_layout)
        layout.addWidget(editor_group)

        # --- Result Display Section ---
        result_group = QGroupBox("Test Result")
        result_layout = QVBoxLayout(result_group)

        self._result_label = QLabel("Click 'Test Template' to see the expanded result.")
        self._result_label.setWordWrap(True)
        self._result_label.setTextInteractionFlags(Qt.TextInteractionFlag.TextSelectableByMouse)
        result_layout.addWidget(self._result_label)

        layout.addWidget(result_group)

        # --- Available Tags Reference ---
        tags_group = QGroupBox("Available Tags")
        tags_layout = QVBoxLayout(tags_group)

        tags_text = ", ".join(f"{{{tag}}}" for tag in AVAILABLE_TAGS)
        tags_label = QLabel(tags_text)
        tags_label.setWordWrap(True)
        tags_layout.addWidget(tags_label)

        layout.addWidget(tags_group)

        layout.addStretch()

    def _insert_tag(self):
        """Insert the selected tag at the current cursor position in the editor."""
        tag = self._tag_combo.currentText()
        if tag:
            self._editor.insertPlainText(f"{{{tag}}}")
            self._editor.setFocus()

    def _test_template(self):
        """
        Expand the current template using sample metadata and display the result.
        Reports any missing or invalid tags with a clear error message.
        """
        template = self._editor.toPlainText().strip()
        if not template:
            self._result_label.setText("Please enter a template to test.")
            return

        try:
            result = template.format(**SAMPLE_METADATA)
            self._result_label.setText(f"Result: {result}")
        except KeyError as e:
            self._result_label.setText(
                f"Missing tag: {e}\n\nAvailable tags: {', '.join(AVAILABLE_TAGS)}"
            )
        except Exception as e:
            self._result_label.setText(f"Error: {e}")

    def get_template(self) -> str:
        """Return the current template text from the editor."""
        return self._editor.toPlainText().strip()

    def set_template(self, template: str):
        """Set the template text in the editor."""
        self._editor.setPlainText(template)
