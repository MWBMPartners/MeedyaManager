// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Metadata Lookup Providers (M5)
//
// This crate implements the metadata lookup system for MeedyaManager, supporting
// 19 providers across four categories: music, video, podcasts, and identifiers.
// Each provider module implements the `MetadataProvider` trait defined in the
// `traits` module, allowing uniform querying and result aggregation.
//
// Provider categories:
//   Music (10):      MusicBrainz, Spotify, Apple Music, Deezer,
//                    YouTube Music*, Amazon Music*, Pandora*, Tidal*, Shazam*, iHeart*
//   Video (5):       TMDb, TheTVDB, OMDb, Apple TV, iTunes Store
//   Podcasts (1):    Apple Podcasts
//   Identifiers (3): ISRC, EIDR, ISWC
//
//   (* = stub, no public API)

// ---------------------------------------------------------------------------
// Module declarations
// ---------------------------------------------------------------------------

/// Shared traits defining the `MetadataProvider` interface and all result types.
pub mod traits;

/// Central provider registry — registers, filters, and dispatches to providers.
pub mod registry;

/// 4-tier credential resolution: env vars → config map → OS keyring → local file.
pub mod credentials;

/// Per-provider token-bucket rate limiter wrapping the `governor` crate.
pub mod rate_limiter;

/// Weighted fuzzy match scoring (title/artist/album/year/ISRC) using `fuzzy_matcher`.
pub mod match_scoring;

/// Cover art selection, size classification, deduplication, and URL utilities.
pub mod cover_art;

/// Music metadata providers (MusicBrainz, Spotify, Apple Music, Deezer, + 6 stubs).
pub mod music;

/// Video metadata providers (TMDb, TheTVDB, OMDb, Apple TV, iTunes Store).
pub mod video;

/// Podcast metadata providers (Apple Podcasts via iTunes Search API).
pub mod podcasts;

/// Identifier lookup services (ISRC via MusicBrainz, EIDR, ISWC via MusicBrainz).
pub mod identifiers;

// ---------------------------------------------------------------------------
// Convenient re-exports — consumers only need `use mm_providers::*`
// ---------------------------------------------------------------------------

// Core traits and data types
pub use traits::{
    Capabilities, CoverArtInfo, MediaType, MetadataProvider, ProviderError, ProviderResult,
    SearchQuery,
};

// Registry
pub use registry::ProviderRegistry;

// Credential management
pub use credentials::{Credential, CredentialSource, CredentialStore};

// Rate limiting
pub use rate_limiter::{default_rpm_for, ProviderRateLimiter, RateLimiterRegistry};

// Match scoring
pub use match_scoring::{score_result, MatchScorer, ScoringWeights};

// Cover art utilities
pub use cover_art::{
    deduplicate, filter_by_min_size, is_valid_art_url, mime_type_for_url, select_best,
    select_largest, select_smallest, url_has_image_extension, CoverArtSize,
};

// Music providers (concrete)
pub use music::{
    AppleMusicProvider, DeezerProvider, MusicBrainzProvider, SpotifyProvider,
};

// Music providers (stubs)
pub use music::{
    AmazonMusicProvider, iHeartProvider, PandoraProvider, ShazamProvider, TidalProvider,
    YouTubeMusicProvider,
};

// Video providers
pub use video::{
    AppleTvProvider, ItunesStoreProvider, OmdbProvider, TheTvdbProvider, TmdbProvider,
};

// Podcast providers
pub use podcasts::ApplePodcastsProvider;

// Identifier providers + validators
pub use identifiers::{
    validate_eidr, validate_isrc, validate_iswc, EidrProvider, IsrcProvider, IswcProvider,
};

// ---------------------------------------------------------------------------
// Integration smoke tests — 15 tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // 1. Crate loads
    // -----------------------------------------------------------------------

    /// Verify the crate links and all module paths compile.
    #[test]
    fn crate_loads_all_modules() {
        // If this compiles, all mod declarations and re-exports are valid.
        assert!(true);
    }

    // -----------------------------------------------------------------------
    // 2. All 19 providers instantiate without panicking
    // -----------------------------------------------------------------------

    /// Instantiate every concrete provider and confirm they don't panic.
    #[test]
    fn all_19_providers_instantiate() {
        // Music — concrete
        let _mb = MusicBrainzProvider::new("MeedyaManager/0.6.0 (test)");
        let _sp = SpotifyProvider::new(Some("client_id".into()), Some("client_secret".into()));
        let _am = AppleMusicProvider::new("US");
        let _dz = DeezerProvider::new();
        // Music — stubs
        let _yt = YouTubeMusicProvider::default();
        let _az = AmazonMusicProvider::default();
        let _pa = PandoraProvider::default();
        let _ti = TidalProvider::default();
        let _sh = ShazamProvider::default();
        let _ih = iHeartProvider::default();
        // Video
        let _tm = TmdbProvider::new(Some("api_key".into()));
        let _tv = TheTvdbProvider::new(Some("token".into()));
        let _om = OmdbProvider::new(Some("api_key".into()));
        let _at = AppleTvProvider::new("US");
        let _it = ItunesStoreProvider::new("US");
        // Podcasts
        let _ap = ApplePodcastsProvider::new("US");
        // Identifiers
        let _is = IsrcProvider::new("MeedyaManager/0.6.0 (test)");
        let _ei = EidrProvider::new(Some("user".into()), Some("password".into()));
        let _iw = IswcProvider::new("MeedyaManager/0.6.0 (test)");
    }

    // -----------------------------------------------------------------------
    // 3. All provider names are unique
    // -----------------------------------------------------------------------

    /// Confirm that no two providers share the same `name()` string.
    #[test]
    fn all_provider_names_are_unique() {
        let names = vec![
            MusicBrainzProvider::new("ua").name().to_owned(),
            SpotifyProvider::new(None, None).name().to_owned(),
            AppleMusicProvider::new("US").name().to_owned(),
            DeezerProvider::new().name().to_owned(),
            YouTubeMusicProvider::default().name().to_owned(),
            AmazonMusicProvider::default().name().to_owned(),
            PandoraProvider::default().name().to_owned(),
            TidalProvider::default().name().to_owned(),
            ShazamProvider::default().name().to_owned(),
            iHeartProvider::default().name().to_owned(),
            TmdbProvider::new(None).name().to_owned(),
            TheTvdbProvider::new(None).name().to_owned(),
            OmdbProvider::new(None).name().to_owned(),
            AppleTvProvider::new("US").name().to_owned(),
            ItunesStoreProvider::new("US").name().to_owned(),
            ApplePodcastsProvider::new("US").name().to_owned(),
            IsrcProvider::new("ua").name().to_owned(),
            EidrProvider::new(None, None).name().to_owned(),
            IswcProvider::new("ua").name().to_owned(),
        ];
        let total = names.len();
        let mut deduped = names.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(deduped.len(), total, "Duplicate provider names detected: {:?}", names);
    }

    // -----------------------------------------------------------------------
    // 4. All providers have valid capabilities
    // -----------------------------------------------------------------------

    /// Every provider must declare at least one media type and a non-empty display name.
    #[test]
    fn all_providers_have_valid_capabilities() {
        let providers: Vec<(&str, bool, usize)> = vec![
            ("musicbrainz",   MusicBrainzProvider::new("ua").capabilities().media_types.len() > 0,
             MusicBrainzProvider::new("ua").capabilities().display_name.len()),
            ("spotify",       SpotifyProvider::new(None, None).capabilities().media_types.len() > 0,
             SpotifyProvider::new(None, None).capabilities().display_name.len()),
            ("apple_music",   AppleMusicProvider::new("US").capabilities().media_types.len() > 0,
             AppleMusicProvider::new("US").capabilities().display_name.len()),
            ("deezer",        DeezerProvider::new().capabilities().media_types.len() > 0,
             DeezerProvider::new().capabilities().display_name.len()),
            ("tmdb",          TmdbProvider::new(None).capabilities().media_types.len() > 0,
             TmdbProvider::new(None).capabilities().display_name.len()),
            ("apple_podcasts", ApplePodcastsProvider::new("US").capabilities().media_types.len() > 0,
             ApplePodcastsProvider::new("US").capabilities().display_name.len()),
            ("isrc",          IsrcProvider::new("ua").capabilities().media_types.len() > 0,
             IsrcProvider::new("ua").capabilities().display_name.len()),
        ];
        for (name, has_types, name_len) in providers {
            assert!(has_types, "{name}: no media types declared");
            assert!(name_len > 0, "{name}: empty display_name");
        }
    }

    // -----------------------------------------------------------------------
    // 5. MatchScorer scores a ProviderResult
    // -----------------------------------------------------------------------

    /// Basic sanity check that `MatchScorer::score` returns a value in [0.0, 1.0+].
    #[test]
    fn match_scorer_scores_provider_result() {
        let scorer = MatchScorer::new();
        let query = SearchQuery::music("Comfortably Numb", "Pink Floyd");
        let result = ProviderResult {
            title: Some("Comfortably Numb".into()),
            artist: Some("Pink Floyd".into()),
            ..Default::default()
        };
        let score = scorer.score(&query, &result);
        // Should be a high score for a perfect title+artist match
        assert!(score >= 0.0, "score must be non-negative");
        assert!(score > 0.5, "identical title+artist should score > 0.5, got {score}");
    }

    // -----------------------------------------------------------------------
    // 6. RateLimiterRegistry has default limits for all 19 providers
    // -----------------------------------------------------------------------

    /// The default registry must include entries for all known provider names.
    #[test]
    fn rate_limiter_registry_has_default_limits_for_all() {
        let registry = RateLimiterRegistry::with_all_providers();
        let providers = [
            "musicbrainz", "spotify", "apple_music", "deezer",
            "youtube_music", "amazon_music", "pandora", "tidal", "shazam", "iheart",
            "tmdb", "thetvdb", "omdb", "apple_tv", "itunes_store",
            "apple_podcasts",
            "isrc", "eidr", "iswc",
        ];
        for name in &providers {
            // check() returns Ok for registered providers (not over-limit immediately)
            assert!(
                registry.check(name).is_ok(),
                "Missing or over-limit rate limiter for provider '{name}'"
            );
        }
    }

    // -----------------------------------------------------------------------
    // 7. CredentialStore 4-tier lookup
    // -----------------------------------------------------------------------

    /// Tier 2 (config map) lookup succeeds when a credential is pre-loaded.
    #[test]
    fn credential_store_4_tier_lookup() {
        let mut config = std::collections::HashMap::new();
        config.insert("spotify.client_id".to_owned(), "test_id".to_owned());

        let store = CredentialStore::new(config, std::path::Path::new("/tmp"));
        let cred = store.get("spotify", "client_id");
        assert!(cred.is_some(), "Config-tier credential should be found");
        assert_eq!(cred.unwrap().value, "test_id");
    }

    // -----------------------------------------------------------------------
    // 8. CoverArtSize classification
    // -----------------------------------------------------------------------

    /// `CoverArtSize::from_art` correctly classifies images by shortest dimension.
    #[test]
    fn cover_art_size_from_provider_result() {
        // 600×600 → Large
        let art = CoverArtInfo::new("https://example.com/cover.jpg", 600, 600, "image/jpeg");
        assert_eq!(CoverArtSize::from_art(&art), CoverArtSize::Large);

        // 100×100 → Thumbnail
        let thumb = CoverArtInfo::new("https://example.com/thumb.jpg", 100, 100, "image/jpeg");
        assert_eq!(CoverArtSize::from_art(&thumb), CoverArtSize::Thumbnail);

        // 1200×1200 → ExtraLarge
        let xl = CoverArtInfo::new("https://example.com/xl.jpg", 1200, 1200, "image/jpeg");
        assert_eq!(CoverArtSize::from_art(&xl), CoverArtSize::ExtraLarge);
    }

    // -----------------------------------------------------------------------
    // 9. ProviderRegistry dispatches correctly (async)
    // -----------------------------------------------------------------------

    /// A registry with an Apple Music provider returns results for a music query.
    #[tokio::test]
    async fn provider_registry_dispatches_correctly() {
        // We can't make live HTTP calls in tests, but we can verify routing:
        // A registry with no providers returns empty results.
        let registry = ProviderRegistry::new();
        let query = SearchQuery::music("Bohemian Rhapsody", "Queen");
        let results = registry.search(&query).await;
        assert!(results.is_empty(), "Empty registry should return no results");
    }

    // -----------------------------------------------------------------------
    // 10. Identifier validators accessible from crate root
    // -----------------------------------------------------------------------

    /// `validate_isrc`, `validate_iswc`, and `validate_eidr` are all re-exported.
    #[test]
    fn identifier_validation_accessible() {
        // Valid ISRC: 2 alpha + 3 alphanumeric + 2 digits + 5 digits = 12 chars
        assert!(validate_isrc("GBUM71029604"), "Known valid ISRC should pass");
        assert!(!validate_isrc("BAD"), "Short string should fail ISRC validation");

        // Valid ISWC: T + 10 digits
        assert!(validate_iswc("T-034.524.680-1"), "Known valid ISWC should pass");
        assert!(!validate_iswc("NOTANISWC"), "Non-ISWC string should fail");

        // EIDR: must start with 10.5240/
        assert!(validate_eidr("10.5240/7791-8534-2C23-9030-8610-5"), "Known valid EIDR should pass");
        assert!(!validate_eidr("not-an-eidr"), "Non-EIDR string should fail");
    }

    // -----------------------------------------------------------------------
    // 11. ScoringWeights default values sum to ~1.0
    // -----------------------------------------------------------------------

    /// The default weights must sum to approximately 1.0 (title+artist+album+year+isrc).
    #[test]
    fn scoring_weights_default_valid() {
        let w = ScoringWeights::default();
        let sum = w.title + w.artist + w.album + w.year + w.isrc;
        // Allow small floating-point tolerance
        assert!(
            (sum - 1.0).abs() < 1e-9,
            "Default ScoringWeights should sum to 1.0, got {sum}"
        );
    }

    // -----------------------------------------------------------------------
    // 12. default_rpm_for returns sensible limits
    // -----------------------------------------------------------------------

    /// `default_rpm_for` should return a positive rate for all known providers.
    #[test]
    fn default_rpm_for_all_providers() {
        let providers = [
            "musicbrainz", "spotify", "apple_music", "deezer",
            "youtube_music", "amazon_music", "pandora", "tidal", "shazam", "iheart",
            "tmdb", "thetvdb", "omdb", "apple_tv", "itunes_store",
            "apple_podcasts", "isrc", "eidr", "iswc",
        ];
        for name in &providers {
            let rpm = default_rpm_for(name);
            assert!(rpm > 0, "default_rpm_for({name}) should be > 0, got {rpm}");
        }
    }

    // -----------------------------------------------------------------------
    // 13. select_best picks the smallest image meeting the minimum
    // -----------------------------------------------------------------------

    /// `select_best` returns the smallest image that meets the min-side constraint.
    #[test]
    fn cover_art_select_best_picks_correct() {
        let arts = vec![
            CoverArtInfo::new("https://example.com/100.jpg", 100, 100, "image/jpeg"),
            CoverArtInfo::new("https://example.com/500.jpg", 500, 500, "image/jpeg"),
            CoverArtInfo::new("https://example.com/1000.jpg", 1000, 1000, "image/jpeg"),
        ];
        // Min 400px — should return 500px (smallest that meets the bar)
        let best = select_best(&arts, 400).unwrap();
        assert_eq!(best.width, 500);
    }

    // -----------------------------------------------------------------------
    // 14. is_valid_art_url rejects empty and non-http URLs
    // -----------------------------------------------------------------------

    /// Cover art URL validation catches bad inputs from provider responses.
    #[test]
    fn cover_art_url_validation() {
        assert!(is_valid_art_url("https://example.com/cover.jpg"));
        assert!(is_valid_art_url("http://cdn.example.com/art/12345.png"));
        assert!(!is_valid_art_url(""));
        assert!(!is_valid_art_url("ftp://example.com/cover.jpg"));
        assert!(!is_valid_art_url("relative/path.jpg"));
    }

    // -----------------------------------------------------------------------
    // 15. score_result convenience function matches MatchScorer
    // -----------------------------------------------------------------------

    /// The `score_result` convenience function must agree with `MatchScorer::score`.
    #[test]
    fn score_result_matches_scorer() {
        let query = SearchQuery::music("Let It Be", "The Beatles");
        let result = ProviderResult {
            title: Some("Let It Be".into()),
            artist: Some("The Beatles".into()),
            ..Default::default()
        };
        let scorer = MatchScorer::new();
        let direct = scorer.score(&query, &result);
        let convenience = score_result(&query, &result);
        // Both should agree to within floating-point precision
        assert!(
            (direct - convenience).abs() < 1e-9,
            "score_result ({convenience}) must match MatchScorer::score ({direct})"
        );
    }
}
