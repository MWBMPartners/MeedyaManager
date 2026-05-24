// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Metadata Lookup Observable Model (macOS, M6)
//
// LookupModel manages the provider search workflow:
//   1. User enters a query (title + optional artist)
//   2. search() is called — dispatches to MmCore FFI (or stubs)
//   3. Results are sorted by score and stored in `results`
//   4. Selected result is stored for apply-to-file

import SwiftUI
import Foundation

// MARK: – Data Types

/// A single provider search result.
struct LookupResult: Identifiable, Equatable {
    let id = UUID()
    var provider:    String
    var title:       String?
    var artist:      String?
    var album:       String?
    var year:        Int?
    var genre:       String?
    var providerId:  String
    var score:       Double
    var coverArtUrl: String?

    /// Human-readable title display string.
    var displayTitle: String {
        title ?? "(Untitled)"
    }

    /// Human-readable subtitle: "Artist — Album (Year)"
    var displaySubtitle: String {
        var parts: [String] = []
        if let a = artist { parts.append(a) }
        if let al = album  { parts.append(al) }
        let artistAlbum = parts.joined(separator: " — ")
        if let y = year {
            return artistAlbum.isEmpty ? "(\(y))" : "\(artistAlbum) (\(y))"
        }
        return artistAlbum
    }
}

/// Provider entry in the provider selector list.
struct ProviderEntry: Identifiable {
    let id:      String   // internal name, e.g. "musicbrainz"
    let label:   String   // display name, e.g. "MusicBrainz"
    var enabled: Bool
    var isStub:  Bool     // stub providers have no public API
}

// MARK: – LookupModel

/// Observable state for the Metadata Lookup panel.
@Observable
final class LookupModel {

    // MARK: Input
    var query:      String = ""
    var artistHint: String = ""

    // MARK: State
    var isSearching: Bool = false
    var statusMessage: String = "Enter a title to search for metadata."

    // MARK: Results
    var results:  [LookupResult] = []
    var selected: LookupResult?

    // MARK: Providers
    // Use concrete class name (not Self) — Swift 6 forbids covariant Self in
    // stored property initializers, even on final classes.
    var providers: [ProviderEntry] = LookupModel.defaultProviders()

    // MARK: – Search

    /// Perform a metadata search across all enabled providers.
    ///
    /// Dispatches asynchronously; updates are published on the MainActor.
    @MainActor
    func search() async {
        guard !query.trimmingCharacters(in: .whitespaces).isEmpty else {
            statusMessage = "⚠ Enter a title to search."
            return
        }

        isSearching   = true
        results       = []
        selected      = nil
        statusMessage = "Searching…"

        // The FFI layer (MmCore) does not yet expose provider search in M6.
        // We call a stub that returns a small set of synthetic results so the
        // UI layout and selection logic can be verified without live API calls.
        // Full wiring to mm-providers happens when the FFI exposes the search API.
        // Capture values explicitly so the detached closure doesn't
        // capture `self` (which is @MainActor-isolated and not Sendable).
        let mockResults = await Task.detached(priority: .userInitiated) { [query, artistHint] in
            Self.mockSearch(query: query, artist: artistHint)
        }.value

        results       = mockResults
        isSearching   = false
        statusMessage = mockResults.isEmpty
            ? "No results found."
            : "\(mockResults.count) result(s) found."
    }

    /// Clear all results and reset the input fields.
    @MainActor
    func clear() {
        results       = []
        selected      = nil
        query         = ""
        artistHint    = ""
        statusMessage = "Enter a title to search for metadata."
    }

    /// Toggle a provider's enabled state by its internal ID.
    func toggleProvider(id: String) {
        if let idx = providers.firstIndex(where: { $0.id == id }) {
            providers[idx].enabled.toggle()
        }
    }

    // MARK: – Private helpers

    /// Synthetic search results for UI development (replaced by real FFI in a later patch).
    private static func mockSearch(query: String, artist: String) -> [LookupResult] {
        guard !query.isEmpty else { return [] }
        // Return 3 mock results to exercise the results list layout
        return [
            LookupResult(
                provider: "musicbrainz",
                title: query,
                artist: artist.isEmpty ? "Unknown Artist" : artist,
                album: "Unknown Album",
                year: 2024,
                genre: "Rock",
                providerId: "mb-\(query.lowercased().prefix(6))",
                score: 0.95,
                coverArtUrl: nil
            ),
            LookupResult(
                provider: "spotify",
                title: query,
                artist: artist.isEmpty ? "Various Artists" : artist,
                album: nil,
                year: 2023,
                genre: nil,
                providerId: "sp-\(query.lowercased().prefix(6))",
                score: 0.78,
                coverArtUrl: nil
            ),
            LookupResult(
                provider: "apple_music",
                title: query + " (Remaster)",
                artist: artist.isEmpty ? "Artist" : artist,
                album: "Remastered Collection",
                year: 2022,
                genre: "Pop",
                providerId: "am-\(query.lowercased().prefix(6))",
                score: 0.62,
                coverArtUrl: nil
            ),
        ]
    }

    /// Default provider list: concrete providers on, stubs off.
    private static func defaultProviders() -> [ProviderEntry] {
        [
            ProviderEntry(id: "musicbrainz",   label: "MusicBrainz",    enabled: true,  isStub: false),
            ProviderEntry(id: "spotify",        label: "Spotify",         enabled: true,  isStub: false),
            ProviderEntry(id: "apple_music",    label: "Apple Music",     enabled: true,  isStub: false),
            ProviderEntry(id: "deezer",         label: "Deezer",          enabled: true,  isStub: false),
            ProviderEntry(id: "tmdb",           label: "TMDb",            enabled: true,  isStub: false),
            ProviderEntry(id: "thetvdb",        label: "TheTVDB",         enabled: true,  isStub: false),
            ProviderEntry(id: "omdb",           label: "OMDb",            enabled: true,  isStub: false),
            ProviderEntry(id: "apple_tv",       label: "Apple TV",        enabled: true,  isStub: false),
            ProviderEntry(id: "itunes_store",   label: "iTunes Store",    enabled: true,  isStub: false),
            ProviderEntry(id: "apple_podcasts", label: "Apple Podcasts",  enabled: true,  isStub: false),
            ProviderEntry(id: "isrc",           label: "ISRC",            enabled: true,  isStub: false),
            ProviderEntry(id: "eidr",           label: "EIDR",            enabled: true,  isStub: false),
            ProviderEntry(id: "iswc",           label: "ISWC",            enabled: true,  isStub: false),
            ProviderEntry(id: "youtube_music",  label: "YouTube Music*",  enabled: false, isStub: true),
            ProviderEntry(id: "amazon_music",   label: "Amazon Music*",   enabled: false, isStub: true),
            ProviderEntry(id: "pandora",        label: "Pandora*",        enabled: false, isStub: true),
            ProviderEntry(id: "tidal",          label: "Tidal*",          enabled: false, isStub: true),
            ProviderEntry(id: "shazam",         label: "Shazam*",         enabled: false, isStub: true),
            ProviderEntry(id: "iheart",         label: "iHeart*",         enabled: false, isStub: true),
        ]
    }
}
