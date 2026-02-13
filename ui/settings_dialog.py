# ============================================================================
# File: /ui/settings_dialog.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Settings dialog for the MeedyaManager GUI.
# Provides tabbed interface to configure:
#   - Watch Folders: Directories to monitor for new media files
#   - Extensions: Supported file extensions for processing
#   - Rename Template: Format string with placeholder tokens
#   - Fallback Metadata: Default values when tags are missing
#   - Character Replacements: Filename-safe character substitutions
#
# Settings are loaded from and saved back to config/settings.json5.
# ============================================================================

import os                                                   # File path operations
import json5                                                # JSON5 config file parsing
import logging                                              # Structured logging
from core.rule_engine import RuleEngine, TemplateSyntaxError  # Template evaluation
from core.tag_registry import get_display_tags               # Available tag names

from PySide6.QtCore import Qt, Signal                       # Core constants and signals
from pathlib import Path                                   # Cross-platform path handling
from PySide6.QtWidgets import (
    QDialog,                                                # Modal dialog base class
    QVBoxLayout,                                            # Vertical layout
    QHBoxLayout,                                            # Horizontal layout
    QTabWidget,                                             # Tabbed container
    QWidget,                                                # Base widget
    QListWidget,                                            # List display widget
    QLineEdit,                                              # Single-line text input
    QPushButton,                                            # Button widget
    QLabel,                                                 # Text label
    QGroupBox,                                              # Framed group container
    QFormLayout,                                            # Label-field pairs layout
    QTableWidget,                                           # Editable table widget
    QTableWidgetItem,                                       # Table cell data item
    QHeaderView,                                            # Table header config
    QDialogButtonBox,                                       # OK/Cancel button row
    QFileDialog,                                            # Native file/folder chooser
    QMessageBox,                                            # Alert/confirm dialog
    QCheckBox,                                              # Checkbox for include-secrets toggle
)

logger = logging.getLogger("MeedyaManager.Settings")

# Path to the config file — resolved relative to the project root
CONFIG_PATH = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "config", "settings.json5"
)


class SettingsDialog(QDialog):
    """
    Modal settings dialog with tabbed sections for all MeedyaManager
    configuration options. Changes are saved to config/settings.json5
    when the user clicks OK.

    Signals:
        settings_changed: Emitted when settings are saved, so the main
                         window can reload config.
    """

    # Signal emitted when settings are successfully saved
    settings_changed = Signal()

    def __init__(self, parent=None):
        """Initialize the settings dialog, load current config, and build UI."""
        super().__init__(parent)
        self.setWindowTitle("MeedyaManager Settings")
        self.setMinimumSize(600, 500)

        # Load current configuration from file
        self._config = self._load_config()

        # Build the dialog UI
        self._setup_ui()

        # Populate all fields with current config values
        self._populate_fields()

    def _load_config(self) -> dict:
        """
        Load settings from config/settings.json5.

        Returns:
            dict: Parsed configuration dictionary, or empty dict on error
        """
        try:
            with open(CONFIG_PATH, "r", encoding="utf-8") as f:
                return json5.load(f)
        except Exception as e:
            logger.error(f"Failed to load config: {e}")
            return {}

    def _save_config(self):
        """
        Save current settings back to config/settings.json5.
        Uses json5.dumps for readable output with proper formatting.
        """
        try:
            with open(CONFIG_PATH, "w", encoding="utf-8") as f:
                # json5.dumps preserves readability; indent=2 for clean formatting
                f.write(json5.dumps(self._config, indent=2))
            logger.info("Settings saved to config/settings.json5")
            self.settings_changed.emit()
        except Exception as e:
            logger.error(f"Failed to save config: {e}")
            QMessageBox.critical(self, "Error", f"Failed to save settings:\n{e}")

    def _setup_ui(self):
        """Create the tabbed settings interface with OK/Cancel buttons."""
        layout = QVBoxLayout(self)

        # Tabbed container for settings sections
        self._tabs = QTabWidget()
        layout.addWidget(self._tabs)

        # Create each settings tab
        self._tabs.addTab(self._create_watch_folders_tab(), "Watch Folders")
        self._tabs.addTab(self._create_extensions_tab(), "Extensions")
        self._tabs.addTab(self._create_template_tab(), "Rename Template")
        self._tabs.addTab(self._create_fallback_tab(), "Fallback Metadata")
        self._tabs.addTab(self._create_replacements_tab(), "Replacements")

        # --- Profile Export / Import section ---
        profile_group = QGroupBox("Configuration Profiles")
        profile_layout = QHBoxLayout(profile_group)

        export_btn = QPushButton("Export Settings...")
        export_btn.setToolTip(
            "Save current settings as a portable .mmprofile file\n"
            "for migration to another machine or platform."
        )
        export_btn.clicked.connect(self._export_profile)
        profile_layout.addWidget(export_btn)

        import_btn = QPushButton("Import Settings...")
        import_btn.setToolTip(
            "Load settings from a .mmprofile file.\n"
            "You can preview changes before applying."
        )
        import_btn.clicked.connect(self._import_profile)
        profile_layout.addWidget(import_btn)

        profile_layout.addStretch()
        layout.addWidget(profile_group)

        # OK / Cancel buttons
        button_box = QDialogButtonBox(
            QDialogButtonBox.StandardButton.Ok | QDialogButtonBox.StandardButton.Cancel
        )
        button_box.accepted.connect(self._on_accept)
        button_box.rejected.connect(self.reject)
        layout.addWidget(button_box)

    # =========================================================================
    # Tab: Watch Folders
    # =========================================================================

    def _create_watch_folders_tab(self) -> QWidget:
        """Create the Watch Folders tab with add/remove/browse controls."""
        tab = QWidget()
        layout = QVBoxLayout(tab)

        layout.addWidget(QLabel(
            "Directories to monitor for new media files.\n"
            "Use ~ for your home directory (e.g. ~/Downloads/Media)."
        ))

        # List of watch folder paths
        self._watch_list = QListWidget()
        layout.addWidget(self._watch_list)

        # Add/Remove/Browse buttons
        btn_layout = QHBoxLayout()

        add_btn = QPushButton("Add")
        add_btn.clicked.connect(self._add_watch_folder)
        btn_layout.addWidget(add_btn)

        browse_btn = QPushButton("Browse...")
        browse_btn.clicked.connect(self._browse_watch_folder)
        btn_layout.addWidget(browse_btn)

        remove_btn = QPushButton("Remove")
        remove_btn.clicked.connect(self._remove_watch_folder)
        btn_layout.addWidget(remove_btn)

        btn_layout.addStretch()
        layout.addLayout(btn_layout)

        return tab

    def _add_watch_folder(self):
        """Add a new empty entry to the watch folders list for manual editing."""
        self._watch_list.addItem("~/")
        # Make the new item editable so the user can type a path
        item = self._watch_list.item(self._watch_list.count() - 1)
        item.setFlags(item.flags() | Qt.ItemFlag.ItemIsEditable)
        self._watch_list.editItem(item)

    def _browse_watch_folder(self):
        """Open a native folder chooser and add the selected path."""
        folder = QFileDialog.getExistingDirectory(self, "Select Watch Folder")
        if folder:
            self._watch_list.addItem(folder)

    def _remove_watch_folder(self):
        """Remove the currently selected watch folder from the list."""
        current_row = self._watch_list.currentRow()
        if current_row >= 0:
            self._watch_list.takeItem(current_row)

    # =========================================================================
    # Tab: Extensions
    # =========================================================================

    def _create_extensions_tab(self) -> QWidget:
        """Create the Extensions tab with add/remove controls."""
        tab = QWidget()
        layout = QVBoxLayout(tab)

        layout.addWidget(QLabel(
            "File extensions to process (without leading dot).\n"
            "Example: mp3, flac, mp4, mkv"
        ))

        # List of valid extensions
        self._ext_list = QListWidget()
        layout.addWidget(self._ext_list)

        # Add/Remove controls
        btn_layout = QHBoxLayout()

        self._ext_input = QLineEdit()
        self._ext_input.setPlaceholderText("Enter extension (e.g. mp3)")
        self._ext_input.returnPressed.connect(self._add_extension)
        btn_layout.addWidget(self._ext_input)

        add_btn = QPushButton("Add")
        add_btn.clicked.connect(self._add_extension)
        btn_layout.addWidget(add_btn)

        remove_btn = QPushButton("Remove")
        remove_btn.clicked.connect(self._remove_extension)
        btn_layout.addWidget(remove_btn)

        layout.addLayout(btn_layout)

        return tab

    def _add_extension(self):
        """Add the typed extension to the list (strips leading dots)."""
        ext = self._ext_input.text().strip().lstrip(".")
        if ext and ext not in [
            self._ext_list.item(i).text()
            for i in range(self._ext_list.count())
        ]:
            self._ext_list.addItem(ext)
            self._ext_input.clear()

    def _remove_extension(self):
        """Remove the selected extension from the list."""
        current_row = self._ext_list.currentRow()
        if current_row >= 0:
            self._ext_list.takeItem(current_row)

    # =========================================================================
    # Tab: Rename Template
    # =========================================================================

    def _create_template_tab(self) -> QWidget:
        """Create the Rename Template tab with editor and live preview."""
        tab = QWidget()
        layout = QVBoxLayout(tab)

        # Help text showing the new <Tag> and $Function() syntax
        layout.addWidget(QLabel(
            "Rename template using <Tag> references and $Function() calls.\n"
            "Examples: <Artist>, <Album>, <Title>, <Ext>, <Media Class>,\n"
            "<$Pad(<Track #>,2)>, $If(<Genre>=Rock,Rock,Other)\n"
            "Use the Rule Builder tab for interactive template editing."
        ))

        # Template text editor
        self._template_input = QLineEdit()
        self._template_input.setPlaceholderText(
            "<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>"
        )
        self._template_input.textChanged.connect(self._update_template_preview)
        layout.addWidget(QLabel("Template:"))
        layout.addWidget(self._template_input)

        # Live preview of template expansion
        self._template_preview = QLabel("")
        self._template_preview.setStyleSheet("padding: 8px; border-radius: 4px;")
        layout.addWidget(QLabel("Preview (sample data):"))
        layout.addWidget(self._template_preview)

        layout.addStretch()

        # Rule engine instance for live preview evaluation
        self._preview_engine = RuleEngine()

        return tab

    def _update_template_preview(self, template_text):
        """
        Update the live preview with sample data expanded into the template.
        Uses the RuleEngine to evaluate <Tag> and $Function() syntax.

        Args:
            template_text (str): Current template string from the editor
        """
        # Sample metadata for preview display (internal snake_case keys)
        sample = {
            "media_class": "Music",
            "artist": "Test Artist",
            "album": "Test Album",
            "album_artist": "Test Artist",
            "track_num": "3",
            "disc_num": "1",
            "title": "Sample Track",
            "year": "2024",
            "genre": "Rock",
            "extension": "mp3",
            "ext": "mp3",
            "filename": "sample_track",
            "format_class": "mp3",
            "quality_type": "Lossy",
            "media_group": "Audio",
            "codec": "MP3",
            "bitrate": "320",
            "sample_rate": "44100",
            "audio_channels": "2",
        }
        try:
            preview = self._preview_engine.evaluate(template_text, sample)
            self._template_preview.setText(f"Result: {preview}")
        except TemplateSyntaxError as e:
            self._template_preview.setText(f"Syntax error: {e}")
        except Exception as e:
            self._template_preview.setText(f"Error: {e}")

    # =========================================================================
    # Tab: Fallback Metadata
    # =========================================================================

    def _create_fallback_tab(self) -> QWidget:
        """Create the Fallback Metadata tab with form fields."""
        tab = QWidget()
        layout = QVBoxLayout(tab)

        layout.addWidget(QLabel(
            "Default values used when metadata tags are missing from a file.\n"
            "These ensure files can still be organized even without complete tags."
        ))

        # Form layout for fallback fields
        form_group = QGroupBox("Default Values")
        form = QFormLayout(form_group)

        self._fb_media_group = QLineEdit()
        form.addRow("Media Group:", self._fb_media_group)

        self._fb_format_class = QLineEdit()
        form.addRow("Format Class:", self._fb_format_class)

        self._fb_media_class = QLineEdit()
        form.addRow("Media Class:", self._fb_media_class)

        self._fb_quality_type = QLineEdit()
        form.addRow("Quality Type:", self._fb_quality_type)

        layout.addWidget(form_group)
        layout.addStretch()

        return tab

    # =========================================================================
    # Tab: Character Replacements
    # =========================================================================

    def _create_replacements_tab(self) -> QWidget:
        """Create the Character Replacements tab with editable table."""
        tab = QWidget()
        layout = QVBoxLayout(tab)

        layout.addWidget(QLabel(
            "Characters in metadata that are unsafe for filenames.\n"
            "Each character on the left is replaced with the character on the right."
        ))

        # Editable two-column table: Character → Replacement
        self._replacements_table = QTableWidget()
        self._replacements_table.setColumnCount(2)
        self._replacements_table.setHorizontalHeaderLabels(["Character", "Replacement"])
        self._replacements_table.horizontalHeader().setSectionResizeMode(
            QHeaderView.ResizeMode.Stretch
        )
        layout.addWidget(self._replacements_table)

        # Add/Remove row buttons
        btn_layout = QHBoxLayout()

        add_btn = QPushButton("Add Row")
        add_btn.clicked.connect(self._add_replacement_row)
        btn_layout.addWidget(add_btn)

        remove_btn = QPushButton("Remove Row")
        remove_btn.clicked.connect(self._remove_replacement_row)
        btn_layout.addWidget(remove_btn)

        btn_layout.addStretch()
        layout.addLayout(btn_layout)

        return tab

    def _add_replacement_row(self):
        """Add a new empty row to the replacements table."""
        row = self._replacements_table.rowCount()
        self._replacements_table.insertRow(row)
        self._replacements_table.setItem(row, 0, QTableWidgetItem(""))
        self._replacements_table.setItem(row, 1, QTableWidgetItem(""))

    def _remove_replacement_row(self):
        """Remove the selected row from the replacements table."""
        current_row = self._replacements_table.currentRow()
        if current_row >= 0:
            self._replacements_table.removeRow(current_row)

    # =========================================================================
    # Data Population & Collection
    # =========================================================================

    def _populate_fields(self):
        """Fill all UI fields with values from the loaded config dictionary."""
        # Watch Folders tab
        for path in self._config.get("watch_paths", []):
            item = self._watch_list.addItem(path)

        # Extensions tab
        for ext in self._config.get("valid_extensions", []):
            self._ext_list.addItem(ext)

        # Rename Template tab (default uses new <Tag> syntax)
        self._template_input.setText(
            self._config.get(
                "rename_format",
                "<Media Class>/<Artist>/<Album>/<$Pad(<Track #>,2)> - <Title>.<Ext>"
            )
        )

        # Fallback Metadata tab
        fallback = self._config.get("fallback_metadata", {})
        self._fb_media_group.setText(fallback.get("media_group", "Audio"))
        self._fb_format_class.setText(fallback.get("format_class", "unknown"))
        self._fb_media_class.setText(fallback.get("media_class", "Music"))
        self._fb_quality_type.setText(fallback.get("quality_type", "Lossy"))

        # Character Replacements tab
        replacements = self._config.get("filename_replacements", {})
        self._replacements_table.setRowCount(len(replacements))
        for row, (char, replacement) in enumerate(replacements.items()):
            self._replacements_table.setItem(row, 0, QTableWidgetItem(char))
            self._replacements_table.setItem(row, 1, QTableWidgetItem(replacement))

    def _collect_fields(self):
        """
        Read all UI fields back into the config dictionary.
        Called before saving to ensure the config reflects current UI state.
        """
        # Watch Folders
        self._config["watch_paths"] = [
            self._watch_list.item(i).text()
            for i in range(self._watch_list.count())
        ]

        # Extensions
        self._config["valid_extensions"] = [
            self._ext_list.item(i).text()
            for i in range(self._ext_list.count())
        ]

        # Rename Template
        self._config["rename_format"] = self._template_input.text()

        # Fallback Metadata
        self._config["fallback_metadata"] = {
            "media_group": self._fb_media_group.text(),
            "format_class": self._fb_format_class.text(),
            "media_class": self._fb_media_class.text(),
            "quality_type": self._fb_quality_type.text(),
        }

        # Character Replacements
        replacements = {}
        for row in range(self._replacements_table.rowCount()):
            char_item = self._replacements_table.item(row, 0)
            repl_item = self._replacements_table.item(row, 1)
            if char_item and char_item.text():
                replacements[char_item.text()] = repl_item.text() if repl_item else ""
        self._config["filename_replacements"] = replacements

    # =========================================================================
    # Profile Export / Import
    # =========================================================================

    def _export_profile(self):
        """Export current settings to a portable .mmprofile file.

        Opens a save dialog, optionally includes API key secrets, then
        calls the config_profile export function.
        """
        from utils.config_profile import export_profile

        # Ask the user where to save the profile
        file_path, _ = QFileDialog.getSaveFileName(
            self,
            "Export Settings Profile",
            str(Path.home() / "MeedyaManager.mmprofile"),
            "MeedyaManager Profiles (*.mmprofile);;All Files (*)",
        )

        if not file_path:
            return                                          # User cancelled

        # Ask whether to include actual API key values
        include_secrets = False
        reply = QMessageBox.question(
            self,
            "Include API Keys?",
            "Do you want to include your actual API key values in the export?\n\n"
            "If YES, the file will contain sensitive credentials —\n"
            "treat it as confidential.\n\n"
            "If NO, only the key names will be included (recommended).",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No,
        )
        if reply == QMessageBox.StandardButton.Yes:
            include_secrets = True

        try:
            result_path = export_profile(
                output_path=Path(file_path),
                profile_name="",
                include_secrets=include_secrets,
            )
            QMessageBox.information(
                self,
                "Export Successful",
                f"Settings exported to:\n{result_path}",
            )
            logger.info(f"Profile exported to {result_path}")
        except Exception as e:
            logger.error(f"Profile export failed: {e}")
            QMessageBox.critical(
                self, "Export Failed", f"Could not export settings:\n{e}"
            )

    def _import_profile(self):
        """Import settings from a .mmprofile file.

        Opens a file dialog, validates the profile, shows a preview of
        changes, and applies them if the user confirms.
        """
        from utils.config_profile import (
            import_profile,
            validate_profile,
        )

        # Ask the user to select a profile file
        file_path, _ = QFileDialog.getOpenFileName(
            self,
            "Import Settings Profile",
            str(Path.home()),
            "MeedyaManager Profiles (*.mmprofile);;ZIP Files (*.zip);;All Files (*)",
        )

        if not file_path:
            return                                          # User cancelled

        profile_path = Path(file_path)

        # Validate the profile before doing anything
        errors = validate_profile(profile_path)
        if errors:
            QMessageBox.critical(
                self,
                "Invalid Profile",
                "The selected profile is invalid:\n\n"
                + "\n".join(f"  - {e}" for e in errors),
            )
            return

        # Preview the changes (dry run)
        try:
            preview = import_profile(profile_path, mode="replace", dry_run=True)
        except Exception as e:
            logger.error(f"Profile import preview failed: {e}")
            QMessageBox.critical(
                self, "Import Failed", f"Could not read profile:\n{e}"
            )
            return

        changes = preview.get("changes", {})
        profile_name = preview.get("profile_name", "Unknown")

        if not changes:
            QMessageBox.information(
                self,
                "No Changes",
                f"Profile '{profile_name}' matches your current settings.\n"
                "Nothing to change.",
            )
            return

        # Build a human-readable change summary
        change_lines = []
        for key, diff in sorted(changes.items()):
            old_val = str(diff["old"])[:40] if diff["old"] is not None else "(not set)"
            new_val = str(diff["new"])[:40] if diff["new"] is not None else "(removed)"
            change_lines.append(f"  {key}: {old_val}  →  {new_val}")

        summary = "\n".join(change_lines[:20])              # Cap display at 20 lines
        if len(change_lines) > 20:
            summary += f"\n  ... and {len(change_lines) - 20} more changes"

        # Confirm with the user
        reply = QMessageBox.question(
            self,
            f"Import '{profile_name}'?",
            f"This will change {len(changes)} setting(s):\n\n"
            f"{summary}\n\n"
            "Apply these changes?",
            QMessageBox.StandardButton.Yes | QMessageBox.StandardButton.No,
            QMessageBox.StandardButton.No,
        )

        if reply != QMessageBox.StandardButton.Yes:
            return

        # Apply the import
        try:
            import_profile(profile_path, mode="replace", dry_run=False)

            # Reload the config into the dialog so the UI reflects new values
            self._config = self._load_config()
            self._watch_list.clear()
            self._ext_list.clear()
            self._populate_fields()

            QMessageBox.information(
                self,
                "Import Successful",
                f"Profile '{profile_name}' imported successfully.\n"
                "Settings have been updated.",
            )
            logger.info(f"Profile '{profile_name}' imported from {profile_path}")
            self.settings_changed.emit()

        except Exception as e:
            logger.error(f"Profile import failed: {e}")
            QMessageBox.critical(
                self, "Import Failed", f"Could not apply settings:\n{e}"
            )

    # =========================================================================
    # Accept / Reject
    # =========================================================================

    def _on_accept(self):
        """Collect field values, save config, and close the dialog."""
        self._collect_fields()
        self._save_config()
        self.accept()
