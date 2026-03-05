// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Rules Panel (M4 stub)
//
// The full rule builder (visual condition editor, live preview, rule ordering)
// is planned for M6.  This M4 stub provides:
//   - A template validator that gives instant feedback on syntax errors
//   - A tag reference panel listing all known tag names
//   - A live-preview text field to test a template against sample values
//
// The stub already wires up the rule engine parser from mm-core so the
// validate-on-keypress feature is fully functional.

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use mm_core::rule_engine;

/// The Rules tab panel.
pub struct RulesPanel {
    root: gtk::Box,
}

impl RulesPanel {
    /// Construct the rules panel widget tree.
    pub fn new() -> Self {
        // ------------------------------------------------------------------
        // Template validator section
        // ------------------------------------------------------------------

        let template_entry = gtk::Entry::builder()
            .placeholder_text("Enter a rename template, e.g. <Artist> - <Title>")
            .hexpand(true)
            .build();

        // Validation status label — updated live on every keystroke
        let validation_label = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();

        // Validate on every keystroke
        {
            let vl = validation_label.clone();
            template_entry.connect_changed(move |entry| {
                let text = entry.text();
                if text.trim().is_empty() {
                    vl.set_text("");
                    vl.remove_css_class("error");
                    vl.remove_css_class("success");
                    return;
                }

                match rule_engine::parse_template(text.as_str()) {
                    Ok(_) => {
                        vl.set_text("✓ Valid template");
                        vl.remove_css_class("error");
                        vl.add_css_class("success");
                    }
                    Err(e) => {
                        vl.set_text(&format!("✗ {e}"));
                        vl.remove_css_class("success");
                        vl.add_css_class("error");
                    }
                }
            });
        }

        let validator_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(8)
            .build();

        validator_box.append(
            &gtk::Label::builder()
                .label("Template Validator")
                .halign(gtk::Align::Start)
                .css_classes(["heading"])
                .build(),
        );
        validator_box.append(&template_entry);
        validator_box.append(&validation_label);

        // ------------------------------------------------------------------
        // Tag reference panel
        // ------------------------------------------------------------------

        let tags_label = gtk::Label::builder()
            .label("Available Tags")
            .halign(gtk::Align::Start)
            .margin_start(12)
            .margin_top(12)
            .margin_bottom(4)
            .css_classes(["heading"])
            .build();

        // Build the tag list grid — 3 columns
        let tags_flow = gtk::FlowBox::builder()
            .homogeneous(false)
            .column_spacing(6)
            .row_spacing(4)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .selection_mode(gtk::SelectionMode::None)
            .build();

        // Known tags from the registry (mirroring uniffi_api::list_known_tags)
        let known_tags = [
            "Title", "Artist", "Album", "AlbumArtist", "Year", "Genre",
            "TrackNumber", "TrackTotal", "DiscNumber", "DiscTotal",
            "Composer", "Comment", "Lyrics", "ISRC", "Barcode",
            "CatalogNumber", "Label", "Compilation", "BPM",
            "Filename", "Extension", "Folder", "Duration",
            "BitrateKbps", "SampleRateHz", "MediaClass", "MediaFormat",
        ];

        for tag in &known_tags {
            // Each tag shown as a pill button that copies "<Tag>" into the template entry
            let pill = gtk::Button::builder()
                .label(&format!("<{tag}>"))
                .css_classes(["pill"])
                .build();

            let entry_clone = template_entry.clone();
            let tag_text = format!("<{tag}>");
            pill.connect_clicked(move |_| {
                // Insert the tag at the current cursor position.
                // position() returns i32; insert_text requires &mut i32.
                let mut pos = entry_clone.position();
                entry_clone.insert_text(&tag_text, &mut pos);
            });

            tags_flow.insert(&pill, -1);
        }

        // ------------------------------------------------------------------
        // M6 coming-soon notice
        // ------------------------------------------------------------------

        let notice = gtk::Label::builder()
            .label("🚧  Full visual rule builder coming in M6 (v2.0.0-beta.1)")
            .halign(gtk::Align::Center)
            .margin_top(24)
            .margin_bottom(24)
            .css_classes(["dim-label"])
            .build();

        // ------------------------------------------------------------------
        // Root layout
        // ------------------------------------------------------------------

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .build();

        let inner = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        inner.append(&validator_box);
        inner.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        inner.append(&tags_label);
        inner.append(&tags_flow);
        inner.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        inner.append(&notice);

        scrolled.set_child(Some(&inner));

        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        root.append(&scrolled);

        Self { root }
    }

    /// Return the root widget for placement in AdwTabView.
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }
}
