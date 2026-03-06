// (C) 2025-2026 MWBM Partners Ltd
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

/// Cloud Storage Monitor panel — provider connections + event log (M7)
pub mod cloud_panel;

/// Database Export panel — backend picker, DSN input, schema preview, export log (M9)
pub mod export_panel;

/// Secure Media Server panel — start/stop, route table, access log (M10)
pub mod server_panel;

/// Accessibility helpers — consistent AT-SPI2 role/label utilities (Issue #128)
pub mod accessibility;
