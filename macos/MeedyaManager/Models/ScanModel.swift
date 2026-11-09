// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Scan / Library Panel Model
//
// Drives the Library tab: holds the selected directory, template, scan results,
// and executes renames via MmCore (the UniFFI bridge or its stub).

import SwiftUI

/// Rename preview item ready for display in the scan results list.
struct RenamePreviewItem: Identifiable {
    let id = UUID()
    let sourcePath: String
    let destinationPath: String
    let conflict: Bool
    let unchanged: Bool

    /// Display name for the source (basename only)
    var sourceName: String { URL(fileURLWithPath: sourcePath).lastPathComponent }

    /// Display name for the destination (basename only)
    var destinationName: String { URL(fileURLWithPath: destinationPath).lastPathComponent }

    /// Badge text shown alongside each row
    var badgeText: String {
        if conflict  { return "CONFLICT"  }
        if unchanged { return "UNCHANGED" }
        return "RENAME"
    }

    /// Is this preview ready to execute (no conflict, not unchanged)?
    var isExecutable: Bool { !conflict && !unchanged }
}

/// Observable state for the Library / Scan panel.
@Observable
final class ScanModel {

    // MARK: – Inputs (bound to UI controls)

    /// Absolute path of the directory to scan
    var directoryPath: String = ""

    /// Rename template string
    var template: String = "<Artist> - <Title>"

    /// Whether to scan sub-directories recursively
    var recursive: Bool = false

    // MARK: – Outputs

    /// Rename previews computed by the last scan
    var previews: [RenamePreviewItem] = []

    /// Human-readable status / progress message
    var status: String = "Select a folder and click Scan."

    /// True while a scan or rename is running
    var isRunning: Bool = false

    // MARK: – Computed

    /// Summary description of the last scan results
    var summary: String {
        guard !previews.isEmpty else { return "No files scanned." }
        let total     = previews.count
        let toRename  = previews.filter(\.isExecutable).count
        let unchanged = previews.filter(\.unchanged).count
        let conflicts = previews.filter(\.conflict).count
        return "\(total) files — \(toRename) to rename, \(unchanged) unchanged, \(conflicts) conflicts"
    }

    /// True if there are previews that can be executed.
    ///
    /// Requires the engine to actually be linked, on top of the existing
    /// per-preview check — without this, a stub build with fabricated
    /// previews (all marked `conflict: false, unchanged: false`) would
    /// still show Execute as enabled. See issue #222.
    var canExecute: Bool { MmCore.isEngineAvailable && previews.contains(where: \.isExecutable) }

    /// Work out why Execute must refuse to run right now, if it must.
    ///
    /// Kept as a `static` pure function (no `self`) so
    /// `ScanModelTests.swift`'s standalone replica — required because SwiftPM
    /// cannot `@testable import` an executable target — can mirror the exact
    /// same decision and the two can't quietly drift apart on which check
    /// wins when both are true.
    ///
    /// The engine check comes first: if the engine is missing, Test Mode's
    /// own state cannot be trusted either (it is read through the same
    /// stubbed bridge), so naming the engine problem first is the more
    /// useful message.
    ///
    /// - Returns: A user-facing refusal message, or `nil` if execution may
    ///   proceed.
    static func executeRefusalReason(engineAvailable: Bool, testModeEnabled: Bool) -> String? {
        if !engineAvailable {
            return "Nothing was renamed. This build does not include the MeedyaManager engine, so there are no trustworthy previews to apply. (issue #222)"
        }
        if testModeEnabled {
            return "Nothing was renamed. Test Mode is on, and renames are not staged by Test Mode — turn it off in Settings to rename files for real."
        }
        return nil
    }

    /// Short count string announced by VoiceOver for the results list.
    /// e.g. "No files to rename", "1 file to rename", "5 files to rename"
    var renameCountDescription: String {
        let count = previews.filter(\.isExecutable).count
        switch count {
        case 0:  return "No files to rename."
        case 1:  return "1 file to rename."
        default: return "\(count) files to rename."
        }
    }

    // MARK: – Actions

    /// Run a scan of the selected directory using MmCore.
    @MainActor
    func scan() async {
        guard !directoryPath.isEmpty else {
            status = "Please select a folder first."
            return
        }

        // Refuse before even trying, rather than let MmCore's thrown
        // EngineUnavailableError surface as a generic "Scan failed: …"
        // message down in the catch block below. Saying it here, plainly
        // and specifically, is what issue #222 asks for: the user must be
        // told they are running without the engine, not left to guess from
        // an error string. Clearing `previews` too, so any stale results
        // from an earlier scan can't be mistaken for a fresh, trustworthy
        // one.
        guard MmCore.isEngineAvailable else {
            previews = []
            status = "Scanning is unavailable. This build does not include the MeedyaManager engine, so there are no trustworthy previews to show. (issue #222)"
            return
        }

        isRunning = true
        status    = "Scanning…"
        previews  = []

        do {
            let results = try await MmCore.shared.scanDirectory(
                directory: directoryPath,
                template:  template,
                recursive: recursive
            )

            previews = results.map { p in
                RenamePreviewItem(
                    sourcePath:      p.source,
                    destinationPath: p.destination,
                    conflict:        p.conflict,
                    unchanged:       p.unchanged
                )
            }

            status = summary

        } catch {
            status = "Scan failed: \(error.localizedDescription)"
        }

        isRunning = false
    }

    /// Execute the pending renames using MmCore.
    ///
    /// - Parameter testModeEnabled: The current Test Mode setting, read from
    ///   `AppState` by the caller. Test Mode has never staged renames (only
    ///   tag writes go through the staging area — see issue #222's
    ///   discussion), so the only honest thing this can do while it is on is
    ///   refuse and say so, rather than pretend the rename was staged
    ///   somewhere it was not.
    @MainActor
    func executeRenames(testModeEnabled: Bool) async {
        // Second layer of defence: the Execute button in ScanView is already
        // disabled in both of these situations, but a model-level check
        // means the refusal holds even if a future UI change forgets to
        // wire the button's `.disabled(...)` up correctly.
        if let reason = ScanModel.executeRefusalReason(
            engineAvailable: MmCore.isEngineAvailable,
            testModeEnabled: testModeEnabled
        ) {
            status = reason
            return
        }

        let executable = previews.filter(\.isExecutable)
        guard !executable.isEmpty else { return }

        isRunning = true
        status    = "Renaming \(executable.count) files…"

        do {
            // The only file-moving call left anywhere in this app's Swift
            // code — see MmCore.applyRenames's doc comment. Before this
            // change, this loop moved files directly via the file manager,
            // straight onto destinations a stub had invented, which is how
            // issue #222's real files ended up renamed to meaningless
            // "Preview"-prefixed names.
            let ffiPreviews = executable.map {
                FfiRenamePreview(
                    source:      $0.sourcePath,
                    destination: $0.destinationPath,
                    conflict:    $0.conflict,
                    unchanged:   $0.unchanged
                )
            }
            let count = try await MmCore.shared.applyRenames(ffiPreviews)
            previews = []
            status = "✓ Renamed \(count) files."
        } catch {
            status = "Rename failed: \(error.localizedDescription)"
        }

        isRunning = false
    }
}
