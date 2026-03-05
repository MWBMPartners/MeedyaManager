// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Error Dialog Helper (M6)
//
// Provides a reusable function for showing user-friendly error dialogs
// using `adw::AlertDialog` (libadwaita 1.5+).
//
// Usage:
//   show_error(window, "Save Failed", "The file could not be written. Check permissions.");

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

/// Show a modal error dialog attached to `parent`.
///
/// - `heading` — short title (e.g. "Save Failed")
/// - `body`    — full detail message shown below the heading
///
/// The dialog has a single "OK" close button.  It is presented non-blocking;
/// the GTK main loop continues to run while the dialog is open.
pub fn show_error(parent: &impl IsA<gtk::Window>, heading: &str, body: &str) {
    let dialog = adw::AlertDialog::new(Some(heading), Some(body));
    // Add a single close button
    dialog.add_response("close", "OK");
    dialog.set_default_response(Some("close"));
    dialog.set_close_response("close");
    dialog.present(Some(parent));
}

/// Show an informational (non-error) dialog with a single "OK" button.
pub fn show_info(parent: &impl IsA<gtk::Window>, heading: &str, body: &str) {
    let dialog = adw::AlertDialog::new(Some(heading), Some(body));
    dialog.add_response("close", "OK");
    dialog.set_default_response(Some("close"));
    dialog.set_close_response("close");
    dialog.present(Some(parent));
}

/// Build a confirmation dialog with "Cancel" and a labelled confirm button.
///
/// Returns the dialog; the caller connects `dialog.connect_response` to
/// check whether the user clicked the confirm response ID `"confirm"`.
pub fn build_confirm_dialog(
    heading: &str,
    body: &str,
    confirm_label: &str,
    destructive: bool,
) -> adw::AlertDialog {
    let dialog = adw::AlertDialog::new(Some(heading), Some(body));
    dialog.add_response("cancel",  "Cancel");
    dialog.add_response("confirm", confirm_label);
    dialog.set_default_response(Some("cancel"));
    dialog.set_close_response("cancel");

    // Mark destructive actions (e.g. delete, overwrite) with the red button appearance
    if destructive {
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
    } else {
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);
    }

    dialog
}
