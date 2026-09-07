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

// MARK: – Engine availability (issue #222)
//
// `MM_FFI_AVAILABLE` is only ever set once the Rust core is actually linked
// into this build (see Package.swift's comment on line 73 — nothing defines
// it today, so every build of this app is currently running on stubs).
// Everything below exists so that an unlinked build is *incapable* of moving,
// copying or deleting a real file, rather than relying on a warning nobody
// may ever see.

/// Thrown by every MmCore method that would otherwise need to touch the
/// Rust engine, when that engine is not present in this build.
///
/// Before this fix, the "no engine" case silently fell back to inventing
/// data (fake scan previews, fake metadata) that the rest of the app then
/// treated as real and safe to act on — including renaming actual files on
/// disk. Throwing here removes that whole failure mode: there is no code
/// path left that manufactures a result the UI could mistake for genuine.
struct EngineUnavailableError: LocalizedError {
    var errorDescription: String? {
        "This build of MeedyaManager does not include its engine (the Rust core is not linked, issue #66), so this action is unavailable. Nothing on disk has been changed."
    }
}

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

    /// Whether this build actually links the Rust engine (mm-ffi).
    ///
    /// A `static let` rather than a stored instance property, so it does not
    /// disturb the "no mutable stored properties" invariant above — it is
    /// computed once from a compile-time flag and never changes at runtime.
    /// Every screen that could act on engine output (scan previews, tag
    /// edits, renames) must check this before offering that action, and must
    /// refuse plainly rather than substitute fabricated data. See issue #222.
    static let isEngineAvailable: Bool = {
        #if MM_FFI_AVAILABLE
        return true
        #else
        return false
        #endif
    }()

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
        // No engine linked: refuse outright. This used to walk the folder
        // with FileManager and invent a "Preview"-prefixed destination for
        // every audio file, without ever reading the user's template. Every
        // one of those fabricated previews was then flagged safe to execute,
        // so pressing Execute renamed real files to meaningless names — see
        // issue #222. There is no honest stand-in for a real scan, so this
        // now throws instead of guessing.
        throw EngineUnavailableError()
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
        // No engine linked: refuse rather than return the old "Sample Track"
        // placeholder tags, which looked like a real file's metadata and
        // were not (issue #222).
        throw EngineUnavailableError()
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
        // No engine linked: refuse rather than silently pretend the write
        // happened. The old stub slept for 200ms and returned success,
        // which told the user their tags were saved when nothing was
        // written anywhere (issue #222).
        throw EngineUnavailableError()
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
        // No engine linked: refuse rather than return the old made-up
        // "44100 Hz · 2ch · 3:42 · 320 kbps (stub)" string, which read like
        // a genuine measurement of the file (issue #222).
        throw EngineUnavailableError()
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
        // No engine linked: there is no staging area to count files in, so
        // the honest answer is always zero — not a made-up number read back
        // from UserDefaults (issue #222).
        return 0
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
        // No engine linked: there is nothing staged to commit, so refuse
        // rather than pretend a commit happened after a fake delay
        // (issue #222).
        throw EngineUnavailableError()
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
        // No engine linked: there is nothing staged to revert, so refuse
        // rather than pretend a revert happened after a fake delay
        // (issue #222).
        throw EngineUnavailableError()
        #endif
    }

    // MARK: – Renaming (the only place allowed to move a file — issue #222)

    /// Move each of `previews`'s source files to its destination for real.
    ///
    /// This is the single method in the whole app permitted to touch the
    /// filesystem for a rename — `ScanModel` used to move files via the
    /// file manager itself, straight after a stub had
    /// invented the destination names, which is exactly how issue #222's
    /// data loss happened. Routing every rename through here, on top of the
    /// stub no longer producing anything executable, means there is no
    /// second path back into the same mistake.
    ///
    /// Named `applyRenames` (not `executeRenames`) because the UniFFI
    /// bindings below already generate a free function of that name, and
    /// Swift resolves an unqualified call to the nearer instance method
    /// first — silently shadowing the real one. Keeping the names distinct
    /// avoids that trap entirely rather than relying on careful call-site
    /// qualification.
    ///
    /// - Returns: The number of files the engine actually renamed.
    func applyRenames(_ previews: [FfiRenamePreview]) async throws -> Int {
        #if MM_FFI_AVAILABLE
        // UNVERIFIED — cannot be compiled or exercised on this machine, since
        // MM_FFI_AVAILABLE is never defined anywhere yet (issue #66) and the
        // generated bindings this depends on are excluded from the build
        // (see Package.swift's comment on `Bindings/generated`). Written to
        // mirror the existing `scanDirectory` real-branch pattern above so
        // that wiring it up is a small, obvious change once the XCFramework
        // is actually linked, not a redesign.
        return try await Task.detached(priority: .userInitiated) {
            let ffi = previews.map { RenamePreviewFfi(source: $0.source, destination: $0.destination, conflict: $0.conflict, unchanged: $0.unchanged) }
            return Int(try executeRenames(previews: ffi))
        }.value
        #else
        throw EngineUnavailableError()
        #endif
    }

    // MARK: – Stubs (development-only, removed when FFI is available)

    #if !MM_FFI_AVAILABLE

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
