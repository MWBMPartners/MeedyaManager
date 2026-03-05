// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Main Application Window (M8)
//
// Constructs the top-level AdwApplicationWindow with:
//   - AdwHeaderBar (title + dark/light toggle + about button)
//   - AdwBanner   (update notification — shown when a newer version is available)
//   - AdwTabBar   (tab navigation)
//   - AdwTabView  (hosts six panel tabs)
//   - AdwToastOverlay (notification toasts over the tab content)
//
// Tab layout (M7):
//   📁 Library  — scan a folder and preview renames
//   🏷️ Metadata  — view and edit file tags (with cover art)
//   🔍 Lookup    — search metadata providers
//   ⚙️ Rules     — full template/rule builder
//   ☁️ Cloud     — cloud storage monitor (OneDrive, Google Drive, Dropbox)
//   🗄️ Export    — database export (SQLite, MySQL, MariaDB, PostgreSQL, SQL Server)
//   🌐 Server    — HTTPS media server with JWT authentication (M10)
//   🔧 Settings  — application configuration (with save)

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use super::{cloud_panel, export_panel, lookup_panel, metadata_panel, rules_panel, scan_panel, server_panel, settings_panel};

/// Build and return the fully constructed main application window.
///
/// Called from `app::on_activate` immediately after the app is ready.
pub fn build(app: &adw::Application) -> adw::ApplicationWindow {
    // -----------------------------------------------------------------------
    // Create panels (each returns its root widget + owns its internal state)
    // -----------------------------------------------------------------------
    let scan     = scan_panel::ScanPanel::new();
    let metadata = metadata_panel::MetadataPanel::new();
    let lookup   = lookup_panel::LookupPanel::new();
    let rules    = rules_panel::RulesPanel::new();
    let cloud    = cloud_panel::CloudPanel::new();
    let export   = export_panel::ExportPanel::new();
    let server   = server_panel::ServerPanel::new();
    let settings = settings_panel::SettingsPanel::new();

    // -----------------------------------------------------------------------
    // AdwTabView — one AdwTabPage per panel
    // -----------------------------------------------------------------------
    let tab_view = adw::TabView::new();

    let add_tab = |view: &adw::TabView,
                   widget: &gtk::Widget,
                   title: &str,
                   icon_name: &str| -> adw::TabPage {
        let page = view.append(widget);
        page.set_title(title);
        let icon = gtk::gio::ThemedIcon::new(icon_name);
        page.set_icon(Some(&icon));
        page
    };

    add_tab(&tab_view, scan.widget(),     "Library",  "folder-open-symbolic");
    add_tab(&tab_view, metadata.widget(), "Metadata", "tag-symbolic");
    add_tab(&tab_view, lookup.widget(),   "Lookup",   "system-search-symbolic");
    add_tab(&tab_view, rules.widget(),    "Rules",    "preferences-system-symbolic");
    add_tab(&tab_view, cloud.widget(),    "Cloud",    "network-wireless-symbolic");
    add_tab(&tab_view, export.widget(),   "Export",   "drive-harddisk-symbolic");
    add_tab(&tab_view, server.widget(),   "Server",   "network-server-symbolic");
    add_tab(&tab_view, settings.widget(), "Settings", "emblem-system-symbolic");

    // -----------------------------------------------------------------------
    // AdwTabBar — horizontal tab strip
    // -----------------------------------------------------------------------
    let tab_bar = adw::TabBar::new();
    tab_bar.set_view(Some(&tab_view));
    tab_bar.set_expand_tabs(true);

    // -----------------------------------------------------------------------
    // AdwHeaderBar — title + dark/light toggle + about button
    // -----------------------------------------------------------------------
    let header_bar = adw::HeaderBar::new();

    let title_widget = adw::WindowTitle::new("MeedyaManager", "Media Organizer");
    header_bar.set_title_widget(Some(&title_widget));

    // Dark / Light theme toggle button (moon icon = currently dark)
    let theme_btn = gtk::ToggleButton::builder()
        .icon_name("weather-clear-night-symbolic")
        .tooltip_text("Toggle dark / light theme")
        .build();

    let style_manager = adw::StyleManager::default();
    theme_btn.set_active(style_manager.is_dark());

    {
        let sm = style_manager.clone();
        theme_btn.connect_toggled(move |btn| {
            if btn.is_active() {
                sm.set_color_scheme(adw::ColorScheme::ForceDark);
            } else {
                sm.set_color_scheme(adw::ColorScheme::ForceLight);
            }
        });
    }
    header_bar.pack_start(&theme_btn);

    // About button
    let about_btn = gtk::Button::builder()
        .icon_name("help-about-symbolic")
        .tooltip_text("About MeedyaManager")
        .build();

    let app_clone = app.clone();
    about_btn.connect_clicked(move |_| {
        show_about_dialog(&app_clone);
    });
    header_bar.pack_end(&about_btn);

    // -----------------------------------------------------------------------
    // AdwBanner — update notification (hidden by default; shown when a newer
    // version is detected by the background update checker in mm-update)
    // -----------------------------------------------------------------------
    let update_banner = adw::Banner::new(
        "A new version of MeedyaManager is available."
    );
    // "Download" button opens the GitHub releases page in the default browser
    update_banner.set_button_label(Some("Download"));
    update_banner.connect_button_clicked(|_| {
        // Open the releases page using the GIO app launcher
        let _ = gtk::gio::AppInfo::launch_default_for_uri(
            "https://github.com/MWBMPartners/MeedyaManager/releases/latest",
            gtk::gio::AppLaunchContext::NONE,
        );
    });
    // Banner starts hidden; set to `true` once update_checker confirms a newer
    // release is available (wired in mm-update integration, M9+).
    update_banner.set_revealed(false);

    // -----------------------------------------------------------------------
    // AdwToastOverlay + AdwToolbarView + AdwApplicationWindow
    // -----------------------------------------------------------------------
    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&tab_view));

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    // The update banner sits between the header bar and the tab strip so it
    // draws attention without obscuring navigation
    toolbar_view.add_top_bar(&update_banner);
    toolbar_view.add_top_bar(&tab_bar);
    toolbar_view.set_content(Some(&toast_overlay));

    adw::ApplicationWindow::builder()
        .application(app)
        .title("MeedyaManager")
        .default_width(1280)
        .default_height(820)
        .content(&toolbar_view)
        .build()
}

/// Show the About dialog with version, author, and licence information.
fn show_about_dialog(app: &adw::Application) {
    let about = adw::AboutDialog::new();
    about.set_application_name("MeedyaManager");
    about.set_application_icon("multimedia-player");
    about.set_version(env!("CARGO_PKG_VERSION"));
    about.set_developer_name("MWBM Partners Ltd");
    about.set_copyright("© 2025-2026 MWBM Partners Ltd");
    about.set_license_type(gtk::License::Gpl20Only);
    about.set_website("https://github.com/MWBMPartners/MeedyaManager");
    about.set_comments("Cross-platform media file manager and auto-organizer");

    let window = app.active_window();
    about.present(window.as_ref());
}
