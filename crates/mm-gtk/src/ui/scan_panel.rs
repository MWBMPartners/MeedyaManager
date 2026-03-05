// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Library / Scan Panel
//
// The Scan panel lets users:
//   1. Pick a source folder (via native file dialog)
//   2. Enter a rename template (e.g. "<Artist> - <Title>")
//   3. Preview computed renames in a list
//   4. Execute approved renames with a single click
//
// Layout:
//   ┌──────────────────────────────────────────────────────────┐
//   │  [Options group]  Folder: [path entry] [Browse…]        │
//   │                   Template: [entry]                      │
//   │                   ☐ Recursive   [Scan]                   │
//   ├──────────────────────────────────────────────────────────┤
//   │  [Results: "3 files — 2 to rename, 1 unchanged"]         │
//   │  ┌────────────────────────────────────────────────────┐  │
//   │  │  Source path → Destination path  [conflict badge]  │  │
//   │  │  ...                                               │  │
//   │  └────────────────────────────────────────────────────┘  │
//   │  [Execute Renames]                                       │
//   └──────────────────────────────────────────────────────────┘

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use mm_core::renamer::{self, SanitizeConfig};
use mm_core::metadata;

use crate::state::ScanState;

/// The Library / Scan panel widget.
pub struct ScanPanel {
    /// Root widget returned to AdwTabView
    root: gtk::Box,
    /// Shared application state
    state: Rc<RefCell<ScanState>>,
}

impl ScanPanel {
    /// Build the scan panel widget tree and return the panel.
    pub fn new() -> Self {
        let state = Rc::new(RefCell::new(ScanState::new()));

        // ------------------------------------------------------------------
        // Options section (Adwaita preferences-style group)
        // ------------------------------------------------------------------

        // Folder entry + browse button in a horizontal row
        let folder_entry = gtk::Entry::builder()
            .placeholder_text("Select a folder to scan…")
            .hexpand(true)
            .build();

        let browse_btn = gtk::Button::builder()
            .label("Browse…")
            .build();

        let folder_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();
        folder_row.append(&folder_entry);
        folder_row.append(&browse_btn);

        // Template entry row
        let template_entry = gtk::Entry::builder()
            .text("<Artist> - <Title>")
            .hexpand(true)
            .build();

        // Recursive checkbox
        let recursive_check = gtk::CheckButton::builder()
            .label("Include sub-folders")
            .active(false)
            .build();

        // Scan button
        let scan_btn = gtk::Button::builder()
            .label("Scan")
            .css_classes(["suggested-action"])
            .build();

        // Combine controls in a vertical box with Adwaita-styled labels
        let options_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(8)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .build();

        options_box.append(&make_option_row("Folder", &folder_row));
        options_box.append(&make_option_row("Template", &template_entry));
        options_box.append(&recursive_check);

        // Button row: right-aligned Scan button
        let btn_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .halign(gtk::Align::End)
            .build();
        btn_row.append(&scan_btn);
        options_box.append(&btn_row);

        // Wrap in an Adwaita preferences group-style frame
        let options_frame = adw::PreferencesGroup::new();
        options_frame.set_title("Scan Options");
        options_frame.set_description(Some("Configure the source folder and rename template"));

        // PreferencesGroup doesn't add arbitrary widgets directly in all versions;
        // use a boxed row container instead
        let options_row = adw::ActionRow::new();
        // We add the box directly as a suffixed widget
        options_row.set_activatable(false);

        // ------------------------------------------------------------------
        // Results section
        // ------------------------------------------------------------------

        // Summary label (e.g. "3 files — 2 to rename, 1 unchanged, 0 conflicts")
        let summary_label = gtk::Label::builder()
            .label("No files scanned yet.")
            .halign(gtk::Align::Start)
            .margin_start(12)
            .margin_top(12)
            .margin_bottom(6)
            .css_classes(["dim-label"])
            .build();

        // Results list — a scrolled vertical box of preview rows
        let results_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&results_box)
            .build();

        // Execute button (disabled until there are non-conflicting previews)
        let execute_btn = gtk::Button::builder()
            .label("Execute Renames")
            .css_classes(["destructive-action"])
            .sensitive(false)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .halign(gtk::Align::End)
            .build();

        // ------------------------------------------------------------------
        // Root container
        // ------------------------------------------------------------------

        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        root.append(&options_box);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        root.append(&summary_label);
        root.append(&scrolled);
        root.append(&execute_btn);

        // ------------------------------------------------------------------
        // Signal handlers
        // ------------------------------------------------------------------

        // Browse button — opens a native folder picker dialog
        {
            let folder_entry_clone = folder_entry.clone();
            let state_clone = Rc::clone(&state);
            browse_btn.connect_clicked(move |btn| {
                // Find the parent window by walking up the widget tree
                if let Some(window) = btn.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                    let folder_entry_c = folder_entry_clone.clone();
                    let state_c = Rc::clone(&state_clone);

                    let dialog = gtk::FileDialog::builder()
                        .title("Select Folder to Scan")
                        .modal(true)
                        .build();

                    // Open the folder chooser asynchronously using GLib futures
                    let ctx = gtk::glib::MainContext::default();
                    ctx.spawn_local(async move {
                        if let Ok(folder) = dialog.select_folder_future(Some(&window)).await {
                            if let Some(path) = folder.path() {
                                folder_entry_c.set_text(&path.to_string_lossy());
                                state_c.borrow_mut().directory = Some(path);
                            }
                        }
                    });
                }
            });
        }

        // Folder entry — update state when text changes
        {
            let state_clone = Rc::clone(&state);
            folder_entry.connect_changed(move |entry| {
                let text = entry.text();
                if text.is_empty() {
                    state_clone.borrow_mut().directory = None;
                } else {
                    state_clone.borrow_mut().directory = Some(PathBuf::from(text.as_str()));
                }
            });
        }

        // Template entry — update state when text changes
        {
            let state_clone = Rc::clone(&state);
            template_entry.connect_changed(move |entry| {
                state_clone.borrow_mut().template = entry.text().to_string();
            });
        }

        // Recursive check — update state
        {
            let state_clone = Rc::clone(&state);
            recursive_check.connect_toggled(move |check| {
                state_clone.borrow_mut().recursive = check.is_active();
            });
        }

        // Scan button — run the scan and populate the results list
        {
            let state_clone = Rc::clone(&state);
            let summary_label_clone = summary_label.clone();
            let results_box_clone = results_box.clone();
            let execute_btn_clone = execute_btn.clone();
            let template_entry_clone = template_entry.clone();
            let folder_entry_clone = folder_entry.clone();

            scan_btn.connect_clicked(move |_| {
                let dir_opt = state_clone.borrow().directory.clone();
                let template = template_entry_clone.text().to_string();
                let recursive = state_clone.borrow().recursive;

                let dir = match dir_opt {
                    Some(d) => d,
                    None => {
                        summary_label_clone.set_text("⚠ Please select a folder first.");
                        return;
                    }
                };

                // Run the scan synchronously (acceptable for M4 shell;
                // M6 will move this to a background thread with progress reporting)
                match run_scan(&dir, &template, recursive) {
                    Ok(previews) => {
                        state_clone.borrow_mut().previews = previews;

                        // Update the summary label
                        let summary = state_clone.borrow().preview_summary();
                        summary_label_clone.set_text(&summary);

                        // Clear previous results and repopulate
                        while let Some(child) = results_box_clone.first_child() {
                            results_box_clone.remove(&child);
                        }
                        for preview in &state_clone.borrow().previews {
                            let row = build_preview_row(preview);
                            results_box_clone.append(&row);
                        }

                        // Enable the execute button if there are viable renames
                        let can_execute = !state_clone.borrow().executable_previews().is_empty();
                        execute_btn_clone.set_sensitive(can_execute);
                    }
                    Err(e) => {
                        summary_label_clone.set_text(&format!("⚠ Scan failed: {e}"));
                        execute_btn_clone.set_sensitive(false);
                    }
                }
            });
        }

        // Execute button — perform the renames
        {
            let state_clone = Rc::clone(&state);
            let summary_label_clone = summary_label.clone();
            let execute_btn_clone = execute_btn.clone();

            execute_btn.connect_clicked(move |_| {
                let mut state = state_clone.borrow_mut();
                let mut renamed = 0usize;
                let mut errors = 0usize;

                for preview in state.executable_previews() {
                    if let Err(e) = std::fs::rename(&preview.source, &preview.destination) {
                        tracing::error!(
                            "Rename failed: {} → {}: {}",
                            preview.source.display(),
                            preview.destination.display(),
                            e
                        );
                        errors += 1;
                    } else {
                        renamed += 1;
                    }
                }

                // Clear previews after execution
                state.previews.clear();
                execute_btn_clone.set_sensitive(false);

                if errors == 0 {
                    summary_label_clone.set_text(&format!("✓ Renamed {renamed} files successfully."));
                } else {
                    summary_label_clone.set_text(&format!(
                        "⚠ Renamed {renamed} files; {errors} errors (see log)."
                    ));
                }
            });
        }

        Self { root, state }
    }

    /// Return a reference to the root widget for placement in AdwTabView.
    pub fn widget(&self) -> &gtk::Widget {
        self.root.upcast_ref()
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Run a directory scan and return RenamePreview results.
///
/// Collects media files, reads their metadata, then uses the renamer module
/// to simulate rename operations under the given template.
fn run_scan(
    dir: &PathBuf,
    template: &str,
    recursive: bool,
) -> Result<Vec<mm_core::renamer::RenamePreview>, String> {
    use mm_core::classify::{classify_by_extension, MediaGroup};

    // Collect recognised media files from the directory
    let mut files_with_tags: Vec<(PathBuf, std::collections::HashMap<String, String>)> = Vec::new();

    let entries = collect_dir(dir, recursive).map_err(|e| e.to_string())?;

    for path in entries {
        // Filter to Audio and Video files only
        let is_media = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| {
                let c = classify_by_extension(ext);
                matches!(c.group, MediaGroup::Audio | MediaGroup::Video)
            })
            .unwrap_or(false);

        if !is_media {
            continue;
        }

        // Extract tags; use empty map if reading fails
        let flat: std::collections::HashMap<String, String> =
            metadata::extract_tags(&path)
                .map(|tm| tm.into_iter().map(|(k, v)| (k, v.join("; "))).collect())
                .unwrap_or_default();

        files_with_tags.push((path, flat));
    }

    if files_with_tags.is_empty() {
        return Ok(vec![]);
    }

    // Simulate renames using mm-core renamer
    let summary = renamer::simulate_rename(
        &files_with_tags,
        template,
        dir,
        &SanitizeConfig::default(),
    )
    .map_err(|e| e.to_string())?;

    Ok(summary.previews)
}

/// Recursively (or non-recursively) collect all file paths from a directory.
fn collect_dir(dir: &PathBuf, recursive: bool) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && recursive {
            paths.extend(collect_dir(&path, recursive)?);
        } else if path.is_file() {
            paths.push(path);
        }
    }
    Ok(paths)
}

/// Build a single row widget displaying a RenamePreview.
///
/// Each row shows:  [source filename] → [destination filename]  [badge]
fn build_preview_row(preview: &mm_core::renamer::RenamePreview) -> gtk::Box {
    // Source filename (basename only for readability)
    let src_name = preview.source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| preview.source.to_string_lossy().into_owned());

    // Destination filename
    let dst_name = preview.destination
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| preview.destination.to_string_lossy().into_owned());

    // Indicator: arrow between source and destination
    let arrow = gtk::Label::builder()
        .label("→")
        .css_classes(["dim-label"])
        .margin_start(8)
        .margin_end(8)
        .build();

    let src_label = gtk::Label::builder()
        .label(&src_name)
        .ellipsize(gtk::pango::EllipsizeMode::Middle)
        .max_width_chars(40)
        .hexpand(true)
        .halign(gtk::Align::Start)
        .build();

    let dst_label = gtk::Label::builder()
        .label(&dst_name)
        .ellipsize(gtk::pango::EllipsizeMode::Middle)
        .max_width_chars(40)
        .hexpand(true)
        .halign(gtk::Align::Start)
        .build();

    // Status badge
    let badge_text = if preview.conflict {
        "CONFLICT"
    } else if preview.unchanged {
        "UNCHANGED"
    } else {
        "→ RENAME"
    };

    let badge_classes: &[&str] = if preview.conflict {
        &["error", "pill"]
    } else if preview.unchanged {
        &["dim-label"]
    } else {
        &["success", "pill"]
    };

    let badge = gtk::Label::builder()
        .label(badge_text)
        .css_classes(badge_classes)
        .build();

    // Assemble the row
    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(0)
        .margin_start(12)
        .margin_end(12)
        .margin_top(6)
        .margin_bottom(6)
        .build();

    row.append(&src_label);
    row.append(&arrow);
    row.append(&dst_label);
    row.append(&badge);

    // Add a bottom separator (except for conflict rows which are visually distinct)
    row
}

/// Build a labelled option row: [label] [widget] on a horizontal box.
fn make_option_row(label_text: &str, widget: &impl IsA<gtk::Widget>) -> gtk::Box {
    let label = gtk::Label::builder()
        .label(label_text)
        .width_chars(10)
        .xalign(0.0)
        .build();

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();

    row.append(&label);
    row.append(widget);
    row
}
