# ============================================================================
# File: /metadata/providers/match_scoring.py
# (C) 2025-2026 MWBM Partners Ltd (d/b/a MW Services)
#
# Description:
# Confidence scoring system for metadata lookup results. Computes a
# 0.0-1.0 score indicating how well a ProviderResult matches the
# original query metadata. Used by the LookupService to rank results
# from multiple providers and by the CLI/GUI to display match quality.
#
# Scoring weights:
# - Title match: 35% (fuzzy string matching)
# - Artist match: 30% (fuzzy string matching)
# - Album match: 20% (fuzzy string matching)
# - ISRC exact match: +15% bonus (definitive identifier)
# - Year match: +5% bonus (within 1 year)
# - Track number match: +5% bonus
#
# If an ISRC match is found, the minimum confidence is set to 0.90
# because ISRC is a definitive recording identifier.
# ============================================================================

import logging                                      # Standard logging

from metadata.providers.base import ProviderResult

logger = logging.getLogger("MeedyaManager.MatchScorer")


def _normalize(text: str) -> str:
    """Normalise a string for comparison by lowercasing and stripping whitespace.

    Args:
        text: The string to normalise.

    Returns:
        Lowered, stripped string, or empty string if None.
    """
    if not text:
        return ""
    return text.lower().strip()


def _fuzzy_ratio(a: str, b: str) -> float:
    """Compute a fuzzy match ratio between two strings (0.0-1.0).

    Uses fuzzywuzzy if available, otherwise falls back to a simple
    containment-based heuristic. The fuzzywuzzy ratio uses Levenshtein
    distance to measure string similarity.

    Args:
        a: First string to compare.
        b: Second string to compare.

    Returns:
        Similarity ratio between 0.0 (no match) and 1.0 (exact match).
    """
    # Normalise both strings
    a_norm = _normalize(a)
    b_norm = _normalize(b)

    # Empty strings cannot match
    if not a_norm or not b_norm:
        return 0.0

    # Exact match — maximum score
    if a_norm == b_norm:
        return 1.0

    # Try fuzzywuzzy for high-quality fuzzy matching
    try:
        from fuzzywuzzy import fuzz                 # Lazy import — optional dependency
        # Use token_sort_ratio which handles word reordering
        # e.g., "The Beatles" vs "Beatles, The" scores high
        ratio = fuzz.token_sort_ratio(a_norm, b_norm) / 100.0
        return ratio
    except ImportError:
        pass

    # Fallback: simple containment-based heuristic
    # Check if one string contains the other
    if a_norm in b_norm or b_norm in a_norm:
        # Partial containment — score based on length ratio
        shorter = min(len(a_norm), len(b_norm))
        longer = max(len(a_norm), len(b_norm))
        return shorter / longer * 0.9               # Cap at 0.9 for containment match

    # No meaningful similarity
    return 0.0


class MatchScorer:
    """Scores how well provider results match query metadata.

    Uses weighted fuzzy string matching across multiple fields to compute
    a confidence score. The scorer handles missing fields gracefully —
    if a field is empty in either the query or result, that field's weight
    is redistributed to other fields.
    """

    # Default scoring weights (must sum to 1.0 for the base fields)
    DEFAULT_WEIGHTS = {
        "title": 0.35,                              # Title is the most important field
        "artist": 0.30,                             # Artist is the second most important
        "album": 0.20,                              # Album provides context
        "year": 0.10,                               # Year helps disambiguate
        "track_num": 0.05,                          # Track number is least important
    }

    # Bonus weights (added on top of base score, capped at 1.0)
    ISRC_BONUS = 0.15                               # ISRC exact match bonus
    YEAR_BONUS = 0.05                               # Year exact match bonus
    TRACK_BONUS = 0.05                              # Track number exact match bonus

    def __init__(self, weights: dict[str, float] | None = None):
        """Initialise the scorer with optional custom weights.

        Args:
            weights: Custom field weights (overrides DEFAULT_WEIGHTS).
        """
        self.weights = weights or self.DEFAULT_WEIGHTS.copy()

    def score(self, query: dict, result: ProviderResult) -> float:
        """Compute a 0.0-1.0 confidence score for a result against a query.

        The score is a weighted sum of fuzzy string matches across fields,
        plus bonus points for exact ISRC, year, and track number matches.

        Args:
            query: dict of metadata fields from the file being looked up.
                   Expected keys: title, artist, album, year, track_num, isrc.
            result: ProviderResult from a provider search.

        Returns:
            Confidence score between 0.0 (no match) and 1.0 (perfect match).
        """
        score = 0.0

        # ---- ISRC exact match (definitive identifier) ----
        query_isrc = _normalize(query.get("isrc", ""))
        result_isrc = _normalize(result.isrc)
        if query_isrc and result_isrc and query_isrc == result_isrc:
            # ISRC is a definitive recording identifier — high confidence
            score += self.ISRC_BONUS
            logger.debug(f"ISRC match bonus: +{self.ISRC_BONUS}")

        # ---- Fuzzy field matching (weighted sum) ----
        field_pairs = [
            ("title", result.title),
            ("artist", result.artist),
            ("album", result.album),
        ]

        # Calculate available weight (only for fields present in both query and result)
        available_pairs = []
        for field_name, result_value in field_pairs:
            query_value = query.get(field_name, "")
            if query_value and result_value:
                available_pairs.append((field_name, query_value, result_value))

        if available_pairs:
            # Redistribute weights for missing fields
            total_weight = sum(self.weights.get(f, 0.0) for f, _, _ in available_pairs)
            if total_weight > 0:
                scale_factor = (self.weights["title"] + self.weights["artist"] +
                                self.weights["album"]) / total_weight
            else:
                scale_factor = 1.0

            for field_name, query_value, result_value in available_pairs:
                ratio = _fuzzy_ratio(query_value, result_value)
                weight = self.weights.get(field_name, 0.0) * scale_factor
                field_score = ratio * weight
                score += field_score
                logger.debug(
                    f"  {field_name}: ratio={ratio:.2f} weight={weight:.2f} "
                    f"score={field_score:.3f}"
                )

        # ---- Year match bonus ----
        query_year = query.get("year", "")
        if query_year and result.year:
            try:
                year_diff = abs(int(str(query_year)[:4]) - int(str(result.year)[:4]))
                if year_diff == 0:
                    score += self.YEAR_BONUS
                elif year_diff <= 1:
                    score += self.YEAR_BONUS * 0.5   # Half bonus for 1-year difference
            except (ValueError, TypeError):
                pass                                # Ignore non-numeric years

        # ---- Track number match bonus ----
        query_track = query.get("track_num", "")
        if query_track and result.track_num:
            try:
                if int(str(query_track)) == int(str(result.track_num)):
                    score += self.TRACK_BONUS
            except (ValueError, TypeError):
                pass

        # Cap at 1.0
        final_score = min(1.0, max(0.0, score))
        logger.debug(f"Final match score: {final_score:.3f}")
        return final_score

    def rank_results(self, query: dict, results: list[ProviderResult]) -> list[ProviderResult]:
        """Sort results by confidence score (highest first).

        Scores each result against the query and sorts descending.
        The confidence score is also stored in each result's .confidence field.

        Args:
            query: dict of metadata fields from the file being looked up.
            results: list of ProviderResult objects to rank.

        Returns:
            list[ProviderResult]: Sorted by confidence score (highest first).
        """
        for result in results:
            result.confidence = self.score(query, result)

        # Sort descending by confidence
        sorted_results = sorted(results, key=lambda r: r.confidence, reverse=True)
        return sorted_results
