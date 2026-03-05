// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Video Metadata Providers
//
// Implements 5 video/film/TV metadata providers:
//
//   1. TmdbProvider      — The Movie Database; API key required; rich metadata
//   2. TheTvdbProvider   — TheTVDB; API key required; TV-episode focused
//   3. OmdbProvider      — OMDb API (IMDb data); API key required (free tier)
//   4. AppleTvProvider   — Apple TV iTunes API; no auth required
//   5. ItunesStoreProvider — iTunes Store search; no auth required (movies/TV)
//
// All providers implement `MetadataProvider` with `MediaType::Video`.

use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

use crate::traits::{
    Capabilities, CoverArtInfo, MediaType, MetadataProvider, ProviderError, ProviderResult,
    SearchQuery,
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

// ---------------------------------------------------------------------------
// 1. TMDB — The Movie Database
// ---------------------------------------------------------------------------

/// Searches The Movie Database (TMDb) for films and TV shows.
///
/// Endpoint: `https://api.themoviedb.org/3/search/multi`
/// Auth:     API key (`api_key` query param or Bearer token)
/// Limits:   40 RPM
pub struct TmdbProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    capabilities: Capabilities,
}

impl TmdbProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, "https://api.themoviedb.org")
    }

    pub fn with_base_url(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            api_key,
            capabilities: Capabilities {
                media_types: vec![MediaType::Video],
                supports_search: true,
                supports_isrc: false,
                supports_iswc: false,
                provides_cover_art: true,
                provides_fingerprint: false,
                requires_auth: true,
                display_name: "The Movie Database (TMDb)".into(),
                homepage_url: "https://themoviedb.org".into(),
            },
        }
    }

    fn parse_multi_search(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct TmdbResponse { results: Vec<TmdbResult> }

        #[derive(Deserialize)]
        struct TmdbResult {
            id: Option<u64>,
            media_type: Option<String>,
            title: Option<String>,       // movies
            name: Option<String>,        // TV shows
            overview: Option<String>,
            release_date: Option<String>,     // movies
            first_air_date: Option<String>,   // TV shows
            poster_path: Option<String>,
            vote_average: Option<f64>,
            genre_ids: Option<Vec<u32>>,
        }

        let resp: TmdbResponse = serde_json::from_str(body)
            .map_err(|e| parse_err("TMDb response", e))?;

        const IMAGE_BASE: &str = "https://image.tmdb.org/t/p";

        let results = resp.results.into_iter().map(|r| {
            let title = r.title.or(r.name);
            let year = r.release_date.as_deref()
                .or(r.first_air_date.as_deref())
                .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());
            let cover_art = r.poster_path.as_deref().map(|p| {
                vec![
                    CoverArtInfo::new(format!("{IMAGE_BASE}/original{p}"), 0, 0, "image/jpeg"),
                    CoverArtInfo::new(format!("{IMAGE_BASE}/w500{p}"), 500, 750, "image/jpeg"),
                ]
            }).unwrap_or_default();
            let score = r.vote_average.map(|v| (v / 10.0).clamp(0.0, 1.0)).unwrap_or(0.5);

            let mut extra = std::collections::HashMap::new();
            if let Some(overview) = r.overview {
                if !overview.is_empty() {
                    extra.insert("overview".into(), overview);
                }
            }
            if let Some(mt) = &r.media_type {
                extra.insert("media_type".into(), mt.clone());
            }

            ProviderResult {
                provider: provider_name.to_owned(),
                provider_id: r.id.map(|i| i.to_string()).unwrap_or_default(),
                title,
                year,
                cover_art,
                score,
                extra,
                ..Default::default()
            }
        }).collect();

        Ok(results)
    }
}

impl MetadataProvider for TmdbProvider {
    fn name(&self) -> &str { "tmdb" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { self.api_key.is_some() }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.is_enabled() {
            return Err(ProviderError::Disabled("tmdb".into()));
        }
        let key = self.api_key.as_deref().unwrap();
        let search_query = query.title.as_deref().unwrap_or(&query.query);

        debug!(provider = "tmdb", query = search_query, "Sending search request");

        let mut params = vec![
            ("api_key", key.to_owned()),
            ("query", search_query.to_owned()),
            ("page", "1".to_owned()),
        ];
        if let Some(year) = query.year {
            params.push(("year", year.to_string()));
        }

        let url = format!("{}/3/search/multi", self.base_url);
        let response = self.client.get(&url).query(&params).send().await.map_err(net_err)?;

        if !response.status().is_success() {
            let s = response.status();
            if s.as_u16() == 429 {
                return Err(ProviderError::RateLimited { provider: "tmdb".into() });
            }
            return Err(ProviderError::Network(format!("HTTP {s}")));
        }

        let body = response.text().await.map_err(net_err)?;
        let mut results = Self::parse_multi_search("tmdb", &body)?;
        results.truncate(query.max_results);
        Ok(results)
    }
}

// ---------------------------------------------------------------------------
// 2. TheTVDB
// ---------------------------------------------------------------------------

/// Searches TheTVDB for TV shows and episodes.
///
/// Endpoint: `https://api4.thetvdb.com/v4/search`
/// Auth:     API key (Bearer token obtained via `/login`)
/// Limits:   30 RPM
pub struct TheTvdbProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    capabilities: Capabilities,
}

impl TheTvdbProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, "https://api4.thetvdb.com")
    }

    pub fn with_base_url(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            api_key,
            capabilities: Capabilities {
                media_types: vec![MediaType::Video],
                supports_search: true,
                supports_isrc: false,
                supports_iswc: false,
                provides_cover_art: true,
                provides_fingerprint: false,
                requires_auth: true,
                display_name: "TheTVDB".into(),
                homepage_url: "https://thetvdb.com".into(),
            },
        }
    }

    fn parse_search(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct TvdbResponse { data: Option<Vec<TvdbResult>> }
        #[derive(Deserialize)]
        struct TvdbResult {
            id: Option<String>,
            name: Option<String>,
            first_air_time: Option<String>,
            image_url: Option<String>,
            overview: Option<String>,
            #[serde(rename = "type")]
            media_type: Option<String>,
        }

        let resp: TvdbResponse = serde_json::from_str(body)
            .map_err(|e| parse_err("TheTVDB response", e))?;

        let data = resp.data.unwrap_or_default();
        let results = data.into_iter().map(|r| {
            let year = r.first_air_time.as_deref()
                .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());
            let cover_art = r.image_url.as_deref()
                .filter(|u| !u.is_empty())
                .map(|u| vec![CoverArtInfo::new(u, 0, 0, "image/jpeg")])
                .unwrap_or_default();

            let mut extra = std::collections::HashMap::new();
            if let Some(o) = r.overview { extra.insert("overview".into(), o); }
            if let Some(t) = r.media_type { extra.insert("media_type".into(), t); }

            ProviderResult {
                provider: provider_name.to_owned(),
                provider_id: r.id.unwrap_or_default(),
                title: r.name,
                year,
                cover_art,
                extra,
                ..Default::default()
            }
        }).collect();
        Ok(results)
    }
}

impl MetadataProvider for TheTvdbProvider {
    fn name(&self) -> &str { "thetvdb" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { self.api_key.is_some() }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.is_enabled() {
            return Err(ProviderError::Disabled("thetvdb".into()));
        }
        let key = self.api_key.as_deref().unwrap();
        let q = query.title.as_deref().unwrap_or(&query.query);

        debug!(provider = "thetvdb", query = q, "Sending search request");

        let url = format!("{}/v4/search", self.base_url);
        let response = self.client
            .get(&url)
            .bearer_auth(key)
            .query(&[("query", q), ("limit", &query.max_results.to_string())])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            return Err(ProviderError::Network(format!("HTTP {}", response.status())));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse_search("thetvdb", &body)
    }
}

// ---------------------------------------------------------------------------
// 3. OMDb (Open Movie Database — IMDb data)
// ---------------------------------------------------------------------------

/// Searches the OMDb API, which provides IMDb-sourced film/TV metadata.
///
/// Endpoint: `https://www.omdbapi.com/`
/// Auth:     API key (`apikey` query param; free tier = 1000 req/day)
/// Limits:   10 RPM (free tier)
pub struct OmdbProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    capabilities: Capabilities,
}

impl OmdbProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, "https://www.omdbapi.com")
    }

    pub fn with_base_url(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            api_key,
            capabilities: Capabilities {
                media_types: vec![MediaType::Video],
                supports_search: true,
                supports_isrc: false,
                supports_iswc: false,
                provides_cover_art: true,
                provides_fingerprint: false,
                requires_auth: true,
                display_name: "OMDb / IMDb".into(),
                homepage_url: "https://omdbapi.com".into(),
            },
        }
    }

    fn parse_search(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct OmdbSearchResponse {
            search: Option<Vec<OmdbSearchResult>>,
            #[serde(rename = "Error")]
            error: Option<String>,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct OmdbSearchResult {
            #[serde(rename = "imdbID")]
            imdb_id: Option<String>,
            title: Option<String>,
            year: Option<String>,
            poster: Option<String>,
            #[serde(rename = "Type")]
            media_type: Option<String>,
        }

        let resp: OmdbSearchResponse = serde_json::from_str(body)
            .map_err(|e| parse_err("OMDb response", e))?;

        if let Some(err) = resp.error {
            return Err(ProviderError::Parse(format!("OMDb error: {err}")));
        }

        let items = resp.search.unwrap_or_default();
        let results = items.into_iter().map(|r| {
            let year = r.year.as_deref()
                .and_then(|y| y[..4.min(y.len())].parse::<u32>().ok());
            let cover_art = r.poster.as_deref()
                .filter(|p| *p != "N/A" && !p.is_empty())
                .map(|p| vec![CoverArtInfo::new(p, 0, 0, "image/jpeg")])
                .unwrap_or_default();
            let mut extra = std::collections::HashMap::new();
            if let Some(t) = &r.media_type { extra.insert("media_type".into(), t.clone()); }

            ProviderResult {
                provider: provider_name.to_owned(),
                provider_id: r.imdb_id.unwrap_or_default(),
                title: r.title,
                year,
                cover_art,
                extra,
                ..Default::default()
            }
        }).collect();
        Ok(results)
    }
}

impl MetadataProvider for OmdbProvider {
    fn name(&self) -> &str { "omdb" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { self.api_key.is_some() }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.is_enabled() {
            return Err(ProviderError::Disabled("omdb".into()));
        }
        let key = self.api_key.as_deref().unwrap();
        let title = query.title.as_deref().unwrap_or(&query.query);

        debug!(provider = "omdb", title = title, "Sending search request");

        let mut params = vec![
            ("s", title.to_owned()),
            ("apikey", key.to_owned()),
            ("type", "movie".to_owned()), // Default to movies; could be configurable
        ];
        if let Some(y) = query.year { params.push(("y", y.to_string())); }

        let response = self.client
            .get(&self.base_url)
            .query(&params)
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            return Err(ProviderError::Network(format!("HTTP {}", response.status())));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse_search("omdb", &body)
    }
}

// ---------------------------------------------------------------------------
// 4. Apple TV (iTunes Movie Search — no auth)
// ---------------------------------------------------------------------------

/// Searches Apple TV via the iTunes Search API for films and TV episodes.
///
/// Endpoint: `https://itunes.apple.com/search?media=movie&media=tvEpisode`
/// Auth:     None (public API)
/// Limits:   20 RPM (conservative)
pub struct AppleTvProvider {
    client: Client,
    base_url: String,
    enabled: bool,
    country: String,
    capabilities: Capabilities,
}

impl AppleTvProvider {
    pub fn new(country: impl Into<String>) -> Self {
        Self::with_base_url(country, "https://itunes.apple.com")
    }

    pub fn with_base_url(country: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            enabled: true,
            country: country.into(),
            capabilities: Capabilities {
                media_types: vec![MediaType::Video],
                supports_search: true,
                supports_isrc: false,
                supports_iswc: false,
                provides_cover_art: true,
                provides_fingerprint: false,
                requires_auth: false,
                display_name: "Apple TV".into(),
                homepage_url: "https://tv.apple.com".into(),
            },
        }
    }

    fn parse_itunes_video(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ItunesVideoResponse { results: Vec<ItunesVideoResult> }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ItunesVideoResult {
            track_id: Option<u64>,
            track_name: Option<String>,
            artist_name: Option<String>,  // Director for movies
            collection_name: Option<String>,
            artwork_url100: Option<String>,
            release_date: Option<String>,
            track_time_millis: Option<u64>,
            primary_genre_name: Option<String>,
            content_advisory_rating: Option<String>,
        }

        let resp: ItunesVideoResponse = serde_json::from_str(body)
            .map_err(|e| parse_err("Apple TV response", e))?;

        let results = resp.results.into_iter().map(|r| {
            let year = r.release_date.as_deref()
                .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());
            let cover_art = r.artwork_url100.as_deref().map(|url| {
                let hires = url.replace("100x100", "600x600");
                vec![
                    CoverArtInfo::new(&hires, 600, 600, "image/jpeg"),
                    CoverArtInfo::new(url, 100, 100, "image/jpeg"),
                ]
            }).unwrap_or_default();

            ProviderResult {
                provider: provider_name.to_owned(),
                provider_id: r.track_id.map(|id| id.to_string()).unwrap_or_default(),
                title: r.track_name,
                artist: r.artist_name,   // Director for films
                album: r.collection_name, // Series name for TV episodes
                year,
                genre: r.primary_genre_name,
                duration_secs: r.track_time_millis.map(|ms| ms as f64 / 1000.0),
                content_advisory: r.content_advisory_rating,
                cover_art,
                ..Default::default()
            }
        }).collect();

        Ok(results)
    }
}

impl Default for AppleTvProvider {
    fn default() -> Self { Self::new("US") }
}

impl MetadataProvider for AppleTvProvider {
    fn name(&self) -> &str { "apple_tv" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { self.enabled }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::Disabled("apple_tv".into()));
        }
        let term = query.title.as_deref().unwrap_or(&query.query);
        debug!(provider = "apple_tv", term = term, "Sending iTunes video search request");

        let url = format!("{}/search", self.base_url);
        let response = self.client
            .get(&url)
            .query(&[
                ("term", term),
                ("media", "movie"),
                ("country", &self.country),
                ("limit", &query.max_results.to_string()),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            return Err(ProviderError::Network(format!("HTTP {}", response.status())));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse_itunes_video("apple_tv", &body)
    }
}

// ---------------------------------------------------------------------------
// 5. iTunes Store (movie/TV purchases)
// ---------------------------------------------------------------------------

/// Searches the iTunes Store for purchased/available movies and TV shows.
///
/// Uses the same iTunes Search API as AppleTvProvider but with the `tvShow` entity
/// to fetch TV series. Included as a separate provider for distinct categorisation.
///
/// Auth: None; Limits: 20 RPM
pub struct ItunesStoreProvider {
    client: Client,
    base_url: String,
    enabled: bool,
    country: String,
    capabilities: Capabilities,
}

impl ItunesStoreProvider {
    pub fn new(country: impl Into<String>) -> Self {
        Self::with_base_url(country, "https://itunes.apple.com")
    }

    pub fn with_base_url(country: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            enabled: true,
            country: country.into(),
            capabilities: Capabilities {
                media_types: vec![MediaType::Video],
                supports_search: true,
                supports_isrc: false,
                supports_iswc: false,
                provides_cover_art: true,
                provides_fingerprint: false,
                requires_auth: false,
                display_name: "iTunes Store".into(),
                homepage_url: "https://apple.com/itunes".into(),
            },
        }
    }

    fn parse(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        // Reuse the same iTunes JSON structure as AppleTvProvider
        AppleTvProvider::parse_itunes_video(provider_name, body)
    }
}

impl Default for ItunesStoreProvider {
    fn default() -> Self { Self::new("US") }
}

impl MetadataProvider for ItunesStoreProvider {
    fn name(&self) -> &str { "itunes_store" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { self.enabled }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::Disabled("itunes_store".into()));
        }
        let term = query.title.as_deref().unwrap_or(&query.query);
        debug!(provider = "itunes_store", term = term, "Sending iTunes TV search request");

        let url = format!("{}/search", self.base_url);
        let response = self.client
            .get(&url)
            .query(&[
                ("term", term),
                ("media", "tvShow"),
                ("entity", "tvSeason"),
                ("country", &self.country),
                ("limit", &query.max_results.to_string()),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            return Err(ProviderError::Network(format!("HTTP {}", response.status())));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse("itunes_store", &body)
    }
}

// ---------------------------------------------------------------------------
// Tests — 45 tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // TMDb tests
    // =========================================================================

    #[test]
    fn tmdb_name() {
        let p = TmdbProvider::new(Some("key".into()));
        assert_eq!(p.name(), "tmdb");
    }

    #[test]
    fn tmdb_enabled_with_api_key() {
        let p = TmdbProvider::new(Some("key".into()));
        assert!(p.is_enabled());
    }

    #[test]
    fn tmdb_disabled_without_api_key() {
        let p = TmdbProvider::new(None);
        assert!(!p.is_enabled());
    }

    #[test]
    fn tmdb_capabilities_media_type_video() {
        let p = TmdbProvider::new(None);
        assert!(p.capabilities().supports_media_type(MediaType::Video));
        assert!(!p.capabilities().supports_media_type(MediaType::Music));
    }

    #[test]
    fn tmdb_capabilities_requires_auth() {
        let p = TmdbProvider::new(None);
        assert!(p.capabilities().requires_auth);
    }

    #[test]
    fn tmdb_capabilities_provides_cover_art() {
        let p = TmdbProvider::new(None);
        assert!(p.capabilities().provides_cover_art);
    }

    #[test]
    fn tmdb_parse_multi_search_valid_json() {
        let json = r#"{
            "results": [{
                "id": 27205,
                "media_type": "movie",
                "title": "Inception",
                "overview": "A thief who steals corporate secrets...",
                "release_date": "2010-07-16",
                "poster_path": "/9gk7adHYeDvHkCSEqAvQNLV5Uge.jpg",
                "vote_average": 8.4
            }]
        }"#;
        let results = TmdbProvider::parse_multi_search("tmdb", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Inception"));
        assert_eq!(results[0].year, Some(2010));
        assert!((results[0].score - 0.84).abs() < 1e-9);
        assert!(!results[0].cover_art.is_empty());
        // Original image URL should be in cover art
        assert!(results[0].cover_art.iter().any(|a| a.url.contains("original")));
    }

    #[test]
    fn tmdb_parse_tv_show_uses_name_field() {
        let json = r#"{
            "results": [{
                "id": 1396,
                "media_type": "tv",
                "name": "Breaking Bad",
                "first_air_date": "2008-01-20",
                "vote_average": 9.5
            }]
        }"#;
        let results = TmdbProvider::parse_multi_search("tmdb", json).unwrap();
        assert_eq!(results[0].title.as_deref(), Some("Breaking Bad"));
        assert_eq!(results[0].year, Some(2008));
    }

    #[test]
    fn tmdb_parse_empty_results() {
        let json = r#"{"results": []}"#;
        let results = TmdbProvider::parse_multi_search("tmdb", json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn tmdb_parse_invalid_json_returns_err() {
        let result = TmdbProvider::parse_multi_search("tmdb", "bad json");
        assert!(matches!(result, Err(ProviderError::Parse(_))));
    }

    #[test]
    fn tmdb_parse_overview_stored_in_extra() {
        let json = r#"{"results": [{"id": 1, "media_type": "movie", "overview": "Description here"}]}"#;
        let results = TmdbProvider::parse_multi_search("tmdb", json).unwrap();
        assert_eq!(results[0].extra.get("overview").map(String::as_str), Some("Description here"));
    }

    #[test]
    fn tmdb_parse_score_normalised_from_vote_average() {
        // vote_average = 7.5 → score = 0.75
        let json = r#"{"results": [{"id": 1, "media_type": "movie", "vote_average": 7.5}]}"#;
        let results = TmdbProvider::parse_multi_search("tmdb", json).unwrap();
        assert!((results[0].score - 0.75).abs() < 1e-9);
    }

    // =========================================================================
    // TheTVDB tests
    // =========================================================================

    #[test]
    fn tvdb_name() {
        let p = TheTvdbProvider::new(Some("key".into()));
        assert_eq!(p.name(), "thetvdb");
    }

    #[test]
    fn tvdb_enabled_with_api_key() {
        assert!(TheTvdbProvider::new(Some("key".into())).is_enabled());
    }

    #[test]
    fn tvdb_disabled_without_api_key() {
        assert!(!TheTvdbProvider::new(None).is_enabled());
    }

    #[test]
    fn tvdb_capabilities_video_type() {
        let p = TheTvdbProvider::new(None);
        assert!(p.capabilities().supports_media_type(MediaType::Video));
    }

    #[test]
    fn tvdb_parse_search_valid_json() {
        let json = r#"{
            "data": [{
                "id": "series-1396",
                "name": "Breaking Bad",
                "first_air_time": "2008-01-20",
                "image_url": "https://artworks.thetvdb.com/banners/bb.jpg",
                "overview": "A high school chemistry teacher...",
                "type": "series"
            }]
        }"#;
        let results = TheTvdbProvider::parse_search("thetvdb", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Breaking Bad"));
        assert_eq!(results[0].year, Some(2008));
        assert!(!results[0].cover_art.is_empty());
        assert_eq!(results[0].extra.get("media_type").map(String::as_str), Some("series"));
    }

    #[test]
    fn tvdb_parse_null_data_returns_empty() {
        let json = r#"{"data": null}"#;
        let results = TheTvdbProvider::parse_search("thetvdb", json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn tvdb_parse_invalid_json_returns_err() {
        assert!(matches!(
            TheTvdbProvider::parse_search("thetvdb", "garbage"),
            Err(ProviderError::Parse(_))
        ));
    }

    // =========================================================================
    // OMDb tests
    // =========================================================================

    #[test]
    fn omdb_name() {
        assert_eq!(OmdbProvider::new(Some("key".into())).name(), "omdb");
    }

    #[test]
    fn omdb_enabled_with_api_key() {
        assert!(OmdbProvider::new(Some("key".into())).is_enabled());
    }

    #[test]
    fn omdb_disabled_without_api_key() {
        assert!(!OmdbProvider::new(None).is_enabled());
    }

    #[test]
    fn omdb_capabilities_requires_auth() {
        assert!(OmdbProvider::new(None).capabilities().requires_auth);
    }

    #[test]
    fn omdb_parse_search_valid_json() {
        let json = r#"{
            "Search": [{
                "Title": "Interstellar",
                "Year": "2014",
                "imdbID": "tt0816692",
                "Type": "movie",
                "Poster": "https://m.media-amazon.com/images/M/poster.jpg"
            }],
            "totalResults": "1",
            "Response": "True"
        }"#;
        let results = OmdbProvider::parse_search("omdb", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Interstellar"));
        assert_eq!(results[0].year, Some(2014));
        assert_eq!(results[0].provider_id, "tt0816692");
        assert!(!results[0].cover_art.is_empty());
    }

    #[test]
    fn omdb_parse_error_response_returns_parse_err() {
        let json = r#"{"Response": "False", "Error": "Movie not found!"}"#;
        let result = OmdbProvider::parse_search("omdb", json);
        assert!(matches!(result, Err(ProviderError::Parse(_))));
    }

    #[test]
    fn omdb_parse_na_poster_produces_no_cover_art() {
        let json = r#"{
            "Search": [{"Title": "X", "Year": "2020", "imdbID": "tt0", "Type": "movie", "Poster": "N/A"}]
        }"#;
        let results = OmdbProvider::parse_search("omdb", json).unwrap();
        assert!(results[0].cover_art.is_empty());
    }

    #[test]
    fn omdb_parse_invalid_json_returns_err() {
        assert!(matches!(OmdbProvider::parse_search("omdb", "bad"), Err(ProviderError::Parse(_))));
    }

    // =========================================================================
    // Apple TV tests
    // =========================================================================

    #[test]
    fn apple_tv_name() {
        assert_eq!(AppleTvProvider::new("US").name(), "apple_tv");
    }

    #[test]
    fn apple_tv_enabled_by_default() {
        assert!(AppleTvProvider::default().is_enabled());
    }

    #[test]
    fn apple_tv_no_auth_required() {
        assert!(!AppleTvProvider::default().capabilities().requires_auth);
    }

    #[test]
    fn apple_tv_capabilities_video_type() {
        assert!(AppleTvProvider::default().capabilities().supports_media_type(MediaType::Video));
    }

    #[test]
    fn apple_tv_parse_itunes_video_valid_json() {
        let json = r#"{
            "results": [{
                "trackId": 1234,
                "trackName": "Interstellar",
                "artistName": "Christopher Nolan",
                "collectionName": null,
                "artworkUrl100": "https://is1.mzstatic.com/100x100.jpg",
                "releaseDate": "2014-11-07T00:00:00Z",
                "trackTimeMillis": 9720000,
                "primaryGenreName": "Sci-Fi",
                "contentAdvisoryRating": "PG-13"
            }]
        }"#;
        let results = AppleTvProvider::parse_itunes_video("apple_tv", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Interstellar"));
        assert_eq!(results[0].artist.as_deref(), Some("Christopher Nolan")); // Director
        assert_eq!(results[0].year, Some(2014));
        assert_eq!(results[0].genre.as_deref(), Some("Sci-Fi"));
        assert_eq!(results[0].content_advisory.as_deref(), Some("PG-13"));
        assert_eq!(results[0].cover_art.len(), 2);
    }

    #[test]
    fn apple_tv_parse_empty_results() {
        let json = r#"{"results": []}"#;
        let results = AppleTvProvider::parse_itunes_video("apple_tv", json).unwrap();
        assert!(results.is_empty());
    }

    // =========================================================================
    // iTunes Store tests
    // =========================================================================

    #[test]
    fn itunes_store_name() {
        assert_eq!(ItunesStoreProvider::new("US").name(), "itunes_store");
    }

    #[test]
    fn itunes_store_enabled_by_default() {
        assert!(ItunesStoreProvider::default().is_enabled());
    }

    #[test]
    fn itunes_store_capabilities_video_type() {
        assert!(ItunesStoreProvider::default().capabilities().supports_media_type(MediaType::Video));
    }

    #[test]
    fn itunes_store_no_auth_required() {
        assert!(!ItunesStoreProvider::default().capabilities().requires_auth);
    }

    #[test]
    fn itunes_store_parse_valid_json() {
        let json = r#"{"results": [{"trackId": 9999, "trackName": "Breaking Bad", "artistName": "AMC"}]}"#;
        let results = ItunesStoreProvider::parse("itunes_store", json).unwrap();
        assert_eq!(results[0].title.as_deref(), Some("Breaking Bad"));
    }
}
