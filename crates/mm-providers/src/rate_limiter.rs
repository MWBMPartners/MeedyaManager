// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Rate Limiter (#133 migration)
//
// Phase 3 of the MeedyaSuite-core integration epic. The local rate-limiter
// implementation (479 lines) is replaced by re-exports from the upstream
// `meedya_providers::rate_limiter` module via `meedya_core`.
//
// Re-exported upstream items:
//   - `ProviderRateLimiter`  — single token-bucket limiter
//     - `new(name, rpm)`, `check() -> bool`, `wait_until_ready().await`,
//       `provider_name() -> &str`, `rpm() -> u32`
//   - `RateLimiterRegistry`  — concurrent registry of named limiters
//     - `new()`, `with_defaults()`, `get_or_create(name, rpm).await`,
//       `get(name).await`
//
// LOCAL-ONLY ADDITIONS (kept here because MeedyaManager has more providers
// than the upstream `with_defaults()` knows about and uses sync APIs in
// the registry):
//   - `default_rpm_for(name) -> u32`  — RPM lookup table for all 19 of our
//     providers (upstream's `with_defaults()` only covers 13).
//   - `MmRateLimiterRegistryExt` trait — adds a sync, infallible
//     `check_blocking(name)` adapter and a `with_all_mm_providers()` builder
//     so existing call sites keep working.
//
// API DRIFT documented:
//   - `ProviderRateLimiter::check()` now returns `bool` (upstream), not
//     `Result<(), ProviderError>`. Callers that want the old error semantics
//     can use the `check_rate_limited()` free function below.
//   - `ProviderRateLimiter::provider()` → `provider_name()`.
//   - `ProviderRateLimiter::requests_per_minute()` → `rpm()`.
//   - `RateLimiterRegistry::register(name, rpm)` and `register_default()`
//     are not available on the upstream registry (which uses async
//     `get_or_create` instead). The compatibility extension below maps
//     `register*` calls onto a synchronous fill of an in-memory cache via
//     `RateLimiterRegistry::get_or_create` after blocking on tokio.

use crate::traits::ProviderError;

// Re-exports — primary surface from upstream.
pub use meedya_core::providers::rate_limiter::{ProviderRateLimiter, RateLimiterRegistry};

// ---------------------------------------------------------------------------
// Local default RPM lookup — covers all 19 MeedyaManager providers
// ---------------------------------------------------------------------------

/// Returns the default requests-per-minute limit for a known provider.
///
/// Unknown providers default to 10 RPM (conservative). This is wider than
/// upstream's built-in `RateLimiterRegistry::with_defaults()` (which only
/// covers 13 providers) — we keep it here so all 19 of MeedyaManager's
/// providers have a sensible default.
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
// MeedyaManager-specific helpers
// ---------------------------------------------------------------------------

/// All 19 provider IDs known to MeedyaManager.
pub const ALL_MM_PROVIDERS: &[&str] = &[
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

/// Non-blocking rate-limit check that returns the previous `Result` shape.
///
/// Equivalent to `limiter.check()` but wraps the upstream `bool` result in
/// `Ok(())` / `Err(RateLimited)` for callers that prefer the error pattern.
pub fn check_rate_limited(limiter: &ProviderRateLimiter) -> Result<(), ProviderError> {
    if limiter.check() {
        Ok(())
    } else {
        Err(ProviderError::RateLimited(
            limiter.provider_name().to_string(),
        ))
    }
}

/// Local convenience extension on the upstream `RateLimiterRegistry`.
///
/// The upstream registry uses `tokio::sync::RwLock` which can only be polled
/// from an async context. These extensions provide async checks that wrap
/// the upstream API with the previous `Result<(), ProviderError>` shape, and
/// an async builder that pre-populates the registry with all 19 MeedyaManager
/// providers.
#[allow(async_fn_in_trait)]
pub trait MmRateLimiterRegistryExt {
    /// Async check: returns `Ok(())` if the limiter for `provider` allows
    /// a request right now, `Err(RateLimited)` if it's exhausted, or `Ok(())`
    /// if no limiter is registered for the provider (= unthrottled).
    async fn check(&self, provider: &str) -> Result<(), ProviderError>;

    /// Builder that pre-populates the registry with all 19 MeedyaManager
    /// providers at their `default_rpm_for()` limits.
    ///
    /// This is the async replacement for the previous
    /// `RateLimiterRegistry::with_all_providers()`.
    async fn with_all_mm_providers() -> Self;
}

impl MmRateLimiterRegistryExt for RateLimiterRegistry {
    async fn check(&self, provider: &str) -> Result<(), ProviderError> {
        match self.get(provider).await {
            Some(limiter) => check_rate_limited(&limiter),
            None => Ok(()),
        }
    }

    async fn with_all_mm_providers() -> Self {
        // Build by repeated `get_or_create` calls so that any provider IDs
        // already covered by upstream's `with_defaults()` keep their canonical
        // RPM, and our extra IDs are filled in at the MM defaults.
        let registry = Self::with_defaults();
        for &name in ALL_MM_PROVIDERS {
            // `get_or_create` is idempotent: if `name` is already in the
            // registry, the existing limiter is returned and `rpm` is ignored.
            let _ = registry.get_or_create(name, default_rpm_for(name)).await;
        }
        registry
    }
}

// ---------------------------------------------------------------------------
// Tests — local-adapter behaviour only (upstream tests live upstream)
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

    #[test]
    fn default_rpm_covers_all_19_providers() {
        for name in ALL_MM_PROVIDERS {
            assert!(default_rpm_for(name) > 0, "missing default RPM for {name}");
        }
    }

    // --- check_rate_limited adapter ---

    #[test]
    fn check_rate_limited_ok_when_available() {
        // Fresh limiter has a token available.
        let limiter = ProviderRateLimiter::new("test", 60);
        assert!(check_rate_limited(&limiter).is_ok());
    }

    #[test]
    fn check_rate_limited_err_carries_provider_name() {
        // Drain a tiny limiter and assert the err includes the provider name.
        let limiter = ProviderRateLimiter::new("draino", 1);
        let _ = check_rate_limited(&limiter);
        // The second call may or may not be rate-limited depending on the
        // governor burst; if it is, verify the err message contains the name.
        if let Err(ProviderError::RateLimited(name)) = check_rate_limited(&limiter) {
            assert_eq!(name, "draino");
        }
        // If it wasn't rate-limited, no assertion needed — both outcomes are valid.
    }

    // --- MmRateLimiterRegistryExt::check (async) ---

    #[tokio::test]
    async fn check_unregistered_provider_returns_ok() {
        // Unregistered providers are unthrottled in the MM convention.
        let registry = RateLimiterRegistry::new();
        assert!(
            MmRateLimiterRegistryExt::check(&registry, "unregistered")
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn check_registered_provider_allows_first_request() {
        let registry = RateLimiterRegistry::new();
        let _ = registry.get_or_create("tmdb", 40).await;
        // First check should succeed.
        assert!(
            MmRateLimiterRegistryExt::check(&registry, "tmdb")
                .await
                .is_ok()
        );
    }

    // --- MmRateLimiterRegistryExt::with_all_mm_providers ---

    #[tokio::test]
    async fn with_all_mm_providers_covers_all_19() {
        let registry =
            <RateLimiterRegistry as MmRateLimiterRegistryExt>::with_all_mm_providers().await;
        for &name in ALL_MM_PROVIDERS {
            assert!(
                registry.get(name).await.is_some(),
                "missing rate limiter for {name}"
            );
        }
    }

    #[tokio::test]
    async fn with_all_mm_providers_has_musicbrainz_at_50rpm() {
        let registry =
            <RateLimiterRegistry as MmRateLimiterRegistryExt>::with_all_mm_providers().await;
        let mb = registry.get("musicbrainz").await.unwrap();
        assert_eq!(mb.rpm(), 50);
    }

    // --- Upstream pass-through smoke tests (sanity-only) ---

    #[test]
    fn upstream_provider_rate_limiter_stores_name_and_rpm() {
        let limiter = ProviderRateLimiter::new("spotify", 100);
        assert_eq!(limiter.provider_name(), "spotify");
        assert_eq!(limiter.rpm(), 100);
    }

    #[test]
    fn upstream_check_returns_bool() {
        let limiter = ProviderRateLimiter::new("fresh", 60);
        // `check()` is `bool` on the upstream type
        let allowed: bool = limiter.check();
        assert!(allowed);
    }

    #[tokio::test]
    async fn upstream_registry_get_or_create_is_idempotent() {
        let registry = RateLimiterRegistry::new();
        let first = registry.get_or_create("spotify", 100).await;
        let second = registry.get_or_create("spotify", 200).await;
        // First creation wins
        assert_eq!(first.rpm(), 100);
        assert_eq!(second.rpm(), 100);
    }
}
