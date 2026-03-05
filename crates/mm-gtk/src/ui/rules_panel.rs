// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rules Panel (M6 — Full Implementation)
//
// The full rule builder for the GTK4 desktop interface.
//
// Layout:
//   ┌────────────────────────────────────────────────────────────┐
//   │  Template Builder                                          │
//   │    [template entry]                                        │
//   │    ✓ Valid / ✗ Error message                              │
//   ├────────────────────────────────────────────────────────────┤
//   │  Live Preview                                              │
//   │    (sample output based on sample tags)                    │
//   ├────────────────────────────────────────────────────────────┤
//   │  Tag Reference (click to insert)                          │
//   │  [<Title>] [<Artist>] [<Album>] … (pill buttons)          │
//   ├────────────────────────────────────────────────────────────┤
//   │  Sample Tag Values (editable — drives the live preview)    │
//   │    Title:  [entry]  Artist: [entry]  Year: [entry]…       │
//   └────────────────────────────────────────────────────────────┘

use std::cell::RefCell;
use std::rc::Rc;

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use mm_core::rule_engine;

use crate::state::RulesState;

/// The Rules tab panel.
pub struct RulesPanel {
    root: gtk::Box,
}

impl RulesPanel {
    /// Construct the rules panel widget tree.
    pub fn new() -> Self {
        let state = Rc::new(RefCell::new(RulesState::new()));

        // ------------------------------------------------------------------
        // Template entry + validation indicator
        // ------------------------------------------------------------------

        let template_entry = gtk::Entry::builder()
            .text(state.borrow().template.clone())
            .placeholder_text("Rename template, e.g. <Artist>/<Album>/<TrackNumber>. <Title>")
            .hexpand(true)
            .build();

        // Validation label (green on valid, red on error)
        let validation_label = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();

        // Validate on every keystroke and update preview
        {
            let vl      = validation_label.clone();
            let state_c = Rc::clone(&state);

            template_entry.connect_changed(move |entry| {
                let text = entry.text().to_string();
                let mut s = state_c.borrow_mut();
                s.set_template(&text);

                if text.trim().is_empty() {
                    vl.set_text("");
                    vl.remove_css_class("error");
                    vl.remove_css_class("success");
                    s.validation = None;
                    return;
                }

                match rule_engine::parse_template(&text) {
                    Ok(_) => {
                        s.validation = Some(Ok(()));
                        vl.set_text("✓ Valid template");
                        vl.remove_css_class("error");
                        vl.add_css_class("success");
                    }
                    Err(e) => {
                        s.validation = Some(Err(e.to_string()));
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
                .label("Rename Template")
                .halign(gtk::Align::Start)
                .css_classes(["heading"])
                .build(),
        );
        validator_box.append(&template_entry);
        validator_box.append(&validation_label);

        // ------------------------------------------------------------------
        // Live preview
        // ------------------------------------------------------------------

        let preview_label = gtk::Label::builder()
            .label(&state.borrow().compute_preview())
            .halign(gtk::Align::Start)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .css_classes(["monospace"])
            .selectable(true)
            .build();

        let preview_group = adw::PreferencesGroup::new();
        preview_group.set_title("Live Preview");
        preview_group.set_description(Some(
            "Output with the sample tag values shown below. Click to copy.",
        ));

        let preview_row = adw::ActionRow::new();
        preview_row.set_activatable(false);
        preview_row.set_title("Output");
        preview_row.add_suffix(&preview_label);
        preview_group.add(&preview_row);

        // Keep preview updated when the template entry changes
        {
            let pl      = preview_label.clone();
            let state_c = Rc::clone(&state);

            template_entry.connect_changed(move |_| {
                pl.set_text(&state_c.borrow().preview_output);
            });
        }

        // ------------------------------------------------------------------
        // Tag reference panel — pill buttons that insert tags into the entry
        // ------------------------------------------------------------------

        let known_tags = [
            "Title", "Artist", "Album", "AlbumArtist", "Year", "Genre",
            "TrackNumber", "TrackTotal", "DiscNumber", "DiscTotal",
            "Composer", "Comment", "Lyrics", "ISRC", "Barcode",
            "CatalogNumber", "Label", "Compilation", "BPM",
            "Filename", "Extension", "Folder", "Duration",
            "BitrateKbps", "SampleRateHz", "MediaClass", "MediaFormat",
        ];

        let tags_flow = gtk::FlowBox::builder()
            .homogeneous(false)
            .column_spacing(6)
            .row_spacing(4)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .selection_mode(gtk::SelectionMode::None)
            .build();

        for tag in &known_tags {
            let pill = gtk::Button::builder()
                .label(&format!("<{tag}>"))
                .css_classes(["pill"])
                .build();

            let entry_clone = template_entry.clone();
            let tag_text = format!("<{tag}>");

            pill.connect_clicked(move |_| {
                // Insert tag text at the current cursor position
                let mut pos = entry_clone.position();
                entry_clone.insert_text(&tag_text, &mut pos);
                entry_clone.grab_focus();
            });

            tags_flow.insert(&pill, -1);
        }

        // ------------------------------------------------------------------
        // Sample tag editor — drives the live preview
        // ------------------------------------------------------------------

        let sample_group = adw::PreferencesGroup::new();
        sample_group.set_title("Sample Values");
        sample_group.set_description(Some(
            "Edit these to see how your template behaves with different data.",
        ));

        // Display key sample tags as editable rows
        let sample_keys = ["Title", "Artist", "Album", "Year", "TrackNumber", "Genre", "Composer"];

        for key in &sample_keys {
            let initial = state.borrow().sample_tags.get(*key).cloned().unwrap_or_default();

            let row = adw::EntryRow::new();
            row.set_title(*key);
            row.set_text(&initial);

            let state_c = Rc::clone(&state);
            let pl      = preview_label.clone();
            let key_owned = key.to_string();

            row.connect_changed(move |r| {
                let val = r.text().to_string();
                let mut s = state_c.borrow_mut();
                s.sample_tags.insert(key_owned.clone(), val);
                pl.set_text(&s.compute_preview());
                s.preview_output = s.compute_preview();
            });

            sample_group.add(&row);
        }

        // ------------------------------------------------------------------
        // Root layout (scrolled)
        // ------------------------------------------------------------------

        let inner = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .margin_start(12)
            .margin_end(12)
            .margin_top(4)
            .margin_bottom(12)
            .build();

        inner.append(&preview_group);
        inner.append(&sample_group);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&inner)
            .build();

        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        // Template builder always visible at top
        root.append(&validator_box);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // Tag pills
        root.append(
            &gtk::Label::builder()
                .label("Insert Tag")
                .halign(gtk::Align::Start)
                .margin_start(12)
                .margin_top(8)
                .margin_bottom(4)
                .css_classes(["heading"])
                .build(),
        );
        root.append(&tags_flow);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

        // Preview + sample editor (scrolled)
        root.append(&scrolled);

        Self { root }
    }

    /// Return the root widget for placement in AdwTabView.
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }
}
