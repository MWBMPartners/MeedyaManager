// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — MmCore Swift Bridge
//
// Wraps the UniFFI-generated bindings from mm-ffi.  In production builds
// (CI/release), the XCFramework containing the generated bindings is linked
// and MM_FFI_AVAILABLE is set, enabling the real implementations below.
//
// In development builds without the XCFramework, all functions return
// realistic stub values so the UI can be previewed and tested.
//
// Generated binding files (produced by uniffi-bindgen):
//   macos/MeedyaManager/Bindings/MeedyaManager.swift   — auto-generated
//   macos/Frameworks/MeedyaManagerFFI.xcframework       — Rust cdylib
//
// Usage:
//   let results = try await MmCore.shared.scanDirectory(...)
//   let tags    = try await MmCore.shared.getMetadata(path: ...)

import Foundation

// MARK: – Data transfer types (mirroring UniFFI-generated structs)

/// A single metadata tag pair passed across the FFI boundary.
struct FfiTagEntry {
    let key:   String
    let value: String
}

/// A single rename preview item returned by scanDirectory.
struct FfiRenamePreview {
    let source:      String
    let destination: String
    let conflict:    Bool
    let unchanged:   Bool
}

// MARK: – MmCore singleton

/// The single entry point for all calls into the Rust mm-core library.
///
/// Use `MmCore.shared` throughout the application.
/// All methods are `async throws` to support long-running operations
/// without blocking the main thread.
final class MmCore: @unchecked Sendable {

    // Shared singleton. @unchecked Sendable on the class is the documented
    // Swift 6 escape hatch for types with no mutable state — MmCore has none
    // (all stored properties are forbidden by convention; methods are pure
    // FFI calls or stateless stubs). Maintain this invariant: do NOT add
    // mutable stored properties without revisiting the Sendable conformance.
    static let shared = MmCore()
    private init() {}

    // MARK: – Version

    /// Return the MeedyaManager core version string.
    func version() -> String {
        #if MM_FFI_AVAILABLE
        // Real implementation: return mmVersion() from UniFFI bindings
        return mmVersion()
        #else
        return "\(Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0.5.0") (stub)"
        #endif
    }

    // MARK: – Configuration

    /// Return the platform-specific path to `settings.json5`.
    func configPath() -> String {
        #if MM_FFI_AVAILABLE
        return configPath()
        #else
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
        return support?.appendingPathComponent("MeedyaManager/settings.json5").path ?? "settings.json5"
        #endif
    }

    // MARK: – Scanning

    /// Scan `directory` for media files and compute rename previews.
    ///
    /// - Parameters:
    ///   - directory: Absolute path to the directory to scan.
    ///   - template:  Rename template (e.g. `"<Artist> - <Title>"`).
    ///   - recursive: Include sub-directories when true.
    /// - Returns: Sorted array of rename preview items.
    func scanDirectory(
        directory: String,
        template:  String,
        recursive: Bool
    ) async throws -> [FfiRenamePreview] {
        #if MM_FFI_AVAILABLE
        // Real: let previews = try scanDirectory(directory: directory, template: template, recursive: recursive)
        // return previews.map { FfiRenamePreview(source: $0.source, destination: $0.destination, conflict: $0.conflict, unchanged: $0.unchanged) }
        return try await Task.detached(priority: .userInitiated) {
            let raw = try scanDirectory(directory: directory, template: template, recursive: recursive)
            return raw.map { FfiRenamePreview(source: $0.source, destination: $0.destination, conflict: $0.conflict, unchanged: $0.unchanged) }
        }.value
        #else
        // Stub: scan using FileManager, return placeholder previews
        return try await Task.detached(priority: .userInitiated) {
            try self.stubScanDirectory(directory: directory, template: template)
        }.value
        #endif
    }

    // MARK: – Metadata

    /// Read all metadata tags from a media file.
    ///
    /// - Parameter path: Absolute path to the media file.
    /// - Returns: Sorted array of tag entries.
    func getMetadata(path: String) async throws -> [FfiTagEntry] {
        #if MM_FFI_AVAILABLE
        return try await Task.detached(priority: .userInitiated) {
            let raw = try getMetadata(path: path)
            return raw.map { FfiTagEntry(key: $0.key, value: $0.value) }
        }.value
        #else
        return stubMetadata(for: path)
        #endif
    }

    /// Write updated tag values to a media file.
    ///
    /// - Parameters:
    ///   - path: Absolute path to the media file.
    ///   - tags: Array of (key, value) pairs to write.
    func writeMetadata(path: String, tags: [(key: String, value: String)]) async throws {
        #if MM_FFI_AVAILABLE
        try await Task.detached(priority: .userInitiated) {
            let ffi = tags.map { TagEntry(key: $0.key, value: $0.value) }
            try writeMetadata(path: path, tags: ffi)
        }.value
        #else
        // Stub: no-op (cannot write to actual files without the FFI layer)
        try await Task.sleep(nanoseconds: 200_000_000) // simulate delay
        #endif
    }

    /// Read audio technical properties from a media file.
    ///
    /// Returns a human-readable string like "FLAC · 44100 Hz · 2ch · 5:23".
    func getAudioProperties(path: String) async throws -> String {
        #if MM_FFI_AVAILABLE
        return try await Task.detached(priority: .userInitiated) {
            let p = try getAudioProperties(path: path)
            let ext = URL(fileURLWithPath: path).pathExtension.uppercased()
            let mins = p.durationSecs / 60
            let secs = p.durationSecs % 60
            return "\(ext) · \(p.sampleRateHz) Hz · \(p.channels)ch · \(mins):\(String(format: "%02d", secs)) · \(p.bitrateKbps) kbps"
        }.value
        #else
        return stubAudioProperties(for: path)
        #endif
    }

    // MARK: – Template engine

    /// Validate a rename template string.
    ///
    /// - Parameter template: The template to validate.
    /// - Returns: `(isValid, errorMessage)` tuple.
    func validateTemplate(_ template: String) -> (isValid: Bool, message: String) {
        #if MM_FFI_AVAILABLE
        let result = validateTemplate(template: template)
        return (result.isValid, result.errorMessage)
        #else
        // Stub: basic syntax check (balanced angle brackets)
        return stubValidateTemplate(template)
        #endif
    }

    /// Return all recognised tag names for use in the rule builder.
    func listKnownTags() -> [String] {
        #if MM_FFI_AVAILABLE
        return listKnownTags()
        #else
        return ["Title", "Artist", "Album", "AlbumArtist", "Year", "Genre",
                "TrackNumber", "TrackTotal", "DiscNumber", "DiscTotal",
                "Composer", "Comment", "Lyrics", "ISRC", "Barcode",
                "CatalogNumber", "Label", "Compilation", "BPM",
                "Filename", "Extension", "Folder", "Duration",
                "BitrateKbps", "SampleRateHz", "MediaClass", "MediaFormat"]
        #endif
    }

    // MARK: – Test Mode

    /// Check whether test mode is currently enabled.
    ///
    /// In test mode, rename and write operations are staged in a
    /// temporary scratch area instead of touching real media files.
    /// - Returns: `true` if test mode is active.
    func testModeEnabled() -> Bool {
        #if MM_FFI_AVAILABLE
        // Real: call mm-ffi testModeEnabled()
        return testModeEnabled()
        #else
        // Stub: delegate to UserDefaults so the toggle persists across launches
        return UserDefaults.standard.bool(forKey: "mm_test_mode_enabled")
        #endif
    }

    /// Enable or disable test mode.
    ///
    /// When enabled, all file-system mutations are redirected to a
    /// staging directory; when disabled, the staging area may be
    /// committed or reverted via the corresponding functions.
    /// - Parameter enabled: `true` to activate test mode, `false` to deactivate.
    func setTestMode(enabled: Bool) {
        #if MM_FFI_AVAILABLE
        // Real: call mm-ffi setTestMode(enabled:)
        setTestMode(enabled: enabled)
        #else
        // Stub: persist the flag in UserDefaults for development UI
        UserDefaults.standard.set(enabled, forKey: "mm_test_mode_enabled")
        #endif
    }

    /// Return the number of files currently staged in test mode.
    ///
    /// A non-zero count means there are uncommitted rename or tag-write
    /// operations waiting in the staging area.
    /// - Returns: Count of staged files (0 when test mode is off).
    func testModeFileCount() -> Int {
        #if MM_FFI_AVAILABLE
        // Real: call mm-ffi testModeFileCount()
        return Int(testModeFileCount())
        #else
        // Stub: return a realistic number when test mode is on, 0 otherwise
        return UserDefaults.standard.bool(forKey: "mm_test_mode_enabled")
            ? UserDefaults.standard.integer(forKey: "mm_test_mode_file_count")
            : 0
        #endif
    }

    /// Commit all staged test-mode operations to real files.
    ///
    /// Moves renamed files from the staging area to their final destinations
    /// and applies any queued tag writes.  Resets the staged file count to 0.
    /// - Throws: If any staged operation fails to apply.
    func commitTestModeFiles() async throws {
        #if MM_FFI_AVAILABLE
        // Real: call mm-ffi commitTestModeFiles()
        try await Task.detached(priority: .userInitiated) {
            try commitTestModeFiles()
        }.value
        #else
        // Stub: simulate a commit delay, then reset the staged file count
        try await Task.sleep(nanoseconds: 500_000_000)
        UserDefaults.standard.set(0, forKey: "mm_test_mode_file_count")
        #endif
    }

    /// Revert all staged test-mode operations, discarding changes.
    ///
    /// Deletes all files in the staging area and resets the staged file
    /// count to 0 without applying any operations.
    /// - Throws: If the staging area cleanup fails.
    func revertTestModeFiles() async throws {
        #if MM_FFI_AVAILABLE
        // Real: call mm-ffi revertTestModeFiles()
        try await Task.detached(priority: .userInitiated) {
            try revertTestModeFiles()
        }.value
        #else
        // Stub: simulate a revert delay, then reset the staged file count
        try await Task.sleep(nanoseconds: 300_000_000)
        UserDefaults.standard.set(0, forKey: "mm_test_mode_file_count")
        #endif
    }

    // MARK: – Stubs (development-only, removed when FFI is available)

    #if !MM_FFI_AVAILABLE

    private func stubScanDirectory(directory: String, template: String) throws -> [FfiRenamePreview] {
        // Walk the directory and create placeholder previews
        let url = URL(fileURLWithPath: directory)
        let fm  = FileManager.default
        let audioExtensions = Set(["mp3", "flac", "m4a", "aac", "ogg", "opus", "wav"])

        guard let enumerator = fm.enumerator(at: url, includingPropertiesForKeys: nil) else {
            return []
        }

        var previews: [FfiRenamePreview] = []

        for case let fileURL as URL in enumerator {
            guard audioExtensions.contains(fileURL.pathExtension.lowercased()) else { continue }
            let src = fileURL.path
            // Placeholder destination (template not evaluated in stub)
            let dst = fileURL.deletingLastPathComponent()
                .appendingPathComponent("[Preview] \(fileURL.lastPathComponent)").path
            previews.append(FfiRenamePreview(source: src, destination: dst, conflict: false, unchanged: false))
        }

        return previews.prefix(50).sorted { $0.source < $1.source }
    }

    private func stubMetadata(for path: String) -> [FfiTagEntry] {
        // Return sample metadata so the UI is non-empty in development
        let ext = URL(fileURLWithPath: path).pathExtension.lowercased()
        var tags: [FfiTagEntry] = [
            FfiTagEntry(key: "title",        value: "Sample Track"),
            FfiTagEntry(key: "artist",       value: "Sample Artist"),
            FfiTagEntry(key: "album",        value: "Sample Album"),
            FfiTagEntry(key: "year",         value: "2024"),
            FfiTagEntry(key: "track_number", value: "1"),
            FfiTagEntry(key: "genre",        value: "Electronic"),
        ]
        if ["flac", "wav", "aiff"].contains(ext) {
            tags.append(FfiTagEntry(key: "comment", value: "Lossless format"))
        }
        return tags.sorted { $0.key < $1.key }
    }

    private func stubAudioProperties(for path: String) -> String {
        let ext = URL(fileURLWithPath: path).pathExtension.uppercased()
        return "\(ext) · 44100 Hz · 2ch · 3:42 · 320 kbps (stub)"
    }

    private func stubValidateTemplate(_ template: String) -> (isValid: Bool, message: String) {
        guard !template.trimmingCharacters(in: .whitespaces).isEmpty else {
            return (false, "Template must not be empty")
        }

        // Count angle brackets to detect basic balance errors
        var depth = 0
        for ch in template {
            if ch == "<" { depth += 1 }
            if ch == ">" { depth -= 1 }
            if depth < 0 { return (false, "Unexpected '>' without matching '<'") }
        }
        if depth != 0 { return (false, "Unmatched '<' — missing '>'") }

        return (true, "")
    }

    #endif // !MM_FFI_AVAILABLE
}
