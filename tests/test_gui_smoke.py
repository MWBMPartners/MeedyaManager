# ============================================================================
# File: /tests/test_gui_smoke.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Smoke tests for the MeedyaManager GUI components.
# Verifies that all major widgets can be instantiated without crashing.
# Uses QT_QPA_PLATFORM=offscreen to run in headless/CI environments.
# ============================================================================

import os
import sys
import pytest

# Force offscreen rendering for headless testing (CI, SSH, etc.)
os.environ["QT_QPA_PLATFORM"] = "offscreen"

from PySide6.QtWidgets import QApplication                  # Qt application instance


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


def test_main_window_instantiates(qapp):
    """Verify MainWindow can be created without crashing."""
    from ui.main_window import MainWindow
    window = MainWindow()
    assert window is not None
    assert window.windowTitle() == "MeedyaManager"


def test_main_window_has_tabs(qapp):
    """Verify MainWindow contains the expected tabs."""
    from ui.main_window import MainWindow
    window = MainWindow()
    assert window._tab_widget.count() == 2
    assert window._tab_widget.tabText(0) == "Scan / Preview"
    assert window._tab_widget.tabText(1) == "Rules"


def test_preview_panel_instantiates(qapp):
    """Verify PreviewPanel can be created without crashing."""
    from ui.preview_panel import PreviewPanel
    panel = PreviewPanel()
    assert panel is not None
    assert panel._scan_btn is not None
    assert panel._table_view is not None


def test_settings_dialog_instantiates(qapp):
    """Verify SettingsDialog can be created without crashing."""
    from ui.settings_dialog import SettingsDialog
    dialog = SettingsDialog()
    assert dialog is not None
    assert dialog.windowTitle() == "MeedyaManager Settings"


def test_settings_dialog_has_tabs(qapp):
    """Verify SettingsDialog contains all expected settings tabs."""
    from ui.settings_dialog import SettingsDialog
    dialog = SettingsDialog()
    tab_count = dialog._tabs.count()
    assert tab_count == 5, f"Expected 5 settings tabs, got {tab_count}"


def test_rule_builder_instantiates(qapp):
    """Verify RuleBuilder can be created without crashing."""
    from ui.rule_builder import RuleBuilder
    builder = RuleBuilder()
    assert builder is not None
    assert builder._editor is not None
    assert builder._tag_combo is not None


def test_rule_builder_get_set_template(qapp):
    """Verify RuleBuilder can get and set template text."""
    from ui.rule_builder import RuleBuilder
    builder = RuleBuilder()
    test_template = "{media_class}/{artist}/{title}.{extension}"
    builder.set_template(test_template)
    assert builder.get_template() == test_template


def test_system_tray_instantiates(qapp):
    """Verify SystemTrayIcon can be created without crashing."""
    from ui.system_tray import SystemTrayIcon
    tray = SystemTrayIcon()
    assert tray is not None
    assert tray.toolTip() == "MeedyaManager"


def test_detect_platform(qapp):
    """Verify platform detection returns a valid platform string."""
    from ui.platform_style import detect_platform
    platform = detect_platform()
    assert platform in ("macos", "windows", "linux")


def test_get_system_theme(qapp):
    """Verify system theme detection returns dark or light."""
    from ui.platform_style import get_system_theme
    theme = get_system_theme()
    assert theme in ("dark", "light")


def test_template_highlighter_instantiates(qapp):
    """Verify TemplateHighlighter can be attached to a document."""
    from PySide6.QtGui import QTextDocument
    from ui.rule_builder import TemplateHighlighter
    doc = QTextDocument()
    highlighter = TemplateHighlighter(doc)
    assert highlighter is not None
