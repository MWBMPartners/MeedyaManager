# ============================================================================
# File: /ui/rule_builder.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Rule builder widget for the MeedyaManager GUI.
# Provides a text-based template editor with:
#   - Syntax highlighting for <Tag> references and $Function() calls
#   - Tag dropdown for quick insertion of available metadata tags
#   - Test button to preview template expansion with sample data
#   - Live result display showing the expanded rename path
#   - Template validation with error reporting
#
# Uses MusicBee-style template syntax (M3): <Tag Name>, $Function()
# Legacy {placeholder} syntax is highlighted with a deprecation warning.
# ============================================================================

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

from core.tag_registry import get_display_tags, is_valid_tag  # Tag name registry
from core.rule_engine import (
    RuleEngine,                                             # Template evaluator
    TemplateSyntaxError,                                    # Syntax error type
    TemplateEvalError,                                      # Evaluation error type
)

logger = logging.getLogger("MeedyaManager.RuleBuilder")

# List of available template functions for reference display
AVAILABLE_FUNCTIONS = [
    "$If()", "$And()", "$Or()", "$IsNull()", "$Contains()", "$IsMatch()",
    "$Replace()", "$RxReplace()", "$Left()", "$Right()", "$Upper()", "$Lower()",
    "$Trim()", "$Split()", "$RSplit()", "$First()", "$Pad()", "$Date()",
    "$Sort()", "$Group()",
]

# Sample metadata for template testing — uses internal snake_case keys
# (the rule engine handles <Tag Name> → internal key resolution)
SAMPLE_METADATA = {
    "filepath": "/example/media/sample_track.mp3",
    "filename": "sample_track",
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
    "album_artist": "Test Artist",
    "year": "2025",
    "genre": "Rock; Alternative",
    "track_num": "3",
    "track_number": "3",
    "disc_num": "1",
    "total_tracks": "12",
    "codec": "MP3",
    "bitrate": "320",
    "sample_rate": "44100",
    "bit_depth": "16",
    "date_added": "2025-06-15",
}

# Shared rule engine instance for template testing
_engine = RuleEngine()


class TemplateHighlighter(QSyntaxHighlighter):
    """
    Syntax highlighter for MusicBee-style rename templates.
    Highlights three pattern types:
      - <TagName> references: cyan/blue for valid, red for unknown
      - $FunctionName( calls: green, bold
      - {placeholder} (legacy): yellow with deprecation indication
    """

    def __init__(self, parent=None):
        """Initialize with highlighting rules for all token types."""
        super().__init__(parent)

        # Format for valid <Tag> references — cyan, bold
        self._tag_format = QTextCharFormat()
        self._tag_format.setForeground(QColor("#4fc3f7"))
        self._tag_format.setFontWeight(QFont.Weight.Bold)

        # Format for unrecognised <Tags> — red warning colour
        self._unknown_tag_format = QTextCharFormat()
        self._unknown_tag_format.setForeground(QColor("#ef5350"))
        self._unknown_tag_format.setFontWeight(QFont.Weight.Bold)

        # Format for $Function( calls — green, bold
        self._func_format = QTextCharFormat()
        self._func_format.setForeground(QColor("#66bb6a"))
        self._func_format.setFontWeight(QFont.Weight.Bold)

        # Format for legacy {placeholder} tokens — yellow (deprecated)
        self._legacy_format = QTextCharFormat()
        self._legacy_format.setForeground(QColor("#ffa726"))

        # Regex patterns for the three token types
        self._tag_pattern = QRegularExpression(r"<([^>]+)>")      # <TagName>
        self._func_pattern = QRegularExpression(r"\$([A-Za-z]+)\(")  # $FuncName(
        self._legacy_pattern = QRegularExpression(r"\{(\w+)\}")   # {placeholder}

    def highlightBlock(self, text):
        """
        Apply syntax highlighting to a single block (line) of text.
        Called automatically by Qt whenever the text content changes.

        Args:
            text (str): The line of text to highlight
        """
        # Highlight <Tag> references
        match_iterator = self._tag_pattern.globalMatch(text)
        while match_iterator.hasNext():
            match = match_iterator.next()
            start = match.capturedStart()
            length = match.capturedLength()
            tag_name = match.captured(1).strip()   # Text between < and >

            # Use valid format for known/custom tags, warning format otherwise
            if is_valid_tag(tag_name):
                self.setFormat(start, length, self._tag_format)
            else:
                self.setFormat(start, length, self._unknown_tag_format)

        # Highlight $Function( calls — green
        match_iterator = self._func_pattern.globalMatch(text)
        while match_iterator.hasNext():
            match = match_iterator.next()
            start = match.capturedStart()
            length = match.capturedLength()
            self.setFormat(start, length, self._func_format)

        # Highlight legacy {placeholder} tokens — yellow (deprecated)
        match_iterator = self._legacy_pattern.globalMatch(text)
        while match_iterator.hasNext():
            match = match_iterator.next()
            start = match.capturedStart()
            length = match.capturedLength()
            self.setFormat(start, length, self._legacy_format)


class RuleBuilder(QWidget):
    """
    Rule builder panel containing:
    - Template text editor with MusicBee-style syntax highlighting
    - Tag dropdown for quick <Tag Name> insertion
    - Test button to expand template with sample data via the rule engine
    - Result display showing the expanded path
    - Available tags and functions reference

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
            "Enter your rename template below using <Tag Name> syntax.\n"
            "Tags shown in cyan are valid; unknown tags are shown in red.\n"
            "Functions like $If(), $Pad() are shown in green."
        ))

        # Multi-line template editor with syntax highlighting
        self._editor = QPlainTextEdit()
        self._editor.setPlaceholderText(
            "<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>"
        )
        self._editor.setMaximumHeight(100)

        # Attach the syntax highlighter to the editor's document
        self._highlighter = TemplateHighlighter(self._editor.document())

        editor_layout.addWidget(self._editor)

        # Tag insertion controls (dropdown + insert button)
        tag_layout = QHBoxLayout()

        tag_layout.addWidget(QLabel("Insert tag:"))

        # Populate dropdown from the tag registry
        self._tag_combo = QComboBox()
        self._tag_combo.addItems(get_display_tags())
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

        # --- Available Tags & Functions Reference ---
        ref_group = QGroupBox("Available Tags & Functions")
        ref_layout = QVBoxLayout(ref_group)

        # Display tags from registry
        tags_text = ", ".join(f"<{tag}>" for tag in get_display_tags())
        tags_label = QLabel(f"Tags: {tags_text}")
        tags_label.setWordWrap(True)
        ref_layout.addWidget(tags_label)

        # Display available functions
        funcs_text = ", ".join(AVAILABLE_FUNCTIONS)
        funcs_label = QLabel(f"Functions: {funcs_text}")
        funcs_label.setWordWrap(True)
        ref_layout.addWidget(funcs_label)

        layout.addWidget(ref_group)

        layout.addStretch()

    def _insert_tag(self):
        """Insert the selected tag at the current cursor position using <Tag> syntax."""
        tag = self._tag_combo.currentText()
        if tag:
            # Insert as <Tag Name> (MusicBee-style syntax)
            self._editor.insertPlainText(f"<{tag}>")
            self._editor.setFocus()

    def _test_template(self):
        """
        Expand the current template using sample metadata via the rule engine.
        Reports any syntax or evaluation errors with clear error messages.
        """
        template = self._editor.toPlainText().strip()
        if not template:
            self._result_label.setText("Please enter a template to test.")
            return

        try:
            # Evaluate using the M3 rule engine
            result = _engine.evaluate(template, SAMPLE_METADATA)
            self._result_label.setText(f"Result: {result}")
        except TemplateSyntaxError as e:
            self._result_label.setText(f"Syntax error: {e}")
        except TemplateEvalError as e:
            self._result_label.setText(f"Evaluation error: {e}")
        except Exception as e:
            self._result_label.setText(f"Error: {e}")

    def get_template(self) -> str:
        """Return the current template text from the editor."""
        return self._editor.toPlainText().strip()

    def set_template(self, template: str):
        """Set the template text in the editor."""
        self._editor.setPlainText(template)
