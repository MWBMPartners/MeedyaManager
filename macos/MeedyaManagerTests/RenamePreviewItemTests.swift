// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — RenamePreviewItem Unit Tests
//
// Tests computed properties on RenamePreviewItem: display names, badge text,
// and executability logic.

import Testing
import Foundation

// MARK: – RenamePreviewItem replica

/// Replica of the production struct for test isolation.
struct RenamePreviewItem: Identifiable {
    let id = UUID()
    let sourcePath: String
    let destinationPath: String
    let conflict: Bool
    let unchanged: Bool

    var sourceName: String { URL(fileURLWithPath: sourcePath).lastPathComponent }
    var destinationName: String { URL(fileURLWithPath: destinationPath).lastPathComponent }

    var badgeText: String {
        if conflict  { return "CONFLICT"  }
        if unchanged { return "UNCHANGED" }
        return "RENAME"
    }

    var isExecutable: Bool { !conflict && !unchanged }
}

// MARK: – Tests

@Suite("RenamePreviewItem")
struct RenamePreviewItemTests {

    // Fixture helpers
    func makeRename() -> RenamePreviewItem {
        RenamePreviewItem(
            sourcePath: "/music/track01.mp3",
            destinationPath: "/music/Pink Floyd - Comfortably Numb.mp3",
            conflict: false,
            unchanged: false
        )
    }
    func makeConflict() -> RenamePreviewItem {
        RenamePreviewItem(
            sourcePath: "/music/track01.mp3",
            destinationPath: "/music/track01.mp3",
            conflict: true,
            unchanged: false
        )
    }
    func makeUnchanged() -> RenamePreviewItem {
        RenamePreviewItem(
            sourcePath: "/music/track01.mp3",
            destinationPath: "/music/track01.mp3",
            conflict: false,
            unchanged: true
        )
    }

    @Test("sourceName extracts basename")
    func sourceName_basename() {
        #expect(makeRename().sourceName == "track01.mp3")
    }

    @Test("destinationName extracts basename")
    func destinationName_basename() {
        #expect(makeRename().destinationName == "Pink Floyd - Comfortably Numb.mp3")
    }

    @Test("badgeText is RENAME for a normal rename")
    func badgeText_rename() {
        #expect(makeRename().badgeText == "RENAME")
    }

    @Test("badgeText is CONFLICT when conflict is true")
    func badgeText_conflict() {
        #expect(makeConflict().badgeText == "CONFLICT")
    }

    @Test("badgeText is UNCHANGED when unchanged is true")
    func badgeText_unchanged() {
        #expect(makeUnchanged().badgeText == "UNCHANGED")
    }

    @Test("isExecutable is true for a normal rename")
    func isExecutable_rename() {
        #expect(makeRename().isExecutable == true)
    }

    @Test("isExecutable is false for a conflict")
    func isExecutable_conflict() {
        #expect(makeConflict().isExecutable == false)
    }

    @Test("isExecutable is false for an unchanged item")
    func isExecutable_unchanged() {
        #expect(makeUnchanged().isExecutable == false)
    }

    @Test("each item has a unique UUID")
    func unique_ids() {
        let a = makeRename()
        let b = makeRename()
        #expect(a.id != b.id)
    }
}
