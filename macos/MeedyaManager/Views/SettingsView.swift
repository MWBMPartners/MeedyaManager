// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Settings View (macOS)
//
// Displays application preferences mirroring the JSON5 config file:
//   - General: dry-run mode toggle
//   - Watching: recursive scan toggle, debounce interval
//   - Logging: log level picker, PII redaction toggle
//   - About: core version, config file path, open-in-Finder button
//
// Full settings editing (save back to settings.json5) is planned for M6.
// This M4 view is read-only with clearly labelled stub indicators.

import SwiftUI

// MARK: – Main Settings View

/// Preferences panel hosted in the Settings scene (⌘,) and the sidebar tab.
struct SettingsView: View {

    @Environment(AppState.self) private var appState

    // Local mirror of config values — sourced from MmCore stub in M4
    @State private var dryRun:        Bool   = false
    @State private var recursive:     Bool   = true
    @State private var debounceMs:    Int    = 500
    @State private var logLevel:      String = "info"
    @State private var redactPii:     Bool   = true

    // Raw JSON5 config text for the read-only preview panel
    @State private var rawConfigText: String = ""

    // Config file path reported by the core
    private let configFilePath = MmCore.shared.configPath()

    // Available log levels (mirrors LogLevel enum in mm-core)
    private let logLevels = ["error", "warn", "info", "debug", "trace"]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {

                // ── General ──────────────────────────────────────────────────
                SettingsGroup(title: "General") {
                    SettingsToggleRow(
                        label:    "Dry-run mode",
                        detail:   "Preview renames without writing any changes to disk.",
                        isOn:     $dryRun
                    )
                }

                // ── Watching ─────────────────────────────────────────────────
                SettingsGroup(title: "Watching") {
                    SettingsToggleRow(
                        label:   "Recursive scan",
                        detail:  "Include sub-directories when scanning a folder.",
                        isOn:    $recursive
                    )

                    Divider().padding(.leading, 16)

                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Debounce interval")
                            Text("Milliseconds to wait before processing a file-system event.")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        // Stepper clamped to 50–5000 ms in 50 ms steps
                        Stepper(
                            value:  $debounceMs,
                            in:     50...5000,
                            step:   50
                        ) {
                            Text("\(debounceMs) ms")
                                .monospacedDigit()
                                .frame(width: 72, alignment: .trailing)
                        }
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                }

                // ── Logging ──────────────────────────────────────────────────
                SettingsGroup(title: "Logging") {
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text("Log level")
                            Text("Controls verbosity of the structured log output.")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        Picker("Log level", selection: $logLevel) {
                            ForEach(logLevels, id: \.self) { level in
                                Text(level).tag(level)
                            }
                        }
                        .labelsHidden()
                        .frame(width: 100)
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)

                    Divider().padding(.leading, 16)

                    SettingsToggleRow(
                        label:   "Redact PII in logs",
                        detail:  "Replace file paths and tag values with '[REDACTED]' in log output.",
                        isOn:    $redactPii
                    )
                }

                // ── Config File ───────────────────────────────────────────────
                SettingsGroup(title: "Config File") {
                    VStack(alignment: .leading, spacing: 6) {
                        HStack(alignment: .firstTextBaseline) {
                            Text("Path")
                                .foregroundStyle(.secondary)
                                .frame(width: 48, alignment: .trailing)
                            Text(configFilePath)
                                .font(.system(.caption, design: .monospaced))
                                .textSelection(.enabled)
                                .lineLimit(3)
                        }

                        HStack(spacing: 8) {
                            Spacer()
                            // Open the containing folder in Finder
                            Button("Show in Finder") {
                                openInFinder(path: configFilePath)
                            }
                            .controlSize(.small)

                            // Copy path to clipboard
                            Button("Copy Path") {
                                NSPasteboard.general.clearContents()
                                NSPasteboard.general.setString(configFilePath, forType: .string)
                            }
                            .controlSize(.small)
                        }
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                }

                // ── Raw Config Preview ────────────────────────────────────────
                SettingsGroup(title: "Raw Config (Read-only)") {
                    ScrollView(.vertical) {
                        Text(rawConfigText.isEmpty ? "Config file not found — defaults are in use." : rawConfigText)
                            .font(.system(.caption, design: .monospaced))
                            .foregroundStyle(rawConfigText.isEmpty ? .secondary : .primary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(12)
                            .textSelection(.enabled)
                    }
                    .frame(minHeight: 120, maxHeight: 240)
                    .background(.background.secondary)
                    .clipShape(RoundedRectangle(cornerRadius: 6))
                    .padding(.horizontal, 12)
                    .padding(.bottom, 12)
                }

                // ── M6 Notice ────────────────────────────────────────────────
                Label("Settings editing (save to JSON5) coming in M6", systemImage: "hammer.fill")
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.vertical, 16)

                // ── Core Version ─────────────────────────────────────────────
                Text("MeedyaManager core \(appState.coreVersion)")
                    .font(.caption2)
                    .foregroundStyle(.tertiary)
                    .frame(maxWidth: .infinity, alignment: .center)
                    .padding(.bottom, 20)
            }
            .padding(.horizontal, 16)
            .padding(.top, 16)
        }
        .navigationTitle("Settings")
        .onAppear { loadConfig() }
    }

    // MARK: – Helpers

    /// Load the raw config text from disk (best-effort; silently ignores errors).
    private func loadConfig() {
        let url = URL(fileURLWithPath: configFilePath)
        rawConfigText = (try? String(contentsOf: url, encoding: .utf8)) ?? ""
    }

    /// Reveal the given path in Finder (opens the parent directory and selects the file).
    private func openInFinder(path: String) {
        let url = URL(fileURLWithPath: path)
        NSWorkspace.shared.activateFileViewerSelecting([url])
    }
}

// MARK: – Reusable Settings Components

/// A titled card that groups related settings rows.
private struct SettingsGroup<Content: View>: View {
    let title:   String
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(title)
                .font(.headline)
                .padding(.horizontal, 4)
                .padding(.bottom, 6)

            VStack(alignment: .leading, spacing: 0) {
                content()
            }
            .background(.background)
            .clipShape(RoundedRectangle(cornerRadius: 8))
            .overlay(
                RoundedRectangle(cornerRadius: 8)
                    .stroke(.separator, lineWidth: 0.5)
            )
        }
        .padding(.bottom, 20)
    }
}

/// A single toggle row with a label and secondary detail line.
private struct SettingsToggleRow: View {
    let label:  String
    let detail: String
    @Binding var isOn: Bool

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(label)
                Text(detail)
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Toggle("", isOn: $isOn)
                .labelsHidden()
                .toggleStyle(.switch)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
    }
}

// MARK: – Preview

#Preview("Settings — sidebar") {
    SettingsView()
        .environment(AppState())
        .frame(width: 560, height: 700)
}

#Preview("Settings — preferences window") {
    SettingsView()
        .environment(AppState())
        .frame(width: 600, height: 400)
}
