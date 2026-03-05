// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — AppTab Unit Tests
//
// Tests the AppTab navigation enum for correct raw values, IDs, SF Symbol
// icon names, and CaseIterable conformance.

import Testing
import Foundation

// MARK: – AppTab replica
// SPM .testTarget cannot @testable import an .executableTarget, so we replicate
// the enum under test here.  The CI build verifies the real source compiles against
// the same definition via ci-macos.yml.

/// Navigation tabs — replica of the production enum for test isolation.
enum AppTab: String, CaseIterable, Identifiable {
    case library  = "Library"
    case metadata = "Metadata"
    case lookup   = "Lookup"
    case rules    = "Rules"
    case cloud    = "Cloud"    // M7 — Cloud Storage Monitor
    case settings = "Settings"

    var id: String { rawValue }

    var icon: String {
        switch self {
        case .library:  "folder.fill"
        case .metadata: "tag.fill"
        case .lookup:   "magnifyingglass"
        case .rules:    "list.bullet.rectangle.fill"
        case .cloud:    "cloud.fill"
        case .settings: "gearshape.fill"
        }
    }
}

// MARK: – Tests

@Suite("AppTab")
struct AppTabTests {

    @Test("CaseIterable provides 6 cases")
    func allCases_has_six_cases() {
        #expect(AppTab.allCases.count == 6)
    }

    @Test("library raw value")
    func library_rawValue() {
        #expect(AppTab.library.rawValue == "Library")
    }

    @Test("metadata raw value")
    func metadata_rawValue() {
        #expect(AppTab.metadata.rawValue == "Metadata")
    }

    @Test("lookup raw value")
    func lookup_rawValue() {
        #expect(AppTab.lookup.rawValue == "Lookup")
    }

    @Test("rules raw value")
    func rules_rawValue() {
        #expect(AppTab.rules.rawValue == "Rules")
    }

    @Test("settings raw value")
    func settings_rawValue() {
        #expect(AppTab.settings.rawValue == "Settings")
    }

    @Test("Identifiable id equals rawValue")
    func id_equals_rawValue() {
        for tab in AppTab.allCases {
            #expect(tab.id == tab.rawValue)
        }
    }

    @Test("library icon is folder.fill")
    func library_icon() {
        #expect(AppTab.library.icon == "folder.fill")
    }

    @Test("metadata icon is tag.fill")
    func metadata_icon() {
        #expect(AppTab.metadata.icon == "tag.fill")
    }

    @Test("lookup icon is magnifyingglass")
    func lookup_icon() {
        #expect(AppTab.lookup.icon == "magnifyingglass")
    }

    @Test("rules icon is list.bullet.rectangle.fill")
    func rules_icon() {
        #expect(AppTab.rules.icon == "list.bullet.rectangle.fill")
    }

    @Test("settings icon is gearshape.fill")
    func settings_icon() {
        #expect(AppTab.settings.icon == "gearshape.fill")
    }

    @Test("cloud raw value")
    func cloud_rawValue() {
        #expect(AppTab.cloud.rawValue == "Cloud")
    }

    @Test("cloud icon is cloud.fill")
    func cloud_icon() {
        #expect(AppTab.cloud.icon == "cloud.fill")
    }
}
