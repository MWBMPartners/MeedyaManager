// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Metadata Editor Panel
//
// Lets users view and edit the embedded tags of any media file:
//   1. Pick a file via native file dialog
//   2. Tags are displayed in an editable list (one row per tag)
//   3. User edits values inline
//   4. "Save" writes the changes back to the file via mm-core
//   5. "Revert" discards unsaved edits
//
// Layout:
//   ┌────────────────────────────────────────────────────────┐
//   │  File: [path entry]  [Open…]                          │
//   ├────────────────────────────────────────────────────────┤
//   │  Audio: FLAC · 44100 Hz · 2ch · 5:23 · 1411 kbps     │
//   ├────────────────────────────────────────────────────────┤
//   │  [scrolled tag list]                                  │
//   │    Title:       [editable entry]                      │
//   │    Artist:      [editable entry]                      │
//   │    Album:       [editable entry]                      │
//   │    Year:        [editable entry]                      │
//   │    ...                                                │
//   ├────────────────────────────────────────────────────────┤
//   │  [Revert]                    [Save Tags]               │
//   └────────────────────────────────────────────────────────┘

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use mm_core::metadata::{self, TagMap};

use crate::state::MetadataState;

// Cover art display size (px × px square)
const COVER_ART_SIZE: i32 = 180;

/// The Metadata editor panel.
pub struct MetadataPanel {
    root: gtk::Box,
    state: Rc<RefCell<MetadataState>>,
}

impl MetadataPanel {
    /// Construct the metadata panel widget tree.
    pub fn new() -> Self {
        let state = Rc::new(RefCell::new(MetadataState::new()));

        // ------------------------------------------------------------------
        // File picker row
        // ------------------------------------------------------------------

        let file_entry = gtk::Entry::builder()
            .placeholder_text("Select a media file…")
            .hexpand(true)
            .editable(false) // read-only — path is set by dialog only
            .build();

        let open_btn = gtk::Button::builder()
            .label("Open…")
            .build();

        let file_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(8)
            .build();

        file_row.append(&gtk::Label::builder().label("File").width_chars(8).xalign(0.0).build());
        file_row.append(&file_entry);
        file_row.append(&open_btn);

        // ------------------------------------------------------------------
        // Audio properties bar (shown after a file is loaded)
        // ------------------------------------------------------------------

        let props_label = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Start)
            .margin_start(12)
            .margin_bottom(8)
            .css_classes(["dim-label", "caption"])
            .visible(false)
            .build();

        // ------------------------------------------------------------------
        // Tag list (scrolled, dynamic rows)
        // ------------------------------------------------------------------

        // Container that holds dynamically built AdwEntryRow widgets
        let tags_list = gtk::ListBox::builder()
            .css_classes(["boxed-list"])
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(8)
            .selection_mode(gtk::SelectionMode::None)
            .build();

        // Map: tag key → gtk::Entry widget (so we can read values at save time)
        // Wrapped in Rc<RefCell> so signal handlers can access it
        let entry_map: Rc<RefCell<HashMap<String, gtk::Entry>>> =
            Rc::new(RefCell::new(HashMap::new()));

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&tags_list)
            .build();

        // ------------------------------------------------------------------
        // Action buttons
        // ------------------------------------------------------------------

        let revert_btn = gtk::Button::builder()
            .label("Revert")
            .sensitive(false)
            .build();

        let save_btn = gtk::Button::builder()
            .label("Save Tags")
            .css_classes(["suggested-action"])
            .sensitive(false)
            .build();

        let btn_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::End)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .build();

        btn_row.append(&revert_btn);
        btn_row.append(&save_btn);

        // Status label
        let status_label = gtk::Label::builder()
            .label("Select a media file to view its metadata.")
            .halign(gtk::Align::Start)
            .margin_start(12)
            .margin_bottom(4)
            .css_classes(["dim-label"])
            .build();

        // ------------------------------------------------------------------
        // Cover art display (gtk::Picture, shown when art is available)
        // ------------------------------------------------------------------

        let cover_frame = gtk::Frame::builder()
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(8)
            .visible(false)
            .build();

        let cover_picture = gtk::Picture::builder()
            .width_request(COVER_ART_SIZE)
            .height_request(COVER_ART_SIZE)
            .can_shrink(true)
            .keep_aspect_ratio(true)
            .build();

        cover_frame.set_child(Some(&cover_picture));

        // ------------------------------------------------------------------
        // Root layout (cover art on the left, tags on the right in a paned)
        // ------------------------------------------------------------------

        let content_paned = gtk::Paned::builder()
            .orientation(gtk::Orientation::Horizontal)
            .position(COVER_ART_SIZE + 32) // default split position
            .vexpand(true)
            .build();

        // Left side: cover art + file info
        let left_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        left_box.append(&cover_frame);

        // Right side: scrolled tag list
        content_paned.set_start_child(Some(&left_box));
        content_paned.set_end_child(Some(&scrolled));
        content_paned.set_shrink_start_child(false);

        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        root.append(&file_row);
        root.append(&props_label);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        root.append(&content_paned);
        root.append(&status_label);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        root.append(&btn_row);

        // ------------------------------------------------------------------
        // Signal handlers
        // ------------------------------------------------------------------

        // Open button — native file picker for audio/video files
        {
            let file_entry_clone = file_entry.clone();
            let state_clone = Rc::clone(&state);
            let tags_list_clone = tags_list.clone();
            let entry_map_clone = Rc::clone(&entry_map);
            let props_label_clone = props_label.clone();
            let save_btn_clone = save_btn.clone();
            let revert_btn_clone = revert_btn.clone();
            let status_label_clone = status_label.clone();

            open_btn.connect_clicked(move |btn| {
                if let Some(window) = btn.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                    let fe = file_entry_clone.clone();
                    let sc = Rc::clone(&state_clone);
                    let tl = tags_list_clone.clone();
                    let em = Rc::clone(&entry_map_clone);
                    let pl = props_label_clone.clone();
                    let sb = save_btn_clone.clone();
                    let rb = revert_btn_clone.clone();
                    let sl = status_label_clone.clone();
                    // cover art widgets captured from outer scope are not
                    // available here — cover loading happens in load_file()

                    // Build file filter for audio/video formats
                    let filter = gtk::FileFilter::new();
                    filter.set_name(Some("Media files"));
                    // Common audio extensions
                    for ext in &["mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "aiff",
                                  "wv", "ape", "mp4", "mkv", "avi", "mov"] {
                        filter.add_suffix(ext);
                    }

                    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
                    filters.append(&filter);

                    let dialog = gtk::FileDialog::builder()
                        .title("Open Media File")
                        .modal(true)
                        .filters(&filters)
                        .build();

                    let ctx = gtk::glib::MainContext::default();
                    ctx.spawn_local(async move {
                        if let Ok(file) = dialog.open_future(Some(&window)).await {
                            if let Some(path) = file.path() {
                                fe.set_text(&path.to_string_lossy());
                                load_file(&path, &sc, &tl, &em, &pl, &sb, &rb, &sl);
                            }
                        }
                    });
                }
            });
        }

        // Save button — write pending edits to file
        {
            let state_clone = Rc::clone(&state);
            let entry_map_clone = Rc::clone(&entry_map);
            let save_btn_clone = save_btn.clone();
            let revert_btn_clone = revert_btn.clone();
            let status_label_clone = status_label.clone();

            save_btn.connect_clicked(move |_| {
                let file_path = match state_clone.borrow().file_path.clone() {
                    Some(p) => p,
                    None => return,
                };

                // Build TagMap from the current entry widget values
                let tag_map: TagMap = entry_map_clone
                    .borrow()
                    .iter()
                    .map(|(key, entry)| {
                        let val = entry.text().to_string();
                        // Split multi-values on "; " to restore the Vec<String>
                        let values = val
                            .split("; ")
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_owned())
                            .collect::<Vec<_>>();
                        (key.clone(), if values.is_empty() { vec![val] } else { values })
                    })
                    .collect();

                match metadata::write_tags(&file_path, &tag_map) {
                    Ok(()) => {
                        // Commit edits to state
                        state_clone.borrow_mut().commit_edits();
                        status_label_clone.set_text("✓ Tags saved successfully.");
                        save_btn_clone.set_sensitive(false);
                        revert_btn_clone.set_sensitive(false);
                    }
                    Err(e) => {
                        status_label_clone.set_text(&format!("⚠ Save failed: {e}"));
                    }
                }
            });
        }

        // Revert button — reset entry widgets to the last saved values
        {
            let state_clone = Rc::clone(&state);
            let entry_map_clone = Rc::clone(&entry_map);
            let save_btn_clone = save_btn.clone();
            let revert_btn_clone = revert_btn.clone();
            let status_label_clone = status_label.clone();

            revert_btn.connect_clicked(move |_| {
                let state = state_clone.borrow();
                for (key, entry) in entry_map_clone.borrow().iter() {
                    let saved = state.tags.get(key).cloned().unwrap_or_default();
                    entry.set_text(&saved);
                }
                drop(state);

                state_clone.borrow_mut().discard_edits();
                save_btn_clone.set_sensitive(false);
                revert_btn_clone.set_sensitive(false);
                status_label_clone.set_text("Changes reverted.");
            });
        }

        Self { root, state }
    }

    /// Return the root widget for placement in AdwTabView.
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Load a media file: read its tags, populate the list box with editable rows,
/// and update the audio properties label.
fn load_file(
    path: &PathBuf,
    state: &Rc<RefCell<MetadataState>>,
    tags_list: &gtk::ListBox,
    entry_map: &Rc<RefCell<HashMap<String, gtk::Entry>>>,
    props_label: &gtk::Label,
    save_btn: &gtk::Button,
    revert_btn: &gtk::Button,
    status_label: &gtk::Label,
) {
    // Clear previous contents
    while let Some(child) = tags_list.first_child() {
        tags_list.remove(&child);
    }
    entry_map.borrow_mut().clear();

    // Read tags from the file
    let tag_map = match metadata::extract_tags(path) {
        Ok(tm) => tm,
        Err(e) => {
            status_label.set_text(&format!("⚠ Could not read tags: {e}"));
            return;
        }
    };

    // Convert TagMap to flat string map for state storage
    let flat: HashMap<String, String> = tag_map
        .into_iter()
        .map(|(k, v)| (k, v.join("; ")))
        .collect();

    // Update state
    {
        let mut s = state.borrow_mut();
        s.file_path = Some(path.clone());
        s.tags = flat.clone();
        s.pending_edits.clear();
        s.status = format!("Loaded: {}", path.file_name().unwrap_or_default().to_string_lossy());
    }
    status_label.set_text(&state.borrow().status);

    // Load audio properties for the info bar
    if let Ok(props) = metadata::extract_audio_properties(path) {
        let codec = path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("?")
            .to_ascii_uppercase();

        let duration_secs = props.duration_secs as u32;
        let mins = duration_secs / 60;
        let secs = duration_secs % 60;
        let props_text = format!(
            "{codec} · {}Hz · {}ch · {mins}:{secs:02} · {}kbps",
            props.sample_rate_hz.unwrap_or(0),
            props.channels.unwrap_or(0),
            props.bitrate_kbps.unwrap_or(0),
        );
        props_label.set_text(&props_text);
        props_label.set_visible(true);
    }

    // Populate the tag list: one AdwEntryRow per tag, sorted by key
    let mut sorted_tags: Vec<(String, String)> = flat.into_iter().collect();
    sorted_tags.sort_by(|a, b| a.0.cmp(&b.0));

    for (key, value) in &sorted_tags {
        let row = build_tag_row(
            key,
            value,
            state,
            entry_map,
            save_btn,
            revert_btn,
        );
        tags_list.append(&row);
    }

    save_btn.set_sensitive(false);
    revert_btn.set_sensitive(false);
}

/// Build one editable tag row: [key label] [value entry].
///
/// The entry widget is registered in `entry_map` so the save handler can
/// collect all values without iterating the widget tree.
fn build_tag_row(
    key: &str,
    value: &str,
    state: &Rc<RefCell<MetadataState>>,
    entry_map: &Rc<RefCell<HashMap<String, gtk::Entry>>>,
    save_btn: &gtk::Button,
    revert_btn: &gtk::Button,
) -> gtk::Box {
    // Tag key label (left column)
    let key_label = gtk::Label::builder()
        .label(key)
        .width_chars(16)
        .xalign(0.0)
        .css_classes(["dim-label"])
        .build();

    // Editable value entry (right column)
    let entry = gtk::Entry::builder()
        .text(value)
        .hexpand(true)
        .build();

    // Register this entry in the map
    entry_map.borrow_mut().insert(key.to_string(), entry.clone());

    // Mark edits as pending when the entry changes
    {
        let key_owned = key.to_string();
        let state_clone = Rc::clone(state);
        let entry_map_clone = Rc::clone(entry_map);
        let save_btn_clone = save_btn.clone();
        let revert_btn_clone = revert_btn.clone();

        entry.connect_changed(move |e| {
            state_clone.borrow_mut().pending_edits.insert(
                key_owned.clone(),
                e.text().to_string(),
            );
            save_btn_clone.set_sensitive(true);
            revert_btn_clone.set_sensitive(true);
        });
    }

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .margin_start(12)
        .margin_end(12)
        .margin_top(4)
        .margin_bottom(4)
        .build();

    row.append(&key_label);
    row.append(&entry);
    row
}
