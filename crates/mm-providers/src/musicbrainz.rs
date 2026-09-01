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
// NOTE: the shared rate limiter and request executor (`mb_limiter_for()`,
// `ensure_contact()`, `mb_get()`) land in a follow-up change on top of this
// one — see issue #198 for the full sequencing.
//
// Public API summary:
//   MB_DEFAULT_BASE_URL          — tunable base-URL constant
//   MbEntity                     — recording / work / isrc
//   search_url() / lookup_url()  — endpoint URL builders
//   search_params() / lookup_params() — query-string builders
//   lucene_escape() / lucene_phrase() — Lucene syntax helpers
//   normalise_isrc() / normalise_iswc() — identifier canonicalisation
//   recording_query() / isrc_query() / iswc_query() — full Lucene queries
//   models::*                    — response structs + mapping helpers

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
    let mut parts = Vec::new();
    if let Some(title) = title {
        parts.push(format!("recording:{}", lucene_phrase(title)));
    }
    if let Some(artist) = artist {
        parts.push(format!("artistname:{}", lucene_phrase(artist)));
    }
    if parts.is_empty() {
        lucene_escape(free_text)
    } else {
        parts.join(" AND ")
    }
}

/// Build the Lucene query for an ISRC-based recording search:
/// `isrc:<normalised-isrc>`.
pub fn isrc_query(isrc: &str) -> String {
    format!("isrc:{}", normalise_isrc(isrc))
}

/// Build the Lucene query for an ISWC-based work search:
/// `iswc:<normalised-iswc>`.
pub fn iswc_query(iswc: &str) -> String {
    format!("iswc:{}", normalise_iswc(iswc))
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
    // normalise_isrc / normalise_iswc / isrc_query / iswc_query
    // -----------------------------------------------------------------------

    #[test]
    fn isrc_query_normalises_and_prefixes() {
        assert_eq!(isrc_query("gb-aye-06-01498"), "isrc:GBAYE0601498");
    }

    #[test]
    fn iswc_query_normalises_and_prefixes() {
        assert_eq!(iswc_query("T-034.524.680-1"), "iswc:T0345246801");
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
}
