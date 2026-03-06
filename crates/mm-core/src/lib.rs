// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Core Business Logic
//
// This crate contains all core business logic for MeedyaManager:
// - Configuration loading (JSON5 + .env)
// - File system watching (notify + polling fallback)
// - Media classification (4-level hierarchy)
// - Rule engine (MusicBee-inspired template parser/evaluator)
// - Rename simulation and execution
// - Metadata extraction and writing (lofty)
// - Companion file tracking
// - Application state management
// - Structured logging with PII redaction
// - Health checks
//
// License: GPL-2.0-or-later

/// Configuration loading and management
pub mod config;

/// Media classification engine (Group → Format → Class → Quality)
pub mod classify;

/// File system watcher with retry queue
pub mod watcher;

/// MusicBee-inspired rule engine (lexer → parser → evaluator)
pub mod rule_engine;

/// File rename simulation and execution
pub mod renamer;

/// Companion file detection and grouping
pub mod companion;

/// Metadata extraction and tag writing
pub mod metadata;

/// Application state persistence and crash recovery
pub mod state;

/// Structured logging with PII redaction
pub mod logging;

/// Startup health checks
pub mod health;

/// Unified error types for the core crate
pub mod error;

/// Internationalisation — gettext initialisation and locale helpers
pub mod i18n;

/// File type registry — single source of truth for all recognised extensions,
/// MIME types, companion scopes, and subtitle kinds
pub mod filetype_registry;

#[cfg(test)]
mod tests {
    #[test]
    fn core_crate_loads() {
        // Stub test — verifies the crate compiles and loads
        assert!(true);
    }
}
