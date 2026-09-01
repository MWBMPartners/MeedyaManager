// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — MusicBrainz Integration Seam
//
// *** THE MIGRATION SEAM ***
//
// MusicBrainz has announced BREAKING changes to its search API, effective
// 2026-11-30 (the replacement spec has not been published yet at the time
// this module was written). To make that migration a one-file change
// instead of a hunt-and-peck across the whole crate, EVERY piece of
// MusicBrainz-specific knowledge — endpoint URLs, query-string parameter
// names, Lucene query syntax, and response models — lives in THIS MODULE
// and ONLY this module. No other provider module (music/mod.rs,
// identifiers/mod.rs, or any future caller) may construct a MusicBrainz URL,
// build a MusicBrainz query string, hand-write Lucene syntax, or parse a
// MusicBrainz JSON response directly; they call into the functions exported
// here instead.
//
// When the new spec lands, apply the deltas in exactly these places:
//
//   - Endpoint PATHS changed?
//       → `MB_DEFAULT_BASE_URL`, `WS2_ROOT`, `MbEntity::path_segment()`
//   - Query-string PARAMETER NAMES changed (e.g. `query` → `q`)?
//       → `search_params()`, `lookup_params()`
//   - Lucene QUERY DIALECT changed (escaping rules, field names, operators)?
//       → `lucene_escape()`, `lucene_phrase()`, `recording_query()`,
//         `isrc_query()`, `iswc_query()`
//   - Response SHAPE changed (renamed/restructured JSON fields)?
//       → the `models` sub-module (`MbRecordingSearchResponse`, `MbRecording`,
//         `MbIsrcLookupResponse`, `MbWorkSearchResponse`, `MbWork`, etc.)
//
// Nothing else in the crate should need to change. Providers depend on this
// module's public functions/types, not on MusicBrainz's wire format.
//
// *** RATE LIMITING ***
//
// MusicBrainz documents a shared 1 request/second (60 RPM) limit across ALL
// traffic to musicbrainz.org — not 1 req/sec *per feature*. Because
// MusicBrainz search, ISRC lookup, and ISWC lookup are really the same
// upstream service, `mb_get()` below routes every request through
// `mb_limiter_for()`, which resolves to `rate_limiter::shared_host_limiter()`
// keyed by target host — so all three providers draw from ONE shared budget.
//
// Public API summary:
//   MB_DEFAULT_BASE_URL, MB_REQUESTS_PER_MINUTE, MB_BURST — tunable constants
//   MbEntity                     — recording / work / isrc
//   search_url() / lookup_url()  — endpoint URL builders
//   search_params() / lookup_params() — query-string builders
//   lucene_escape() / lucene_phrase() — Lucene syntax helpers
//   normalise_isrc() / normalise_iswc() — identifier canonicalisation
//   recording_query() / recording_query_loose() / isrc_query() / iswc_query() — full Lucene queries
//   models::*                    — response structs + mapping helpers
//   mb_limiter_for() / ensure_contact() / mb_get() — shared request executor

use std::sync::Arc;

use governor::DefaultDirectRateLimiter;

use crate::traits::ProviderError;

// ---------------------------------------------------------------------------
// Endpoints
// ---------------------------------------------------------------------------

/// Production MusicBrainz base URL. Tests and self-hosted mirrors override
/// this by passing a different `base_url` into `search_url()` / `lookup_url()`.
pub const MB_DEFAULT_BASE_URL: &str = "https://musicbrainz.org";

/// The MusicBrainz Web Service version-2 root path, common to every endpoint.
const WS2_ROOT: &str = "/ws/2";

/// The MusicBrainz entity types this crate looks up. Each variant maps to one
/// URL path segment under `WS2_ROOT` — see `path_segment()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MbEntity {
    /// A recording (track) — used for title/artist search and ISRC-embedded search.
    Recording,
    /// A musical work (composition) — used for ISWC lookups.
    Work,
    /// The dedicated ISRC lookup entity (`/ws/2/isrc/<isrc>`), distinct from
    /// searching recordings BY isrc via the recording search endpoint.
    Isrc,
}

impl MbEntity {
    /// The URL path segment MusicBrainz uses for this entity type.
    fn path_segment(self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::Work => "work",
            Self::Isrc => "isrc",
        }
    }
}

/// Build the search endpoint URL for `entity` under `base_url`.
///
/// Trims exactly ONE trailing `/` from `base_url` (if present) before
/// appending the WS2 root and entity segment, so both
/// `"https://musicbrainz.org"` and `"https://musicbrainz.org/"` produce the
/// same, correctly-slashed URL.
///
/// Format: `{base_trimmed}/ws/2/{entity}/` (trailing slash — MusicBrainz's
/// documented convention for the search/browse form of each endpoint).
pub fn search_url(base_url: &str, entity: MbEntity) -> String {
    let trimmed = base_url.strip_suffix('/').unwrap_or(base_url);
    format!("{trimmed}{WS2_ROOT}/{}/", entity.path_segment())
}

/// Build the lookup-by-id endpoint URL for `entity` under `base_url`.
///
/// Format: `{base_trimmed}/ws/2/{entity}/{id}` (no trailing slash — this is
/// the single-resource lookup form, distinct from `search_url()`'s
/// collection-search form).
pub fn lookup_url(base_url: &str, entity: MbEntity, id: &str) -> String {
    let trimmed = base_url.strip_suffix('/').unwrap_or(base_url);
    format!("{trimmed}{WS2_ROOT}/{}/{id}", entity.path_segment())
}

/// Build the query-string parameters for a MusicBrainz search request.
///
/// Always includes `query`, `limit`, and `fmt=json`; includes `offset` ONLY
/// when `offset > 0` — MusicBrainz treats an explicit `offset=0` identically
/// to omitting it, so omitting keeps request logs and cache keys tidier.
pub fn search_params(
    lucene_query: &str,
    limit: usize,
    offset: usize,
) -> Vec<(&'static str, String)> {
    let mut params = vec![
        ("query", lucene_query.to_owned()),
        ("limit", limit.to_string()),
        ("fmt", "json".to_owned()),
    ];
    if offset > 0 {
        params.push(("offset", offset.to_string()));
    }
    params
}

/// Build the query-string parameters for a MusicBrainz lookup-by-id request.
///
/// Always includes `fmt=json`; includes `inc` (the MusicBrainz "include
/// sub-resources" parameter, e.g. `"artist-credits+releases"`) only when the
/// caller supplies one.
pub fn lookup_params(inc: Option<&str>) -> Vec<(&'static str, String)> {
    let mut params = vec![("fmt", "json".to_owned())];
    if let Some(inc) = inc {
        params.push(("inc", inc.to_owned()));
    }
    params
}

// ---------------------------------------------------------------------------
// Lucene query building
// ---------------------------------------------------------------------------
//
// Per-field escaping policy (this is the one place that dialect is decided):
//   - Title / artist values are PHRASE-QUOTED (`lucene_phrase()`), not
//     character-escaped, so that Lucene operators appearing naturally inside
//     a title (e.g. "AC/DC: Back? [Live] AND More") are treated as literal
//     text rather than query syntax — a quoted phrase only needs its own
//     quote and backslash characters escaped.
//   - Free-text fallback queries (used when neither title nor artist is
//     known) are ESCAPED, not phrase-quoted (`lucene_escape()`), so the user
//     can still type simple unquoted searches. This is a DOCUMENTED
//     LIMITATION: a bare `AND` / `OR` / `NOT` typed by the user in free text
//     remains a live Lucene operator, since escaping individual words is not
//     something we do (and MusicBrainz has no free-text phrase mode that
//     preserves operator-like words as literals without full phrase quoting,
//     which would itself break multi-term free-text matching).
//   - Identifiers (ISRC, ISWC) are NORMALISED, not escaped — canonical
//     identifier characters (`[A-Z0-9]`) never collide with Lucene syntax, so
//     escaping would be a no-op at best and a correctness risk at worst if
//     normalisation and escaping ever disagreed on what counts as "safe".

/// Backslash-escape every Lucene special character in `raw`.
///
/// The escaped set is exactly: `+ - & | ! ( ) { } [ ] ^ " ~ * ? : \ /`
/// (this is Lucene's standard special-character list, plus `/` which Lucene
/// treats as the regex-query delimiter).
pub fn lucene_escape(raw: &str) -> String {
    // Every character requiring a backslash escape before it appears
    // literally in a Lucene query string.
    const SPECIAL: [char; 19] = [
        '+', '-', '&', '|', '!', '(', ')', '{', '}', '[', ']', '^', '"', '~', '*', '?', ':', '\\',
        '/',
    ];
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        if SPECIAL.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Wrap `raw` in a Lucene double-quoted phrase.
///
/// Escapes only the two characters that would otherwise break out of the
/// phrase: `\` and `"`. `\` is escaped FIRST so that the backslashes
/// introduced while escaping `"` are not themselves re-escaped on a second pass.
pub fn lucene_phrase(raw: &str) -> String {
    let mut escaped = String::with_capacity(raw.len() + 2);
    for c in raw.chars() {
        match c {
            // Escape backslash first (see doc comment above) — a literal
            // backslash in the input must become a *single* escaped backslash.
            '\\' => escaped.push_str(r"\\"),
            // A literal quote must not terminate the phrase early.
            '"' => escaped.push_str("\\\""),
            _ => escaped.push(c),
        }
    }
    format!("\"{escaped}\"")
}

/// Upper-case `raw` and strip every character that is not ASCII alphanumeric,
/// producing the canonical MusicBrainz ISRC form (e.g. `"gb-aye-06-01498"` →
/// `"GBAYE0601498"`).
pub fn normalise_isrc(raw: &str) -> String {
    raw.to_uppercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect()
}

/// Upper-case `raw` and strip every character that is not ASCII alphanumeric,
/// producing the canonical MusicBrainz ISWC form (e.g. `"T-034.524.680-1"` →
/// `"T0345246801"`).
pub fn normalise_iswc(raw: &str) -> String {
    raw.to_uppercase()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect()
}

/// Shared field-joining logic behind `recording_query()` and
/// `recording_query_loose()`: build the `recording:`/`artistname:` clause
/// list, running each present value through `quote` (the only thing that
/// differs between the two callers), then join the clauses with `" AND "`.
/// Returns `None` when neither `title` nor `artist` is present, so callers
/// can fall back to their own free-text handling.
fn join_recording_fields(
    title: Option<&str>,
    artist: Option<&str>,
    quote: impl Fn(&str) -> String,
) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(title) = title {
        parts.push(format!("recording:{}", quote(title)));
    }
    if let Some(artist) = artist {
        parts.push(format!("artistname:{}", quote(artist)));
    }
    (!parts.is_empty()).then(|| parts.join(" AND "))
}

/// Build the Lucene query for a MusicBrainz recording search.
///
/// Mirrors the query-shape logic that used to live inline in
/// `music/mod.rs`'s `MusicBrainzProvider::search()`, but with title/artist
/// PHRASE-QUOTED rather than merely quote-stripped — see the module-level
/// escaping-policy comment above for why.
///
/// - `title` and/or `artist` present → `recording:"<title>"` and/or
///   `artistname:"<artist>"`, joined with `" AND "` when both are present.
/// - Neither present → falls back to `lucene_escape(free_text)`.
pub fn recording_query(title: Option<&str>, artist: Option<&str>, free_text: &str) -> String {
    join_recording_fields(title, artist, lucene_phrase).unwrap_or_else(|| lucene_escape(free_text))
}

/// Build a LOOSENED Lucene query for a MusicBrainz recording search — same
/// fields as `recording_query()`, but with title/artist ESCAPED
/// (`lucene_escape()`) rather than phrase-quoted.
///
/// This is the RETRY query `MusicBrainzProvider::search()` uses when a
/// phrase-quoted `recording_query()` search comes back with zero results.
/// A phrase query is an exact, ordered-token match, so real-world tag
/// decorations a MusicBrainz title lacks — `(Remastered 2011)`, `(Live)`,
/// `feat. X` — can make an otherwise-correct phrase query match nothing.
/// Escaped (rather than phrase-quoted) terms still keep Lucene syntax
/// neutered — the whole point of the original escaping fix — but no longer
/// require the escaped tokens to appear as one exact contiguous phrase, so
/// MusicBrainz's own relevance ranking gets a chance to find a near-match
/// the strict phrase query couldn't. This restores (most of) the recall the
/// old, pre-migration, unescaped/loose query accidentally provided, without
/// reintroducing the Lucene-injection risk that query was buggy about.
///
/// - `title` and/or `artist` present → `recording:<escaped title>` and/or
///   `artistname:<escaped artist>`, joined with `" AND "` when both present.
/// - Neither present → falls back to `lucene_escape(free_text)` — identical
///   to `recording_query()`'s free-text handling, since there is nothing
///   left to "loosen" once a query is already a bag of escaped free-text
///   words rather than a phrase. Callers should not actually invoke this
///   path (see `MusicBrainzProvider::search()`'s retry-gating comment), but
///   it degrades safely if they do.
pub fn recording_query_loose(title: Option<&str>, artist: Option<&str>, free_text: &str) -> String {
    join_recording_fields(title, artist, lucene_escape).unwrap_or_else(|| lucene_escape(free_text))
}

/// Build the Lucene query for an ISRC-based recording search:
/// `isrc:<normalised-isrc>`.
///
/// Unlike `iswc_query()` below, this normalises unconditionally and never
/// falls back to also sending the raw form. That asymmetry is intentional:
/// MusicBrainz canonically stores AND displays ISRCs UNPUNCTUATED
/// (`GBAYE0601498`, never `GB-AYE-06-01498`), so normalising here is a strict
/// improvement over sending the user's raw (possibly hyphenated) input — it
/// is not a change that risks breaking a previously-working query the way
/// `iswc_query()`'s normalisation did (see that function's doc comment).
pub fn isrc_query(isrc: &str) -> String {
    format!("isrc:{}", normalise_isrc(isrc))
}

/// Build the Lucene query for an ISWC-based work search.
///
/// Emits BOTH the normalised (punctuation-stripped) form and the caller's
/// original input as a phrase-quoted term, `OR`-ed together — e.g.
/// `iswc:T0345246801 OR iswc:"T-034.524.680-1"` — UNLESS the raw input is
/// already identical to its normalised form, in which case only the single
/// bare term is emitted (no redundant `OR` clause).
///
/// Why both forms: MusicBrainz canonically stores and DISPLAYS ISWCs
/// PUNCTUATED (`T-034.524.680-1`), unlike ISRCs. Whether MusicBrainz's search
/// index actually normalises punctuation out of `iswc:` queries server-side
/// is an analyzer implementation detail this project has no way to verify
/// without live network access to musicbrainz.org — and got wrong once
/// already: an earlier version of this function unconditionally stripped
/// punctuation before this comment was written, which would have silently
/// broken every previously-working punctuated-ISWC lookup if the server-side
/// index does NOT normalise. Querying both forms costs nothing extra on the
/// wire (still one HTTP request) and is correct regardless of which way that
/// unverifiable analyzer behaviour actually goes.
///
/// The raw form is phrase-quoted (`lucene_phrase()`), not character-escaped,
/// so its punctuation (`-`, `.`) can't be misparsed as Lucene query syntax.
pub fn iswc_query(iswc: &str) -> String {
    let normalised = normalise_iswc(iswc);
    if iswc == normalised {
        // Already in canonical form — a second, redundant OR clause would
        // just cost the server extra matching work for zero benefit.
        format!("iswc:{normalised}")
    } else {
        format!("iswc:{normalised} OR iswc:{}", lucene_phrase(iswc))
    }
}

// ---------------------------------------------------------------------------
// Response models
// ---------------------------------------------------------------------------

/// MusicBrainz JSON response shapes and the mapping from those shapes into
/// this crate's `ProviderResult`.
///
/// IMPORTANT: never add `#[serde(deny_unknown_fields)]` to any struct in this
/// module. MusicBrainz responses routinely carry fields we intentionally
/// don't map (tags, disambiguation text, relation URLs, ...), and the
/// announced 2026-11-30 breaking change may add further fields before
/// removing/renaming the ones we depend on — `deny_unknown_fields` would turn
/// those harmless additions into hard parse failures for us.
pub mod models {
    use serde::Deserialize;

    use crate::traits::ProviderResult;

    // -----------------------------------------------------------------------
    // Score deserialization
    // -----------------------------------------------------------------------

    /// Deserialize a MusicBrainz relevance `score`, which the API returns as
    /// an integer 0-100 in the documented case but has also been observed
    /// (undocumented) as a JSON float or a numeric string. Any value that
    /// isn't one of {int, float, numeric string} — a bool, object, array, or
    /// unparseable string — resolves to `None` rather than failing the whole
    /// response: `score` is advisory ranking data, never worth losing an
    /// entire result set over.
    fn de_score<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize into a generic JSON value first. This is deliberately
        // more permissive than an untagged enum over {f64, i64, String}: an
        // untagged enum would still hard-fail on a bool/object/array score,
        // whereas going through `serde_json::Value` lets us fall through to
        // `None` for absolutely any shape we don't recognise.
        let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
        Ok(value.and_then(|v| match v {
            // `null` is already collapsed to `None` by `Option::deserialize`
            // above, but is listed here too for completeness/clarity.
            serde_json::Value::Null => None,
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => s.trim().parse::<f64>().ok(),
            // Booleans, objects, arrays: not a score, and not an error either.
            serde_json::Value::Bool(_)
            | serde_json::Value::Array(_)
            | serde_json::Value::Object(_) => None,
        }))
    }

    // -----------------------------------------------------------------------
    // Recording search (GET /ws/2/recording/)
    // -----------------------------------------------------------------------

    /// Top-level response from a MusicBrainz recording search.
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct MbRecordingSearchResponse {
        /// The matched recordings, ranked by relevance. Absent entirely on a
        /// zero-result search (MusicBrainz omits the key rather than sending
        /// `[]` in some observed responses), hence `#[serde(default)]`.
        #[serde(default)]
        pub recordings: Vec<MbRecording>,
        /// Total number of matches on the server side (may exceed `recordings.len()`
        /// when `limit` truncated the page).
        #[serde(default)]
        pub count: Option<u64>,
        /// The offset this page of results started at.
        #[serde(default)]
        pub offset: Option<u64>,
    }

    /// A single MusicBrainz recording (track), as returned by search or by
    /// the ISRC lookup endpoint's embedded `recordings` array.
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct MbRecording {
        /// MusicBrainz Identifier (MBID) — the recording's canonical UUID.
        pub id: Option<String>,
        /// Recording (track) title.
        pub title: Option<String>,
        /// Performing artist(s) — MusicBrainz's kebab-case JSON key
        /// (`"artist-credit"`) is remapped to idiomatic Rust naming here.
        #[serde(rename = "artist-credit", default)]
        pub artist_credit: Vec<MbArtistCredit>,
        /// Releases (albums/singles) this recording appears on.
        #[serde(default)]
        pub releases: Vec<MbRelease>,
        /// ISRC codes registered against this recording (a recording can have
        /// more than one, e.g. re-releases).
        #[serde(default)]
        pub isrcs: Vec<String>,
        /// Recording length in milliseconds.
        pub length: Option<u64>,
        /// Search relevance score, 0-100 (see `de_score()` for the lenient
        /// parsing this field goes through).
        #[serde(default, deserialize_with = "de_score")]
        pub score: Option<f64>,
    }

    /// One entry in a recording's `artist-credit` array — pairs an artist
    /// with how they're credited on this specific recording.
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct MbArtistCredit {
        /// The credited artist's core identity fields.
        pub artist: Option<MbArtist>,
    }

    /// Minimal artist identity as embedded in an `artist-credit` entry.
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct MbArtist {
        /// The artist's canonical name.
        pub name: Option<String>,
    }

    /// A release (album/single/EP) a recording appears on.
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct MbRelease {
        /// Release (album) title.
        pub title: Option<String>,
        /// Release date, ISO-8601-ish (`"YYYY"`, `"YYYY-MM"`, or `"YYYY-MM-DD"`)
        /// — only the first 4 characters (the year) are ever extracted by
        /// `recording_to_result()`.
        pub date: Option<String>,
        /// Number of tracks on the release. Not currently mapped into
        /// `ProviderResult`, but kept so callers that need it don't have to
        /// re-fetch — and so this struct stays a faithful mirror of the wire
        /// shape for the migration seam's sake.
        #[serde(rename = "track-count")]
        pub track_count: Option<u32>,
    }

    // -----------------------------------------------------------------------
    // ISRC lookup (GET /ws/2/isrc/<isrc>)
    // -----------------------------------------------------------------------

    /// Response from the dedicated MusicBrainz ISRC lookup endpoint — a
    /// slimmer shape than recording search, wrapping the matching recordings.
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct MbIsrcLookupResponse {
        /// The ISRC that was looked up, echoed back by MusicBrainz.
        pub isrc: Option<String>,
        /// Every recording registered against that ISRC.
        #[serde(default)]
        pub recordings: Vec<MbRecording>,
    }

    // -----------------------------------------------------------------------
    // Work search (GET /ws/2/work/) — used for ISWC lookups
    // -----------------------------------------------------------------------

    /// Top-level response from a MusicBrainz work (composition) search.
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct MbWorkSearchResponse {
        /// The matched works, ranked by relevance.
        #[serde(default)]
        pub works: Vec<MbWork>,
    }

    /// A single MusicBrainz work (musical composition, as distinct from a
    /// recorded performance of it).
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct MbWork {
        /// MusicBrainz Identifier (MBID) for the work.
        pub id: Option<String>,
        /// Work (composition) title.
        pub title: Option<String>,
        /// ISWC codes registered against this work.
        #[serde(default)]
        pub iswcs: Vec<String>,
        /// Relationships to other entities (artists, other works, URLs, ...) —
        /// used here to extract the composer credit via `work_composer()`.
        #[serde(default)]
        pub relations: Vec<MbRelation>,
        /// Search relevance score, 0-100 (lenient parsing — see `de_score()`).
        #[serde(default, deserialize_with = "de_score")]
        pub score: Option<f64>,
    }

    /// One entry in a work's `relations` array — a typed link to another
    /// entity (an artist, another work, an external URL, ...).
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct MbRelation {
        /// The relation type, e.g. `"composer"`, `"lyricist"`, `"performer"`.
        /// MusicBrainz's JSON key is the bare word `"type"`, which collides
        /// with the Rust keyword, hence the rename.
        #[serde(rename = "type")]
        pub rel_type: Option<String>,
        /// The related artist, present only for artist-type relations.
        pub artist: Option<MbRelArtist>,
    }

    /// Minimal artist identity as embedded in a work relation.
    #[derive(Debug, Clone, Default, Deserialize)]
    pub struct MbRelArtist {
        /// The artist's canonical name.
        pub name: Option<String>,
    }

    // -----------------------------------------------------------------------
    // Mapping into ProviderResult
    // -----------------------------------------------------------------------

    /// Map a `MbRecording` into this crate's unified `ProviderResult`.
    ///
    /// Replicates, field for field, the mapping that used to live inline in
    /// `MusicBrainzProvider::parse_recordings()` (`music/mod.rs`):
    ///   - artist-credit names are joined with `"; "`
    ///   - the FIRST release supplies album title + year (first 4 chars of
    ///     its `date`, parsed as `u32`; anything that doesn't parse as a
    ///     4-digit year yields `None` rather than erroring)
    ///   - the FIRST ISRC (if any) is used
    ///   - length is converted from milliseconds to seconds
    ///   - MusicBrainz's 0-100 score is normalised to `[0.0, 1.0]` and
    ///     defensively clamped in case an out-of-range value is ever returned
    pub fn recording_to_result(provider_name: &str, rec: MbRecording) -> ProviderResult {
        // Combine every credited artist's name with "; " — MusicBrainz
        // splits collaborations ("Artist A feat. Artist B") into separate
        // artist-credit entries rather than a single combined string.
        let artist = (!rec.artist_credit.is_empty()).then(|| {
            rec.artist_credit
                .iter()
                .filter_map(|c| c.artist.as_ref()?.name.as_deref())
                .collect::<Vec<_>>()
                .join("; ")
        });

        // Use the first release (if any) for album/year info — MusicBrainz
        // does not indicate a "primary" release, so first-in-list is the
        // same heuristic the pre-migration code used.
        let first_release = rec.releases.first();
        let album = first_release.and_then(|r| r.title.clone());
        let year = first_release
            .and_then(|r| r.date.as_deref())
            .and_then(|d| d[..4.min(d.len())].parse::<u32>().ok());

        // MusicBrainz score is documented as 0-100; normalise to [0.0, 1.0]
        // and clamp defensively in case upstream ever sends an out-of-range
        // value (e.g. the observed-but-undocumented float form).
        let score = rec.score.map_or(0.0, |s| (s / 100.0).clamp(0.0, 1.0));

        ProviderResult {
            provider: provider_name.to_owned(),
            provider_id: rec.id.unwrap_or_default(),
            title: rec.title,
            artist,
            album,
            year,
            // First registered ISRC, if any — a recording can have several
            // (e.g. across re-releases) but ProviderResult carries only one.
            isrc: rec.isrcs.into_iter().next(),
            duration_secs: rec.length.map(|ms| ms as f64 / 1000.0),
            score,
            ..Default::default()
        }
    }

    /// Extract the composer credit from a work's `relations` array — the
    /// first relation whose `rel_type` is exactly `"composer"`.
    ///
    /// Replicates the extraction that used to live inline in
    /// `IswcProvider::parse_works()` (`identifiers/mod.rs`).
    pub fn work_composer(relations: &[MbRelation]) -> Option<String> {
        relations
            .iter()
            .find(|r| r.rel_type.as_deref() == Some("composer"))
            .and_then(|r| r.artist.as_ref()?.name.clone())
    }
}

// ---------------------------------------------------------------------------
// Shared rate limit + request executor
// ---------------------------------------------------------------------------

/// MusicBrainz's documented rate limit: 1 request/second, expressed here as
/// requests-per-minute to match `shared_host_limiter()`'s unit.
pub const MB_REQUESTS_PER_MINUTE: u32 = 60;

/// Burst allowance for the MusicBrainz limiter.
///
/// Kept at 1 (no burst above the steady 1 req/sec rate) because
/// MusicBrainz's own documentation warns that bursty traffic — even if it
/// averages out — can trigger a temporary ban.
pub const MB_BURST: u32 = 1;

/// Upper bound, in seconds, on a `Retry-After` value we're willing to honour
/// with an automatic single retry. A server asking for a longer wait than
/// this is treated the same as one that sent no `Retry-After` at all — the
/// caller gets `RateLimited` immediately rather than this function silently
/// blocking the caller's task for an arbitrarily long time.
const RETRY_AFTER_MAX_SECS: u64 = 10;

/// Resolve the `"host:port"` rate-limit bucket key for `url`.
///
/// Falls back to the raw `url` string when it fails to parse as a URL —
/// better to give the caller *a* (possibly over-isolated) limiter than to
/// panic on a malformed URL that reqwest will reject anyway.
fn host_key(url: &str) -> String {
    // Written as if-let/else (rather than a two-armed `match` with a
    // catch-all `Err(_)`) to keep clippy::single_match_else happy.
    if let Ok(parsed) = reqwest::Url::parse(url) {
        let host = parsed.host_str().unwrap_or_default();
        // `port_or_known_default()` resolves the scheme's default port
        // (443 for https, 80 for http) when the URL didn't specify one
        // explicitly, so that "https://musicbrainz.org/..." and
        // "https://musicbrainz.org:443/..." share the same bucket.
        let port = parsed.port_or_known_default().unwrap_or_default();
        format!("{host}:{port}")
    } else {
        url.to_owned()
    }
}

/// Resolve the shared MusicBrainz rate limiter for `url`.
///
/// Every request this module sends — recording search, ISRC lookup, ISWC
/// (work) search alike — routes through this function, so they all share one
/// token bucket keyed by target host (see the module doc comment's
/// "RATE LIMITING" section).
pub fn mb_limiter_for(url: &str) -> Arc<DefaultDirectRateLimiter> {
    crate::rate_limiter::shared_host_limiter(&host_key(url), MB_REQUESTS_PER_MINUTE, MB_BURST)
}

/// Ensure `user_agent` carries a contact address, as MusicBrainz's usage
/// policy requires (a User-Agent with no way to reach the operator can get
/// silently deprioritised or banned).
///
/// - An empty `user_agent` is left unchanged (an empty UA is a separate,
///   pre-existing "disabled provider" signal handled elsewhere; adding a
///   contact segment to it would misleadingly make it look configured).
/// - A `user_agent` that already contains `'@'` (an email address) or
///   `"://"` (a URL) is assumed to already carry contact info and is left
///   unchanged, so callers that already build a contact-bearing UA (e.g. via
///   `mm_core::useragent::build_user_agent_with_contact()`) don't get a
///   second contact segment appended.
/// - Otherwise, the compiled-in / env-overridden contact string (from
///   `mm_core::useragent::contact_string()`) is appended in MusicBrainz's
///   documented `"UA ( contact )"` form.
pub fn ensure_contact(user_agent: String) -> String {
    if user_agent.is_empty() || user_agent.contains('@') || user_agent.contains("://") {
        user_agent
    } else {
        let contact = mm_core::useragent::contact_string();
        format!("{user_agent} ( {contact} )")
    }
}

/// Extract a `Retry-After` value as a plain integer delta-seconds count.
///
/// `Retry-After` may legally be either an integer delta-seconds count or an
/// HTTP-date. We only honour the delta-seconds form here — an HTTP-date (or
/// anything else that doesn't parse as a bare `u64`) is treated as "no
/// usable Retry-After", not as a parse error.
fn retry_after_secs(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
}

/// Send one MusicBrainz GET attempt — no rate-limit wait, no retry logic.
/// Shared by both the initial attempt and the single retry in `mb_get()`.
async fn mb_get_once(
    client: &reqwest::Client,
    user_agent: &str,
    url: &str,
    params: &[(&str, String)],
) -> Result<reqwest::Response, reqwest::Error> {
    let mut request = client
        .get(url)
        // MusicBrainz requires JSON responses to be requested explicitly.
        .header(reqwest::header::ACCEPT, "application/json");
    // Only set an explicit User-Agent header when one was supplied — an
    // empty string would otherwise override the client's own default UA
    // (set by `crate::http::build_client()`) with nothing at all.
    if !user_agent.is_empty() {
        request = request.header(reqwest::header::USER_AGENT, user_agent);
    }
    request.query(params).send().await
}

/// Handle a 429/503 response from the FIRST `mb_get_once()` attempt: honour a
/// small `Retry-After`, re-acquire the shared limiter, and retry exactly
/// once. A second 429/503 — or an absent/oversized/HTTP-date `Retry-After` —
/// is reported as `RateLimited` rather than retried further, so a
/// misbehaving upstream can never wedge the caller in an unbounded loop.
///
/// Split out from `mb_get()` to keep that function under the project's
/// function-length/complexity thresholds.
async fn retry_once_after_rate_limit(
    client: &reqwest::Client,
    provider_name: &str,
    user_agent: &str,
    url: &str,
    params: &[(&str, String)],
    first_response: &reqwest::Response,
) -> Result<String, ProviderError> {
    // No usable Retry-After (absent, an HTTP-date, or too large a wait) —
    // give up immediately rather than retrying blind.
    let Some(delay_secs) = retry_after_secs(first_response).filter(|&s| s <= RETRY_AFTER_MAX_SECS)
    else {
        return Err(ProviderError::RateLimited {
            provider: provider_name.to_owned(),
        });
    };

    // Honour the server's requested backoff before trying again.
    tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;

    // Re-acquire the shared limiter — the sleep covers the server's
    // requested wait, but another concurrent caller may have consumed the
    // bucket's tokens in the meantime, so we still queue behind them fairly.
    mb_limiter_for(url).until_ready().await;

    let response = mb_get_once(client, user_agent, url, params)
        .await
        .map_err(|e| ProviderError::Network(e.to_string()))?;

    if response.status().is_success() {
        return response
            .text()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()));
    }

    let status = response.status();
    if status.as_u16() == 429 || status.as_u16() == 503 {
        // Second consecutive throttle — stop retrying, surface RateLimited.
        return Err(ProviderError::RateLimited {
            provider: provider_name.to_owned(),
        });
    }
    Err(ProviderError::Network(format!("HTTP {status}")))
}

/// Execute a rate-limited, contact-header-bearing GET against MusicBrainz.
///
/// This is the ONLY way any provider in this crate should talk to
/// MusicBrainz over HTTP. Sequence:
///   1. Wait for the shared MusicBrainz token bucket (`mb_limiter_for()`).
///   2. Send the GET with `Accept: application/json` and (if non-empty) the
///      given `User-Agent`, plus `params` as the query string.
///   3. A 2xx response's body is returned as `Ok(String)`.
///   4. A 429 or 503 triggers `retry_once_after_rate_limit()` (see its doc
///      comment for the exact retry policy).
///   5. Any other non-2xx status is reported as `ProviderError::Network`.
pub async fn mb_get(
    client: &reqwest::Client,
    provider_name: &str,
    user_agent: &str,
    url: &str,
    params: &[(&str, String)],
) -> Result<String, ProviderError> {
    // Step 1: respect the shared MusicBrainz rate-limit bucket before sending.
    mb_limiter_for(url).until_ready().await;

    // Step 2: the first attempt.
    let response = mb_get_once(client, user_agent, url, params)
        .await
        .map_err(|e| ProviderError::Network(e.to_string()))?;

    // Step 3: success — hand back the raw body for the caller to parse via
    // the `models` sub-module.
    if response.status().is_success() {
        return response
            .text()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()));
    }

    // Step 4: rate-limited — delegate to the retry helper.
    let status = response.status();
    if status.as_u16() == 429 || status.as_u16() == 503 {
        return retry_once_after_rate_limit(
            client,
            provider_name,
            user_agent,
            url,
            params,
            &response,
        )
        .await;
    }

    // Step 5: any other non-2xx status.
    Err(ProviderError::Network(format!("HTTP {status}")))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::models::{
        MbArtistCredit, MbRecording, MbRelArtist, MbRelation, MbRelease, recording_to_result,
        work_composer,
    };
    use super::*;

    // -----------------------------------------------------------------------
    // lucene_escape / lucene_phrase
    // -----------------------------------------------------------------------

    #[test]
    fn lucene_escape_escapes_every_special_char() {
        let raw = r#"+-&|!(){}[]^"~*?:\/"#;
        let escaped = lucene_escape(raw);
        assert_eq!(escaped, r#"\+\-\&\|\!\(\)\{\}\[\]\^\"\~\*\?\:\\\/"#);
    }

    #[test]
    fn lucene_escape_leaves_plain_text_untouched() {
        assert_eq!(lucene_escape("hello world"), "hello world");
    }

    #[test]
    fn lucene_phrase_escapes_quote_and_backslash() {
        let phrase = lucene_phrase(r#"Song "Quoted" \ Title"#);
        assert_eq!(phrase, r#""Song \"Quoted\" \\ Title""#);
    }

    #[test]
    fn lucene_phrase_wraps_plain_text_in_quotes() {
        assert_eq!(lucene_phrase("Comfortably Numb"), "\"Comfortably Numb\"");
    }

    // -----------------------------------------------------------------------
    // recording_query
    // -----------------------------------------------------------------------

    #[test]
    fn recording_query_title_and_artist_are_phrase_quoted_and_joined() {
        let query = recording_query(
            Some("AC/DC: Back? [Live] AND More"),
            Some("Simon & Garfunkel"),
            "",
        );
        assert_eq!(
            query,
            r#"recording:"AC/DC: Back? [Live] AND More" AND artistname:"Simon & Garfunkel""#
        );
    }

    #[test]
    fn recording_query_title_only() {
        let query = recording_query(Some("Let It Be"), None, "");
        assert_eq!(query, r#"recording:"Let It Be""#);
    }

    #[test]
    fn recording_query_free_text_only_escapes_special_chars() {
        // Free text goes through lucene_escape, not phrase-quoting — bare
        // '?' and '/' must come out backslash-escaped.
        let query = recording_query(None, None, "what is love?/reprise");
        assert_eq!(query, r"what is love\?\/reprise");
    }

    // -----------------------------------------------------------------------
    // recording_query_loose
    // -----------------------------------------------------------------------

    #[test]
    fn recording_query_loose_escapes_instead_of_phrase_quoting() {
        // Same fields as the phrase-quoted form, but title/artist come out
        // escaped (Lucene special chars backslashed) rather than wrapped in
        // a `"..."` phrase — no quote characters should appear at all.
        let query = recording_query_loose(Some("Comfortably Numb"), Some("Pink Floyd"), "");
        assert_eq!(
            query,
            r"recording:Comfortably Numb AND artistname:Pink Floyd"
        );
        assert!(!query.contains('"'));
    }

    #[test]
    fn recording_query_loose_title_only() {
        let query = recording_query_loose(Some("Let It Be"), None, "");
        assert_eq!(query, "recording:Let It Be");
    }

    #[test]
    fn recording_query_loose_special_chars_are_escaped() {
        let query = recording_query_loose(Some("AC/DC: Back?"), None, "");
        assert_eq!(query, r"recording:AC\/DC\: Back\?");
    }

    #[test]
    fn recording_query_loose_free_text_only_matches_recording_query() {
        // With neither title nor artist, both functions degrade identically
        // to escaped free text — there is nothing left to "loosen".
        let free_text = "what is love?/reprise";
        assert_eq!(
            recording_query_loose(None, None, free_text),
            recording_query(None, None, free_text)
        );
    }

    // -----------------------------------------------------------------------
    // normalise_isrc / normalise_iswc / isrc_query / iswc_query
    // -----------------------------------------------------------------------

    #[test]
    fn isrc_query_normalises_and_prefixes() {
        assert_eq!(isrc_query("gb-aye-06-01498"), "isrc:GBAYE0601498");
    }

    #[test]
    fn iswc_query_punctuated_input_emits_both_forms_joined_by_or() {
        // Punctuated input differs from its normalised form, so both the
        // bare normalised term AND the raw (phrase-quoted) form are sent,
        // `OR`-ed together — see `iswc_query()`'s doc comment for why the
        // server-side analyzer behaviour can't be verified offline.
        let query = iswc_query("T-034.524.680-1");
        assert_eq!(query, r#"iswc:T0345246801 OR iswc:"T-034.524.680-1""#);
    }

    #[test]
    fn iswc_query_already_normalised_input_emits_single_bare_term() {
        // Raw input already equals its normalised form — no redundant OR.
        assert_eq!(iswc_query("T0345246801"), "iswc:T0345246801");
    }

    #[test]
    fn iswc_query_raw_form_is_phrase_quoted() {
        // The raw (punctuated) form must be phrase-quoted, not bare, so its
        // hyphens/dots can't be misparsed as Lucene syntax.
        let query = iswc_query("T-034.524.680-1");
        assert!(query.contains(r#"iswc:"T-034.524.680-1""#));
    }

    // -----------------------------------------------------------------------
    // search_params / lookup_params
    // -----------------------------------------------------------------------

    #[test]
    fn search_params_omits_offset_when_zero() {
        let params = search_params("recording:Numb", 10, 0);
        assert_eq!(
            params,
            vec![
                ("query", "recording:Numb".to_owned()),
                ("limit", "10".to_owned()),
                ("fmt", "json".to_owned()),
            ]
        );
    }

    #[test]
    fn search_params_includes_offset_when_nonzero() {
        let params = search_params("recording:Numb", 10, 250);
        assert_eq!(
            params,
            vec![
                ("query", "recording:Numb".to_owned()),
                ("limit", "10".to_owned()),
                ("fmt", "json".to_owned()),
                ("offset", "250".to_owned()),
            ]
        );
    }

    #[test]
    fn lookup_params_includes_inc_when_present() {
        let params = lookup_params(Some("artist-credits+releases"));
        assert_eq!(
            params,
            vec![
                ("fmt", "json".to_owned()),
                ("inc", "artist-credits+releases".to_owned()),
            ]
        );
    }

    #[test]
    fn lookup_params_omits_inc_when_absent() {
        let params = lookup_params(None);
        assert_eq!(params, vec![("fmt", "json".to_owned())]);
    }

    // -----------------------------------------------------------------------
    // search_url / lookup_url
    // -----------------------------------------------------------------------

    #[test]
    fn search_url_without_trailing_slash() {
        assert_eq!(
            search_url("https://musicbrainz.org", MbEntity::Recording),
            "https://musicbrainz.org/ws/2/recording/"
        );
    }

    #[test]
    fn search_url_with_trailing_slash() {
        assert_eq!(
            search_url("https://musicbrainz.org/", MbEntity::Recording),
            "https://musicbrainz.org/ws/2/recording/"
        );
    }

    #[test]
    fn lookup_url_for_isrc_entity() {
        assert_eq!(
            lookup_url("https://musicbrainz.org", MbEntity::Isrc, "GBAYE0601498"),
            "https://musicbrainz.org/ws/2/isrc/GBAYE0601498"
        );
    }

    // -----------------------------------------------------------------------
    // de_score (via MbRecording round-trips)
    // -----------------------------------------------------------------------

    #[test]
    fn score_deserializes_from_int() {
        let rec: MbRecording = serde_json::from_str(r#"{"score": 100}"#).unwrap();
        assert_eq!(rec.score, Some(100.0));
    }

    #[test]
    fn score_deserializes_from_float() {
        let rec: MbRecording = serde_json::from_str(r#"{"score": 87.5}"#).unwrap();
        assert_eq!(rec.score, Some(87.5));
    }

    #[test]
    fn score_deserializes_from_numeric_string() {
        let rec: MbRecording = serde_json::from_str(r#"{"score": "93"}"#).unwrap();
        assert_eq!(rec.score, Some(93.0));
    }

    #[test]
    fn score_absent_is_none() {
        let rec: MbRecording = serde_json::from_str(r"{}").unwrap();
        assert_eq!(rec.score, None);
    }

    #[test]
    fn score_null_is_none() {
        let rec: MbRecording = serde_json::from_str(r#"{"score": null}"#).unwrap();
        assert_eq!(rec.score, None);
    }

    // -----------------------------------------------------------------------
    // Response parsing — shape tolerance
    // -----------------------------------------------------------------------

    #[test]
    fn empty_object_parses_to_empty_recordings() {
        let resp: models::MbRecordingSearchResponse = serde_json::from_str("{}").unwrap();
        assert!(resp.recordings.is_empty());
        assert_eq!(resp.count, None);
        assert_eq!(resp.offset, None);
    }

    #[test]
    fn recording_missing_optional_arrays_parses() {
        // Only `id` and `title` present — artist-credit, releases, and isrcs
        // are all absent from the JSON entirely.
        let rec: MbRecording =
            serde_json::from_str(r#"{"id": "abc-123", "title": "Untitled"}"#).unwrap();
        assert_eq!(rec.id.as_deref(), Some("abc-123"));
        assert!(rec.artist_credit.is_empty());
        assert!(rec.releases.is_empty());
        assert!(rec.isrcs.is_empty());
    }

    #[test]
    fn unknown_extra_fields_are_tolerated() {
        // A field MusicBrainz sends that we don't map (e.g. "disambiguation")
        // must not break parsing, since deny_unknown_fields is never applied.
        let rec: MbRecording = serde_json::from_str(
            r#"{"id": "abc-123", "disambiguation": "live version", "video": false}"#,
        )
        .unwrap();
        assert_eq!(rec.id.as_deref(), Some("abc-123"));
    }

    // -----------------------------------------------------------------------
    // recording_to_result
    // -----------------------------------------------------------------------

    #[test]
    fn recording_to_result_full_fixture_matches_expected_mapping() {
        let rec = MbRecording {
            id: Some("mbid-1".into()),
            title: Some("Comfortably Numb".into()),
            artist_credit: vec![
                MbArtistCredit {
                    artist: Some(models::MbArtist {
                        name: Some("Pink Floyd".into()),
                    }),
                },
                MbArtistCredit {
                    artist: Some(models::MbArtist {
                        name: Some("David Gilmour".into()),
                    }),
                },
            ],
            releases: vec![MbRelease {
                title: Some("The Wall".into()),
                date: Some("1979-11-30".into()),
                track_count: Some(26),
            }],
            isrcs: vec!["GBAYE7900123".into()],
            length: Some(384_000), // 384 seconds, in ms
            score: Some(100.0),
        };

        let result = recording_to_result("musicbrainz", rec);

        assert_eq!(result.provider, "musicbrainz");
        assert_eq!(result.provider_id, "mbid-1");
        assert_eq!(result.title.as_deref(), Some("Comfortably Numb"));
        assert_eq!(result.artist.as_deref(), Some("Pink Floyd; David Gilmour"));
        assert_eq!(result.album.as_deref(), Some("The Wall"));
        assert_eq!(result.year, Some(1979));
        assert_eq!(result.isrc.as_deref(), Some("GBAYE7900123"));
        assert_eq!(result.duration_secs, Some(384.0));
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn recording_to_result_clamps_out_of_range_score() {
        let rec = MbRecording {
            score: Some(150.0),
            ..Default::default()
        };
        let result = recording_to_result("musicbrainz", rec);
        assert_eq!(result.score, 1.0);
    }

    #[test]
    fn recording_to_result_no_score_defaults_to_zero() {
        let rec = MbRecording::default();
        let result = recording_to_result("musicbrainz", rec);
        assert_eq!(result.score, 0.0);
    }

    // -----------------------------------------------------------------------
    // work_composer
    // -----------------------------------------------------------------------

    #[test]
    fn work_composer_finds_composer_relation() {
        let relations = vec![
            MbRelation {
                rel_type: Some("lyricist".into()),
                artist: Some(MbRelArtist {
                    name: Some("Lyricist Name".into()),
                }),
            },
            MbRelation {
                rel_type: Some("composer".into()),
                artist: Some(MbRelArtist {
                    name: Some("Composer Name".into()),
                }),
            },
        ];
        assert_eq!(work_composer(&relations), Some("Composer Name".to_owned()));
    }

    #[test]
    fn work_composer_none_when_absent() {
        let relations = vec![MbRelation {
            rel_type: Some("lyricist".into()),
            artist: Some(MbRelArtist {
                name: Some("Lyricist Name".into()),
            }),
        }];
        assert_eq!(work_composer(&relations), None);
    }

    // -----------------------------------------------------------------------
    // host_key
    // -----------------------------------------------------------------------

    #[test]
    fn host_key_default_port_matches_explicit_port() {
        let default = host_key("https://musicbrainz.org/ws/2/recording/");
        let explicit = host_key("https://musicbrainz.org:443/ws/2/recording/");
        assert_eq!(default, explicit);
    }

    #[test]
    fn host_key_ignores_path() {
        let a = host_key("https://musicbrainz.org/ws/2/recording/");
        let b = host_key("https://musicbrainz.org/ws/2/work/?query=x");
        assert_eq!(a, b);
    }

    #[test]
    fn host_key_differs_across_ports() {
        let a = host_key("http://127.0.0.1:8081/ws/2/recording/");
        let b = host_key("http://127.0.0.1:8082/ws/2/recording/");
        assert_ne!(a, b);
    }

    // -----------------------------------------------------------------------
    // mb_limiter_for
    // -----------------------------------------------------------------------

    #[test]
    fn mb_limiter_for_same_base_returns_same_arc() {
        let a = mb_limiter_for("https://mb-limiter-test-1.example/ws/2/recording/");
        let b = mb_limiter_for("https://mb-limiter-test-1.example/ws/2/work/");
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn mb_limiter_for_different_ports_returns_different_arc() {
        let a = mb_limiter_for("http://mb-limiter-test-2.example:9001/ws/2/recording/");
        let b = mb_limiter_for("http://mb-limiter-test-2.example:9002/ws/2/recording/");
        assert!(!Arc::ptr_eq(&a, &b));
    }

    // -----------------------------------------------------------------------
    // ensure_contact
    // -----------------------------------------------------------------------

    #[test]
    fn ensure_contact_leaves_empty_unchanged() {
        assert_eq!(ensure_contact(String::new()), "");
    }

    #[test]
    fn ensure_contact_leaves_email_bearing_ua_unchanged() {
        let ua = "MeedyaManager/1.3.0 (Linux; x86_64) ( me@example.com )".to_owned();
        assert_eq!(ensure_contact(ua.clone()), ua);
    }

    #[test]
    fn ensure_contact_leaves_url_bearing_ua_unchanged() {
        let ua = "MeedyaManager/1.3.0 (Linux; x86_64) ( https://example.com )".to_owned();
        assert_eq!(ensure_contact(ua.clone()), ua);
    }

    #[test]
    fn ensure_contact_appends_contact_when_missing() {
        let ua = "MeedyaManager/1.3.0 (Linux; x86_64)".to_owned();
        let result = ensure_contact(ua.clone());
        assert!(result.starts_with(&ua));
        assert!(result.contains(" ( "));
        assert!(result.ends_with(')'));
    }

    // -----------------------------------------------------------------------
    // mb_get — wiremock integration tests
    // -----------------------------------------------------------------------

    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn mb_get_success_sends_expected_headers_and_returns_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .and(header("Accept", "application/json"))
            .and(header("User-Agent", "test-agent/1.0"))
            .and(query_param("fmt", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"recordings":[]}"#))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let url = search_url(&server.uri(), MbEntity::Recording);
        let params = search_params("recording:Numb", 10, 0);

        let body = mb_get(&client, "musicbrainz", "test-agent/1.0", &url, &params)
            .await
            .expect("mb_get should succeed against a 200 mock");
        assert_eq!(body, r#"{"recordings":[]}"#);
    }

    #[tokio::test]
    async fn mb_get_429_with_retry_after_then_200_succeeds_after_delay() {
        let server = MockServer::start().await;
        // First call: 429 with a short Retry-After, only ever matched once.
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        // Every subsequent call: success.
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let url = search_url(&server.uri(), MbEntity::Recording);
        let params = search_params("x", 1, 0);

        let start = std::time::Instant::now();
        let body = mb_get(&client, "musicbrainz", "ua", &url, &params)
            .await
            .expect("mb_get should retry once and succeed");
        let elapsed = start.elapsed();

        assert_eq!(body, "ok");
        // Generous LOWER bound only: the mandated 1s Retry-After sleep must
        // actually have happened. No upper bound — CI machines vary.
        assert!(
            elapsed >= std::time::Duration::from_millis(900),
            "expected at least ~1s elapsed for the Retry-After delay, got {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn mb_get_429_without_retry_after_is_rate_limited() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let url = search_url(&server.uri(), MbEntity::Recording);
        let params = search_params("x", 1, 0);

        let err = mb_get(&client, "musicbrainz", "ua", &url, &params)
            .await
            .expect_err("mb_get should fail without a usable Retry-After");
        match err {
            ProviderError::RateLimited { provider } => assert_eq!(provider, "musicbrainz"),
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn mb_get_503_with_retry_after_then_200_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(503).insert_header("Retry-After", "1"))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("recovered"))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let url = search_url(&server.uri(), MbEntity::Recording);
        let params = search_params("x", 1, 0);

        let body = mb_get(&client, "musicbrainz", "ua", &url, &params)
            .await
            .expect("mb_get should retry once after a 503 and succeed");
        assert_eq!(body, "recovered");
    }

    #[tokio::test]
    async fn mb_get_404_is_network_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/isrc/DOESNOTEXIST"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let url = lookup_url(&server.uri(), MbEntity::Isrc, "DOESNOTEXIST");
        let params = lookup_params(None);

        let err = mb_get(&client, "isrc", "ua", &url, &params)
            .await
            .expect_err("mb_get should surface a 404 as a Network error");
        assert!(matches!(err, ProviderError::Network(_)));
    }

    #[tokio::test]
    async fn mb_get_concurrent_calls_share_one_rate_limit_bucket() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ws/2/recording/"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let url = search_url(&server.uri(), MbEntity::Recording);
        let params = search_params("x", 1, 0);

        let start = std::time::Instant::now();
        // Two concurrent requests against the SAME host — with burst=1 and
        // 60 RPM (1/sec), the second must wait behind the first rather than
        // both firing immediately, proving they share one token bucket.
        let (first, second) = tokio::join!(
            mb_get(&client, "musicbrainz", "ua", &url, &params),
            mb_get(&client, "musicbrainz", "ua", &url, &params),
        );
        let elapsed = start.elapsed();

        assert!(first.is_ok());
        assert!(second.is_ok());
        assert!(
            elapsed >= std::time::Duration::from_millis(900),
            "expected the second call to queue behind the shared bucket for ~1s, got {elapsed:?}"
        );
    }
}
