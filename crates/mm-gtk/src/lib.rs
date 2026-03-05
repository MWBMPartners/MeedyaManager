// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — GTK4/Adwaita Desktop GUI Library
//
// This library crate exposes the application modules so they can be imported
// in integration tests without running the GTK event loop.  The binary crate
// (`main.rs`) simply calls `run_app()` from here.
//
// Module layout:
//   app.rs         — Application struct, activate handler, window construction
//   state.rs       — Non-GTK application state (testable without a display)
//   ui/mod.rs      — UI component module root
//   ui/main_window — AdwApplicationWindow + AdwTabView shell
//   ui/scan_panel  — Library/Scan tab: folder picker + preview list
//   ui/metadata_panel — Metadata editor tab
//   ui/rules_panel — Rules tab (stub for M6)
//   ui/settings_panel — Settings tab

/// Application setup and event-loop entry point
pub mod app;

/// Non-GTK application state (testable without display access)
pub mod state;

/// GTK4/Adwaita UI components
pub mod ui;

/// Reverse-DNS application identifier (required by GTK)
pub const APP_ID: &str = "uk.co.mwbm.MeedyaManager";

/// Run the GTK application. Blocks until the application exits.
pub fn run_app() {
    app::run(APP_ID);
}
