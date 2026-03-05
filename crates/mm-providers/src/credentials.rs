// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
//
// MeedyaManager — Credential Management
//
// Implements 4-tier credential resolution for metadata provider API keys:
//
//   Tier 1 — Environment variable: `MM_<PROVIDER>_<KEY>` (highest priority)
//   Tier 2 — Configuration file: values injected from `settings.json5`
//   Tier 3 — OS keyring: macOS Keychain, Windows Credential Manager, Linux Secret Service
//   Tier 4 — Local credential file: `credentials.json` in the app config directory
//
// Resolution order: Env → Config → Keyring → Local File → None
//
// All credential access is logged (at debug level) but values are never logged.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// Credential source tag
// ---------------------------------------------------------------------------

/// Indicates which tier supplied a credential (for diagnostics / UI display).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    /// Resolved from an environment variable (`MM_<PROVIDER>_<KEY>`)
    Environment,
    /// Resolved from the configuration file (`settings.json5`)
    Config,
    /// Resolved from the OS keyring (macOS Keychain / Windows Credential Manager / Secret Service)
    Keyring,
    /// Resolved from the local credential JSON file in the config directory
    LocalFile,
}

impl std::fmt::Display for CredentialSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CredentialSource::Environment => write!(f, "environment variable"),
            CredentialSource::Config     => write!(f, "config file"),
            CredentialSource::Keyring    => write!(f, "OS keyring"),
            CredentialSource::LocalFile  => write!(f, "local credential file"),
        }
    }
}

// ---------------------------------------------------------------------------
// Resolved credential
// ---------------------------------------------------------------------------

/// A successfully resolved credential value with provenance information.
#[derive(Debug, Clone)]
pub struct Credential {
    /// The secret value (never log this)
    pub value: String,
    /// Which tier supplied this credential
    pub source: CredentialSource,
}

impl Credential {
    /// Create a new credential with the given value and source.
    fn new(value: impl Into<String>, source: CredentialSource) -> Self {
        Self { value: value.into(), source }
    }
}

// ---------------------------------------------------------------------------
// Credential store
// ---------------------------------------------------------------------------

/// 4-tier credential resolver for metadata provider API keys.
///
/// # Example
///
/// ```ignore
/// let store = CredentialStore::new(config_values, config_dir);
/// if let Some(cred) = store.get("spotify", "client_id") {
///     // Use cred.value
/// }
/// ```
#[derive(Debug)]
pub struct CredentialStore {
    /// Tier 2: credentials injected from `settings.json5` at startup.
    /// Key format: `<provider>.<key>` (e.g., `"spotify.client_id"`)
    config_credentials: HashMap<String, String>,

    /// Tier 4: path to the local `credentials.json` file
    local_file_path: PathBuf,
}

impl CredentialStore {
    /// Create a new credential store.
    ///
    /// - `config_credentials` — flat map of `"provider.key" → "value"` from `settings.json5`
    /// - `config_dir`         — directory where `credentials.json` is stored
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
        // Tier 1 — environment variable
        if let Some(val) = Self::from_env(provider, key) {
            debug!(provider = provider, key = key, source = "env", "Credential resolved");
            return Some(Credential::new(val, CredentialSource::Environment));
        }

        // Tier 2 — config file
        if let Some(val) = self.from_config(provider, key) {
            debug!(provider = provider, key = key, source = "config", "Credential resolved");
            return Some(Credential::new(val, CredentialSource::Config));
        }

        // Tier 3 — OS keyring
        if let Some(val) = Self::from_keyring(provider, key) {
            debug!(provider = provider, key = key, source = "keyring", "Credential resolved");
            return Some(Credential::new(val, CredentialSource::Keyring));
        }

        // Tier 4 — local credential file
        if let Some(val) = self.from_local_file(provider, key) {
            debug!(provider = provider, key = key, source = "local_file", "Credential resolved");
            return Some(Credential::new(val, CredentialSource::LocalFile));
        }

        debug!(provider = provider, key = key, "Credential not found in any tier");
        None
    }

    /// Store a credential in the OS keyring.
    ///
    /// Returns `Ok(())` on success, or an error message string on failure.
    pub fn store_in_keyring(provider: &str, key: &str, value: &str) -> Result<(), String> {
        let service = Self::keyring_service(provider);
        let entry = keyring::Entry::new(&service, key)
            .map_err(|e| format!("Failed to create keyring entry: {e}"))?;
        entry.set_password(value)
            .map_err(|e| format!("Failed to store in keyring: {e}"))
    }

    /// Delete a credential from the OS keyring.
    pub fn delete_from_keyring(provider: &str, key: &str) -> Result<(), String> {
        let service = Self::keyring_service(provider);
        let entry = keyring::Entry::new(&service, key)
            .map_err(|e| format!("Failed to create keyring entry: {e}"))?;
        entry.delete_credential()
            .map_err(|e| format!("Failed to delete from keyring: {e}"))
    }

    /// Store a credential in the local credential file (tier 4).
    ///
    /// Creates the file if it does not exist. Thread-safe via file-level write.
    pub fn store_in_local_file(&self, provider: &str, key: &str, value: &str) -> Result<(), String> {
        // Load existing credentials (or start fresh)
        let mut map = self.load_local_file().unwrap_or_default();
        // Insert / overwrite the credential
        let composite_key = format!("{provider}.{key}");
        map.insert(composite_key, value.to_owned());
        // Serialise and write atomically (write → rename)
        let json = serde_json::to_string_pretty(&map)
            .map_err(|e| format!("Failed to serialise credentials: {e}"))?;
        // Write to a temp file first, then rename for atomicity
        let tmp_path = self.local_file_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &json)
            .map_err(|e| format!("Failed to write credential file: {e}"))?;
        std::fs::rename(&tmp_path, &self.local_file_path)
            .map_err(|e| format!("Failed to rename credential file: {e}"))
    }

    /// Remove a credential from the local file.
    pub fn remove_from_local_file(&self, provider: &str, key: &str) -> Result<(), String> {
        let mut map = self.load_local_file().unwrap_or_default();
        map.remove(&format!("{provider}.{key}"));
        let json = serde_json::to_string_pretty(&map)
            .map_err(|e| format!("Failed to serialise credentials: {e}"))?;
        std::fs::write(&self.local_file_path, json)
            .map_err(|e| format!("Failed to write credential file: {e}"))
    }

    /// Check whether a credential exists in any tier (without returning the value).
    pub fn has(&self, provider: &str, key: &str) -> bool {
        self.get(provider, key).is_some()
    }

    // -----------------------------------------------------------------------
    // Internal tier implementations
    // -----------------------------------------------------------------------

    /// Tier 1: Look up `MM_<PROVIDER>_<KEY>` in the environment.
    fn from_env(provider: &str, key: &str) -> Option<String> {
        // Normalise: uppercase, replace hyphens/dots/spaces with underscores
        let var = format!(
            "MM_{}_{}",
            Self::normalise_id(provider),
            Self::normalise_id(key)
        );
        std::env::var(&var).ok().filter(|v| !v.is_empty())
    }

    /// Tier 2: Look up `<provider>.<key>` in the config-derived credentials map.
    fn from_config(&self, provider: &str, key: &str) -> Option<String> {
        let composite = format!("{}.{}", provider.to_lowercase(), key.to_lowercase());
        self.config_credentials
            .get(&composite)
            .cloned()
            .filter(|v| !v.is_empty())
    }

    /// Tier 3: Look up the credential in the OS keyring.
    fn from_keyring(provider: &str, key: &str) -> Option<String> {
        let service = Self::keyring_service(provider);
        match keyring::Entry::new(&service, key) {
            Ok(entry) => match entry.get_password() {
                Ok(password) if !password.is_empty() => Some(password),
                Ok(_) => None,
                Err(keyring::Error::NoEntry) => None,
                Err(e) => {
                    warn!(provider = provider, key = key, error = %e, "Keyring lookup failed");
                    None
                }
            },
            Err(e) => {
                warn!(provider = provider, key = key, error = %e, "Failed to create keyring entry");
                None
            }
        }
    }

    /// Tier 4: Look up the credential in the local `credentials.json` file.
    fn from_local_file(&self, provider: &str, key: &str) -> Option<String> {
        let map = self.load_local_file()?;
        let composite = format!("{}.{}", provider.to_lowercase(), key.to_lowercase());
        map.get(&composite).cloned().filter(|v| !v.is_empty())
    }

    /// Load and parse the local credentials JSON file.
    fn load_local_file(&self) -> Option<HashMap<String, String>> {
        if !self.local_file_path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&self.local_file_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Build the keyring service name for a provider.
    fn keyring_service(provider: &str) -> String {
        format!("meedyamanager.{}", provider.to_lowercase())
    }

    /// Normalise an identifier for use in an environment variable name.
    /// Uppercases and replaces hyphens, dots, and spaces with underscores.
    fn normalise_id(s: &str) -> String {
        s.to_uppercase()
            .replace('-', "_")
            .replace('.', "_")
            .replace(' ', "_")
    }
}

// ---------------------------------------------------------------------------
// Tests — 30 tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    // Helper: create a store backed by a temp directory
    fn make_store(config_creds: HashMap<String, String>) -> (CredentialStore, TempDir) {
        let dir = TempDir::new().unwrap();
        let store = CredentialStore::new(config_creds, dir.path());
        (store, dir)
    }

    // Helper: empty config credential store
    fn empty_store() -> (CredentialStore, TempDir) {
        make_store(HashMap::new())
    }

    // --- CredentialSource Display ---

    #[test]
    fn credential_source_display_env() {
        assert_eq!(CredentialSource::Environment.to_string(), "environment variable");
    }

    #[test]
    fn credential_source_display_config() {
        assert_eq!(CredentialSource::Config.to_string(), "config file");
    }

    #[test]
    fn credential_source_display_keyring() {
        assert_eq!(CredentialSource::Keyring.to_string(), "OS keyring");
    }

    #[test]
    fn credential_source_display_local_file() {
        assert_eq!(CredentialSource::LocalFile.to_string(), "local credential file");
    }

    // --- Tier 1: Environment variable ---

    #[test]
    fn tier1_env_var_found() {
        // Set the env var for this test
        std::env::set_var("MM_TESTPROVIDER_TESTKEY", "secret-value");
        let (store, _dir) = empty_store();
        let cred = store.get("testprovider", "testkey");
        std::env::remove_var("MM_TESTPROVIDER_TESTKEY");

        let cred = cred.unwrap();
        assert_eq!(cred.value, "secret-value");
        assert_eq!(cred.source, CredentialSource::Environment);
    }

    #[test]
    fn tier1_env_var_hyphenated_provider() {
        // Hyphens in provider name should be normalised to underscores
        std::env::set_var("MM_APPLE_MUSIC_API_KEY", "apple-key");
        let (store, _dir) = empty_store();
        let cred = store.get("apple-music", "api_key");
        std::env::remove_var("MM_APPLE_MUSIC_API_KEY");

        assert!(cred.is_some());
        assert_eq!(cred.unwrap().value, "apple-key");
    }

    #[test]
    fn tier1_env_var_not_set_returns_none() {
        // Use a unique key name to avoid collision with existing env vars
        let (store, _dir) = empty_store();
        // No env var set for this unique provider/key combo
        let cred = store.get("mm_no_such_provider_xyz", "no_such_key_xyz");
        assert!(cred.is_none());
    }

    #[test]
    fn tier1_env_var_empty_string_treated_as_missing() {
        std::env::set_var("MM_EMPTYPROVIDER_EMPTYKEY", "");
        let (store, _dir) = empty_store();
        let cred = store.get("emptyprovider", "emptykey");
        std::env::remove_var("MM_EMPTYPROVIDER_EMPTYKEY");
        // Empty string should not be returned
        assert!(cred.is_none());
    }

    // --- Tier 2: Config file ---

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
    fn tier2_config_key_is_case_insensitive() {
        // Config lookup normalises to lowercase
        let mut config = HashMap::new();
        config.insert("musicbrainz.user_agent".to_owned(), "my-app/1.0".to_owned());
        let (store, _dir) = make_store(config);

        let cred = store.get("MusicBrainz", "user_agent").unwrap();
        assert_eq!(cred.value, "my-app/1.0");
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
        // Empty config value → should fall through to next tier (None for now)
        assert!(store.get("deezer", "app_id").is_none());
    }

    // --- Tier 1 takes priority over Tier 2 ---

    #[test]
    fn tier1_takes_priority_over_tier2() {
        std::env::set_var("MM_PRIORITY_KEY", "env-value");
        let mut config = HashMap::new();
        config.insert("priority.key".to_owned(), "config-value".to_owned());
        let (store, _dir) = make_store(config);

        let cred = store.get("priority", "key").unwrap();
        std::env::remove_var("MM_PRIORITY_KEY");

        // Env should win
        assert_eq!(cred.value, "env-value");
        assert_eq!(cred.source, CredentialSource::Environment);
    }

    // --- Tier 4: Local credential file ---

    #[test]
    fn tier4_store_and_retrieve_from_local_file() {
        let (store, _dir) = empty_store();
        // Store a credential
        store.store_in_local_file("tmdb", "api_key", "tmdb-secret").unwrap();
        // Retrieve it
        let cred = store.get("tmdb", "api_key").unwrap();
        assert_eq!(cred.value, "tmdb-secret");
        assert_eq!(cred.source, CredentialSource::LocalFile);
    }

    #[test]
    fn tier4_overwrite_existing_local_credential() {
        let (store, _dir) = empty_store();
        store.store_in_local_file("tmdb", "api_key", "old-value").unwrap();
        store.store_in_local_file("tmdb", "api_key", "new-value").unwrap();
        let cred = store.get("tmdb", "api_key").unwrap();
        assert_eq!(cred.value, "new-value");
    }

    #[test]
    fn tier4_remove_from_local_file() {
        let (store, _dir) = empty_store();
        store.store_in_local_file("tmdb", "api_key", "to-delete").unwrap();
        store.remove_from_local_file("tmdb", "api_key").unwrap();
        assert!(store.get("tmdb", "api_key").is_none());
    }

    #[test]
    fn tier4_multiple_providers_in_local_file() {
        let (store, _dir) = empty_store();
        store.store_in_local_file("provider_a", "key", "value_a").unwrap();
        store.store_in_local_file("provider_b", "key", "value_b").unwrap();
        store.store_in_local_file("provider_c", "secret", "value_c").unwrap();

        assert_eq!(store.get("provider_a", "key").unwrap().value, "value_a");
        assert_eq!(store.get("provider_b", "key").unwrap().value, "value_b");
        assert_eq!(store.get("provider_c", "secret").unwrap().value, "value_c");
    }

    #[test]
    fn tier4_local_file_not_found_returns_none() {
        let (store, _dir) = empty_store();
        // No local file has been written
        assert!(store.get("nobody", "nothing").is_none());
    }

    // --- Config takes priority over Tier 4 ---

    #[test]
    fn tier2_takes_priority_over_tier4() {
        let mut config = HashMap::new();
        config.insert("conflict.key".to_owned(), "config-wins".to_owned());
        let (store, _dir) = make_store(config);
        store.store_in_local_file("conflict", "key", "file-loses").unwrap();

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
        assert_eq!(CredentialStore::normalise_id("spotify"), "SPOTIFY");
    }

    #[test]
    fn normalise_id_replaces_hyphens() {
        assert_eq!(CredentialStore::normalise_id("apple-music"), "APPLE_MUSIC");
    }

    #[test]
    fn normalise_id_replaces_dots() {
        assert_eq!(CredentialStore::normalise_id("my.provider"), "MY_PROVIDER");
    }

    #[test]
    fn normalise_id_replaces_spaces() {
        assert_eq!(CredentialStore::normalise_id("youtube music"), "YOUTUBE_MUSIC");
    }

    #[test]
    fn normalise_id_mixed_chars() {
        assert_eq!(CredentialStore::normalise_id("apple-music.api"), "APPLE_MUSIC_API");
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

    // --- CredentialStore construction ---

    #[test]
    fn store_new_creates_correct_path() {
        let dir = TempDir::new().unwrap();
        let store = CredentialStore::new(HashMap::new(), dir.path());
        assert_eq!(
            store.local_file_path,
            dir.path().join("credentials.json")
        );
    }
}
