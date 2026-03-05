// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Application-wide Observable State
//
// AppState is the root ObservableObject injected via .environmentObject()
// into all views.  It owns the two main feature states (scan and metadata)
// and drives navigation selection.

import SwiftUI
import Combine

/// Application-wide shared state injected into the SwiftUI environment.
///
/// All published properties trigger view updates automatically via Combine.
@Observable
final class AppState {

    // MARK: – Navigation

    /// The currently selected sidebar/tab item
    var selectedTab: AppTab = .library

    // MARK: – Feature States

    /// State for the Library / Scan panel
    var scan: ScanModel = ScanModel()

    /// State for the Metadata editor panel
    var metadata: MetadataModel = MetadataModel()

    // MARK: – Core Version

    /// The MeedyaManager core version string (from mm-ffi or hardcoded fallback)
    var coreVersion: String = MmCore.shared.version()
}

/// The top-level navigation destinations in the sidebar / tab strip.
enum AppTab: String, CaseIterable, Identifiable {
    case library  = "Library"
    case metadata = "Metadata"
    case rules    = "Rules"
    case settings = "Settings"

    var id: String { rawValue }

    /// SF Symbol name for each tab's icon
    var icon: String {
        switch self {
        case .library:  "folder.fill"
        case .metadata: "tag.fill"
        case .rules:    "list.bullet.rectangle.fill"
        case .settings: "gearshape.fill"
        }
    }
}
