// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Identifier Lookup Providers
//
// Implements 3 identifier registry providers:
//
//   1. IsrcProvider   — ISRC lookup via MusicBrainz recordings API
//   2. EidrProvider   — EIDR lookup via the EIDR REST API (paid account required)
//   3. IswcProvider   — ISWC lookup via MusicBrainz works API
//
// All three providers target `MediaType::Identifier`. They augment music/video
// results with authoritative identifier-to-track/work/title mappings.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use crate::traits::{
    META_DURATION_SECS, META_EIDR, META_ISWC, META_PROVIDER_ID, MetadataProvider,
    ProviderCapabilities, ProviderError, ProviderResult, SearchQuery,
};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn net_err(e: reqwest::Error) -> ProviderError {
    ProviderError::NetworkError(e.to_string())
}

fn parse_err(context: &str, e: impl std::fmt::Display) -> ProviderError {
    ProviderError::Other(format!("parse error: {context}: {e}"))
}

/// Validate ISRC format: 2 country + 3 registrant + 2 year + 5 designation = 12 chars.
/// Accepts hyphens as separators (e.g. `GB-AYE-06-01498`).
pub fn validate_isrc(isrc: &str) -> bool {
    let normalised: String = isrc.chars().filter(|c| c.is_alphanumeric()).collect();
    normalised.len() == 12
        && normalised[..2].chars().all(|c| c.is_ascii_alphabetic())
        && normalised[2..5].chars().all(|c| c.is_ascii_alphanumeric())
        && normalised[5..7].chars().all(|c| c.is_ascii_digit())
        && normalised[7..12].chars().all(|c| c.is_ascii_digit())
}

/// Validate ISWC format: `T-123456789-C` (T + 9 digits + check digit).
/// Accepts the format with or without hyphens.
pub fn validate_iswc(iswc: &str) -> bool {
    let normalised: String = iswc
        .to_uppercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    // Must be exactly 11 chars: T + 9 digits + 1 check digit
    normalised.len() == 11
        && normalised.starts_with('T')
        && normalised[1..].chars().all(|c| c.is_ascii_digit())
}

/// Validate EIDR format: `10.5240/XXXX-XXXX-XXXX-XXXX-XXXX-C` (DOI-based).
pub fn validate_eidr(eidr: &str) -> bool {
    // Must start with the EIDR DOI prefix
    eidr.starts_with("10.5240/") && eidr.len() > 10
}

// ---------------------------------------------------------------------------
// 1. ISRC Provider (via MusicBrainz)
// ---------------------------------------------------------------------------

/// Looks up ISRC identifiers via MusicBrainz recording search.
///
/// Endpoint: `https://musicbrainz.org/ws/2/recording/?query=isrc:<ISRC>`
/// Auth:     None (but User-Agent required)
/// Limits:   30 RPM
pub struct IsrcProvider {
    client: Client,
    base_url: String,
    user_agent: String,
}

impl IsrcProvider {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self::with_base_url(user_agent, "https://musicbrainz.org")
    }

    pub fn with_base_url(user_agent: impl Into<String>, base_url: impl Into<String>) -> Self {
        let user_agent = user_agent.into();
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            user_agent,
        }
    }

    /// True if a User-Agent string is configured. Required by MusicBrainz API.
    fn configured(&self) -> bool {
        !self.user_agent.is_empty()
    }

    fn parse_recordings(
        provider_name: &str,
        body: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct MbResponse {
            recordings: Vec<MbRecording>,
        }
        #[derive(Deserialize)]
        struct MbRecording {
            id: Option<String>,
            title: Option<String>,
            #[serde(rename = "artist-credit")]
            artist_credit: Option<Vec<MbCredit>>,
            releases: Option<Vec<MbRelease>>,
            isrcs: Option<Vec<String>>,
            length: Option<u64>,
        }
        #[derive(Deserialize)]
        struct MbCredit {
            artist: Option<MbArtist>,
        }
        #[derive(Deserialize)]
        struct MbArtist {
            name: Option<String>,
        }
        #[derive(Deserialize)]
        struct MbRelease {
            title: Option<String>,
            date: Option<String>,
        }

        let resp: MbResponse =
            serde_json::from_str(body).map_err(|e| parse_err("ISRC/MusicBrainz response", e))?;

        let results = resp
            .recordings
            .into_iter()
            .map(|rec| {
                let artist = rec.artist_credit.as_deref().map(|credits| {
                    credits
                        .iter()
                        .filter_map(|c| c.artist.as_ref()?.name.as_deref())
                        .collect::<Vec<_>>()
                        .join("; ")
                });
                let first_release = rec.releases.as_deref().and_then(|r| r.first());
                let album = first_release.and_then(|r| r.title.clone());
                let year = first_release
                    .and_then(|r| r.date.as_deref())
                    .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());

                let mut result = ProviderResult::new(provider_name);
                result.title = rec.title;
                result.artist = artist;
                result.album = album;
                result.year = year;
                result.isrc = rec.isrcs.and_then(|v| v.into_iter().next());

                if let Some(id) = rec.id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id));
                }
                if let Some(length_ms) = rec.length {
                    let secs = length_ms as f64 / 1000.0;
                    if let Some(num) = serde_json::Number::from_f64(secs) {
                        result
                            .metadata
                            .insert(META_DURATION_SECS.into(), Value::Number(num));
                    }
                }

                result
            })
            .collect();
        Ok(results)
    }
}

#[async_trait]
impl MetadataProvider for IsrcProvider {
    fn id(&self) -> &str {
        "isrc"
    }

    fn display_name(&self) -> &str {
        "ISRC (via MusicBrainz)"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            music_search: true,
            video_search: false,
            podcast_search: false,
            cover_art: false,
            lyrics: false,
            fingerprint_lookup: false,
            identifier_lookup: true,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("isrc".into()));
        }

        let isrc = query.isrc.as_deref().ok_or_else(|| {
            ProviderError::NotSupported("isrc: ISRC query requires an ISRC code".into())
        })?;

        if !validate_isrc(isrc) {
            return Err(ProviderError::Other(format!(
                "parse error: Invalid ISRC format: {isrc}"
            )));
        }

        debug!(
            provider = "isrc",
            isrc = isrc,
            "Sending ISRC lookup request"
        );

        let limit = query.max_results.unwrap_or(10).to_string();
        let url = format!("{}/ws/2/recording/", self.base_url);
        let response = self
            .client
            .get(&url)
            // User-Agent is set at client level by crate::http::build_client()
            .header("Accept", "application/json")
            .query(&[
                ("query", &format!("isrc:{isrc}")),
                ("limit", &limit),
                ("fmt", &"json".to_owned()),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let s = response.status();
            if s.as_u16() == 503 {
                return Err(ProviderError::RateLimited("isrc".into()));
            }
            return Err(ProviderError::NetworkError(format!("HTTP {s}")));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse_recordings("isrc", &body)
    }
}

// ---------------------------------------------------------------------------
// 2. EIDR Provider
// ---------------------------------------------------------------------------

/// Looks up EIDR (Entertainment Identifier Registry) titles for video content.
///
/// Endpoint: `https://id.eidr.org/EIDR/object/<DOI>`
/// Auth:     Basic auth (EIDR registry account required)
/// Limits:   10 RPM (paid API)
pub struct EidrProvider {
    client: Client,
    base_url: String,
    username: Option<String>,
    password: Option<String>,
}

impl EidrProvider {
    pub fn new(username: Option<String>, password: Option<String>) -> Self {
        Self::with_base_url(username, password, "https://id.eidr.org")
    }

    pub fn with_base_url(
        username: Option<String>,
        password: Option<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            username,
            password,
        }
    }

    /// True if both username and password are present.
    fn configured(&self) -> bool {
        self.username.is_some() && self.password.is_some()
    }

    /// Parse an EIDR JSON response into a single `ProviderResult`.
    fn parse_eidr_json(
        provider_name: &str,
        body: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct EidrRecord {
            #[serde(rename = "ID")]
            id: Option<String>,
            #[serde(rename = "ResourceName")]
            resource_name: Option<EidrLocalizedName>,
            #[serde(rename = "ReleaseDate")]
            release_date: Option<String>,
            #[serde(rename = "ExtraObjectMetadata")]
            extra: Option<EidrExtra>,
        }

        #[derive(Deserialize)]
        struct EidrLocalizedName {
            value: Option<String>,
        }

        #[derive(Deserialize)]
        struct EidrExtra {
            movie: Option<EidrMovie>,
        }

        #[derive(Deserialize)]
        struct EidrMovie {
            directors: Option<Vec<String>>,
        }

        let record: EidrRecord =
            serde_json::from_str(body).map_err(|e| parse_err("EIDR response", e))?;

        let year = record
            .release_date
            .as_deref()
            .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());

        let director = record
            .extra
            .as_ref()
            .and_then(|e| e.movie.as_ref())
            .and_then(|m| m.directors.as_deref())
            .and_then(|d| d.first())
            .cloned();

        let mut result = ProviderResult::new(provider_name);
        result.title = record.resource_name.and_then(|n| n.value);
        result.artist = director; // Director for film
        result.year = year;

        if let Some(id) = record.id {
            result
                .metadata
                .insert(META_PROVIDER_ID.into(), Value::String(id.clone()));
            result.metadata.insert(META_EIDR.into(), Value::String(id));
        }

        Ok(vec![result])
    }
}

#[async_trait]
impl MetadataProvider for EidrProvider {
    fn id(&self) -> &str {
        "eidr"
    }

    fn display_name(&self) -> &str {
        "EIDR"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            music_search: false,
            video_search: true,
            podcast_search: false,
            cover_art: false,
            lyrics: false,
            fingerprint_lookup: false,
            identifier_lookup: true,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("eidr".into()));
        }

        let eidr = query.eidr.as_deref().ok_or_else(|| {
            ProviderError::NotSupported("eidr: EIDR query requires an EIDR DOI".into())
        })?;

        if !validate_eidr(eidr) {
            return Err(ProviderError::Other(format!(
                "parse error: Invalid EIDR format: {eidr}"
            )));
        }

        debug!(
            provider = "eidr",
            eidr = eidr,
            "Sending EIDR lookup request"
        );

        let url = format!("{}/EIDR/object/{}", self.base_url, eidr);
        let response = self
            .client
            .get(&url)
            .basic_auth(self.username.as_deref().unwrap(), self.password.as_deref())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let s = response.status();
            if s.as_u16() == 401 {
                return Err(ProviderError::AuthenticationFailed {
                    provider: "eidr".into(),
                    reason: "Invalid EIDR credentials".into(),
                });
            }
            return Err(ProviderError::NetworkError(format!("HTTP {s}")));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse_eidr_json("eidr", &body)
    }
}

// ---------------------------------------------------------------------------
// 3. ISWC Provider (via MusicBrainz Works)
// ---------------------------------------------------------------------------

/// Looks up ISWC identifiers via MusicBrainz works API.
///
/// Endpoint: `https://musicbrainz.org/ws/2/work/?query=iswc:<ISWC>`
/// Auth:     None (but User-Agent required)
/// Limits:   50 RPM
pub struct IswcProvider {
    client: Client,
    base_url: String,
    user_agent: String,
}

impl IswcProvider {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self::with_base_url(user_agent, "https://musicbrainz.org")
    }

    pub fn with_base_url(user_agent: impl Into<String>, base_url: impl Into<String>) -> Self {
        let user_agent = user_agent.into();
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            user_agent,
        }
    }

    fn configured(&self) -> bool {
        !self.user_agent.is_empty()
    }

    fn parse_works(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct MbWorksResponse {
            works: Vec<MbWork>,
        }
        #[derive(Deserialize)]
        struct MbWork {
            id: Option<String>,
            title: Option<String>,
            iswcs: Option<Vec<String>>,
            relations: Option<Vec<MbRelation>>,
        }
        #[derive(Deserialize)]
        struct MbRelation {
            #[serde(rename = "type")]
            rel_type: Option<String>,
            artist: Option<MbRelArtist>,
        }
        #[derive(Deserialize)]
        struct MbRelArtist {
            name: Option<String>,
        }

        let resp: MbWorksResponse =
            serde_json::from_str(body).map_err(|e| parse_err("ISWC/MusicBrainz response", e))?;

        let results = resp
            .works
            .into_iter()
            .map(|work| {
                // Find the composer from relations
                let composer = work.relations.as_deref().and_then(|rels| {
                    rels.iter()
                        .find(|r| r.rel_type.as_deref() == Some("composer"))
                        .and_then(|r| r.artist.as_ref()?.name.clone())
                });

                let mut result = ProviderResult::new(provider_name);
                result.title = work.title;
                result.artist = composer;

                if let Some(id) = work.id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id));
                }
                if let Some(iswc) = work.iswcs.and_then(|v| v.into_iter().next()) {
                    result
                        .metadata
                        .insert(META_ISWC.into(), Value::String(iswc));
                }
                result
            })
            .collect();
        Ok(results)
    }
}

#[async_trait]
impl MetadataProvider for IswcProvider {
    fn id(&self) -> &str {
        "iswc"
    }

    fn display_name(&self) -> &str {
        "ISWC (via MusicBrainz)"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            music_search: true,
            video_search: false,
            podcast_search: false,
            cover_art: false,
            lyrics: false,
            fingerprint_lookup: false,
            identifier_lookup: true,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("iswc".into()));
        }

        let iswc = query.iswc.as_deref().ok_or_else(|| {
            ProviderError::NotSupported("iswc: ISWC query requires an ISWC code".into())
        })?;

        if !validate_iswc(iswc) {
            return Err(ProviderError::Other(format!(
                "parse error: Invalid ISWC format: {iswc}"
            )));
        }

        debug!(
            provider = "iswc",
            iswc = iswc,
            "Sending ISWC lookup request"
        );

        let limit = query.max_results.unwrap_or(10).to_string();
        let url = format!("{}/ws/2/work/", self.base_url);
        let response = self
            .client
            .get(&url)
            // User-Agent is set at client level by crate::http::build_client()
            .header("Accept", "application/json")
            .query(&[
                ("query", &format!("iswc:{iswc}")),
                ("limit", &limit),
                ("fmt", &"json".to_owned()),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let s = response.status();
            if s.as_u16() == 503 {
                return Err(ProviderError::RateLimited("iswc".into()));
            }
            return Err(ProviderError::NetworkError(format!("HTTP {s}")));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse_works("iswc", &body)
    }
}

// ---------------------------------------------------------------------------
// Tests — 30 tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Validation helpers
    // =========================================================================

    #[test]
    fn validate_isrc_valid_standard() {
        assert!(validate_isrc("GBAYE0601498")); // 12 chars, no hyphens
    }

    #[test]
    fn validate_isrc_valid_with_hyphens() {
        assert!(validate_isrc("GB-AYE-06-01498"));
    }

    #[test]
    fn validate_isrc_too_short() {
        assert!(!validate_isrc("GBAYE060149")); // 11 chars
    }

    #[test]
    fn validate_isrc_too_long() {
        assert!(!validate_isrc("GBAYE06014980")); // 13 chars
    }

    #[test]
    fn validate_isrc_invalid_country_code() {
        // Country must be 2 letters; digits in first 2 positions → invalid
        assert!(!validate_isrc("12AYE0601498"));
    }

    #[test]
    fn validate_iswc_valid_standard() {
        assert!(validate_iswc("T0345246801")); // T + 10 digits
    }

    #[test]
    fn validate_iswc_valid_with_hyphens() {
        assert!(validate_iswc("T-034524680-1"));
    }

    #[test]
    fn validate_iswc_wrong_prefix() {
        assert!(!validate_iswc("X0345246801")); // Must start with T
    }

    #[test]
    fn validate_iswc_too_short() {
        assert!(!validate_iswc("T034524680")); // 10 chars (T + 9 digits) — need 11
    }

    #[test]
    fn validate_eidr_valid() {
        assert!(validate_eidr("10.5240/AEBE-0317-CE0D-4943-5916-E"));
    }

    #[test]
    fn validate_eidr_wrong_prefix() {
        assert!(!validate_eidr("10.1000/AEBE-0317-CE0D-4943-5916-E"));
    }

    #[test]
    fn validate_eidr_too_short() {
        assert!(!validate_eidr("10.5240/"));
    }

    // =========================================================================
    // ISRC Provider tests
    // =========================================================================

    #[test]
    fn isrc_provider_name() {
        assert_eq!(IsrcProvider::new("App/1.0").id(), "isrc");
    }

    #[test]
    fn isrc_provider_capabilities() {
        let caps = IsrcProvider::new("App/1.0").capabilities();
        assert!(caps.identifier_lookup);
        assert!(caps.music_search);
        assert!(!caps.cover_art);
    }

    #[test]
    fn isrc_provider_parse_recordings_valid() {
        let json = r#"{
            "recordings": [{
                "id": "mb-rec-1",
                "title": "Comfortably Numb",
                "artist-credit": [{"artist": {"name": "Pink Floyd"}}],
                "releases": [{"title": "The Wall", "date": "1979-11-30"}],
                "isrcs": ["GBAYE7900498"],
                "length": 382000
            }]
        }"#;
        let results = IsrcProvider::parse_recordings("isrc", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Comfortably Numb"));
        assert_eq!(results[0].isrc.as_deref(), Some("GBAYE7900498"));
        assert_eq!(results[0].artist.as_deref(), Some("Pink Floyd"));
    }

    #[test]
    fn isrc_provider_parse_invalid_json_returns_err() {
        assert!(matches!(
            IsrcProvider::parse_recordings("isrc", "bad"),
            Err(ProviderError::Other(_))
        ));
    }

    #[tokio::test]
    async fn isrc_provider_search_without_isrc_returns_not_supported() {
        let p = IsrcProvider::new("App/1.0");
        let q = SearchQuery {
            max_results: Some(5),
            ..Default::default()
        };
        assert!(matches!(
            p.search(&q).await,
            Err(ProviderError::NotSupported(_))
        ));
    }

    #[tokio::test]
    async fn isrc_provider_search_invalid_isrc_returns_parse_err() {
        let p = IsrcProvider::new("App/1.0");
        let q = SearchQuery {
            isrc: Some("BAD".into()),
            max_results: Some(5),
            ..Default::default()
        };
        assert!(matches!(p.search(&q).await, Err(ProviderError::Other(_))));
    }

    // =========================================================================
    // EIDR Provider tests
    // =========================================================================

    #[test]
    fn eidr_provider_name() {
        assert_eq!(EidrProvider::new(None, None).id(), "eidr");
    }

    #[test]
    fn eidr_provider_capabilities() {
        let caps = EidrProvider::new(None, None).capabilities();
        assert!(caps.identifier_lookup);
        assert!(caps.video_search);
    }

    #[test]
    fn eidr_provider_parse_json_valid() {
        let json = r#"{
            "ID": "10.5240/AEBE-0317-CE0D-4943-5916-E",
            "ResourceName": {"value": "Inception"},
            "ReleaseDate": "2010-07-16",
            "ExtraObjectMetadata": {
                "movie": {"directors": ["Christopher Nolan"]}
            }
        }"#;
        let results = EidrProvider::parse_eidr_json("eidr", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Inception"));
        assert_eq!(results[0].year, Some(2010));
        assert_eq!(results[0].artist.as_deref(), Some("Christopher Nolan"));
        // EIDR is now stored in metadata
        assert_eq!(
            results[0]
                .metadata
                .get(META_EIDR)
                .and_then(serde_json::Value::as_str),
            Some("10.5240/AEBE-0317-CE0D-4943-5916-E")
        );
    }

    // =========================================================================
    // ISWC Provider tests
    // =========================================================================

    #[test]
    fn iswc_provider_name() {
        assert_eq!(IswcProvider::new("App/1.0").id(), "iswc");
    }

    #[test]
    fn iswc_provider_capabilities() {
        let caps = IswcProvider::new("App/1.0").capabilities();
        assert!(caps.identifier_lookup);
        assert!(caps.music_search);
    }

    #[test]
    fn iswc_provider_parse_works_valid() {
        let json = r#"{
            "works": [{
                "id": "mb-work-1",
                "title": "Bohemian Rhapsody",
                "iswcs": ["T0345246801"],
                "relations": [{
                    "type": "composer",
                    "artist": {"name": "Freddie Mercury"}
                }]
            }]
        }"#;
        let results = IswcProvider::parse_works("iswc", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Bohemian Rhapsody"));
        assert_eq!(results[0].artist.as_deref(), Some("Freddie Mercury"));
        assert_eq!(
            results[0]
                .metadata
                .get(META_ISWC)
                .and_then(serde_json::Value::as_str),
            Some("T0345246801")
        );
    }

    #[test]
    fn iswc_provider_parse_invalid_json_returns_err() {
        assert!(matches!(
            IswcProvider::parse_works("iswc", "bad"),
            Err(ProviderError::Other(_))
        ));
    }

    #[tokio::test]
    async fn iswc_provider_search_without_iswc_returns_not_supported() {
        let p = IswcProvider::new("App/1.0");
        let q = SearchQuery {
            max_results: Some(5),
            ..Default::default()
        };
        assert!(matches!(
            p.search(&q).await,
            Err(ProviderError::NotSupported(_))
        ));
    }
}
