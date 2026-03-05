// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Root Navigation View
//
// Provides the top-level navigation shell using TabView with
// `.tabViewStyle(.sidebarAdaptable)` which renders as a sidebar on macOS 15+.
//
// macOS 26+ Liquid Glass:
//   When running on macOS 26 or later, the `.glassBackground()` modifier is
//   applied to the content area for the frosted-glass visual effect.
//   On earlier versions, the standard window material is used.

import SwiftUI

/// Root navigation view for the MeedyaManager macOS application.
///
/// Hosts the sidebar tab navigation and routes to each feature panel.
struct ContentView: View {

    @Environment(AppState.self) private var appState

    var body: some View {
        // sidebarAdaptable shows a macOS-native sidebar with icon + label
        // on macOS 15+ and falls back to a standard tab bar on older systems.
        TabView(selection: Bindable(appState).selectedTab) {

            // ── Library / Scan tab ─────────────────────────────────────────
            Tab("Library", systemImage: "folder.fill", value: AppTab.library) {
                ScanView()
                    .applyContentBackground()
            }

            // ── Metadata editor tab ────────────────────────────────────────
            Tab("Metadata", systemImage: "tag.fill", value: AppTab.metadata) {
                MetadataView()
                    .applyContentBackground()
            }

            // ── Metadata Lookup tab (M6) ───────────────────────────────────
            Tab("Lookup", systemImage: "magnifyingglass", value: AppTab.lookup) {
                LookupView()
                    .applyContentBackground()
            }

            // ── Rules / Template builder tab ───────────────────────────────
            Tab("Rules", systemImage: "list.bullet.rectangle.fill", value: AppTab.rules) {
                RulesView()
                    .applyContentBackground()
            }

            // ── Cloud Storage Monitor tab (M7) ────────────────────────────
            Tab("Cloud", systemImage: "cloud.fill", value: AppTab.cloud) {
                CloudView()
                    .applyContentBackground()
            }

            // ── Settings tab ───────────────────────────────────────────────
            Tab("Settings", systemImage: "gearshape.fill", value: AppTab.settings) {
                SettingsView()
                    .applyContentBackground()
            }
        }
        .tabViewStyle(.sidebarAdaptable)
        // Increased minimum width to 960 to accommodate the 7-tab sidebar
        .frame(minWidth: 960, minHeight: 560)
    }
}

// MARK: – Liquid Glass / background helper

private extension View {
    /// Apply the appropriate background material for the running macOS version.
    ///
    /// - macOS 26+: Liquid Glass `.glassBackground()` (frosted, translucent).
    /// - macOS 15–25: `.ultraThinMaterial` (standard vibrancy).
    @ViewBuilder
    func applyContentBackground() -> some View {
        if #available(macOS 26.0, *) {
            // Liquid Glass — Apple's new translucent material in macOS 26
            self.background(.thinMaterial)
        } else {
            // Standard vibrancy on macOS 15–25
            self.background(.ultraThinMaterial)
        }
    }
}

#Preview {
    ContentView()
        .environment(AppState())
}
