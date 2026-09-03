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
/// @MainActor: state is read by SwiftUI views (which run on MainActor) and
/// mutated by user actions; isolating to MainActor satisfies Swift 6 strict
/// concurrency without adding capture-list ceremony at every call site.
@MainActor
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
    //
    // Connecting and disconnecting a provider are disabled in this alpha —
    // mm-cloud does not yet make a real network call for any provider, so
    // there is nothing genuine for Connect/Disconnect to do. The button in
    // the row view below is permanently disabled and no longer calls into
    // this model; see issue #205.
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

            // Persistent notice — mm-cloud does not yet make a real network
            // call for any provider. See issue #205.
            AlphaPreviewBanner()
                .padding(.horizontal)
                .padding(.bottom, 8)

            Divider()

            // ── Provider rows ─────────────────────────────────────────────────
            // Connect/Disconnect is disabled in this alpha (see CloudProviderRow),
            // so the toggle action below is never invoked.
            List(model.providers) { entry in
                CloudProviderRow(entry: entry) {}
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
                        .accessibilityLabel("Clear event log")
                        .accessibilityHint("Removes all cloud storage events from the log")
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

// MARK: – AlphaPreviewBanner (persistent "not functional" notice for this tab)

/// A persistent, prominent notice that this tab does not perform any real
/// action in this alpha build. mm-cloud does not yet make a real network
/// call for any provider, so nothing behind this tab can actually sync.
/// See issue #205.
private struct AlphaPreviewBanner: View {
    var body: some View {
        Label(
            "Preview — not functional in this alpha. Nothing is started, exported or synced. (issue #205)",
            systemImage: "exclamationmark.triangle.fill"
        )
        .font(.callout)
        .foregroundStyle(.orange)
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color.orange.opacity(0.15))
        .clipShape(RoundedRectangle(cornerRadius: 8))
        .accessibilityElement(children: .combine)
    }
}

// MARK: – CloudProviderRow

/// A single row in the provider list.
private struct CloudProviderRow: View {
    let entry         : CloudProviderEntry
    let toggleAction  : () -> Void

    var body: some View {
        HStack(spacing: 12) {
            // Provider icon — decorative, label provided by row accessibilityElement
            Image(systemName: entry.iconName)
                .frame(width: 24)
                .foregroundStyle(entry.isConnected ? .green : .secondary)
                .accessibilityHidden(true)

            // Provider name
            Text(entry.label)
                .frame(width: 120, alignment: .leading)

            // Status indicator dot + label
            HStack(spacing: 6) {
                // Decorative dot — status already announced via text
                Circle()
                    .fill(statusColor)
                    .frame(width: 8, height: 8)
                    .accessibilityHidden(true)
                Text(entry.syncStatus)
                    .foregroundStyle(.secondary)
                    .font(.caption)
            }

            Spacer()

            // Connect / Disconnect button (hidden for stubs) — disabled in this
            // alpha, since mm-cloud does not yet make a real network call for
            // any provider. See the banner above and issue #205.
            if !entry.isStub {
                Button(entry.isConnected ? "Disconnect" : "Connect", action: toggleAction)
                    .buttonStyle(.borderless)
                    .controlSize(.small)
                    .disabled(true)
                    .foregroundStyle(entry.isConnected ? .red : .accentColor)
                    .accessibilityLabel("\(entry.isConnected ? "Disconnect" : "Connect") \(entry.label) (disabled — not functional in this alpha)")
                    .accessibilityHint("Connecting \(entry.label) is disabled in this alpha; mm-cloud does not yet make a real network call. See issue #205.")
            } else {
                Text("—")
                    .foregroundStyle(.tertiary)
                    .font(.caption)
                    .accessibilityHidden(true)
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
