// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Settings View (macOS, M6)
//
// Displays application preferences mirroring the JSON5 config file:
//   - General: dry-run mode toggle
//   - Watching: recursive scan toggle, debounce interval
//   - Logging: log level picker, PII redaction toggle
//   - About: core version, config file path, open-in-Finder button
//
// Settings are editable and saved to disk as JSON (M6).

import SwiftUI

// MARK: – Main Settings View

/// Preferences panel hosted in the Settings scene (⌘,) and the sidebar tab.
struct SettingsView: View {

    @Environment(AppState.self) private var appState

    // Local mirror of config values — editable in M6
    @State private var dryRun:        Bool   = false
    @State private var recursive:     Bool   = true
    @State private var debounceMs:    Int    = 500
    @State private var logLevel:      String = "info"
    @State private var redactPii:     Bool   = true

    // Raw config text for the preview panel (refreshed after save)
    @State private var rawConfigText: String = ""

    // Status message shown below the save button
    @State private var saveStatus: String = ""

    // True between save start and completion for the progress indicator
    @State private var isSaving: Bool = false

    // ── Update checker state ──────────────────────────────────────────────────
    // "idle" | "checking" | "up-to-date" | "available:<version>"
    @State private var updateStatus: String = "idle"

    // ── Test Mode state ───────────────────────────────────────────────────────

    // Controls the confirmation alert shown when disabling test mode with staged files
    @State private var showTestModeConfirm: Bool = false

    // True while a commit or revert operation is in progress
    @State private var isTestModeProcessing: Bool = false

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

                // ── Test Mode ──────────────────────────────────────────────────
                SettingsGroup(title: "Test Mode") {
                    // Toggle to enable/disable test mode
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            // Primary label for the test mode toggle
                            Text("Test Mode")
                            // Description explaining what test mode does
                            Text("When enabled, renames and tag writes go to a staging area instead of modifying real files.")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        // The actual toggle switch for test mode
                        Toggle("", isOn: Binding(
                            // Read the current value from appState
                            get: { appState.testModeEnabled },
                            // Handle the toggle change with confirmation logic
                            set: { newValue in
                                if !newValue && appState.testModeFileCount > 0 {
                                    // User is turning OFF test mode but has staged files — ask what to do
                                    showTestModeConfirm = true
                                } else {
                                    // Safe to toggle directly (either enabling, or disabling with no staged files)
                                    MmCore.shared.setTestMode(enabled: newValue)
                                    appState.testModeEnabled = newValue
                                    // Refresh the staged file count after toggling
                                    appState.testModeFileCount = MmCore.shared.testModeFileCount()
                                }
                            }
                        ))
                        .labelsHidden()
                        .toggleStyle(.switch)
                        .accessibilityLabel("Test Mode")
                        .accessibilityHint("When enabled, file operations go to a staging area instead of modifying real files")
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)

                    // Show the staged file count when test mode is active
                    if appState.testModeEnabled {
                        Divider().padding(.leading, 16)

                        // Staged file count indicator
                        HStack {
                            // File count label with dynamic icon
                            Label(
                                "\(appState.testModeFileCount) file\(appState.testModeFileCount == 1 ? "" : "s") staged",
                                systemImage: appState.testModeFileCount > 0
                                    ? "doc.on.doc.fill"
                                    : "doc"
                            )
                            .font(.subheadline)
                            .foregroundStyle(appState.testModeFileCount > 0 ? .primary : .secondary)

                            Spacer()

                            // Show processing spinner during commit/revert
                            if isTestModeProcessing {
                                ProgressView()
                                    .controlSize(.small)
                                    .padding(.trailing, 4)
                            }
                        }
                        .padding(.horizontal, 16)
                        .padding(.vertical, 8)

                        // Commit and Revert buttons — only visible when staged files exist
                        if appState.testModeFileCount > 0 {
                            Divider().padding(.leading, 16)

                            HStack(spacing: 12) {
                                Spacer()

                                // Revert button — discards all staged changes
                                Button(role: .destructive) {
                                    revertTestModeFiles()
                                } label: {
                                    Label("Revert", systemImage: "arrow.uturn.backward")
                                }
                                .controlSize(.small)
                                .disabled(isTestModeProcessing)
                                .accessibilityLabel("Revert staged files")
                                .accessibilityHint("Discards all staged test-mode changes without applying them")

                                // Commit button — applies all staged changes to real files
                                Button {
                                    commitTestModeFiles()
                                } label: {
                                    Label("Commit", systemImage: "checkmark.circle")
                                }
                                .controlSize(.small)
                                .buttonStyle(.borderedProminent)
                                .disabled(isTestModeProcessing)
                                .accessibilityLabel("Commit staged files")
                                .accessibilityHint("Applies all staged test-mode changes to real files")
                            }
                            .padding(.horizontal, 16)
                            .padding(.vertical, 8)
                        }
                    }
                }
                // Confirmation alert when disabling test mode with uncommitted staged files
                .alert(
                    "Uncommitted Test Files",
                    isPresented: $showTestModeConfirm
                ) {
                    // "Yes" commits the staged files, then disables test mode
                    Button("Yes — Commit") {
                        commitTestModeFiles()
                        MmCore.shared.setTestMode(enabled: false)
                        appState.testModeEnabled = false
                    }
                    // "No" reverts the staged files, then disables test mode
                    Button("No — Revert", role: .destructive) {
                        revertTestModeFiles()
                        MmCore.shared.setTestMode(enabled: false)
                        appState.testModeEnabled = false
                    }
                    // Cancel keeps test mode enabled
                    Button("Cancel", role: .cancel) { }
                } message: {
                    Text("You have \(appState.testModeFileCount) staged file\(appState.testModeFileCount == 1 ? "" : "s"). Would you like to commit them to disk or revert?")
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
                        .accessibilityLabel("Debounce interval")
                        .accessibilityHint("Milliseconds to wait before processing a file-system event. Range: 50 to 5000.")
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
                        .accessibilityLabel("Log level")
                        .accessibilityHint("Controls the verbosity of structured log output")
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
                            .accessibilityLabel("Show config file in Finder")
                            .accessibilityHint("Opens the Finder and selects the settings file")

                            // Copy path to clipboard
                            Button("Copy Path") {
                                NSPasteboard.general.clearContents()
                                NSPasteboard.general.setString(configFilePath, forType: .string)
                            }
                            .controlSize(.small)
                            .accessibilityLabel("Copy config file path")
                            .accessibilityHint("Copies the settings file path to the clipboard")
                        }
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                }

                // ── Updates ──────────────────────────────────────────────────
                SettingsGroup(title: "Updates") {
                    HStack(alignment: .center, spacing: 12) {
                        VStack(alignment: .leading, spacing: 4) {
                            // Show current installed version
                            Text("Current version: \(appState.coreVersion)")
                                .font(.subheadline)

                            // Dynamic status line driven by updateStatus state
                            Group {
                                switch updateStatus {
                                case "idle":
                                    Text("Tap \"Check\" to look for a newer release.")
                                        .foregroundStyle(.secondary)
                                case "checking":
                                    Label("Checking for updates…", systemImage: "arrow.clockwise")
                                        .foregroundStyle(.secondary)
                                case "up-to-date":
                                    Label("You're up to date.", systemImage: "checkmark.circle.fill")
                                        .foregroundStyle(.green)
                                case let s where s.hasPrefix("available:"):
                                    let ver = s.dropFirst("available:".count)
                                    Label("Version \(ver) is available!", systemImage: "arrow.down.circle.fill")
                                        .foregroundStyle(.blue)
                                default:
                                    Text("Unknown status.")
                                        .foregroundStyle(.secondary)
                                }
                            }
                            .font(.caption)
                        }

                        Spacer()

                        VStack(spacing: 6) {
                            // Check for updates button — simulates a network call
                            Button("Check") {
                                checkForUpdates()
                            }
                            .controlSize(.regular)
                            .buttonStyle(.bordered)
                            .disabled(updateStatus == "checking")
                            .accessibilityLabel("Check for updates")
                            .accessibilityHint("Checks whether a newer version of MeedyaManager is available")

                            // "Download" link — only shown when an update is found
                            if updateStatus.hasPrefix("available:") {
                                Link("Download", destination: URL(
                                    string: "https://github.com/MWBMPartners/MeedyaManager/releases/latest"
                                )!)
                                .controlSize(.small)
                            }
                        }
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 12)
                }

                // ── About & Legal ────────────────────────────────────────────
                SettingsGroup(title: "About") {
                    HStack {
                        // Privacy Policy link — opens the policy document on GitHub
                        Label("Privacy Policy", systemImage: "hand.raised.fill")
                        Spacer()
                        Link(
                            "View",
                            destination: URL(
                                string: "https://github.com/MWBMPartners/MeedyaManager/blob/main/help/privacy-policy.md"
                            )!
                        )
                        .controlSize(.small)
                        .accessibilityLabel("View Privacy Policy")
                        .accessibilityHint("Opens the MeedyaManager privacy policy in your web browser")
                    }
                    .padding(.horizontal, 16)
                    .padding(.vertical, 10)
                }

                // ── Save Button ───────────────────────────────────────────────
                HStack {
                    if isSaving {
                        ProgressView()
                            .controlSize(.small)
                    }
                    if !saveStatus.isEmpty {
                        Text(saveStatus)
                            .font(.caption)
                            .foregroundStyle(saveStatus.hasPrefix("✓") ? .green : .red)
                    }
                    Spacer()
                    Button("Save Settings") {
                        saveConfig()
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(isSaving)
                    .accessibilityLabel("Save settings")
                    .accessibilityHint("Writes the current settings to the config file on disk")
                }
                .padding(.bottom, 8)

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

    /// Serialise the current settings to JSON and write them to `configFilePath`.
    private func saveConfig() {
        isSaving  = true
        saveStatus = ""

        // Build a JSON representation of the current settings snapshot
        let snapshot: [String: Any] = [
            "dry_run":      dryRun,
            "recursive":    recursive,
            "debounce_ms":  debounceMs,
            "log_level":    logLevel,
            "redact_pii":   redactPii,
        ]

        do {
            // Serialize as pretty-printed JSON (JSON5 superset — valid JSON5)
            let data = try JSONSerialization.data(
                withJSONObject: snapshot,
                options: [.prettyPrinted, .sortedKeys]
            )
            guard let jsonString = String(data: data, encoding: .utf8) else {
                throw CocoaError(.fileWriteUnknown)
            }

            // Ensure the parent directory exists
            let url = URL(fileURLWithPath: configFilePath)
            try FileManager.default.createDirectory(
                at: url.deletingLastPathComponent(),
                withIntermediateDirectories: true
            )

            // Write to disk
            try jsonString.write(to: url, atomically: true, encoding: .utf8)

            // Refresh the raw config preview
            rawConfigText = jsonString
            saveStatus    = "✓ Settings saved."
        } catch {
            saveStatus = "Save failed: \(error.localizedDescription)"
        }

        isSaving = false
    }

    /// Simulate an update check against the GitHub Releases API.
    ///
    /// In production this will call `mm-update`'s `UpdateChecker` via FFI.
    /// For M8 the check is simulated with a 1.5 s delay to exercise the UI states.
    private func checkForUpdates() {
        updateStatus = "checking"
        // Dispatch the simulated network call off the main thread
        DispatchQueue.global().asyncAfter(deadline: .now() + 1.5) {
            // Parse the current version from the bundle for comparison
            let current = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0.0.0"
            // M8 stub: always report up-to-date (real check wired in M9+)
            let result: String = "up-to-date"
            _ = current // suppress unused warning until real check is wired
            DispatchQueue.main.async {
                self.updateStatus = result
            }
        }
    }

    /// Commit all staged test-mode files to their real locations.
    ///
    /// Runs the FFI commit asynchronously, disabling the commit/revert
    /// buttons while in progress and refreshing the file count on completion.
    private func commitTestModeFiles() {
        // Mark the operation as in progress to disable buttons
        isTestModeProcessing = true
        Task {
            do {
                // Ask the core to apply all staged operations
                try await MmCore.shared.commitTestModeFiles()
            } catch {
                // Log commit failures (in production, surface to the user)
                print("[MeedyaManager] Test mode commit failed: \(error)")
            }
            // Refresh the staged file count from the core
            appState.testModeFileCount = MmCore.shared.testModeFileCount()
            // Re-enable the buttons
            isTestModeProcessing = false
        }
    }

    /// Revert all staged test-mode files, discarding uncommitted changes.
    ///
    /// Runs the FFI revert asynchronously, disabling the commit/revert
    /// buttons while in progress and refreshing the file count on completion.
    private func revertTestModeFiles() {
        // Mark the operation as in progress to disable buttons
        isTestModeProcessing = true
        Task {
            do {
                // Ask the core to discard all staged operations
                try await MmCore.shared.revertTestModeFiles()
            } catch {
                // Log revert failures (in production, surface to the user)
                print("[MeedyaManager] Test mode revert failed: \(error)")
            }
            // Refresh the staged file count from the core
            appState.testModeFileCount = MmCore.shared.testModeFileCount()
            // Re-enable the buttons
            isTestModeProcessing = false
        }
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
                .accessibilityLabel(label)
                .accessibilityHint(detail)
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
