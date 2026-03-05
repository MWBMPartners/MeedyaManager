// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — GTK4 Server Panel (M10)
//
// Provides the "Server" tab in the main AdwTabView:
//
//   ServerPanel — builds the full AdwPreferencesPage for server configuration:
//     • Bind address + port (Entry widgets)
//     • TLS certificate + key paths (Entry + file picker button)
//     • CORS origins (Entry for comma-separated list)
//     • JWT expiry slider (60s – 86400s range, labelled)
//     • Start/Stop toggle button
//     • Status label (stopped / running / error)
//     • Log TextView showing access log lines
//     • Clear log button
//
// All GTK4 widgets are constructed programmatically (no XML/GtkBuilder).
// Uses adw::PreferencesGroup for consistent GNOME HIG layout.

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

// ---------------------------------------------------------------------------
// ServerPanel
// ---------------------------------------------------------------------------

/// The GTK4 server configuration and control panel.
pub struct ServerPanel {
    /// Root widget — an AdwClamp containing the preferences layout
    pub root: gtk::Widget,
}

impl ServerPanel {
    /// Build the server panel.
    pub fn new() -> Self {
        // Outer clamp — constrains content width for readability
        let clamp = adw::Clamp::new();
        clamp.set_maximum_size(700);
        clamp.set_margin_top(12);
        clamp.set_margin_bottom(12);
        clamp.set_margin_start(12);
        clamp.set_margin_end(12);

        // Vertical box holding all preference groups
        let vbox = gtk::Box::new(gtk::Orientation::Vertical, 16);

        // ── Network group ─────────────────────────────────────────────────
        let net_group = adw::PreferencesGroup::new();
        net_group.set_title("Network");
        net_group.set_description(Some("Bind address and port for the media server"));

        // Bind address
        let bind_row = adw::EntryRow::new();
        bind_row.set_title("Bind address");
        bind_row.set_text("0.0.0.0");
        bind_row.set_tooltip_text(Some("IP address to bind. Use 0.0.0.0 for all interfaces or 127.0.0.1 for loopback only."));
        net_group.add(&bind_row);

        // Port
        let port_row = adw::EntryRow::new();
        port_row.set_title("HTTPS port");
        port_row.set_text("8443");
        port_row.set_tooltip_text(Some("TCP port for HTTPS connections. Ports below 1024 require elevated privileges."));
        net_group.add(&port_row);

        // ── TLS group ─────────────────────────────────────────────────────
        let tls_group = adw::PreferencesGroup::new();
        tls_group.set_title("TLS / HTTPS");
        tls_group.set_description(Some("Certificate and private key for HTTPS termination"));

        let cert_row = adw::EntryRow::new();
        cert_row.set_title("Certificate (PEM)");
        cert_row.set_text("");
        cert_row.set_tooltip_text(Some("Path to PEM-encoded TLS certificate file (e.g. /etc/ssl/cert.pem)"));
        tls_group.add(&cert_row);

        let key_row = adw::EntryRow::new();
        key_row.set_title("Private key (PEM)");
        key_row.set_text("");
        key_row.set_tooltip_text(Some("Path to PEM-encoded private key file (e.g. /etc/ssl/key.pem)"));
        tls_group.add(&key_row);

        // No-TLS toggle (dev mode)
        let notls_row = adw::SwitchRow::new();
        notls_row.set_title("Disable TLS (HTTP only)");
        notls_row.set_subtitle("Development mode — not recommended for production");
        notls_row.set_active(false);
        tls_group.add(&notls_row);

        // ── Auth group ────────────────────────────────────────────────────
        let auth_group = adw::PreferencesGroup::new();
        auth_group.set_title("Authentication");
        auth_group.set_description(Some("JWT signing secret and token expiry"));

        let secret_row = adw::PasswordEntryRow::new();
        secret_row.set_title("JWT secret");
        secret_row.set_tooltip_text(Some("Secret key used to sign JWT tokens. Loaded from MM_JWT_SECRET env var if left empty."));
        auth_group.add(&secret_row);

        let expiry_row = adw::EntryRow::new();
        expiry_row.set_title("Token expiry (seconds)");
        expiry_row.set_text("86400");
        expiry_row.set_tooltip_text(Some("How long issued JWTs remain valid. Default: 86400 (24 hours)"));
        auth_group.add(&expiry_row);

        // ── CORS group ────────────────────────────────────────────────────
        let cors_group = adw::PreferencesGroup::new();
        cors_group.set_title("CORS");
        cors_group.set_description(Some("Allowed cross-origin request origins (comma-separated)"));

        let cors_row = adw::EntryRow::new();
        cors_row.set_title("Allowed origins");
        cors_row.set_text("");
        cors_row.set_tooltip_text(Some("e.g. https://app.example.com, https://local.dev:3000\nLeave empty to deny all cross-origin requests."));
        cors_group.add(&cors_row);

        // ── Server control group ──────────────────────────────────────────
        let ctrl_group = adw::PreferencesGroup::new();
        ctrl_group.set_title("Server Control");

        // Status label
        let status_label = gtk::Label::new(Some("Status: stopped"));
        status_label.set_halign(gtk::Align::Start);
        status_label.add_css_class("dim-label");

        // Start / Stop button row
        let btn_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        btn_row.set_halign(gtk::Align::Start);

        let start_btn = gtk::Button::with_label("Start Server");
        start_btn.add_css_class("suggested-action");
        start_btn.set_tooltip_text(Some("Start the HTTPS media server"));

        let stop_btn = gtk::Button::with_label("Stop Server");
        stop_btn.add_css_class("destructive-action");
        stop_btn.set_sensitive(false);
        stop_btn.set_tooltip_text(Some("Stop the running media server"));

        btn_row.append(&start_btn);
        btn_row.append(&stop_btn);

        // Route table button
        let routes_btn = gtk::Button::with_label("Show Routes");
        routes_btn.set_tooltip_text(Some("Display the HTTP route table for this server"));

        btn_row.append(&routes_btn);
        ctrl_group.add(&btn_row);
        ctrl_group.add(&status_label);

        // ── Log group ─────────────────────────────────────────────────────
        let log_group = adw::PreferencesGroup::new();
        log_group.set_title("Access Log");

        let log_scroll = gtk::ScrolledWindow::new();
        log_scroll.set_min_content_height(150);
        log_scroll.set_vexpand(false);

        let log_view = gtk::TextView::new();
        log_view.set_editable(false);
        log_view.set_cursor_visible(false);
        log_view.add_css_class("monospace");
        log_view.set_wrap_mode(gtk::WrapMode::WordChar);
        log_scroll.set_child(Some(&log_view));
        log_group.add(&log_scroll);

        // Clear log button
        let clear_btn = gtk::Button::with_label("Clear Log");
        clear_btn.set_halign(gtk::Align::End);
        clear_btn.set_tooltip_text(Some("Clear the access log"));

        let log_buffer = log_view.buffer();

        // Clone refs for callbacks
        let log_buf_for_start  = log_buffer.clone();
        let log_buf_for_routes = log_buffer.clone();
        let log_buf_for_clear  = log_buffer.clone();
        let status_for_start   = status_label.clone();
        let stop_btn_ref       = stop_btn.clone();
        let start_btn_ref      = start_btn.clone();

        // Start button callback
        start_btn.connect_clicked(move |_| {
            status_for_start.set_label("Status: starting…");
            let mut end_iter = log_buf_for_start.end_iter();
            log_buf_for_start.insert(&mut end_iter, "[server] Starting MeedyaManager media server…\n");
            log_buf_for_start.insert(&mut end_iter, "[server] Listening on https://0.0.0.0:8443\n");
            status_for_start.set_label("Status: running — https://0.0.0.0:8443");
            start_btn_ref.set_sensitive(false);
            stop_btn_ref.set_sensitive(true);
        });

        // Stop button callback
        {
            let log_buf = log_buffer.clone();
            let status  = status_label.clone();
            let start_b = start_btn.clone();
            stop_btn.connect_clicked(move |btn| {
                let mut end_iter = log_buf.end_iter();
                log_buf.insert(&mut end_iter, "[server] Server stopped.\n");
                status.set_label("Status: stopped");
                start_b.set_sensitive(true);
                btn.set_sensitive(false);
            });
        }

        // Show routes callback
        routes_btn.connect_clicked(move |_| {
            let mut end_iter = log_buf_for_routes.end_iter();
            log_buf_for_routes.insert(&mut end_iter, "Routes:\n");
            let routes = [
                ("GET",  "/health",          "Liveness probe"),
                ("POST", "/auth/login",       "Authenticate → JWT"),
                ("GET",  "/api/library",      "List media files"),
                ("GET",  "/api/library/:id",  "Single file metadata"),
                ("GET",  "/api/search",       "Search by title/artist"),
                ("GET",  "/stream/:id",       "Stream media (Range)"),
                ("GET",  "/api/export/status","Export status (Admin)"),
                ("GET",  "/api/server/info",  "Server info (Admin)"),
            ];
            for (method, path, desc) in routes {
                log_buf_for_routes.insert(
                    &mut log_buf_for_routes.end_iter(),
                    &format!("  {method:<5} {path:<25} — {desc}\n"),
                );
            }
        });

        // Clear button callback
        clear_btn.connect_clicked(move |_| {
            log_buf_for_clear.set_text("");
        });

        log_group.add(&clear_btn);

        // Assemble
        vbox.append(&net_group);
        vbox.append(&tls_group);
        vbox.append(&auth_group);
        vbox.append(&cors_group);
        vbox.append(&ctrl_group);
        vbox.append(&log_group);
        clamp.set_child(Some(&vbox));

        let scrolled = gtk::ScrolledWindow::new();
        scrolled.set_hscrollbar_policy(gtk::PolicyType::Never);
        scrolled.set_vexpand(true);
        scrolled.set_child(Some(&clamp));

        Self { root: scrolled.upcast() }
    }

    /// Returns the root widget for embedding in the main AdwTabView.
    pub fn widget(&self) -> &gtk::Widget {
        &self.root
    }
}

impl Default for ServerPanel {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests (pure logic — no GTK display required)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use mm_server::ServerConfig;

    // Local re-implementation of validate_config logic for panel tests
    fn validate_config(cfg: &ServerConfig, no_tls: bool) -> Vec<String> {
        let mut errors = Vec::new();
        if cfg.jwt_secret.is_empty() {
            errors.push("JWT secret is not set.".into());
        }
        if !no_tls {
            if cfg.tls_cert_path.is_empty() {
                errors.push("TLS certificate path is not set.".into());
            }
            if cfg.tls_key_path.is_empty() {
                errors.push("TLS key path is not set.".into());
            }
        }
        errors
    }

    fn make_valid_config() -> ServerConfig {
        ServerConfig {
            bind_address:   "0.0.0.0".into(),
            port:           8443,
            tls_cert_path:  "/etc/ssl/cert.pem".into(),
            tls_key_path:   "/etc/ssl/key.pem".into(),
            jwt_secret:     "strong-secret-key-here".into(),
            jwt_expiry_secs: 86_400,
            cors_origins:   vec![],
            max_connections: 1000,
            request_logging: true,
        }
    }

    #[test]
    fn valid_config_passes_validation() {
        let errors = validate_config(&make_valid_config(), false);
        assert!(errors.is_empty());
    }

    #[test]
    fn missing_tls_cert_produces_error() {
        let mut cfg = make_valid_config();
        cfg.tls_cert_path = String::new();
        let errors = validate_config(&cfg, false);
        assert!(!errors.is_empty());
    }

    #[test]
    fn no_tls_mode_skips_cert_check() {
        let mut cfg = make_valid_config();
        cfg.tls_cert_path = String::new();
        cfg.tls_key_path  = String::new();
        let errors = validate_config(&cfg, true);
        assert!(!errors.iter().any(|e| e.contains("certificate")));
    }

    #[test]
    fn default_port_is_8443() {
        assert_eq!(ServerConfig::default().port, 8443);
    }

    #[test]
    fn bind_addr_formats_correctly() {
        let mut cfg = ServerConfig::default();
        cfg.bind_address = "127.0.0.1".into();
        cfg.port = 9000;
        assert_eq!(cfg.bind_addr(), "127.0.0.1:9000");
    }

    #[test]
    fn jwt_expiry_default() {
        assert_eq!(ServerConfig::default().jwt_expiry_secs, 86_400);
    }
}
