// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Metadata Editor Panel Model
//
// Drives the Metadata tab: loads tags from a file via MmCore, tracks edits,
// and writes them back using the UniFFI bridge (or stub).

import SwiftUI

/// A single metadata tag for display and editing.
struct TagItem: Identifiable {
    let id    = UUID()
    let key:   String     // canonical key, e.g. "artist"
    var value: String     // current (possibly edited) value
}

/// Observable state for the Metadata editor panel.
@Observable
final class MetadataModel {

    // MARK: – State

    /// Path of the currently loaded file (nil if none)
    var filePath: String? = nil

    /// Tags loaded from the file
    var tags: [TagItem] = []

    /// True when there are unsaved edits
    var hasUnsavedEdits: Bool = false

    /// Human-readable audio properties string (codec, sample rate, duration)
    var audioPropertiesText: String = ""

    /// Status message shown at the bottom of the panel
    var status: String = "Select a media file to view its metadata."

    /// True while a load or save operation is running
    var isRunning: Bool = false

    // MARK: – Actions

    /// Load all tags from the file at `path` via MmCore.
    @MainActor
    func loadFile(_ path: String) async {
        isRunning = true
        status    = "Loading…"
        tags      = []
        hasUnsavedEdits  = false
        audioPropertiesText = ""

        do {
            let loaded = try await MmCore.shared.getMetadata(path: path)
            tags = loaded
                .sorted { $0.key < $1.key }
                .map { TagItem(key: $0.key, value: $0.value) }

            // Load audio properties
            if let props = try? await MmCore.shared.getAudioProperties(path: path) {
                audioPropertiesText = props
            }

            filePath = path
            status   = URL(fileURLWithPath: path).lastPathComponent
        } catch {
            status = "Failed to load: \(error.localizedDescription)"
        }

        isRunning = false
    }

    /// Save all current tag values back to the file.
    @MainActor
    func saveAll() async {
        guard let path = filePath else { return }

        isRunning = true
        status    = "Saving…"

        do {
            try await MmCore.shared.writeMetadata(
                path: path,
                tags: tags.map { (key: $0.key, value: $0.value) }
            )
            hasUnsavedEdits = false
            status = "✓ Tags saved."
        } catch {
            status = "Save failed: \(error.localizedDescription)"
        }

        isRunning = false
    }

    /// Discard all edits and reload from the file.
    @MainActor
    func revert() async {
        guard let path = filePath else { return }
        await loadFile(path)
    }

    /// Update the value for a tag by key (called from the UI on edit).
    func updateTag(key: String, newValue: String) {
        if let idx = tags.firstIndex(where: { $0.key == key }) {
            tags[idx].value = newValue
            hasUnsavedEdits = true
        }
    }
}
