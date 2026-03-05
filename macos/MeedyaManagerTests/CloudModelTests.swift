// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — CloudModel Unit Tests
//
// Tests the CloudProviderEntry view-model and CloudModel state logic used
// by CloudView.  No SwiftUI or WinUI dependency — pure Swift logic.
//
// SPM .testTarget cannot @testable import the .executableTarget, so the
// types under test are replicated here as standalone structs/classes.

import Testing
import Foundation

// MARK: – Replicas

struct CloudProviderEntry: Identifiable {
    let id          : String
    let label       : String
    var isConnected : Bool   = false
    var syncStatus  : String = "Not Connected"
    var rootFolder  : String = "/Music"
    let isStub      : Bool   = false
}

final class CloudModel {

    var providers: [CloudProviderEntry]
    var eventLog: [String] = []

    init() {
        providers = [
            CloudProviderEntry(id: "onedrive",    label: "OneDrive"),
            CloudProviderEntry(id: "googledrive", label: "Google Drive"),
            CloudProviderEntry(id: "dropbox",     label: "Dropbox"),
            CloudProviderEntry(id: "mega",        label: "MEGA",         syncStatus: "Coming Soon"),
            CloudProviderEntry(id: "icloud",      label: "iCloud Drive", syncStatus: "macOS native"),
        ]
    }

    func connect(id: String) {
        guard let idx = providers.firstIndex(where: { $0.id == id }) else { return }
        providers[idx].isConnected  = true
        providers[idx].syncStatus   = "Syncing…"
        appendEvent("[\(providers[idx].label)] Connecting…")
        providers[idx].syncStatus   = "Synced"
        appendEvent("[\(providers[idx].label)] Connected")
    }

    func disconnect(id: String) {
        guard let idx = providers.firstIndex(where: { $0.id == id }) else { return }
        providers[idx].isConnected = false
        providers[idx].syncStatus  = "Not Connected"
        appendEvent("[\(providers[idx].label)] Disconnected")
    }

    func appendEvent(_ message: String) {
        eventLog.insert(message, at: 0)
        if eventLog.count > 50 { eventLog = Array(eventLog.prefix(50)) }
    }
}

// MARK: – Tests

@Suite("CloudModel")
struct CloudModelTests {

    @Test("Default providers list has 5 entries")
    func default_providers_has_five() {
        #expect(CloudModel().providers.count == 5)
    }

    @Test("All providers start disconnected")
    func all_start_disconnected() {
        let m = CloudModel()
        #expect(m.providers.allSatisfy { !$0.isConnected })
    }

    @Test("connect marks provider as connected")
    func connect_marks_connected() {
        let m = CloudModel()
        m.connect(id: "onedrive")
        #expect(m.providers.first(where: { $0.id == "onedrive" })?.isConnected == true)
    }

    @Test("connect sets status to Synced")
    func connect_sets_status_synced() {
        let m = CloudModel()
        m.connect(id: "dropbox")
        #expect(m.providers.first(where: { $0.id == "dropbox" })?.syncStatus == "Synced")
    }

    @Test("disconnect marks provider as not connected")
    func disconnect_marks_not_connected() {
        let m = CloudModel()
        m.connect(id: "googledrive")
        m.disconnect(id: "googledrive")
        #expect(m.providers.first(where: { $0.id == "googledrive" })?.isConnected == false)
    }

    @Test("disconnect sets status to Not Connected")
    func disconnect_sets_status_not_connected() {
        let m = CloudModel()
        m.connect(id: "onedrive")
        m.disconnect(id: "onedrive")
        #expect(m.providers.first(where: { $0.id == "onedrive" })?.syncStatus == "Not Connected")
    }

    @Test("event log is non-empty after connect")
    func event_log_non_empty_after_connect() {
        let m = CloudModel()
        m.connect(id: "onedrive")
        #expect(!m.eventLog.isEmpty)
    }

    @Test("event log capped at 50 entries")
    func event_log_capped_at_50() {
        let m = CloudModel()
        for _ in 0..<60 {
            m.appendEvent("test event")
        }
        #expect(m.eventLog.count == 50)
    }

    @Test("connect with unknown id does nothing")
    func connect_unknown_id_noop() {
        let m = CloudModel()
        m.connect(id: "nonexistent")
        #expect(m.providers.allSatisfy { !$0.isConnected })
    }

    @Test("provider ids are unique")
    func provider_ids_are_unique() {
        let ids = CloudModel().providers.map { $0.id }
        #expect(Set(ids).count == ids.count)
    }

    @Test("onedrive is the first provider")
    func onedrive_is_first() {
        #expect(CloudModel().providers.first?.id == "onedrive")
    }
}
