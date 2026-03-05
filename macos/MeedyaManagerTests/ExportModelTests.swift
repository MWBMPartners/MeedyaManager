// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — ExportModel unit tests (macOS, M9)
//
// Tests standalone replicas of ExportModel logic without requiring a
// running SwiftUI host (Swift Testing framework, no @MainActor needed).

import Testing
@testable import MeedyaManager   // excluded types replicated below

// ── Replicas (SPM executableTarget cannot be @testable-imported) ─────────────

private enum TestBackend: String, CaseIterable, Identifiable {
    case sqlite   = "SQLite"
    case mysql    = "MySQL"
    case mariadb  = "MariaDB"
    case postgres = "PostgreSQL"
    case mssql    = "SQL Server"
    var id: String { rawValue }

    var exampleDSN: String {
        switch self {
        case .sqlite:   return "sqlite:///Users/you/library.db"
        case .mysql:    return "mysql://user:pass@localhost/meedya"
        case .mariadb:  return "mysql://user:pass@localhost/meedya"
        case .postgres: return "postgres://user:pass@localhost/meedya"
        case .mssql:    return "server=tcp:host,1433;database=meedya;user=sa;password=P"
        }
    }
}

private class TestExportModel {
    var backend: TestBackend = .sqlite
    var connectionString: String = ""
    var tablePrefix: String = "mm_"
    var batchSize: Int = 500
    var dryRun: Bool = false
    var exportStatus: String = "idle"
    var resultMessage: String = ""
    var logLines: [String] = []

    var isExporting: Bool { exportStatus == "exporting" }

    func clearLog() {
        logLines.removeAll()
        exportStatus  = "idle"
        resultMessage = ""
    }

    func runExport() {
        guard !connectionString.trimmingCharacters(in: .whitespaces).isEmpty else {
            exportStatus  = "error"
            resultMessage = "Please enter a connection string before exporting."
            return
        }
        exportStatus = "exporting"
        logLines.append("Starting export to \(backend.rawValue)…")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

@Suite("ExportModel")
struct ExportModelTests {

    @Test func defaultBackendIsSQLite() {
        let m = TestExportModel()
        #expect(m.backend == .sqlite)
    }

    @Test func defaultStatusIsIdle() {
        let m = TestExportModel()
        #expect(m.exportStatus == "idle")
        #expect(!m.isExporting)
    }

    @Test func defaultPrefixIsMmUnderscore() {
        let m = TestExportModel()
        #expect(m.tablePrefix == "mm_")
    }

    @Test func defaultBatchSizeIs500() {
        let m = TestExportModel()
        #expect(m.batchSize == 500)
    }

    @Test func runExportEmptyDsnSetsError() {
        let m = TestExportModel()
        m.connectionString = ""
        m.runExport()
        #expect(m.exportStatus == "error")
        #expect(!m.resultMessage.isEmpty)
    }

    @Test func runExportValidDsnSetsExporting() {
        let m = TestExportModel()
        m.connectionString = "sqlite://:memory:"
        m.runExport()
        #expect(m.exportStatus == "exporting")
    }

    @Test func runExportAppendsToLog() {
        let m = TestExportModel()
        m.connectionString = "sqlite://:memory:"
        m.runExport()
        #expect(!m.logLines.isEmpty)
    }

    @Test func clearLogResetsState() {
        let m = TestExportModel()
        m.connectionString = "sqlite://:memory:"
        m.runExport()
        m.clearLog()
        #expect(m.logLines.isEmpty)
        #expect(m.exportStatus == "idle")
        #expect(m.resultMessage.isEmpty)
    }

    @Test func backendCount() {
        #expect(TestBackend.allCases.count == 5)
    }

    @Test func allBackendsHaveExampleDSN() {
        for b in TestBackend.allCases {
            #expect(!b.exampleDSN.isEmpty, "missing example DSN for \(b.rawValue)")
        }
    }

    @Test func sqliteExampleDsnContainsSqlite() {
        #expect(TestBackend.sqlite.exampleDSN.contains("sqlite"))
    }

    @Test func postgresExampleDsnContainsPostgres() {
        #expect(TestBackend.postgres.exampleDSN.contains("postgres"))
    }
}
