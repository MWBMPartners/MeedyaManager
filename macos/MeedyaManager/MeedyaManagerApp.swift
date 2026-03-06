// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — macOS Application Entry Point
//
// Configures the SwiftUI app with:
//   - A main WindowGroup hosting ContentView
//   - macOS 26+ Liquid Glass window styling (runtime detection)
//   - A Settings scene for the preferences window
//   - Menu commands for the File and Help menus
//   - Pre-release version detection on first launch (auto-enables test mode)

import SwiftUI

@main
struct MeedyaManagerApp: App {

    /// Application-wide observable state shared across all views.
    @State private var appState = AppState()

    var body: some Scene {

        // ── Main application window ─────────────────────────────────────────
        WindowGroup {
            ContentView()
                .environment(appState)
                // Minimum window size enforced by the OS
                .frame(minWidth: 800, minHeight: 560)
                // Perform pre-release detection on first appearance
                .onAppear {
                    detectPreReleaseVersion()
                }
                // Alert shown when a pre-release version is detected
                .alert(
                    "Pre-release Version Detected",
                    isPresented: $appState.showPreReleaseAlert
                ) {
                    // Acknowledge the alert and keep test mode enabled
                    Button("OK") {
                        // Test mode is already enabled by detectPreReleaseVersion()
                    }
                    // Open the releases page to check for a stable update
                    Button("Check for Stable Update") {
                        // Open the GitHub releases page in the default browser
                        NSWorkspace.shared.open(
                            URL(string: "https://github.com/MWBMPartners/MeedyaManager/releases/latest")!
                        )
                    }
                } message: {
                    // Informative message explaining what happened and why
                    Text("You are running a pre-release build (\(appState.coreVersion)). Test mode has been automatically enabled to protect your media files. You can disable it in Settings.")
                }
        }
        .windowStyle(.titleBar)
        .defaultSize(width: 1200, height: 800)
        .commands {
            // Replace the default Help menu
            CommandGroup(replacing: .help) {
                Link(
                    "MeedyaManager Documentation",
                    destination: URL(string: "https://github.com/MWBMPartners/MeedyaManager")!
                )
                Divider()
                // Privacy Policy link in the Help menu
                Link(
                    "Privacy Policy",
                    destination: URL(
                        string: "https://github.com/MWBMPartners/MeedyaManager/blob/main/help/privacy-policy.md"
                    )!
                )
                Divider()
                Button("Report a Bug…") {
                    NSWorkspace.shared.open(
                        URL(string: "https://github.com/MWBMPartners/MeedyaManager/issues")!
                    )
                }
            }
            // File menu additions
            CommandGroup(after: .newItem) {
                Divider()
                Button("Open Folder…") {
                    appState.selectedTab = .library
                }
                .keyboardShortcut("O", modifiers: .command)
            }
        }

        // ── Settings / Preferences window (⌘,) ─────────────────────────────
        Settings {
            SettingsView()
                .environment(appState)
                .frame(width: 600, height: 400)
        }
    }

    // MARK: – Pre-release Detection

    /// Detect whether this build is a pre-release version.
    ///
    /// A version is considered pre-release if it contains a hyphen
    /// (e.g. "1.3.0-beta.1", "2.0.0-rc.2").  When detected:
    ///   1. Mark the app state as pre-release
    ///   2. Auto-enable test mode to protect the user's real files
    ///   3. Show an informational alert offering to check for a stable update
    ///
    /// This check runs once per launch via `.onAppear` on the main window.
    private func detectPreReleaseVersion() {
        // Read the version string from the FFI bridge (includes "(stub)" suffix in dev)
        let version = appState.coreVersion

        // A hyphen in the version string indicates a pre-release suffix (SemVer §9)
        // Strip the " (stub)" suffix first so stub builds can also be detected
        let cleanVersion = version.replacingOccurrences(of: " (stub)", with: "")

        // Check if the clean version contains a hyphen (pre-release indicator)
        if cleanVersion.contains("-") {
            // Mark the app state as a pre-release build
            appState.isPreRelease = true

            // Auto-enable test mode to protect real media files
            MmCore.shared.setTestMode(enabled: true)
            appState.testModeEnabled = true

            // Refresh the staged file count (should be 0 at launch)
            appState.testModeFileCount = MmCore.shared.testModeFileCount()

            // Show the pre-release alert to inform the user
            appState.showPreReleaseAlert = true
        }
    }
}
