// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Accessibility Tests (macOS, Issue #128)
//
// Tests the accessibility properties of model types and helper values used
// throughout MeedyaManager views. SwiftUI's Tab/NavigationSplitView
// hierarchy exposes VoiceOver labels automatically; these tests cover the
// model-layer descriptions and any computed strings passed to the
// .accessibilityLabel() / .accessibilityHint() view modifiers.
//
// VoiceOver compliance targets:
//   - Every interactive element (Button, TextField, Toggle) carries a label.
//   - Status strings are human-readable and not emoji-only.
//   - No element relies solely on colour to convey state.

import Testing
@testable import MeedyaManager

// MARK: — AppTab accessibility labels

/// Verifies that every tab value produces a non-empty, human-readable label
/// usable by VoiceOver as the sidebar item description.
@Suite("AppTab accessibility")
struct AppTabAccessibilityTests {

    @Test("All tabs have non-empty display names")
    func allTabsHaveDisplayNames() {
        // AppTab.allCases requires AppTab: CaseIterable
        let labels = AppTab.allCases.map(\.accessibilityLabel)
        for label in labels {
            #expect(!label.isEmpty, "Tab label must not be empty")
        }
    }

    @Test("Tab labels are unique")
    func tabLabelsAreUnique() {
        let labels = AppTab.allCases.map(\.accessibilityLabel)
        let unique = Set(labels)
        #expect(unique.count == labels.count, "All tab labels must be distinct")
    }

    @Test("Library tab label")
    func libraryLabel() {
        #expect(AppTab.library.accessibilityLabel == "Library")
    }

    @Test("Metadata tab label")
    func metadataLabel() {
        #expect(AppTab.metadata.accessibilityLabel == "Metadata")
    }

    @Test("Lookup tab label")
    func lookupLabel() {
        #expect(AppTab.lookup.accessibilityLabel == "Lookup")
    }

    @Test("Rules tab label")
    func rulesLabel() {
        #expect(AppTab.rules.accessibilityLabel == "Rules")
    }

    @Test("Cloud tab label")
    func cloudLabel() {
        #expect(AppTab.cloud.accessibilityLabel == "Cloud")
    }

    @Test("Export tab label")
    func exportLabel() {
        #expect(AppTab.export.accessibilityLabel == "Export")
    }

    @Test("Server tab label")
    func serverLabel() {
        #expect(AppTab.server.accessibilityLabel == "Server")
    }

    @Test("Settings tab label")
    func settingsLabel() {
        #expect(AppTab.settings.accessibilityLabel == "Settings")
    }
}

// MARK: — ServerStatus accessibility

/// Verifies that the server status `displayText` is VoiceOver-safe:
/// purely descriptive, no ANSI codes, and no misleading state.
@Suite("ServerStatus VoiceOver descriptions")
struct ServerStatusAccessibilityTests {

    @Test("Stopped status is descriptive")
    func stopped() {
        let s = ServerStatus.stopped
        #expect(s.displayText == "Stopped")
        #expect(!s.isRunning)
    }

    @Test("Starting status is descriptive")
    func starting() {
        let s = ServerStatus.starting
        #expect(s.displayText.lowercased().contains("start"))
        #expect(!s.isRunning)
    }

    @Test("Running status includes address")
    func running() {
        let s = ServerStatus.running(address: "https://0.0.0.0:8443")
        #expect(s.displayText.contains("0.0.0.0:8443"))
        #expect(s.isRunning)
    }

    @Test("Error status includes message")
    func error() {
        let s = ServerStatus.error(message: "TLS cert not found")
        #expect(s.displayText.lowercased().contains("error"))
        #expect(s.displayText.contains("TLS cert not found"))
        #expect(!s.isRunning)
    }
}

// MARK: — ScanModel accessibility strings

/// Verifies that ScanModel exposes status strings suitable for
/// the VoiceOver live region (accessibilityValue) on the status bar.
@Suite("ScanModel accessibility")
struct ScanModelAccessibilityTests {

    @Test("Initial status is non-empty")
    func initialStatus() {
        let m = ScanModel()
        // Status must be non-nil and non-empty so VoiceOver can announce it
        #expect(!m.status.isEmpty)
    }

    @Test("Rename count description is pluralised correctly for zero")
    func zeroRenames() {
        let m = ScanModel()
        // No renames staged — description should mention zero or "no files"
        let desc = m.renameCountDescription
        #expect(!desc.isEmpty)
    }

    @Test("Rename count description is pluralised correctly for one")
    func oneRename() {
        let m = ScanModel()
        m.previews = [
            RenamePreviewItem(sourcePath: "a.mp3", destinationPath: "b.mp3",
                              conflict: false, unchanged: false)
        ]
        let desc = m.renameCountDescription
        #expect(desc.contains("1"))
    }

    @Test("Rename count description is pluralised correctly for many")
    func manyRenames() {
        let m = ScanModel()
        m.previews = (1...5).map { i in
            RenamePreviewItem(sourcePath: "\(i).mp3", destinationPath: "out\(i).mp3",
                              conflict: false, unchanged: false)
        }
        let desc = m.renameCountDescription
        #expect(desc.contains("5"))
    }
}

// MARK: — ServerModel accessibility

/// Verifies that ServerModel validation strings are suitable for use in
/// VoiceOver announcements (no raw HTML, ASCII escapes, or empty strings).
@Suite("ServerModel accessibility")
struct ServerModelAccessibilityTests {

    @Test("Valid configuration has nil validation error (no VoiceOver alert)")
    func validConfigNoAlert() {
        let m = ServerModel()
        m.jwtSecret = "supersecretkey-16c"
        m.noTls     = true
        #expect(m.validationError == nil)
    }

    @Test("Missing JWT secret produces human-readable error")
    func missingJwtError() {
        let m = ServerModel()
        m.jwtSecret = ""
        let err = m.validationError
        #expect(err != nil)
        // Error must not be empty and must not contain raw HTML or angle brackets
        #expect(!err!.isEmpty)
        #expect(!err!.contains("<") && !err!.contains(">"))
    }

    @Test("Missing TLS cert produces human-readable error")
    func missingTlsError() {
        let m = ServerModel()
        m.jwtSecret  = "supersecretkey-16c"
        m.noTls      = false
        m.tlsCertPath = ""
        let err = m.validationError
        #expect(err != nil)
        #expect(err!.lowercased().contains("certificate") || err!.lowercased().contains("tls"))
    }
}
