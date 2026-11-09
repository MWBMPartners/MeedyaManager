// (C) 2025-2026 MWBM Partners Ltd
//
// MeedyaManager — Match Scoring (#133 migration)
//
// Phase 3 of the MeedyaSuite-core integration epic. The 652-line local
// scorer is replaced by re-exports from `meedya_core::providers::match_scoring`.
//
// Re-exported upstream items:
//   - `MatchScorer`     — fuzzy match scorer (constructed with `new(weights)`
//                         or `default()` for the recommended weights)
//   - `ScoringWeights`  — title 0.35 / artist 0.30 / album 0.20 / year 0.10
//                         / isrc 0.05
//
// LOCAL-ONLY ADDITIONS:
//   - `rank_results()`         — free function to score and sort a slice of
//                                results in-place. Replacement for the
//                                previous `MatchScorer::rank_results()` method,
//                                which isn't on the upstream type.
//   - `score_result()`         — free function = `MatchScorer::default().score()`,
//                                kept for backward compatibility.
//   - `MmScoringWeightsExt`    — extension trait with `is_valid()` that checks
//                                weights sum to 1.0 (was a method locally).
//
// BEHAVIOUR DRIFT documented:
//   - The upstream `score()` SKIPS a component when either side is `None`
//     (so missing fields don't drag the score down). The previous local
//     implementation applied 0.5 neutrals / 0.0–0.3 penalties for missing
//     fields. Code paths that relied on missing-field penalties may see
//     marginally higher scores after this migration.
//   - The previous `MatchScorer::fuzzy_ratio()` and `with_weights()`
//     helpers are gone (the upstream API is `new(weights)` / `default()`
//     and exposes only `score()` publicly).

use crate::traits::{ProviderResult, SearchQuery};

// Re-exports — primary surface from upstream.
pub use meedya_core::providers::match_scoring::{MatchScorer, ScoringWeights};

// ---------------------------------------------------------------------------
// Local-only extension: ScoringWeights validity check
// ---------------------------------------------------------------------------

/// Extension trait on the upstream `ScoringWeights` adding the previous
/// `is_valid()` sanity check.
pub trait MmScoringWeightsExt {
    /// Returns `true` if the weights sum to approximately 1.0
    /// (within `1e-6` floating-point tolerance).
    fn is_valid(&self) -> bool;
}

impl MmScoringWeightsExt for ScoringWeights {
    fn is_valid(&self) -> bool {
        let sum = self.title + self.artist + self.album + self.year + self.isrc;
        (sum - 1.0).abs() < 1e-6
    }
}

// ---------------------------------------------------------------------------
// Module-level convenience functions
// ---------------------------------------------------------------------------

/// Score a result against a query using default weights.
///
/// Equivalent to `MatchScorer::default().score(query, result)`. Kept as a
/// free function so the previous call-site shape continues to work.
pub fn score_result(query: &SearchQuery, result: &ProviderResult) -> f64 {
    MatchScorer::default().score(query, result)
}

/// Score every result in a list and sort highest-first.
///
/// Equivalent to the previous `MatchScorer::rank_results()` method. The
/// upstream `MatchScorer` only exposes `score()`, so we do the loop and
/// sort here.
pub fn rank_results(query: &SearchQuery, results: &mut [ProviderResult]) {
    let scorer = MatchScorer::default();
    for r in results.iter_mut() {
        r.score = scorer.score(query, r);
    }
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Score every result in a list using a custom scorer and sort highest-first.
///
/// Local-only helper used by the registry so it can reuse a single scorer
/// instance across calls.
pub fn rank_results_with(
    scorer: &MatchScorer,
    query: &SearchQuery,
    results: &mut [ProviderResult],
) {
    for r in results.iter_mut() {
        r.score = scorer.score(query, r);
    }
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

// ---------------------------------------------------------------------------
// Tests — local-only behaviour (upstream tests live upstream)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{ProviderResult, music_query};

    fn make_result(title: &str, artist: &str) -> ProviderResult {
        let mut r = ProviderResult::new("test");
        r.title = Some(title.into());
        r.artist = Some(artist.into());
        r
    }

    // --- MmScoringWeightsExt::is_valid ---

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

    // --- MatchScorer construction (upstream API) ---

    #[test]
    fn scorer_default_uses_default_weights() {
        let _scorer = MatchScorer::default();
        // No public accessor; verify by scoring an exact match.
        let query = music_query("Track", "Artist");
        let result = make_result("Track", "Artist");
        let score = MatchScorer::default().score(&query, &result);
        assert!(score > 0.9);
    }

    #[test]
    fn scorer_new_with_custom_weights() {
        let weights = ScoringWeights {
            title: 0.40,
            artist: 0.30,
            album: 0.15,
            year: 0.10,
            isrc: 0.05,
        };
        let scorer = MatchScorer::new(weights);
        // Custom weights also produce a valid score
        let query = music_query("Track", "Artist");
        let result = make_result("Track", "Artist");
        let score = scorer.score(&query, &result);
        assert!(score > 0.9);
    }

    // --- score() — upstream behaviour ---

    #[test]
    fn score_exact_match_title_and_artist() {
        let scorer = MatchScorer::default();
        let query = music_query("Comfortably Numb", "Pink Floyd");
        let result = make_result("Comfortably Numb", "Pink Floyd");
        let score = scorer.score(&query, &result);
        assert!(score > 0.80, "score={score}");
    }

    #[test]
    fn score_wrong_artist_reduces_score() {
        let scorer = MatchScorer::default();
        let query = music_query("Comfortably Numb", "Pink Floyd");
        let correct_artist = make_result("Comfortably Numb", "Pink Floyd");
        let wrong_artist = make_result("Comfortably Numb", "Radiohead");

        let s1 = scorer.score(&query, &correct_artist);
        let s2 = scorer.score(&query, &wrong_artist);
        assert!(s1 > s2, "correct={s1} should be > wrong={s2}");
    }

    #[test]
    fn score_wrong_title_reduces_score() {
        let scorer = MatchScorer::default();
        let query = music_query("Comfortably Numb", "Pink Floyd");
        let correct = make_result("Comfortably Numb", "Pink Floyd");
        let wrong = make_result("Money", "Pink Floyd");
        assert!(scorer.score(&query, &correct) > scorer.score(&query, &wrong));
    }

    #[test]
    fn score_isrc_exact_match_bonus() {
        let scorer = MatchScorer::default();
        let mut query = music_query("Track", "Artist");
        query.isrc = Some("GBAYE0601498".into());

        let mut with_isrc = make_result("Track", "Artist");
        with_isrc.isrc = Some("GBAYE0601498".into());

        let mut without_isrc = make_result("Track", "Artist");
        without_isrc.isrc = None;

        // ISRC match should score >= no-ISRC (upstream skips the component
        // when either side is None, so without_isrc has no penalty; with_isrc
        // exact match contributes positively).
        assert!(
            scorer.score(&query, &with_isrc) >= scorer.score(&query, &without_isrc),
            "ISRC match should not reduce score"
        );
    }

    #[test]
    fn score_year_match_bonus() {
        let scorer = MatchScorer::default();
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
        let scorer = MatchScorer::default();
        let query = music_query("Some Track", "Some Artist");
        let result = make_result("Completely Different", "Nothing In Common");
        let s = scorer.score(&query, &result);
        assert!((0.0..=1.0).contains(&s), "score={s}");
    }

    // --- rank_results (local free fn) ---

    #[test]
    fn rank_results_sorts_highest_first() {
        let query = music_query("Comfortably Numb", "Pink Floyd");

        let mut results = vec![
            make_result("Money", "Pink Floyd"),            // wrong title
            make_result("Comfortably Numb", "Pink Floyd"), // exact match
            make_result("Bohemian Rhapsody", "Queen"),     // different artist
        ];
        rank_results(&query, &mut results);

        assert_eq!(results[0].title.as_deref(), Some("Comfortably Numb"));
    }

    #[test]
    fn rank_results_attaches_scores_to_results() {
        let query = music_query("Track", "Artist");
        let mut results = vec![make_result("Track", "Artist")];
        rank_results(&query, &mut results);
        assert!(results[0].score > 0.0, "Score should be attached");
    }

    #[test]
    fn rank_results_with_custom_scorer() {
        let query = music_query("Track", "Artist");
        let mut results = vec![
            make_result("Other", "Artist"),
            make_result("Track", "Artist"),
        ];
        let scorer = MatchScorer::default();
        rank_results_with(&scorer, &query, &mut results);
        assert_eq!(results[0].title.as_deref(), Some("Track"));
    }

    // --- score_result convenience function ---

    #[test]
    fn convenience_score_result_matches_scorer() {
        let query = music_query("Track", "Artist");
        let result = make_result("Track", "Artist");
        let conv = score_result(&query, &result);
        let direct = MatchScorer::default().score(&query, &result);
        assert!((conv - direct).abs() < 1e-9);
    }
}
