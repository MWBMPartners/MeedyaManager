// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — ScanModel Logic Unit Tests
//
// Tests computed properties of ScanModel: summary generation, canExecute,
// and RenamePreviewItem aggregation logic.

import Testing
import Foundation

// MARK: – Minimal replicas

struct ScanPreviewItem: Identifiable {
    let id = UUID()
    let sourcePath: String
    let destinationPath: String
    let conflict: Bool
    let unchanged: Bool

    var isExecutable: Bool { !conflict && !unchanged }
}

final class ScanModelLogic {

    var directoryPath: String = ""
    var template:      String = "<Artist> - <Title>"
    var recursive:     Bool   = false
    var previews:      [ScanPreviewItem] = []
    var isRunning:     Bool = false

    var summary: String {
        guard !previews.isEmpty else { return "No files scanned." }
        let total     = previews.count
        let toRename  = previews.filter(\.isExecutable).count
        let unchanged = previews.filter(\.unchanged).count
        let conflicts = previews.filter(\.conflict).count
        return "\(total) files — \(toRename) to rename, \(unchanged) unchanged, \(conflicts) conflicts"
    }

    var canExecute: Bool { previews.contains(where: \.isExecutable) }
}

// MARK: – Tests

@Suite("ScanModel logic")
struct ScanModelTests {

    @Test("initial directory path is empty")
    func initial_directory_empty() {
        #expect(ScanModelLogic().directoryPath.isEmpty)
    }

    @Test("initial template has default value")
    func initial_template_default() {
        #expect(ScanModelLogic().template == "<Artist> - <Title>")
    }

    @Test("initial recursive is false")
    func initial_recursive_false() {
        #expect(ScanModelLogic().recursive == false)
    }

    @Test("initial previews are empty")
    func initial_previews_empty() {
        #expect(ScanModelLogic().previews.isEmpty)
    }

    @Test("summary is 'No files scanned.' when empty")
    func summary_empty() {
        #expect(ScanModelLogic().summary == "No files scanned.")
    }

    @Test("canExecute is false when previews empty")
    func canExecute_false_when_empty() {
        #expect(ScanModelLogic().canExecute == false)
    }

    @Test("canExecute is false when all are conflicts")
    func canExecute_false_all_conflicts() {
        let m = ScanModelLogic()
        m.previews = [
            ScanPreviewItem(sourcePath: "/a", destinationPath: "/a", conflict: true,  unchanged: false),
            ScanPreviewItem(sourcePath: "/b", destinationPath: "/b", conflict: true,  unchanged: false),
        ]
        #expect(m.canExecute == false)
    }

    @Test("canExecute is true when at least one is executable")
    func canExecute_true_when_one_executable() {
        let m = ScanModelLogic()
        m.previews = [
            ScanPreviewItem(sourcePath: "/a", destinationPath: "/b", conflict: false, unchanged: false),
        ]
        #expect(m.canExecute == true)
    }

    @Test("summary counts are correct for mixed previews")
    func summary_mixed_counts() {
        let m = ScanModelLogic()
        m.previews = [
            ScanPreviewItem(sourcePath: "/a", destinationPath: "/b", conflict: false, unchanged: false), // rename
            ScanPreviewItem(sourcePath: "/c", destinationPath: "/c", conflict: true,  unchanged: false), // conflict
            ScanPreviewItem(sourcePath: "/d", destinationPath: "/d", conflict: false, unchanged: true),  // unchanged
        ]
        #expect(m.summary == "3 files — 1 to rename, 1 unchanged, 1 conflicts")
    }

    @Test("summary shows all renames when no conflicts or unchanged")
    func summary_all_renames() {
        let m = ScanModelLogic()
        m.previews = [
            ScanPreviewItem(sourcePath: "/a", destinationPath: "/b", conflict: false, unchanged: false),
            ScanPreviewItem(sourcePath: "/c", destinationPath: "/d", conflict: false, unchanged: false),
        ]
        #expect(m.summary == "2 files — 2 to rename, 0 unchanged, 0 conflicts")
    }

    @Test("isRunning defaults to false")
    func isRunning_default_false() {
        #expect(ScanModelLogic().isRunning == false)
    }
}
