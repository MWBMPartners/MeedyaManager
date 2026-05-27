// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Provider Trait Re-exports (#132 migration)
//
// Phase 2 of the MeedyaSuite-core integration epic. The local trait
// definitions previously lived here (599 lines) have been DELETED in
// favour of the upstream `meedya-providers` crate's equivalents,
// re-exported through `meedya-core`'s `providers` module.
//
// This file is now a thin shim so that `use crate::traits::*` continues
// to work across the codebase during migration.
//
// FIELD-LOSS NOTE: the upstream `ProviderResult` lacks 8 fields the local
// version had: `album_artist`, `track_total`, `iswc`, `eidr`,
// `content_advisory`, `duration_secs`, `bpm`, and `provider_id`. Provider
// implementations now stash these in the upstream `metadata: HashMap<
// String, serde_json::Value>` blob using the constants in `extras_keys`
// below — keeps the data accessible without forking the upstream type.
//
// LOSSY CAPABILITIES: the upstream `ProviderCapabilities` is shaped as
// per-media-type bools (music_search/video_search/podcast_search) rather
// than `Vec<MediaType>`. The local-only fields `supports_isrc`,
// `supports_iswc`, `requires_auth`, and `homepage_url` are not part of
// `ProviderCapabilities` at all — those are recorded in the registry
// separately if needed.

// Re-exports — the new canonical home of these types is upstream.
pub use meedya_core::providers::{
    CoverArtInfo, MediaType, MetadataProvider, ProviderCapabilities, ProviderError,
    ProviderResult, SearchQuery,
};

// ---------------------------------------------------------------------------
// Extra-key constants for lossy ProviderResult fields
// ---------------------------------------------------------------------------
//
// When a provider has extra fields the upstream `ProviderResult` doesn't
// natively carry, store them under `result.metadata` with these keys.
// Keys are lowercase ASCII to match the upstream metadata HashMap convention.

/// Album artist (when different from track artist). Stored as `Value::String`.
pub const META_ALBUM_ARTIST: &str = "mm_album_artist";
/// Total tracks on the album. Stored as `Value::Number(u32)`.
pub const META_TRACK_TOTAL: &str = "mm_track_total";
/// ISWC (composition identifier). Stored as `Value::String`.
pub const META_ISWC: &str = "mm_iswc";
/// EIDR (video identifier). Stored as `Value::String`.
pub const META_EIDR: &str = "mm_eidr";
/// Content advisory label ("explicit", "clean"). Stored as `Value::String`.
pub const META_CONTENT_ADVISORY: &str = "mm_content_advisory";
/// Duration in seconds. Stored as `Value::Number(f64)`.
pub const META_DURATION_SECS: &str = "mm_duration_secs";
/// Beats per minute. Stored as `Value::Number(f64)`.
pub const META_BPM: &str = "mm_bpm";
/// Provider-specific item ID (the old `provider_id` field). Stored as `Value::String`.
pub const META_PROVIDER_ID: &str = "mm_provider_id";

// ---------------------------------------------------------------------------
// Local-only helpers for the upstream `ProviderError`
// ---------------------------------------------------------------------------

/// Returns `true` for transient errors that may succeed on retry.
///
/// Replaces the local-only `ProviderError::is_retryable()` method that no
/// longer exists on the upstream type.
pub fn is_retryable(err: &ProviderError) -> bool {
    matches!(
        err,
        ProviderError::NetworkError(_) | ProviderError::RateLimited(_)
    )
}

// ---------------------------------------------------------------------------
// Local-only constructors for `SearchQuery`
// ---------------------------------------------------------------------------
//
// The local `SearchQuery::music()`, `::video()`, `::by_isrc()` helpers were
// removed when we adopted the upstream type. These free functions provide
// equivalent behaviour without monkey-patching the upstream struct.

/// Create a simple title + artist music query.
pub fn music_query(title: impl Into<String>, artist: impl Into<String>) -> SearchQuery {
    let title = title.into();
    let artist = artist.into();
    SearchQuery {
        title: Some(title),
        artist: Some(artist),
        media_type: Some(MediaType::Music),
        max_results: Some(10),
        ..Default::default()
    }
}

/// Create a query by ISRC identifier.
pub fn isrc_query(isrc: impl Into<String>) -> SearchQuery {
    SearchQuery {
        isrc: Some(isrc.into()),
        media_type: Some(MediaType::Identifier),
        max_results: Some(5),
        ..Default::default()
    }
}

/// Create a video / film lookup query.
pub fn video_query(title: impl Into<String>, year: Option<u32>) -> SearchQuery {
    let title = title.into();
    SearchQuery {
        title: Some(title),
        year,
        media_type: Some(MediaType::Video),
        max_results: Some(10),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Local-only helpers for `ProviderResult` cover art
// ---------------------------------------------------------------------------
//
// The local `ProviderResult` previously offered convenience methods for
// inspecting cover art. The upstream `ProviderResult` is a plain struct, so
// we expose equivalent behaviour as free functions.

/// Returns the cover art entry with the most pixels (width × height).
///
/// Entries with missing `width` or `height` are treated as zero pixels.
/// Returns `None` if the result has no cover art at all.
pub fn best_cover_art(r: &ProviderResult) -> Option<&CoverArtInfo> {
    r.cover_art.iter().max_by_key(|a| {
        u64::from(a.width.unwrap_or(0)) * u64::from(a.height.unwrap_or(0))
    })
}

/// Returns `true` if the result has at least one cover art entry.
pub fn has_cover_art(r: &ProviderResult) -> bool {
    !r.cover_art.is_empty()
}
