// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Application-wide Observable State (M6)
//
// AppState is the root @Observable class injected via .environment()
// into all views.  It owns all feature states and drives navigation selection.

import SwiftUI
import Combine

/// Application-wide shared state injected into the SwiftUI environment.
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

    /// State for the Metadata Lookup panel (M6)
    var lookup: LookupModel = LookupModel()

    // MARK: – Core Version

    /// The MeedyaManager core version string (from mm-ffi or hardcoded fallback)
    var coreVersion: String = MmCore.shared.version()
}

/// The top-level navigation destinations in the sidebar / tab strip.
enum AppTab: String, CaseIterable, Identifiable {
    case library  = "Library"
    case metadata = "Metadata"
    case lookup   = "Lookup"
    case rules    = "Rules"
    case settings = "Settings"

    var id: String { rawValue }

    /// SF Symbol name for each tab's icon
    var icon: String {
        switch self {
        case .library:  "folder.fill"
        case .metadata: "tag.fill"
        case .lookup:   "magnifyingglass"
        case .rules:    "list.bullet.rectangle.fill"
        case .settings: "gearshape.fill"
        }
    }
}
