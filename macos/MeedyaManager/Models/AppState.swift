// (C) 2025-2026 MWBM Partners Ltd
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

    // MARK: – Test Mode

    /// Whether test mode is currently active.
    ///
    /// When enabled, all file-system mutations (renames, tag writes) are
    /// redirected to a staging area.  The UI reflects this state with
    /// visual indicators and commit/revert controls.
    var testModeEnabled: Bool = MmCore.shared.testModeEnabled()

    /// The number of files currently staged in test mode.
    ///
    /// Updated whenever test mode state changes, files are scanned,
    /// or commit/revert operations complete.
    var testModeFileCount: Int = MmCore.shared.testModeFileCount()

    // MARK: – Pre-release Detection

    /// Whether the current build is a pre-release version.
    ///
    /// Determined at launch by checking whether the version string
    /// contains a hyphen (e.g. "1.3.0-beta.1").
    var isPreRelease: Bool = false

    /// Controls visibility of the pre-release alert on first launch.
    var showPreReleaseAlert: Bool = false
}

/// The top-level navigation destinations in the sidebar / tab strip.
enum AppTab: String, CaseIterable, Identifiable {
    case library  = "Library"
    case metadata = "Metadata"
    case lookup   = "Lookup"
    case rules    = "Rules"
    case cloud    = "Cloud"    // M7 — Cloud Storage Monitor
    case export   = "Export"   // M9 — Database Export
    case server   = "Server"   // M10 — Secure Media Server
    case settings = "Settings"

    var id: String { rawValue }

    /// SF Symbol name for each tab's icon
    var icon: String {
        switch self {
        case .library:  "folder.fill"
        case .metadata: "tag.fill"
        case .lookup:   "magnifyingglass"
        case .rules:    "list.bullet.rectangle.fill"
        case .cloud:    "cloud.fill"
        case .export:   "cylinder.split.1x2.fill"
        case .server:   "network"
        case .settings: "gearshape.fill"
        }
    }

    /// Human-readable label announced by VoiceOver when the tab gains focus.
    /// Identical to the raw value; exposed as a named property for clarity.
    var accessibilityLabel: String { rawValue }
}
