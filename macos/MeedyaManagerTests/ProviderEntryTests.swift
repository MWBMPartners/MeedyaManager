// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — ProviderEntry + LookupModel logic Unit Tests
//
// Tests the provider list initialisation and toggle behaviour independently
// of SwiftUI and MmCore.

import Testing
import Foundation

// MARK: – Minimal replicas

struct ProviderEntry: Identifiable {
    let id:      String
    let label:   String
    var enabled: Bool
    var isStub:  Bool
}

/// Pure logic extracted from LookupModel, isolated for testing.
struct LookupModelLogic {

    var providers: [ProviderEntry]

    init() {
        providers = Self.defaultProviders()
    }

    var enabledCount: Int  { providers.filter(\.enabled).count }
    var stubCount:    Int  { providers.filter(\.isStub).count  }
    var concreteCount: Int { providers.filter { !$0.isStub }.count }

    mutating func toggleProvider(id: String) {
        if let idx = providers.firstIndex(where: { $0.id == id }) {
            providers[idx].enabled.toggle()
        }
    }

    func isEnabled(_ id: String) -> Bool {
        providers.first(where: { $0.id == id })?.enabled ?? false
    }

    static func defaultProviders() -> [ProviderEntry] {
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

// MARK: – Tests

@Suite("LookupModel provider logic")
struct ProviderEntryTests {

    @Test("default provider list has 19 entries")
    func provider_count_is_19() {
        let logic = LookupModelLogic()
        #expect(logic.providers.count == 19)
    }

    @Test("13 concrete providers are enabled by default")
    func concrete_enabled_count() {
        let logic = LookupModelLogic()
        #expect(logic.enabledCount == 13)
    }

    @Test("6 stub providers are disabled by default")
    func stub_disabled_count() {
        let logic = LookupModelLogic()
        #expect(logic.stubCount == 6)
    }

    @Test("all stub providers start disabled")
    func stubs_disabled() {
        let logic = LookupModelLogic()
        let stubs = logic.providers.filter(\.isStub)
        #expect(stubs.allSatisfy { !$0.enabled })
    }

    @Test("all concrete providers start enabled")
    func concrete_enabled() {
        let logic = LookupModelLogic()
        let concrete = logic.providers.filter { !$0.isStub }
        // Use explicit closure rather than \.enabled keypath: Swift Testing's
        // #expect macro expansion confuses keypath-vs-rethrows analysis and
        // emits "call can throw, but it is not marked with 'try'" even
        // though neither side actually throws.
        #expect(concrete.allSatisfy { $0.enabled })
    }

    @Test("toggleProvider disables an enabled provider")
    func toggle_disables_enabled() {
        var logic = LookupModelLogic()
        #expect(logic.isEnabled("musicbrainz") == true)
        logic.toggleProvider(id: "musicbrainz")
        #expect(logic.isEnabled("musicbrainz") == false)
    }

    @Test("toggleProvider enables a disabled provider")
    func toggle_enables_disabled() {
        var logic = LookupModelLogic()
        #expect(logic.isEnabled("spotify") == true)
        logic.toggleProvider(id: "spotify")
        #expect(logic.isEnabled("spotify") == false)
        logic.toggleProvider(id: "spotify")
        #expect(logic.isEnabled("spotify") == true)
    }

    @Test("toggleProvider on unknown id does nothing")
    func toggle_unknown_id_no_change() {
        var logic = LookupModelLogic()
        let before = logic.providers.map(\.enabled)
        logic.toggleProvider(id: "nonexistent_provider_xyz")
        let after = logic.providers.map(\.enabled)
        #expect(before == after)
    }

    @Test("MusicBrainz is in the provider list")
    func musicbrainz_in_list() {
        let logic = LookupModelLogic()
        #expect(logic.providers.contains(where: { $0.id == "musicbrainz" }))
    }

    @Test("iHeart stub provider has correct id")
    func iheart_stub_id() {
        let logic = LookupModelLogic()
        let entry = logic.providers.first(where: { $0.id == "iheart" })
        #expect(entry != nil)
        #expect(entry?.isStub == true)
        #expect(entry?.enabled == false)
    }

    @Test("all provider ids are unique")
    func provider_ids_unique() {
        let logic = LookupModelLogic()
        let ids = logic.providers.map(\.id)
        let uniqueIds = Set(ids)
        #expect(ids.count == uniqueIds.count)
    }
}
