// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Music Metadata Providers
//
// Implements 10 music metadata providers, each as a struct that implements
// the upstream `meedya_providers::MetadataProvider` trait:
//
//   1. MusicBrainzProvider  — Free, open API; no auth required; rate-limited
//   2. SpotifyProvider      — OAuth2 client-credentials flow; rich metadata
//   3. AppleMusicProvider   — JWT-authenticated; iTunes Search API (public) fallback
//   4. DeezerProvider       — Public API; no auth required
//   5. YouTubeMusicProvider — Unofficial; requires cookie/auth
//   6. AmazonMusicProvider  — Unofficial; no public API
//   7. PandoraProvider      — Unofficial; requires auth
//   8. TidalProvider        — OAuth2; HiFi/MQA metadata
//   9. ShazamProvider       — Audio fingerprinting API
//  10. iHeartProvider       — Undocumented radio API
//
// All providers share a common pattern:
//   - A configurable `base_url` (default = production; overridable in tests)
//   - A `reqwest::Client` for HTTP requests
//   - A `ProviderCapabilities` declaring per-media-type support
//
// Network calls use JSON transport. Auth tokens are refreshed lazily.

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, warn};

use crate::traits::{
    CoverArtInfo, META_CONTENT_ADVISORY, META_DURATION_SECS, META_PROVIDER_ID, MetadataProvider,
    ProviderCapabilities, ProviderError, ProviderResult, SearchQuery,
};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Build a `ProviderError::NetworkError` from a `reqwest::Error`.
fn net_err(e: reqwest::Error) -> ProviderError {
    ProviderError::NetworkError(e.to_string())
}

/// Build a parse-style `ProviderError::Other`.
fn parse_err(context: &str, e: impl std::fmt::Display) -> ProviderError {
    ProviderError::Other(format!("parse error: {context}: {e}"))
}

/// Trim and convert an empty string to `None`.
#[allow(dead_code)]
fn opt_str(s: impl Into<String>) -> Option<String> {
    let s = s.into();
    let trimmed = s.trim().to_owned();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Capabilities for a music-only provider.
fn music_caps(cover_art: bool) -> ProviderCapabilities {
    ProviderCapabilities {
        music_search: true,
        video_search: false,
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

/// Resolve a free-text search term from `SearchQuery`. Combines title and artist
/// because the upstream `SearchQuery` has no free-text `query` field.
fn search_term(query: &SearchQuery) -> String {
    let combined = format!(
        "{} {}",
        query.title.as_deref().unwrap_or(""),
        query.artist.as_deref().unwrap_or("")
    );
    combined.trim().to_owned()
}

// ---------------------------------------------------------------------------
// 1. MusicBrainz
// ---------------------------------------------------------------------------

/// Searches the MusicBrainz open database.
///
/// Endpoint: `https://musicbrainz.org/ws/2/recording/`
/// Auth:     None required (but a User-Agent string is required)
/// Limits:   50 RPM (free tier)
pub struct MusicBrainzProvider {
    client: Client,
    base_url: String,
    /// Required by MusicBrainz API: identifies the application making requests
    #[allow(dead_code)]
    user_agent: String,
}

impl MusicBrainzProvider {
    /// Create a provider with the standard MusicBrainz endpoint.
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self::with_base_url(user_agent, "https://musicbrainz.org")
    }

    /// Create a provider with a custom base URL (useful for test mocking).
    pub fn with_base_url(user_agent: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            user_agent: user_agent.into(),
        }
    }

    /// True when a User-Agent string is configured. Required by MusicBrainz API.
    fn configured(&self) -> bool {
        !self.user_agent.is_empty()
    }

    /// Parse a MusicBrainz recording search response into `ProviderResult`s.
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
            artist_credit: Option<Vec<MbArtistCredit>>,
            releases: Option<Vec<MbRelease>>,
            isrcs: Option<Vec<String>>,
            length: Option<u64>,
            score: Option<u32>,
        }

        #[derive(Deserialize)]
        struct MbArtistCredit {
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
            #[serde(rename = "track-count")]
            #[allow(dead_code)]
            track_count: Option<u32>,
        }

        let resp: MbResponse =
            serde_json::from_str(body).map_err(|e| parse_err("MusicBrainz response", e))?;

        let results = resp
            .recordings
            .into_iter()
            .map(|rec| {
                // Combine artist-credit names
                let artist = rec.artist_credit.as_deref().map(|credits| {
                    credits
                        .iter()
                        .filter_map(|c| c.artist.as_ref()?.name.as_deref())
                        .collect::<Vec<_>>()
                        .join("; ")
                });

                // Use the first release for album/year info
                let first_release = rec.releases.as_deref().and_then(|r| r.first());
                let album = first_release.and_then(|r| r.title.clone());
                let year = first_release
                    .and_then(|r| r.date.as_deref())
                    .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());

                // MusicBrainz score is 0–100; normalise to [0.0, 1.0]
                let score = f64::from(rec.score.unwrap_or(0)) / 100.0;

                let mut result = ProviderResult::new(provider_name);
                result.title = rec.title;
                result.artist = artist;
                result.album = album;
                result.year = year;
                result.isrc = rec.isrcs.and_then(|v| v.into_iter().next());
                result.score = score;

                if let Some(id) = rec.id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id));
                }
                if let Some(ms) = rec.length {
                    insert_duration(&mut result, ms as f64 / 1000.0);
                }

                result
            })
            .collect();

        Ok(results)
    }
}

#[async_trait]
impl MetadataProvider for MusicBrainzProvider {
    fn id(&self) -> &str {
        "musicbrainz"
    }

    fn display_name(&self) -> &str {
        "MusicBrainz"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        music_caps(false) // Cover art via Cover Art Archive (separate)
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("musicbrainz".into()));
        }

        // Build query string: ISRC takes priority over free-text
        let lucene_query = if let Some(isrc) = &query.isrc {
            format!("isrc:{isrc}")
        } else {
            let mut parts = Vec::new();
            if let Some(title) = &query.title {
                parts.push(format!("recording:{}", title.replace('"', "")));
            }
            if let Some(artist) = &query.artist {
                parts.push(format!("artistname:{}", artist.replace('"', "")));
            }
            if parts.is_empty() {
                search_term(query)
            } else {
                parts.join(" AND ")
            }
        };

        let url = format!("{}/ws/2/recording/", self.base_url);
        debug!(
            provider = "musicbrainz",
            query = &lucene_query,
            "Sending search request"
        );

        let limit = query.max_results.unwrap_or(10).to_string();
        let response = self
            .client
            .get(&url)
            // User-Agent is set at the client level by crate::http::build_client()
            // so no per-request override is needed.
            .header("Accept", "application/json")
            .query(&[
                ("query", &lucene_query as &str),
                ("limit", &limit),
                ("fmt", "json"),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 503 {
                return Err(ProviderError::RateLimited("musicbrainz".into()));
            }
            return Err(ProviderError::NetworkError(format!("HTTP {status}")));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse_recordings("musicbrainz", &body)
    }
}

// ---------------------------------------------------------------------------
// 2. Spotify
// ---------------------------------------------------------------------------

/// Searches the Spotify Web API using Client Credentials OAuth2.
///
/// Endpoint: `https://api.spotify.com/v1/search`
/// Auth:     OAuth2 client-credentials (`client_id` + `client_secret`)
/// Limits:   100 RPM (standard tier)
pub struct SpotifyProvider {
    client: Client,
    base_url: String,
    client_id: Option<String>,
    client_secret: Option<String>,
}

impl SpotifyProvider {
    /// Create a Spotify provider. `client_id` and `client_secret` are optional;
    /// the provider is disabled if either is `None`.
    pub fn new(client_id: Option<String>, client_secret: Option<String>) -> Self {
        Self::with_base_url(client_id, client_secret, "https://api.spotify.com")
    }

    /// Create a Spotify provider with a custom base URL (for test mocking).
    pub fn with_base_url(
        client_id: Option<String>,
        client_secret: Option<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            client_id,
            client_secret,
        }
    }

    fn configured(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some()
    }

    /// Obtain an access token using Client Credentials OAuth2.
    async fn get_access_token(&self) -> Result<String, ProviderError> {
        let id = self
            .client_id
            .as_deref()
            .ok_or_else(|| ProviderError::AuthenticationFailed {
                provider: "spotify".into(),
                reason: "No client_id".into(),
            })?;
        let secret = self.client_secret.as_deref().ok_or_else(|| {
            ProviderError::AuthenticationFailed {
                provider: "spotify".into(),
                reason: "No client_secret".into(),
            }
        })?;

        let resp = self
            .client
            .post("https://accounts.spotify.com/api/token")
            .basic_auth(id, Some(secret))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await
            .map_err(net_err)?;

        if !resp.status().is_success() {
            return Err(ProviderError::AuthenticationFailed {
                provider: "spotify".into(),
                reason: format!("Token request failed: HTTP {}", resp.status()),
            });
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
        }
        let token: TokenResponse = resp
            .json()
            .await
            .map_err(|e| parse_err("Spotify token", e))?;
        Ok(token.access_token)
    }

    /// Parse a Spotify track search response into `ProviderResult`s.
    fn parse_tracks(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct SpotifySearchResponse {
            tracks: Option<SpotifyTrackPage>,
        }
        #[derive(Deserialize)]
        struct SpotifyTrackPage {
            items: Vec<SpotifyTrack>,
        }
        #[derive(Deserialize)]
        struct SpotifyTrack {
            id: Option<String>,
            name: Option<String>,
            artists: Option<Vec<SpotifyArtist>>,
            album: Option<SpotifyAlbum>,
            duration_ms: Option<u64>,
            explicit: Option<bool>,
            external_ids: Option<SpotifyExternalIds>,
            popularity: Option<u32>,
        }
        #[derive(Deserialize)]
        struct SpotifyArtist {
            name: Option<String>,
        }
        #[derive(Deserialize)]
        struct SpotifyAlbum {
            name: Option<String>,
            release_date: Option<String>,
            images: Option<Vec<SpotifyImage>>,
        }
        #[derive(Deserialize)]
        struct SpotifyImage {
            url: String,
            width: Option<u32>,
            height: Option<u32>,
        }
        #[derive(Deserialize)]
        struct SpotifyExternalIds {
            isrc: Option<String>,
        }

        let resp: SpotifySearchResponse =
            serde_json::from_str(body).map_err(|e| parse_err("Spotify search", e))?;

        let tracks = resp.tracks.map(|p| p.items).unwrap_or_default();

        let results = tracks
            .into_iter()
            .map(|track| {
                let artist = track.artists.as_deref().map(|artists| {
                    artists
                        .iter()
                        .filter_map(|a| a.name.as_deref())
                        .collect::<Vec<_>>()
                        .join("; ")
                });
                let album_name = track.album.as_ref().and_then(|a| a.name.clone());
                let year = track
                    .album
                    .as_ref()
                    .and_then(|a| a.release_date.as_deref())
                    .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());
                let cover_art = track
                    .album
                    .as_ref()
                    .and_then(|a| a.images.as_deref())
                    .map(|imgs| {
                        imgs.iter()
                            .map(|img| CoverArtInfo {
                                url: img.url.clone(),
                                width: img.width,
                                height: img.height,
                                mime_type: Some("image/jpeg".into()),
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let isrc = track.external_ids.and_then(|ids| ids.isrc);
                // Normalise Spotify popularity 0–100 to [0.0, 1.0]
                let score = f64::from(track.popularity.unwrap_or(0)) / 100.0;
                let content_advisory = if track.explicit.unwrap_or(false) {
                    "explicit"
                } else {
                    "clean"
                };

                let mut result = ProviderResult::new(provider_name);
                result.title = track.name;
                result.artist = artist;
                result.album = album_name;
                result.year = year;
                result.isrc = isrc;
                result.score = score;
                result.cover_art = cover_art;
                result.metadata.insert(
                    META_CONTENT_ADVISORY.into(),
                    Value::String(content_advisory.into()),
                );
                if let Some(id) = track.id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id));
                }
                if let Some(ms) = track.duration_ms {
                    insert_duration(&mut result, ms as f64 / 1000.0);
                }

                result
            })
            .collect();

        Ok(results)
    }
}

#[async_trait]
impl MetadataProvider for SpotifyProvider {
    fn id(&self) -> &str {
        "spotify"
    }

    fn display_name(&self) -> &str {
        "Spotify"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        music_caps(true)
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("spotify".into()));
        }

        let token = self.get_access_token().await?;

        // Build Spotify search query
        let sp_query = if let Some(isrc) = &query.isrc {
            format!("isrc:{isrc}")
        } else {
            let mut parts = Vec::new();
            if let Some(title) = &query.title {
                parts.push(format!("track:{title}"));
            }
            if let Some(artist) = &query.artist {
                parts.push(format!("artist:{artist}"));
            }
            if parts.is_empty() {
                search_term(query)
            } else {
                parts.join(" ")
            }
        };

        let url = format!("{}/v1/search", self.base_url);
        debug!(
            provider = "spotify",
            query = &sp_query,
            "Sending search request"
        );

        let limit = query.max_results.unwrap_or(10).to_string();
        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .query(&[
                ("q", &sp_query),
                ("type", &"track".to_owned()),
                ("limit", &limit),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 429 {
                return Err(ProviderError::RateLimited("spotify".into()));
            }
            return Err(ProviderError::NetworkError(format!("HTTP {status}")));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse_tracks("spotify", &body)
    }
}

// ---------------------------------------------------------------------------
// 3. Apple Music (iTunes Search API)
// ---------------------------------------------------------------------------

/// Searches via the iTunes Search API (no auth required for basic track search).
///
/// Endpoint: `https://itunes.apple.com/search`
/// Auth:     None (JWT for full Apple Music API — JWT path stubbed for M5)
/// Limits:   20 RPM (conservative; Apple does not publish limits)
pub struct AppleMusicProvider {
    client: Client,
    base_url: String,
    enabled: bool,
    country: String,
}

impl AppleMusicProvider {
    /// Create an Apple Music provider. The iTunes Search API is always available (no auth).
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

    fn parse_itunes(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ItunesResponse {
            results: Vec<ItunesTrack>,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ItunesTrack {
            track_id: Option<u64>,
            track_name: Option<String>,
            artist_name: Option<String>,
            collection_name: Option<String>,
            artwork_url100: Option<String>,
            release_date: Option<String>,
            track_number: Option<u32>,
            track_count: Option<u32>,
            disc_number: Option<u32>,
            primary_genre_name: Option<String>,
            track_time_millis: Option<u64>,
            explicit_ness: Option<String>,
        }

        let resp: ItunesResponse =
            serde_json::from_str(body).map_err(|e| parse_err("iTunes response", e))?;

        let results = resp
            .results
            .into_iter()
            .map(|t| {
                let cover_art = t
                    .artwork_url100
                    .as_deref()
                    .map(|url| {
                        // Replace 100x100 with higher-res variant
                        let hires = url.replace("100x100", "3000x3000");
                        vec![
                            CoverArtInfo {
                                url: hires,
                                width: Some(3000),
                                height: Some(3000),
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

                let year = t
                    .release_date
                    .as_deref()
                    .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());

                let content_advisory = t.explicit_ness.as_deref().map(|e| {
                    if e.to_lowercase() == "explicit" {
                        "explicit"
                    } else {
                        "clean"
                    }
                    .to_owned()
                });

                let mut result = ProviderResult::new(provider_name);
                result.title = t.track_name;
                result.artist = t.artist_name;
                result.album = t.collection_name;
                result.year = year;
                result.track_number = t.track_number;
                result.disc_number = t.disc_number;
                result.genre = t.primary_genre_name;
                result.cover_art = cover_art;

                if let Some(id) = t.track_id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id.to_string()));
                }
                if let Some(total) = t.track_count {
                    result.metadata.insert(
                        crate::traits::META_TRACK_TOTAL.into(),
                        Value::Number(total.into()),
                    );
                }
                if let Some(ms) = t.track_time_millis {
                    insert_duration(&mut result, ms as f64 / 1000.0);
                }
                if let Some(advisory) = content_advisory {
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

#[async_trait]
impl MetadataProvider for AppleMusicProvider {
    fn id(&self) -> &str {
        "apple_music"
    }

    fn display_name(&self) -> &str {
        "Apple Music"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        music_caps(true)
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotConfigured("apple_music".into()));
        }

        let search_term = if let Some(title) = &query.title {
            if let Some(artist) = &query.artist {
                format!("{title} {artist}")
            } else {
                title.clone()
            }
        } else {
            crate::music::search_term(query)
        };

        let url = format!("{}/search", self.base_url);
        debug!(
            provider = "apple_music",
            term = &search_term,
            "Sending iTunes search request"
        );

        let limit = query.max_results.unwrap_or(10).to_string();
        let response = self
            .client
            .get(&url)
            .query(&[
                ("term", &search_term),
                ("media", &"music".to_owned()),
                ("entity", &"song".to_owned()),
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
        Self::parse_itunes("apple_music", &body)
    }
}

// ---------------------------------------------------------------------------
// 4. Deezer
// ---------------------------------------------------------------------------

/// Searches the Deezer public API (no auth required).
///
/// Endpoint: `https://api.deezer.com/search`
/// Auth:     None
/// Limits:   50 RPM
pub struct DeezerProvider {
    client: Client,
    base_url: String,
    enabled: bool,
}

impl DeezerProvider {
    pub fn new() -> Self {
        Self::with_base_url("https://api.deezer.com")
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            enabled: true,
        }
    }

    fn parse_deezer(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct DeezerResponse {
            data: Vec<DeezerTrack>,
        }
        #[derive(Deserialize)]
        struct DeezerTrack {
            id: Option<u64>,
            title: Option<String>,
            artist: Option<DeezerArtist>,
            album: Option<DeezerAlbum>,
            duration: Option<u64>,
            isrc: Option<String>,
            explicit_lyrics: Option<bool>,
            rank: Option<u64>,
        }
        #[derive(Deserialize)]
        struct DeezerArtist {
            name: Option<String>,
        }
        #[derive(Deserialize)]
        struct DeezerAlbum {
            title: Option<String>,
            cover_xl: Option<String>,
            cover_medium: Option<String>,
        }

        let resp: DeezerResponse =
            serde_json::from_str(body).map_err(|e| parse_err("Deezer response", e))?;

        let results = resp
            .data
            .into_iter()
            .map(|t| {
                let mut cover_art = Vec::new();
                if let Some(xl) = t.album.as_ref().and_then(|a| a.cover_xl.as_deref()) {
                    cover_art.push(CoverArtInfo {
                        url: xl.to_owned(),
                        width: Some(1000),
                        height: Some(1000),
                        mime_type: Some("image/jpeg".into()),
                    });
                }
                if let Some(med) = t.album.as_ref().and_then(|a| a.cover_medium.as_deref()) {
                    cover_art.push(CoverArtInfo {
                        url: med.to_owned(),
                        width: Some(250),
                        height: Some(250),
                        mime_type: Some("image/jpeg".into()),
                    });
                }

                // Deezer rank is up to ~100_000; normalise to [0.0, 1.0]
                let score = t
                    .rank
                    .map_or(0.5, |r| (r as f64 / 100_000.0).clamp(0.0, 1.0));

                let content_advisory = if t.explicit_lyrics.unwrap_or(false) {
                    "explicit"
                } else {
                    "clean"
                };

                let mut result = ProviderResult::new(provider_name);
                result.title = t.title;
                result.artist = t.artist.and_then(|a| a.name);
                result.album = t.album.and_then(|a| a.title);
                result.isrc = t.isrc;
                result.score = score;
                result.cover_art = cover_art;
                result.metadata.insert(
                    META_CONTENT_ADVISORY.into(),
                    Value::String(content_advisory.into()),
                );
                if let Some(id) = t.id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id.to_string()));
                }
                if let Some(secs) = t.duration {
                    insert_duration(&mut result, secs as f64);
                }

                result
            })
            .collect();

        Ok(results)
    }
}

impl Default for DeezerProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MetadataProvider for DeezerProvider {
    fn id(&self) -> &str {
        "deezer"
    }

    fn display_name(&self) -> &str {
        "Deezer"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        music_caps(true)
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::NotConfigured("deezer".into()));
        }

        // Deezer supports ISRC lookup via `/track/isrc:<isrc>`
        let url = if let Some(isrc) = &query.isrc {
            format!("{}/track/isrc:{isrc}", self.base_url)
        } else {
            format!("{}/search", self.base_url)
        };

        let q = if query.isrc.is_some() {
            None
        } else {
            let term = if let (Some(t), Some(a)) = (&query.title, &query.artist) {
                format!("{t} {a}")
            } else {
                search_term(query)
            };
            Some(term)
        };

        debug!(provider = "deezer", query = ?q, "Sending search request");

        let mut req = self.client.get(&url);
        if let Some(q) = &q {
            let limit = query.max_results.unwrap_or(10).to_string();
            req = req.query(&[("q", q.as_str()), ("limit", &limit)]);
        }

        let response = req.send().await.map_err(net_err)?;

        if !response.status().is_success() {
            return Err(ProviderError::NetworkError(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let body = response.text().await.map_err(net_err)?;

        // ISRC lookup returns a single track object; wrap it
        if query.isrc.is_some() {
            let wrapped = format!("{{\"data\":[{body}]}}");
            Self::parse_deezer("deezer", &wrapped)
        } else {
            Self::parse_deezer("deezer", &body)
        }
    }
}

// ---------------------------------------------------------------------------
// 5–10. Stub Providers (unofficial APIs / no public API)
// ---------------------------------------------------------------------------
//
// The following providers are implemented as stubs for M5. Full implementations
// that call real API endpoints will be added when ToS review is complete or
// community-contributed authentication flows are verified.
//
// Each stub:
//   - Has correct `id()`, `display_name()`, and `capabilities()` implementations
//   - Returns `NotSupported` from `search()` when the stub is "enabled"
//   - Returns `NotConfigured` from `search()` when the stub is disabled
//   - Has a configurable `enabled` flag (defaults to false for unofficial APIs)

macro_rules! stub_provider {
    (
        $struct_name:ident,
        $id:literal,
        $display_name:literal,
        $enabled_default:literal,
        $cover_art:literal
    ) => {
        #[allow(non_camel_case_types, non_snake_case)]
        pub struct $struct_name {
            enabled: bool,
        }

        #[allow(non_snake_case)]
        impl $struct_name {
            pub fn new(enabled: bool) -> Self {
                Self { enabled }
            }
        }

        #[allow(non_snake_case)]
        impl Default for $struct_name {
            fn default() -> Self {
                Self::new($enabled_default)
            }
        }

        #[async_trait::async_trait]
        #[allow(non_snake_case)]
        impl MetadataProvider for $struct_name {
            fn id(&self) -> &str {
                $id
            }

            fn display_name(&self) -> &str {
                $display_name
            }

            fn capabilities(&self) -> ProviderCapabilities {
                music_caps($cover_art)
            }

            async fn search(
                &self,
                _query: &SearchQuery,
            ) -> Result<Vec<ProviderResult>, ProviderError> {
                if !self.enabled {
                    return Err(ProviderError::NotConfigured($id.into()));
                }
                warn!(
                    provider = $id,
                    "Provider not fully implemented in M5 (stub)"
                );
                Err(ProviderError::NotSupported(format!(
                    "{}: Provider implementation pending API review",
                    $id
                )))
            }
        }
    };
}

// Provider 5: YouTube Music (unofficial API — requires cookie auth)
stub_provider!(
    YouTubeMusicProvider,
    "youtube_music",
    "YouTube Music",
    false, // enabled_default
    true   // cover_art
);

// Provider 6: Amazon Music (no public API)
stub_provider!(
    AmazonMusicProvider,
    "amazon_music",
    "Amazon Music",
    false,
    true
);

// Provider 7: Pandora (no public API)
stub_provider!(PandoraProvider, "pandora", "Pandora", false, true);

// Provider 8: Tidal (OAuth2 — implementation pending)
stub_provider!(TidalProvider, "tidal", "Tidal", false, true);

// Provider 9: Shazam (audio fingerprinting — requires audio input, not metadata text)
stub_provider!(ShazamProvider, "shazam", "Shazam", false, true);

// Provider 10: iHeart (undocumented API)
stub_provider!(iHeartProvider, "iheart", "iHeart", false, true);

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::META_TRACK_TOTAL;

    // =========================================================================
    // MusicBrainz tests
    // =========================================================================

    #[test]
    fn mb_name() {
        let p = MusicBrainzProvider::new("TestApp/1.0");
        assert_eq!(p.id(), "musicbrainz");
    }

    #[test]
    fn mb_capabilities_music_type() {
        let p = MusicBrainzProvider::new("TestApp/1.0");
        assert!(p.capabilities().music_search);
        assert!(!p.capabilities().video_search);
    }

    #[test]
    fn mb_capabilities_no_cover_art() {
        let p = MusicBrainzProvider::new("TestApp/1.0");
        // MusicBrainz exposes cover art via the Cover Art Archive (a separate provider).
        assert!(!p.capabilities().cover_art);
    }

    #[test]
    fn mb_parse_recordings_valid_json() {
        let json = r#"{
            "recordings": [{
                "id": "abc123",
                "title": "Comfortably Numb",
                "artist-credit": [{"artist": {"name": "Pink Floyd"}}],
                "releases": [{"title": "The Wall", "date": "1979-11-30"}],
                "isrcs": ["GBAYE7900498"],
                "length": 382000,
                "score": 100
            }]
        }"#;
        let results = MusicBrainzProvider::parse_recordings("musicbrainz", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Comfortably Numb"));
        assert_eq!(results[0].artist.as_deref(), Some("Pink Floyd"));
        assert_eq!(results[0].album.as_deref(), Some("The Wall"));
        assert_eq!(results[0].year, Some(1979));
        assert_eq!(results[0].isrc.as_deref(), Some("GBAYE7900498"));
        assert!((results[0].score - 1.0).abs() < 1e-9);
    }

    #[test]
    fn mb_parse_recordings_empty_list() {
        let json = r#"{"recordings": []}"#;
        let results = MusicBrainzProvider::parse_recordings("musicbrainz", json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn mb_parse_recordings_invalid_json_returns_err() {
        let result = MusicBrainzProvider::parse_recordings("musicbrainz", "not json");
        assert!(matches!(result, Err(ProviderError::Other(_))));
    }

    #[test]
    fn mb_parse_duration_conversion_ms_to_secs() {
        let json = r#"{"recordings": [{"id": "x", "length": 240000, "score": 50}]}"#;
        let results = MusicBrainzProvider::parse_recordings("musicbrainz", json).unwrap();
        let duration = results[0]
            .metadata
            .get(META_DURATION_SECS)
            .and_then(serde_json::Value::as_f64)
            .unwrap();
        assert!((duration - 240.0).abs() < 1e-3);
    }

    // =========================================================================
    // Spotify tests
    // =========================================================================

    #[test]
    fn spotify_name() {
        let p = SpotifyProvider::new(Some("id".into()), Some("secret".into()));
        assert_eq!(p.id(), "spotify");
    }

    #[test]
    fn spotify_capabilities_provides_cover_art() {
        let p = SpotifyProvider::new(None, None);
        assert!(p.capabilities().cover_art);
    }

    #[test]
    fn spotify_capabilities_music_search() {
        let p = SpotifyProvider::new(None, None);
        assert!(p.capabilities().music_search);
    }

    #[test]
    fn spotify_parse_tracks_valid_json() {
        let json = r#"{
            "tracks": {
                "items": [{
                    "id": "sp123",
                    "name": "Bohemian Rhapsody",
                    "artists": [{"name": "Queen"}],
                    "album": {
                        "name": "A Night at the Opera",
                        "release_date": "1975-11-21",
                        "images": [{"url": "https://img.spotify.com/big.jpg", "width": 640, "height": 640}]
                    },
                    "duration_ms": 354000,
                    "explicit": false,
                    "external_ids": {"isrc": "GBUM71505078"},
                    "popularity": 90
                }]
            }
        }"#;
        let results = SpotifyProvider::parse_tracks("spotify", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Bohemian Rhapsody"));
        assert_eq!(results[0].artist.as_deref(), Some("Queen"));
        assert_eq!(results[0].album.as_deref(), Some("A Night at the Opera"));
        assert_eq!(results[0].year, Some(1975));
        assert_eq!(results[0].isrc.as_deref(), Some("GBUM71505078"));
        assert!((results[0].score - 0.9).abs() < 1e-9);
        assert!(!results[0].cover_art.is_empty());
    }

    #[test]
    fn spotify_parse_tracks_empty() {
        let json = r#"{"tracks": {"items": []}}"#;
        let results = SpotifyProvider::parse_tracks("spotify", json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn spotify_parse_tracks_invalid_json() {
        let result = SpotifyProvider::parse_tracks("spotify", "bad json");
        assert!(matches!(result, Err(ProviderError::Other(_))));
    }

    #[test]
    fn spotify_parse_explicit_track_flagged() {
        let json =
            r#"{"tracks": {"items": [{"id": "x","name": "T","explicit": true,"popularity": 0}]}}"#;
        let results = SpotifyProvider::parse_tracks("spotify", json).unwrap();
        assert_eq!(
            results[0]
                .metadata
                .get(META_CONTENT_ADVISORY)
                .and_then(serde_json::Value::as_str),
            Some("explicit")
        );
    }

    // =========================================================================
    // Apple Music tests
    // =========================================================================

    #[test]
    fn apple_music_name() {
        let p = AppleMusicProvider::new("US");
        assert_eq!(p.id(), "apple_music");
    }

    #[test]
    fn apple_music_capabilities_provides_cover_art() {
        let p = AppleMusicProvider::new("US");
        assert!(p.capabilities().cover_art);
    }

    #[test]
    fn apple_music_parse_itunes_valid_json() {
        let json = r#"{
            "results": [{
                "trackId": 123456,
                "trackName": "Yesterday",
                "artistName": "The Beatles",
                "collectionName": "Help!",
                "artworkUrl100": "https://is1.mzstatic.com/100x100.jpg",
                "releaseDate": "1965-08-06T00:00:00Z",
                "trackNumber": 10,
                "trackCount": 14,
                "discNumber": 1,
                "primaryGenreName": "Rock",
                "trackTimeMillis": 125000
            }]
        }"#;
        let results = AppleMusicProvider::parse_itunes("apple_music", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Yesterday"));
        assert_eq!(results[0].artist.as_deref(), Some("The Beatles"));
        assert_eq!(results[0].year, Some(1965));
        assert_eq!(results[0].genre.as_deref(), Some("Rock"));
        assert_eq!(results[0].track_number, Some(10));
        // Track total now in metadata
        assert_eq!(
            results[0]
                .metadata
                .get(META_TRACK_TOTAL)
                .and_then(serde_json::Value::as_u64),
            Some(14)
        );
        // Cover art: hi-res + thumbnail
        assert_eq!(results[0].cover_art.len(), 2);
    }

    #[test]
    fn apple_music_parse_hi_res_url_generated() {
        let json = r#"{
            "results": [{"artworkUrl100": "https://x.com/100x100.jpg"}]
        }"#;
        let results = AppleMusicProvider::parse_itunes("apple_music", json).unwrap();
        let largest = results[0].cover_art.iter().max_by_key(|a| {
            u64::from(a.width.unwrap_or(0)) * u64::from(a.height.unwrap_or(0))
        });
        assert!(largest.unwrap().url.contains("3000x3000"));
    }

    #[test]
    fn apple_music_parse_empty_results() {
        let json = r#"{"results": []}"#;
        let results = AppleMusicProvider::parse_itunes("apple_music", json).unwrap();
        assert!(results.is_empty());
    }

    // =========================================================================
    // Deezer tests
    // =========================================================================

    #[test]
    fn deezer_name() {
        let p = DeezerProvider::new();
        assert_eq!(p.id(), "deezer");
    }

    #[test]
    fn deezer_capabilities_music_search() {
        let p = DeezerProvider::new();
        assert!(p.capabilities().music_search);
    }

    #[test]
    fn deezer_parse_valid_json() {
        let json = r#"{
            "data": [{
                "id": 9876,
                "title": "Get Lucky",
                "artist": {"name": "Daft Punk"},
                "album": {
                    "title": "Random Access Memories",
                    "cover_xl": "https://cdn.deezer.com/xl.jpg",
                    "cover_medium": "https://cdn.deezer.com/med.jpg"
                },
                "duration": 248,
                "isrc": "GBUM71300400",
                "explicit_lyrics": false,
                "rank": 850000
            }]
        }"#;
        let results = DeezerProvider::parse_deezer("deezer", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Get Lucky"));
        assert_eq!(results[0].artist.as_deref(), Some("Daft Punk"));
        assert_eq!(results[0].isrc.as_deref(), Some("GBUM71300400"));
        let duration = results[0]
            .metadata
            .get(META_DURATION_SECS)
            .and_then(serde_json::Value::as_f64)
            .unwrap();
        assert!((duration - 248.0).abs() < 1e-3);
        assert_eq!(results[0].cover_art.len(), 2);
    }

    #[test]
    fn deezer_parse_empty_data() {
        let json = r#"{"data": []}"#;
        let results = DeezerProvider::parse_deezer("deezer", json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn deezer_parse_invalid_json_returns_err() {
        let result = DeezerProvider::parse_deezer("deezer", "bad");
        assert!(matches!(result, Err(ProviderError::Other(_))));
    }

    // =========================================================================
    // Stub provider tests
    // =========================================================================

    #[test]
    fn youtube_music_name() {
        let p = YouTubeMusicProvider::new(false);
        assert_eq!(p.id(), "youtube_music");
    }

    #[test]
    fn youtube_music_capabilities() {
        let caps = YouTubeMusicProvider::new(false).capabilities();
        assert!(caps.music_search);
        assert!(caps.cover_art);
    }

    #[tokio::test]
    async fn youtube_music_search_disabled_returns_err() {
        let p = YouTubeMusicProvider::new(false);
        let q = crate::traits::music_query("Track", "Artist");
        assert!(matches!(
            p.search(&q).await,
            Err(ProviderError::NotConfigured(_))
        ));
    }

    #[tokio::test]
    async fn youtube_music_search_enabled_returns_not_supported() {
        let p = YouTubeMusicProvider::new(true);
        let q = crate::traits::music_query("Track", "Artist");
        assert!(matches!(
            p.search(&q).await,
            Err(ProviderError::NotSupported(_))
        ));
    }

    #[test]
    fn amazon_music_name() {
        assert_eq!(AmazonMusicProvider::new(false).id(), "amazon_music");
    }

    #[test]
    fn pandora_name() {
        assert_eq!(PandoraProvider::new(false).id(), "pandora");
    }

    #[test]
    fn tidal_name() {
        assert_eq!(TidalProvider::new(false).id(), "tidal");
    }

    #[test]
    fn shazam_name() {
        assert_eq!(ShazamProvider::new(false).id(), "shazam");
    }

    #[test]
    fn iheart_name() {
        assert_eq!(iHeartProvider::new(false).id(), "iheart");
    }

    #[test]
    fn stub_providers_all_music_type() {
        let providers: Vec<Box<dyn MetadataProvider>> = vec![
            Box::new(YouTubeMusicProvider::default()),
            Box::new(AmazonMusicProvider::default()),
            Box::new(PandoraProvider::default()),
            Box::new(TidalProvider::default()),
            Box::new(ShazamProvider::default()),
            Box::new(iHeartProvider::default()),
        ];
        for p in &providers {
            assert!(
                p.capabilities().music_search,
                "Provider {} should support music_search",
                p.id()
            );
        }
    }

    // --- Shared helper tests ---

    #[test]
    fn opt_str_empty_returns_none() {
        assert!(opt_str("").is_none());
        assert!(opt_str("  ").is_none());
    }

    #[test]
    fn opt_str_non_empty_returns_some() {
        assert_eq!(opt_str("hello"), Some("hello".into()));
    }
}
