// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Database Export
//
// This crate implements the database export system for MeedyaManager (Milestone 9).
// It exports media library metadata to relational databases, supporting schema
// generation, incremental updates, and full re-exports.
//
// Supported databases:
//   - MySQL      (via sqlx)
//   - MariaDB    (via sqlx MySQL driver, MariaDB-compatible)
//   - PostgreSQL (via sqlx)
//   - SQLite     (via sqlx)
//   - SQL Server (via tiberius, TDS protocol)

// --- Module declarations ---

/// Shared traits defining the `DatabaseExporter` interface for all backends.
pub mod traits;

/// Schema generation and migration utilities for target databases.
pub mod schema;

/// MySQL exporter implementation.
pub mod mysql;

/// PostgreSQL exporter implementation.
pub mod postgres;

/// SQLite exporter implementation.
pub mod sqlite;

/// MariaDB exporter implementation (extends MySQL with MariaDB-specific features).
pub mod mariadb;

/// Microsoft SQL Server exporter implementation (via Tiberius / TDS).
pub mod mssql;

// --- Unit tests ---

#[cfg(test)]
mod tests {
    /// Smoke test to verify the crate compiles and the module tree is valid.
    #[test]
    fn export_crate_loads() {
        // Confirms that the mm-export crate links correctly.
        // Database-specific tests live in each submodule and integration tests.
        assert!(true, "mm-export crate loaded successfully");
    }
}
