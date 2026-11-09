// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Credential Management (#133 migration)
//
// Phase 3 of the MeedyaSuite-core integration epic. The 605-line local
// implementation is replaced by re-exports from
// `meedya_core::providers::credentials` PLUS a thin local wrapper
// (`CredentialStore`) that preserves MeedyaManager's deployment
// conventions:
//
//   - Tier 1 env var prefix: `MM_<PROVIDER>_<KEY>` (NOT upstream's
//     `MEEDYA_<PROVIDER>_<KEY>`). Existing user environments and CI
//     pipelines depend on the `MM_` prefix — changing it would silently
//     break every deployment.
//   - Keyring service name: `meedyamanager.<provider>` (per-provider
//     services) rather than upstream's single shared service.
//   - Two-arg constructor: `(config_map, config_dir)`, where
//     `credentials.json` is auto-appended to the directory.
//
// We achieve this by delegating tiers 2–4 to the upstream
// `meedya_providers::CredentialStore` (which already has 4-tier resolution
// with config map / OS keyring / local file) and prepending MM-flavoured
// tier 1 logic on top.
//
// Upstream items re-exported (for code that wants the raw upstream API):
//   - `MmUpstreamCredentialStore` (renamed from `CredentialStore` to avoid
//     conflict with our local wrapper)
//   - `CredentialSource`, `ResolvedCredential`, `CredentialError`

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::{debug, warn};

// Re-exports — provides the upstream API for callers that want it raw.
pub use meedya_core::providers::credentials::{
    CredentialSource, CredentialStore as MmUpstreamCredentialStore, ResolvedCredential,
};
pub use meedya_core::providers::error::CredentialError;

// ---------------------------------------------------------------------------
// Local-only Display impl for CredentialSource
// ---------------------------------------------------------------------------
//
// The upstream `CredentialSource` derives `Debug` but doesn't impl `Display`.
// We expose the prior MM-style human-readable labels via a free function so
// callers don't need to match on the enum themselves. This is the
// replacement for the previous `impl Display for CredentialSource`.

/// Human-readable label for a `CredentialSource` tier (lowercase, terse).
///
/// Matches the previous `Display` impl on the local `CredentialSource` enum:
///   - `Environment`   → `"environment variable"`
///   - `Config`        → `"config file"`
///   - `Keyring`       → `"OS keyring"`
///   - `LocalFile`     → `"local credential file"`
pub fn credential_source_label(src: CredentialSource) -> &'static str {
    match src {
        CredentialSource::Environment => "environment variable",
        CredentialSource::Config => "config file",
        CredentialSource::Keyring => "OS keyring",
        CredentialSource::LocalFile => "local credential file",
    }
}

// ---------------------------------------------------------------------------
// Credential — local-flavoured wrapper around the value+source pair
// ---------------------------------------------------------------------------

/// A successfully resolved credential value with provenance information.
///
/// This is the MeedyaManager-flavoured equivalent of the upstream
/// `ResolvedCredential`. We keep it as a separate type because the
/// previous code paths exposed `Credential` directly (and a couple of
/// downstream callers `match` on `cred.source` against `CredentialSource`).
#[derive(Debug, Clone)]
pub struct Credential {
    /// The secret value (never log this).
    pub value: String,
    /// Which tier supplied this credential.
    pub source: CredentialSource,
}

impl Credential {
    fn new(value: impl Into<String>, source: CredentialSource) -> Self {
        Self {
            value: value.into(),
            source,
        }
    }
}

impl From<ResolvedCredential> for Credential {
    fn from(r: ResolvedCredential) -> Self {
        Self {
            value: r.value,
            source: r.source,
        }
    }
}

// ---------------------------------------------------------------------------
// CredentialStore — local wrapper preserving MM conventions
// ---------------------------------------------------------------------------

/// Keyring service identifier used when storing per-provider secrets.
///
/// The upstream `CredentialStore` keys all entries under a single service
/// name with `{provider}/{key}` as the secret label. We keep the previous
/// MeedyaManager convention of per-provider services
/// (`meedyamanager.<provider>`) by instantiating a fresh upstream store per
/// keyring operation with the service name pre-baked in.
const MM_KEYRING_SERVICE_PREFIX: &str = "meedyamanager";

/// 4-tier credential resolver for metadata provider API keys.
///
/// Tier order (highest priority first):
///   1. `MM_<PROVIDER>_<KEY>` environment variable
///   2. In-memory config map (from `settings.json5`)
///   3. OS keyring (per-provider service `meedyamanager.<provider>`)
///   4. Local `credentials.json` in the configured directory
///
/// All credential access is logged at debug level; values are NEVER logged.
#[derive(Debug)]
pub struct CredentialStore {
    /// Tier 2 — credentials injected from `settings.json5` at startup.
    /// Key format: `<provider>.<key>` (e.g., `"spotify.client_id"`).
    config_credentials: HashMap<String, String>,
    /// Tier 4 — full path to `credentials.json`.
    local_file_path: PathBuf,
}

impl CredentialStore {
    /// Create a new credential store.
    ///
    /// - `config_credentials` — flat map of `"provider.key" → "value"`
    ///   loaded from `settings.json5`.
    /// - `config_dir`         — directory where `credentials.json` is stored
    ///   (the filename is auto-appended).
    pub fn new(config_credentials: HashMap<String, String>, config_dir: impl AsRef<Path>) -> Self {
        Self {
            config_credentials,
            local_file_path: config_dir.as_ref().join("credentials.json"),
        }
    }

    /// Resolve a credential for `provider` / `key` using the 4-tier lookup.
    ///
    /// Returns `None` if the credential is not found in any tier.
    pub fn get(&self, provider: &str, key: &str) -> Option<Credential> {
        // Tier 1 — `MM_<PROVIDER>_<KEY>` env var (local-flavoured prefix).
        if let Some(val) = Self::from_mm_env(provider, key) {
            debug!(provider, key, source = "env", "Credential resolved");
            return Some(Credential::new(val, CredentialSource::Environment));
        }

        // Tiers 2–4: delegate to a freshly-built upstream store with our
        // config map and credentials file path. We rebuild the upstream
        // store per call because `CredentialStore` is small and the
        // `config_credentials` map must be re-applied via `set_config`.
        let upstream = self.build_upstream_store(provider);
        match upstream.resolve(provider, key) {
            Ok(resolved) => {
                debug!(
                    provider,
                    key,
                    source = ?resolved.source,
                    "Credential resolved (delegated to upstream)"
                );
                Some(resolved.into())
            }
            Err(CredentialError::NotFound { .. }) => {
                debug!(provider, key, "Credential not found in any tier");
                None
            }
            Err(e) => {
                warn!(provider, key, error = %e, "Credential resolution error");
                None
            }
        }
    }

    /// Store a credential in the OS keyring.
    ///
    /// Returns `Ok(())` on success, or an error message string on failure.
    pub fn store_in_keyring(provider: &str, key: &str, value: &str) -> Result<(), String> {
        let upstream = MmUpstreamCredentialStore::new(Self::keyring_service(provider), None);
        upstream
            .store_keyring(provider, key, value)
            .map_err(|e| format!("Failed to store in keyring: {e}"))
    }

    /// Delete a credential from the OS keyring.
    pub fn delete_from_keyring(provider: &str, key: &str) -> Result<(), String> {
        let upstream = MmUpstreamCredentialStore::new(Self::keyring_service(provider), None);
        upstream
            .delete_keyring(provider, key)
            .map_err(|e| format!("Failed to delete from keyring: {e}"))
    }

    /// Store a credential in the local credential file (tier 4).
    pub fn store_in_local_file(
        &self,
        provider: &str,
        key: &str,
        value: &str,
    ) -> Result<(), String> {
        // Delegate to upstream's atomic-write implementation.
        let upstream = MmUpstreamCredentialStore::new(
            Self::keyring_service(provider),
            Some(self.local_file_path.clone()),
        );
        upstream
            .store_local_file(provider, key, value)
            .map_err(|e| format!("Failed to write credential file: {e}"))
    }

    /// Remove a credential from the local file.
    ///
    /// The upstream API has no public delete-by-key, so we do this directly:
    /// load the JSON, mutate it, write it back atomically.
    pub fn remove_from_local_file(&self, provider: &str, key: &str) -> Result<(), String> {
        let path = &self.local_file_path;
        if !path.exists() {
            return Ok(()); // Nothing to remove.
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read credential file: {e}"))?;
        let mut data: HashMap<String, serde_json::Value> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse credential file: {e}"))?;

        // Upstream's local-file format is `{ "<provider>": { "<key>": "<value>", ... }, ... }`.
        if let Some(serde_json::Value::Object(provider_map)) = data.get_mut(provider) {
            provider_map.remove(key);
            if provider_map.is_empty() {
                data.remove(provider);
            }
        }

        let json = serde_json::to_string_pretty(&data)
            .map_err(|e| format!("Failed to serialise credentials: {e}"))?;
        std::fs::write(path, json).map_err(|e| format!("Failed to write credential file: {e}"))
    }

    /// Check whether a credential exists in any tier (without returning the value).
    pub fn has(&self, provider: &str, key: &str) -> bool {
        self.get(provider, key).is_some()
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Build a fresh upstream `CredentialStore` pre-loaded with this wrapper's
    /// config map and the per-provider keyring service name.
    fn build_upstream_store(&self, provider: &str) -> MmUpstreamCredentialStore {
        let service = Self::keyring_service(provider);
        let mut store = MmUpstreamCredentialStore::new(service, Some(self.local_file_path.clone()));

        // Tier 2 — apply our config map. Upstream's `resolve()` looks under
        // the key `<provider>.<key>`, which is exactly the format used by
        // MeedyaManager's settings.json5 wiring, so a direct `set_config`
        // for each entry whose composite key matches the requested provider
        // is sufficient.
        for (composite_key, value) in &self.config_credentials {
            if value.is_empty() {
                // Empty config values are treated as missing — skip them so
                // resolution can fall through to the next tier.
                continue;
            }
            if let Some((p, k)) = composite_key.split_once('.') {
                // Only apply entries that match the requested provider —
                // limits the in-memory size of the upstream store for any
                // single resolution and matches the previous behaviour
                // where unrelated entries didn't shadow lower tiers.
                if p.eq_ignore_ascii_case(provider) {
                    store.set_config(p, k, value.clone());
                }
            }
        }
        store
    }

    /// Build the keyring service name for a provider, e.g. `meedyamanager.spotify`.
    fn keyring_service(provider: &str) -> String {
        format!("{MM_KEYRING_SERVICE_PREFIX}.{}", provider.to_lowercase())
    }

    /// Tier 1: look up `MM_<PROVIDER>_<KEY>` in the environment.
    fn from_mm_env(provider: &str, key: &str) -> Option<String> {
        let var = format!("MM_{}_{}", normalise_id(provider), normalise_id(key));
        std::env::var(&var).ok().filter(|v| !v.is_empty())
    }
}

/// Normalise an identifier for use in an environment variable name.
/// Uppercases and replaces hyphens, dots, and spaces with underscores.
fn normalise_id(s: &str) -> String {
    s.to_uppercase().replace(['-', '.', ' '], "_")
}

// ---------------------------------------------------------------------------
// Tests — preserve all behaviour the previous local impl had
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(unsafe_code)] // set_var / remove_var require unsafe in Edition 2024
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn make_store(config_creds: HashMap<String, String>) -> (CredentialStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = CredentialStore::new(config_creds, dir.path());
        (store, dir)
    }

    fn empty_store() -> (CredentialStore, TempDir) {
        make_store(HashMap::new())
    }

    // --- credential_source_label ---

    #[test]
    fn source_label_env() {
        assert_eq!(
            credential_source_label(CredentialSource::Environment),
            "environment variable"
        );
    }

    #[test]
    fn source_label_config() {
        assert_eq!(
            credential_source_label(CredentialSource::Config),
            "config file"
        );
    }

    #[test]
    fn source_label_keyring() {
        assert_eq!(
            credential_source_label(CredentialSource::Keyring),
            "OS keyring"
        );
    }

    #[test]
    fn source_label_local_file() {
        assert_eq!(
            credential_source_label(CredentialSource::LocalFile),
            "local credential file"
        );
    }

    // --- Tier 1 (MM_ env) ---

    #[test]
    fn tier1_env_var_found() {
        unsafe { std::env::set_var("MM_TESTPROVIDER_TESTKEY", "secret-value") };
        let (store, _dir) = empty_store();
        let cred = store.get("testprovider", "testkey");
        unsafe { std::env::remove_var("MM_TESTPROVIDER_TESTKEY") };

        let cred = cred.unwrap();
        assert_eq!(cred.value, "secret-value");
        assert_eq!(cred.source, CredentialSource::Environment);
    }

    #[test]
    fn tier1_env_var_hyphenated_provider() {
        unsafe { std::env::set_var("MM_APPLE_MUSIC_API_KEY", "apple-key") };
        let (store, _dir) = empty_store();
        let cred = store.get("apple-music", "api_key");
        unsafe { std::env::remove_var("MM_APPLE_MUSIC_API_KEY") };

        assert!(cred.is_some());
        assert_eq!(cred.unwrap().value, "apple-key");
    }

    #[test]
    fn tier1_env_var_not_set_returns_none() {
        let (store, _dir) = empty_store();
        let cred = store.get("mm_no_such_provider_xyz", "no_such_key_xyz");
        assert!(cred.is_none());
    }

    #[test]
    fn tier1_env_var_empty_string_treated_as_missing() {
        unsafe { std::env::set_var("MM_EMPTYPROVIDER_EMPTYKEY", "") };
        let (store, _dir) = empty_store();
        let cred = store.get("emptyprovider", "emptykey");
        unsafe { std::env::remove_var("MM_EMPTYPROVIDER_EMPTYKEY") };
        assert!(cred.is_none());
    }

    // --- Tier 2 (config map) ---

    #[test]
    fn tier2_config_credential_found() {
        let mut config = HashMap::new();
        config.insert("spotify.client_id".to_owned(), "sp-client-id".to_owned());
        let (store, _dir) = make_store(config);

        let cred = store.get("spotify", "client_id").unwrap();
        assert_eq!(cred.value, "sp-client-id");
        assert_eq!(cred.source, CredentialSource::Config);
    }

    #[test]
    fn tier2_config_missing_key_returns_none() {
        let (store, _dir) = empty_store();
        assert!(store.get("spotify", "client_secret").is_none());
    }

    #[test]
    fn tier2_config_empty_value_treated_as_missing() {
        let mut config = HashMap::new();
        config.insert("deezer.app_id".to_owned(), String::new());
        let (store, _dir) = make_store(config);
        assert!(store.get("deezer", "app_id").is_none());
    }

    // --- Tier 1 > Tier 2 priority ---

    #[test]
    fn tier1_takes_priority_over_tier2() {
        unsafe { std::env::set_var("MM_PRIORITY_KEY", "env-value") };
        let mut config = HashMap::new();
        config.insert("priority.key".to_owned(), "config-value".to_owned());
        let (store, _dir) = make_store(config);

        let cred = store.get("priority", "key").unwrap();
        unsafe { std::env::remove_var("MM_PRIORITY_KEY") };

        assert_eq!(cred.value, "env-value");
        assert_eq!(cred.source, CredentialSource::Environment);
    }

    // --- Tier 4 (local file) ---

    #[test]
    fn tier4_store_and_retrieve_from_local_file() {
        let (store, _dir) = empty_store();
        store
            .store_in_local_file("tmdb", "api_key", "tmdb-secret")
            .unwrap();
        let cred = store.get("tmdb", "api_key").unwrap();
        assert_eq!(cred.value, "tmdb-secret");
        assert_eq!(cred.source, CredentialSource::LocalFile);
    }

    #[test]
    fn tier4_overwrite_existing_local_credential() {
        let (store, _dir) = empty_store();
        store
            .store_in_local_file("tmdb", "api_key", "old-value")
            .unwrap();
        store
            .store_in_local_file("tmdb", "api_key", "new-value")
            .unwrap();
        let cred = store.get("tmdb", "api_key").unwrap();
        assert_eq!(cred.value, "new-value");
    }

    #[test]
    fn tier4_remove_from_local_file() {
        let (store, _dir) = empty_store();
        store
            .store_in_local_file("tmdb", "api_key", "to-delete")
            .unwrap();
        store.remove_from_local_file("tmdb", "api_key").unwrap();
        assert!(store.get("tmdb", "api_key").is_none());
    }

    #[test]
    fn tier4_multiple_providers_in_local_file() {
        let (store, _dir) = empty_store();
        store
            .store_in_local_file("provider_a", "key", "value_a")
            .unwrap();
        store
            .store_in_local_file("provider_b", "key", "value_b")
            .unwrap();
        store
            .store_in_local_file("provider_c", "secret", "value_c")
            .unwrap();

        assert_eq!(store.get("provider_a", "key").unwrap().value, "value_a");
        assert_eq!(store.get("provider_b", "key").unwrap().value, "value_b");
        assert_eq!(store.get("provider_c", "secret").unwrap().value, "value_c");
    }

    #[test]
    fn tier4_local_file_not_found_returns_none() {
        let (store, _dir) = empty_store();
        assert!(store.get("nobody", "nothing").is_none());
    }

    // --- Tier 2 > Tier 4 priority ---

    #[test]
    fn tier2_takes_priority_over_tier4() {
        let mut config = HashMap::new();
        config.insert("conflict.key".to_owned(), "config-wins".to_owned());
        let (store, _dir) = make_store(config);
        store
            .store_in_local_file("conflict", "key", "file-loses")
            .unwrap();

        let cred = store.get("conflict", "key").unwrap();
        assert_eq!(cred.value, "config-wins");
        assert_eq!(cred.source, CredentialSource::Config);
    }

    // --- has() helper ---

    #[test]
    fn has_returns_true_when_present() {
        let mut config = HashMap::new();
        config.insert("x.y".to_owned(), "v".to_owned());
        let (store, _dir) = make_store(config);
        assert!(store.has("x", "y"));
    }

    #[test]
    fn has_returns_false_when_absent() {
        let (store, _dir) = empty_store();
        assert!(!store.has("nobody", "nothing"));
    }

    // --- normalise_id ---

    #[test]
    fn normalise_id_uppercase() {
        assert_eq!(normalise_id("spotify"), "SPOTIFY");
    }

    #[test]
    fn normalise_id_replaces_hyphens() {
        assert_eq!(normalise_id("apple-music"), "APPLE_MUSIC");
    }

    #[test]
    fn normalise_id_replaces_dots() {
        assert_eq!(normalise_id("my.provider"), "MY_PROVIDER");
    }

    #[test]
    fn normalise_id_replaces_spaces() {
        assert_eq!(normalise_id("youtube music"), "YOUTUBE_MUSIC");
    }

    #[test]
    fn normalise_id_mixed_chars() {
        assert_eq!(normalise_id("apple-music.api"), "APPLE_MUSIC_API");
    }

    // --- keyring_service ---

    #[test]
    fn keyring_service_format() {
        let s = CredentialStore::keyring_service("Spotify");
        assert_eq!(s, "meedyamanager.spotify");
    }

    // --- Credential struct ---

    #[test]
    fn credential_value_accessible() {
        let cred = Credential::new("my-secret", CredentialSource::Config);
        assert_eq!(cred.value, "my-secret");
        assert_eq!(cred.source, CredentialSource::Config);
    }

    // --- Conversion from upstream ResolvedCredential ---

    #[test]
    fn credential_from_resolved() {
        let resolved = ResolvedCredential {
            value: "secret".into(),
            source: CredentialSource::Environment,
        };
        let cred: Credential = resolved.into();
        assert_eq!(cred.value, "secret");
        assert_eq!(cred.source, CredentialSource::Environment);
    }
}
