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

    /// True if there are previews that can be executed
    var canExecute: Bool { previews.contains(where: \.isExecutable) }

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
    @MainActor
    func executeRenames() async {
        let executable = previews.filter(\.isExecutable)
        guard !executable.isEmpty else { return }

        isRunning = true
        status    = "Renaming \(executable.count) files…"

        var renamed = 0
        var errors  = 0

        for preview in executable {
            do {
                try FileManager.default.moveItem(
                    atPath: preview.sourcePath,
                    toPath: preview.destinationPath
                )
                renamed += 1
            } catch {
                errors += 1
            }
        }

        previews = []
        isRunning = false

        if errors == 0 {
            status = "✓ Renamed \(renamed) files successfully."
        } else {
            status = "⚠ Renamed \(renamed) files; \(errors) errors."
        }
    }
}
