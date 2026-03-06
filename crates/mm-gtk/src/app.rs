// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — GTK Application Setup
//
// Initialises libadwaita, creates the GApplication, registers the activate
// handler, and starts the GLib event loop.  The activate handler builds
// the main window via the `ui` module.

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::ui::main_window;

/// Initialise libadwaita and run the GTK event loop.
///
/// Blocks until the application window is closed.
pub fn run(app_id: &str) {
    // Initialise i18n — must run before any user-visible strings are produced.
    // Resolves the system locale and binds the "meedyamanager" gettext domain.
    mm_core::i18n::init();

    // Initialise libadwaita (also initialises GTK4 and GLib internally)
    // This must be called before any GTK/Adwaita object is created.
    adw::init().expect("Failed to initialise libadwaita — is GTK4 installed?");

    // Create the application instance with our reverse-DNS identifier.
    // GTK uses the application ID for D-Bus session uniqueness checks.
    let app = adw::Application::builder()
        .application_id(app_id)
        .build();

    // Register the activate handler — called on first launch and when a
    // second instance tries to start (GTK raises the existing window instead)
    app.connect_activate(on_activate);

    // Start the GLib main event loop; blocks until all windows are closed
    // The return value is the process exit code (ignored here)
    let _exit_code = app.run();
}

/// Activate handler — builds and presents the main application window.
///
/// GTK calls this function when the application is ready to display UI.
fn on_activate(app: &adw::Application) {
    // Build the main window via the UI module
    let window = main_window::build(app);

    // Present the window — makes it visible and raises it if it already exists
    window.present();
}
