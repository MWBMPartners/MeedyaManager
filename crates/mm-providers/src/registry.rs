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
//   2. Call `search()` with a `SearchQuery` to fan out across registered providers.
//   3. Results are collected, scored, and returned sorted by score.
//
// MIGRATION NOTE (#132): the upstream `MetadataProvider` trait does NOT have
// an `is_enabled()` method. Disabled providers are now expected to return
// `ProviderError::NotConfigured` from `search()`; the registry treats those as
// a non-fatal failure (logged and skipped). The previous `enabled_count()` API
// is preserved as an alias of `total_count()` for backward compatibility but is
// no longer meaningful — fix this when the registry is overhauled in #133.

use std::sync::Arc;

use tracing::{debug, warn};

use crate::match_scoring::{MatchScorer, rank_results_with};
use crate::traits::{MediaType, MetadataProvider, ProviderResult, SearchQuery};

// ---------------------------------------------------------------------------
// ProviderRegistry
// ---------------------------------------------------------------------------

/// Central registry that manages and dispatches to all metadata providers.
///
/// Providers are stored as `Arc<dyn MetadataProvider>` so they can be shared
/// across async tasks without cloning the full provider implementation.
pub struct ProviderRegistry {
    /// All registered providers
    providers: Vec<Arc<dyn MetadataProvider>>,

    /// Scorer used to rank results after aggregation
    scorer: MatchScorer,
}

impl ProviderRegistry {
    /// Create an empty registry with no providers registered.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            scorer: MatchScorer::default(),
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

    /// Returns all registered providers.
    pub fn all_providers(&self) -> &[Arc<dyn MetadataProvider>] {
        &self.providers
    }

    /// Returns the number of registered providers.
    pub fn total_count(&self) -> usize {
        self.providers.len()
    }

    /// Returns the number of registered providers. Alias of `total_count()` after
    /// the upstream trait migration (#132) removed per-provider enabled tracking.
    pub fn enabled_count(&self) -> usize {
        self.providers.len()
    }

    /// Returns all registered providers that support the given `media_type`.
    pub fn providers_for(&self, media_type: MediaType) -> Vec<Arc<dyn MetadataProvider>> {
        self.providers
            .iter()
            .filter(|p| supports_media_type(p.as_ref(), media_type))
            .cloned()
            .collect()
    }

    /// Find a provider by its exact id (case-insensitive).
    ///
    /// Returns the first registered provider whose `id()` matches, or `None`.
    pub fn find_by_name(&self, name: &str) -> Option<Arc<dyn MetadataProvider>> {
        let lower = name.to_lowercase();
        self.providers
            .iter()
            .find(|p| p.id().to_lowercase() == lower)
            .cloned()
    }

    /// Search all compatible providers concurrently and return ranked results.
    ///
    /// - Queries are fanned out in parallel.
    /// - Results from all providers are merged into a single list.
    /// - Each result is scored against the query using `MatchScorer`.
    /// - The merged list is sorted by score (highest first).
    /// - Results from failed providers (incl. `NotConfigured`) are logged and skipped.
    ///
    /// Returns an empty `Vec` if no providers are available or all fail.
    pub async fn search(&self, query: &SearchQuery) -> Vec<ProviderResult> {
        // Determine which providers to query based on the query's media type hint
        let providers: Vec<Arc<dyn MetadataProvider>> = if let Some(mt) = query.media_type {
            self.providers_for(mt)
        } else {
            // No type hint: query all registered providers
            self.providers.clone()
        };

        if providers.is_empty() {
            debug!("No providers available for this query");
            return Vec::new();
        }

        debug!(
            provider_count = providers.len(),
            title = ?query.title,
            "Dispatching search"
        );

        // Fan out to all providers, collecting results
        let mut merged: Vec<ProviderResult> = Vec::new();
        for provider in &providers {
            let p = Arc::clone(provider);
            let name = p.id().to_owned();
            match p.search(query).await {
                Ok(results) => {
                    debug!(
                        provider = &name,
                        result_count = results.len(),
                        "Provider returned results"
                    );
                    merged.extend(results);
                }
                Err(e) => {
                    warn!(provider = &name, error = %e, "Provider search failed");
                }
            }
        }

        // Score and sort merged results
        rank_results_with(&self.scorer, query, &mut merged);

        // Respect the max_results limit from the query
        if let Some(limit) = query.max_results {
            merged.truncate(limit);
        }

        merged
    }

    /// Search a specific named provider only (bypasses media-type filtering).
    ///
    /// Returns `Err(String)` if the provider is not found or its search fails.
    pub async fn search_provider(
        &self,
        name: &str,
        query: &SearchQuery,
    ) -> Result<Vec<ProviderResult>, String> {
        let provider = self
            .find_by_name(name)
            .ok_or_else(|| format!("Provider '{name}' not registered"))?;

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
        let names: Vec<&str> = self.providers.iter().map(|p| p.id()).collect();
        f.debug_struct("ProviderRegistry")
            .field("providers", &names)
            .field("count", &self.total_count())
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the provider's capabilities cover the requested media type.
///
/// Replaces the old `Capabilities::supports_media_type()` method, which doesn't
/// exist on the upstream `ProviderCapabilities` struct.
fn supports_media_type(p: &dyn MetadataProvider, media_type: MediaType) -> bool {
    let caps = p.capabilities();
    match media_type {
        MediaType::Music => caps.music_search,
        MediaType::Video => caps.video_search,
        MediaType::Podcast => caps.podcast_search,
        MediaType::Identifier => caps.identifier_lookup,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{
        MediaType, MetadataProvider, ProviderCapabilities, ProviderError, ProviderResult,
        SearchQuery, music_query,
    };
    use async_trait::async_trait;

    // --- Mock providers for testing ---

    /// A minimal mock provider that always returns a single result.
    struct MockProvider {
        name: String,
        media_type: MediaType,
        /// When false, the search returns `NotConfigured`, mirroring the new
        /// "disabled = not configured" convention.
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
    }

    #[async_trait]
    impl MetadataProvider for MockProvider {
        fn id(&self) -> &str {
            &self.name
        }

        fn display_name(&self) -> &str {
            &self.name
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                music_search: matches!(self.media_type, MediaType::Music),
                video_search: matches!(self.media_type, MediaType::Video),
                podcast_search: matches!(self.media_type, MediaType::Podcast),
                cover_art: false,
                lyrics: false,
                fingerprint_lookup: false,
                identifier_lookup: matches!(self.media_type, MediaType::Identifier),
            }
        }

        async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
            if !self.enabled {
                return Err(ProviderError::NotConfigured(self.name.clone()));
            }
            let mut result = ProviderResult::new(&self.name);
            result.title = Some(self.result_title.clone());
            result.artist = query.artist.clone();
            result.score = 1.0;
            Ok(vec![result])
        }
    }

    /// A provider that always fails with a network error.
    struct FailingProvider {
        name: String,
    }

    #[async_trait]
    impl MetadataProvider for FailingProvider {
        fn id(&self) -> &str {
            &self.name
        }
        fn display_name(&self) -> &str {
            &self.name
        }
        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities {
                music_search: true,
                ..Default::default()
            }
        }
        async fn search(&self, _query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
            Err(ProviderError::NetworkError("connection refused".into()))
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

    // --- providers_for ---

    #[test]
    fn providers_for_music_excludes_video() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Track A"));
        r.register(MockProvider::video("tmdb", "Movie B"));

        let music = r.providers_for(MediaType::Music);
        assert_eq!(music.len(), 1);
        assert_eq!(music[0].id(), "mb");
    }

    #[test]
    fn providers_for_video_excludes_music() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Track A"));
        r.register(MockProvider::video("tmdb", "Movie B"));

        let video = r.providers_for(MediaType::Video);
        assert_eq!(video.len(), 1);
        assert_eq!(video[0].id(), "tmdb");
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

        let query = music_query("Comfortably Numb", "Pink Floyd");
        let results = r.search(&query).await;
        assert!(!results.is_empty());
        assert_eq!(results[0].title.as_deref(), Some("Comfortably Numb"));
    }

    #[tokio::test]
    async fn search_skips_unconfigured_providers() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::disabled("disabled"));
        let query = music_query("Track", "Artist");
        let results = r.search(&query).await;
        // Disabled provider returns NotConfigured → swallowed; no results returned
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn search_aggregates_results_from_multiple_providers() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Track from MB"));
        r.register(MockProvider::music("spotify", "Track from Spotify"));

        let query = music_query("Track", "Artist");
        let results = r.search(&query).await;
        // Both providers return one result each
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn search_skips_failing_providers() {
        let mut r = ProviderRegistry::new();
        r.register(FailingProvider {
            name: "failer".into(),
        });
        r.register(MockProvider::music("good", "Good Track"));

        let query = music_query("Track", "Artist");
        let results = r.search(&query).await;
        // Only the good provider's result is returned
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider_name, "good");
    }

    #[tokio::test]
    async fn search_filters_by_media_type_from_query() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "Music Result"));
        r.register(MockProvider::video("tmdb", "Video Result"));

        // Music query should only hit music provider
        let query = music_query("Track", "Artist");
        let results = r.search(&query).await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider_name, "mb");
    }

    #[tokio::test]
    async fn search_respects_max_results() {
        let mut r = ProviderRegistry::new();
        // Register 3 music providers, each returns 1 result
        r.register(MockProvider::music("p1", "Result 1"));
        r.register(MockProvider::music("p2", "Result 2"));
        r.register(MockProvider::music("p3", "Result 3"));

        let mut query = music_query("Track", "Artist");
        query.max_results = Some(2);
        let results = r.search(&query).await;
        assert!(results.len() <= 2);
    }

    #[tokio::test]
    async fn search_empty_registry_returns_empty() {
        let r = ProviderRegistry::new();
        let query = music_query("Track", "Artist");
        let results = r.search(&query).await;
        assert!(results.is_empty());
    }

    // --- search_provider ---

    #[tokio::test]
    async fn search_provider_by_name_success() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::music("mb", "MB Track"));

        let query = music_query("Track", "Artist");
        let results = r.search_provider("mb", &query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider_name, "mb");
    }

    #[tokio::test]
    async fn search_provider_not_registered_returns_err() {
        let r = ProviderRegistry::new();
        let query = music_query("Track", "Artist");
        let result = r.search_provider("nobody", &query).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not registered"));
    }

    #[tokio::test]
    async fn search_provider_unconfigured_returns_err() {
        let mut r = ProviderRegistry::new();
        r.register(MockProvider::disabled("disabled_p"));

        let query = music_query("Track", "Artist");
        let result = r.search_provider("disabled_p", &query).await;
        assert!(result.is_err());
        // Error message includes the provider name plus its NotConfigured message
        assert!(result.unwrap_err().to_lowercase().contains("disabled_p"));
    }
}
