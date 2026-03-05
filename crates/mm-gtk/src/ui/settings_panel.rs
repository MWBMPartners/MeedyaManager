// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Settings Panel
//
// Displays the active configuration (loaded from settings.json5) and
// exposes the most important toggles as native GTK widgets.
//
// Layout:
//   ┌───────────────────────────────────────────────────────────┐
//   │  Config path: ~/.config/MeedyaManager/settings.json5     │
//   ├───────────────────────────────────────────────────────────┤
//   │  General                                                  │
//   │    Dry-run mode  [switch]                                 │
//   │                                                           │
//   │  Watching                                                 │
//   │    Recursive     [switch]                                 │
//   │    Debounce ms   [spin]                                   │
//   │                                                           │
//   │  Logging                                                  │
//   │    Level         [combo: trace/debug/info/warn/error]     │
//   │    Redact PII    [switch]                                 │
//   ├───────────────────────────────────────────────────────────┤
//   │  Raw JSON5 config (read-only)                             │
//   │  [scrolled text view]                                     │
//   └───────────────────────────────────────────────────────────┘

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use mm_core::config::AppConfig;

/// The Settings panel.
pub struct SettingsPanel {
    root: gtk::Box,
}

impl SettingsPanel {
    /// Construct the settings panel.
    pub fn new() -> Self {
        // Load the active configuration (or defaults if no file exists)
        let config = AppConfig::load().unwrap_or_default();

        // Config file path info bar
        let path_label = gtk::Label::builder()
            .label(&format!(
                "Config: {}",
                mm_config_path()
            ))
            .halign(gtk::Align::Start)
            .margin_start(12)
            .margin_top(12)
            .margin_bottom(8)
            .css_classes(["dim-label", "caption"])
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .build();

        // ------------------------------------------------------------------
        // General group
        // ------------------------------------------------------------------

        let dry_run_row = adw::SwitchRow::new();
        dry_run_row.set_title("Dry-run mode");
        dry_run_row.set_subtitle("Preview renames without moving any files");
        dry_run_row.set_active(config.dry_run);

        let general_group = adw::PreferencesGroup::new();
        general_group.set_title("General");
        general_group.add(&dry_run_row);

        // ------------------------------------------------------------------
        // Watch group
        // ------------------------------------------------------------------

        let recursive_row = adw::SwitchRow::new();
        recursive_row.set_title("Recursive watching");
        recursive_row.set_subtitle("Watch sub-directories for new files");
        recursive_row.set_active(config.watch.recursive);

        let debounce_adj = gtk::Adjustment::new(
            config.watch.debounce_ms as f64,
            50.0,   // min
            5000.0, // max
            50.0,   // step
            100.0,  // page step
            0.0,    // page size
        );
        let debounce_spin = gtk::SpinButton::new(Some(&debounce_adj), 50.0, 0);

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
        let current_level_idx = match config.logging.level.as_str() {
            "trace" => 0,
            "debug" => 1,
            "info"  => 2,
            "warn"  => 3,
            "error" => 4,
            _       => 2, // default to info
        };
        level_combo.set_selected(current_level_idx);

        let level_row = adw::ActionRow::new();
        level_row.set_title("Log level");
        level_row.set_subtitle("Verbosity of the application log");
        level_row.add_suffix(&level_combo);

        let pii_row = adw::SwitchRow::new();
        pii_row.set_title("Redact PII in logs");
        pii_row.set_subtitle("Replace file paths and personal data with hashes");
        pii_row.set_active(config.logging.redact_pii);

        let logging_group = adw::PreferencesGroup::new();
        logging_group.set_title("Logging");
        logging_group.add(&level_row);
        logging_group.add(&pii_row);

        // ------------------------------------------------------------------
        // Raw JSON5 view (read-only reference)
        // ------------------------------------------------------------------

        let json_label = gtk::Label::builder()
            .label("Raw Configuration (read-only)")
            .halign(gtk::Align::Start)
            .margin_start(12)
            .margin_top(12)
            .margin_bottom(4)
            .css_classes(["heading"])
            .build();

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
            .min_content_height(200)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .child(&text_view)
            .build();

        // ------------------------------------------------------------------
        // Save button (saves the toggle/spin values only — not raw JSON)
        // ------------------------------------------------------------------

        let save_btn = gtk::Button::builder()
            .label("Save Settings")
            .css_classes(["suggested-action"])
            .halign(gtk::Align::End)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .build();

        let status_label = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Start)
            .margin_start(12)
            .css_classes(["dim-label"])
            .build();

        save_btn.connect_clicked({
            let sl = status_label.clone();
            move |_| {
                // In M4, saving just shows a confirmation — full config write in M6
                sl.set_text("⚠ Full config saving coming in M6.");
            }
        });

        // ------------------------------------------------------------------
        // Root layout (scrolled page)
        // ------------------------------------------------------------------

        let prefs_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(4)
            .margin_bottom(4)
            .build();

        prefs_content.append(&general_group);
        prefs_content.append(&watch_group);
        prefs_content.append(&logging_group);

        let outer = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        outer.append(&path_label);
        outer.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        let scrolled_prefs = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&prefs_content)
            .build();

        outer.append(&scrolled_prefs);
        outer.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        outer.append(&json_label);
        outer.append(&json_scrolled);
        outer.append(&status_label);
        outer.append(&save_btn);

        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        // Wrap the whole settings page inside a clamp for comfortable reading width
        let clamp = adw::Clamp::builder()
            .maximum_size(900)
            .build();
        clamp.set_child(Some(&outer));

        root.append(&clamp);

        Self { root }
    }

    /// Return the root widget for placement in AdwTabView.
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }
}

/// Return the platform config path for display.
fn mm_config_path() -> String {
    dirs::config_dir()
        .map(|d| d.join("MeedyaManager").join("settings.json5"))
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}
