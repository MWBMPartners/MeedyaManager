// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — GTK Application Setup
//
// Initialises libadwaita, creates the GApplication, registers the activate
// handler, and starts the GLib event loop.  The activate handler builds
// the main window via the `ui` module.
//
// Pre-release safety:
//   If the compiled version has a semver pre-release label (e.g. "1.3.0-beta.1"),
//   the app auto-enables Test Mode on first launch of that version and shows a
//   one-time informational dialog.  This protects user files from potential bugs
//   in unreleased code.

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use mm_core::test_mode;

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
/// After presenting the window, checks for pre-release builds and shows
/// a one-time informational dialog if Test Mode was auto-enabled.
fn on_activate(app: &adw::Application) {
    // Build the main window via the UI module
    let window = main_window::build(app);

    // Present the window — makes it visible and raises it if it already exists
    window.present();

    // --- Pre-release safety check ---
    // If the compiled version has a semver pre-release label (e.g. "-beta.1",
    // "-alpha", "-rc.1"), auto-enable Test Mode so that file-mutating
    // operations create safe copies instead of overwriting originals.
    //
    // The dialog is shown only once per pre-release version: we store a
    // sentinel file `<config_dir>/meedyamanager/prerelease_notified_<version>`
    // to remember that the user has already been notified.
    if test_mode::is_current_prerelease() {
        // Attempt to auto-enable test mode (no-op if already enabled)
        let _ = test_mode::enable();

        // Check whether we have already shown the notification for this version
        let version = env!("CARGO_PKG_VERSION");
        let already_notified = prerelease_sentinel_exists(version);

        if !already_notified {
            // Mark that we have notified the user for this version
            write_prerelease_sentinel(version);

            // Show an informational dialog explaining the auto-enabled test mode
            let dialog = adw::AlertDialog::new(
                Some("Pre-release Build Detected"),
                Some(&format!(
                    "You are running MeedyaManager v{version}, which is a \
                     pre-release build.\n\n\
                     Test Mode has been automatically enabled to protect your \
                     files. All edits will create copies with a _MeedyaManager \
                     suffix instead of overwriting originals.\n\n\
                     You can disable Test Mode in Settings when you are ready."
                )),
            );
            // Single "OK" acknowledgement button
            dialog.add_response("ok", "OK");
            dialog.set_default_response(Some("ok"));
            dialog.set_close_response("ok");

            // Present the dialog modally on top of the main window
            dialog.present(Some(&window));
        }
    }
}

// ---------------------------------------------------------------------------
// Pre-release sentinel helpers
// ---------------------------------------------------------------------------

/// Check whether a sentinel file exists for the given pre-release version.
///
/// The sentinel is a zero-byte marker at:
///   `<config_dir>/meedyamanager/prerelease_notified_<version>`
///
/// Its presence means the user was already shown the pre-release dialog
/// for this particular version.
fn prerelease_sentinel_exists(version: &str) -> bool {
    sentinel_path(version)
        .map(|p| p.exists())
        .unwrap_or(false)
}

/// Write the sentinel file so that subsequent launches of the same
/// pre-release version do not show the dialog again.
fn write_prerelease_sentinel(version: &str) {
    if let Some(path) = sentinel_path(version) {
        // Ensure the parent directory exists
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Write a zero-byte marker file
        let _ = std::fs::write(&path, b"");
    }
}

/// Compute the sentinel file path for a given version string.
///
/// Returns `None` if the platform config directory cannot be determined.
fn sentinel_path(version: &str) -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| {
        d.join("meedyamanager")
            .join(format!("prerelease_notified_{version}"))
    })
}
