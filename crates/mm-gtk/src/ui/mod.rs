// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — GTK UI Component Module (M6)
//
// All GTK4/Adwaita widget construction lives in this module tree.
// Each sub-module owns one panel/view and its associated state binding.

/// Main application window (AdwApplicationWindow + AdwTabView shell + theme toggle)
pub mod main_window;

/// Library / Scan panel — folder picker, drag-and-drop, rename preview, execute
pub mod scan_panel;

/// Metadata editor panel — tag viewer and editor with cover art display (M6)
pub mod metadata_panel;

/// Metadata Lookup panel — provider search, results list, apply-to-file (M6)
pub mod lookup_panel;

/// Rules panel — full rename template builder with live preview and sample editor (M6)
pub mod rules_panel;

/// Settings panel — configuration display and editing with full JSON5 save (M6)
pub mod settings_panel;

/// Error and confirmation dialog helpers (M6)
pub mod error_dialog;
