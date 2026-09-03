// (C) 2025-2026 MWBM Partners Ltd
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

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, warn};

use crate::traits::{
    META_EIDR, META_ISWC, META_PROVIDER_ID, MetadataProvider, ProviderCapabilities, ProviderError,
    ProviderResult, SearchQuery,
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
    let normalised: String = iswc
        .to_uppercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
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

/// Result-count fallback used when a `SearchQuery` leaves `max_results` unset.
///
/// The upstream `SearchQuery.max_results` is `Option<usize>` (the pre-migration
/// local type made it a bare `usize`), so every provider needs *some* default
/// page size; 10 matches what the CLI and registry already request explicitly.
const DEFAULT_MAX_RESULTS: usize = 10;

/// Structural check for a MusicBrainz Identifier (MBID) shape: 5
/// hyphen-separated groups of hex digits, 8-4-4-4-12 characters long — e.g.
/// `"b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d"`.
///
/// This is NOT a strict RFC 4122 UUID validator (no version/variant nibble
/// checks) — it exists only to catch obviously-not-an-MBID values (most
/// importantly the empty string, which is what `IswcProvider::work_mbid()`
/// returns when a search result carried no `id` at all) BEFORE spending a
/// shared-budget rate-limit token on a lookup-by-id request that is guaranteed
/// to fail. An empty id turns `lookup_url()` into `{base}/ws/2/work/` — the
/// collection/search endpoint, not a single resource — which is a guaranteed
/// 4xx that still costs a request.
///
/// Deliberately hand-rolled rather than pulling in the `uuid` crate: this crate
/// (`mm-providers`) doesn't already depend on it, and full RFC 4122 compliance
/// isn't needed for a pre-flight "is this worth a request at all" check.
fn looks_like_mbid(id: &str) -> bool {
    const GROUP_LENS: [usize; 5] = [8, 4, 4, 4, 12];
    let groups: Vec<&str> = id.split('-').collect();
    groups.len() == GROUP_LENS.len()
        && groups
            .iter()
            .zip(GROUP_LENS)
            .all(|(group, expected_len)| group.len() == expected_len)
        && groups
            .iter()
            .all(|group| group.chars().all(|c| c.is_ascii_hexdigit()))
}

// ---------------------------------------------------------------------------
// 1. ISRC Provider (via MusicBrainz)
// ---------------------------------------------------------------------------

/// Looks up ISRC identifiers, primarily via MusicBrainz's dedicated ISRC
/// lookup endpoint, falling back to a recording search for resilience.
///
/// Endpoint (PRIMARY): `GET {base}/ws/2/isrc/<CODE>?fmt=json&inc=artist-credits+releases`
///   — a direct, cheap, exact-match lookup by ISRC code. `<CODE>` is the
///   normalised (uppercased, hyphen-stripped) ISRC — see `normalise_isrc()`.
///
/// On any lookup failure OTHER than a rate limit (404 "not registered",
/// network error, unparseable body, an endpoint that no longer exists, ...)
/// `search()` falls back to the general recording-search endpoint queried by
/// `isrc:<code>` — the same endpoint/query `MusicBrainzProvider` itself uses
/// for ISRC searches. The SAME fallback also fires when the lookup returns a
/// 2xx with a body that parses cleanly but yields ZERO recordings — because
/// every field of `MbIsrcLookupResponse` is optional, a response whose shape
/// has drifted (e.g. the endpoint's JSON restructured under the announced
/// 2026-11-30 breaking release) parses successfully into an empty result set
/// rather than raising a parse error, so "parsed OK but empty" gets treated
/// as equally suspect as "failed to parse" — neither is trusted as a genuine
/// zero-result answer on its own. That fallback costs one extra request for
/// ISRCs the dedicated endpoint doesn't recognise (or whose response this
/// build no longer understands), but is the resilience path that keeps ISRC
/// lookups working if `/ws/2/isrc/` moves or changes shape. A RATE-LIMITED
/// primary response is returned directly WITHOUT attempting the fallback —
/// see the comment in `search()` for why.
///
/// Auth:   None (but a contact-bearing User-Agent is required)
/// Limits: 60 RPM (1 request/second, burst 1) — a budget SHARED with
///         `MusicBrainzProvider` and `IswcProvider` (all three route through
///         `crate::musicbrainz::mb_get()`'s one token bucket per host).
pub struct IsrcProvider {
    client: Client,
    base_url: String,
    user_agent: String,
}

impl IsrcProvider {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self::with_base_url(user_agent, crate::musicbrainz::MB_DEFAULT_BASE_URL)
    }

    pub fn with_base_url(user_agent: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            // Ensure the User-Agent carries a contact address per
            // MusicBrainz's usage policy — see `ensure_contact()`'s doc
            // comment. An empty `user_agent` (the "provider not configured"
            // signal `configured()` checks for) is left unchanged.
            user_agent: crate::musicbrainz::ensure_contact(user_agent.into()),
        }
    }

    /// True if a User-Agent string is configured. Required by MusicBrainz API.
    fn configured(&self) -> bool {
        !self.user_agent.is_empty()
    }

    /// Parse a MusicBrainz recording-SEARCH response (the fallback path) into
    /// `ProviderResult`s. Name/signature kept unchanged so the existing tests
    /// below keep compiling without modification.
    fn parse_recordings(
        provider_name: &str,
        body: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        // All response-shape knowledge lives in `crate::musicbrainz` — this is
        // just "deserialize the shared model, map each recording".
        let resp: crate::musicbrainz::models::MbRecordingSearchResponse =
            serde_json::from_str(body).map_err(|e| parse_err("ISRC/MusicBrainz response", e))?;

        let results = resp
            .recordings
            .into_iter()
            .map(|rec| crate::musicbrainz::models::recording_to_result(provider_name, rec))
            .collect();
        Ok(results)
    }

    /// Parse a response from the dedicated ISRC LOOKUP endpoint (the primary
    /// path, `GET /ws/2/isrc/<code>`) into `ProviderResult`s.
    ///
    /// This endpoint's embedded recordings omit their own `isrcs` array unless
    /// the caller additionally requests `inc=isrcs` — which `search()`
    /// deliberately doesn't, since every recording this endpoint returns is by
    /// definition registered against the ISRC we just looked up, so echoing it
    /// back via `inc=isrcs` would be a redundant extra field. That means
    /// `recording_to_result()` leaves `result.isrc` as `None` here; this
    /// function fills it back in with the ISRC we actually queried (`isrc`,
    /// already normalised by the caller) whenever that happens.
    fn parse_isrc_lookup(
        provider_name: &str,
        isrc: &str,
        body: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        let resp: crate::musicbrainz::models::MbIsrcLookupResponse =
            serde_json::from_str(body).map_err(|e| parse_err("ISRC lookup response", e))?;

        let results = resp
            .recordings
            .into_iter()
            .map(|rec| {
                let mut result =
                    crate::musicbrainz::models::recording_to_result(provider_name, rec);
                // Backfill the queried ISRC when the recording carried no
                // `isrcs` of its own (see doc comment above for why that's the
                // normal case for this endpoint).
                if result.isrc.is_none() {
                    result.isrc = Some(isrc.to_owned());
                }
                result
            })
            .collect();
        Ok(results)
    }

    /// The FALLBACK path: the general recording-search endpoint, queried by
    /// Lucene `isrc:<code>` — the same endpoint `MusicBrainzProvider` itself
    /// uses for ISRC searches. Split out of `search()` so that function stays
    /// within the project's length/complexity limits.
    ///
    /// This costs one extra request for ISRCs the dedicated lookup endpoint
    /// doesn't recognise (or when that endpoint is unreachable altogether),
    /// but is the resilience path that keeps ISRC lookups working if
    /// `/ws/2/isrc/` moves or changes shape in MusicBrainz's announced
    /// 2026-11-30 breaking release.
    async fn search_recordings_by_isrc(
        &self,
        code: &str,
        limit: usize,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
        let search_url =
            crate::musicbrainz::search_url(&self.base_url, crate::musicbrainz::MbEntity::Recording);
        // Offset is hardcoded to 0: pagination is blocked on the upstream
        // `SearchQuery` gaining an `offset` field (issue #198).
        let search_params =
            crate::musicbrainz::search_params(&crate::musicbrainz::isrc_query(code), limit, 0);

        let body = crate::musicbrainz::mb_get(
            &self.client,
            "isrc",
            &self.user_agent,
            &search_url,
            &search_params,
        )
        .await?;

        Self::parse_recordings("isrc", &body)
    }
}

#[async_trait]
impl MetadataProvider for IsrcProvider {
    fn id(&self) -> &str {
        "isrc"
    }

    fn display_name(&self) -> &str {
        "ISRC (via MusicBrainz)"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            music_search: true,
            video_search: false,
            podcast_search: false,
            cover_art: false,
            lyrics: false,
            fingerprint_lookup: false,
            identifier_lookup: true,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("isrc".into()));
        }

        let isrc = query.isrc.as_deref().ok_or_else(|| {
            ProviderError::NotSupported("isrc: ISRC query requires an ISRC code".into())
        })?;

        if !validate_isrc(isrc) {
            return Err(ProviderError::Other(format!(
                "parse error: Invalid ISRC format: {isrc}"
            )));
        }

        // Canonicalise once up front: used as the lookup URL's path segment,
        // inside the fallback's Lucene `isrc:` query, and to backfill
        // `ProviderResult.isrc` on lookup hits (see `parse_isrc_lookup()`).
        let code = crate::musicbrainz::normalise_isrc(isrc);
        let limit = query.max_results.unwrap_or(DEFAULT_MAX_RESULTS);

        debug!(
            provider = "isrc",
            isrc = %code,
            "Sending ISRC lookup request"
        );

        // PRIMARY PATH: the dedicated /ws/2/isrc/<code> endpoint — one cheap,
        // exact-match request instead of a full Lucene search.
        let lookup_url = crate::musicbrainz::lookup_url(
            &self.base_url,
            crate::musicbrainz::MbEntity::Isrc,
            &code,
        );
        let lookup_params = crate::musicbrainz::lookup_params(Some("artist-credits+releases"));

        match crate::musicbrainz::mb_get(
            &self.client,
            "isrc",
            &self.user_agent,
            &lookup_url,
            &lookup_params,
        )
        .await
        {
            // The HTTP call succeeded — but that alone doesn't mean this is a
            // genuine hit. Every field of `MbIsrcLookupResponse` is optional,
            // so a 200 whose body has drifted out from under us (the endpoint
            // changing shape) parses cleanly into ZERO recordings rather than
            // raising a parse error. Treat that the same as an outright parse
            // failure below: both are "the lookup didn't really answer the
            // question", so both fall through to the search fallback instead
            // of returning a misleadingly-confident `Ok(vec![])`.
            Ok(body) => match Self::parse_isrc_lookup("isrc", &code, &body) {
                Ok(mut results) if !results.is_empty() => {
                    // A genuine hit — done, no fallback needed. The dedicated
                    // lookup endpoint takes no `limit` parameter of its own
                    // (MusicBrainz returns every recording registered against
                    // the ISRC), so truncate here to honour `max_results`
                    // ourselves — `0` means "no limit", matching how
                    // `Registry::search()` treats the same field.
                    if limit > 0 {
                        results.truncate(limit);
                    }
                    return Ok(results);
                }
                Ok(_) => {
                    warn!(
                        provider = "isrc",
                        isrc = %code,
                        "ISRC lookup parsed OK but yielded zero recordings \
                         (possible endpoint shape change); falling back \
                         to recording search"
                    );
                }
                Err(e) => {
                    warn!(
                        provider = "isrc",
                        isrc = %code,
                        error = %e,
                        "ISRC lookup response failed to parse; falling \
                         back to recording search"
                    );
                }
            },
            // The server just told us to back off. Piling a SECOND request
            // onto it via the fallback search below would be exactly the wrong
            // response to a rate limit, so surface this directly instead of
            // trying the fallback path.
            Err(e @ ProviderError::RateLimited(_)) => return Err(e),
            // Any other failure (404 "not registered", a network error, the
            // endpoint having moved, ...) is NOT fatal — fall through to the
            // search fallback.
            Err(e) => {
                warn!(
                    provider = "isrc",
                    isrc = %code,
                    error = %e,
                    "ISRC lookup failed; falling back to recording search"
                );
            }
        }

        // FALLBACK PATH — see `search_recordings_by_isrc()`'s doc comment.
        self.search_recordings_by_isrc(&code, limit).await
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
            client: crate::http::build_client(),
            base_url: base_url.into(),
            username,
            password,
        }
    }

    /// True if both username and password are present.
    fn configured(&self) -> bool {
        self.username.is_some() && self.password.is_some()
    }

    /// Parse an EIDR JSON response into a single `ProviderResult`.
    fn parse_eidr_json(
        provider_name: &str,
        body: &str,
    ) -> Result<Vec<ProviderResult>, ProviderError> {
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
        struct EidrLocalizedName {
            value: Option<String>,
        }

        #[derive(Deserialize)]
        struct EidrExtra {
            movie: Option<EidrMovie>,
        }

        #[derive(Deserialize)]
        struct EidrMovie {
            directors: Option<Vec<String>>,
        }

        let record: EidrRecord =
            serde_json::from_str(body).map_err(|e| parse_err("EIDR response", e))?;

        let year = record
            .release_date
            .as_deref()
            .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());

        let director = record
            .extra
            .as_ref()
            .and_then(|e| e.movie.as_ref())
            .and_then(|m| m.directors.as_deref())
            .and_then(|d| d.first())
            .cloned();

        let mut result = ProviderResult::new(provider_name);
        result.title = record.resource_name.and_then(|n| n.value);
        result.artist = director; // Director for film
        result.year = year;

        if let Some(id) = record.id {
            result
                .metadata
                .insert(META_PROVIDER_ID.into(), Value::String(id.clone()));
            result.metadata.insert(META_EIDR.into(), Value::String(id));
        }

        Ok(vec![result])
    }
}

#[async_trait]
impl MetadataProvider for EidrProvider {
    fn id(&self) -> &str {
        "eidr"
    }

    fn display_name(&self) -> &str {
        "EIDR"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            music_search: false,
            video_search: true,
            podcast_search: false,
            cover_art: false,
            lyrics: false,
            fingerprint_lookup: false,
            identifier_lookup: true,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("eidr".into()));
        }

        let eidr = query.eidr.as_deref().ok_or_else(|| {
            ProviderError::NotSupported("eidr: EIDR query requires an EIDR DOI".into())
        })?;

        if !validate_eidr(eidr) {
            return Err(ProviderError::Other(format!(
                "parse error: Invalid EIDR format: {eidr}"
            )));
        }

        debug!(
            provider = "eidr",
            eidr = eidr,
            "Sending EIDR lookup request"
        );

        let url = format!("{}/EIDR/object/{}", self.base_url, eidr);
        let response = self
            .client
            .get(&url)
            .basic_auth(self.username.as_deref().unwrap(), self.password.as_deref())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(net_err)?;

        if !response.status().is_success() {
            let s = response.status();
            if s.as_u16() == 401 {
                return Err(ProviderError::AuthenticationFailed {
                    provider: "eidr".into(),
                    reason: "Invalid EIDR credentials".into(),
                });
            }
            return Err(ProviderError::NetworkError(format!("HTTP {s}")));
        }

        let body = response.text().await.map_err(net_err)?;
        Self::parse_eidr_json("eidr", &body)
    }
}

// ---------------------------------------------------------------------------
// 3. ISWC Provider (via MusicBrainz Works)
// ---------------------------------------------------------------------------

/// Looks up ISWC identifiers via MusicBrainz's works API, with a follow-up
/// enrichment lookup for composer credit / ISWC data the plain search
/// response doesn't carry.
///
/// Endpoint (SEARCH): `GET {base}/ws/2/work/?query=iswc:<ISWC>&limit=<n>&fmt=json`
///   — the Lucene query is built by `crate::musicbrainz::iswc_query()`, which
///   emits BOTH the punctuation-stripped and the caller's original form
///   (`iswc:T0345246801 OR iswc:"T-034.524.680-1"`) because MusicBrainz
///   canonically DISPLAYS ISWCs punctuated and we cannot verify server-side
///   analyzer behaviour without live network access.
///
/// Endpoint (ENRICHMENT, first result only): `GET {base}/ws/2/work/<mbid>?fmt=json&inc=artist-rels`
///   — a plain work SEARCH response never includes `relations` (composer
///   credits), so `search()` issues ONE additional lookup-by-id requesting
///   `inc=artist-rels` for the first result only, to respect the shared 1 rps
///   budget. Skipped entirely when that first result's MBID doesn't
///   structurally look like one (see `looks_like_mbid()`) — most notably when
///   it's absent, which would otherwise turn the lookup URL into the
///   collection endpoint and waste a rate-limit token on a guaranteed 4xx.
///   Any enrichment failure (network, parse, rate-limit) degrades gracefully
///   to the un-enriched search results — see `enrich_with_work_relations()`.
///
/// Auth:   None (but a contact-bearing User-Agent is required)
/// Limits: 60 RPM (1 request/second, burst 1) — a budget SHARED with
///         `MusicBrainzProvider` and `IsrcProvider` (all three route through
///         `crate::musicbrainz::mb_get()`'s one token bucket per host).
pub struct IswcProvider {
    client: Client,
    base_url: String,
    user_agent: String,
}

impl IswcProvider {
    pub fn new(user_agent: impl Into<String>) -> Self {
        Self::with_base_url(user_agent, crate::musicbrainz::MB_DEFAULT_BASE_URL)
    }

    pub fn with_base_url(user_agent: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: crate::http::build_client(),
            base_url: base_url.into(),
            // Ensure the User-Agent carries a contact address per
            // MusicBrainz's usage policy — see `ensure_contact()`'s doc
            // comment. An empty `user_agent` (the "provider not configured"
            // signal `configured()` checks for) is left unchanged.
            user_agent: crate::musicbrainz::ensure_contact(user_agent.into()),
        }
    }

    fn configured(&self) -> bool {
        !self.user_agent.is_empty()
    }

    /// Read a work's MusicBrainz Identifier back out of the upstream
    /// `ProviderResult`'s `metadata` blob.
    ///
    /// The upstream `ProviderResult` has no `provider_id` field, so
    /// `parse_works()` stashes the work MBID under `META_PROVIDER_ID`; the
    /// enrichment lookup needs it back as a URL path segment. Returns `""`
    /// when the key is absent or not a string — which `looks_like_mbid()`
    /// then rejects, skipping the enrichment request entirely.
    fn work_mbid(result: &ProviderResult) -> &str {
        result
            .metadata
            .get(META_PROVIDER_ID)
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    /// Parse a MusicBrainz work-SEARCH response into `ProviderResult`s. Name
    /// and signature kept unchanged so the existing tests below keep
    /// compiling without modification.
    fn parse_works(provider_name: &str, body: &str) -> Result<Vec<ProviderResult>, ProviderError> {
        // All response-shape knowledge lives in `crate::musicbrainz` — this is
        // just "deserialize the shared model, map each work". A plain search
        // response's `relations` array is normally empty (see the struct-level
        // doc comment above), so `work_composer()` usually yields `None` here
        // — that gets filled in by the enrichment lookup in `search()` when
        // it's worth the extra request.
        let resp: crate::musicbrainz::models::MbWorkSearchResponse =
            serde_json::from_str(body).map_err(|e| parse_err("ISWC/MusicBrainz response", e))?;

        let results = resp
            .works
            .into_iter()
            .map(|work| {
                let composer = crate::musicbrainz::models::work_composer(&work.relations);

                let mut result = ProviderResult::new(provider_name);
                result.title = work.title;
                result.artist = composer;

                // The upstream `ProviderResult` has neither `provider_id` nor
                // `iswc`, so both go in the `metadata` blob under the
                // `META_*` keys from `crate::traits`. The MBID is
                // deliberately NOT also copied into the first-class
                // `musicbrainz_id` field: this is a WORK (composition) MBID,
                // whereas consumers reading `musicbrainz_id` on a music
                // result expect a recording/release MBID, and conflating the
                // two would silently corrupt downstream tag writes.
                if let Some(id) = work.id {
                    result
                        .metadata
                        .insert(META_PROVIDER_ID.into(), Value::String(id));
                }
                if let Some(iswc) = work.iswcs.into_iter().next() {
                    result
                        .metadata
                        .insert(META_ISWC.into(), Value::String(iswc));
                }
                result
            })
            .collect();
        Ok(results)
    }

    /// Enrich `result` (the FIRST search result only — see `search()`'s doc
    /// comment for why) with composer credit and/or ISWC data pulled from a
    /// SINGLE work-relationship lookup-by-id.
    ///
    /// `work_id` is passed in rather than re-read from `result.metadata`
    /// because `result` is borrowed mutably here; the caller already had to
    /// read it to run the `looks_like_mbid()` gate anyway.
    ///
    /// This is a best-effort enhancement, never a source of hard errors: any
    /// failure along the way (the lookup request itself failing — network,
    /// 404, RATE-LIMITED — or the response body failing to parse) is logged
    /// and swallowed, leaving `result` exactly as the search response left it.
    /// A caller who already has *a* result from search should never lose it
    /// just because this optional follow-up request didn't pan out.
    async fn enrich_with_work_relations(
        client: &reqwest::Client,
        user_agent: &str,
        base_url: &str,
        work_id: &str,
        result: &mut ProviderResult,
    ) {
        // Lookup-by-id, requesting only the artist-relations sub-resource
        // (composer credits) — cheaper than requesting everything.
        let lookup_url =
            crate::musicbrainz::lookup_url(base_url, crate::musicbrainz::MbEntity::Work, work_id);
        let lookup_params = crate::musicbrainz::lookup_params(Some("artist-rels"));

        // Network/HTTP failure (including RateLimited — `mb_get()` already
        // applies its own retry-once policy before giving up) — degrade
        // gracefully rather than losing the result we already have.
        let body = match crate::musicbrainz::mb_get(
            client,
            "iswc",
            user_agent,
            &lookup_url,
            &lookup_params,
        )
        .await
        {
            Ok(body) => body,
            Err(e) => {
                debug!(
                    provider = "iswc",
                    work_id = %work_id,
                    error = %e,
                    "ISWC work-relations enrichment lookup failed; returning un-enriched result"
                );
                return;
            }
        };

        // Parse failure — same graceful degradation, but logged at `warn`
        // since a 2xx response with an unparseable body is more surprising
        // than a network hiccup.
        let work: crate::musicbrainz::models::MbWork = match serde_json::from_str(&body) {
            Ok(work) => work,
            Err(e) => {
                warn!(
                    provider = "iswc",
                    work_id = %work_id,
                    error = %e,
                    "ISWC work-relations enrichment response failed to parse; returning un-enriched result"
                );
                return;
            }
        };

        // Merge whatever the enrichment lookup found. Composer is always taken
        // from the enrichment response when present (the search result's own
        // composer was `None`, or `search()` wouldn't have enriched at all);
        // the ISWC is only backfilled if the search result didn't already
        // carry one.
        if let Some(composer) = crate::musicbrainz::models::work_composer(&work.relations) {
            result.artist = Some(composer);
        }
        if !result.metadata.contains_key(META_ISWC)
            && let Some(iswc) = work.iswcs.into_iter().next()
        {
            result
                .metadata
                .insert(META_ISWC.into(), Value::String(iswc));
        }
    }
}

#[async_trait]
impl MetadataProvider for IswcProvider {
    fn id(&self) -> &str {
        "iswc"
    }

    fn display_name(&self) -> &str {
        "ISWC (via MusicBrainz)"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            music_search: true,
            video_search: false,
            podcast_search: false,
            cover_art: false,
            lyrics: false,
            fingerprint_lookup: false,
            identifier_lookup: true,
        }
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<ProviderResult>, ProviderError> {
        if !self.configured() {
            return Err(ProviderError::NotConfigured("iswc".into()));
        }

        let iswc = query.iswc.as_deref().ok_or_else(|| {
            ProviderError::NotSupported("iswc: ISWC query requires an ISWC code".into())
        })?;

        if !validate_iswc(iswc) {
            return Err(ProviderError::Other(format!(
                "parse error: Invalid ISWC format: {iswc}"
            )));
        }

        debug!(
            provider = "iswc",
            iswc = iswc,
            "Sending ISWC lookup request"
        );

        // SEARCH: the works endpoint, queried by Lucene `iswc:<code>` —
        // `iswc_query()` emits both the punctuated and bare forms (see the
        // struct doc comment). Offset is hardcoded to 0: pagination is blocked
        // on the upstream `SearchQuery` gaining an `offset` field (issue #198).
        let search_url =
            crate::musicbrainz::search_url(&self.base_url, crate::musicbrainz::MbEntity::Work);
        let search_params = crate::musicbrainz::search_params(
            &crate::musicbrainz::iswc_query(iswc),
            query.max_results.unwrap_or(DEFAULT_MAX_RESULTS),
            0,
        );
        let body = crate::musicbrainz::mb_get(
            &self.client,
            "iswc",
            &self.user_agent,
            &search_url,
            &search_params,
        )
        .await?;
        let mut results = Self::parse_works("iswc", &body)?;

        // ENRICHMENT: a plain work-search response never includes `relations`,
        // so the first result's composer is normally `None` at this point.
        // Issue ONE additional lookup-by-id to fetch it — enriching ONLY the
        // first result (not every result in the page) to respect the shared
        // 1 rps MusicBrainz budget; a multi-result search that enriched every
        // row would multiply outbound requests by up to `max_results`.
        // Additionally gated on `looks_like_mbid()`: a work with no `id` at
        // all leaves no `META_PROVIDER_ID` in the metadata blob, and an empty
        // id would turn the lookup URL into `{base}/ws/2/work/` — the
        // collection endpoint, not a single resource — a guaranteed 4xx that
        // would still spend a rate-limit token for nothing.
        if let Some(first) = results.first_mut()
            && first.artist.is_none()
        {
            // Copy the id out before taking the mutable borrow below.
            let work_id = Self::work_mbid(first).to_owned();
            if looks_like_mbid(&work_id) {
                Self::enrich_with_work_relations(
                    &self.client,
                    &self.user_agent,
                    &self.base_url,
                    &work_id,
                    first,
                )
                .await;
            } else {
                debug!(
                    provider = "iswc",
                    work_id = %work_id,
                    "Skipping ISWC enrichment lookup: first result has no MBID-shaped id"
                );
            }
        }

        Ok(results)
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

    #[test]
    fn looks_like_mbid_valid_uuid() {
        assert!(looks_like_mbid("b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d"));
    }

    #[test]
    fn looks_like_mbid_empty_string_is_false() {
        // The exact case this guard exists for: `IswcProvider::work_mbid()`
        // returns "" when a search result carried no `id`.
        assert!(!looks_like_mbid(""));
    }

    #[test]
    fn looks_like_mbid_wrong_group_lengths_is_false() {
        assert!(!looks_like_mbid("not-a-real-mbid"));
    }

    #[test]
    fn looks_like_mbid_non_hex_chars_is_false() {
        assert!(!looks_like_mbid("zzzzzzzz-cf9e-42e0-be17-e2c3e1d2600d"));
    }

    // =========================================================================
    // ISRC Provider tests
    // =========================================================================

    #[test]
    fn isrc_provider_name() {
        assert_eq!(IsrcProvider::new("App/1.0").id(), "isrc");
    }

    #[test]
    fn isrc_provider_capabilities() {
        let caps = IsrcProvider::new("App/1.0").capabilities();
        assert!(caps.identifier_lookup);
        assert!(caps.music_search);
        assert!(!caps.cover_art);
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
            Err(ProviderError::Other(_))
        ));
    }

    #[tokio::test]
    async fn isrc_provider_search_without_isrc_returns_not_supported() {
        let p = IsrcProvider::new("App/1.0");
        let q = SearchQuery {
            max_results: Some(5),
            ..Default::default()
        };
        assert!(matches!(
            p.search(&q).await,
            Err(ProviderError::NotSupported(_))
        ));
    }

    #[tokio::test]
    async fn isrc_provider_search_invalid_isrc_returns_parse_err() {
        let p = IsrcProvider::new("App/1.0");
        let q = SearchQuery {
            isrc: Some("BAD".into()),
            max_results: Some(5),
            ..Default::default()
        };
        assert!(matches!(p.search(&q).await, Err(ProviderError::Other(_))));
    }

    #[test]
    fn isrc_provider_parse_isrc_lookup_backfills_queried_isrc() {
        // The dedicated ISRC lookup endpoint's embedded recordings omit their
        // own `isrcs` array unless `inc=isrcs` is requested — which `search()`
        // deliberately doesn't (see `parse_isrc_lookup()`'s doc comment).
        // `parse_isrc_lookup()` must backfill the queried ISRC in that
        // (normal) case.
        let json = r#"{
            "isrc": "GBAYE0601498",
            "recordings": [{
                "id": "mb-rec-2",
                "title": "Some Track"
            }]
        }"#;
        let results = IsrcProvider::parse_isrc_lookup("isrc", "GBAYE0601498", json).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].isrc.as_deref(), Some("GBAYE0601498"));
    }

    #[test]
    fn isrc_provider_parse_isrc_lookup_keeps_recording_own_isrc() {
        // When the recording DOES carry its own `isrcs` (e.g. a future build
        // that requests `inc=isrcs`), the backfill must not clobber it.
        let json = r#"{
            "isrc": "GBAYE0601498",
            "recordings": [{"id": "mb-rec-3", "isrcs": ["USRC17607839"]}]
        }"#;
        let results = IsrcProvider::parse_isrc_lookup("isrc", "GBAYE0601498", json).unwrap();
        assert_eq!(results[0].isrc.as_deref(), Some("USRC17607839"));
    }

    // -------------------------------------------------------------------
    // IsrcProvider — wiremock integration tests (search() end to end)
    // -------------------------------------------------------------------

    use wiremock::matchers::{method, path, query_param, query_param_is_missing};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn isrc_provider_search_lookup_hit_uses_normalised_path_and_inc() {
        // Passing a HYPHENATED ISRC in proves `normalise_isrc()` runs before
        // the lookup URL is built — the mock only matches the canonical,
        // unhyphenated path segment.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/isrc/GBAYE0601498"))
            .and(query_param("inc", "artist-credits+releases"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"isrc":"GBAYE0601498","recordings":[{"id":"mb-1","title":"T"}]}"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let provider = IsrcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            isrc: Some("GB-AYE-06-01498".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("lookup hit should return results");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].isrc.as_deref(), Some("GBAYE0601498"));
    }

    #[tokio::test]
    async fn isrc_provider_search_lookup_404_falls_back_to_search() {
        // Lookup 404s (ISRC not registered under the dedicated endpoint); the
        // fallback recording search then succeeds.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/isrc/GBAYE0601498"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(query_param("query", "isrc:GBAYE0601498"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    r#"{"recordings":[{"id":"mb-2","title":"Fallback Track","isrcs":["GBAYE0601498"]}]}"#,
                ),
            )
            .expect(1)
            .mount(&server)
            .await;

        let provider = IsrcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            isrc: Some("GBAYE0601498".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("fallback search should return results");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Fallback Track"));
    }

    #[tokio::test]
    async fn isrc_provider_search_lookup_and_search_both_404_is_network_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/isrc/GBAYE0601498"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let provider = IsrcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            isrc: Some("GBAYE0601498".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let err = provider
            .search(&query)
            .await
            .expect_err("both lookup and fallback 404ing should surface as NetworkError");
        assert!(matches!(err, ProviderError::NetworkError(_)));
    }

    #[tokio::test]
    async fn isrc_provider_search_lookup_rate_limited_skips_fallback() {
        // A 429 on the PRIMARY lookup must be surfaced directly — the fallback
        // recording-search endpoint must NEVER be hit, proven here via
        // `.expect(0)` on its mock (piling a second request onto an
        // already-throttling server would defeat the point of backing off).
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/isrc/GBAYE0601498"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"recordings":[]}"#))
            .expect(0)
            .mount(&server)
            .await;

        let provider = IsrcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            isrc: Some("GBAYE0601498".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let err = provider
            .search(&query)
            .await
            .expect_err("a rate-limited lookup must not fall back to search");
        match err {
            ProviderError::RateLimited(provider) => assert_eq!(provider, "isrc"),
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn isrc_provider_search_lookup_unrecognised_shape_falls_back_to_search() {
        // A 200 body that parses cleanly (every field of
        // `MbIsrcLookupResponse` is optional) but doesn't carry a `recordings`
        // array we recognise must be treated the same as an outright parse
        // failure — falling through to the search fallback rather than
        // returning a misleadingly-confident `Ok(vec![])`. This is exactly the
        // shape-changed-under-us scenario the 2026-11-30 migration seam exists
        // for.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/isrc/GBAYE0601498"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"unexpected":"shape"}"#))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(query_param("query", "isrc:GBAYE0601498"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    r#"{"recordings":[{"id":"mb-3","title":"Shape Changed Track"}]}"#,
                ),
            )
            .expect(1)
            .mount(&server)
            .await;

        let provider = IsrcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            isrc: Some("GBAYE0601498".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("an unrecognised-but-valid lookup shape should fall back to search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Shape Changed Track"));
    }

    #[tokio::test]
    async fn isrc_provider_search_lookup_non_json_body_falls_back_to_search() {
        // A 200 whose body isn't even valid JSON — `parse_isrc_lookup()`
        // returns `Err(Other)`, which must ALSO fall through to the search
        // fallback rather than being returned directly.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/isrc/GBAYE0601498"))
            .respond_with(ResponseTemplate::new(200).set_body_string("<html>not json</html>"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(query_param("query", "isrc:GBAYE0601498"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(
                    r#"{"recordings":[{"id":"mb-4","title":"Non JSON Fallback"}]}"#,
                ),
            )
            .expect(1)
            .mount(&server)
            .await;

        let provider = IsrcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            isrc: Some("GBAYE0601498".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("a non-JSON lookup body should fall back to search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Non JSON Fallback"));
    }

    #[tokio::test]
    async fn isrc_provider_search_lookup_empty_recordings_falls_back_to_search() {
        // A well-formed 200 whose `recordings` array is simply empty (e.g. the
        // ISRC genuinely isn't registered under this endpoint's index) must
        // also fall back — not just outright parse failures.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/isrc/GBAYE0601498"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"isrc":"GBAYE0601498","recordings":[]}"#),
            )
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(query_param("query", "isrc:GBAYE0601498"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"recordings":[{"id":"mb-5","title":"Empty Lookup Fallback"}]}"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let provider = IsrcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            isrc: Some("GBAYE0601498".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("an empty-but-well-formed lookup should fall back to search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Empty Lookup Fallback"));
    }

    #[tokio::test]
    async fn isrc_provider_search_lookup_hit_never_calls_fallback_search() {
        // The mirror image of the fallback tests above: a lookup that DOES
        // return real recordings must never touch the fallback search endpoint
        // at all — proven via `.expect(0)`.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/isrc/GBAYE0601498"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"{"isrc":"GBAYE0601498","recordings":[{"id":"mb-6","title":"Real Hit"}]}"#,
            ))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"recordings":[]}"#))
            .expect(0)
            .mount(&server)
            .await;

        let provider = IsrcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            isrc: Some("GBAYE0601498".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("a genuine lookup hit should return directly without a fallback request");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title.as_deref(), Some("Real Hit"));
    }

    #[tokio::test]
    async fn isrc_provider_search_lookup_hit_truncates_to_max_results() {
        // FIX (issue #198): the dedicated lookup endpoint takes no `limit`
        // parameter of its own, so `search()` must truncate the parsed lookup
        // results to `max_results` itself, the same way `Registry::search()`
        // truncates the merged results from every other path.
        let server = MockServer::start().await;
        let recordings = (1..=5)
            .map(|n| format!(r#"{{"id":"mb-{n}","title":"Track {n}"}}"#))
            .collect::<Vec<_>>()
            .join(",");
        Mock::given(method("GET"))
            .and(path("/ws/2/isrc/GBAYE0601498"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"{{"isrc":"GBAYE0601498","recordings":[{recordings}]}}"#
            )))
            .mount(&server)
            .await;

        let provider = IsrcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            isrc: Some("GBAYE0601498".into()),
            max_results: Some(2),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("lookup hit should succeed");
        assert_eq!(results.len(), 2);
    }

    // =========================================================================
    // EIDR Provider tests
    // =========================================================================

    #[test]
    fn eidr_provider_name() {
        assert_eq!(EidrProvider::new(None, None).id(), "eidr");
    }

    #[test]
    fn eidr_provider_capabilities() {
        let caps = EidrProvider::new(None, None).capabilities();
        assert!(caps.identifier_lookup);
        assert!(caps.video_search);
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
        // EIDR is now stored in metadata
        assert_eq!(
            results[0]
                .metadata
                .get(META_EIDR)
                .and_then(serde_json::Value::as_str),
            Some("10.5240/AEBE-0317-CE0D-4943-5916-E")
        );
    }

    // =========================================================================
    // ISWC Provider tests
    // =========================================================================

    #[test]
    fn iswc_provider_name() {
        assert_eq!(IswcProvider::new("App/1.0").id(), "iswc");
    }

    #[test]
    fn iswc_provider_capabilities() {
        let caps = IswcProvider::new("App/1.0").capabilities();
        assert!(caps.identifier_lookup);
        assert!(caps.music_search);
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
        assert_eq!(
            results[0]
                .metadata
                .get(META_ISWC)
                .and_then(serde_json::Value::as_str),
            Some("T0345246801")
        );
    }

    #[test]
    fn iswc_provider_parse_invalid_json_returns_err() {
        assert!(matches!(
            IswcProvider::parse_works("iswc", "bad"),
            Err(ProviderError::Other(_))
        ));
    }

    #[tokio::test]
    async fn iswc_provider_search_without_iswc_returns_not_supported() {
        let p = IswcProvider::new("App/1.0");
        let q = SearchQuery {
            max_results: Some(5),
            ..Default::default()
        };
        assert!(matches!(
            p.search(&q).await,
            Err(ProviderError::NotSupported(_))
        ));
    }

    // -------------------------------------------------------------------
    // IswcProvider — wiremock integration tests (search() end to end)
    // -------------------------------------------------------------------

    #[tokio::test]
    async fn iswc_provider_search_queries_both_punctuated_and_bare_forms() {
        // FIX (issue #198): MusicBrainz canonically DISPLAYS ISWCs punctuated,
        // and whether its search index normalises punctuation out of `iswc:`
        // queries server-side is an analyzer detail we cannot verify offline.
        // `iswc_query()` therefore emits BOTH forms OR-ed together — one HTTP
        // request either way — and this asserts that's what hits the wire.
        let raw = "T-034524680-1";
        let expected_query = format!(r#"iswc:T0345246801 OR iswc:"{raw}""#);
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/work/"))
            .and(query_param("query", expected_query.as_str()))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"works":[]}"#))
            .expect(1)
            .mount(&server)
            .await;

        let provider = IswcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            iswc: Some(raw.into()),
            max_results: Some(5),
            ..Default::default()
        };
        provider
            .search(&query)
            .await
            .expect("a punctuated ISWC should query both forms in one request");
    }

    #[tokio::test]
    async fn iswc_provider_search_enriches_first_result_with_composer() {
        // A plain work-search response carries no `relations` at all — the
        // enrichment lookup is what supplies the composer credit (and, in this
        // fixture, echoes the ISWC too). Uses an MBID-shaped id — real
        // MusicBrainz work IDs are UUIDs, and `looks_like_mbid()` (FIX for
        // issue #198) gates enrichment on that shape.
        let mbid = "b10bbbfc-cf9e-42e0-be17-e2c3e1d2600d";
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/work/"))
            .and(query_param("query", "iswc:T0345246801"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"{{"works":[{{"id":"{mbid}","title":"Bohemian Rhapsody"}}]}}"#
            )))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/ws/2/work/{mbid}")))
            .and(query_param("inc", "artist-rels"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"{{"id":"{mbid}","title":"Bohemian Rhapsody","iswcs":["T0345246801"],"relations":[{{"type":"composer","artist":{{"name":"Freddie Mercury"}}}}]}}"#
            )))
            .expect(1)
            .mount(&server)
            .await;

        let provider = IswcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            iswc: Some("T0345246801".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("search should succeed and be enriched");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].artist.as_deref(), Some("Freddie Mercury"));
        // Upstream `ProviderResult` has no `iswc` field — it lands in the
        // metadata blob under `META_ISWC`.
        assert_eq!(
            results[0]
                .metadata
                .get(META_ISWC)
                .and_then(serde_json::Value::as_str),
            Some("T0345246801")
        );
    }

    #[tokio::test]
    async fn iswc_provider_search_enrichment_failure_returns_unenriched_results() {
        // The enrichment lookup 500s — the overall search must still succeed,
        // just without a composer credit. MBID-shaped id so the enrichment
        // lookup is actually ATTEMPTED (and fails), rather than being
        // pre-emptively skipped by `looks_like_mbid()`'s guard.
        let mbid = "c20ccccc-df9e-42e0-be17-e2c3e1d2600e";
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/work/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"{{"works":[{{"id":"{mbid}","title":"Some Work"}}]}}"#
            )))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/ws/2/work/{mbid}")))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let provider = IswcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            iswc: Some("T0345246801".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("a failed enrichment lookup must not fail the overall search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].artist, None);
    }

    #[tokio::test]
    async fn iswc_provider_search_skips_enrichment_when_composer_already_present() {
        // The search response already carries a composer via inlined
        // `relations` — no enrichment lookup should ever be sent, proven by
        // `.expect(0)` on that mock.
        let mbid = "d30ddddd-ef9e-42e0-be17-e2c3e1d2600f";
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/work/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(format!(
                r#"{{"works":[{{"id":"{mbid}","title":"Already Enriched","relations":[{{"type":"composer","artist":{{"name":"Existing Composer"}}}}]}}]}}"#
            )))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/ws/2/work/{mbid}")))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .expect(0)
            .mount(&server)
            .await;

        let provider = IswcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            iswc: Some("T0345246801".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("search should succeed");
        assert_eq!(results[0].artist.as_deref(), Some("Existing Composer"));
    }

    #[tokio::test]
    async fn iswc_provider_search_no_id_skips_enrichment_request() {
        // FIX (issue #198): a search result with no `id` at all leaves no
        // `META_PROVIDER_ID` in the metadata blob, so `work_mbid()` yields ""
        // — which would turn the enrichment lookup URL into
        // `{base}/ws/2/work/`, the collection/search endpoint rather than a
        // single-resource lookup: a guaranteed 4xx that still spends a
        // rate-limit token. `looks_like_mbid()` must gate that off. Proven
        // here by asserting `.expect(0)` on a mock that matches the SAME path
        // the (guarded-against) enrichment request would hit, but WITHOUT the
        // `query` param the initial search request carries — the two are
        // otherwise indistinguishable by path alone, since an empty id
        // collapses the lookup-by-id URL onto the search URL.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/work/"))
            .and(query_param("query", "iswc:T0345246801"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"works":[{"title":"No MBID Work"}]}"#),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ws/2/work/"))
            .and(query_param_is_missing("query"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
            .expect(0)
            .mount(&server)
            .await;

        let provider = IswcProvider::with_base_url("TestApp/1.0", server.uri());
        let query = SearchQuery {
            iswc: Some("T0345246801".into()),
            max_results: Some(5),
            ..Default::default()
        };
        let results = provider
            .search(&query)
            .await
            .expect("search should succeed even with no id on the result");
        assert_eq!(results.len(), 1);
        assert!(!results[0].metadata.contains_key(META_PROVIDER_ID));
        assert_eq!(results[0].artist, None);
    }
}
