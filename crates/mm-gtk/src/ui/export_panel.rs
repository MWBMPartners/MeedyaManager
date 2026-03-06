// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Database Export Panel (GTK4, M9)
//
// Provides a GUI front-end for `mm-export`:
//   - Backend picker (SQLite / MySQL / MariaDB / PostgreSQL / SQL Server)
//   - DSN / connection string input
//   - Table prefix input
//   - "Export" and "Show Schema" buttons
//   - Progress + status label
//   - Scrollable log area showing export results
//
// The actual export is dispatched via `meedya export` CLI arguments and
// the `mm-export` crate.  For M9 the database write is stubbed; the UI
// skeleton is fully wired.

use gtk4 as gtk;
use gtk::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::ui::accessibility;

// ─── Backend option ──────────────────────────────────────────────────────────

/// A single database backend option displayed in the backend picker.
struct BackendOption {
    /// Identifier used when building the DSN
    id:      &'static str,
    /// Human-readable label
    label:   &'static str,
    /// DSN placeholder shown in the connection field
    example: &'static str,
}

/// All five supported backends.
const BACKENDS: &[BackendOption] = &[
    BackendOption { id: "sqlite",   label: "SQLite",       example: "sqlite:///home/user/library.db" },
    BackendOption { id: "mysql",    label: "MySQL",        example: "mysql://user:pass@localhost/meedya" },
    BackendOption { id: "mariadb",  label: "MariaDB",      example: "mysql://user:pass@localhost/meedya" },
    BackendOption { id: "postgres", label: "PostgreSQL",   example: "postgres://user:pass@localhost/meedya" },
    BackendOption { id: "mssql",    label: "SQL Server",   example: "server=tcp:host,1433;database=meedya;user=sa;password=P" },
];

// ─── ExportPanel ─────────────────────────────────────────────────────────────

/// GTK4 widget group for the Database Export tab.
pub struct ExportPanel {
    /// Root widget returned to the tab view
    root: gtk::Widget,
}

impl ExportPanel {
    /// Build the export panel and return an `ExportPanel` owning its state.
    pub fn new() -> Self {
        // ── Top-level scrollable container ───────────────────────────────────
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .build();

        let clamp = adw::Clamp::builder()
            .maximum_size(720)
            .tightening_threshold(600)
            .build();
        scroll.set_child(Some(&clamp));

        let outer_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(20)
            .margin_top(20)
            .margin_bottom(20)
            .margin_start(16)
            .margin_end(16)
            .build();
        clamp.set_child(Some(&outer_box));

        // ── Backend selection group ──────────────────────────────────────────
        let backend_group = adw::PreferencesGroup::new();
        backend_group.set_title("Database Backend");
        backend_group.set_description(Some(
            "Choose the target database engine for your media library export."
        ));

        // Backend picker — ComboBoxText with one entry per supported backend
        let backend_combo = gtk::ComboBoxText::new();
        for b in BACKENDS {
            backend_combo.append(Some(b.id), b.label);
        }
        backend_combo.set_active_id(Some("sqlite")); // default to SQLite
        accessibility::set_label(&backend_combo, "Database backend");
        accessibility::set_description(&backend_combo, "Select the database engine to export your media library to.");

        let backend_row = adw::ActionRow::builder()
            .title("Backend")
            .subtitle("Select the database engine")
            .build();
        backend_row.add_suffix(&backend_combo);
        backend_group.add(&backend_row);

        outer_box.append(&backend_group);

        // ── Connection settings group ────────────────────────────────────────
        let conn_group = adw::PreferencesGroup::new();
        conn_group.set_title("Connection");

        // DSN / connection string entry
        let dsn_entry = gtk::Entry::builder()
            .placeholder_text("sqlite:///home/user/library.db")
            .hexpand(true)
            .build();
        accessibility::set_label(&dsn_entry, "Connection string");
        accessibility::set_description(&dsn_entry, "Enter the DSN connection string for the selected database backend.");

        // Update placeholder when backend changes
        {
            let entry_clone = dsn_entry.clone();
            backend_combo.connect_changed(move |combo| {
                if let Some(id) = combo.active_id() {
                    if let Some(b) = BACKENDS.iter().find(|b| b.id == id.as_str()) {
                        entry_clone.set_placeholder_text(Some(b.example));
                    }
                }
            });
        }

        let dsn_row = adw::ActionRow::builder()
            .title("Connection string")
            .subtitle("Database DSN or ADO connection string")
            .build();
        dsn_row.add_suffix(&dsn_entry);
        conn_group.add(&dsn_row);

        // Table prefix entry
        let prefix_entry = gtk::Entry::builder()
            .text("mm_")
            .max_length(32)
            .build();
        accessibility::set_label(&prefix_entry, "Table prefix");
        accessibility::set_description(&prefix_entry, "Prefix applied to all created database tables.");

        let prefix_row = adw::ActionRow::builder()
            .title("Table prefix")
            .subtitle("Prefix for all created tables (default: mm_)")
            .build();
        prefix_row.add_suffix(&prefix_entry);
        conn_group.add(&prefix_row);

        outer_box.append(&conn_group);

        // ── Action buttons ────────────────────────────────────────────────────
        let btn_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::End)
            .build();

        let schema_btn = gtk::Button::builder()
            .label("Show Schema DDL")
            .tooltip_text("Preview the CREATE TABLE statements without running the export")
            .build();
        schema_btn.add_css_class("pill");
        accessibility::set_label(&schema_btn, "Show schema DDL");
        accessibility::set_description(&schema_btn, "Displays the database schema SQL statements in the export log.");

        let export_btn = gtk::Button::builder()
            .label("Export Library")
            .tooltip_text("Export scanned media metadata to the configured database")
            .build();
        export_btn.add_css_class("pill");
        export_btn.add_css_class("suggested-action");
        accessibility::set_label(&export_btn, "Export library");
        accessibility::set_description(&export_btn, "Exports your media library to the configured database backend.");

        btn_box.append(&schema_btn);
        btn_box.append(&export_btn);
        outer_box.append(&btn_box);

        // ── Status label ─────────────────────────────────────────────────────
        let status_label = gtk::Label::builder()
            .label("Ready. Configure a connection string and click Export Library.")
            .wrap(true)
            .xalign(0.0)
            .build();
        status_label.add_css_class("caption");
        accessibility::set_label(&status_label, "Export status");
        outer_box.append(&status_label);

        // ── Log / results area ────────────────────────────────────────────────
        let log_group = adw::PreferencesGroup::new();
        log_group.set_title("Export Log");

        let log_buffer = gtk::TextBuffer::new(None);
        log_buffer.set_text("Export log will appear here.\n");

        let log_view = gtk::TextView::builder()
            .buffer(&log_buffer)
            .editable(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::Word)
            .build();

        let log_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .min_content_height(160)
            .child(&log_view)
            .build();
        log_scroll.add_css_class("card");

        log_group.add(&log_scroll);
        outer_box.append(&log_group);

        // Clear log button
        let clear_btn = gtk::Button::builder()
            .label("Clear Log")
            .halign(gtk::Align::End)
            .build();
        clear_btn.add_css_class("flat");
        accessibility::set_label(&clear_btn, "Clear export log");
        accessibility::set_description(&clear_btn, "Removes all log entries and resets the export status.");
        {
            let buf = log_buffer.clone();
            clear_btn.connect_clicked(move |_| {
                buf.set_text("");
            });
        }
        outer_box.append(&clear_btn);

        // ── Wire up Export button ─────────────────────────────────────────────
        {
            let dsn    = dsn_entry.clone();
            let prefix = prefix_entry.clone();
            let combo  = backend_combo.clone();
            let status = status_label.clone();
            let buf    = log_buffer.clone();

            export_btn.connect_clicked(move |_| {
                let dsn_val    = dsn.text().to_string();
                let prefix_val = prefix.text().to_string();
                let backend_id = combo.active_id().map(|s| s.to_string())
                    .unwrap_or_else(|| "sqlite".to_string());

                if dsn_val.trim().is_empty() {
                    status.set_text("⚠ Please enter a connection string before exporting.");
                    return;
                }

                // Append to log
                let entry = format!(
                    "[Export] backend={backend_id} prefix={prefix_val} dsn_len={}\n",
                    dsn_val.len()
                );
                let mut end = buf.end_iter();
                buf.insert(&mut end, &entry);

                // M9 stub: real export dispatched via mm-export backend
                status.set_text("✓ Export completed (stub — no DB writes in M9 UI). Check log for details.");
            });
        }

        // ── Wire up Schema DDL button ─────────────────────────────────────────
        {
            let prefix = prefix_entry.clone();
            let combo  = backend_combo.clone();
            let buf    = log_buffer.clone();
            let status = status_label.clone();

            schema_btn.connect_clicked(move |_| {
                use mm_export::{ExportConfig, SchemaBuilder, DbDialect};

                let backend_id = combo.active_id().map(|s| s.to_string())
                    .unwrap_or_else(|| "sqlite".to_string());
                let dialect = match backend_id.as_str() {
                    "mysql"    => DbDialect::MySql,
                    "mariadb"  => DbDialect::MariaDb,
                    "postgres" => DbDialect::Postgres,
                    "mssql"    => DbDialect::SqlServer,
                    _          => DbDialect::Sqlite,
                };

                let mut cfg = ExportConfig::with_dsn("stub://");
                cfg.table_prefix = prefix.text().to_string();

                let ddl = SchemaBuilder::new(dialect, &cfg).all_ddl();
                let mut end = buf.end_iter();
                buf.insert(&mut end, "\n--- Schema DDL ---\n");
                for stmt in &ddl {
                    buf.insert(&mut end, stmt);
                    buf.insert(&mut end, "\n\n");
                }
                status.set_text("Schema DDL appended to log.");
            });
        }

        Self {
            root: scroll.upcast(),
        }
    }

    /// Returns the root widget to be added to the AdwTabView.
    pub fn widget(&self) -> &gtk::Widget {
        &self.root
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backends_array_has_five_entries() {
        assert_eq!(BACKENDS.len(), 5);
    }

    #[test]
    fn all_backends_have_non_empty_ids() {
        for b in BACKENDS { assert!(!b.id.is_empty()); }
    }

    #[test]
    fn all_backends_have_non_empty_labels() {
        for b in BACKENDS { assert!(!b.label.is_empty()); }
    }

    #[test]
    fn all_backends_have_example_dsns() {
        for b in BACKENDS { assert!(!b.example.is_empty(), "missing example for {}", b.id); }
    }

    #[test]
    fn sqlite_is_first_backend() {
        assert_eq!(BACKENDS[0].id, "sqlite");
    }

    #[test]
    fn backend_ids_are_unique() {
        let ids: Vec<&str> = BACKENDS.iter().map(|b| b.id).collect();
        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "backend IDs must be unique");
    }

    #[test]
    fn backend_labels_are_unique() {
        let labels: Vec<&str> = BACKENDS.iter().map(|b| b.label).collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len());
    }
}
