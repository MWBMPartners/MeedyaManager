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

/// Root view for MeedyaManager.
/// Presents a TabView with placeholder tabs for each major feature area.
struct ContentView: View {
    var body: some View {
        TabView {
            // Library tab — browse and manage media files
            Text("Library")
                .font(.largeTitle)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .tabItem {
                    Label("Library", systemImage: "books.vertical")
                }

            // Rules tab — configure auto-organization rules
            Text("Rules")
                .font(.largeTitle)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .tabItem {
                    Label("Rules", systemImage: "list.bullet.rectangle")
                }

            // Metadata tab — view and edit file metadata
            Text("Metadata")
                .font(.largeTitle)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .tabItem {
                    Label("Metadata", systemImage: "tag")
                }

            // Lookup tab — search online metadata providers
            Text("Lookup")
                .font(.largeTitle)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .tabItem {
                    Label("Lookup", systemImage: "magnifyingglass")
                }

            // Settings tab — application preferences and configuration
            Text("Settings")
                .font(.largeTitle)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .tabItem {
                    Label("Settings", systemImage: "gearshape")
                }
        }
        .frame(minWidth: 640, minHeight: 480) // Minimum window size
    }
}

#Preview {
    ContentView()
}
