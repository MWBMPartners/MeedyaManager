// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Settings Panel (M6 — Full Config Save + Test Mode)
//
// Displays the active configuration (loaded from settings.json5) and
// exposes the most important toggles as native GTK/Adwaita widgets.
// Saving writes the modified JSON5 back to disk.
//
// Layout:
//   ┌────────────────────────────────────────────────────────────┐
//   │  Config path: ~/.config/MeedyaManager/settings.json5       │
//   │  [Open Folder]  [Copy Path]                                │
//   ├────────────────────────────────────────────────────────────┤
//   │  General                                                   │
//   │    Dry-run mode          [switch]                          │
//   │  File Watching                                             │
//   │    Recursive watching    [switch]                          │
//   │    Debounce (ms)         [spin]                            │
//   │  Logging                                                   │
//   │    Log level             [dropdown]                        │
//   │    Redact PII            [switch]                          │
//   │  Test Mode (Safe Edit Mode)                                │
//   │    Enable test mode      [switch]                          │
//   │    Tracked files: N                                        │
//   │    [Commit Changes]  [Revert Changes]                      │
//   │  About                                                     │
//   │    [Privacy Policy]                                        │
//   ├────────────────────────────────────────────────────────────┤
//   │  Raw Configuration (read-only reference)                   │
//   │  [text view]                                               │
//   ├────────────────────────────────────────────────────────────┤
//   │  [status label]              [Save Settings]               │
//   └────────────────────────────────────────────────────────────┘

use std::cell::RefCell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use mm_core::config::AppConfig;
use mm_core::test_mode;

use crate::state::SettingsSnapshot;
use crate::ui::accessibility;

/// The Settings panel.
pub struct SettingsPanel {
    root: gtk::Box,
}

impl SettingsPanel {
    /// Construct the settings panel.
    pub fn new() -> Self {
        // Load the active configuration (or defaults if no file exists)
        let config = AppConfig::load().unwrap_or_default();

        // Build a snapshot of the editable fields from the loaded config
        let snapshot = Rc::new(RefCell::new(SettingsSnapshot {
            dry_run:     config.dry_run,
            recursive:   config.watch.recursive,
            debounce_ms: config.watch.debounce_ms,
            log_level:   config.logging.level.clone(),
            redact_pii:  config.logging.redact_pii,
        }));

        // ------------------------------------------------------------------
        // Config path bar + action buttons
        // ------------------------------------------------------------------

        let config_path = mm_config_path();
        let config_dir  = mm_config_dir();

        let path_label = gtk::Label::builder()
            .label(&format!("Config: {config_path}"))
            .halign(gtk::Align::Start)
            .margin_start(12)
            .margin_top(8)
            .margin_bottom(4)
            .css_classes(["dim-label", "caption"])
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .hexpand(true)
            .build();

        // "Open Folder" button — opens the config directory in the file manager
        let open_folder_btn = gtk::Button::builder()
            .label("Open Folder")
            .margin_end(4)
            .build();
        accessibility::set_label(&open_folder_btn, "Open config folder");
        accessibility::set_description(&open_folder_btn, "Opens the folder containing the settings file in the file manager.");
        {
            let dir = config_dir.clone();
            open_folder_btn.connect_clicked(move |_| {
                let _ = gtk::gio::AppInfo::launch_default_for_uri(
                    &format!("file://{dir}"),
                    gtk::gio::AppLaunchContext::NONE,
                );
            });
        }

        // "Copy Path" button — copies the config path to clipboard
        let copy_path_btn = gtk::Button::builder()
            .label("Copy Path")
            .build();
        accessibility::set_label(&copy_path_btn, "Copy config file path");
        accessibility::set_description(&copy_path_btn, "Copies the settings file path to the clipboard.");
        {
            let path = config_path.clone();
            copy_path_btn.connect_clicked(move |btn| {
                if let Some(display) = gtk::gdk::Display::default() {
                    display.clipboard().set_text(&path);
                    // Brief button text change to confirm copy
                    btn.set_label("Copied!");
                    let btn_c = btn.clone();
                    gtk::glib::timeout_add_seconds_local(2, move || {
                        btn_c.set_label("Copy Path");
                        gtk::glib::ControlFlow::Break
                    });
                }
            });
        }

        let path_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(4)
            .margin_start(12)
            .margin_end(12)
            .margin_top(8)
            .margin_bottom(8)
            .build();
        path_row.append(&path_label);
        path_row.append(&open_folder_btn);
        path_row.append(&copy_path_btn);

        // ------------------------------------------------------------------
        // General group
        // ------------------------------------------------------------------

        let dry_run_row = adw::SwitchRow::new();
        dry_run_row.set_title("Dry-run mode");
        dry_run_row.set_subtitle("Preview renames without moving any files");
        dry_run_row.set_active(snapshot.borrow().dry_run);

        {
            let snap = Rc::clone(&snapshot);
            dry_run_row.connect_active_notify(move |r| {
                snap.borrow_mut().dry_run = r.is_active();
            });
        }

        let general_group = adw::PreferencesGroup::new();
        general_group.set_title("General");
        general_group.add(&dry_run_row);

        // ------------------------------------------------------------------
        // Watch group
        // ------------------------------------------------------------------

        let recursive_row = adw::SwitchRow::new();
        recursive_row.set_title("Recursive watching");
        recursive_row.set_subtitle("Watch sub-directories for new files");
        recursive_row.set_active(snapshot.borrow().recursive);

        {
            let snap = Rc::clone(&snapshot);
            recursive_row.connect_active_notify(move |r| {
                snap.borrow_mut().recursive = r.is_active();
            });
        }

        let debounce_adj = gtk::Adjustment::new(
            snapshot.borrow().debounce_ms as f64,
            50.0, 5000.0, 50.0, 100.0, 0.0,
        );
        let debounce_spin = gtk::SpinButton::new(Some(&debounce_adj), 50.0, 0);
        accessibility::set_label(&debounce_spin, "Debounce interval in milliseconds");
        accessibility::set_description(&debounce_spin, "Milliseconds to wait before processing a file-system event. Range: 50 to 5000.");

        {
            let snap = Rc::clone(&snapshot);
            debounce_spin.connect_value_changed(move |spin| {
                snap.borrow_mut().debounce_ms = spin.value() as u64;
            });
        }

        let debounce_row = adw::ActionRow::new();
        debounce_row.set_title("Debounce (ms)");
        debounce_row.set_subtitle("Coalesce rapid events into a single notification");
        debounce_row.add_suffix(&debounce_spin);
        debounce_row.set_activatable_widget(Some(&debounce_spin));

        let watch_group = adw::PreferencesGroup::new();
        watch_group.set_title("File Watching");
        watch_group.add(&recursive_row);
        watch_group.add(&debounce_row);

        // ------------------------------------------------------------------
        // Logging group
        // ------------------------------------------------------------------

        let level_combo = gtk::DropDown::from_strings(&["trace", "debug", "info", "warn", "error"]);
        accessibility::set_label(&level_combo, "Log level");
        accessibility::set_description(&level_combo, "Controls the verbosity of structured log output.");
        let current_level_idx = match snapshot.borrow().log_level.as_str() {
            "trace" => 0u32,
            "debug" => 1,
            "info"  => 2,
            "warn"  => 3,
            "error" => 4,
            _       => 2,
        };
        level_combo.set_selected(current_level_idx);

        {
            let snap = Rc::clone(&snapshot);
            level_combo.connect_selected_notify(move |combo| {
                let level = match combo.selected() {
                    0 => "trace",
                    1 => "debug",
                    2 => "info",
                    3 => "warn",
                    4 => "error",
                    _ => "info",
                };
                snap.borrow_mut().log_level = level.into();
            });
        }

        let level_row = adw::ActionRow::new();
        level_row.set_title("Log level");
        level_row.set_subtitle("Verbosity of the application log");
        level_row.add_suffix(&level_combo);

        let pii_row = adw::SwitchRow::new();
        pii_row.set_title("Redact PII in logs");
        pii_row.set_subtitle("Replace file paths and personal data with hashes");
        pii_row.set_active(snapshot.borrow().redact_pii);

        {
            let snap = Rc::clone(&snapshot);
            pii_row.connect_active_notify(move |r| {
                snap.borrow_mut().redact_pii = r.is_active();
            });
        }

        let logging_group = adw::PreferencesGroup::new();
        logging_group.set_title("Logging");
        logging_group.add(&level_row);
        logging_group.add(&pii_row);

        // ------------------------------------------------------------------
        // Test Mode (Safe Edit Mode) group
        // ------------------------------------------------------------------

        // Read the current test-mode state from the persistent manifest
        let test_mode_enabled = test_mode::is_enabled();
        let tracked_count     = test_mode::tracked_file_count();

        // Toggle row — enables/disables test mode globally
        let test_mode_row = adw::SwitchRow::new();
        test_mode_row.set_title("Test Mode (Safe Edit Mode)");
        test_mode_row.set_subtitle(
            "Edits create copies with a _MeedyaManager suffix instead of \
             overwriting originals. Tracked files are recorded in a manifest \
             so they survive restarts."
        );
        test_mode_row.set_active(test_mode_enabled);

        // Label showing the number of files tracked in the manifest
        let tracked_label = gtk::Label::builder()
            .label(&format!("Tracked files: {tracked_count}"))
            .halign(gtk::Align::Start)
            .margin_start(16)
            .margin_top(4)
            .margin_bottom(4)
            .css_classes(["dim-label"])
            .build();

        // Commit button — deletes originals and renames copies to originals
        let commit_btn = gtk::Button::builder()
            .label("Commit Changes")
            .css_classes(["suggested-action"])
            .sensitive(tracked_count > 0)
            .tooltip_text("Delete originals, keep the tagged copies as the new files")
            .build();
        accessibility::set_label(&commit_btn, "Commit test mode changes");
        accessibility::set_description(
            &commit_btn,
            "Replaces original files with their tagged copies and clears the manifest.",
        );

        // Revert button — clears the manifest but keeps both originals and copies
        let revert_btn = gtk::Button::builder()
            .label("Revert Changes")
            .css_classes(["destructive-action"])
            .sensitive(tracked_count > 0)
            .tooltip_text("Keep both originals and copies, clear the manifest")
            .build();
        accessibility::set_label(&revert_btn, "Revert test mode changes");
        accessibility::set_description(
            &revert_btn,
            "Clears the test-mode manifest without deleting any files.",
        );

        // Wire up the Commit button
        {
            let tl = tracked_label.clone();
            let cb = commit_btn.clone();
            let rb = revert_btn.clone();
            commit_btn.connect_clicked(move |_| {
                match test_mode::commit_files() {
                    Ok(n) => {
                        // Update UI to reflect cleared manifest
                        tl.set_label("Tracked files: 0");
                        cb.set_sensitive(false);
                        rb.set_sensitive(false);
                        tl.set_label(&format!("Committed {n} file(s). Tracked files: 0"));
                    }
                    Err(e) => {
                        tl.set_label(&format!("Commit error: {e}"));
                    }
                }
            });
        }

        // Wire up the Revert button
        {
            let tl = tracked_label.clone();
            let cb = commit_btn.clone();
            let rb = revert_btn.clone();
            revert_btn.connect_clicked(move |_| {
                match test_mode::revert_files() {
                    Ok(()) => {
                        tl.set_label("Tracked files: 0 (reverted)");
                        cb.set_sensitive(false);
                        rb.set_sensitive(false);
                    }
                    Err(e) => {
                        tl.set_label(&format!("Revert error: {e}"));
                    }
                }
            });
        }

        // Wire up the Test Mode toggle switch.
        // When toggling OFF with tracked files, show a confirmation dialog.
        {
            let tl = tracked_label.clone();
            let cb = commit_btn.clone();
            let rb = revert_btn.clone();
            test_mode_row.connect_active_notify(move |row| {
                let is_active = row.is_active();

                if is_active {
                    // --- Turning ON ---
                    if let Err(e) = test_mode::enable() {
                        tl.set_label(&format!("Enable error: {e}"));
                        return;
                    }
                    tl.set_label(&format!(
                        "Tracked files: {}",
                        test_mode::tracked_file_count()
                    ));
                } else {
                    // --- Turning OFF ---
                    let count = test_mode::tracked_file_count();

                    if count > 0 {
                        // Show a confirmation dialog asking the user whether
                        // to commit (keep tagged copies) or keep both files
                        let dialog = adw::AlertDialog::new(
                            Some("Keep only the tagged files?"),
                            Some(&format!(
                                "Test Mode has {count} tracked file(s).\n\n\
                                 Yes = delete originals, rename copies (commit)\n\
                                 No  = keep both originals and copies (revert)"
                            )),
                        );
                        dialog.add_response("no",  "No");
                        dialog.add_response("yes", "Yes");
                        dialog.set_default_response(Some("no"));
                        dialog.set_close_response("no");
                        dialog.set_response_appearance(
                            "yes",
                            adw::ResponseAppearance::Destructive,
                        );

                        // Clone references for the async response handler
                        let tl2 = tl.clone();
                        let cb2 = cb.clone();
                        let rb2 = rb.clone();

                        dialog.connect_response(None, move |_dlg, response| {
                            match response {
                                "yes" => {
                                    // Commit: delete originals, rename copies
                                    match test_mode::commit_files() {
                                        Ok(n) => {
                                            tl2.set_label(&format!(
                                                "Committed {n} file(s). Tracked files: 0"
                                            ));
                                        }
                                        Err(e) => {
                                            tl2.set_label(&format!("Commit error: {e}"));
                                        }
                                    }
                                }
                                _ => {
                                    // No / Cancel: revert — keep both, clear manifest
                                    let _ = test_mode::revert_files();
                                    tl2.set_label("Tracked files: 0 (reverted)");
                                }
                            }
                            // Disable the action buttons (manifest is now empty)
                            cb2.set_sensitive(false);
                            rb2.set_sensitive(false);
                        });

                        // Present the dialog. We need a parent window for proper
                        // modal behaviour; walk up the widget tree to find it.
                        let parent = row
                            .root()
                            .and_then(|r| r.downcast::<gtk::Window>().ok());
                        dialog.present(parent.as_ref());
                    }

                    // Disable test mode in the manifest regardless of dialog
                    if let Err(e) = test_mode::disable() {
                        tl.set_label(&format!("Disable error: {e}"));
                    }
                }
            });
        }

        // Button box for Commit / Revert (horizontal, end-aligned)
        let test_mode_btn_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::End)
            .margin_end(16)
            .margin_bottom(8)
            .build();
        test_mode_btn_box.append(&commit_btn);
        test_mode_btn_box.append(&revert_btn);

        // Wrap the extra widgets (tracked label + buttons) in an ActionRow
        // so they sit inside the PreferencesGroup styling
        let test_mode_info_row = adw::ActionRow::new();
        test_mode_info_row.set_title("Manifest");
        test_mode_info_row.set_subtitle("Files tracked by the test-mode manifest");
        test_mode_info_row.add_suffix(&tracked_label);

        let test_mode_group = adw::PreferencesGroup::new();
        test_mode_group.set_title("Test Mode (Safe Edit Mode)");
        test_mode_group.set_description(Some(
            "When enabled, file edits create copies with a _MeedyaManager suffix. \
             Use Commit to replace originals or Revert to keep both."
        ));
        test_mode_group.add(&test_mode_row);
        test_mode_group.add(&test_mode_info_row);

        // ------------------------------------------------------------------
        // About / Legal group — Privacy Policy link
        // ------------------------------------------------------------------

        // Privacy Policy row — opens the privacy policy URL in the default browser
        let privacy_row = adw::ActionRow::new();
        privacy_row.set_title("Privacy Policy");
        privacy_row.set_subtitle("View the MeedyaManager privacy policy");
        privacy_row.set_activatable(true);

        // Arrow icon to indicate it opens externally
        let arrow_icon = gtk::Image::from_icon_name("go-next-symbolic");
        privacy_row.add_suffix(&arrow_icon);

        // Open the privacy policy page in the default web browser on activation
        privacy_row.connect_activated(|_row| {
            let _ = gtk::gio::AppInfo::launch_default_for_uri(
                "https://github.com/MWBMPartners/MeedyaManager/wiki/Privacy-Policy",
                gtk::gio::AppLaunchContext::NONE,
            );
        });

        let about_group = adw::PreferencesGroup::new();
        about_group.set_title("About");
        about_group.add(&privacy_row);

        // ------------------------------------------------------------------
        // Raw JSON5 view (read-only reference)
        // ------------------------------------------------------------------

        let raw_json = serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string());

        let text_buffer = gtk::TextBuffer::new(None);
        text_buffer.set_text(&raw_json);

        let text_view = gtk::TextView::builder()
            .buffer(&text_buffer)
            .editable(false)
            .monospace(true)
            .left_margin(8)
            .right_margin(8)
            .top_margin(8)
            .bottom_margin(8)
            .build();

        let json_scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .min_content_height(160)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(8)
            .child(&text_view)
            .build();

        // ------------------------------------------------------------------
        // Status label + Save button
        // ------------------------------------------------------------------

        let status_label = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Start)
            .hexpand(true)
            .margin_start(12)
            .css_classes(["dim-label"])
            .build();

        let save_btn = gtk::Button::builder()
            .label("Save Settings")
            .css_classes(["suggested-action"])
            .halign(gtk::Align::End)
            .margin_end(12)
            .margin_bottom(12)
            .build();
        accessibility::set_label(&save_btn, "Save settings");
        accessibility::set_description(&save_btn, "Writes the current settings to the config file on disk.");

        {
            let snap     = Rc::clone(&snapshot);
            let sl       = status_label.clone();
            let buf      = text_buffer.clone();
            let path_str = config_path.clone();

            save_btn.connect_clicked(move |_| {
                // Load current config and apply snapshot values
                let mut cfg = AppConfig::load().unwrap_or_default();
                {
                    let s = snap.borrow();
                    cfg.dry_run          = s.dry_run;
                    cfg.watch.recursive  = s.recursive;
                    cfg.watch.debounce_ms = s.debounce_ms;
                    cfg.logging.level    = s.log_level.clone();
                    cfg.logging.redact_pii = s.redact_pii;
                }

                // Serialise to JSON5 and write
                match save_config(&cfg, &path_str) {
                    Ok(new_json) => {
                        sl.set_text("✓ Settings saved.");
                        buf.set_text(&new_json);
                    }
                    Err(e) => {
                        sl.set_text(&format!("⚠ Save failed: {e}"));
                    }
                }
            });
        }

        let btn_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .margin_bottom(12)
            .build();
        btn_row.append(&status_label);
        btn_row.append(&save_btn);

        // ------------------------------------------------------------------
        // Root layout (scrolled prefs + raw json + button row)
        // ------------------------------------------------------------------

        let prefs_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        prefs_content.append(&general_group);
        prefs_content.append(&watch_group);
        prefs_content.append(&logging_group);
        prefs_content.append(&test_mode_group);
        prefs_content.append(&test_mode_btn_box);
        prefs_content.append(&about_group);

        let scrolled_prefs = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&prefs_content)
            .build();

        let outer = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        outer.append(&path_row);
        outer.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        outer.append(&scrolled_prefs);
        outer.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        outer.append(
            &gtk::Label::builder()
                .label("Raw Configuration (read-only)")
                .halign(gtk::Align::Start)
                .margin_start(12)
                .margin_top(8)
                .margin_bottom(4)
                .css_classes(["heading"])
                .build(),
        );
        outer.append(&json_scrolled);
        outer.append(&btn_row);

        // Wrap in a clamp for comfortable reading width
        let clamp = adw::Clamp::builder().maximum_size(960).build();
        clamp.set_child(Some(&outer));

        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        root.append(&clamp);

        Self { root }
    }

    /// Return the root widget for placement in AdwTabView.
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Return the platform config file path as a displayable string.
fn mm_config_path() -> String {
    dirs::config_dir()
        .map(|d| d.join("MeedyaManager").join("settings.json5"))
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// Return the platform config directory as a string.
fn mm_config_dir() -> String {
    dirs::config_dir()
        .map(|d| d.join("MeedyaManager"))
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// Serialise `cfg` to pretty JSON and write it to the config file path.
///
/// Creates the parent directory if it does not exist.
/// Returns the serialised JSON string on success.
fn save_config(cfg: &AppConfig, path_str: &str) -> Result<String, String> {
    let json = serde_json::to_string_pretty(cfg)
        .map_err(|e| format!("serialise error: {e}"))?;

    let path = std::path::Path::new(path_str);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create dir: {e}"))?;
    }

    std::fs::write(path, &json)
        .map_err(|e| format!("write error: {e}"))?;

    Ok(json)
}
