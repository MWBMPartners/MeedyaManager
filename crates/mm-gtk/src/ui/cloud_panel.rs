// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Cloud Storage Monitor Panel (GTK4 / Adwaita, M7)
//
// Displays connected cloud providers, their sync status, and buttons to
// connect or disconnect each provider.
//
// Layout:
//   AdwPreferencesGroup "Cloud Providers"
//     AdwActionRow per provider (name | status | Connect/Disconnect button)
//   Separator
//   GtkScrolledWindow  →  GtkTextView (event log)
//   GtkButton "Clear Log"

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use std::cell::RefCell;
use std::rc::Rc;

// ---------------------------------------------------------------------------
// Provider descriptor
// ---------------------------------------------------------------------------

/// Static metadata for one cloud provider row.
struct ProviderInfo {
    /// Internal identifier (matches mm-cloud provider name).
    id:      &'static str,
    /// Display name shown in the UI.
    label:   &'static str,
    /// `true` for providers not yet implemented (MEGA, iCloud).
    is_stub: bool,
}

/// The ordered list of cloud providers.
const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo { id: "onedrive",    label: "OneDrive",     is_stub: false },
    ProviderInfo { id: "googledrive", label: "Google Drive", is_stub: false },
    ProviderInfo { id: "dropbox",     label: "Dropbox",      is_stub: false },
    ProviderInfo { id: "mega",        label: "MEGA",         is_stub: true  },
    ProviderInfo { id: "icloud",      label: "iCloud Drive", is_stub: true  },
];

// ---------------------------------------------------------------------------
// CloudPanel
// ---------------------------------------------------------------------------

/// GTK4 Cloud Storage Monitor panel.
pub struct CloudPanel {
    /// Root widget placed in the AdwTabView.
    root: gtk::Box,
}

impl CloudPanel {
    /// Builds the Cloud panel and returns a new `CloudPanel` instance.
    pub fn new() -> Self {
        // ── Root vertical box ──────────────────────────────────────────────
        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .build();

        // ── Title bar ──────────────────────────────────────────────────────
        let title_label = gtk::Label::builder()
            .label("<b>Cloud Storage Monitor</b>")
            .use_markup(true)
            .xalign(0.0)
            .margin_start(12)
            .margin_top(12)
            .margin_bottom(8)
            .build();
        root.append(&title_label);

        // ── Provider preference group ──────────────────────────────────────
        let prefs_group = adw::PreferencesGroup::builder()
            .title("Connected Providers")
            .description("Connect your cloud storage to monitor media files automatically.")
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(8)
            .build();

        // Shared event log buffer (GtkTextBuffer is reference-counted).
        let log_buf = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);

        // Build one row per provider.
        for info in PROVIDERS {
            let row = build_provider_row(info, log_buf.clone());
            prefs_group.add(&row);
        }

        root.append(&prefs_group);

        // ── Separator ─────────────────────────────────────────────────────
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // ── Event log ─────────────────────────────────────────────────────
        let log_label = gtk::Label::builder()
            .label("<b>Event Log</b>")
            .use_markup(true)
            .xalign(0.0)
            .margin_start(12)
            .margin_top(8)
            .margin_bottom(4)
            .build();
        root.append(&log_label);

        // Read-only text view for event entries.
        let log_view = gtk::TextView::builder()
            .buffer(&log_buf)
            .editable(false)
            .cursor_visible(false)
            .wrap_mode(gtk::WrapMode::Word)
            .monospace(true)
            .margin_start(12)
            .margin_end(12)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .child(&log_view)
            .vexpand(true)
            .margin_start(0)
            .margin_end(0)
            .margin_bottom(8)
            .build();
        root.append(&scrolled);

        // "Clear Log" button
        let clear_btn = gtk::Button::builder()
            .label("Clear Log")
            .halign(gtk::Align::End)
            .margin_end(12)
            .margin_bottom(12)
            .build();

        {
            let lb = log_buf.clone();
            clear_btn.connect_clicked(move |_| {
                lb.set_text("");
            });
        }
        root.append(&clear_btn);

        Self { root }
    }

    /// Returns the root widget to be placed in the `AdwTabView`.
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }
}

// ---------------------------------------------------------------------------
// Helper: build one provider row
// ---------------------------------------------------------------------------

/// Creates an `AdwActionRow` for a cloud provider with connect/disconnect button.
fn build_provider_row(info: &ProviderInfo, log_buf: gtk::TextBuffer) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(info.label)
        .subtitle(if info.is_stub { "Coming Soon" } else { "Not Connected" })
        .build();

    if info.is_stub {
        // Stub providers show a disabled label instead of a button.
        let stub_label = gtk::Label::builder()
            .label("—")
            .sensitive(false)
            .build();
        row.add_suffix(&stub_label);
        return row;
    }

    // Connect / Disconnect toggle button.
    let btn = gtk::Button::builder()
        .label("Connect")
        .valign(gtk::Align::Center)
        .css_classes(["suggested-action"])
        .build();

    let provider_label = info.label.to_string();
    let row_clone      = row.clone();
    let connected      = Rc::new(RefCell::new(false));

    btn.connect_clicked(move |b| {
        let mut is_connected = connected.borrow_mut();
        *is_connected = !*is_connected;

        if *is_connected {
            // Mark as connected
            row_clone.set_subtitle("Syncing…");
            b.set_label("Disconnect");
            b.remove_css_class("suggested-action");
            b.add_css_class("destructive-action");
            append_log(&log_buf, &format!("[{}] Connecting…", provider_label));
            // Simulate successful sync (in production this calls mm-cloud FFI)
            append_log(&log_buf, &format!("[{}] Connected — watching /Music", provider_label));
            row_clone.set_subtitle("Synced");
        } else {
            // Mark as disconnected
            row_clone.set_subtitle("Not Connected");
            b.set_label("Connect");
            b.remove_css_class("destructive-action");
            b.add_css_class("suggested-action");
            append_log(&log_buf, &format!("[{}] Disconnected", provider_label));
        }
    });

    row.add_suffix(&btn);
    row
}

/// Appends a timestamped message to the shared event log text buffer.
fn append_log(buf: &gtk::TextBuffer, msg: &str) {
    let mut end = buf.end_iter();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    let line = format!("[{h:02}:{m:02}:{s:02}] {msg}\n");
    buf.insert(&mut end, &line);
}

// ---------------------------------------------------------------------------
// Unit tests (pure logic, no GTK initialisation required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_list_has_five_entries() {
        assert_eq!(PROVIDERS.len(), 5);
    }

    #[test]
    fn first_three_providers_are_not_stubs() {
        let non_stubs: Vec<_> = PROVIDERS.iter().filter(|p| !p.is_stub).collect();
        assert_eq!(non_stubs.len(), 3);
    }

    #[test]
    fn last_two_providers_are_stubs() {
        let stubs: Vec<_> = PROVIDERS.iter().filter(|p| p.is_stub).collect();
        assert_eq!(stubs.len(), 2);
    }

    #[test]
    fn provider_ids_are_unique() {
        let ids: Vec<_> = PROVIDERS.iter().map(|p| p.id).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn onedrive_is_first() {
        assert_eq!(PROVIDERS[0].id, "onedrive");
    }

    #[test]
    fn mega_is_stub() {
        assert!(PROVIDERS.iter().find(|p| p.id == "mega").unwrap().is_stub);
    }

    #[test]
    fn icloud_is_stub() {
        assert!(PROVIDERS.iter().find(|p| p.id == "icloud").unwrap().is_stub);
    }
}
