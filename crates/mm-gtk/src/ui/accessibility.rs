// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — GTK4 Accessibility Helpers (Issue #128)
//
// Provides convenience wrappers for applying AT-SPI2 accessibility
// metadata to GTK4 widgets consistently across all panels.
//
// GTK4 implements the AT-SPI2 protocol via libatk-bridge.  Every widget
// inherits the `Accessible` trait from `gtk4::prelude::AccessibleExt`.
// Key properties set here:
//   - AccessibleRole  — semantics (button, text-box, list, status, etc.)
//   - AccessibleProperty::Label   — the primary name spoken by screen readers
//   - AccessibleProperty::Description — longer hint (analogous to tooltip)
//   - AccessibleState::Busy       — set while async operations are running
//
// Screen reader tested against: Orca 46+ on GNOME 47.
//
// Usage example:
//
//   use crate::ui::accessibility;
//   accessibility::set_label(&my_button, "Scan folder");
//   accessibility::set_description(&my_button, "Scans the selected directory for media files.");

use gtk4 as gtk;
// AccessibleExt provides auto-generated bindings; AccessibleExtManual provides
// the hand-written ones, which is where update_property() and update_state()
// live (they have variadic-like signatures gir can't auto-generate). Importing
// only AccessibleExt means those two methods are not in scope — gtk4 0.9 splits
// the trait this way intentionally.
use gtk::prelude::{AccessibleExt, AccessibleExtManual};

// ── Label helpers ─────────────────────────────────────────────────────────

/// Set the primary accessible label on any GTK4 widget.
///
/// The label is announced by Orca when the widget receives focus.
/// It is equivalent to HTML `aria-label`.
pub fn set_label<W: AccessibleExt>(widget: &W, label: &str) {
    // GTK4 AccessibleExt::update_property() accepts an array of (property, value) pairs.
    // AccessibleProperty::Label corresponds to AT-SPI2 "accessible-name".
    widget.update_property(&[(
        gtk::AccessibleProperty::Label,
        &label,
    )]);
}

/// Set the accessible description (secondary hint) on a GTK4 widget.
///
/// The description is a longer clarifying hint read after the label,
/// analogous to HTML `aria-description` or a tooltip for screen readers.
pub fn set_description<W: AccessibleExt>(widget: &W, description: &str) {
    widget.update_property(&[(
        gtk::AccessibleProperty::Description,
        &description,
    )]);
}

// ── State helpers ─────────────────────────────────────────────────────────

/// Mark a widget as busy (async operation in progress) for AT-SPI2.
///
/// Orca announces "busy" when the state transitions to true, and
/// re-announces the widget role when it transitions back to false.
/// Use this during scan, rename, export, and server-start operations.
pub fn set_busy<W: AccessibleExt>(widget: &W, busy: bool) {
    // AT-SPI2 AccessibleState::Busy maps to aria-busy in web terms.
    widget.update_state(&[(gtk::AccessibleState::Busy, &busy)]);
}

/// Mark a widget as expanded or collapsed (e.g. a disclosure group).
pub fn set_expanded<W: AccessibleExt>(widget: &W, expanded: bool) {
    widget.update_state(&[(gtk::AccessibleState::Expanded, &expanded)]);
}

// ── Tab label helper ──────────────────────────────────────────────────────

/// Returns the accessible label for each top-level application tab.
///
/// These strings are set on the `AdwTabPage` title and must be consistent
/// with the icon tooltips and `update_property` calls on the page widgets.
/// They are also used in accessibility tests to verify label completeness.
pub fn tab_label(index: usize) -> Option<&'static str> {
    // Tab order must match `main_window::build()` add_tab() call order.
    const LABELS: [&str; 8] = [
        "Library",   // 0 — folder scan + rename preview
        "Metadata",  // 1 — tag editor + cover art
        "Lookup",    // 2 — metadata provider search
        "Rules",     // 3 — rename template builder
        "Cloud",     // 4 — cloud storage monitor
        "Export",    // 5 — database export
        "Server",    // 6 — secure media server
        "Settings",  // 7 — application settings
    ];
    LABELS.get(index).copied()
}

/// Returns the accessible description for each top-level application tab.
pub fn tab_description(index: usize) -> Option<&'static str> {
    const DESCRIPTIONS: [&str; 8] = [
        "Scan a media folder and preview or execute renames.",
        "View and edit metadata tags and cover art for media files.",
        "Search online metadata providers for track and video information.",
        "Build and test rename templates using the MusicBee-style rule engine.",
        "Monitor OneDrive, Google Drive, and Dropbox for new media files.",
        "Export the media library to a SQL database backend.",
        "Configure and control the HTTPS media streaming server.",
        "Configure MeedyaManager preferences and provider API keys.",
    ];
    DESCRIPTIONS.get(index).copied()
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Number of tabs in the main window (must stay in sync with main_window.rs)
    const EXPECTED_TAB_COUNT: usize = 8;

    #[test]
    fn tab_labels_cover_all_tabs() {
        // Every tab index 0..N-1 must have a non-empty label
        for i in 0..EXPECTED_TAB_COUNT {
            let label = tab_label(i);
            assert!(label.is_some(), "Missing label for tab index {i}");
            assert!(!label.unwrap().is_empty(), "Empty label for tab index {i}");
        }
    }

    #[test]
    fn tab_label_out_of_bounds_returns_none() {
        assert!(tab_label(EXPECTED_TAB_COUNT).is_none());
        assert!(tab_label(99).is_none());
    }

    #[test]
    fn tab_descriptions_cover_all_tabs() {
        for i in 0..EXPECTED_TAB_COUNT {
            let desc = tab_description(i);
            assert!(desc.is_some(), "Missing description for tab index {i}");
            assert!(!desc.unwrap().is_empty(), "Empty description for tab index {i}");
        }
    }

    #[test]
    fn tab_description_out_of_bounds_returns_none() {
        assert!(tab_description(EXPECTED_TAB_COUNT).is_none());
    }

    #[test]
    fn tab_labels_are_unique() {
        // Collect all labels and check for duplicates
        let labels: Vec<_> = (0..EXPECTED_TAB_COUNT)
            .filter_map(tab_label)
            .collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(labels.len(), unique.len(), "Tab labels must all be unique");
    }

    #[test]
    fn tab_labels_match_expected() {
        // Explicit checks so a reorder is caught immediately
        assert_eq!(tab_label(0), Some("Library"));
        assert_eq!(tab_label(1), Some("Metadata"));
        assert_eq!(tab_label(2), Some("Lookup"));
        assert_eq!(tab_label(3), Some("Rules"));
        assert_eq!(tab_label(4), Some("Cloud"));
        assert_eq!(tab_label(5), Some("Export"));
        assert_eq!(tab_label(6), Some("Server"));
        assert_eq!(tab_label(7), Some("Settings"));
    }

    #[test]
    fn all_descriptions_end_with_period() {
        // Accessibility descriptions should be full sentences
        for i in 0..EXPECTED_TAB_COUNT {
            let desc = tab_description(i).unwrap();
            assert!(
                desc.ends_with('.'),
                "Tab {i} description should end with a period: '{desc}'"
            );
        }
    }

    #[test]
    fn all_labels_no_html() {
        // Labels must not contain HTML/angle brackets
        for i in 0..EXPECTED_TAB_COUNT {
            let label = tab_label(i).unwrap();
            assert!(!label.contains('<') && !label.contains('>'),
                "Tab {i} label must not contain HTML: '{label}'");
        }
    }
}
