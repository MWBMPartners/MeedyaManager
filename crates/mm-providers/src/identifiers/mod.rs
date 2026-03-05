// (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
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

use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

use crate::traits::{
    Capabilities, MediaType, MetadataProvider, ProviderError, ProviderResult, SearchQuery,
};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn net_err(e: reqwest::Error) -> ProviderError {
    ProviderError::Network(e.to_string())
}

fn parse_err(context: &str, e: impl std::fmt::Display) -> ProviderError {
    ProviderError::Parse(format!("{context}: {e}"))
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
    let normalised: String = iswc.to_uppercase().chars()
        .filter(|c| c.is_ascii_alphanumeric())
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
    capabilities: Capabilities,
}

impl IsrcProvider {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self::with_base_url(user_agent, "https://musicbrainz.org")
    }

    pub fn with_base_url(user_agent: impl Into<String>, base_url: impl Into<String>) -> Self {
        let user_agent = user_agent.into();
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            user_agent,
            capabilities: Capabilities {
                media_types: vec![MediaType::Identifier, MediaType::Music],
                supports_search: false,
                supports_isrc: true,
                supports_iswc: false,
                provides_cover_art: false,
                provides_fingerprint: false,
                requires_auth: false,
                display_name: "ISRC (via MusicBrainz)".into(),
                homepage_url: "https://isrc.ifpi.org".into(),
            },
        }
    }

    fn parse_recordings(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct MbResponse { recordings: Vec<MbRecording> }
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
        struct MbCredit { artist: Option<MbArtist> }
        #[derive(Deserialize)]
        struct MbArtist { name: Option<String> }
        #[derive(Deserialize)]
        struct MbRelease {
            title: Option<String>,
            date: Option<String>,
        }

        let resp: MbResponse = serde_json::from_str(body)
            .map_err(|e| parse_err("ISRC/MusicBrainz response", e))?;

        let results = resp.recordings.into_iter().map(|rec| {
            let artist = rec.artist_credit.as_deref().map(|credits| {
                credits.iter()
                    .filter_map(|c| c.artist.as_ref()?.name.as_deref())
                    .collect::<Vec<_>>()
                    .join("; ")
            });
            let first_release = rec.releases.as_deref().and_then(|r| r.first());
            let album = first_release.and_then(|r| r.title.clone());
            let year = first_release
                .and_then(|r| r.date.as_deref())
                .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());

            ProviderResult {
                provider: provider_name.to_owned(),
                provider_id: rec.id.unwrap_or_default(),
                title: rec.title,
                artist,
                album,
                year,
                isrc: rec.isrcs.and_then(|v| v.into_iter().next()),
                duration_secs: rec.length.map(|ms| ms as f64 / 1000.0),
                ..Default::default()
            }
        }).collect();
        Ok(results)
    }
}

impl MetadataProvider for IsrcProvider {
    fn name(&self) -> &str { "isrc" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { !self.user_agent.is_empty() }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.is_enabled() {
            return Err(ProviderError::Disabled("isrc".into()));
        }

        let isrc = query.isrc.as_deref().ok_or_else(|| ProviderError::NotSupported {
            provider: "isrc".into(),
            reason: "ISRC query requires an ISRC code".into(),
        })?;

        if !validate_isrc(isrc) {
            return Err(ProviderError::Parse(format!("Invalid ISRC format: {isrc}")));
        }

        debug!(provider = "isrc", isrc = isrc, "Sending ISRC lookup request");

        let url = format!("{}/ws/2/recording/", self.base_url);
        let response = self.client
            .get(&url)
            .header("User-Agent", &self.user_agent)
            .header("Accept", "application/json")
            .query(&[
                ("query", &format!("isrc:{isrc}")),
                ("limit", &query.max_results.to_string()),
                ("fmt", &"json".to_owned()),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let s = response.status();
            if s.as_u16() == 503 {
                return Err(ProviderError::RateLimited { provider: "isrc".into() });
            }
            return Err(ProviderError::Network(format!("HTTP {s}")));
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
    capabilities: Capabilities,
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
            client: Client::new(),
            base_url: base_url.into(),
            username,
            password,
            capabilities: Capabilities {
                media_types: vec![MediaType::Identifier, MediaType::Video],
                supports_search: true,
                supports_isrc: false,
                supports_iswc: false,
                provides_cover_art: false,
                provides_fingerprint: false,
                requires_auth: true,
                display_name: "EIDR".into(),
                homepage_url: "https://eidr.org".into(),
            },
        }
    }

    /// Parse an EIDR XML response into a `ProviderResult`.
    ///
    /// EIDR returns XML, but the registry also offers JSON via Accept header.
    /// We request JSON for simplicity.
    fn parse_eidr_json(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
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
        #[serde(rename_all = "PascalCase")]
        struct EidrLocalizedName { value: Option<String> }

        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct EidrExtra { movie: Option<EidrMovie> }

        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct EidrMovie { directors: Option<Vec<String>> }

        let record: EidrRecord = serde_json::from_str(body)
            .map_err(|e| parse_err("EIDR response", e))?;

        let year = record.release_date.as_deref()
            .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());

        let director = record.extra.as_ref()
            .and_then(|e| e.movie.as_ref())
            .and_then(|m| m.directors.as_deref())
            .and_then(|d| d.first())
            .cloned();

        let result = ProviderResult {
            provider: provider_name.to_owned(),
            provider_id: record.id.clone().unwrap_or_default(),
            title: record.resource_name.and_then(|n| n.value),
            artist: director,  // Director for film
            year,
            eidr: record.id,
            ..Default::default()
        };

        Ok(vec![result])
    }
}

impl MetadataProvider for EidrProvider {
    fn name(&self) -> &str { "eidr" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { self.username.is_some() && self.password.is_some() }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.is_enabled() {
            return Err(ProviderError::Disabled("eidr".into()));
        }

        let eidr = query.eidr.as_deref().ok_or_else(|| ProviderError::NotSupported {
            provider: "eidr".into(),
            reason: "EIDR query requires an EIDR DOI".into(),
        })?;

        if !validate_eidr(eidr) {
            return Err(ProviderError::Parse(format!("Invalid EIDR format: {eidr}")));
        }

        debug!(provider = "eidr", eidr = eidr, "Sending EIDR lookup request");

        let url = format!("{}/EIDR/object/{}", self.base_url, eidr);
        let response = self.client
            .get(&url)
            .basic_auth(
                self.username.as_deref().unwrap(),
                self.password.as_deref(),
            )
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let s = response.status();
            if s.as_u16() == 401 {
                return Err(ProviderError::Auth("Invalid EIDR credentials".into()));
            }
            return Err(ProviderError::Network(format!("HTTP {s}")));
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
    capabilities: Capabilities,
}

impl IswcProvider {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self::with_base_url(user_agent, "https://musicbrainz.org")
    }

    pub fn with_base_url(user_agent: impl Into<String>, base_url: impl Into<String>) -> Self {
        let user_agent = user_agent.into();
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            user_agent,
            capabilities: Capabilities {
                media_types: vec![MediaType::Identifier, MediaType::Music],
                supports_search: false,
                supports_isrc: false,
                supports_iswc: true,
                provides_cover_art: false,
                provides_fingerprint: false,
                requires_auth: false,
                display_name: "ISWC (via MusicBrainz)".into(),
                homepage_url: "https://iswc.org".into(),
            },
        }
    }

    fn parse_works(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct MbWorksResponse { works: Vec<MbWork> }
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
        struct MbRelArtist { name: Option<String> }

        let resp: MbWorksResponse = serde_json::from_str(body)
            .map_err(|e| parse_err("ISWC/MusicBrainz response", e))?;

        let results = resp.works.into_iter().map(|work| {
            // Find the composer from relations
            let composer = work.relations.as_deref().and_then(|rels| {
                rels.iter()
                    .find(|r| r.rel_type.as_deref() == Some("composer"))
                    .and_then(|r| r.artist.as_ref()?.name.clone())
            });

            ProviderResult {
                provider: provider_name.to_owned(),
                provider_id: work.id.unwrap_or_default(),
                title: work.title,
                artist: composer,
                iswc: work.iswcs.and_then(|v| v.into_iter().next()),
                ..Default::default()
            }
        }).collect();
        Ok(results)
    }
}

impl MetadataProvider for IswcProvider {
    fn name(&self) -> &str { "iswc" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { !self.user_agent.is_empty() }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.is_enabled() {
            return Err(ProviderError::Disabled("iswc".into()));
        }

        let iswc = query.iswc.as_deref().ok_or_else(|| ProviderError::NotSupported {
            provider: "iswc".into(),
            reason: "ISWC query requires an ISWC code".into(),
        })?;

        if !validate_iswc(iswc) {
            return Err(ProviderError::Parse(format!("Invalid ISWC format: {iswc}")));
        }

        debug!(provider = "iswc", iswc = iswc, "Sending ISWC lookup request");

        let url = format!("{}/ws/2/work/", self.base_url);
        let response = self.client
            .get(&url)
            .header("User-Agent", &self.user_agent)
            .header("Accept", "application/json")
            .query(&[
                ("query", &format!("iswc:{iswc}")),
                ("limit", &query.max_results.to_string()),
                ("fmt", &"json".to_owned()),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let s = response.status();
            if s.as_u16() == 503 {
                return Err(ProviderError::RateLimited { provider: "iswc".into() });
            }
            return Err(ProviderError::Network(format!("HTTP {s}")));
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
        assert_eq!(IsrcProvider::new("App/1.0").name(), "isrc");
    }

    #[test]
    fn isrc_provider_enabled_with_user_agent() {
        assert!(IsrcProvider::new("App/1.0").is_enabled());
    }

    #[test]
    fn isrc_provider_disabled_without_user_agent() {
        assert!(!IsrcProvider::new("").is_enabled());
    }

    #[test]
    fn isrc_provider_supports_isrc() {
        assert!(IsrcProvider::new("App/1.0").capabilities().supports_isrc);
    }

    #[test]
    fn isrc_provider_no_auth_required() {
        assert!(!IsrcProvider::new("App/1.0").capabilities().requires_auth);
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
            Err(ProviderError::Parse(_))
        ));
    }

    #[tokio::test]
    async fn isrc_provider_search_without_isrc_returns_not_supported() {
        let p = IsrcProvider::new("App/1.0");
        let q = SearchQuery { query: "track".into(), max_results: 5, ..Default::default() };
        assert!(matches!(p.search(&q).await, Err(ProviderError::NotSupported { .. })));
    }

    #[tokio::test]
    async fn isrc_provider_search_invalid_isrc_returns_parse_err() {
        let p = IsrcProvider::new("App/1.0");
        let mut q = SearchQuery::default();
        q.isrc = Some("BAD".into());
        q.max_results = 5;
        assert!(matches!(p.search(&q).await, Err(ProviderError::Parse(_))));
    }

    // =========================================================================
    // EIDR Provider tests
    // =========================================================================

    #[test]
    fn eidr_provider_name() {
        assert_eq!(EidrProvider::new(None, None).name(), "eidr");
    }

    #[test]
    fn eidr_provider_enabled_with_credentials() {
        assert!(EidrProvider::new(Some("user".into()), Some("pass".into())).is_enabled());
    }

    #[test]
    fn eidr_provider_disabled_without_credentials() {
        assert!(!EidrProvider::new(None, None).is_enabled());
    }

    #[test]
    fn eidr_provider_requires_auth() {
        assert!(EidrProvider::new(None, None).capabilities().requires_auth);
    }

    #[test]
    fn eidr_provider_video_media_type() {
        let p = EidrProvider::new(None, None);
        assert!(p.capabilities().supports_media_type(MediaType::Video));
        assert!(p.capabilities().supports_media_type(MediaType::Identifier));
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
        assert_eq!(results[0].eidr.as_deref(), Some("10.5240/AEBE-0317-CE0D-4943-5916-E"));
    }

    // =========================================================================
    // ISWC Provider tests
    // =========================================================================

    #[test]
    fn iswc_provider_name() {
        assert_eq!(IswcProvider::new("App/1.0").name(), "iswc");
    }

    #[test]
    fn iswc_provider_enabled_with_user_agent() {
        assert!(IswcProvider::new("App/1.0").is_enabled());
    }

    #[test]
    fn iswc_provider_supports_iswc() {
        assert!(IswcProvider::new("App/1.0").capabilities().supports_iswc);
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
        assert_eq!(results[0].iswc.as_deref(), Some("T0345246801"));
    }

    #[test]
    fn iswc_provider_parse_invalid_json_returns_err() {
        assert!(matches!(
            IswcProvider::parse_works("iswc", "bad"),
            Err(ProviderError::Parse(_))
        ));
    }

    #[tokio::test]
    async fn iswc_provider_search_without_iswc_returns_not_supported() {
        let p = IswcProvider::new("App/1.0");
        let q = SearchQuery { query: "track".into(), max_results: 5, ..Default::default() };
        assert!(matches!(p.search(&q).await, Err(ProviderError::NotSupported { .. })));
    }
}
