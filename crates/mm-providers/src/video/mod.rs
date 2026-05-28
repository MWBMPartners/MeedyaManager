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
// All providers implement `MetadataProvider` with video search capability.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use crate::traits::{
    CoverArtInfo, META_CONTENT_ADVISORY, META_DURATION_SECS, META_PROVIDER_ID, MetadataProvider,
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

/// Standard `ProviderCapabilities` for a video-only provider.
fn video_caps(cover_art: bool) -> ProviderCapabilities {
    ProviderCapabilities {
        music_search: false,
        video_search: true,
        podcast_search: false,
        cover_art,
        lyrics: false,
        fingerprint_lookup: false,
        identifier_lookup: false,
    }
}

/// Insert duration (seconds) into result metadata using the conventional key.
fn insert_duration(result: &mut ProviderResult, secs: f64) {
    if let Some(num) = serde_json::Number::from_f64(secs) {
        result
            .metadata
            .insert(META_DURATION_SECS.into(), Value::Number(num));
    }
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
}

impl TmdbProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, "https://api.themoviedb.org")
    }

    pub fn with_base_url(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            api_key,
        }
    }

    fn configured(&self) -> bool {
        self.api_key.is_some()
    }

    fn parse_multi_search(
        provider_name: &str,
        body: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct TmdbResponse {
            results: Vec<TmdbResult>,
        }

        #[derive(Deserialize)]
        struct TmdbResult {
            id: Option<u64>,
            media_type: Option<String>,
            title: Option<String>, // movies
            name: Option<String>,  // TV shows
            overview: Option<String>,
            release_date: Option<String>,   // movies
            first_air_date: Option<String>, // TV shows
            poster_path: Option<String>,
            vote_average: Option<f64>,
            #[allow(dead_code)]
            genre_ids: Option<Vec<u32>>,
        }

        let resp: TmdbResponse =
            serde_json::from_str(body).map_err(|e| parse_err("TMDb response", e))?;

        const IMAGE_BASE: &str = "https://image.tmdb.org/t/p";

        let results = resp
            .results
            .into_iter()
            .map(|r| {
                let title = r.title.or(r.name);
                let year = r
                    .release_date
                    .as_deref()
                    .or(r.first_air_date.as_deref())
                    .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());
                let cover_art = r
                    .poster_path
                    .as_deref()
                    .map(|p| {
                        vec![
                            CoverArtInfo {
                                url: format!("{IMAGE_BASE}/original{p}"),
                                width: None,
                                height: None,
                                mime_type: Some("image/jpeg".into()),
                            },
                            CoverArtInfo {
                                url: format!("{IMAGE_BASE}/w500{p}"),
                                width: Some(500),
                                height: Some(750),
                                mime_type: Some("image/jpeg".into()),
                            },
                        ]
                    })
                    .unwrap_or_default();
                let score = r.vote_average.map_or(0.5, |v| (v / 10.0).clamp(0.0, 1.0));

                let mut result = ProviderResult::new(provider_name);
                result.title = title;
                result.year = year;
                result.cover_art = cover_art;
                result.score = score;

                if let Some(id) = r.id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id.to_string()));
                }
                if let Some(overview) = r.overview {
                    if !overview.is_empty() {
                        result
                            .metadata
                            .insert("overview".into(), Value::String(overview));
                    }
                }
                if let Some(mt) = &r.media_type {
                    result
                        .metadata
                        .insert("media_type".into(), Value::String(mt.clone()));
                }

                result
            })
            .collect();

        Ok(results)
    }
}

#[async_trait]
impl MetadataProvider for TmdbProvider {
    fn id(&self) -> &str {
        "tmdb"
    }

    fn display_name(&self) -> &str {
        "The Movie Database (TMDb)"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        video_caps(true)
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("tmdb".into()));
        }
        let key = self.api_key.as_deref().unwrap();
        let title_fallback = format!(
            "{} {}",
            query.title.as_deref().unwrap_or(""),
            query.artist.as_deref().unwrap_or("")
        );
        let search_query: String = query
            .title
            .clone()
            .unwrap_or_else(|| title_fallback.trim().to_owned());

        debug!(
            provider = "tmdb",
            query = %search_query,
            "Sending search request"
        );

        let mut params = vec![
            ("api_key", key.to_owned()),
            ("query", search_query),
            ("page", "1".to_owned()),
        ];
        if let Some(year) = query.year {
            params.push(("year", year.to_string()));
        }

        let url = format!("{}/3/search/multi", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let s = response.status();
            if s.as_u16() == 429 {
                return Err(ProviderError::RateLimited("tmdb".into()));
            }
            return Err(ProviderError::NetworkError(format!("HTTP {s}")));
        }

        let body = response.text().await.map_err(net_err)?;
        let mut results = Self::parse_multi_search("tmdb", &body)?;
        results.truncate(query.max_results.unwrap_or(10));
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
}

impl TheTvdbProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, "https://api4.thetvdb.com")
    }

    pub fn with_base_url(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            api_key,
        }
    }

    fn configured(&self) -> bool {
        self.api_key.is_some()
    }

    fn parse_search(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct TvdbResponse {
            data: Option<Vec<TvdbResult>>,
        }
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

        let resp: TvdbResponse =
            serde_json::from_str(body).map_err(|e| parse_err("TheTVDB response", e))?;

        let data = resp.data.unwrap_or_default();
        let results = data
            .into_iter()
            .map(|r| {
                let year = r
                    .first_air_time
                    .as_deref()
                    .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());
                let cover_art = r
                    .image_url
                    .as_deref()
                    .filter(|u| !u.is_empty())
                    .map(|u| {
                        vec![CoverArtInfo {
                            url: u.to_owned(),
                            width: None,
                            height: None,
                            mime_type: Some("image/jpeg".into()),
                        }]
                    })
                    .unwrap_or_default();

                let mut result = ProviderResult::new(provider_name);
                result.title = r.name;
                result.year = year;
                result.cover_art = cover_art;

                if let Some(id) = r.id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id));
                }
                if let Some(o) = r.overview {
                    result.metadata.insert("overview".into(), Value::String(o));
                }
                if let Some(t) = r.media_type {
                    result
                        .metadata
                        .insert("media_type".into(), Value::String(t));
                }

                result
            })
            .collect();
        Ok(results)
    }
}

#[async_trait]
impl MetadataProvider for TheTvdbProvider {
    fn id(&self) -> &str {
        "thetvdb"
    }

    fn display_name(&self) -> &str {
        "TheTVDB"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        video_caps(true)
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("thetvdb".into()));
        }
        let key = self.api_key.as_deref().unwrap();
        let fallback = format!(
            "{} {}",
            query.title.as_deref().unwrap_or(""),
            query.artist.as_deref().unwrap_or("")
        );
        let q: &str = query.title.as_deref().unwrap_or(fallback.trim());

        debug!(provider = "thetvdb", query = q, "Sending search request");

        let limit = query.max_results.unwrap_or(10).to_string();
        let url = format!("{}/v4/search", self.base_url);
        let response = self
            .client
            .get(&url)
            .bearer_auth(key)
            .query(&[("query", q), ("limit", &limit)])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            return Err(ProviderError::NetworkError(format!(
                "HTTP {}",
                response.status()
            )));
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
}

impl OmdbProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, "https://www.omdbapi.com")
    }

    pub fn with_base_url(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            api_key,
        }
    }

    fn configured(&self) -> bool {
        self.api_key.is_some()
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

        let resp: OmdbSearchResponse =
            serde_json::from_str(body).map_err(|e| parse_err("OMDb response", e))?;

        if let Some(err) = resp.error {
            return Err(ProviderError::Other(format!(
                "parse error: OMDb error: {err}"
            )));
        }

        let items = resp.search.unwrap_or_default();
        let results = items
            .into_iter()
            .map(|r| {
                let year = r
                    .year
                    .as_deref()
                    .and_then(|y| y[..4.min(y.len())].parse::<u32>().ok());
                let cover_art = r
                    .poster
                    .as_deref()
                    .filter(|p| *p != "N/A" && !p.is_empty())
                    .map(|p| {
                        vec![CoverArtInfo {
                            url: p.to_owned(),
                            width: None,
                            height: None,
                            mime_type: Some("image/jpeg".into()),
                        }]
                    })
                    .unwrap_or_default();

                let mut result = ProviderResult::new(provider_name);
                result.title = r.title;
                result.year = year;
                result.cover_art = cover_art;

                if let Some(id) = r.imdb_id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id));
                }
                if let Some(t) = &r.media_type {
                    result
                        .metadata
                        .insert("media_type".into(), Value::String(t.clone()));
                }

                result
            })
            .collect();
        Ok(results)
    }
}

#[async_trait]
impl MetadataProvider for OmdbProvider {
    fn id(&self) -> &str {
        "omdb"
    }

    fn display_name(&self) -> &str {
        "OMDb / IMDb"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        video_caps(true)
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("omdb".into()));
        }
        let key = self.api_key.as_deref().unwrap();
        let fallback = format!(
            "{} {}",
            query.title.as_deref().unwrap_or(""),
            query.artist.as_deref().unwrap_or("")
        );
        let title: &str = query.title.as_deref().unwrap_or(fallback.trim());

        debug!(provider = "omdb", title = title, "Sending search request");

        let mut params = vec![
            ("s", title.to_owned()),
            ("apikey", key.to_owned()),
            ("type", "movie".to_owned()), // Default to movies; could be configurable
        ];
        if let Some(y) = query.year {
            params.push(("y", y.to_string()));
        }

        let response = self
            .client
            .get(&self.base_url)
            .query(&params)
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            return Err(ProviderError::NetworkError(format!(
                "HTTP {}",
                response.status()
            )));
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
}

impl AppleTvProvider {
    pub fn new(country: impl Into<String>) -> Self {
        Self::with_base_url(country, "https://itunes.apple.com")
    }

    pub fn with_base_url(country: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            enabled: true,
            country: country.into(),
        }
    }

    pub(crate) fn parse_itunes_video(
        provider_name: &str,
        body: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ItunesVideoResponse {
            results: Vec<ItunesVideoResult>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ItunesVideoResult {
            track_id: Option<u64>,
            track_name: Option<String>,
            artist_name: Option<String>, // Director for movies
            collection_name: Option<String>,
            artwork_url100: Option<String>,
            release_date: Option<String>,
            track_time_millis: Option<u64>,
            primary_genre_name: Option<String>,
            content_advisory_rating: Option<String>,
        }

        let resp: ItunesVideoResponse =
            serde_json::from_str(body).map_err(|e| parse_err("Apple TV response", e))?;

        let results = resp
            .results
            .into_iter()
            .map(|r| {
                let year = r
                    .release_date
                    .as_deref()
                    .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());
                let cover_art = r
                    .artwork_url100
                    .as_deref()
                    .map(|url| {
                        let hires = url.replace("100x100", "600x600");
                        vec![
                            CoverArtInfo {
                                url: hires,
                                width: Some(600),
                                height: Some(600),
                                mime_type: Some("image/jpeg".into()),
                            },
                            CoverArtInfo {
                                url: url.to_owned(),
                                width: Some(100),
                                height: Some(100),
                                mime_type: Some("image/jpeg".into()),
                            },
                        ]
                    })
                    .unwrap_or_default();

                let mut result = ProviderResult::new(provider_name);
                result.title = r.track_name;
                result.artist = r.artist_name; // Director for films
                result.album = r.collection_name; // Series name for TV episodes
                result.year = year;
                result.genre = r.primary_genre_name;
                result.cover_art = cover_art;

                if let Some(id) = r.track_id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id.to_string()));
                }
                if let Some(ms) = r.track_time_millis {
                    insert_duration(&mut result, ms as f64 / 1000.0);
                }
                if let Some(advisory) = r.content_advisory_rating {
                    result
                        .metadata
                        .insert(META_CONTENT_ADVISORY.into(), Value::String(advisory));
                }

                result
            })
            .collect();

        Ok(results)
    }
}

impl Default for AppleTvProvider {
    fn default() -> Self {
        Self::new("US")
    }
}

#[async_trait]
impl MetadataProvider for AppleTvProvider {
    fn id(&self) -> &str {
        "apple_tv"
    }

    fn display_name(&self) -> &str {
        "Apple TV"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        video_caps(true)
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotConfigured("apple_tv".into()));
        }
        let fallback = format!(
            "{} {}",
            query.title.as_deref().unwrap_or(""),
            query.artist.as_deref().unwrap_or("")
        );
        let term: &str = query.title.as_deref().unwrap_or(fallback.trim());
        debug!(
            provider = "apple_tv",
            term = term,
            "Sending iTunes video search request"
        );

        let limit = query.max_results.unwrap_or(10).to_string();
        let url = format!("{}/search", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[
                ("term", term),
                ("media", "movie"),
                ("country", &self.country),
                ("limit", &limit),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            return Err(ProviderError::NetworkError(format!(
                "HTTP {}",
                response.status()
            )));
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
}

impl ItunesStoreProvider {
    pub fn new(country: impl Into<String>) -> Self {
        Self::with_base_url(country, "https://itunes.apple.com")
    }

    pub fn with_base_url(country: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            enabled: true,
            country: country.into(),
        }
    }

    pub(crate) fn parse(
        provider_name: &str,
        body: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        // Reuse the same iTunes JSON structure as AppleTvProvider
        AppleTvProvider::parse_itunes_video(provider_name, body)
    }
}

impl Default for ItunesStoreProvider {
    fn default() -> Self {
        Self::new("US")
    }
}

#[async_trait]
impl MetadataProvider for ItunesStoreProvider {
    fn id(&self) -> &str {
        "itunes_store"
    }

    fn display_name(&self) -> &str {
        "iTunes Store"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        video_caps(true)
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotConfigured("itunes_store".into()));
        }
        let fallback = format!(
            "{} {}",
            query.title.as_deref().unwrap_or(""),
            query.artist.as_deref().unwrap_or("")
        );
        let term: &str = query.title.as_deref().unwrap_or(fallback.trim());
        debug!(
            provider = "itunes_store",
            term = term,
            "Sending iTunes TV search request"
        );

        let limit = query.max_results.unwrap_or(10).to_string();
        let url = format!("{}/search", self.base_url);
        let response = self
            .client
            .get(&url)
            .query(&[
                ("term", term),
                ("media", "tvShow"),
                ("entity", "tvSeason"),
                ("country", &self.country),
                ("limit", &limit),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            return Err(ProviderError::NetworkError(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse("itunes_store", &body)
    }
}

// ---------------------------------------------------------------------------
// Tests
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
        assert_eq!(p.id(), "tmdb");
    }

    #[test]
    fn tmdb_capabilities_video_type() {
        let caps = TmdbProvider::new(None).capabilities();
        assert!(caps.video_search);
        assert!(!caps.music_search);
        assert!(caps.cover_art);
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
        assert!(
            results[0]
                .cover_art
                .iter()
                .any(|a| a.url.contains("original"))
        );
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
        assert!(matches!(result, Err(ProviderError::Other(_))));
    }

    #[test]
    fn tmdb_parse_overview_stored_in_metadata() {
        let json =
            r#"{"results": [{"id": 1, "media_type": "movie", "overview": "Description here"}]}"#;
        let results = TmdbProvider::parse_multi_search("tmdb", json).unwrap();
        assert_eq!(
            results[0]
                .metadata
                .get("overview")
                .and_then(serde_json::Value::as_str),
            Some("Description here")
        );
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
        assert_eq!(p.id(), "thetvdb");
    }

    #[test]
    fn tvdb_capabilities_video_type() {
        let caps = TheTvdbProvider::new(None).capabilities();
        assert!(caps.video_search);
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
        assert_eq!(
            results[0]
                .metadata
                .get("media_type")
                .and_then(serde_json::Value::as_str),
            Some("series")
        );
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
            Err(ProviderError::Other(_))
        ));
    }

    // =========================================================================
    // OMDb tests
    // =========================================================================

    #[test]
    fn omdb_name() {
        assert_eq!(OmdbProvider::new(Some("key".into())).id(), "omdb");
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
        assert_eq!(
            results[0]
                .metadata
                .get(META_PROVIDER_ID)
                .and_then(serde_json::Value::as_str),
            Some("tt0816692")
        );
        assert!(!results[0].cover_art.is_empty());
    }

    #[test]
    fn omdb_parse_error_response_returns_err() {
        let json = r#"{"Response": "False", "Error": "Movie not found!"}"#;
        let result = OmdbProvider::parse_search("omdb", json);
        assert!(matches!(result, Err(ProviderError::Other(_))));
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
        assert!(matches!(
            OmdbProvider::parse_search("omdb", "bad"),
            Err(ProviderError::Other(_))
        ));
    }

    // =========================================================================
    // Apple TV tests
    // =========================================================================

    #[test]
    fn apple_tv_name() {
        assert_eq!(AppleTvProvider::new("US").id(), "apple_tv");
    }

    #[test]
    fn apple_tv_capabilities_video_type() {
        assert!(AppleTvProvider::default().capabilities().video_search);
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
        assert_eq!(
            results[0]
                .metadata
                .get(META_CONTENT_ADVISORY)
                .and_then(serde_json::Value::as_str),
            Some("PG-13")
        );
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
        assert_eq!(ItunesStoreProvider::new("US").id(), "itunes_store");
    }

    #[test]
    fn itunes_store_capabilities_video_type() {
        assert!(ItunesStoreProvider::default().capabilities().video_search);
    }

    #[test]
    fn itunes_store_parse_valid_json() {
        let json =
            r#"{"results": [{"trackId": 9999, "trackName": "Breaking Bad", "artistName": "AMC"}]}"#;
        let results = ItunesStoreProvider::parse("itunes_store", json).unwrap();
        assert_eq!(results[0].title.as_deref(), Some("Breaking Bad"));
    }
}
