// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — GTK4/Adwaita desktop binary entry point
//
// This file is intentionally minimal — all application logic lives in the
// library crate (`mm_gtk`) so it can be unit-tested without a live display.

fn main() {
    // Delegate to the library's run_app() function which sets up GTK,
    // builds the main window, and starts the GLib event loop.
    mm_gtk::run_app();
}
