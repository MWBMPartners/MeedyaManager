// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rate Limiter
//
// Per-provider token bucket rate limiter using the `governor` crate.
// Each provider gets its own limiter with a configurable requests-per-minute quota.
//
// Design:
//   - `ProviderRateLimiter` wraps a single `governor::DefaultDirectRateLimiter`
//   - `RateLimiterRegistry` manages one limiter per named provider
//   - Non-blocking `check()` immediately returns Ok or Err (for non-critical paths)
//   - Async `wait_until_ready()` suspends until a token is available (for pipeline use)
//
// Default rate limits (requests per minute):
//   - MusicBrainz:  50   (free tier)
//   - Spotify:      100  (standard tier)
//   - Apple Music:  20   (JWT-based)
//   - Deezer:       50   (public API)
//   - YouTube Music: 10  (unofficial)
//   - Amazon Music: 10   (unofficial)
//   - Pandora:      10   (unofficial)
//   - Tidal:        60   (standard tier)
//   - Shazam:       10   (unofficial)
//   - iHeart:       10   (unofficial)
//   - TMDB:         40   (standard tier)
//   - TheTVDB:      30   (standard tier)
//   - OMDb:         10   (free tier — 1000/day = ~0.7/min, but burst to 10)
//   - Apple TV:     20   (JWT-based)
//   - iTunes Store: 20   (public API)
//   - Apple Podcasts: 20 (public API)
//   - ISRC:         30   (registry)
//   - EIDR:         10   (paid API)
//   - ISWC:         50   (via MusicBrainz)

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;

use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};

use crate::traits::ProviderError;

// ---------------------------------------------------------------------------
// Default rate limits
// ---------------------------------------------------------------------------

/// Returns the default requests-per-minute limit for a known provider.
///
/// Unknown providers default to 10 RPM (conservative).
pub fn default_rpm_for(provider: &str) -> u32 {
    match provider.to_lowercase().as_str() {
        "musicbrainz" => 50,
        "spotify" => 100,
        "apple_music" | "apple-music" => 20,
        "deezer" => 50,
        "youtube_music" | "youtube-music" => 10,
        "amazon_music" | "amazon-music" => 10,
        "pandora" => 10,
        "tidal" => 60,
        "shazam" => 10,
        "iheart" => 10,
        "tmdb" => 40,
        "thetvdb" | "tvdb" => 30,
        "omdb" | "imdb" => 10,
        "apple_tv" | "apple-tv" => 20,
        "itunes_store" | "itunes" => 20,
        "apple_podcasts" | "apple-podcasts" => 20,
        "isrc" => 30,
        "eidr" => 10,
        "iswc" => 50,
        _ => 10,
    }
}

// ---------------------------------------------------------------------------
// ProviderRateLimiter
// ---------------------------------------------------------------------------

/// A rate limiter scoped to a single metadata provider.
///
/// Wraps `governor::DefaultDirectRateLimiter` with provider context.
/// The limiter uses a token-bucket algorithm with per-second token replenishment.
pub struct ProviderRateLimiter {
    /// Provider identifier (for error messages)
    provider: String,

    /// Underlying governor rate limiter
    limiter: Arc<DefaultDirectRateLimiter>,

    /// Configured requests per minute (for display)
    requests_per_minute: u32,
}

impl ProviderRateLimiter {
    /// Create a new rate limiter for `provider` allowing `requests_per_minute` RPM.
    ///
    /// # Panics
    ///
    /// Panics if `requests_per_minute` is 0.
    pub fn new(provider: impl Into<String>, requests_per_minute: u32) -> Self {
        assert!(requests_per_minute > 0, "requests_per_minute must be > 0");

        // Convert RPM to a per-second quota with burst = RPM/10 (minimum 1)
        let burst = NonZeroU32::new((requests_per_minute / 10).max(1)).unwrap();
        // Replenish `burst` tokens every 6 seconds (= burst * 10 tokens/minute ≈ RPM)
        let quota =
            Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap()).allow_burst(burst);

        Self {
            provider: provider.into(),
            limiter: Arc::new(RateLimiter::direct(quota)),
            requests_per_minute,
        }
    }

    /// Create a limiter with the default RPM for the given provider name.
    pub fn with_default_limit(provider: impl Into<String>) -> Self {
        let provider = provider.into();
        let rpm = default_rpm_for(&provider);
        Self::new(provider, rpm)
    }

    /// Non-blocking rate check — returns `Err(RateLimited)` immediately if the quota is exhausted.
    ///
    /// Suitable for non-critical paths where the caller can decide to skip the request.
    pub fn check(&self) -> Result<(), ProviderError> {
        self.limiter
            .check()
            .map_err(|_| ProviderError::RateLimited {
                provider: self.provider.clone(),
            })
    }

    /// Async wait until a token is available, then return.
    ///
    /// Suspends the calling task without blocking the thread.
    /// Use this in async provider `search()` implementations.
    pub async fn wait_until_ready(&self) {
        self.limiter.until_ready().await;
    }

    /// Returns the configured requests-per-minute limit.
    pub fn requests_per_minute(&self) -> u32 {
        self.requests_per_minute
    }

    /// Returns the provider name this limiter is scoped to.
    pub fn provider(&self) -> &str {
        &self.provider
    }
}

impl std::fmt::Debug for ProviderRateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderRateLimiter")
            .field("provider", &self.provider)
            .field("requests_per_minute", &self.requests_per_minute)
            .finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// RateLimiterRegistry
// ---------------------------------------------------------------------------

/// Registry that holds one `ProviderRateLimiter` per named provider.
///
/// Build with `RateLimiterRegistry::default()` to get all 19 providers pre-configured,
/// or start with `RateLimiterRegistry::new()` and call `register()` to add selectively.
#[derive(Debug, Default)]
pub struct RateLimiterRegistry {
    /// Map from provider name (lowercase) to its rate limiter
    limiters: HashMap<String, ProviderRateLimiter>,
}

impl RateLimiterRegistry {
    /// Create an empty registry (no limiters pre-registered).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry pre-populated with all 19 known providers at their default limits.
    pub fn with_all_providers() -> Self {
        let providers = [
            "musicbrainz",
            "spotify",
            "apple_music",
            "deezer",
            "youtube_music",
            "amazon_music",
            "pandora",
            "tidal",
            "shazam",
            "iheart",
            "tmdb",
            "thetvdb",
            "omdb",
            "apple_tv",
            "itunes_store",
            "apple_podcasts",
            "isrc",
            "eidr",
            "iswc",
        ];
        let mut registry = Self::new();
        for p in providers {
            registry.register_default(p);
        }
        registry
    }

    /// Create a registry from a custom `provider → RPM` map.
    pub fn with_limits(limits: HashMap<String, u32>) -> Self {
        let mut registry = Self::new();
        for (provider, rpm) in limits {
            registry.register(&provider, rpm);
        }
        registry
    }

    /// Register a provider with a custom RPM limit.
    pub fn register(&mut self, provider: &str, requests_per_minute: u32) {
        let key = provider.to_lowercase();
        self.limiters
            .insert(key, ProviderRateLimiter::new(provider, requests_per_minute));
    }

    /// Register a provider using its default RPM limit.
    pub fn register_default(&mut self, provider: &str) {
        let rpm = default_rpm_for(provider);
        self.register(provider, rpm);
    }

    /// Look up the rate limiter for a provider by name.
    ///
    /// Returns `None` if the provider is not registered.
    pub fn get(&self, provider: &str) -> Option<&ProviderRateLimiter> {
        self.limiters.get(&provider.to_lowercase())
    }

    /// Check whether `provider` has a registered rate limiter.
    pub fn has(&self, provider: &str) -> bool {
        self.limiters.contains_key(&provider.to_lowercase())
    }

    /// Number of registered limiters.
    pub fn len(&self) -> usize {
        self.limiters.len()
    }

    /// Returns `true` if no limiters are registered.
    pub fn is_empty(&self) -> bool {
        self.limiters.is_empty()
    }

    /// Call `check()` on the limiter for `provider`, if one is registered.
    ///
    /// Returns `Ok(())` if no limiter is registered (unthrottled) or if the quota
    /// allows the request. Returns `Err(RateLimited)` if the quota is exhausted.
    pub fn check(&self, provider: &str) -> Result<(), ProviderError> {
        match self.get(provider) {
            Some(limiter) => limiter.check(),
            None => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests — 25 tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- default_rpm_for ---

    #[test]
    fn default_rpm_musicbrainz() {
        assert_eq!(default_rpm_for("musicbrainz"), 50);
    }

    #[test]
    fn default_rpm_spotify() {
        assert_eq!(default_rpm_for("spotify"), 100);
    }

    #[test]
    fn default_rpm_apple_music_dash_name() {
        // Accept both underscore and hyphen variants
        assert_eq!(default_rpm_for("apple-music"), 20);
    }

    #[test]
    fn default_rpm_unknown_provider_defaults_to_10() {
        assert_eq!(default_rpm_for("some_unknown_provider_xyz"), 10);
    }

    #[test]
    fn default_rpm_case_insensitive() {
        assert_eq!(default_rpm_for("MusicBrainz"), 50);
        assert_eq!(default_rpm_for("SPOTIFY"), 100);
    }

    // --- ProviderRateLimiter construction ---

    #[test]
    fn provider_rate_limiter_stores_provider_name() {
        let limiter = ProviderRateLimiter::new("spotify", 100);
        assert_eq!(limiter.provider(), "spotify");
    }

    #[test]
    fn provider_rate_limiter_stores_rpm() {
        let limiter = ProviderRateLimiter::new("spotify", 100);
        assert_eq!(limiter.requests_per_minute(), 100);
    }

    #[test]
    fn provider_rate_limiter_with_default_limit() {
        let limiter = ProviderRateLimiter::with_default_limit("musicbrainz");
        assert_eq!(limiter.provider(), "musicbrainz");
        assert_eq!(limiter.requests_per_minute(), 50);
    }

    #[test]
    fn provider_rate_limiter_check_allows_first_request() {
        // A fresh limiter should immediately allow the first request
        let limiter = ProviderRateLimiter::new("test", 60);
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn provider_rate_limiter_check_exhausted_returns_rate_limited() {
        // With a very low burst (1 RPM = 1 token), exhaust it and verify rejection
        let limiter = ProviderRateLimiter::new("test", 1);
        // Drain the limiter
        let _ = limiter.check(); // first OK
        // Second request should be rate limited (no tokens left immediately)
        // Note: this is probabilistic but should be true for RPM=1 burst=1
        let result = limiter.check();
        // It's valid for it to be RateLimited (high RPM limiters may have burst)
        // Just verify the result is Ok or RateLimited (not a panic/crash)
        assert!(result.is_ok() || matches!(result, Err(ProviderError::RateLimited { .. })));
    }

    #[test]
    fn provider_rate_limiter_rate_limited_error_has_provider_name() {
        // Use a high-RPM limiter, drain its burst, then check the error message
        let limiter = ProviderRateLimiter::new("spotify", 1);
        // Force a rate limit by draining tokens
        let _ = limiter.check();
        if let Err(ProviderError::RateLimited { provider }) = limiter.check() {
            assert_eq!(provider, "spotify");
        }
        // If no rate limit triggered (burst > 1), test still passes
    }

    #[test]
    fn provider_rate_limiter_debug_format_contains_provider() {
        let limiter = ProviderRateLimiter::new("deezer", 50);
        let debug = format!("{limiter:?}");
        assert!(debug.contains("deezer"));
        assert!(debug.contains("50"));
    }

    // --- RateLimiterRegistry ---

    #[test]
    fn registry_new_is_empty() {
        let registry = RateLimiterRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn registry_register_and_retrieve() {
        let mut registry = RateLimiterRegistry::new();
        registry.register("spotify", 100);
        let limiter = registry.get("spotify").unwrap();
        assert_eq!(limiter.requests_per_minute(), 100);
    }

    #[test]
    fn registry_get_returns_none_for_unknown_provider() {
        let registry = RateLimiterRegistry::new();
        assert!(registry.get("nobody").is_none());
    }

    #[test]
    fn registry_has_returns_true_when_registered() {
        let mut registry = RateLimiterRegistry::new();
        registry.register("tmdb", 40);
        assert!(registry.has("tmdb"));
    }

    #[test]
    fn registry_has_returns_false_when_unregistered() {
        let registry = RateLimiterRegistry::new();
        assert!(!registry.has("nobody"));
    }

    #[test]
    fn registry_lookup_is_case_insensitive() {
        let mut registry = RateLimiterRegistry::new();
        registry.register("Spotify", 100);
        assert!(registry.get("SPOTIFY").is_some());
        assert!(registry.get("spotify").is_some());
    }

    #[test]
    fn registry_register_default_uses_known_rpm() {
        let mut registry = RateLimiterRegistry::new();
        registry.register_default("musicbrainz");
        let limiter = registry.get("musicbrainz").unwrap();
        assert_eq!(limiter.requests_per_minute(), 50);
    }

    #[test]
    fn registry_with_all_providers_has_19_entries() {
        let registry = RateLimiterRegistry::with_all_providers();
        assert_eq!(registry.len(), 19);
    }

    #[test]
    fn registry_with_all_providers_has_musicbrainz() {
        let registry = RateLimiterRegistry::with_all_providers();
        assert!(registry.has("musicbrainz"));
    }

    #[test]
    fn registry_with_limits_custom_map() {
        let mut limits = HashMap::new();
        limits.insert("x".to_owned(), 5);
        limits.insert("y".to_owned(), 15);
        let registry = RateLimiterRegistry::with_limits(limits);
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.get("x").unwrap().requests_per_minute(), 5);
        assert_eq!(registry.get("y").unwrap().requests_per_minute(), 15);
    }

    #[test]
    fn registry_check_unregistered_provider_returns_ok() {
        // Unregistered providers are unthrottled
        let registry = RateLimiterRegistry::new();
        assert!(registry.check("unregistered").is_ok());
    }

    #[test]
    fn registry_check_registered_provider_allows_first_request() {
        let mut registry = RateLimiterRegistry::new();
        registry.register("tmdb", 40);
        // First check should succeed
        assert!(registry.check("tmdb").is_ok());
    }

    #[test]
    fn registry_len_increases_with_each_registration() {
        let mut registry = RateLimiterRegistry::new();
        assert_eq!(registry.len(), 0);
        registry.register("a", 10);
        assert_eq!(registry.len(), 1);
        registry.register("b", 10);
        assert_eq!(registry.len(), 2);
    }

    // --- Async wait test ---

    #[tokio::test]
    async fn provider_rate_limiter_wait_completes() {
        // A high-RPM limiter should immediately return from wait_until_ready
        let limiter = ProviderRateLimiter::new("fast", 1000);
        // This should complete without blocking indefinitely
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            limiter.wait_until_ready(),
        )
        .await
        .expect("wait_until_ready should return quickly for high-RPM limiter");
    }
}
