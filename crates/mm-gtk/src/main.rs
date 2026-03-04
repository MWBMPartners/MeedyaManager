// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — GTK4/Adwaita Desktop GUI
//
// Entry point for the `meedya-gtk` binary. This provides a native Linux
// desktop interface using GTK4 and libadwaita, following GNOME Human
// Interface Guidelines for a modern, adaptive user experience.

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

/// Application ID following reverse-DNS convention.
const APP_ID: &str = "uk.co.mwbm.MeedyaManager";

/// Application entry point.
///
/// Initialises GTK4 and libadwaita, creates the main application window,
/// and starts the GTK event loop.
fn main() {
    // Initialise libadwaita (also initialises GTK4 internally)
    adw::init().expect("Failed to initialise libadwaita");

    // Create the Adwaita application instance
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    // Connect the activate signal to build the main window
    app.connect_activate(build_ui);

    // Run the GTK event loop (blocks until the application exits)
    app.run();
}

/// Builds the main application window with an Adwaita header bar.
///
/// This is called when the application is activated (either on first launch
/// or when a second instance signals the running one).
fn build_ui(app: &adw::Application) {
    // Create the main content label (placeholder for the real UI)
    let content = gtk::Label::builder()
        .label("MeedyaManager")
        .css_classes(["title-1"])
        .vexpand(true)
        .hexpand(true)
        .build();

    // Create an Adwaita toolbar view with a header bar
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&adw::HeaderBar::new());
    toolbar_view.set_content(Some(&content));

    // Create the main application window
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("MeedyaManager")
        .default_width(1024)
        .default_height(768)
        .content(&toolbar_view)
        .build();

    // Present the window to the user
    window.present();
}
