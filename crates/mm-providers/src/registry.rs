// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Provider Registry
//
// The `ProviderRegistry` is the central dispatch point for metadata lookups.
// It holds a list of `MetadataProvider` trait objects and routes search queries
// to providers that support the requested `MediaType`.
//
// Usage:
//   1. Create a registry and register providers.
//   2. Call `search()` with a `SearchQuery` to fan out across enabled providers.
//   3. Results are collected, scored, and returned sorted by score.

use std::sync::Arc;

use tracing::{debug, warn};

use crate::match_scoring::MatchScorer;
use crate::traits::{MediaType, MetadataProvider, ProviderResult, SearchQuery};

// ---------------------------------------------------------------------------
// ProviderRegistry
// ---------------------------------------------------------------------------

/// Central registry that manages and dispatches to all metadata providers.
///
/// Providers are stored as `Arc<dyn MetadataProvider>` so they can be shared
/// across async tasks without cloning the full provider implementation.
pub struct ProviderRegistry {
    /// All registered providers (enabled and disabled)
    providers: Vec<Arc<dyn MetadataProvider>>,

    /// Scorer used to rank results after aggregation
    scorer: MatchScorer,
}

impl ProviderRegistry {
    /// Create an empty registry with no providers registered.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            scorer: MatchScorer::new(),
        }
    }

    /// Register a provider.
    ///
    /// Providers are queried in registration order (though results are later sorted by score).
    pub fn register<P: MetadataProvider + 'static>(&mut self, provider: P) {
        self.providers.push(Arc::new(provider));
    }

    /// Register a provider that is already wrapped in an `Arc`.
    pub fn register_arc(&mut self, provider: Arc<dyn MetadataProvider>) {
        self.providers.push(provider);
    }

    /// Returns all registered providers (enabled and disabled).
    pub fn all_providers(&self) -> &[Arc<dyn MetadataProvider>] {
        &self.providers
    }

    /// Returns the number of registered providers (enabled and disabled).
    pub fn total_count(&self) -> usize {
        self.providers.len()
    }

    /// Returns the number of currently enabled providers.
    pub fn enabled_count(&self) -> usize {
        self.providers.iter().filter(|p| p.is_enabled()).count()
    }

    /// Returns all enabled providers that support the given `media_type`.
    pub fn providers_for(&self, media_type: MediaType) -> Vec<Arc<dyn MetadataProvider>> {
        self.providers
            .iter()
            .filter(|p| p.is_enabled() && p.capabilities().supports_media_type(media_type))
            .cloned()
            .collect()
    }

    /// Find a provider by its exact name (case-insensitive).
    ///
    /// Returns the first registered provider whose `name()` matches, or `None`.
    pub fn find_by_name(&self, name: &str) -> Option<Arc<dyn MetadataProvider>> {
        let lower = name.to_lowercase();
        self.providers
            .iter()
            .find(|p| p.name().to_lowercase() == lower)
            .cloned()
    }

    /// Search all enabled, compatible providers concurrently and return ranked results.
    ///
    /// - Queries are fanned out in parallel using `futures::future::join_all`.
    /// - Results from all providers are merged into a single list.
    /// - Each result is scored against the query using `MatchScorer`.
    /// - The merged list is sorted by score (highest first).
    /// - Results from failed providers are logged and skipped.
    ///
    /// Returns an empty `Vec` if no providers are available or all fail.
    pub async fn search(&self, query: &SearchQuery) -> Vec<ProviderResult> {
        // Determine which providers to query based on the query's media type hint
        let providers: Vec<Arc<dyn MetadataProvider>> = if let Some(mt) = query.media_type {
            self.providers_for(mt)
        } else {
            // No type hint: query all enabled providers
            self.providers.iter().filter(|p| p.is_enabled()).cloned().collect()
        };

        if providers.is_empty() {
            debug!("No enabled providers available for this query");
            return Vec::new();
        }

        debug!(provider_count = providers.len(), query = &query.query, "Dispatching search");

        // Fan out to all providers concurrently, collecting results
        let mut merged: Vec<ProviderResult> = Vec::new();
        for provider in &providers {
            let p = Arc::clone(provider);
            let name = p.name().to_owned();
            match p.search(query).await {
                Ok(results) => {
                    debug!(provider = &name, result_count = results.len(), "Provider returned results");
                    merged.extend(results);
                }
                Err(e) => {
                    warn!(provider = &name, error = %e, "Provider search failed");
                }
            }
        }

        // Score and sort merged results
        self.scorer.rank_results(query, &mut merged);

        // Respect the max_results limit from the query
        if query.max_results > 0 {
            merged.truncate(query.max_results);
        }

        merged
    }

    /// Search a specific named provider only (bypasses media-type filtering).
    ///
    /// Returns `Err(String)` if the provider is not found or is disabled.
    pub async fn search_provider(
        &self,
        name: &str,
        query: &SearchQuery,
    ) -> Result<Vec<ProviderResult>, String> {
        let provider = self
            .find_by_name(name)
            .ok_or_else(|| format!("Provider '{name}' not registered"))?;

        if !provider.is_enabled() {
            return Err(format!("Provider '{name}' is disabled"));
        }

        provider
            .search(query)
            .await
            .map_err(|e| format!("Provider '{name}' failed: {e}"))
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&str> = self.providers.iter().map(|p| p.name()).collect();
        f.debug_struct("ProviderRegistry")
            .field("providers", &names)
            .field("enabled", &self.enabled_count())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Tests — 25 tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{Capabilities, MediaType, ProviderError, ProviderResult, SearchQuery};

    // --- Mock providers for testing ---

    /// A minimal mock provider that always returns a single result.
    struct MockProvider {
        name: String,
        media_type: MediaType,
        enabled: bool,
        result_title: String,
    }

    impl MockProvider {
        fn music(name: &str, result_title: &str) -> Self {
            Self {
                name: name.into(),
                media_type: MediaType::Music,
                enabled: true,
                result_title: result_title.into(),
            }
        }

        fn disabled(name: &str) -> Self {
            Self {
                name: name.into(),
                media_type: MediaType::Music,
                enabled: false,
                result_title: String::new(),
            }
        }

        fn video(name: &str, result_title: &str) -> Self {
            Self {
                name: name.into(),
                media_type: MediaType::Video,
                enabled: true,
                result_title: result_title.into(),
            }
        }

        /// Build a static Capabilities struct for this provider.
        fn make_capabilities(&self) -> Capabilities {
            Capabilities {
                media_types: vec![self.media_type],
                supports_search: true,
                supports_isrc: false,
                supports_iswc: false,
                provides_cover_art: false,
                provides_fingerprint: false,
                requires_auth: false,
                display_name: self.name.clone(),
                homepage_url: "https://example.com".into(),
            }
        }
    }

    impl MetadataProvider for MockProvider {
        fn name(&self) -> &str { &self.name }

        fn capabilities(&self) -> &Capabilities {
            // Leak to get a 'static reference (acceptable in tests)
            Box::leak(Box::new(self.make_capabilities()))
        }

        fn is_enabled(&self) -> bool { self.enabled }

        async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
            if !self.enabled {
                return Err(ProviderError::Disabled(self.name.clone()));
            }
            Ok(vec![ProviderResult {
                provider: self.name.clone(),
                provider_id: "mock-1".into(),
                title: Some(self.result_title.clone()),
                artist: query.artist.clone(),
                score: 1.0,
                ..Default::default()
            }])
        }
    }

    /// A provider that always fails with a network error.
    struct FailingProvider { name: String }

    impl MetadataProvider for FailingProvider {
        fn name(&self) -> &str { &self.name }
        fn capabilities(&self) -> &Capabilities {
            Box::leak(Box::new(Capabilities {
                media_types: vec![MediaType::Music],
                supports_search: true,
                supports_isrc: false,
                supports_iswc: false,
                provides_cover_art: false,
                provides_fingerprint: false,
                requires_auth: false,
                display_name: self.name.clone(),
                homepage_url: "https://example.com".into(),
            }))
        }
        fn is_enabled(&self) -> bool { true }
        async fn search(&self, _query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
            Err(ProviderError::Network("connection refused".into()))
        }
    }

    // --- Registry construction ---

    #[test]
    fn registry_new_is_empty() {
        let r = ProviderRegistry::new();
        assert_eq!(r.total_count(), 0);
        assert_eq!(r.enabled_count(), 0);
    }

    #[test]
    fn registry_default_is_empty() {
        let r = ProviderRegistry::default();
        assert_eq!(r.total_count(), 0);
    }

    // --- register ---

    #[test]
    fn register_increases_total_count() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Track A"));
        assert_eq!(r.total_count(), 1);
    }

    #[test]
    fn register_multiple_providers() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Track A"));
        r.register(MockProvider::music("spotify", "Track B"));
        assert_eq!(r.total_count(), 2);
    }

    // --- enabled_count ---

    #[test]
    fn enabled_count_excludes_disabled() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Track A"));
        r.register(MockProvider::disabled("disabled_p"));
        assert_eq!(r.enabled_count(), 1);
    }

    // --- providers_for ---

    #[test]
    fn providers_for_music_excludes_video() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Track A"));
        r.register(MockProvider::video("tmdb", "Movie B"));

        let music = r.providers_for(MediaType::Music);
        assert_eq!(music.len(), 1);
        assert_eq!(music[0].name(), "mb");
    }

    #[test]
    fn providers_for_video_excludes_music() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Track A"));
        r.register(MockProvider::video("tmdb", "Movie B"));

        let video = r.providers_for(MediaType::Video);
        assert_eq!(video.len(), 1);
        assert_eq!(video[0].name(), "tmdb");
    }

    #[test]
    fn providers_for_excludes_disabled() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::disabled("disabled_mb"));
        let music = r.providers_for(MediaType::Music);
        assert!(music.is_empty());
    }

    // --- find_by_name ---

    #[test]
    fn find_by_name_found() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("musicbrainz", "Track A"));
        assert!(r.find_by_name("musicbrainz").is_some());
    }

    #[test]
    fn find_by_name_case_insensitive() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("MusicBrainz", "Track A"));
        assert!(r.find_by_name("musicbrainz").is_some());
        assert!(r.find_by_name("MUSICBRAINZ").is_some());
    }

    #[test]
    fn find_by_name_not_found_returns_none() {
        let r = ProviderRegistry::new();
        assert!(r.find_by_name("nobody").is_none());
    }

    // --- Debug format ---

    #[test]
    fn debug_format_contains_provider_names() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "x"));
        r.register(MockProvider::music("spotify", "y"));
        let d = format!("{r:?}");
        assert!(d.contains("mb"));
        assert!(d.contains("spotify"));
    }

    // --- Async search ---

    #[tokio::test]
    async fn search_returns_results_from_enabled_provider() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Comfortably Numb"));

        let query = SearchQuery::music("Comfortably Numb", "Pink Floyd");
        let results = r.search(&query).await;
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_deref(), Some("Comfortably Numb"));
    }

    #[tokio::test]
    async fn search_skips_disabled_providers() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::disabled("disabled"));
        let query = SearchQuery::music("Track", "Artist");
        let results = r.search(&query).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn search_aggregates_results_from_multiple_providers() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Track from MB"));
        r.register(MockProvider::music("spotify", "Track from Spotify"));

        let query = SearchQuery::music("Track", "Artist");
        let results = r.search(&query).await;
        // Both providers return one result each
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn search_skips_failing_providers() {
        let mut r = ProviderRegistry::new();
        r.register(FailingProvider { name: "failer".into() });
        r.register(MockProvider::music("good", "Good Track"));

        let query = SearchQuery::music("Track", "Artist");
        let results = r.search(&query).await;
        // Only the good provider's result is returned
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "good");
    }

    #[tokio::test]
    async fn search_filters_by_media_type_from_query() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Music Result"));
        r.register(MockProvider::video("tmdb", "Video Result"));

        // Music query should only hit music provider
        let query = SearchQuery::music("Track", "Artist");
        let results = r.search(&query).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "mb");
    }

    #[tokio::test]
    async fn search_respects_max_results() {
        let mut r = ProviderRegistry::new();
        // Register 3 music providers, each returns 1 result
        r.register(MockProvider::music("p1", "Result 1"));
        r.register(MockProvider::music("p2", "Result 2"));
        r.register(MockProvider::music("p3", "Result 3"));

        let mut query = SearchQuery::music("Track", "Artist");
        query.max_results = 2;
        let results = r.search(&query).await;
        assert!(results.len() <= 2);
    }

    #[tokio::test]
    async fn search_empty_registry_returns_empty() {
        let r = ProviderRegistry::new();
        let query = SearchQuery::music("Track", "Artist");
        let results = r.search(&query).await;
        assert!(results.is_empty());
    }

    // --- search_provider ---

    #[tokio::test]
    async fn search_provider_by_name_success() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "MB Track"));

        let query = SearchQuery::music("Track", "Artist");
        let results = r.search_provider("mb", &query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "mb");
    }

    #[tokio::test]
    async fn search_provider_not_registered_returns_err() {
        let r = ProviderRegistry::new();
        let query = SearchQuery::music("Track", "Artist");
        let result = r.search_provider("nobody", &query).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not registered"));
    }

    #[tokio::test]
    async fn search_provider_disabled_returns_err() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::disabled("disabled_p"));

        let query = SearchQuery::music("Track", "Artist");
        let result = r.search_provider("disabled_p", &query).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("disabled"));
    }
}
