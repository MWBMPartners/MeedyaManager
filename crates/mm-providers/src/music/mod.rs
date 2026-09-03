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

/// Result-count fallback used when a `SearchQuery` leaves `max_results` unset.
///
/// The upstream `SearchQuery.max_results` is `Option<usize>` (the pre-migration
/// local type made it a bare `usize`), so every provider needs *some* default
/// page size; 10 matches what the CLI and registry already request explicitly.
const DEFAULT_MAX_RESULTS: usize = 10;

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
/// Endpoint: `GET {base}/ws/2/recording/?query=<lucene>&limit=<n>&fmt=json`
/// Auth:     None required (but a contact-bearing User-Agent string is required)
/// Limits:   60 RPM (1 request/second, burst 1) — MusicBrainz documents this as
///           a SHARED budget across ALL of musicbrainz.org, not per feature, so
///           this provider, `IsrcProvider`, and `IswcProvider` all draw from one
///           token bucket keyed by host (see `crate::musicbrainz::mb_get()`).
///
/// All MusicBrainz-specific knowledge (endpoint URLs, query params, Lucene
/// escaping, response shapes) lives in the `crate::musicbrainz` module seam —
/// see that module's doc comment for the full rationale (it exists so the
/// announced 2026-11-30 breaking API change is a one-file fix). This struct
/// only orchestrates: build a query, ask `musicbrainz` for a URL/params, hand
/// both to `mb_get()`, then map the body through `musicbrainz::models`.
///
/// A title/artist search that phrase-quotes its terms and comes back with
/// ZERO results is retried exactly once with a loosened, escaped-token query
/// (`crate::musicbrainz::recording_query_loose()`) — see
/// `retry_with_loosened_query()` and the retry-gating comment in `search()`.
pub struct MusicBrainzProvider {
    client: Client,
    base_url: String,
    /// Required by MusicBrainz API: identifies the application making
    /// requests. Sent as the literal `User-Agent` header on every request by
    /// `crate::musicbrainz::mb_get()` rather than relying solely on the
    /// client-level default UA, so it is genuinely read at request time —
    /// hence no `#[allow(dead_code)]` here any more.
    user_agent: String,
}

impl MusicBrainzProvider {
    /// Create a provider with the standard MusicBrainz endpoint.
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self::with_base_url(user_agent, crate::musicbrainz::MB_DEFAULT_BASE_URL)
    }

    /// Create a provider with a custom base URL (useful for test mocking).
    pub fn with_base_url(user_agent: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            // Ensure the User-Agent carries a contact address per
            // MusicBrainz's usage policy (an unreachable UA risks silent
            // deprioritisation or a ban) — `ensure_contact()` appends one
            // only when the UA is non-empty and not already contact-bearing,
            // so the "was a User-Agent supplied at all" signal `configured()`
            // reads below is deliberately left intact for the empty case.
            user_agent: crate::musicbrainz::ensure_contact(user_agent.into()),
        }
    }

    /// True when a User-Agent string is configured. Required by MusicBrainz API.
    ///
    /// Reading the POST-`ensure_contact()` value is safe: that helper
    /// explicitly leaves an empty string unchanged (see its doc comment), so
    /// an unconfigured provider stays unconfigured and a configured one can
    /// never be emptied by the contact-appending step.
    fn configured(&self) -> bool {
        !self.user_agent.is_empty()
    }

    /// Parse a MusicBrainz recording search response into `ProviderResult`s.
    ///
    /// All response-shape knowledge (field names, tolerant score parsing, the
    /// artist/album/year mapping) lives in `crate::musicbrainz::models` — this
    /// function is just "deserialize, then map each recording", so a
    /// 2026-11-30 wire-format change is a one-file fix over there rather than
    /// a hunt through every provider.
    fn parse_recordings(
        provider_name: &str,
        body: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        // Deserialize into the shared response model. `deny_unknown_fields` is
        // deliberately never applied there, so extra MusicBrainz fields we
        // don't map are tolerated rather than failing the whole parse.
        let resp: crate::musicbrainz::models::MbRecordingSearchResponse =
            serde_json::from_str(body).map_err(|e| parse_err("MusicBrainz response", e))?;

        // Map every parsed recording through the shared mapping helper, which
        // also fills the upstream `metadata` blob's `META_*` keys (provider id,
        // duration) that the upstream `ProviderResult` has no fields for.
        let results = resp
            .recordings
            .into_iter()
            .map(|rec| crate::musicbrainz::models::recording_to_result(provider_name, rec))
            .collect();

        Ok(results)
    }

    /// Retry a zero-result phrase-quoted recording search with a loosened,
    /// escaped-token query (`crate::musicbrainz::recording_query_loose()`)
    /// built from the same `title`/`artist`/free-text inputs. See `search()`'s
    /// retry-gating comment for when this is (and isn't) called. Split out as
    /// its own method purely to keep `search()` itself under the project's
    /// function-length/complexity limits.
    async fn retry_with_loosened_query(
        &self,
        query: &SearchQuery,
        free_text: &str,
        url: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        let loose_query = crate::musicbrainz::recording_query_loose(
            query.title.as_deref(),
            query.artist.as_deref(),
            free_text,
        );
        // Offset is hardcoded to 0 — see the equivalent comment in `search()`.
        let loose_params = crate::musicbrainz::search_params(
            &loose_query,
            query.max_results.unwrap_or(DEFAULT_MAX_RESULTS),
            0,
        );

        debug!(
            provider = "musicbrainz",
            query = &loose_query,
            "Phrase-quoted search returned zero results; retrying with a loosened query"
        );

        let body = crate::musicbrainz::mb_get(
            &self.client,
            "musicbrainz",
            &self.user_agent,
            url,
            &loose_params,
        )
        .await?;

        Self::parse_recordings("musicbrainz", &body)
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

        // The upstream `SearchQuery` carries no free-text `query` field, so
        // the free-text fallback term is derived from title + artist by the
        // crate-local `search_term()` helper. In practice that means this
        // value is only ever non-empty when `recording_query()` is already
        // going to use the structured (phrase-quoted) title/artist clauses
        // instead — it exists to keep the shared query builder's three-arm
        // contract intact, not because a free-text-only search can reach it.
        let free_text = search_term(query);

        // Build the Lucene query via the shared `musicbrainz` module: an ISRC
        // takes priority over free-text (an exact identifier match is strictly
        // better than a fuzzy title/artist search), otherwise fall back to a
        // title/artist recording query. `recording_query()` PHRASE-QUOTES
        // title/artist so operator-like characters in real titles (e.g.
        // `AC/DC: Back? [Live] AND More`) are treated as literal text rather
        // than Lucene syntax — this replaces the old `.replace('"', "")`
        // pseudo-escaping that issue #198 flagged as a Lucene-injection
        // defect. See `recording_query()`'s doc comment for the full policy.
        let lucene_query = if let Some(isrc) = &query.isrc {
            crate::musicbrainz::isrc_query(isrc)
        } else {
            crate::musicbrainz::recording_query(
                query.title.as_deref(),
                query.artist.as_deref(),
                &free_text,
            )
        };

        // Endpoint URL and query-string parameters, both built by the shared
        // module so this provider never hand-writes MusicBrainz wire format.
        let url =
            crate::musicbrainz::search_url(&self.base_url, crate::musicbrainz::MbEntity::Recording);
        // Offset is hardcoded to 0: paginated MusicBrainz searches are blocked
        // on the upstream `SearchQuery` gaining an `offset` field (issue #198).
        // `search_params()` omits `offset` from the wire entirely when it is 0.
        let params = crate::musicbrainz::search_params(
            &lucene_query,
            query.max_results.unwrap_or(DEFAULT_MAX_RESULTS),
            0,
        );

        debug!(
            provider = "musicbrainz",
            query = &lucene_query,
            "Sending search request"
        );

        // Execute the rate-limited, contact-header-bearing GET. `mb_get()`
        // owns the shared 1 rps token bucket, the 429/503 `Retry-After`
        // retry-once policy, and non-2xx → `ProviderError::NetworkError`
        // mapping — no inline HTTP handling remains in this provider.
        let body = crate::musicbrainz::mb_get(
            &self.client,
            "musicbrainz",
            &self.user_agent,
            &url,
            &params,
        )
        .await?;

        let results = Self::parse_recordings("musicbrainz", &body)?;

        // FIX (issue #198): a phrase-quoted query is an exact, ordered-token
        // match, so real-world tag decorations a MusicBrainz title lacks —
        // "(Remastered 2011)", "(Live)", "feat. X" — can return zero hits
        // where the pre-hardening loose (buggy, unescaped) query might still
        // have matched something. Retry ONCE with a loosened (escaped-token,
        // not phrase-quoted) query built from the SAME inputs whenever the
        // FIRST query actually used phrase quoting — i.e. `title` and/or
        // `artist` were present. Never retried when neither was present (the
        // free-text path is already escaped, nothing left to loosen) or for an
        // ISRC query (an exact-identifier match has nothing to "loosen"
        // either). This costs one EXTRA rate-limit token, but ONLY in the miss
        // case, and restores the recall the old loose query accidentally
        // provided while keeping the escaping fix intact.
        let used_phrase_query =
            query.isrc.is_none() && (query.title.is_some() || query.artist.is_some());
        if results.is_empty() && used_phrase_query {
            return self
                .retry_with_loosened_query(query, &free_text, &url)
                .await;
        }

        Ok(results)
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
        let secret =
            self.client_secret
                .as_deref()
                .ok_or_else(|| ProviderError::AuthenticationFailed {
                    provider: "spotify".into(),
                    reason: "No client_secret".into(),
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

    #[test]
    fn mb_parse_recordings_float_score_normalises() {
        // MusicBrainz documents `score` as an integer 0-100, but has also
        // been observed sending it as a JSON float — the shared model's
        // `de_score()` tolerates that, and the mapping still normalises to
        // [0.0, 1.0] rather than failing the whole response.
        let json = r#"{"recordings": [{"id": "x", "score": 87.5}]}"#;
        let results = MusicBrainzProvider::parse_recordings("musicbrainz", json).unwrap();
        assert!((results[0].score - 0.875).abs() < 1e-9);
    }

    #[test]
    fn mb_parse_recordings_absent_score_defaults_to_zero() {
        // No `score` key at all (not even `null`) — `de_score()` yields
        // `None`, which `recording_to_result()` maps to a default score of
        // 0.0 rather than failing the parse.
        let json = r#"{"recordings": [{"id": "x"}]}"#;
        let results = MusicBrainzProvider::parse_recordings("musicbrainz", json).unwrap();
        assert_eq!(results[0].score, 0.0);
    }

    #[test]
    fn mb_parse_recordings_missing_recordings_key_is_empty_ok() {
        // MusicBrainz omits the `recordings` key entirely on some zero-result
        // responses rather than sending `[]` — `#[serde(default)]` on the
        // shared model's field must turn this into an empty `Ok(vec![])`, not
        // a parse error. (The pre-#198 hand-rolled struct in this file had a
        // non-defaulted `recordings` field and DID fail here.)
        let json = r"{}";
        let results = MusicBrainzProvider::parse_recordings("musicbrainz", json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn mb_parse_recordings_populates_provider_id_metadata() {
        // The upstream `ProviderResult` has no `provider_id` field, so the
        // MBID lands in the `metadata` blob under `META_PROVIDER_ID` (and, as
        // an MBID, in the first-class `musicbrainz_id` field too).
        let json = r#"{"recordings": [{"id": "abc123", "title": "T"}]}"#;
        let results = MusicBrainzProvider::parse_recordings("musicbrainz", json).unwrap();
        assert_eq!(
            results[0]
                .metadata
                .get(META_PROVIDER_ID)
                .and_then(serde_json::Value::as_str),
            Some("abc123")
        );
        assert_eq!(results[0].musicbrainz_id.as_deref(), Some("abc123"));
    }

    // -------------------------------------------------------------------
    // MusicBrainz — wiremock integration tests (search() end to end)
    // -------------------------------------------------------------------
    //
    // These exercise the full `search()` path — Lucene query building,
    // URL/param construction, and the `mb_get()` executor — against a real
    // (mocked) HTTP server, proving the wire-level behaviour that unit tests
    // on `parse_recordings()` alone can't: what actually gets escaped,
    // headered, and rate-limited on the way out.

    use wiremock::matchers::{header, method, path, query_param, query_param_is_missing};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn mb_search_sends_phrase_quoted_query_on_the_wire() {
        // A title containing Lucene operator-like characters (`/`, `?`, `:`,
        // `[`, `]`, the bare word `AND`) must reach the server phrase-quoted,
        // not merely quote-stripped — proving the `.replace('"', "")`
        // pseudo-escaping issue #198 flagged is truly gone and
        // `recording_query()`'s phrase-quoting is what's actually on the wire.
        let title = "What's / This?: AND [More]";
        let artist = "Test Artist";
        let expected_query = format!(r#"recording:"{title}" AND artistname:"{artist}""#);

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(query_param("query", expected_query.as_str()))
            // A non-empty result here is deliberate: this test's job is to
            // prove the phrase-quoted query hits the wire correctly, not to
            // exercise the zero-result loosened-query retry (see the
            // `mb_search_*retr*` tests for that) — an empty result set here
            // would trigger that retry against a server with no matching mock
            // for the loosened query, and fail for an unrelated reason.
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"recordings":[{"id":"mb-1","title":"What's / This?: AND [More]"}]}"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let provider = MusicBrainzProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            title: Some(title.to_owned()),
            artist: Some(artist.to_owned()),
            max_results: Some(5),
            ..Default::default()
        };
        provider
            .search(&query)
            .await
            .expect("search should succeed against the mock");
    }

    #[tokio::test]
    async fn mb_search_omits_offset_param() {
        // The upstream `SearchQuery` has no `offset` field, so `search()`
        // always passes 0 and `search_params()` omits the parameter entirely
        // (pagination is blocked on that upstream field — issue #198). The
        // nonzero-offset half of this behaviour is covered by
        // `musicbrainz::tests::search_params_includes_offset_when_nonzero`.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(query_param_is_missing("offset"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"recordings":[]}"#))
            .expect(1)
            .mount(&server)
            .await;

        let provider = MusicBrainzProvider::with_base_url("TestApp/1.0", server.uri());
        // Neither title nor artist, so the loosened-query retry never fires
        // and exactly one request reaches the mock.
        let query = SearchQuery {
            max_results: Some(5),
            ..Default::default()
        };
        provider
            .search(&query)
            .await
            .expect("search should succeed without an offset parameter");
    }

    #[tokio::test]
    async fn mb_search_sends_contact_bearing_user_agent() {
        // `with_base_url()` runs the supplied UA through `ensure_contact()`,
        // which appends MusicBrainz's documented "UA ( contact )" segment
        // since "TestApp/1.0" contains neither '@' nor "://".
        //
        // The expected contact segment is derived from
        // `mm_core::useragent::contact_string()` rather than hardcoded, so
        // this assertion stays correct even if `MUSICBRAINZ_CONTACT_EMAIL` is
        // set in the test's environment (e.g. by a CI runner) — the test never
        // mutates env vars itself, it just reads what the runtime resolves.
        let expected_ua = format!("TestApp/1.0 ( {} )", mm_core::useragent::contact_string());
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(header("User-Agent", expected_ua.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"recordings":[]}"#))
            .expect(1)
            .mount(&server)
            .await;

        let provider = MusicBrainzProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            max_results: Some(5),
            ..Default::default()
        };
        provider
            .search(&query)
            .await
            .expect("search should succeed against the mock");
    }

    #[tokio::test]
    async fn mb_search_503_maps_to_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let provider = MusicBrainzProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            max_results: Some(5),
            ..Default::default()
        };
        let err = provider
            .search(&query)
            .await
            .expect_err("a bare 503 (no Retry-After) should surface as RateLimited");
        match err {
            ProviderError::RateLimited(provider) => assert_eq!(provider, "musicbrainz"),
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn mb_search_429_maps_to_rate_limited() {
        // The pre-#198 inline HTTP handling in this provider only special-cased
        // 503; 429 fell through to a generic network error. Routing through
        // `mb_get()` means both throttling statuses now map to `RateLimited`.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let provider = MusicBrainzProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            max_results: Some(5),
            ..Default::default()
        };
        let err = provider
            .search(&query)
            .await
            .expect_err("a bare 429 (no Retry-After) should surface as RateLimited");
        match err {
            ProviderError::RateLimited(provider) => assert_eq!(provider, "musicbrainz"),
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    // -------------------------------------------------------------------
    // MusicBrainz — zero-result loosened-query retry (issue #198)
    // -------------------------------------------------------------------

    #[tokio::test]
    async fn mb_search_zero_results_retries_with_loosened_query() {
        // Real-world tags carry decorations MusicBrainz titles lack (here,
        // "(Remastered 2011)"), so the phrase-quoted query legitimately finds
        // nothing while a loosened, escaped-token query still can.
        let title = "Comfortably Numb (Remastered 2011)";
        let artist = "Pink Floyd";
        let phrase_query = format!(r#"recording:"{title}" AND artistname:"{artist}""#);
        let loose_query = crate::musicbrainz::recording_query_loose(Some(title), Some(artist), "");

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(query_param("query", phrase_query.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"recordings":[]}"#))
            .expect(1)
            .mount(&server)
            .await;
        // The SECOND request's `query` param must be exactly the loosened
        // form — proven by matching on it precisely rather than just "any
        // second request".
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(query_param("query", loose_query.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"recordings":[{"id":"mb-loose","title":"Comfortably Numb"}]}"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let provider = MusicBrainzProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            title: Some(title.to_owned()),
            artist: Some(artist.to_owned()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("a zero-result phrase search should retry and succeed via the loosened query");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Comfortably Numb"));
    }

    #[tokio::test]
    async fn mb_search_nonzero_results_sends_only_one_request() {
        // The mirror image of the retry test: a phrase-quoted search that
        // finds something on the FIRST try must never trigger a retry —
        // proven by `.expect(1)` on a mock with no `query` filter, so a stray
        // second request of ANY shape would violate the expectation.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"recordings":[{"id":"mb-hit","title":"Found On First Try"}]}"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let provider = MusicBrainzProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            title: Some("Some Title".into()),
            artist: Some("Some Artist".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("a non-empty first search must not retry");
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn mb_search_isrc_query_zero_results_does_not_retry() {
        // An ISRC query is an exact-identifier match — there is nothing to
        // "loosen" — so a zero-result ISRC search must NOT retry. Proven via
        // `.expect(1)` on an unfiltered mock.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(query_param("query", "isrc:GBAYE0601498"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"recordings":[]}"#))
            .expect(1)
            .mount(&server)
            .await;

        let provider = MusicBrainzProvider::with_base_url("TestApp/1.0", server.uri());
        // Title and artist are present too, to prove the ISRC branch — not
        // merely "no title/artist" — is what suppresses the retry.
        let query = SearchQuery {
            isrc: Some("GB-AYE-06-01498".into()),
            title: Some("Some Title".into()),
            artist: Some("Some Artist".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("a zero-result ISRC search should succeed without retrying");
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn mb_search_without_title_or_artist_zero_results_does_not_retry() {
        // With neither title nor artist, `recording_query()` takes its
        // free-text branch, which is already `lucene_escape()`d — there is
        // nothing left to "loosen" — so a zero-result search must NOT retry.
        // (Upstream `SearchQuery` has no free-text field, so `search_term()`
        // yields an empty term here; the retry gate is what's under test.)
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"recordings":[]}"#))
            .expect(1)
            .mount(&server)
            .await;

        let provider = MusicBrainzProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("a zero-result free-text search should succeed without retrying");
        assert!(results.is_empty());
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
        let largest = results[0]
            .cover_art
            .iter()
            .max_by_key(|a| u64::from(a.width.unwrap_or(0)) * u64::from(a.height.unwrap_or(0)));
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
