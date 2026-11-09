// (C) 2025-2026 MWBM Partners Ltd
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

    // Added for issue #222 — mirrors the two new properties MmCore.swift and
    // ScanModel.swift use to keep an unlinked build from moving real files.
    // Real defaults ("engine present, Test Mode off") so the pre-existing
    // tests above, which know nothing about either flag, keep passing.
    var engineAvailable: Bool = true
    var testModeEnabled: Bool = false

    var summary: String {
        guard !previews.isEmpty else { return "No files scanned." }
        let total     = previews.count
        let toRename  = previews.filter(\.isExecutable).count
        let unchanged = previews.filter(\.unchanged).count
        let conflicts = previews.filter(\.conflict).count
        return "\(total) files — \(toRename) to rename, \(unchanged) unchanged, \(conflicts) conflicts"
    }

    // Mirrors ScanModel.canExecute: requires the engine to be linked, on top
    // of the pre-existing per-preview check.
    var canExecute: Bool { engineAvailable && previews.contains(where: \.isExecutable) }

    // Mirrors ScanModel.executeRefusalReason. Kept as a free-standing static
    // function (not reading `self`) for the same reason as the original: the
    // production code and this replica must be checking the exact same
    // decision, in the exact same order, or the "mirrors production" claim
    // in this file's own header comment stops being true.
    static func executeRefusalReason(engineAvailable: Bool, testModeEnabled: Bool) -> String? {
        if !engineAvailable {
            return "Nothing was renamed. This build does not include the MeedyaManager engine, so there are no trustworthy previews to apply. (issue #222)"
        }
        if testModeEnabled {
            return "Nothing was renamed. Test Mode is on, and renames are not staged by Test Mode — turn it off in Settings to rename files for real."
        }
        return nil
    }
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

    // MARK: – issue #222: engine-missing / Test Mode guards
    //
    // Honesty note (asked for explicitly in the brief this was written
    // against): because this file only replicates ScanModel's logic rather
    // than importing the real thing, none of these four tests can ever fail
    // against the production code — SwiftPM has no way to link them
    // together. What they prove is that the replicated contract is
    // internally consistent and does what issue #222 asked for; the actual
    // proof that the real ScanModel/MmCore behave this way is the
    // `swift build` + `grep` pair in the acceptance criteria, not this file.

    @Test("engine missing disables execute even when a rename is pending")
    func engine_missing_disables_execute_even_with_renames() {
        let m = ScanModelLogic()
        m.engineAvailable = false
        m.previews = [
            ScanPreviewItem(sourcePath: "/a", destinationPath: "/b", conflict: false, unchanged: false),
        ]
        #expect(m.canExecute == false)
    }

    @Test("refusal names the missing engine before it names Test Mode")
    func refusal_names_engine_first() {
        let reason = ScanModelLogic.executeRefusalReason(engineAvailable: false, testModeEnabled: true)
        #expect(reason?.contains("engine") == true)
    }

    @Test("refusal names Test Mode when only Test Mode is the problem")
    func refusal_names_test_mode() {
        let reason = ScanModelLogic.executeRefusalReason(engineAvailable: true, testModeEnabled: true)
        #expect(reason?.contains("Test Mode") == true)
    }

    @Test("refusal is nil when the engine is present and Test Mode is off")
    func refusal_nil_when_clear_to_execute() {
        let reason = ScanModelLogic.executeRefusalReason(engineAvailable: true, testModeEnabled: false)
        #expect(reason == nil)
    }
}
