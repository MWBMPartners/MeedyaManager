// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Metadata Lookup Providers
//
// This crate implements the metadata lookup system for MeedyaManager, supporting
// 19 providers across four categories: music, video, podcasts, and identifiers.
// Each provider module implements the `MetadataProvider` trait defined in the
// `traits` module, allowing uniform querying and result aggregation.
//
// Provider categories:
//   - Music:       MusicBrainz, Discogs, Spotify, Last.fm, AcoustID, Deezer, Tidal, iTunes
//   - Video:       TMDb, OMDb, TVDb, IMDb (via OMDb)
//   - Podcasts:    Podcast Index, Apple Podcasts, Spotify Podcasts
//   - Identifiers: ISRC, ISWC, GRid, ISNI

// --- Module declarations ---

/// Shared traits defining the `MetadataProvider` interface and result types.
pub mod traits;

/// Provider registry — discovers, registers, and dispatches to available providers.
pub mod registry;

/// Secure credential management for API keys, OAuth tokens, and secrets.
pub mod credentials;

/// Rate limiter wrapping `governor` to enforce per-provider API quotas.
pub mod rate_limiter;

/// Fuzzy match scoring utilities for ranking metadata search results.
pub mod match_scoring;

/// Cover art retrieval and caching across providers.
pub mod cover_art;

/// Music metadata providers (MusicBrainz, Discogs, Spotify, Last.fm, etc.).
pub mod music;

/// Video metadata providers (TMDb, OMDb, TVDb).
pub mod video;

/// Podcast metadata providers (Podcast Index, Apple Podcasts, Spotify Podcasts).
pub mod podcasts;

/// Identifier lookup services (ISRC, ISWC, GRid, ISNI).
pub mod identifiers;

// --- Unit tests ---

#[cfg(test)]
mod tests {
    /// Smoke test to verify the crate compiles and the module tree is valid.
    #[test]
    fn providers_crate_loads() {
        // This test simply confirms that the crate links correctly.
        // Detailed provider tests live in each submodule and in integration tests.
        assert!(true, "mm-providers crate loaded successfully");
    }
}
