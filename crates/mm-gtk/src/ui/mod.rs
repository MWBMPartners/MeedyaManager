// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — GTK UI Component Module
//
// All GTK4/Adwaita widget construction lives in this module tree.
// Each sub-module owns one panel/view and its associated state binding.

/// Main application window (AdwApplicationWindow + AdwTabView shell)
pub mod main_window;

/// Library / Scan panel — folder picker, rename preview, execute
pub mod scan_panel;

/// Metadata editor panel — tag viewer and editor
pub mod metadata_panel;

/// Rules panel — rename template builder (stub for M6 full implementation)
pub mod rules_panel;

/// Settings panel — configuration display and basic editing
pub mod settings_panel;
