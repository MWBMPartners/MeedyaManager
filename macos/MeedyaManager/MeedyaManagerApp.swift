// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// This file is part of MeedyaManager.
//
// MeedyaManager is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// MeedyaManager is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with MeedyaManager. If not, see <https://www.gnu.org/licenses/>.

import SwiftUI

/// The main entry point for the MeedyaManager macOS application.
/// Uses SwiftUI's App protocol with a single WindowGroup containing
/// the root ContentView.
@main
struct MeedyaManagerApp: App {
    var body: some Scene {
        // Main application window
        WindowGroup {
            ContentView()
        }
        .windowStyle(.titleBar)               // Standard macOS title bar
        .defaultSize(width: 960, height: 640) // Reasonable default window size
    }
}
