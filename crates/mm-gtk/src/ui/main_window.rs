// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Main Application Window
//
// Constructs the top-level AdwApplicationWindow with:
//   - AdwHeaderBar (title + global actions)
//   - AdwTabBar (tab navigation)
//   - AdwTabView (hosts the four panel tabs)
//   - AdwToastOverlay (notification toasts over the tab content)
//
// Tab layout:
//   📁 Library  — scan a folder and preview renames
//   🏷️ Metadata  — view and edit file tags
//   ⚙️ Rules     — template/rule builder (stub in M4)
//   🔧 Settings  — application configuration

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use super::{metadata_panel, rules_panel, scan_panel, settings_panel};

/// Build and return the fully constructed main application window.
///
/// Called from `app::on_activate` immediately after the app is ready.
pub fn build(app: &adw::Application) -> adw::ApplicationWindow {
    // -----------------------------------------------------------------------
    // Create panels (each returns its root widget + owns its internal state)
    // -----------------------------------------------------------------------
    let scan     = scan_panel::ScanPanel::new();
    let metadata = metadata_panel::MetadataPanel::new();
    let rules    = rules_panel::RulesPanel::new();
    let settings = settings_panel::SettingsPanel::new();

    // -----------------------------------------------------------------------
    // AdwTabView — contains one AdwTabPage per panel
    // -----------------------------------------------------------------------
    let tab_view = adw::TabView::new();

    // Helper to add a panel tab with title and icon
    let add_tab = |view: &adw::TabView,
                   widget: &gtk::Widget,
                   title: &str,
                   icon_name: &str| -> adw::TabPage {
        let page = view.append(widget);
        page.set_title(title);
        // Use a GThemedIcon from the system icon theme
        let icon = gtk::gio::ThemedIcon::new(icon_name);
        page.set_icon(Some(&icon));
        page
    };

    add_tab(&tab_view, scan.widget(),     "Library",  "folder-open-symbolic");
    add_tab(&tab_view, metadata.widget(), "Metadata", "tag-symbolic");
    add_tab(&tab_view, rules.widget(),    "Rules",    "preferences-system-symbolic");
    add_tab(&tab_view, settings.widget(), "Settings", "emblem-system-symbolic");

    // -----------------------------------------------------------------------
    // AdwTabBar — the horizontal tab strip
    // -----------------------------------------------------------------------
    let tab_bar = adw::TabBar::new();
    tab_bar.set_view(Some(&tab_view));
    // Expand tabs to fill available width for a polished desktop look
    tab_bar.set_expand_tabs(true);

    // -----------------------------------------------------------------------
    // AdwHeaderBar — application title and global action buttons
    // -----------------------------------------------------------------------
    let header_bar = adw::HeaderBar::new();

    // Window title widget with app name and subtitle slot
    let title_widget = adw::WindowTitle::new("MeedyaManager", "Media Organizer");
    header_bar.set_title_widget(Some(&title_widget));

    // "About" menu button in the header bar end section
    let about_button = gtk::Button::builder()
        .icon_name("help-about-symbolic")
        .tooltip_text("About MeedyaManager")
        .build();

    let app_clone = app.clone();
    about_button.connect_clicked(move |_| {
        show_about_dialog(&app_clone);
    });
    header_bar.pack_end(&about_button);

    // -----------------------------------------------------------------------
    // AdwToastOverlay — wraps the tab content so toasts appear over it
    // -----------------------------------------------------------------------
    let toast_overlay = adw::ToastOverlay::new();
    toast_overlay.set_child(Some(&tab_view));

    // -----------------------------------------------------------------------
    // AdwToolbarView — stacks the header bar + tab bar above the content
    // -----------------------------------------------------------------------
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    toolbar_view.add_top_bar(&tab_bar);
    toolbar_view.set_content(Some(&toast_overlay));

    // -----------------------------------------------------------------------
    // AdwApplicationWindow — the top-level OS window
    // -----------------------------------------------------------------------
    adw::ApplicationWindow::builder()
        .application(app)
        .title("MeedyaManager")
        .default_width(1200)
        .default_height(800)
        .content(&toolbar_view)
        .build()
}

/// Show the About dialog with version, author, and licence information.
fn show_about_dialog(app: &adw::Application) {
    // AdwAboutDialog (libadwaita 1.4+) provides a native GNOME about dialog
    let about = adw::AboutDialog::new();
    about.set_application_name("MeedyaManager");
    about.set_application_icon("multimedia-player");
    about.set_version(env!("CARGO_PKG_VERSION"));
    about.set_developer_name("MWBM Partners Ltd (d/b/a MW Services)");
    about.set_copyright("© 2025-2026 MWBM Partners Ltd");
    about.set_license_type(gtk::License::Gpl20Only);
    about.set_website("https://github.com/MWBMPartners/MeedyaManager");
    about.set_comments("Cross-platform media file manager and auto-organizer");

    // Present on the active window
    let window = app.active_window();
    about.present(window.as_ref());
}
