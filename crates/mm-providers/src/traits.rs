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
// String, serde_json::Value>` blob using the constants in `extra_keys`
// below — keeps the data accessible without forking the upstream type.
//
// #196 CONVERGENCE: as of #196 these 8 keys are unprefixed and match
// MeedyaSuite-core's canonical `extra_keys` names 1:1 (`iswc`, not
// `mm_iswc`, …). The pre-#196 `mm_`-prefixed spellings are accepted on
// READ ONLY, for one release, via the `read_meta()` shim below
// (`LEGACY_META_PREFIX`) — see that section's doc comment for why the
// shim exists despite there being no persisted `mm_`-keyed data.
//
// LOSSY CAPABILITIES: the upstream `ProviderCapabilities` is shaped as
// per-media-type bools (music_search/video_search/podcast_search) rather
// than `Vec<MediaType>`. The local-only fields `supports_isrc`,
// `supports_iswc`, `requires_auth`, and `homepage_url` are not part of
// `ProviderCapabilities` at all — those are recorded in the registry
// separately if needed.

// Re-exports — the new canonical home of these types is upstream.
pub use meedya_core::providers::{
    CoverArtInfo, MediaType, MetadataProvider, ProviderCapabilities, ProviderError, ProviderResult,
    SearchQuery,
};

// #196: `read_meta()` reads the `ProviderResult::metadata` blob, which is
// keyed `String -> serde_json::Value` upstream — pull both types in for
// that helper (and its tests) rather than fully-qualifying every use.
use serde_json::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Extra-key constants for lossy ProviderResult fields
// ---------------------------------------------------------------------------
//
// When a provider has extra fields the upstream `ProviderResult` doesn't
// natively carry, store them under `result.metadata` with these keys.
// Keys are lowercase ASCII to match the upstream metadata HashMap convention.
//
// #196: these 8 values are the canonical, UNPREFIXED MeedyaSuite-core
// `extra_keys` names (verified 1:1 against core branch
// `claude/issue-65-identifier-registry` @ `fd2a7c5`) — before #196 they
// carried a local `mm_` prefix (`mm_iswc`, `mm_provider_id`, …). All
// production writers (§ the whole `mm-providers` crate) already read/write
// these exclusively via the consts below, so changing the values here is
// the entire convergence — no other file needed editing. Legacy `mm_`-
// prefixed reads are accepted for one release via `read_meta()` below;
// see `LEGACY_META_PREFIX`'s doc comment for the removal plan
// (TODO(#196)).

/// Album artist (when different from track artist). Stored as `Value::String`.
pub const META_ALBUM_ARTIST: &str = "album_artist";
/// Total tracks on the album. Stored as `Value::Number(u32)`.
pub const META_TRACK_TOTAL: &str = "track_total";
/// ISWC (composition identifier). Stored as `Value::String`.
pub const META_ISWC: &str = "iswc";
/// EIDR (video identifier). Stored as `Value::String`.
pub const META_EIDR: &str = "eidr";
/// Content advisory label ("explicit", "clean"). Stored as `Value::String`.
pub const META_CONTENT_ADVISORY: &str = "content_advisory";
/// Duration in seconds. Stored as `Value::Number(f64)`.
pub const META_DURATION_SECS: &str = "duration_secs";
/// Beats per minute. Stored as `Value::Number(f64)`.
pub const META_BPM: &str = "bpm";
/// Provider-specific item ID (the old `provider_id` field). Stored as `Value::String`.
pub const META_PROVIDER_ID: &str = "provider_id";

// ---------------------------------------------------------------------------
// Legacy-key read shim (#196) — remove after one release
// ---------------------------------------------------------------------------
//
// Before #196 the 8 META_* keys above carried an `mm_` prefix (`mm_iswc`,
// `mm_provider_id`, …). Provider results are in-memory only — nothing in
// MeedyaManager persists `ProviderResult.metadata` to disk or writes it into
// file tags (verified across all crates for #196) — so no on-disk data holds
// the old keys. This fallback is cheap defence for out-of-tree consumers and
// for any result blob produced by a pre-#196 binary that is still in flight;
// it does NOT protect tagged files (none were ever written with these keys).
//
// TODO(#196): delete the `mm_` fallback (and this comment) in the first
// release after the next shipped version — writes have used the canonical
// unprefixed keys since #196, so nothing will produce the legacy form.

/// Prefix the pre-#196 legacy metadata keys carried.
const LEGACY_META_PREFIX: &str = "mm_";

/// Read a value from a `ProviderResult::metadata` blob under its canonical
/// (unprefixed) key, falling back to the legacy `mm_`-prefixed alias.
///
/// `canonical` should be one of the `META_*` constants above. The canonical
/// key always wins when both spellings are present.
pub fn read_meta<'a>(m: &'a HashMap<String, Value>, canonical: &str) -> Option<&'a Value> {
    // Canonical (post-#196) key first — this is what all MM writers produce.
    m.get(canonical)
        // Legacy pre-#196 alias second (read-only compatibility, one release).
        .or_else(|| m.get(&format!("{LEGACY_META_PREFIX}{canonical}")))
}

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
    r.cover_art
        .iter()
        .max_by_key(|a| u64::from(a.width.unwrap_or(0)) * u64::from(a.height.unwrap_or(0)))
}

/// Returns `true` if the result has at least one cover art entry.
pub fn has_cover_art(r: &ProviderResult) -> bool {
    !r.cover_art.is_empty()
}

// ---------------------------------------------------------------------------
// Tests (#196)
// ---------------------------------------------------------------------------
//
// ELI5: these tests make sure (1) `read_meta()` looks up the new-style key
// first and only falls back to the old `mm_`-prefixed key if the new one is
// missing, and (2) the 8 META_* constants above stay equal to core's
// canonical names forever, instead of quietly drifting back to `mm_...`.
#[cfg(test)]
mod tests {
    use super::*;

    /// #196 (§3.1): when a metadata blob carries BOTH the canonical key and
    /// its legacy `mm_`-prefixed alias, `read_meta()` must prefer the
    /// canonical one. The two keys are seeded with DIFFERENT values
    /// deliberately — if `read_meta()`'s lookup order were ever swapped,
    /// this test would still pass with equal values, so precedence is
    /// only actually exercised when the values differ (kills the
    /// precedence-swap mutant called out in §4 of the #196 build spec).
    #[test]
    fn read_meta_prefers_canonical_key() {
        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert(META_ISWC.to_string(), Value::String("T-1".to_string()));
        m.insert(
            format!("{LEGACY_META_PREFIX}{META_ISWC}"),
            Value::String("T-2".to_string()),
        );
        assert_eq!(
            read_meta(&m, META_ISWC),
            Some(&Value::String("T-1".to_string())),
            "read_meta() must prefer the canonical (unprefixed) key over the legacy mm_-prefixed alias"
        );
    }

    /// #196 (§3.2): when ONLY the legacy `mm_`-prefixed key is present,
    /// `read_meta()` must still find it (the one-release compatibility
    /// shim). And when neither spelling is present, it must return `None`.
    #[test]
    fn read_meta_falls_back_to_legacy_mm_prefixed_key() {
        let mut m: HashMap<String, Value> = HashMap::new();
        m.insert(
            format!("{LEGACY_META_PREFIX}{META_ISWC}"),
            Value::String("T-3".to_string()),
        );
        assert_eq!(
            read_meta(&m, META_ISWC),
            Some(&Value::String("T-3".to_string())),
            "read_meta() must fall back to the legacy mm_-prefixed key when the canonical key is absent"
        );

        let empty: HashMap<String, Value> = HashMap::new();
        assert_eq!(
            read_meta(&empty, META_ISWC),
            None,
            "read_meta() must return None when neither the canonical nor legacy key is present"
        );
    }

    /// #196 guard: MM's 8 `META_*` keys ARE MeedyaSuite-core's canonical
    /// `extra_keys` names — the mechanism (not a comment) preventing the
    /// `mm_iswc`-vs-`iswc` drift from recurring.
    ///
    /// Expected values are INDEPENDENT string literals, deliberately NOT derived
    /// from the constants under test (deriving them would make this a tautology).
    ///
    /// NOTE(#196, dep-gated): once the `meedya-core` git rev in the workspace
    /// `Cargo.toml` is bumped to a rev that ships
    /// `meedya_core::providers::extra_keys` (added upstream after pinned rev
    /// 222ca7590493; present on core branch for MeedyaSuite-core#65), replace the
    /// literal table with direct comparisons against
    /// `meedya_core::providers::extra_keys::{ALBUM_ARTIST, …}` — or replace the
    /// consts themselves with re-exports and reduce this test to the
    /// registry-slug assertions described in issue #196.
    #[test]
    fn meta_keys_are_canonical_core_extra_keys() {
        let expected: [(&str, &str); 8] = [
            (META_ALBUM_ARTIST, "album_artist"),
            (META_TRACK_TOTAL, "track_total"),
            (META_ISWC, "iswc"),
            (META_EIDR, "eidr"),
            (META_CONTENT_ADVISORY, "content_advisory"),
            (META_DURATION_SECS, "duration_secs"),
            (META_BPM, "bpm"),
            (META_PROVIDER_ID, "provider_id"),
        ];
        for (actual, canonical) in expected {
            assert_eq!(
                actual, canonical,
                "META_* key drifted from core's canonical extra_keys name"
            );
            assert!(
                !actual.starts_with("mm_"),
                "META_* key regressed to the legacy mm_ prefix"
            );
        }
    }
}
