// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Database Export View (macOS, M9)
//
// Provides a SwiftUI interface for exporting the scanned media library to a
// relational database using the `mm-export` Rust crate (via FFI, wired in M9+).
//
// For M9 the export itself is simulated (no real DB connection); the full UI
// skeleton is wired and ready for the FFI layer.

import SwiftUI

// MARK: – Model

/// Database backend options.
enum ExportBackend: String, CaseIterable, Identifiable {
    case sqlite   = "SQLite"
    case mysql    = "MySQL"
    case mariadb  = "MariaDB"
    case postgres = "PostgreSQL"
    case mssql    = "SQL Server"

    var id: String { rawValue }

    /// Example DSN shown as placeholder text.
    var exampleDSN: String {
        switch self {
        case .sqlite:   return "sqlite:///Users/you/library.db"
        case .mysql:    return "mysql://user:pass@localhost/meedya"
        case .mariadb:  return "mysql://user:pass@localhost/meedya"
        case .postgres: return "postgres://user:pass@localhost/meedya"
        case .mssql:    return "server=tcp:host,1433;database=meedya;user=sa;password=P"
        }
    }
}

/// Observable model driving the Export view.
@Observable
final class ExportModel {

    // ── User inputs ──────────────────────────────────────────────────────────
    /// Selected database backend
    var backend: ExportBackend = .sqlite
    /// Connection string / DSN
    var connectionString: String = ""
    /// Table name prefix
    var tablePrefix: String = "mm_"
    /// Batch size for DB transactions
    var batchSize: Int = 500
    /// Dry run — no DB writes
    var dryRun: Bool = false

    // ── State ────────────────────────────────────────────────────────────────
    /// "idle" | "exporting" | "done" | "error"
    var exportStatus: String = "idle"
    /// Human-readable result message
    var resultMessage: String = ""
    /// Log lines accumulated during an export run
    var logLines: [String] = []

    // ── Computed ─────────────────────────────────────────────────────────────
    var isExporting: Bool { exportStatus == "exporting" }

    // ── Actions ──────────────────────────────────────────────────────────────

    /// Simulate running an export against the configured backend.
    ///
    /// In production this calls `mm_ffi_export()` via the FFI bridge.
    func runExport() {
        guard !connectionString.trimmingCharacters(in: .whitespaces).isEmpty else {
            exportStatus  = "error"
            resultMessage = "Please enter a connection string before exporting."
            return
        }

        exportStatus = "exporting"
        appendLog("Starting export to \(backend.rawValue)…")
        appendLog("DSN length: \(connectionString.count) chars")
        appendLog("Table prefix: \(tablePrefix)")
        appendLog("Dry run: \(dryRun)")

        // Simulate async export with a short delay
        DispatchQueue.global().asyncAfter(deadline: .now() + 1.2) { [self] in
            // M9 stub — no real DB writes
            DispatchQueue.main.async { [self] in
                self.appendLog("Export complete (stub — no DB writes in M9).")
                self.exportStatus  = "done"
                self.resultMessage = dryRun
                    ? "Dry-run complete. No rows written."
                    : "Export finished: 0 inserted, 0 updated, 0 skipped."
            }
        }
    }

    /// Append a schema preview (DDL stubs) to the log.
    func showSchema() {
        appendLog("--- Schema DDL preview (\(backend.rawValue)) ---")
        appendLog("CREATE TABLE IF NOT EXISTS \(tablePrefix)files ( … );")
        appendLog("CREATE TABLE IF NOT EXISTS \(tablePrefix)tags  ( … );")
        appendLog("CREATE TABLE IF NOT EXISTS \(tablePrefix)history ( … );")
        appendLog("Full DDL available via: meedya export --show-schema --db <DSN>")
    }

    /// Clear all log lines.
    func clearLog() {
        logLines.removeAll()
        exportStatus  = "idle"
        resultMessage = ""
    }

    // ── Private ──────────────────────────────────────────────────────────────

    private func appendLog(_ line: String) {
        let ts = DateFormatter.localizedString(from: Date(), dateStyle: .none, timeStyle: .medium)
        logLines.append("[\(ts)] \(line)")
        // Keep last 200 lines to avoid unbounded growth
        if logLines.count > 200 { logLines.removeFirst(logLines.count - 200) }
    }
}

// MARK: – Main View

/// Database Export panel — lets users configure and run a library export.
struct ExportView: View {

    @State private var model = ExportModel()

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {

                // ── Backend ────────────────────────────────────────────────
                SettingsGroupExport(title: "Database Backend") {
                    HStack {
                        Text("Backend")
                        Spacer()
                        Picker("Backend", selection: $model.backend) {
                            ForEach(ExportBackend.allCases) { b in
                                Text(b.rawValue).tag(b)
                            }
                        }
                        .labelsHidden()
                        .frame(width: 150)
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                }

                // ── Connection ─────────────────────────────────────────────
                SettingsGroupExport(title: "Connection") {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Connection String")
                            .font(.subheadline)

                        // Multi-line text field so long SQL Server ADO strings fit
                        TextEditor(text: $model.connectionString)
                            .font(.system(.caption, design: .monospaced))
                            .frame(minHeight: 56, maxHeight: 80)
                            .overlay(
                                RoundedRectangle(cornerRadius: 4)
                                    .stroke(.separator, lineWidth: 0.5)
                            )

                        Text(model.backend.exampleDSN)
                            .font(.caption2)
                            .foregroundStyle(.tertiary)
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)

                    Divider().padding(.leading, 16)

                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Table prefix")
                            Text("Prefix applied to all created tables (default: mm_)")
                                .font(.caption).foregroundStyle(.secondary)
                        }
                        Spacer()
                        TextField("mm_", text: $model.tablePrefix)
                            .multilineTextAlignment(.trailing)
                            .frame(width: 100)
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)

                    Divider().padding(.leading, 16)

                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Batch size")
                            Text("Rows per database transaction")
                                .font(.caption).foregroundStyle(.secondary)
                        }
                        Spacer()
                        Stepper(value: $model.batchSize, in: 50...2000, step: 50) {
                            Text("\(model.batchSize)")
                                .monospacedDigit()
                                .frame(width: 60, alignment: .trailing)
                        }
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)

                    Divider().padding(.leading, 16)

                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Dry run")
                            Text("Preview export without writing to the database.")
                                .font(.caption).foregroundStyle(.secondary)
                        }
                        Spacer()
                        Toggle("", isOn: $model.dryRun).labelsHidden().toggleStyle(.switch)
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                }

                // ── Actions ────────────────────────────────────────────────
                HStack(spacing: 8) {
                    Button("Show Schema DDL") { model.showSchema() }
                        .buttonStyle(.bordered)

                    Spacer()

                    if model.isExporting { ProgressView().controlSize(.small) }

                    Button("Export Library") { model.runExport() }
                        .buttonStyle(.borderedProminent)
                        .disabled(model.isExporting)
                }
                .padding(.bottom, 8)

                // ── Status ─────────────────────────────────────────────────
                if !model.resultMessage.isEmpty {
                    Text(model.resultMessage)
                        .font(.caption)
                        .foregroundStyle(model.exportStatus == "error" ? .red : .green)
                        .padding(.bottom, 6)
                }

                // ── Log ────────────────────────────────────────────────────
                SettingsGroupExport(title: "Export Log") {
                    ScrollView(.vertical) {
                        VStack(alignment: .leading, spacing: 2) {
                            if model.logLines.isEmpty {
                                Text("No log entries yet.")
                                    .font(.system(.caption, design: .monospaced))
                                    .foregroundStyle(.secondary)
                            } else {
                                ForEach(model.logLines, id: \.self) { line in
                                    Text(line)
                                        .font(.system(.caption, design: .monospaced))
                                        .frame(maxWidth: .infinity, alignment: .leading)
                                }
                            }
                        }
                        .padding(8)
                    }
                    .frame(minHeight: 120, maxHeight: 200)
                    .background(.background.secondary)
                    .clipShape(RoundedRectangle(cornerRadius: 6))
                    .padding(.horizontal, 12)
                    .padding(.bottom, 12)
                }

                HStack {
                    Spacer()
                    Button("Clear Log") { model.clearLog() }
                        .buttonStyle(.plain)
                        .foregroundStyle(.secondary)
                        .controlSize(.small)
                }
                .padding(.bottom, 20)
            }
            .padding(.horizontal, 16)
            .padding(.top, 16)
        }
        .navigationTitle("Export")
    }
}

// MARK: – Reusable group (private, avoids name clash with SettingsView)

private struct SettingsGroupExport<Content: View>: View {
    let title: String
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(title).font(.headline).padding(.horizontal, 4).padding(.bottom, 6)
            VStack(alignment: .leading, spacing: 0) { content() }
                .background(.background)
                .clipShape(RoundedRectangle(cornerRadius: 8))
                .overlay(RoundedRectangle(cornerRadius: 8).stroke(.separator, lineWidth: 0.5))
        }
        .padding(.bottom, 20)
    }
}

// MARK: – Preview

#Preview("Export panel") {
    ExportView()
        .environment(AppState())
        .frame(width: 560, height: 700)
}
