// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — macOS Application Entry Point
//
// Configures the SwiftUI app with:
//   - A main WindowGroup hosting ContentView
//   - macOS 26+ Liquid Glass window styling (runtime detection)
//   - A Settings scene for the preferences window
//   - Menu commands for the File and Help menus

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
}
