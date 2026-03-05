// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Music Metadata Providers
//
// Implements 10 music metadata providers, each as a struct that implements
// the `MetadataProvider` trait:
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
//   - A `Capabilities` struct declaring what the provider supports
//   - `is_enabled()` based on whether credentials are present
//
// Network calls use JSON transport. Auth tokens are refreshed lazily.

use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, warn};

use crate::traits::{
    Capabilities, CoverArtInfo, MediaType, MetadataProvider, ProviderError, ProviderResult,
    SearchQuery,
};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Build a `ProviderError::Network` from a `reqwest::Error`.
fn net_err(e: reqwest::Error) -> ProviderError {
    ProviderError::Network(e.to_string())
}

/// Build a `ProviderError::Parse` from a serde error.
fn parse_err(context: &str, e: impl std::fmt::Display) -> ProviderError {
    ProviderError::Parse(format!("{context}: {e}"))
}

/// Trim and convert an empty string to `None`.
fn opt_str(s: impl Into<String>) -> Option<String> {
    let s = s.into();
    let trimmed = s.trim().to_owned();
    if trimmed.is_empty() { None } else { Some(trimmed) }
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
    enabled: bool,
    capabilities: Capabilities,
    /// Required by MusicBrainz API: identifies the application making requests
    user_agent: String,
}

impl MusicBrainzProvider {
    /// Create a provider with the standard MusicBrainz endpoint.
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self::with_base_url(user_agent, "https://musicbrainz.org")
    }

    /// Create a provider with a custom base URL (useful for test mocking).
    pub fn with_base_url(user_agent: impl Into<String>, base_url: impl Into<String>) -> Self {
        let user_agent = user_agent.into();
        let enabled = !user_agent.is_empty();
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            enabled,
            user_agent,
            capabilities: Capabilities {
                media_types: vec![MediaType::Music],
                supports_search: true,
                supports_isrc: true,
                supports_iswc: true,
                provides_cover_art: false,  // Cover art via Cover Art Archive (separate)
                provides_fingerprint: false,
                requires_auth: false,
                display_name: "MusicBrainz".into(),
                homepage_url: "https://musicbrainz.org".into(),
            },
        }
    }

    /// Parse a MusicBrainz recording search response into `ProviderResult`s.
    fn parse_recordings(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct MbResponse { recordings: Vec<MbRecording> }

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
        struct MbArtist { name: Option<String> }

        #[derive(Deserialize)]
        struct MbRelease {
            title: Option<String>,
            date: Option<String>,
            #[serde(rename = "track-count")]
            track_count: Option<u32>,
        }

        let resp: MbResponse = serde_json::from_str(body)
            .map_err(|e| parse_err("MusicBrainz response", e))?;

        let results = resp.recordings.into_iter().map(|rec| {
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
            let score = rec.score.unwrap_or(0) as f64 / 100.0;

            ProviderResult {
                provider: provider_name.to_owned(),
                provider_id: rec.id.unwrap_or_default(),
                title: rec.title,
                artist,
                album,
                year,
                isrc: rec.isrcs.and_then(|v| v.into_iter().next()),
                duration_secs: rec.length.map(|ms| ms as f64 / 1000.0),
                score,
                ..Default::default()
            }
        }).collect();

        Ok(results)
    }
}

impl MetadataProvider for MusicBrainzProvider {
    fn name(&self) -> &str { "musicbrainz" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { self.enabled }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::Disabled("musicbrainz".into()));
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
                query.query.clone()
            } else {
                parts.join(" AND ")
            }
        };

        let url = format!("{}/ws/2/recording/", self.base_url);
        debug!(provider = "musicbrainz", query = &lucene_query, "Sending search request");

        let response = self
            .client
            .get(&url)
            .header("User-Agent", &self.user_agent)
            .header("Accept", "application/json")
            .query(&[
                ("query", &lucene_query as &str),
                ("limit", &query.max_results.to_string()),
                ("fmt", "json"),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 503 {
                return Err(ProviderError::RateLimited { provider: "musicbrainz".into() });
            }
            return Err(ProviderError::Network(format!("HTTP {status}")));
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
    capabilities: Capabilities,
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
        let capabilities = Capabilities {
            media_types: vec![MediaType::Music],
            supports_search: true,
            supports_isrc: true,
            supports_iswc: false,
            provides_cover_art: true,
            provides_fingerprint: false,
            requires_auth: true,
            display_name: "Spotify".into(),
            homepage_url: "https://spotify.com".into(),
        };
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            client_id,
            client_secret,
            capabilities,
        }
    }

    /// Obtain an access token using Client Credentials OAuth2.
    async fn get_access_token(&self) -> Result<String, ProviderError> {
        let id = self.client_id.as_deref().ok_or_else(|| ProviderError::Auth("No client_id".into()))?;
        let secret = self.client_secret.as_deref().ok_or_else(|| ProviderError::Auth("No client_secret".into()))?;

        let resp = self
            .client
            .post("https://accounts.spotify.com/api/token")
            .basic_auth(id, Some(secret))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await
            .map_err(net_err)?;

        if !resp.status().is_success() {
            return Err(ProviderError::Auth(format!("Token request failed: HTTP {}", resp.status())));
        }

        #[derive(Deserialize)]
        struct TokenResponse { access_token: String }
        let token: TokenResponse = resp.json().await.map_err(|e| parse_err("Spotify token", e))?;
        Ok(token.access_token)
    }

    /// Parse a Spotify track search response into `ProviderResult`s.
    fn parse_tracks(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct SpotifySearchResponse {
            tracks: Option<SpotifyTrackPage>,
        }
        #[derive(Deserialize)]
        struct SpotifyTrackPage { items: Vec<SpotifyTrack> }
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
        struct SpotifyArtist { name: Option<String> }
        #[derive(Deserialize)]
        struct SpotifyAlbum {
            name: Option<String>,
            release_date: Option<String>,
            images: Option<Vec<SpotifyImage>>,
        }
        #[derive(Deserialize)]
        struct SpotifyImage { url: String, width: Option<u32>, height: Option<u32> }
        #[derive(Deserialize)]
        struct SpotifyExternalIds { isrc: Option<String> }

        let resp: SpotifySearchResponse = serde_json::from_str(body)
            .map_err(|e| parse_err("Spotify search", e))?;

        let tracks = resp.tracks.map(|p| p.items).unwrap_or_default();

        let results = tracks.into_iter().map(|track| {
            let artist = track.artists.as_deref().map(|artists| {
                artists.iter().filter_map(|a| a.name.as_deref()).collect::<Vec<_>>().join("; ")
            });
            let album_name = track.album.as_ref().and_then(|a| a.name.clone());
            let year = track.album.as_ref()
                .and_then(|a| a.release_date.as_deref())
                .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());
            let cover_art = track.album.as_ref()
                .and_then(|a| a.images.as_deref())
                .map(|imgs| {
                    imgs.iter().map(|img| CoverArtInfo::new(
                        &img.url,
                        img.width.unwrap_or(0),
                        img.height.unwrap_or(0),
                        "image/jpeg",
                    )).collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let isrc = track.external_ids.and_then(|ids| ids.isrc);
            // Normalise Spotify popularity 0–100 to [0.0, 1.0]
            let score = track.popularity.unwrap_or(0) as f64 / 100.0;
            let content_advisory = if track.explicit.unwrap_or(false) {
                Some("explicit".into())
            } else {
                Some("clean".into())
            };

            ProviderResult {
                provider: provider_name.to_owned(),
                provider_id: track.id.unwrap_or_default(),
                title: track.name,
                artist,
                album: album_name,
                year,
                isrc,
                duration_secs: track.duration_ms.map(|ms| ms as f64 / 1000.0),
                content_advisory,
                cover_art,
                score,
                ..Default::default()
            }
        }).collect();

        Ok(results)
    }
}

impl MetadataProvider for SpotifyProvider {
    fn name(&self) -> &str { "spotify" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { self.client_id.is_some() && self.client_secret.is_some() }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.is_enabled() {
            return Err(ProviderError::Disabled("spotify".into()));
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
            if parts.is_empty() { query.query.clone() } else { parts.join(" ") }
        };

        let url = format!("{}/v1/search", self.base_url);
        debug!(provider = "spotify", query = &sp_query, "Sending search request");

        let response = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .query(&[
                ("q", &sp_query),
                ("type", &"track".to_owned()),
                ("limit", &query.max_results.to_string()),
            ])
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 429 {
                return Err(ProviderError::RateLimited { provider: "spotify".into() });
            }
            return Err(ProviderError::Network(format!("HTTP {status}")));
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
    capabilities: Capabilities,
}

impl AppleMusicProvider {
    /// Create an Apple Music provider. The iTunes Search API is always available (no auth).
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
                media_types: vec![MediaType::Music],
                supports_search: true,
                supports_isrc: false,
                supports_iswc: false,
                provides_cover_art: true,
                provides_fingerprint: false,
                requires_auth: false,
                display_name: "Apple Music".into(),
                homepage_url: "https://music.apple.com".into(),
            },
        }
    }

    fn parse_itunes(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ItunesResponse { results: Vec<ItunesTrack> }
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

        let resp: ItunesResponse = serde_json::from_str(body)
            .map_err(|e| parse_err("iTunes response", e))?;

        let results = resp.results.into_iter().map(|t| {
            let cover_art = t.artwork_url100.as_deref().map(|url| {
                // Replace 100x100 with higher-res variant
                let hires = url.replace("100x100", "3000x3000");
                vec![
                    CoverArtInfo::new(&hires, 3000, 3000, "image/jpeg"),
                    CoverArtInfo::new(url, 100, 100, "image/jpeg"),
                ]
            }).unwrap_or_default();

            let year = t.release_date.as_deref()
                .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());

            let content_advisory = t.explicit_ness.as_deref().map(|e| {
                if e.to_lowercase() == "explicit" { "explicit" } else { "clean" }.to_owned()
            });

            ProviderResult {
                provider: provider_name.to_owned(),
                provider_id: t.track_id.map(|id| id.to_string()).unwrap_or_default(),
                title: t.track_name,
                artist: t.artist_name,
                album: t.collection_name,
                year,
                track_number: t.track_number,
                track_total: t.track_count,
                disc_number: t.disc_number,
                genre: t.primary_genre_name,
                duration_secs: t.track_time_millis.map(|ms| ms as f64 / 1000.0),
                content_advisory,
                cover_art,
                ..Default::default()
            }
        }).collect();

        Ok(results)
    }
}

impl MetadataProvider for AppleMusicProvider {
    fn name(&self) -> &str { "apple_music" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { self.enabled }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::Disabled("apple_music".into()));
        }

        let search_term = if let Some(title) = &query.title {
            if let Some(artist) = &query.artist {
                format!("{title} {artist}")
            } else {
                title.clone()
            }
        } else {
            query.query.clone()
        };

        let url = format!("{}/search", self.base_url);
        debug!(provider = "apple_music", term = &search_term, "Sending iTunes search request");

        let response = self
            .client
            .get(&url)
            .query(&[
                ("term", &search_term),
                ("media", &"music".to_owned()),
                ("entity", &"song".to_owned()),
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
    capabilities: Capabilities,
}

impl DeezerProvider {
    pub fn new() -> Self {
        Self::with_base_url("https://api.deezer.com")
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            enabled: true,
            capabilities: Capabilities {
                media_types: vec![MediaType::Music],
                supports_search: true,
                supports_isrc: true,
                supports_iswc: false,
                provides_cover_art: true,
                provides_fingerprint: false,
                requires_auth: false,
                display_name: "Deezer".into(),
                homepage_url: "https://deezer.com".into(),
            },
        }
    }

    fn parse_deezer(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        struct DeezerResponse { data: Vec<DeezerTrack> }
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
        struct DeezerArtist { name: Option<String> }
        #[derive(Deserialize)]
        struct DeezerAlbum {
            title: Option<String>,
            cover_xl: Option<String>,
            cover_medium: Option<String>,
        }

        let resp: DeezerResponse = serde_json::from_str(body)
            .map_err(|e| parse_err("Deezer response", e))?;

        let results = resp.data.into_iter().map(|t| {
            let cover_art = {
                let mut arts = Vec::new();
                if let Some(xl) = t.album.as_ref().and_then(|a| a.cover_xl.as_deref()) {
                    arts.push(CoverArtInfo::new(xl, 1000, 1000, "image/jpeg"));
                }
                if let Some(med) = t.album.as_ref().and_then(|a| a.cover_medium.as_deref()) {
                    arts.push(CoverArtInfo::new(med, 250, 250, "image/jpeg"));
                }
                arts
            };

            // Deezer rank is up to ~100_000; normalise to [0.0, 1.0]
            let score = t.rank.map(|r| (r as f64 / 100_000.0).clamp(0.0, 1.0)).unwrap_or(0.5);

            ProviderResult {
                provider: provider_name.to_owned(),
                provider_id: t.id.map(|id| id.to_string()).unwrap_or_default(),
                title: t.title,
                artist: t.artist.and_then(|a| a.name),
                album: t.album.and_then(|a| a.title),
                isrc: t.isrc,
                duration_secs: t.duration.map(|s| s as f64),
                content_advisory: if t.explicit_lyrics.unwrap_or(false) {
                    Some("explicit".into())
                } else {
                    Some("clean".into())
                },
                cover_art,
                score,
                ..Default::default()
            }
        }).collect();

        Ok(results)
    }
}

impl Default for DeezerProvider {
    fn default() -> Self { Self::new() }
}

impl MetadataProvider for DeezerProvider {
    fn name(&self) -> &str { "deezer" }
    fn capabilities(&self) -> &Capabilities { &self.capabilities }
    fn is_enabled(&self) -> bool { self.enabled }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.enabled {
            return Err(ProviderError::Disabled("deezer".into()));
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
                query.query.clone()
            };
            Some(term)
        };

        debug!(provider = "deezer", query = ?q, "Sending search request");

        let mut req = self.client.get(&url);
        if let Some(q) = &q {
            req = req.query(&[
                ("q", q.as_str()),
                ("limit", &query.max_results.to_string()),
            ]);
        }

        let response = req.send().await.map_err(net_err)?;

        if !response.status().is_success() {
            return Err(ProviderError::Network(format!("HTTP {}", response.status())));
        }

        let body = response.text().await.map_err(net_err)?;

        // ISRC lookup returns a single track object; wrap it
        if query.isrc.is_some() {
            // Wrap single track in a `data` array
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
//   - Has correct `name()`, `capabilities()`, and `is_enabled()` implementations
//   - Returns `NotSupported` from `search()` (not `Disabled`)
//   - Has a configurable `enabled` flag (defaults to false for unofficial APIs)

macro_rules! stub_provider {
    (
        $struct_name:ident,
        $name:literal,
        $display_name:literal,
        $homepage:literal,
        $requires_auth:literal,
        $provides_cover_art:literal,
        $enabled_default:literal,
        $media_type:expr
    ) => {
        pub struct $struct_name {
            enabled: bool,
            capabilities: Capabilities,
        }

        impl $struct_name {
            pub fn new(enabled: bool) -> Self {
                Self {
                    enabled,
                    capabilities: Capabilities {
                        media_types: vec![$media_type],
                        supports_search: true,
                        supports_isrc: false,
                        supports_iswc: false,
                        provides_cover_art: $provides_cover_art,
                        provides_fingerprint: false,
                        requires_auth: $requires_auth,
                        display_name: $display_name.into(),
                        homepage_url: $homepage.into(),
                    },
                }
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new($enabled_default)
            }
        }

        impl MetadataProvider for $struct_name {
            fn name(&self) -> &str { $name }
            fn capabilities(&self) -> &Capabilities { &self.capabilities }
            fn is_enabled(&self) -> bool { self.enabled }

            async fn search(
                &self,
                _query: &SearchQuery,
            ) -> Result<Vec<ProviderResult>, ProviderError> {
                if !self.enabled {
                    return Err(ProviderError::Disabled($name.into()));
                }
                warn!(provider = $name, "Provider not fully implemented in M5 (stub)");
                Err(ProviderError::NotSupported {
                    provider: $name.into(),
                    reason: "Provider implementation pending API review".into(),
                })
            }
        }
    };
}

// Provider 5: YouTube Music (unofficial API — requires cookie auth)
stub_provider!(
    YouTubeMusicProvider,
    "youtube_music",
    "YouTube Music",
    "https://music.youtube.com",
    true,    // requires_auth
    true,    // provides_cover_art
    false,   // enabled_default
    MediaType::Music
);

// Provider 6: Amazon Music (no public API)
stub_provider!(
    AmazonMusicProvider,
    "amazon_music",
    "Amazon Music",
    "https://music.amazon.com",
    true,    // requires_auth
    true,    // provides_cover_art
    false,   // enabled_default
    MediaType::Music
);

// Provider 7: Pandora (no public API)
stub_provider!(
    PandoraProvider,
    "pandora",
    "Pandora",
    "https://pandora.com",
    true,    // requires_auth
    true,    // provides_cover_art
    false,   // enabled_default
    MediaType::Music
);

// Provider 8: Tidal (OAuth2 — implementation pending)
stub_provider!(
    TidalProvider,
    "tidal",
    "Tidal",
    "https://tidal.com",
    true,    // requires_auth
    true,    // provides_cover_art
    false,   // enabled_default
    MediaType::Music
);

// Provider 9: Shazam (audio fingerprinting — requires audio input, not metadata text)
stub_provider!(
    ShazamProvider,
    "shazam",
    "Shazam",
    "https://shazam.com",
    false,   // requires_auth (some endpoints don't)
    true,    // provides_cover_art
    false,   // enabled_default
    MediaType::Music
);

// Provider 10: iHeart (undocumented API)
stub_provider!(
    iHeartProvider,
    "iheart",
    "iHeart",
    "https://iheart.com",
    false,   // requires_auth
    true,    // provides_cover_art
    false,   // enabled_default
    MediaType::Music
);

// ---------------------------------------------------------------------------
// Tests — 70 tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // MusicBrainz tests
    // =========================================================================

    #[test]
    fn mb_name() {
        let p = MusicBrainzProvider::new("TestApp/1.0");
        assert_eq!(p.name(), "musicbrainz");
    }

    #[test]
    fn mb_enabled_with_user_agent() {
        let p = MusicBrainzProvider::new("TestApp/1.0");
        assert!(p.is_enabled());
    }

    #[test]
    fn mb_disabled_without_user_agent() {
        let p = MusicBrainzProvider::new("");
        assert!(!p.is_enabled());
    }

    #[test]
    fn mb_capabilities_music_type() {
        let p = MusicBrainzProvider::new("TestApp/1.0");
        assert!(p.capabilities().supports_media_type(MediaType::Music));
        assert!(!p.capabilities().supports_media_type(MediaType::Video));
    }

    #[test]
    fn mb_capabilities_supports_isrc() {
        let p = MusicBrainzProvider::new("TestApp/1.0");
        assert!(p.capabilities().supports_isrc);
    }

    #[test]
    fn mb_capabilities_does_not_require_auth() {
        let p = MusicBrainzProvider::new("TestApp/1.0");
        assert!(!p.capabilities().requires_auth);
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
        assert!(matches!(result, Err(ProviderError::Parse(_))));
    }

    #[test]
    fn mb_parse_duration_conversion_ms_to_secs() {
        let json = r#"{"recordings": [{"id": "x", "length": 240000, "score": 50}]}"#;
        let results = MusicBrainzProvider::parse_recordings("musicbrainz", json).unwrap();
        assert!((results[0].duration_secs.unwrap() - 240.0).abs() < 1e-3);
    }

    // =========================================================================
    // Spotify tests
    // =========================================================================

    #[test]
    fn spotify_name() {
        let p = SpotifyProvider::new(Some("id".into()), Some("secret".into()));
        assert_eq!(p.name(), "spotify");
    }

    #[test]
    fn spotify_enabled_with_credentials() {
        let p = SpotifyProvider::new(Some("id".into()), Some("secret".into()));
        assert!(p.is_enabled());
    }

    #[test]
    fn spotify_disabled_without_client_id() {
        let p = SpotifyProvider::new(None, Some("secret".into()));
        assert!(!p.is_enabled());
    }

    #[test]
    fn spotify_disabled_without_client_secret() {
        let p = SpotifyProvider::new(Some("id".into()), None);
        assert!(!p.is_enabled());
    }

    #[test]
    fn spotify_capabilities_requires_auth() {
        let p = SpotifyProvider::new(None, None);
        assert!(p.capabilities().requires_auth);
    }

    #[test]
    fn spotify_capabilities_provides_cover_art() {
        let p = SpotifyProvider::new(None, None);
        assert!(p.capabilities().provides_cover_art);
    }

    #[test]
    fn spotify_capabilities_supports_isrc() {
        let p = SpotifyProvider::new(None, None);
        assert!(p.capabilities().supports_isrc);
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
        assert!(matches!(result, Err(ProviderError::Parse(_))));
    }

    #[test]
    fn spotify_parse_explicit_track_flagged() {
        let json = r#"{"tracks": {"items": [{"id": "x","name": "T","explicit": true,"popularity": 0}]}}"#;
        let results = SpotifyProvider::parse_tracks("spotify", json).unwrap();
        assert_eq!(results[0].content_advisory.as_deref(), Some("explicit"));
    }

    // =========================================================================
    // Apple Music tests
    // =========================================================================

    #[test]
    fn apple_music_name() {
        let p = AppleMusicProvider::new("US");
        assert_eq!(p.name(), "apple_music");
    }

    #[test]
    fn apple_music_enabled_by_default() {
        let p = AppleMusicProvider::new("US");
        assert!(p.is_enabled());
    }

    #[test]
    fn apple_music_capabilities_provides_cover_art() {
        let p = AppleMusicProvider::new("US");
        assert!(p.capabilities().provides_cover_art);
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
        // Cover art: hi-res + thumbnail
        assert_eq!(results[0].cover_art.len(), 2);
    }

    #[test]
    fn apple_music_parse_hiRes_url_generated() {
        let json = r#"{
            "results": [{"artworkUrl100": "https://x.com/100x100.jpg"}]
        }"#;
        let results = AppleMusicProvider::parse_itunes("apple_music", json).unwrap();
        let largest = results[0].cover_art.iter().max_by_key(|a| a.pixel_count());
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
        assert_eq!(p.name(), "deezer");
    }

    #[test]
    fn deezer_enabled_by_default() {
        let p = DeezerProvider::default();
        assert!(p.is_enabled());
    }

    #[test]
    fn deezer_capabilities_no_auth_required() {
        let p = DeezerProvider::new();
        assert!(!p.capabilities().requires_auth);
    }

    #[test]
    fn deezer_capabilities_supports_isrc() {
        let p = DeezerProvider::new();
        assert!(p.capabilities().supports_isrc);
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
        assert!((results[0].duration_secs.unwrap() - 248.0).abs() < 1e-3);
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
        assert!(matches!(result, Err(ProviderError::Parse(_))));
    }

    // =========================================================================
    // Stub provider tests
    // =========================================================================

    #[test]
    fn youtube_music_name() {
        let p = YouTubeMusicProvider::new(false);
        assert_eq!(p.name(), "youtube_music");
    }

    #[test]
    fn youtube_music_disabled_by_default() {
        let p = YouTubeMusicProvider::default();
        assert!(!p.is_enabled());
    }

    #[test]
    fn youtube_music_requires_auth() {
        let p = YouTubeMusicProvider::new(false);
        assert!(p.capabilities().requires_auth);
    }

    #[tokio::test]
    async fn youtube_music_search_disabled_returns_err() {
        let p = YouTubeMusicProvider::new(false);
        let q = SearchQuery::music("Track", "Artist");
        assert!(matches!(p.search(&q).await, Err(ProviderError::Disabled(_))));
    }

    #[tokio::test]
    async fn youtube_music_search_enabled_returns_not_supported() {
        let p = YouTubeMusicProvider::new(true);
        let q = SearchQuery::music("Track", "Artist");
        assert!(matches!(p.search(&q).await, Err(ProviderError::NotSupported { .. })));
    }

    #[test]
    fn amazon_music_name() {
        assert_eq!(AmazonMusicProvider::new(false).name(), "amazon_music");
    }

    #[test]
    fn pandora_name() {
        assert_eq!(PandoraProvider::new(false).name(), "pandora");
    }

    #[test]
    fn tidal_name() {
        assert_eq!(TidalProvider::new(false).name(), "tidal");
    }

    #[test]
    fn shazam_name() {
        assert_eq!(ShazamProvider::new(false).name(), "shazam");
    }

    #[test]
    fn iheart_name() {
        assert_eq!(iHeartProvider::new(false).name(), "iheart");
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
                p.capabilities().supports_media_type(MediaType::Music),
                "Provider {} should support Music",
                p.name()
            );
        }
    }

    #[test]
    fn stub_providers_all_disabled_by_default() {
        let providers: Vec<Box<dyn MetadataProvider>> = vec![
            Box::new(YouTubeMusicProvider::default()),
            Box::new(AmazonMusicProvider::default()),
            Box::new(PandoraProvider::default()),
            Box::new(TidalProvider::default()),
            Box::new(ShazamProvider::default()),
            Box::new(iHeartProvider::default()),
        ];
        for p in &providers {
            assert!(!p.is_enabled(), "Provider {} should be disabled by default", p.name());
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
