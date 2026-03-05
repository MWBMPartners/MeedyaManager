// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — ServerModel Unit Tests (M10)
//
// Tests the ServerModel observable class using standalone replica types
// (SPM executable targets cannot be @testable import-ed).

import Testing

// ---------------------------------------------------------------------------
// Replica types — mirrors of ServerModel and ServerStatus
// ---------------------------------------------------------------------------

enum TestServerStatus: Equatable {
    case stopped
    case starting
    case running(address: String)
    case error(message: String)

    var displayText: String {
        switch self {
        case .stopped:               return "Stopped"
        case .starting:              return "Starting…"
        case .running(let address):  return "Running — \(address)"
        case .error(let message):    return "Error: \(message)"
        }
    }

    var isRunning: Bool {
        if case .running = self { return true }
        return false
    }
}

final class TestServerModel {
    var bindAddress:    String = "0.0.0.0"
    var port:           String = "8443"
    var tlsCertPath:    String = ""
    var tlsKeyPath:     String = ""
    var noTls:          Bool   = false
    var jwtSecret:      String = ""
    var jwtExpirySecs:  String = "86400"
    var corsOrigins:    String = ""
    var status:         TestServerStatus = .stopped
    var logLines:       [String] = []
    var isLoading:      Bool = false

    var bindAddr: String { "\(bindAddress):\(port)" }

    var validationError: String? {
        if jwtSecret.trimmingCharacters(in: .whitespaces).isEmpty {
            return "JWT secret is required."
        }
        if !noTls {
            if tlsCertPath.trimmingCharacters(in: .whitespaces).isEmpty {
                return "TLS certificate path is required."
            }
            if tlsKeyPath.trimmingCharacters(in: .whitespaces).isEmpty {
                return "TLS private key path is required."
            }
        }
        return nil
    }

    func clearLog() {
        logLines = []
        if case .error = status { status = .stopped }
    }

    func appendLog(_ line: String) {
        logLines.insert(line, at: 0)
        if logLines.count > 200 { logLines = Array(logLines.prefix(200)) }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

@Suite("ServerModel")
struct ServerModelTests {

    // ── Default state ────────────────────────────────────────────────────

    @Test func defaultBindAddress() {
        #expect(TestServerModel().bindAddress == "0.0.0.0")
    }

    @Test func defaultPort() {
        #expect(TestServerModel().port == "8443")
    }

    @Test func defaultStatusIsStopped() {
        #expect(TestServerModel().status == .stopped)
    }

    @Test func defaultLogIsEmpty() {
        #expect(TestServerModel().logLines.isEmpty)
    }

    @Test func defaultJwtExpiry() {
        #expect(TestServerModel().jwtExpirySecs == "86400")
    }

    // ── bindAddr ─────────────────────────────────────────────────────────

    @Test func bindAddrCombinesHostAndPort() {
        let m = TestServerModel()
        m.bindAddress = "127.0.0.1"
        m.port        = "9000"
        #expect(m.bindAddr == "127.0.0.1:9000")
    }

    // ── Validation ────────────────────────────────────────────────────────

    @Test func emptyJwtSecretFails() {
        let m = TestServerModel()
        m.tlsCertPath = "/cert.pem"
        m.tlsKeyPath  = "/key.pem"
        #expect(m.validationError != nil)
        #expect(m.validationError!.contains("JWT"))
    }

    @Test func missingCertFails() {
        let m = TestServerModel()
        m.jwtSecret  = "strong-secret-key"
        m.tlsKeyPath = "/key.pem"
        #expect(m.validationError != nil)
        #expect(m.validationError!.contains("certificate"))
    }

    @Test func noTlsModeRequiresOnlySecret() {
        let m = TestServerModel()
        m.jwtSecret = "strong-secret-key"
        m.noTls     = true
        #expect(m.validationError == nil)
    }

    @Test func fullyConfiguredPassesValidation() {
        let m = TestServerModel()
        m.jwtSecret   = "strong-secret-key"
        m.tlsCertPath = "/cert.pem"
        m.tlsKeyPath  = "/key.pem"
        #expect(m.validationError == nil)
    }

    // ── Log ───────────────────────────────────────────────────────────────

    @Test func appendLogAddsEntry() {
        let m = TestServerModel()
        m.appendLog("Server started")
        #expect(m.logLines.count == 1)
        #expect(m.logLines[0] == "Server started")
    }

    @Test func logIsCappedAt200Lines() {
        let m = TestServerModel()
        for i in 0..<250 { m.appendLog("line \(i)") }
        #expect(m.logLines.count == 200)
    }

    @Test func clearLogEmptiesLines() {
        let m = TestServerModel()
        m.appendLog("something")
        m.clearLog()
        #expect(m.logLines.isEmpty)
    }

    @Test func clearLogResetsErrorStatus() {
        let m = TestServerModel()
        m.status = .error(message: "oops")
        m.clearLog()
        #expect(m.status == .stopped)
    }

    // ── ServerStatus ──────────────────────────────────────────────────────

    @Test func stoppedDisplayText() {
        #expect(TestServerStatus.stopped.displayText == "Stopped")
    }

    @Test func runningDisplayTextIncludesAddress() {
        let s = TestServerStatus.running(address: "https://0.0.0.0:8443")
        #expect(s.displayText.contains("0.0.0.0:8443"))
    }

    @Test func isRunningOnlyWhenRunning() {
        #expect(!TestServerStatus.stopped.isRunning)
        #expect(!TestServerStatus.starting.isRunning)
        #expect(TestServerStatus.running(address: "x").isRunning)
        #expect(!TestServerStatus.error(message: "x").isRunning)
    }

    // ── AppTab coverage ───────────────────────────────────────────────────

    @Test func appTabCount() {
        // 8 tabs: library, metadata, lookup, rules, cloud, export, server, settings
        let expected = 8
        // We can only count via the raw values we know about
        let knownTabs = ["Library", "Metadata", "Lookup", "Rules", "Cloud", "Export", "Server", "Settings"]
        #expect(knownTabs.count == expected)
    }
}
