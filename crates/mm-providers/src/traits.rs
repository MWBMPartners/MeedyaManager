// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Provider Trait Definitions
//
// Defines the core abstractions shared across all 19 metadata providers:
//   - `MetadataProvider`  — async trait every provider must implement
//   - `SearchQuery`       — unified query type accepted by all providers
//   - `ProviderResult`    — standardised result returned by every provider
//   - `Capabilities`      — declares which features a provider supports
//   - `ProviderError`     — typed error enum for provider failures
//   - `CoverArtInfo`      — cover art attachment on a result
//   - `MediaType`         — discriminates music / video / podcast / identifier

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Media type discriminant
// ---------------------------------------------------------------------------

/// Distinguishes the kind of media a provider or query targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    /// Audio music track or album
    Music,
    /// Film, TV episode, or video
    Video,
    /// Podcast episode or series
    Podcast,
    /// Identifier-only lookup (ISRC, EIDR, ISWC)
    Identifier,
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Music => write!(f, "music"),
            Self::Video => write!(f, "video"),
            Self::Podcast => write!(f, "podcast"),
            Self::Identifier => write!(f, "identifier"),
        }
    }
}

// ---------------------------------------------------------------------------
// Provider error type
// ---------------------------------------------------------------------------

/// Errors that can occur during provider operations.
///
/// Callers should match on the variant to determine how to handle the failure.
/// Network failures and rate-limit hits are recoverable; `NotSupported` is permanent.
#[derive(Error, Debug)]
pub enum ProviderError {
    /// HTTP request failed or timed out
    #[error("Network error: {0}")]
    Network(String),

    /// Provider returned an unexpected or unparseable response body
    #[error("Response parse error: {0}")]
    Parse(String),

    /// API credentials are absent or invalid
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Rate limit exceeded — caller should back off
    #[error("Rate limit exceeded for provider '{provider}'")]
    RateLimited { provider: String },

    /// The provider does not support this type of query
    #[error("Not supported by provider '{provider}': {reason}")]
    NotSupported { provider: String, reason: String },

    /// Provider is disabled in the current configuration
    #[error("Provider '{0}' is disabled")]
    Disabled(String),

    /// An unexpected internal error occurred
    #[error("Internal error in provider '{provider}': {message}")]
    Internal { provider: String, message: String },
}

impl ProviderError {
    /// Returns `true` for transient errors that may succeed on retry.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Network(_) | Self::RateLimited { .. })
    }
}

// ---------------------------------------------------------------------------
// Search query
// ---------------------------------------------------------------------------

/// Unified search query accepted by all providers.
///
/// Fields irrelevant to a particular provider are silently ignored during dispatch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Free-text search terms (title, artist, album concatenated)
    pub query: String,

    /// Track / film / episode title
    pub title: Option<String>,

    /// Primary artist or director
    pub artist: Option<String>,

    /// Album or collection name
    pub album: Option<String>,

    /// Album or track release year
    pub year: Option<u32>,

    /// ISRC identifier for the recording (music)
    pub isrc: Option<String>,

    /// ISWC identifier for the composition (music)
    pub iswc: Option<String>,

    /// EIDR identifier for the video title
    pub eidr: Option<String>,

    /// UPC / barcode for the product
    pub upc: Option<String>,

    /// Explicit media type hint (improves routing)
    pub media_type: Option<MediaType>,

    /// Maximum number of results to return
    pub max_results: usize,

    /// Country/locale code for region-specific results (ISO 3166-1 alpha-2)
    pub country: Option<String>,
}

impl SearchQuery {
    /// Create a simple title + artist query.
    pub fn music(title: impl Into<String>, artist: impl Into<String>) -> Self {
        let title = title.into();
        let artist = artist.into();
        Self {
            query: format!("{title} {artist}"),
            title: Some(title),
            artist: Some(artist),
            media_type: Some(MediaType::Music),
            max_results: 10,
            ..Default::default()
        }
    }

    /// Create a query by ISRC identifier.
    pub fn by_isrc(isrc: impl Into<String>) -> Self {
        let isrc = isrc.into();
        Self {
            query: isrc.clone(),
            isrc: Some(isrc),
            media_type: Some(MediaType::Identifier),
            max_results: 5,
            ..Default::default()
        }
    }

    /// Create a video / film lookup query.
    pub fn video(title: impl Into<String>, year: Option<u32>) -> Self {
        let title = title.into();
        Self {
            query: title.clone(),
            title: Some(title),
            year,
            media_type: Some(MediaType::Video),
            max_results: 10,
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Cover art information
// ---------------------------------------------------------------------------

/// A single cover art image URL with its declared dimensions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverArtInfo {
    /// Direct URL to the image (JPEG or PNG)
    pub url: String,

    /// Image width in pixels (0 = unknown)
    pub width: u32,

    /// Image height in pixels (0 = unknown)
    pub height: u32,

    /// MIME type (e.g. "image/jpeg")
    pub mime_type: String,
}

impl CoverArtInfo {
    /// Construct a new cover art entry.
    pub fn new(
        url: impl Into<String>,
        width: u32,
        height: u32,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            width,
            height,
            mime_type: mime_type.into(),
        }
    }

    /// Returns `true` if both dimensions are known (non-zero).
    pub fn has_dimensions(&self) -> bool {
        self.width > 0 && self.height > 0
    }

    /// Returns the number of pixels (0 if dimensions unknown).
    pub fn pixel_count(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

// ---------------------------------------------------------------------------
// Provider capabilities
// ---------------------------------------------------------------------------

/// Declares which features a provider supports.
///
/// Used by the registry to route queries and display capability information in the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Media types this provider can look up
    pub media_types: Vec<MediaType>,

    /// Whether the provider can search by title/artist (free-text)
    pub supports_search: bool,

    /// Whether the provider can look up by ISRC
    pub supports_isrc: bool,

    /// Whether the provider can look up by ISWC
    pub supports_iswc: bool,

    /// Whether the provider returns cover art URLs
    pub provides_cover_art: bool,

    /// Whether the provider returns audio fingerprint data
    pub provides_fingerprint: bool,

    /// Whether the provider requires authentication (API key / OAuth)
    pub requires_auth: bool,

    /// Friendly display name for UI
    pub display_name: String,

    /// Home-page URL for attribution / credits
    pub homepage_url: String,
}

impl Capabilities {
    /// Returns `true` if `media_type` is in the supported list.
    pub fn supports_media_type(&self, media_type: MediaType) -> bool {
        self.media_types.contains(&media_type)
    }
}

// ---------------------------------------------------------------------------
// Provider result
// ---------------------------------------------------------------------------

/// A single metadata result returned by a provider.
///
/// All fields are `Option<T>` because different providers expose different
/// metadata subsets. Callers should merge results from multiple providers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderResult {
    /// Provider that produced this result (e.g. "musicbrainz")
    pub provider: String,

    /// Provider-specific unique identifier for this item
    pub provider_id: String,

    /// Track, film, or episode title
    pub title: Option<String>,

    /// Primary artist(s) — semicolon-separated for multiple artists
    pub artist: Option<String>,

    /// Album or collection name
    pub album: Option<String>,

    /// Album artist (may differ from track artist)
    pub album_artist: Option<String>,

    /// Release year (4-digit)
    pub year: Option<u32>,

    /// Track number within the album
    pub track_number: Option<u32>,

    /// Total number of tracks on the album
    pub track_total: Option<u32>,

    /// Disc number
    pub disc_number: Option<u32>,

    /// Genre name(s)
    pub genre: Option<String>,

    /// ISRC identifier for the recording
    pub isrc: Option<String>,

    /// ISWC identifier for the composition
    pub iswc: Option<String>,

    /// EIDR identifier (video)
    pub eidr: Option<String>,

    /// UPC / barcode
    pub upc: Option<String>,

    /// Content advisory label (e.g. "explicit", "clean")
    pub content_advisory: Option<String>,

    /// Duration in seconds
    pub duration_secs: Option<f64>,

    /// BPM (beats per minute)
    pub bpm: Option<f64>,

    /// Cover art images (may be empty)
    pub cover_art: Vec<CoverArtInfo>,

    /// Provider-specific extended fields not captured above
    pub extra: HashMap<String, String>,

    /// Confidence score [0.0, 1.0] — how well this result matches the query
    pub score: f64,
}

impl ProviderResult {
    /// Returns the best cover art URL (largest image by pixel count).
    pub fn best_cover_art(&self) -> Option<&CoverArtInfo> {
        self.cover_art.iter().max_by_key(|a| a.pixel_count())
    }

    /// Returns `true` if the result has at least one cover art image.
    pub fn has_cover_art(&self) -> bool {
        !self.cover_art.is_empty()
    }
}

// ---------------------------------------------------------------------------
// MetadataProvider async trait
// ---------------------------------------------------------------------------

/// Core trait that every metadata provider must implement.
///
/// # Implementing a new provider
///
/// 1. Create a struct (e.g. `SpotifyProvider`) and implement this trait.
/// 2. In `new()`, read credentials from the `Credentials` helper.
/// 3. In `search()`, call the API, parse JSON, map results to `ProviderResult`.
/// 4. Register the provider in `ProviderRegistry::default()`.
pub trait MetadataProvider: Send + Sync {
    /// Short ASCII identifier for this provider (e.g. "musicbrainz").
    fn name(&self) -> &str;

    /// What this provider can do — used for query routing and UI display.
    fn capabilities(&self) -> &Capabilities;

    /// Whether this provider is currently active (credentials present + enabled).
    fn is_enabled(&self) -> bool;

    /// Perform a metadata search and return ranked results.
    ///
    /// Takes `query` by value to avoid lifetime issues with the boxed future.
    /// `SearchQuery` is `Clone`, so callers can `.clone()` if they need reuse.
    fn search(
        &self,
        query: SearchQuery,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<ProviderResult>, ProviderError>>
                + Send
                + '_,
        >,
    >;
}

// ---------------------------------------------------------------------------
// Tests — 20 tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- MediaType ---

    #[test]
    fn media_type_display_all_variants() {
        assert_eq!(MediaType::Music.to_string(), "music");
        assert_eq!(MediaType::Video.to_string(), "video");
        assert_eq!(MediaType::Podcast.to_string(), "podcast");
        assert_eq!(MediaType::Identifier.to_string(), "identifier");
    }

    #[test]
    fn media_type_equality_and_inequality() {
        assert_eq!(MediaType::Music, MediaType::Music);
        assert_ne!(MediaType::Music, MediaType::Video);
        assert_ne!(MediaType::Podcast, MediaType::Identifier);
    }

    #[test]
    fn media_type_serde_roundtrip() {
        for variant in [
            MediaType::Music,
            MediaType::Video,
            MediaType::Podcast,
            MediaType::Identifier,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: MediaType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    // --- ProviderError ---

    #[test]
    fn provider_error_network_display() {
        let e = ProviderError::Network("connection refused".into());
        assert!(e.to_string().contains("connection refused"));
    }

    #[test]
    fn provider_error_rate_limited_display() {
        let e = ProviderError::RateLimited {
            provider: "spotify".into(),
        };
        let s = e.to_string();
        assert!(s.contains("spotify") && s.contains("Rate limit"));
    }

    #[test]
    fn provider_error_not_supported_display() {
        let e = ProviderError::NotSupported {
            provider: "tmdb".into(),
            reason: "music queries not supported".into(),
        };
        let s = e.to_string();
        assert!(s.contains("tmdb") && s.contains("music queries"));
    }

    #[test]
    fn provider_error_disabled_display() {
        let e = ProviderError::Disabled("acoustid".into());
        assert!(e.to_string().contains("acoustid") && e.to_string().contains("disabled"));
    }

    #[test]
    fn provider_error_is_retryable_network() {
        assert!(ProviderError::Network("x".into()).is_retryable());
    }

    #[test]
    fn provider_error_is_retryable_rate_limited() {
        assert!(
            ProviderError::RateLimited {
                provider: "x".into()
            }
            .is_retryable()
        );
    }

    #[test]
    fn provider_error_not_retryable_variants() {
        assert!(!ProviderError::Disabled("x".into()).is_retryable());
        assert!(!ProviderError::Auth("x".into()).is_retryable());
        assert!(!ProviderError::Parse("x".into()).is_retryable());
        assert!(
            !ProviderError::NotSupported {
                provider: "x".into(),
                reason: "x".into()
            }
            .is_retryable()
        );
    }

    // --- SearchQuery ---

    #[test]
    fn search_query_music_constructor() {
        let q = SearchQuery::music("Comfortably Numb", "Pink Floyd");
        assert_eq!(q.title.as_deref(), Some("Comfortably Numb"));
        assert_eq!(q.artist.as_deref(), Some("Pink Floyd"));
        assert_eq!(q.media_type, Some(MediaType::Music));
        assert_eq!(q.max_results, 10);
        assert!(q.query.contains("Pink Floyd"));
    }

    #[test]
    fn search_query_by_isrc() {
        let q = SearchQuery::by_isrc("GBAYE0601498");
        assert_eq!(q.isrc.as_deref(), Some("GBAYE0601498"));
        assert_eq!(q.media_type, Some(MediaType::Identifier));
        assert_eq!(q.max_results, 5);
    }

    #[test]
    fn search_query_video_constructor() {
        let q = SearchQuery::video("Inception", Some(2010));
        assert_eq!(q.title.as_deref(), Some("Inception"));
        assert_eq!(q.year, Some(2010));
        assert_eq!(q.media_type, Some(MediaType::Video));
    }

    // --- CoverArtInfo ---

    #[test]
    fn cover_art_info_has_dimensions() {
        let art = CoverArtInfo::new("https://example.com/art.jpg", 1000, 1000, "image/jpeg");
        assert!(art.has_dimensions());
        assert_eq!(art.pixel_count(), 1_000_000);
    }

    #[test]
    fn cover_art_info_unknown_dimensions() {
        let art = CoverArtInfo::new("https://example.com/art.jpg", 0, 0, "image/jpeg");
        assert!(!art.has_dimensions());
        assert_eq!(art.pixel_count(), 0);
    }

    // --- Capabilities ---

    #[test]
    fn capabilities_supports_media_type() {
        let caps = Capabilities {
            media_types: vec![MediaType::Music],
            supports_search: true,
            supports_isrc: false,
            supports_iswc: false,
            provides_cover_art: true,
            provides_fingerprint: false,
            requires_auth: false,
            display_name: "Test".into(),
            homepage_url: "https://example.com".into(),
        };
        assert!(caps.supports_media_type(MediaType::Music));
        assert!(!caps.supports_media_type(MediaType::Video));
    }

    // --- ProviderResult ---

    #[test]
    fn provider_result_default_empty() {
        let r = ProviderResult::default();
        assert!(r.title.is_none());
        assert!(r.cover_art.is_empty());
        assert_eq!(r.score, 0.0);
        assert!(!r.has_cover_art());
    }

    #[test]
    fn provider_result_best_cover_art_picks_largest() {
        let mut r = ProviderResult::default();
        r.cover_art.push(CoverArtInfo::new(
            "https://x.com/s.jpg",
            300,
            300,
            "image/jpeg",
        ));
        r.cover_art.push(CoverArtInfo::new(
            "https://x.com/l.jpg",
            1400,
            1400,
            "image/jpeg",
        ));
        let best = r.best_cover_art().unwrap();
        assert_eq!(best.width, 1400);
    }

    #[test]
    fn provider_result_extra_fields() {
        let mut r = ProviderResult::default();
        r.extra.insert("catalog_number".into(), "CAT-001".into());
        assert_eq!(
            r.extra.get("catalog_number").map(String::as_str),
            Some("CAT-001")
        );
    }
}
