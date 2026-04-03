// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Podcast Metadata Provider
//
// Implements the Apple Podcasts provider using the iTunes Search API.
//
// Provider:
//   ApplePodcastsProvider — iTunes Search API (podcast entity); no auth required

use reqwest::Client;
use serde::Deserialize;
use tracing::debug;

use crate::traits::{
    Capabilities, CoverArtInfo, MediaType, MetadataProvider, ProviderError, ProviderResult,
    SearchQuery,
};

// ---------------------------------------------------------------------------
// Apple Podcasts Provider
// ---------------------------------------------------------------------------

/// Searches Apple Podcasts via the iTunes Search API.
///
/// Endpoint: `https://itunes.apple.com/search?media=podcast`
/// Auth:     None (public API)
/// Limits:   20 RPM (conservative)
pub struct ApplePodcastsProvider {
    client: Client,
    base_url: String,
    enabled: bool,
    country: String,
    capabilities: Capabilities,
}

impl ApplePodcastsProvider {
    /// Create a new Apple Podcasts provider for the given country code (ISO 3166-1 alpha-2).
    pub fn new(country: impl Into<String>) -> Self {
        Self::with_base_url(country, "https://itunes.apple.com")
    }

    /// Create a provider with a custom base URL (for test mocking).
    pub fn with_base_url(country: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            enabled: true,
            country: country.into(),
            capabilities: Capabilities {
                media_types: vec![MediaType::Podcast],
                supports_search: true,
                supports_isrc: false,
                supports_iswc: false,
                provides_cover_art: true,
                provides_fingerprint: false,
                requires_auth: false,
                display_name: "Apple Podcasts".into(),
                homepage_url: "https://podcasts.apple.com".into(),
            },
        }
    }

    /// Parse an iTunes podcast search response into `ProviderResult`s.
    pub(crate) fn parse_podcasts(
        provider_name: &str,
        body: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ItunesPodcastResponse {
            results: Vec<ItunesPodcastResult>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ItunesPodcastResult {
            collection_id: Option<u64>,
            collection_name: Option<String>,
            artist_name: Option<String>,
            artwork_url600: Option<String>,
            artwork_url100: Option<String>,
            release_date: Option<String>,
            primary_genre_name: Option<String>,
            track_count: Option<u32>,
            feed_url: Option<String>,
            collection_view_url: Option<String>,
        }

        let resp: ItunesPodcastResponse = serde_json::from_str(body)
            .map_err(|e| ProviderError::Parse(format!("Apple Podcasts response: {e}")))?;

        let results = resp
            .results
            .into_iter()
            .map(|r| {
                // Prefer 600px cover, fall back to 100px
                let cover_art = {
                    let mut arts = Vec::new();
                    if let Some(url) = &r.artwork_url600 {
                        arts.push(CoverArtInfo::new(url, 600, 600, "image/jpeg"));
                    }
                    if let Some(url) = &r.artwork_url100 {
                        arts.push(CoverArtInfo::new(url, 100, 100, "image/jpeg"));
                    }
                    arts
                };

                let year = r
                    .release_date
                    .as_deref()
                    .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());

                let mut extra = std::collections::HashMap::new();
                if let Some(feed) = &r.feed_url {
                    extra.insert("feed_url".into(), feed.clone());
                }
                if let Some(view_url) = &r.collection_view_url {
                    extra.insert("podcast_url".into(), view_url.clone());
                }
                if let Some(count) = r.track_count {
                    extra.insert("episode_count".into(), count.to_string());
                }

                ProviderResult {
                    provider: provider_name.to_owned(),
                    provider_id: r.collection_id.map(|id| id.to_string()).unwrap_or_default(),
                    title: r.collection_name, // Podcast name
                    artist: r.artist_name,    // Podcast author / network
                    genre: r.primary_genre_name,
                    year,
                    cover_art,
                    extra,
                    ..Default::default()
                }
            })
            .collect();

        Ok(results)
    }
}

impl Default for ApplePodcastsProvider {
    fn default() -> Self {
        Self::new("US")
    }
}

impl MetadataProvider for ApplePodcastsProvider {
    fn name(&self) -> &'static str {
        "apple_podcasts"
    }
    fn capabilities(&self) -> &Capabilities {
        &self.capabilities
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn search(
        &self,
        query: SearchQuery,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<ProviderResult>, ProviderError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            if !self.enabled {
                return Err(ProviderError::Disabled("apple_podcasts".into()));
            }

            let term = query
                .title
                .as_deref()
                .or(query.artist.as_deref())
                .unwrap_or(&query.query);

            debug!(
                provider = "apple_podcasts",
                term = term,
                "Sending iTunes podcast search request"
            );

            let url = format!("{}/search", self.base_url);
            let response = self
                .client
                .get(&url)
                .query(&[
                    ("term", term),
                    ("media", "podcast"),
                    ("entity", "podcast"),
                    ("country", &self.country),
                    ("limit", &query.max_results.to_string()),
                ])
                .send()
                .await
                .map_err(|e| ProviderError::Network(e.to_string()))?;

            if !response.status().is_success() {
                return Err(ProviderError::Network(format!(
                    "HTTP {}",
                    response.status()
                )));
            }

            let body = response
                .text()
                .await
                .map_err(|e| ProviderError::Network(e.to_string()))?;
            Self::parse_podcasts("apple_podcasts", &body)
        })
    }
}

// ---------------------------------------------------------------------------
// Tests — 12 tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_podcasts_name() {
        assert_eq!(ApplePodcastsProvider::new("US").name(), "apple_podcasts");
    }

    #[test]
    fn apple_podcasts_enabled_by_default() {
        assert!(ApplePodcastsProvider::default().is_enabled());
    }

    #[test]
    fn apple_podcasts_no_auth_required() {
        assert!(
            !ApplePodcastsProvider::new("US")
                .capabilities()
                .requires_auth
        );
    }

    #[test]
    fn apple_podcasts_media_type_podcast() {
        let p = ApplePodcastsProvider::new("US");
        assert!(p.capabilities().supports_media_type(MediaType::Podcast));
        assert!(!p.capabilities().supports_media_type(MediaType::Music));
    }

    #[test]
    fn apple_podcasts_provides_cover_art() {
        assert!(
            ApplePodcastsProvider::new("US")
                .capabilities()
                .provides_cover_art
        );
    }

    #[test]
    fn apple_podcasts_parse_valid_json() {
        let json = r#"{
            "results": [{
                "collectionId": 12345678,
                "collectionName": "The Daily",
                "artistName": "The New York Times",
                "artworkUrl600": "https://is1.mzstatic.com/600x600.jpg",
                "artworkUrl100": "https://is1.mzstatic.com/100x100.jpg",
                "releaseDate": "2024-01-15T00:00:00Z",
                "primaryGenreName": "News",
                "trackCount": 2500,
                "feedUrl": "https://feeds.nytimes.com/thedaily",
                "collectionViewUrl": "https://podcasts.apple.com/us/podcast/the-daily/id1200361736"
            }]
        }"#;
        let results = ApplePodcastsProvider::parse_podcasts("apple_podcasts", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("The Daily"));
        assert_eq!(results[0].artist.as_deref(), Some("The New York Times"));
        assert_eq!(results[0].year, Some(2024));
        assert_eq!(results[0].genre.as_deref(), Some("News"));
        assert_eq!(results[0].provider_id, "12345678");
        // Both cover art sizes present
        assert_eq!(results[0].cover_art.len(), 2);
        // Extra fields stored
        assert!(results[0].extra.contains_key("feed_url"));
        assert!(results[0].extra.contains_key("episode_count"));
        assert_eq!(
            results[0].extra.get("episode_count").map(String::as_str),
            Some("2500")
        );
    }

    #[test]
    fn apple_podcasts_parse_empty_results() {
        let json = r#"{"results": []}"#;
        let results = ApplePodcastsProvider::parse_podcasts("apple_podcasts", json).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn apple_podcasts_parse_invalid_json_returns_err() {
        let result = ApplePodcastsProvider::parse_podcasts("apple_podcasts", "bad json");
        assert!(matches!(result, Err(ProviderError::Parse(_))));
    }

    #[test]
    fn apple_podcasts_parse_600px_art_preferred() {
        let json = r#"{
            "results": [{
                "artworkUrl600": "https://x.com/big.jpg",
                "artworkUrl100": "https://x.com/small.jpg"
            }]
        }"#;
        let results = ApplePodcastsProvider::parse_podcasts("apple_podcasts", json).unwrap();
        let largest = results[0]
            .cover_art
            .iter()
            .max_by_key(|a| a.pixel_count())
            .unwrap();
        assert_eq!(largest.width, 600);
    }

    #[test]
    fn apple_podcasts_parse_missing_artwork_produces_no_cover_art() {
        let json = r#"{"results": [{"collectionName": "My Podcast"}]}"#;
        let results = ApplePodcastsProvider::parse_podcasts("apple_podcasts", json).unwrap();
        assert!(results[0].cover_art.is_empty());
    }

    #[test]
    fn apple_podcasts_parse_no_feed_url_skips_extra() {
        let json = r#"{"results": [{"collectionName": "Podcast"}]}"#;
        let results = ApplePodcastsProvider::parse_podcasts("apple_podcasts", json).unwrap();
        assert!(!results[0].extra.contains_key("feed_url"));
    }

    #[tokio::test]
    async fn apple_podcasts_search_disabled_returns_err() {
        let mut p = ApplePodcastsProvider::new("US");
        p.enabled = false;
        let q = SearchQuery {
            query: "Test".into(),
            max_results: 5,
            ..Default::default()
        };
        assert!(matches!(
            p.search(q.clone()).await,
            Err(ProviderError::Disabled(_))
        ));
    }
}
