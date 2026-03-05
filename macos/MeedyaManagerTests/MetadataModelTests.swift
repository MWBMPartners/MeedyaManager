// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — MetadataModel Logic Unit Tests
//
// Tests the pure logic of MetadataModel:
//   - applyLookupResult: merges lookup data into the tag map
//   - updateTag: updates existing tag values
//   - TagItem struct properties

import Testing
import Foundation

// MARK: – Minimal replicas (no SwiftUI / MmCore dependency)

struct TagItem: Identifiable {
    let id    = UUID()
    let key:   String
    var value: String
}

/// Pure logic extracted from MetadataModel, isolated for testing.
final class MetadataModelLogic {

    var tags:           [TagItem] = []
    var hasUnsavedEdits: Bool = false
    var coverArtUrl:     String? = nil
    var status:          String = "Select a media file to view its metadata."

    func updateTag(key: String, newValue: String) {
        if let idx = tags.firstIndex(where: { $0.key == key }) {
            tags[idx].value = newValue
            hasUnsavedEdits = true
        }
    }

    func applyLookupResult(
        title:  String?,
        artist: String?,
        album:  String?,
        year:   String?,
        genre:  String?
    ) {
        func set(_ key: String, _ value: String?) {
            guard let value, !value.isEmpty else { return }
            if let idx = tags.firstIndex(where: { $0.key.lowercased() == key }) {
                tags[idx].value = value
            } else {
                tags.append(TagItem(key: key, value: value))
            }
            hasUnsavedEdits = true
        }
        set("title",  title)
        set("artist", artist)
        set("album",  album)
        set("year",   year)
        set("genre",  genre)

        if hasUnsavedEdits {
            status = "Lookup result applied — review and Save."
        }
    }
}

// MARK: – Tests

@Suite("MetadataModel logic")
struct MetadataModelTests {

    func makeModel(withTags tags: [(String, String)] = []) -> MetadataModelLogic {
        let m = MetadataModelLogic()
        m.tags = tags.map { TagItem(key: $0.0, value: $0.1) }
        return m
    }

    @Test("initial state: no unsaved edits")
    func initial_no_unsaved_edits() {
        #expect(MetadataModelLogic().hasUnsavedEdits == false)
    }

    @Test("initial state: tags empty")
    func initial_tags_empty() {
        #expect(MetadataModelLogic().tags.isEmpty)
    }

    @Test("initial state: coverArtUrl nil")
    func initial_coverArtUrl_nil() {
        #expect(MetadataModelLogic().coverArtUrl == nil)
    }

    @Test("updateTag updates existing tag value")
    func updateTag_updates_value() {
        let m = makeModel(withTags: [("artist", "Pink Floyd")])
        m.updateTag(key: "artist", newValue: "Queen")
        #expect(m.tags.first(where: { $0.key == "artist" })?.value == "Queen")
    }

    @Test("updateTag sets hasUnsavedEdits to true")
    func updateTag_sets_dirty() {
        let m = makeModel(withTags: [("title", "Track 1")])
        m.updateTag(key: "title", newValue: "Track 2")
        #expect(m.hasUnsavedEdits == true)
    }

    @Test("updateTag does nothing for unknown key")
    func updateTag_unknown_key_no_change() {
        let m = makeModel(withTags: [("artist", "X")])
        m.updateTag(key: "unknown", newValue: "Y")
        #expect(m.tags.count == 1)
        #expect(m.hasUnsavedEdits == false)
    }

    @Test("applyLookupResult adds new tag when key absent")
    func apply_adds_new_tag() {
        let m = makeModel()
        m.applyLookupResult(title: "Comfortably Numb", artist: nil, album: nil, year: nil, genre: nil)
        #expect(m.tags.contains(where: { $0.key == "title" && $0.value == "Comfortably Numb" }))
    }

    @Test("applyLookupResult updates existing tag (case-insensitive key match)")
    func apply_updates_existing_tag() {
        let m = makeModel(withTags: [("Title", "Old Title")])
        m.applyLookupResult(title: "New Title", artist: nil, album: nil, year: nil, genre: nil)
        #expect(m.tags.first(where: { $0.key == "Title" })?.value == "New Title")
    }

    @Test("applyLookupResult sets hasUnsavedEdits when value applied")
    func apply_sets_dirty() {
        let m = makeModel()
        m.applyLookupResult(title: "Hello", artist: nil, album: nil, year: nil, genre: nil)
        #expect(m.hasUnsavedEdits == true)
    }

    @Test("applyLookupResult skips nil values")
    func apply_skips_nil() {
        let m = makeModel()
        m.applyLookupResult(title: nil, artist: nil, album: nil, year: nil, genre: nil)
        #expect(m.tags.isEmpty)
        #expect(m.hasUnsavedEdits == false)
    }

    @Test("applyLookupResult skips empty string values")
    func apply_skips_empty() {
        let m = makeModel()
        m.applyLookupResult(title: "", artist: "", album: "", year: "", genre: "")
        #expect(m.tags.isEmpty)
        #expect(m.hasUnsavedEdits == false)
    }

    @Test("applyLookupResult applies all five fields")
    func apply_all_five_fields() {
        let m = makeModel()
        m.applyLookupResult(
            title:  "Track",
            artist: "Artist",
            album:  "Album",
            year:   "2024",
            genre:  "Rock"
        )
        #expect(m.tags.count == 5)
    }

    @Test("applyLookupResult updates status message")
    func apply_updates_status() {
        let m = makeModel()
        m.applyLookupResult(title: "X", artist: nil, album: nil, year: nil, genre: nil)
        #expect(m.status == "Lookup result applied — review and Save.")
    }

    @Test("TagItem has unique id")
    func tagItem_unique_id() {
        let a = TagItem(key: "artist", value: "X")
        let b = TagItem(key: "artist", value: "X")
        #expect(a.id != b.id)
    }

    @Test("TagItem stores key and value")
    func tagItem_stores_kv() {
        let t = TagItem(key: "album", value: "The Wall")
        #expect(t.key == "album")
        #expect(t.value == "The Wall")
    }
}
