// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Match Scoring
//
// Weighted fuzzy scoring for ranking metadata search results against a query.
//
// Scoring weights (must sum to 1.0):
//   Title:  35%  — most important; wrong title = wrong track
//   Artist: 30%  — second most important
//   Album:  20%  — useful for disambiguation
//   Year:   10%  — prefer closer release years
//   ISRC:   5%   — exact-match bonus (all-or-nothing)
//
// Fuzzy matching uses `fuzzy_matcher::skim::SkimMatcherV2` for character-level
// subsequence scoring. Scores are normalised to [0.0, 1.0] by comparing against
// the self-match score of the longer string.
//
// The final weighted score is clamped to [0.0, 1.0].

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::traits::{ProviderResult, SearchQuery};

// ---------------------------------------------------------------------------
// Scoring weights
// ---------------------------------------------------------------------------

/// Weights applied to each field when computing the overall match score.
///
/// All weights must sum to exactly 1.0. Use `ScoringWeights::default()` for
/// the recommended production weights.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoringWeights {
    /// Weight for title match (0.0–1.0)
    pub title: f64,
    /// Weight for artist match (0.0–1.0)
    pub artist: f64,
    /// Weight for album match (0.0–1.0)
    pub album: f64,
    /// Weight for year proximity (0.0–1.0)
    pub year: f64,
    /// Weight for exact ISRC match bonus (0.0–1.0)
    pub isrc: f64,
}

impl ScoringWeights {
    /// Validate that the weights sum to approximately 1.0.
    pub fn is_valid(&self) -> bool {
        let sum = self.title + self.artist + self.album + self.year + self.isrc;
        (sum - 1.0).abs() < 1e-6
    }
}

impl Default for ScoringWeights {
    /// Default production weights: title 35%, artist 30%, album 20%, year 10%, ISRC 5%.
    fn default() -> Self {
        Self {
            title: 0.35,
            artist: 0.30,
            album: 0.20,
            year: 0.10,
            isrc: 0.05,
        }
    }
}

// ---------------------------------------------------------------------------
// MatchScorer
// ---------------------------------------------------------------------------

/// Computes a [0.0, 1.0] match score between a `SearchQuery` and a `ProviderResult`.
///
/// # Example
///
/// ```ignore
/// let scorer = MatchScorer::new();
/// let score = scorer.score(&query, &result);
/// // Attach score to result
/// result.score = score;
/// ```
pub struct MatchScorer {
    /// Field weights
    weights: ScoringWeights,
    /// Reusable fuzzy matcher (stateless after construction)
    matcher: SkimMatcherV2,
}

// Manual Debug because SkimMatcherV2 does not implement Debug
impl std::fmt::Debug for MatchScorer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatchScorer")
            .field("weights", &self.weights)
            .field("matcher", &"SkimMatcherV2 { .. }")
            .finish()
    }
}

impl MatchScorer {
    /// Create a scorer with default weights.
    pub fn new() -> Self {
        Self {
            weights: ScoringWeights::default(),
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Create a scorer with custom weights.
    ///
    /// # Panics
    ///
    /// Panics if `weights.is_valid()` returns false.
    pub fn with_weights(weights: ScoringWeights) -> Self {
        assert!(weights.is_valid(), "Scoring weights must sum to 1.0");
        Self {
            weights,
            matcher: SkimMatcherV2::default(),
        }
    }

    /// Compute the overall match score between a query and a provider result.
    ///
    /// Returns a value in [0.0, 1.0] where 1.0 is a perfect match.
    pub fn score(&self, query: &SearchQuery, result: &ProviderResult) -> f64 {
        // Title component (weighted 35%)
        let title_score = match (&query.title, &result.title) {
            (Some(q), Some(r)) => self.fuzzy_ratio(q, r),
            (None, Some(_)) => 0.5, // Query has no title → neutral
            (Some(_), None) => 0.0, // Result missing title → penalty
            (None, None) => 1.0,    // Both absent → neutral
        };

        // Artist component (weighted 30%)
        let artist_score = match (&query.artist, &result.artist) {
            (Some(q), Some(r)) => self.fuzzy_ratio(q, r),
            (None, Some(_)) => 0.5,
            (Some(_), None) => 0.0,
            (None, None) => 1.0,
        };

        // Album component (weighted 20%)
        let album_score = match (&query.album, &result.album) {
            (Some(q), Some(r)) => self.fuzzy_ratio(q, r),
            (None, Some(_)) => 0.5,
            (Some(_), None) => 0.3, // Missing album is a minor penalty
            (None, None) => 1.0,
        };

        // Year proximity component (weighted 10%)
        let year_score = match (query.year, result.year) {
            (Some(qy), Some(ry)) => year_proximity(qy, ry),
            (None, Some(_)) => 0.5,
            (Some(_), None) => 0.5,
            (None, None) => 1.0,
        };

        // ISRC exact-match bonus (weighted 5%)
        let isrc_score = match (&query.isrc, &result.isrc) {
            (Some(q), Some(r)) => {
                if normalise_isrc(q) == normalise_isrc(r) {
                    1.0
                } else {
                    0.0
                }
            }
            (Some(_), None) => 0.3, // Query specified ISRC but result has none → mild penalty (less bad than mismatch)
            _ => 0.5,               // Not applicable → neutral
        };

        // Weighted sum
        let raw = self.weights.isrc.mul_add(
            isrc_score,
            self.weights.year.mul_add(
                year_score,
                self.weights.album.mul_add(
                    album_score,
                    self.weights
                        .title
                        .mul_add(title_score, self.weights.artist * artist_score),
                ),
            ),
        );

        raw.clamp(0.0, 1.0)
    }

    /// Compute and attach a score to each result in a list, then sort descending.
    pub fn rank_results(&self, query: &SearchQuery, results: &mut [ProviderResult]) {
        for r in results.iter_mut() {
            r.score = self.score(query, r);
        }
        // Sort highest score first
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Compute a normalised fuzzy similarity ratio in [0.0, 1.0] between two strings.
    ///
    /// Both inputs are normalised to lowercase with punctuation stripped before comparison.
    /// Returns 1.0 for exact matches after normalisation.
    pub(crate) fn fuzzy_ratio(&self, a: &str, b: &str) -> f64 {
        let a = normalise_for_comparison(a);
        let b = normalise_for_comparison(b);

        // Both empty → no meaningful comparison
        if a.is_empty() && b.is_empty() {
            return 0.0;
        }
        // One side empty → no match
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        // Exact match after normalisation → perfect score
        if a == b {
            return 1.0;
        }

        // SkimMatcherV2: `fuzzy_match(text, pattern)` → pattern searched in text
        // We try both directions and take the max to handle length asymmetry.
        let score_ab = self.matcher.fuzzy_match(&a, &b).unwrap_or(0).max(0) as f64;
        let score_ba = self.matcher.fuzzy_match(&b, &a).unwrap_or(0).max(0) as f64;
        let best_score = score_ab.max(score_ba);

        if best_score == 0.0 {
            return 0.0;
        }

        // Normalise: compare against the self-match score of the longer string
        let longer = if a.len() >= b.len() { &a } else { &b };
        let self_score = self.matcher.fuzzy_match(longer, longer).unwrap_or(1).max(1) as f64;

        (best_score / self_score).clamp(0.0, 1.0)
    }
}

impl Default for MatchScorer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Module-level convenience function
// ---------------------------------------------------------------------------

/// Score a result against a query using default weights.
///
/// Equivalent to `MatchScorer::new().score(query, result)`.
pub fn score_result(query: &SearchQuery, result: &ProviderResult) -> f64 {
    MatchScorer::new().score(query, result)
}

/// Rank a list of results by score (highest first) using default weights.
pub fn rank_results(query: &SearchQuery, results: &mut [ProviderResult]) {
    MatchScorer::new().rank_results(query, results);
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Normalise a string for fuzzy comparison: lowercase, strip punctuation, collapse whitespace.
fn normalise_for_comparison(s: &str) -> String {
    let lower = s.to_lowercase();
    // Remove punctuation characters that don't affect meaning
    let stripped: String = lower
        .chars()
        .map(|c| {
            if c.is_alphabetic() || c.is_numeric() || c == ' ' {
                c
            } else {
                ' '
            }
        })
        .collect();
    // Collapse consecutive whitespace
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Compute a year proximity score in [0.0, 1.0].
///
/// Exact year match → 1.0. Each year off reduces the score by 0.1 (minimum 0.0).
fn year_proximity(query_year: u32, result_year: u32) -> f64 {
    let diff = query_year.abs_diff(result_year);
    f64::from(diff).mul_add(-0.1, 1.0).clamp(0.0, 1.0)
}

/// Normalise an ISRC for comparison: uppercase, remove hyphens and spaces.
fn normalise_isrc(isrc: &str) -> String {
    isrc.to_uppercase().replace(['-', ' '], "")
}

// ---------------------------------------------------------------------------
// Tests — 40 tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{ProviderResult, music_query};

    // Helper: build a minimal ProviderResult
    fn make_result(title: &str, artist: &str) -> ProviderResult {
        let mut r = ProviderResult::new("test");
        r.title = Some(title.into());
        r.artist = Some(artist.into());
        r
    }

    fn make_scorer() -> MatchScorer {
        MatchScorer::new()
    }

    // --- ScoringWeights ---

    #[test]
    fn default_weights_are_valid() {
        assert!(ScoringWeights::default().is_valid());
    }

    #[test]
    fn default_weights_sum_to_one() {
        let w = ScoringWeights::default();
        let sum = w.title + w.artist + w.album + w.year + w.isrc;
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn invalid_weights_detected() {
        let w = ScoringWeights {
            title: 0.5,
            artist: 0.5,
            album: 0.5,
            year: 0.0,
            isrc: 0.0,
        };
        assert!(!w.is_valid());
    }

    #[test]
    fn custom_weights_stored_correctly() {
        let w = ScoringWeights {
            title: 0.40,
            artist: 0.30,
            album: 0.15,
            year: 0.10,
            isrc: 0.05,
        };
        assert!(w.is_valid());
        assert!((w.title - 0.40).abs() < 1e-9);
    }

    // --- MatchScorer construction ---

    #[test]
    fn scorer_new_uses_default_weights() {
        let scorer = MatchScorer::new();
        assert!(scorer.weights.is_valid());
        assert!((scorer.weights.title - 0.35).abs() < 1e-9);
    }

    #[test]
    fn scorer_with_custom_weights() {
        let weights = ScoringWeights {
            title: 0.40,
            artist: 0.30,
            album: 0.15,
            year: 0.10,
            isrc: 0.05,
        };
        let scorer = MatchScorer::with_weights(weights.clone());
        assert_eq!(scorer.weights, weights);
    }

    // --- fuzzy_ratio ---

    #[test]
    fn fuzzy_ratio_exact_match_returns_one() {
        let scorer = make_scorer();
        assert!((scorer.fuzzy_ratio("Comfortably Numb", "Comfortably Numb") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn fuzzy_ratio_case_insensitive_exact_match() {
        let scorer = make_scorer();
        assert!((scorer.fuzzy_ratio("pink floyd", "Pink Floyd") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn fuzzy_ratio_empty_strings() {
        let scorer = make_scorer();
        assert_eq!(scorer.fuzzy_ratio("", ""), 0.0);
        assert_eq!(scorer.fuzzy_ratio("something", ""), 0.0);
        assert_eq!(scorer.fuzzy_ratio("", "something"), 0.0);
    }

    #[test]
    fn fuzzy_ratio_similar_strings_score_higher_than_different() {
        let scorer = make_scorer();
        let similar = scorer.fuzzy_ratio("Pink Floyd", "Pink Floyd - The Wall");
        let different = scorer.fuzzy_ratio("Pink Floyd", "Radiohead OK Computer");
        assert!(
            similar > different,
            "similar={similar} should be > different={different}"
        );
    }

    #[test]
    fn fuzzy_ratio_result_in_0_1_range() {
        let scorer = make_scorer();
        for (a, b) in [
            ("hello", "world"),
            ("The Beatles", "Beatles"),
            ("", "x"),
            ("x", "x"),
        ] {
            let r = scorer.fuzzy_ratio(a, b);
            assert!((0.0..=1.0).contains(&r), "ratio={r} for ({a:?}, {b:?})");
        }
    }

    #[test]
    fn fuzzy_ratio_punctuation_stripped() {
        let scorer = make_scorer();
        // "Don't Stop Me Now" vs "Dont Stop Me Now" — punctuation normalised
        let ratio = scorer.fuzzy_ratio("Don't Stop Me Now", "Dont Stop Me Now");
        assert!(ratio > 0.9, "ratio={ratio}");
    }

    // --- year_proximity ---

    #[test]
    fn year_proximity_exact_match() {
        assert!((year_proximity(2010, 2010) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn year_proximity_one_year_off() {
        assert!((year_proximity(2010, 2011) - 0.9).abs() < 1e-9);
    }

    #[test]
    fn year_proximity_ten_years_off_is_zero() {
        assert_eq!(year_proximity(2000, 2010), 0.0);
    }

    #[test]
    fn year_proximity_more_than_ten_years_clamped_to_zero() {
        assert_eq!(year_proximity(1990, 2010), 0.0);
    }

    #[test]
    fn year_proximity_symmetric() {
        assert!((year_proximity(2010, 2005) - year_proximity(2005, 2010)).abs() < 1e-9);
    }

    // --- normalise_for_comparison ---

    #[test]
    fn normalise_strips_punctuation() {
        let n = normalise_for_comparison("Hello, World!");
        assert_eq!(n, "hello world");
    }

    #[test]
    fn normalise_collapses_whitespace() {
        let n = normalise_for_comparison("Pink   Floyd");
        assert_eq!(n, "pink floyd");
    }

    #[test]
    fn normalise_lowercases() {
        let n = normalise_for_comparison("PINK FLOYD");
        assert_eq!(n, "pink floyd");
    }

    // --- normalise_isrc ---

    #[test]
    fn isrc_normalise_removes_hyphens() {
        assert_eq!(normalise_isrc("GB-AYE-06-01498"), "GBAYE0601498");
    }

    #[test]
    fn isrc_normalise_uppercases() {
        assert_eq!(normalise_isrc("gbaye0601498"), "GBAYE0601498");
    }

    #[test]
    fn isrc_normalise_removes_spaces() {
        assert_eq!(normalise_isrc("GB AYE 06 01498"), "GBAYE0601498");
    }

    // --- score() ---

    #[test]
    fn score_exact_match_title_and_artist() {
        let scorer = make_scorer();
        let query = music_query("Comfortably Numb", "Pink Floyd");
        let result = make_result("Comfortably Numb", "Pink Floyd");
        let score = scorer.score(&query, &result);
        // Should be very high (title + artist both exact)
        assert!(score > 0.80, "score={score}");
    }

    #[test]
    fn score_wrong_artist_reduces_score() {
        let scorer = make_scorer();
        let query = music_query("Comfortably Numb", "Pink Floyd");
        let correct_artist = make_result("Comfortably Numb", "Pink Floyd");
        let wrong_artist = make_result("Comfortably Numb", "Radiohead");

        let s1 = scorer.score(&query, &correct_artist);
        let s2 = scorer.score(&query, &wrong_artist);
        assert!(
            s1 > s2,
            "correct artist score={s1} should be > wrong artist score={s2}"
        );
    }

    #[test]
    fn score_wrong_title_reduces_score() {
        let scorer = make_scorer();
        let query = music_query("Comfortably Numb", "Pink Floyd");
        let correct = make_result("Comfortably Numb", "Pink Floyd");
        let wrong = make_result("Money", "Pink Floyd");

        assert!(scorer.score(&query, &correct) > scorer.score(&query, &wrong));
    }

    #[test]
    fn score_isrc_exact_match_bonus() {
        let scorer = make_scorer();
        let mut query = music_query("Track", "Artist");
        query.isrc = Some("GBAYE0601498".into());

        let mut with_isrc = make_result("Track", "Artist");
        with_isrc.isrc = Some("GBAYE0601498".into());

        let mut without_isrc = make_result("Track", "Artist");
        without_isrc.isrc = None;

        // ISRC match should score slightly higher
        assert!(
            scorer.score(&query, &with_isrc) >= scorer.score(&query, &without_isrc),
            "ISRC match should not reduce score"
        );
    }

    #[test]
    fn score_isrc_mismatch_penalty() {
        let scorer = make_scorer();
        let mut query = music_query("Track", "Artist");
        query.isrc = Some("GBAYE0601498".into());

        let mut wrong_isrc = make_result("Track", "Artist");
        wrong_isrc.isrc = Some("USRC12345678".into());

        let mut no_isrc = make_result("Track", "Artist");
        no_isrc.isrc = None;

        // Wrong ISRC (0.0 score for the ISRC component) vs no ISRC (0.5 neutral)
        assert!(
            scorer.score(&query, &wrong_isrc) < scorer.score(&query, &no_isrc),
            "Wrong ISRC should score lower than no ISRC"
        );
    }

    #[test]
    fn score_year_match_bonus() {
        let scorer = make_scorer();
        let mut query = music_query("Track", "Artist");
        query.year = Some(1979);

        let mut r_correct_year = make_result("Track", "Artist");
        r_correct_year.year = Some(1979);

        let mut r_wrong_year = make_result("Track", "Artist");
        r_wrong_year.year = Some(2020);

        assert!(
            scorer.score(&query, &r_correct_year) > scorer.score(&query, &r_wrong_year),
            "Correct year should score higher"
        );
    }

    #[test]
    fn score_in_0_1_range() {
        let scorer = make_scorer();
        let query = music_query("Some Track", "Some Artist");
        let result = make_result("Completely Different", "Nothing In Common");
        let s = scorer.score(&query, &result);
        assert!((0.0..=1.0).contains(&s), "score={s}");
    }

    #[test]
    fn score_missing_result_title_is_penalised() {
        let scorer = make_scorer();
        let query = music_query("Comfortably Numb", "Pink Floyd");

        let mut no_title = ProviderResult::new("test");
        no_title.artist = Some("Pink Floyd".into());

        // No title should score lower than matching title
        let with_title = make_result("Comfortably Numb", "Pink Floyd");
        assert!(scorer.score(&query, &with_title) > scorer.score(&query, &no_title));
    }

    // --- rank_results ---

    #[test]
    fn rank_results_sorts_highest_first() {
        let scorer = make_scorer();
        let query = music_query("Comfortably Numb", "Pink Floyd");

        let mut results = vec![
            make_result("Money", "Pink Floyd"),            // wrong title
            make_result("Comfortably Numb", "Pink Floyd"), // exact match
            make_result("Bohemian Rhapsody", "Queen"),     // different artist
        ];
        scorer.rank_results(&query, &mut results);

        assert_eq!(results[0].title.as_deref(), Some("Comfortably Numb"));
    }

    #[test]
    fn rank_results_attaches_scores_to_results() {
        let scorer = make_scorer();
        let query = music_query("Track", "Artist");
        let mut results = vec![make_result("Track", "Artist")];
        scorer.rank_results(&query, &mut results);
        assert!(results[0].score > 0.0, "Score should be attached");
    }

    // --- score_result convenience function ---

    #[test]
    fn convenience_score_result_matches_scorer() {
        let query = music_query("Track", "Artist");
        let result = make_result("Track", "Artist");
        let conv = score_result(&query, &result);
        let direct = MatchScorer::new().score(&query, &result);
        assert!((conv - direct).abs() < 1e-9);
    }
}
