// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Cloud Storage Monitor View (macOS, M7)
//
// Displays connected cloud providers, their sync status, and lets users
// connect / disconnect each provider.  Monitoring of the configured root
// folder starts automatically when a provider is connected.
//
// Layout:
//   ┌──────────────────────────────────────────────────────────┐
//   │  [Provider row]  OneDrive   ● Synced     [Disconnect]    │
//   │  [Provider row]  Google Drive  ○ Not Connected [Connect] │
//   │  [Provider row]  Dropbox   ● Synced     [Disconnect]     │
//   │  [Provider row]  MEGA      — Coming Soon                  │
//   │  [Provider row]  iCloud    — macOS only                   │
//   │──────────────────────────────────────────────────────────│
//   │  Event log (last 50 events, scrollable)                   │
//   └──────────────────────────────────────────────────────────┘

import SwiftUI

// MARK: – CloudProviderEntry

/// View-model for a single cloud provider row.
struct CloudProviderEntry: Identifiable {
    let id        : String    // Internal identifier
    let label     : String    // Display name
    let iconName  : String    // SF Symbol icon
    var isConnected : Bool    // Whether the user is authenticated
    var syncStatus  : String  // Short status string
    var rootFolder  : String  // Cloud folder being monitored
    let isStub      : Bool    // Provider not yet implemented
}

// MARK: – CloudModel

/// Observable model owning the list of cloud provider states.
@Observable
final class CloudModel {

    // Live provider list (mirrors mm-cloud provider order)
    var providers: [CloudProviderEntry] = [
        CloudProviderEntry(
            id: "onedrive", label: "OneDrive",
            iconName: "cloud.fill", isConnected: false,
            syncStatus: "Not Connected", rootFolder: "/Music",
            isStub: false),
        CloudProviderEntry(
            id: "googledrive", label: "Google Drive",
            iconName: "cloud.fill", isConnected: false,
            syncStatus: "Not Connected", rootFolder: "/Music",
            isStub: false),
        CloudProviderEntry(
            id: "dropbox", label: "Dropbox",
            iconName: "archivebox.fill", isConnected: false,
            syncStatus: "Not Connected", rootFolder: "/Music",
            isStub: false),
        CloudProviderEntry(
            id: "mega", label: "MEGA",
            iconName: "externaldrive.fill", isConnected: false,
            syncStatus: "Coming Soon", rootFolder: "/",
            isStub: true),
        CloudProviderEntry(
            id: "icloud", label: "iCloud Drive",
            iconName: "icloud.fill", isConnected: false,
            syncStatus: "macOS native", rootFolder: "/",
            isStub: true),
    ]

    // Scrollable event log
    var eventLog: [String] = []

    // MARK: Actions

    /// Simulates connecting a provider (production wires into mm-cloud FFI).
    func connect(id: String) {
        guard let idx = providers.firstIndex(where: { $0.id == id }) else { return }
        guard !providers[idx].isStub else { return }
        providers[idx].isConnected = true
        providers[idx].syncStatus  = "Syncing…"
        appendEvent("[\(providers[idx].label)] Connecting…")
        // Simulate a successful sync after a brief delay.
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { [weak self] in
            guard let self else { return }
            if let i = self.providers.firstIndex(where: { $0.id == id }) {
                self.providers[i].syncStatus = "Synced"
                self.appendEvent("[\(self.providers[i].label)] Connected — folder: \(self.providers[i].rootFolder)")
            }
        }
    }

    /// Simulates disconnecting a provider.
    func disconnect(id: String) {
        guard let idx = providers.firstIndex(where: { $0.id == id }) else { return }
        providers[idx].isConnected = false
        providers[idx].syncStatus  = "Not Connected"
        appendEvent("[\(providers[idx].label)] Disconnected")
    }

    /// Appends a timestamped entry to the event log (capped at 50 entries).
    func appendEvent(_ message: String) {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss"
        let ts = formatter.string(from: Date())
        eventLog.insert("[\(ts)] \(message)", at: 0)
        if eventLog.count > 50 {
            eventLog = Array(eventLog.prefix(50))
        }
    }
}

// MARK: – CloudView

/// Top-level Cloud Storage Monitor view.
struct CloudView: View {

    @State private var model = CloudModel()

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // ── Header ────────────────────────────────────────────────────────
            HStack {
                Label("Cloud Storage Monitor", systemImage: "cloud.fill")
                    .font(.title2).bold()
                Spacer()
                Text("M7").foregroundStyle(.secondary).font(.caption)
            }
            .padding([.horizontal, .top])
            .padding(.bottom, 8)

            Divider()

            // ── Provider rows ─────────────────────────────────────────────────
            List(model.providers) { entry in
                CloudProviderRow(entry: entry) {
                    if entry.isConnected {
                        model.disconnect(id: entry.id)
                    } else {
                        model.connect(id: entry.id)
                    }
                }
            }
            .listStyle(.inset)
            .frame(minHeight: 220, maxHeight: 280)

            Divider()

            // ── Event log ─────────────────────────────────────────────────────
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text("Event Log")
                        .font(.headline)
                    Spacer()
                    Button("Clear") { model.eventLog.removeAll() }
                        .buttonStyle(.plain)
                        .foregroundStyle(.secondary)
                        .font(.caption)
                }
                .padding(.horizontal)
                .padding(.top, 8)

                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 2) {
                        ForEach(model.eventLog, id: \.self) { entry in
                            Text(entry)
                                .font(.system(.caption, design: .monospaced))
                                .foregroundStyle(.secondary)
                                .padding(.horizontal)
                        }
                    }
                    .padding(.bottom, 8)
                }
            }
            .frame(maxHeight: .infinity)
        }
    }
}

// MARK: – CloudProviderRow

/// A single row in the provider list.
private struct CloudProviderRow: View {
    let entry         : CloudProviderEntry
    let toggleAction  : () -> Void

    var body: some View {
        HStack(spacing: 12) {
            // Provider icon
            Image(systemName: entry.iconName)
                .frame(width: 24)
                .foregroundStyle(entry.isConnected ? .green : .secondary)

            // Provider name
            Text(entry.label)
                .frame(width: 120, alignment: .leading)

            // Status indicator dot + label
            HStack(spacing: 6) {
                Circle()
                    .fill(statusColor)
                    .frame(width: 8, height: 8)
                Text(entry.syncStatus)
                    .foregroundStyle(.secondary)
                    .font(.caption)
            }

            Spacer()

            // Connect / Disconnect button (hidden for stubs)
            if !entry.isStub {
                Button(entry.isConnected ? "Disconnect" : "Connect", action: toggleAction)
                    .buttonStyle(.borderless)
                    .controlSize(.small)
                    .foregroundStyle(entry.isConnected ? .red : .accentColor)
            } else {
                Text("—")
                    .foregroundStyle(.tertiary)
                    .font(.caption)
            }
        }
        .padding(.vertical, 4)
    }

    private var statusColor: Color {
        if entry.isStub               { return .gray   }
        if entry.syncStatus == "Synced" { return .green  }
        if entry.syncStatus.hasPrefix("Syncing") { return .orange }
        return .secondary
    }
}

// MARK: – Preview

#Preview {
    CloudView()
        .frame(width: 700, height: 600)
}
