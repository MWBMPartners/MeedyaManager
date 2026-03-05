// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — LookupResult Unit Tests
//
// Tests the computed display properties of LookupResult:
//   - displayTitle fallback
//   - displaySubtitle composition (artist, album, year combinations)

import Testing
import Foundation

// MARK: – LookupResult replica

/// Replica of the production struct for test isolation.
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

    var displayTitle: String {
        title ?? "(Untitled)"
    }

    var displaySubtitle: String {
        var parts: [String] = []
        if let a  = artist { parts.append(a) }
        if let al = album  { parts.append(al) }
        let artistAlbum = parts.joined(separator: " — ")
        if let y = year {
            return artistAlbum.isEmpty ? "(\(y))" : "\(artistAlbum) (\(y))"
        }
        return artistAlbum
    }
}

// MARK: – Tests

@Suite("LookupResult")
struct LookupResultTests {

    func make(
        title: String? = "Comfortably Numb",
        artist: String? = "Pink Floyd",
        album: String? = "The Wall",
        year: Int? = 1979,
        genre: String? = "Rock",
        score: Double = 0.95
    ) -> LookupResult {
        LookupResult(
            provider:   "musicbrainz",
            title:      title,
            artist:     artist,
            album:      album,
            year:       year,
            genre:      genre,
            providerId: "mb-001",
            score:      score,
            coverArtUrl: nil
        )
    }

    @Test("displayTitle returns title when set")
    func displayTitle_present() {
        #expect(make(title: "Bohemian Rhapsody").displayTitle == "Bohemian Rhapsody")
    }

    @Test("displayTitle falls back to (Untitled) when nil")
    func displayTitle_nil_fallback() {
        #expect(make(title: nil).displayTitle == "(Untitled)")
    }

    @Test("displaySubtitle with artist and album and year")
    func displaySubtitle_full() {
        let r = make(artist: "Pink Floyd", album: "The Wall", year: 1979)
        #expect(r.displaySubtitle == "Pink Floyd — The Wall (1979)")
    }

    @Test("displaySubtitle with artist only")
    func displaySubtitle_artist_only() {
        let r = make(artist: "Pink Floyd", album: nil, year: nil)
        #expect(r.displaySubtitle == "Pink Floyd")
    }

    @Test("displaySubtitle with year only")
    func displaySubtitle_year_only() {
        let r = make(artist: nil, album: nil, year: 1979)
        #expect(r.displaySubtitle == "(1979)")
    }

    @Test("displaySubtitle is empty when all nil")
    func displaySubtitle_all_nil() {
        let r = make(artist: nil, album: nil, year: nil)
        #expect(r.displaySubtitle.isEmpty)
    }

    @Test("displaySubtitle includes album without artist")
    func displaySubtitle_album_no_artist() {
        let r = make(artist: nil, album: "The Wall", year: nil)
        #expect(r.displaySubtitle == "The Wall")
    }

    @Test("displaySubtitle artist and album without year")
    func displaySubtitle_artist_album_no_year() {
        let r = make(artist: "Pink Floyd", album: "The Wall", year: nil)
        #expect(r.displaySubtitle == "Pink Floyd — The Wall")
    }

    @Test("each LookupResult has a unique id")
    func unique_ids() {
        let a = make()
        let b = make()
        #expect(a.id != b.id)
    }

    @Test("score is stored correctly")
    func score_stored() {
        let r = make(score: 0.87)
        #expect(r.score == 0.87)
    }
}
