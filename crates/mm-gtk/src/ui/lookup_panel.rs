// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Metadata Lookup Panel (M6)
//
// Lets users search for metadata across all enabled providers:
//   1. Enter a title (and optional artist) in the search fields
//   2. Check/uncheck the providers to include
//   3. Click "Search" — results arrive sorted by match score
//   4. Select a result row to preview its metadata
//   5. Click "Apply to File" to write the metadata to the open file
//
// Layout:
//   ┌────────────────────────────────────────────────────────┐
//   │  Query: [title entry]                                  │
//   │  Artist: [artist entry]   [Search]                     │
//   ├────────────────────────────────────────────────────────┤
//   │  Providers: [☑ MusicBrainz] [☑ Spotify] [☑ Apple] … │
//   ├────────────────────────────────────────────────────────┤
//   │  [status label]  (searching…)                         │
//   │  [scrolled results list]                              │
//   │    Score  Provider  Title — Artist — Year             │
//   ├────────────────────────────────────────────────────────┤
//   │  [Apply to File]  [Clear]                             │
//   └────────────────────────────────────────────────────────┘

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::state::{LookupResult, LookupState};
use crate::ui::accessibility;

/// The Metadata Lookup panel.
pub struct LookupPanel {
    root: gtk::Box,
}

impl LookupPanel {
    /// Build the lookup panel widget tree.
    pub fn new() -> Self {
        let state = Rc::new(RefCell::new(LookupState::new()));

        // ------------------------------------------------------------------
        // Search fields
        // ------------------------------------------------------------------

        let title_entry = gtk::Entry::builder()
            .placeholder_text("Track / show / podcast title…")
            .hexpand(true)
            .build();

        let artist_entry = gtk::Entry::builder()
            .placeholder_text("Artist (optional)")
            .hexpand(true)
            .build();

        let search_btn = gtk::Button::builder()
            .label("Search")
            .css_classes(["suggested-action"])
            .build();

        // Title row
        let title_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();
        title_row.append(&gtk::Label::builder().label("Query").width_chars(8).xalign(0.0).build());
        title_row.append(&title_entry);

        // Artist row
        let artist_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();
        artist_row.append(&gtk::Label::builder().label("Artist").width_chars(8).xalign(0.0).build());
        artist_row.append(&artist_entry);
        artist_row.append(&search_btn);

        let query_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(8)
            .build();
        query_box.append(&title_row);
        query_box.append(&artist_row);

        // ------------------------------------------------------------------
        // Provider checkboxes (flow box — wraps on narrow windows)
        // ------------------------------------------------------------------

        let providers_label = gtk::Label::builder()
            .label("Providers")
            .halign(gtk::Align::Start)
            .margin_start(12)
            .margin_top(6)
            .margin_bottom(2)
            .css_classes(["heading"])
            .build();

        // All known provider display names + internal names
        let provider_info: Vec<(&str, &str)> = vec![
            ("musicbrainz",   "MusicBrainz"),
            ("spotify",       "Spotify"),
            ("apple_music",   "Apple Music"),
            ("deezer",        "Deezer"),
            ("tmdb",          "TMDb"),
            ("thetvdb",       "TheTVDB"),
            ("omdb",          "OMDb"),
            ("apple_tv",      "Apple TV"),
            ("itunes_store",  "iTunes Store"),
            ("apple_podcasts","Apple Podcasts"),
            ("isrc",          "ISRC"),
            ("eidr",          "EIDR"),
            ("iswc",          "ISWC"),
            ("youtube_music", "YouTube Music*"),
            ("amazon_music",  "Amazon Music*"),
            ("pandora",       "Pandora*"),
            ("tidal",         "Tidal*"),
            ("shazam",        "Shazam*"),
            ("iheart",        "iHeart*"),
        ];

        let providers_flow = gtk::FlowBox::builder()
            .homogeneous(false)
            .column_spacing(4)
            .row_spacing(2)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(8)
            .selection_mode(gtk::SelectionMode::None)
            .build();

        for (name, display) in &provider_info {
            let enabled = state.borrow().providers.get(*name).copied().unwrap_or(false);
            let check = gtk::CheckButton::builder()
                .label(*display)
                .active(enabled)
                .build();

            // Stub providers get a dim appearance
            if display.ends_with('*') {
                check.add_css_class("dim-label");
            }

            let state_clone = Rc::clone(&state);
            let name_owned = name.to_string();
            check.connect_toggled(move |_| {
                state_clone.borrow_mut().toggle_provider(&name_owned);
            });

            providers_flow.insert(&check, -1);
        }

        // ------------------------------------------------------------------
        // Status label + spinner
        // ------------------------------------------------------------------

        let spinner = gtk::Spinner::new();
        let status_label = gtk::Label::builder()
            .label("Enter a title to search.")
            .halign(gtk::Align::Start)
            .hexpand(true)
            .css_classes(["dim-label"])
            .build();

        let status_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .margin_start(12)
            .margin_top(6)
            .margin_bottom(4)
            .build();
        status_row.append(&spinner);
        status_row.append(&status_label);

        // ------------------------------------------------------------------
        // Results list (scrolled)
        // ------------------------------------------------------------------

        let results_list = gtk::ListBox::builder()
            .css_classes(["boxed-list"])
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(8)
            .selection_mode(gtk::SelectionMode::Single)
            .build();

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&results_list)
            .build();

        // ------------------------------------------------------------------
        // Result detail box (shown when a row is selected)
        // ------------------------------------------------------------------

        let detail_label = gtk::Label::builder()
            .label("")
            .halign(gtk::Align::Start)
            .wrap(true)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(4)
            .css_classes(["dim-label", "caption"])
            .visible(false)
            .build();

        // ------------------------------------------------------------------
        // Action buttons
        // ------------------------------------------------------------------

        let apply_btn = gtk::Button::builder()
            .label("Apply to File")
            .css_classes(["suggested-action"])
            .sensitive(false)
            .build();

        let clear_btn = gtk::Button::builder()
            .label("Clear")
            .build();

        let btn_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::End)
            .margin_start(12)
            .margin_end(12)
            .margin_bottom(12)
            .build();
        btn_row.append(&clear_btn);
        btn_row.append(&apply_btn);

        // ------------------------------------------------------------------
        // AT-SPI2 accessibility labels (Issue #128)
        // ------------------------------------------------------------------
        accessibility::set_label(&title_entry, "Search query");
        accessibility::set_description(&title_entry, "Enter a track, show, or podcast title to search across all enabled providers.");
        accessibility::set_label(&artist_entry, "Artist hint (optional)");
        accessibility::set_description(&artist_entry, "Optionally narrow the search by providing an artist name.");
        accessibility::set_label(&search_btn, "Search providers");
        accessibility::set_description(&search_btn, "Searches all enabled metadata providers for the entered query.");
        accessibility::set_label(&apply_btn, "Apply result to file");
        accessibility::set_description(&apply_btn, "Writes the selected result's tags to the currently open media file.");
        accessibility::set_label(&clear_btn, "Clear results");
        accessibility::set_description(&clear_btn, "Clears the search results and resets the lookup form.");
        accessibility::set_label(&status_label, "Lookup status");

        // ------------------------------------------------------------------
        // Root layout
        // ------------------------------------------------------------------

        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        root.append(&query_box);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        root.append(&providers_label);
        root.append(&providers_flow);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        root.append(&status_row);
        root.append(&scrolled);
        root.append(&detail_label);
        root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        root.append(&btn_row);

        // ------------------------------------------------------------------
        // Signal: Search button
        // ------------------------------------------------------------------
        {
            let state_c     = Rc::clone(&state);
            let status_c    = status_label.clone();
            let spinner_c   = spinner.clone();
            let results_c   = results_list.clone();
            let detail_c    = detail_label.clone();
            let apply_c     = apply_btn.clone();
            let title_c     = title_entry.clone();
            let artist_c    = artist_entry.clone();

            search_btn.connect_clicked(move |_| {
                let query = title_c.text().to_string();
                if query.trim().is_empty() {
                    status_c.set_text("⚠ Enter a title to search.");
                    return;
                }

                let artist = artist_c.text().to_string();
                {
                    let mut s = state_c.borrow_mut();
                    s.query = query.clone();
                    s.artist_hint = artist.clone();
                    s.clear_results();
                    s.searching = true;
                    s.status = "Searching…".into();
                }

                // Clear previous results
                while let Some(child) = results_c.first_child() {
                    results_c.remove(&child);
                }
                detail_c.set_visible(false);
                apply_c.set_sensitive(false);
                status_c.set_text("Searching…");
                spinner_c.start();

                // Run the provider search in a background thread using a
                // dedicated tokio runtime, then deliver results back via
                // a glib channel (std::sync::mpsc → glib idle_add).
                let enabled_providers: Vec<String> = state_c
                    .borrow()
                    .enabled_providers()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();

                let (tx, rx) = mpsc::channel::<Vec<LookupResult>>();

                std::thread::spawn(move || {
                    let results = run_search_blocking(&query, &artist, &enabled_providers);
                    let _ = tx.send(results);
                });

                // Poll for results on the GLib main thread
                let state_poll  = Rc::clone(&state_c);
                let status_poll = status_c.clone();
                let spinner_poll = spinner_c.clone();
                let results_poll = results_c.clone();
                let detail_poll  = detail_c.clone();
                let apply_poll   = apply_c.clone();

                gtk::glib::idle_add_local(move || {
                    match rx.try_recv() {
                        Ok(results) => {
                            spinner_poll.stop();
                            let count = results.len();
                            {
                                let mut s = state_poll.borrow_mut();
                                s.results = results;
                                s.searching = false;
                                s.status = if count == 0 {
                                    "No results found.".into()
                                } else {
                                    format!("{count} result(s) found.")
                                };
                            }
                            status_poll.set_text(&state_poll.borrow().status);

                            // Populate results list
                            for (i, result) in state_poll.borrow().results.iter().enumerate() {
                                let row = build_result_row(result, i);
                                results_poll.append(&row);
                            }

                            // Enable apply button when a row is selected
                            let apply_c2 = apply_poll.clone();
                            let detail_c2 = detail_poll.clone();
                            let state_c2 = Rc::clone(&state_poll);
                            results_poll.connect_row_selected(move |_, row| {
                                if let Some(row) = row {
                                    let idx = row.index() as usize;
                                    state_c2.borrow_mut().selected = Some(idx);
                                    if let Some(r) = state_c2.borrow().selected_result() {
                                        let detail = format!(
                                            "Provider: {} · ID: {} · Score: {:.2}{}",
                                            r.provider, r.provider_id, r.score,
                                            r.cover_art_url.as_deref()
                                                .map(|u| format!(" · Cover: {u}"))
                                                .unwrap_or_default()
                                        );
                                        detail_c2.set_text(&detail);
                                        detail_c2.set_visible(true);
                                    }
                                    apply_c2.set_sensitive(true);
                                } else {
                                    state_c2.borrow_mut().selected = None;
                                    apply_c2.set_sensitive(false);
                                    detail_c2.set_visible(false);
                                }
                            });

                            gtk::glib::ControlFlow::Break
                        }
                        Err(mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            spinner_poll.stop();
                            status_poll.set_text("Search failed (internal error).");
                            gtk::glib::ControlFlow::Break
                        }
                    }
                });
            });
        }

        // ------------------------------------------------------------------
        // Signal: Clear button
        // ------------------------------------------------------------------
        {
            let state_c   = Rc::clone(&state);
            let results_c = results_list.clone();
            let status_c  = status_label.clone();
            let detail_c  = detail_label.clone();
            let apply_c   = apply_btn.clone();
            let title_c   = title_entry.clone();
            let artist_c  = artist_entry.clone();

            clear_btn.connect_clicked(move |_| {
                state_c.borrow_mut().clear_results();
                while let Some(child) = results_c.first_child() {
                    results_c.remove(&child);
                }
                status_c.set_text("Enter a title to search.");
                detail_c.set_text("");
                detail_c.set_visible(false);
                apply_c.set_sensitive(false);
                title_c.set_text("");
                artist_c.set_text("");
            });
        }

        // ------------------------------------------------------------------
        // Signal: Apply button (placeholder — requires open file context)
        // ------------------------------------------------------------------
        {
            let state_c  = Rc::clone(&state);
            let status_c = status_label.clone();

            apply_btn.connect_clicked(move |_| {
                let binding = state_c.borrow();
                if let Some(result) = binding.selected_result() {
                    // Full apply-to-file requires coordination with MetadataPanel.
                    // For M6, show the selected metadata as a confirmation message.
                    let msg = format!(
                        "✓ Would apply: {} — {} ({})",
                        result.artist.as_deref().unwrap_or("?"),
                        result.title.as_deref().unwrap_or("?"),
                        result.year.map_or("?".to_owned(), |y| y.to_string()),
                    );
                    status_c.set_text(&msg);
                }
            });
        }

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

/// Build a single result row widget.
///
/// Shows: score badge | provider name | title — artist (year)
fn build_result_row(result: &LookupResult, _index: usize) -> gtk::Box {
    // Score badge (0.00 — 1.00+)
    let score_label = gtk::Label::builder()
        .label(&format!("{:.2}", result.score))
        .width_chars(5)
        .css_classes(["dim-label", "numeric"])
        .build();

    // Provider pill
    let provider_label = gtk::Label::builder()
        .label(&result.provider)
        .css_classes(["pill"])
        .width_chars(14)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();

    // Main info: "Title — Artist (Year)"
    let title_str = result.title.as_deref().unwrap_or("Unknown");
    let artist_str = result.artist.as_deref().unwrap_or("");
    let year_str = result.year.map(|y| format!(" ({y})")).unwrap_or_default();

    let info_text = if artist_str.is_empty() {
        format!("{title_str}{year_str}")
    } else {
        format!("{title_str} — {artist_str}{year_str}")
    };

    let info_label = gtk::Label::builder()
        .label(&info_text)
        .hexpand(true)
        .halign(gtk::Align::Start)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .build();

    let row = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(8)
        .margin_start(12)
        .margin_end(12)
        .margin_top(6)
        .margin_bottom(6)
        .build();

    row.append(&score_label);
    row.append(&provider_label);
    row.append(&info_label);
    row
}

/// Run a provider search in the current thread using a local tokio runtime.
///
/// Called from a background `std::thread::spawn` so it does not block the
/// GTK main loop.  Returns an empty Vec on any error.
fn run_search_blocking(
    query: &str,
    artist: &str,
    _enabled_providers: &[String],
) -> Vec<LookupResult> {
    // Build a minimal tokio runtime for this thread
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    rt.block_on(async move {
        use mm_providers::{ProviderRegistry, SearchQuery, MusicBrainzProvider};

        let mut registry = ProviderRegistry::new();
        // Register only MusicBrainz for the background search (no API key needed).
        // Additional providers (Spotify, TMDb, etc.) require credentials configured
        // by the user in Settings — they will be wired in a future patch.
        // Use the standard MeedyaManager User-Agent (MusicBrainz requires a descriptive UA).
        registry.register(MusicBrainzProvider::new(mm_core::useragent::build_user_agent()));

        let mut search_query = SearchQuery {
            query: query.to_owned(),
            ..Default::default()
        };
        if !artist.is_empty() {
            search_query.artist = Some(artist.to_owned());
        }
        search_query.title = if query.is_empty() { None } else { Some(query.to_owned()) };

        let results = registry.search(&search_query).await;

        results.into_iter().map(|r| {
            let cover_art_url = r.cover_art.into_iter().next().map(|a| a.url);
            LookupResult {
                provider:      r.provider,
                title:         r.title,
                artist:        r.artist,
                album:         r.album,
                year:          r.year,
                genre:         r.genre,
                provider_id:   r.provider_id,
                score:         r.score,
                cover_art_url,
            }
        }).collect()
    })
}
