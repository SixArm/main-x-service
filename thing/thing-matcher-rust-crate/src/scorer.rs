//! Scoring algorithms for string similarity and field comparison.
//!
//! This module exposes a small, focused set of similarity primitives that
//! the matching engine composes together. All scores are normalised to the
//! closed interval `[0.0, 1.0]`, where `1.0` means "identical" and `0.0`
//! means "no observable similarity".
//!
//! ## Algorithm choice
//!
//! | Algorithm | Strength | Weakness |
//! |---|---|---|
//! | [`SimilarityAlgorithm::JaroWinkler`] | Good for short strings; rewards a common prefix. | Saturates quickly on long strings. |
//! | [`SimilarityAlgorithm::Levenshtein`] | Cheap to reason about; tracks edit distance. | Sensitive to length differences. |
//! | [`SimilarityAlgorithm::Exact`]       | Fast; defensible to non-technical reviewers. | No tolerance for typos. |
//! | [`SimilarityAlgorithm::Combined`]    | Default; balances JW and Levenshtein. | Bespoke weighting (0.7 JW + 0.3 Lev). |
//!
//! ## Example
//!
//! ```
//! use thing_matcher::Scorer;
//!
//! let same  = Scorer::jaro_winkler_similarity("smith", "smith");
//! let close = Scorer::jaro_winkler_similarity("smith", "smyth");
//! let far   = Scorer::jaro_winkler_similarity("smith", "jones");
//!
//! assert!(same  > 0.99);
//! assert!(close > 0.85);
//! assert!(far   < same);
//! ```

use strsim::{jaro_winkler, levenshtein};

/// Stateless namespace for string-similarity scorers.
///
/// Like [`crate::Normalizer`], `Scorer` is a unit type with no fields;
/// every method is associated.
///
/// ```
/// use thing_matcher::Scorer;
/// assert_eq!(Scorer::exact_match("a", "a"), 1.0);
/// ```
pub struct Scorer;

impl Scorer {
    /// Jaro-Winkler similarity, normalised to `[0.0, 1.0]`.
    ///
    /// Higher values indicate greater similarity. Strings sharing a common
    /// prefix score noticeably higher than strings that diverge at the start.
    ///
    /// # Edge cases
    ///
    /// - Two empty strings → `1.0` (identical).
    /// - One empty, one not → `0.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_matcher::Scorer;
    /// assert!(Scorer::jaro_winkler_similarity("smith", "smith") > 0.99);
    /// assert!(Scorer::jaro_winkler_similarity("smith", "smyth") > 0.85);
    /// assert_eq!(Scorer::jaro_winkler_similarity("", ""), 1.0);
    /// assert_eq!(Scorer::jaro_winkler_similarity("smith", ""), 0.0);
    /// ```
    #[must_use]
    pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f64 {
        // Two empties are "identical"; one empty is maximally dissimilar.
        // These guards also dodge any divide-by-zero in the underlying
        // algorithm for zero-length inputs.
        if s1.is_empty() && s2.is_empty() {
            return 1.0;
        }
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }
        jaro_winkler(s1, s2)
    }

    /// Normalised Levenshtein similarity, in `[0.0, 1.0]`.
    ///
    /// Computed as `1 - (edit_distance / max_len)`. Higher values indicate
    /// greater similarity.
    ///
    /// # Edge cases
    ///
    /// - Two empty strings → `1.0`.
    /// - One empty, one not → `0.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_matcher::Scorer;
    /// assert_eq!(Scorer::levenshtein_similarity("smith", "smith"), 1.0);
    /// assert!(Scorer::levenshtein_similarity("smith", "smyth") >= 0.79);
    /// assert!(Scorer::levenshtein_similarity("abc", "xyz") < 0.5);
    /// assert_eq!(Scorer::levenshtein_similarity("", ""), 1.0);
    /// ```
    #[must_use]
    pub fn levenshtein_similarity(s1: &str, s2: &str) -> f64 {
        if s1.is_empty() && s2.is_empty() {
            return 1.0;
        }
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }

        let distance = levenshtein(s1, s2);
        // Normalise the edit distance by the longer string's byte length so
        // the result lands in `[0.0, 1.0]`. `max_len` is non-zero here
        // because the empty-input cases were handled above.
        let max_len = s1.len().max(s2.len());
        // `distance` and `max_len` are byte counts; convert losslessly via
        // `u32` (saturating at `u32::MAX`) so the ratio stays exact for any
        // realistic string length without an `as` precision-losing cast.
        let distance_f = u32::try_from(distance).map_or(f64::from(u32::MAX), f64::from);
        let max_len_f = u32::try_from(max_len).map_or(f64::from(u32::MAX), f64::from);
        1.0 - (distance_f / max_len_f)
    }

    /// Binary exact-match score: `1.0` if `s1 == s2`, else `0.0`.
    ///
    /// Case-sensitive and whitespace-sensitive. Pair with
    /// [`crate::Normalizer`] when comparing user-entered text.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_matcher::Scorer;
    /// assert_eq!(Scorer::exact_match("test", "test"), 1.0);
    /// assert_eq!(Scorer::exact_match("Test", "test"), 0.0);  // case-sensitive
    /// assert_eq!(Scorer::exact_match("a", "b"),       0.0);
    /// ```
    #[must_use]
    pub fn exact_match(s1: &str, s2: &str) -> f64 {
        if s1 == s2 { 1.0 } else { 0.0 }
    }

    /// Weighted combination of Jaro-Winkler (0.7) and Levenshtein (0.3).
    ///
    /// Defaults are tuned for short names. Jaro-Winkler dominates because
    /// it handles short-string prefix matches better; Levenshtein contributes
    /// stability for longer or rearranged inputs.
    ///
    /// # Example
    ///
    /// ```
    /// use thing_matcher::Scorer;
    ///
    /// let s = Scorer::combined_similarity("Stephen", "Steven");
    /// assert!(s > 0.80, "combined score for Stephen/Steven was {s}");
    /// ```
    #[must_use]
    pub fn combined_similarity(s1: &str, s2: &str) -> f64 {
        let jw = Self::jaro_winkler_similarity(s1, s2);
        let lev = Self::levenshtein_similarity(s1, s2);
        // Fixed 0.7 / 0.3 blend: Jaro-Winkler leads (it handles short-name
        // prefix matches well), Levenshtein moderates (it is steadier on
        // longer or rearranged strings). The weights sum to 1.0 so the
        // result stays in `[0.0, 1.0]`.
        0.7 * jw + 0.3 * lev
    }

    /// Jaccard similarity for two collections of strings, computed as
    /// `|intersection| / |union|`. Returns `1.0` if both collections are
    /// empty, `0.0` if exactly one is empty.
    ///
    /// Useful for `sameAs`, `additionalType`, and `subjectOf` set
    /// comparison. The caller is responsible for pre-normalising each
    /// element (e.g. via [`crate::Normalizer::normalize_url`]).
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_matcher::Scorer;
    /// let a = ["x", "y", "z"];
    /// let b = ["y", "z", "w"];
    /// // intersection = {y, z}, union = {x, y, z, w}
    /// let s = Scorer::jaccard_set_similarity(&a, &b);
    /// assert!((s - 0.5).abs() < 1e-9);
    /// ```
    pub fn jaccard_set_similarity<S, T>(a: &[S], b: &[T]) -> f64
    where
        S: AsRef<str>,
        T: AsRef<str>,
    {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        // Deduplicate each side into a set, so repeated elements do not
        // skew the cardinalities. `BTreeSet` (rather than `HashSet`) keeps
        // the computation fully deterministic.
        let a_set: std::collections::BTreeSet<&str> = a.iter().map(AsRef::as_ref).collect();
        let b_set: std::collections::BTreeSet<&str> = b.iter().map(AsRef::as_ref).collect();
        let intersection = a_set.intersection(&b_set).count();
        let union = a_set.union(&b_set).count();
        // Defensive: `union` can only be zero if both sets are empty, which
        // the guards above already returned for — but keep the check so the
        // division below can never be `0/0`.
        if union == 0 {
            return 0.0;
        }
        // Set cardinalities; convert losslessly via `u32` (saturating at
        // `u32::MAX`) so the Jaccard ratio is exact for any realistic set
        // size without an `as` precision-losing cast.
        let intersection_f = u32::try_from(intersection).map_or(f64::from(u32::MAX), f64::from);
        let union_f = u32::try_from(union).map_or(f64::from(u32::MAX), f64::from);
        intersection_f / union_f
    }

    /// Score two `Option<String>` fields using the chosen algorithm.
    ///
    /// Returns:
    ///
    /// - `1.0` if both are `None` (both absent → trivially "match").
    /// - `0.0` if exactly one is `None` (asymmetric data → "differ").
    /// - The chosen algorithm's similarity if both are `Some`.
    ///
    /// Note: the matching engine intentionally does **not** use this
    /// helper — it skips fields where either side is absent so that they
    /// neither contribute nor penalise. This helper is kept for callers
    /// who want a different policy.
    ///
    /// # Examples
    ///
    /// ```
    /// use thing_matcher::{Scorer, SimilarityAlgorithm};
    ///
    /// let none: Option<String> = None;
    /// let a = Some("hello".to_string());
    /// let b = Some("hello".to_string());
    ///
    /// assert_eq!(Scorer::optional_field_score(&none, &none, SimilarityAlgorithm::Exact), 1.0);
    /// assert_eq!(Scorer::optional_field_score(&a,    &none, SimilarityAlgorithm::Exact), 0.0);
    /// assert_eq!(Scorer::optional_field_score(&a,    &b,    SimilarityAlgorithm::Exact), 1.0);
    /// ```
    #[must_use]
    pub fn optional_field_score(
        field1: &Option<String>,
        field2: &Option<String>,
        algorithm: SimilarityAlgorithm,
    ) -> f64 {
        match (field1, field2) {
            (None, None) => 1.0,
            (None, Some(_)) | (Some(_), None) => 0.0,
            (Some(s1), Some(s2)) => match algorithm {
                SimilarityAlgorithm::JaroWinkler => Self::jaro_winkler_similarity(s1, s2),
                SimilarityAlgorithm::Levenshtein => Self::levenshtein_similarity(s1, s2),
                SimilarityAlgorithm::Exact => Self::exact_match(s1, s2),
                SimilarityAlgorithm::Combined => Self::combined_similarity(s1, s2),
            },
        }
    }
}

/// Algorithm selector for name comparison in [`crate::MatchConfig`].
///
/// The enum is `Copy`, so it is cheap to embed in a config struct or to
/// pass through scoring helpers.
///
/// ```
/// use thing_matcher::SimilarityAlgorithm;
/// let alg = SimilarityAlgorithm::Combined;
/// let same = alg;          // Copy
/// assert!(matches!(same, SimilarityAlgorithm::Combined));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SimilarityAlgorithm {
    /// Jaro-Winkler similarity — favours common prefixes; good for names.
    JaroWinkler,
    /// Normalised Levenshtein similarity — tracks edit distance.
    Levenshtein,
    /// Exact equality — binary `1.0` / `0.0`.
    Exact,
    /// Weighted blend of Jaro-Winkler (0.7) and Levenshtein (0.3). The default.
    Combined,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact float equality is brittle, so tests compare against an epsilon.
    /// The scorer returns deterministic sentinels (`0.0` / `1.0`) and small
    /// computed ratios; `f64::EPSILON` is tight enough to pin them.
    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < f64::EPSILON
    }

    // ---------- jaro_winkler ----------

    /// Pins that identical strings score near `1.0`.
    #[test]
    fn jaro_winkler_identical() {
        assert!(Scorer::jaro_winkler_similarity("smith", "smith") > 0.99);
    }

    /// Pins that a single-letter typo still scores high (prefix bonus).
    #[test]
    fn jaro_winkler_close_typo() {
        assert!(Scorer::jaro_winkler_similarity("smith", "smyth") > 0.85);
    }

    /// Pins that unrelated short strings score well below the match band.
    #[test]
    fn jaro_winkler_distant() {
        assert!(Scorer::jaro_winkler_similarity("jones", "james") < 0.8);
    }

    /// Pins the both-empty edge case → `1.0`.
    #[test]
    fn jaro_winkler_empty_pair_is_one() {
        assert!(approx_eq(Scorer::jaro_winkler_similarity("", ""), 1.0));
    }

    /// Pins the asymmetric-empty edge case (one side empty) → `0.0`.
    #[test]
    fn jaro_winkler_single_empty_is_zero() {
        assert!(approx_eq(Scorer::jaro_winkler_similarity("smith", ""), 0.0));
        assert!(approx_eq(Scorer::jaro_winkler_similarity("", "smith"), 0.0));
    }

    /// Pins the range invariant: every output is within `[0.0, 1.0]`.
    #[test]
    fn jaro_winkler_in_unit_interval() {
        for (a, b) in [("a", "b"), ("smith", "smyth"), ("abc", "xyz")] {
            let s = Scorer::jaro_winkler_similarity(a, b);
            assert!((0.0..=1.0).contains(&s), "out of range: {s}");
        }
    }

    // ---------- levenshtein ----------

    /// Pins that identical strings score exactly `1.0`.
    #[test]
    fn levenshtein_identical() {
        assert!(approx_eq(
            Scorer::levenshtein_similarity("smith", "smith"),
            1.0
        ));
    }

    /// Pins the exact normalisation formula `1 - distance/max_len` on a
    /// known case (one substitution over five characters → `0.8`).
    #[test]
    fn levenshtein_one_edit() {
        // 1 substitution over 5 chars: 1 - 1/5 = 0.8
        let s = Scorer::levenshtein_similarity("smith", "smyth");
        assert!((s - 0.8).abs() < 1e-9, "got {s}");
    }

    /// Pins that fully distinct equal-length strings score below `0.5`.
    #[test]
    fn levenshtein_completely_different() {
        assert!(Scorer::levenshtein_similarity("abc", "xyz") < 0.5);
    }

    /// Pins the both-empty edge case → `1.0`.
    #[test]
    fn levenshtein_empty_pair_is_one() {
        assert!(approx_eq(Scorer::levenshtein_similarity("", ""), 1.0));
    }

    /// Pins the asymmetric-empty edge case → `0.0`.
    #[test]
    fn levenshtein_single_empty_is_zero() {
        assert!(approx_eq(Scorer::levenshtein_similarity("smith", ""), 0.0));
        assert!(approx_eq(Scorer::levenshtein_similarity("", "smith"), 0.0));
    }

    // ---------- exact ----------

    /// Pins exact-match semantics: binary `1.0` / `0.0`, case-sensitive,
    /// with both-empty counting as a match.
    #[test]
    fn exact_match_basic() {
        assert!(approx_eq(Scorer::exact_match("test", "test"), 1.0));
        assert!(approx_eq(Scorer::exact_match("test", "Test"), 0.0));
        assert!(approx_eq(Scorer::exact_match("test", "other"), 0.0));
        assert!(approx_eq(Scorer::exact_match("", ""), 1.0));
    }

    // ---------- combined ----------

    /// Pins that the 0.7/0.3 blend yields `1.0` for identical inputs (both
    /// components are `1.0`).
    #[test]
    fn combined_identical_is_one() {
        assert!((Scorer::combined_similarity("smith", "smith") - 1.0).abs() < 1e-9);
    }

    /// Pins that the blend keeps a near-homophone pair above `0.80`.
    #[test]
    fn combined_close_typo_is_high() {
        let s = Scorer::combined_similarity("Stephen", "Steven");
        assert!(s > 0.80, "got {s}");
    }

    /// Pins that the blend keeps an unrelated pair below `0.5`.
    #[test]
    fn combined_distant_is_low() {
        assert!(Scorer::combined_similarity("alice", "zachary") < 0.5);
    }

    // ---------- jaccard ----------

    /// Pins that identical sets (order-independent) score `1.0`.
    #[test]
    fn jaccard_identical_sets_is_one() {
        let a = ["x", "y"];
        let b = ["y", "x"];
        assert!((Scorer::jaccard_set_similarity(&a, &b) - 1.0).abs() < 1e-9);
    }

    /// Pins that disjoint sets score `0.0`.
    #[test]
    fn jaccard_disjoint_sets_is_zero() {
        let a = ["x", "y"];
        let b = ["a", "b"];
        assert!(Scorer::jaccard_set_similarity(&a, &b) < 1e-9);
    }

    /// Pins the both-empty edge case → `1.0`.
    #[test]
    fn jaccard_both_empty_is_one() {
        let a: [&str; 0] = [];
        let b: [&str; 0] = [];
        assert!(approx_eq(Scorer::jaccard_set_similarity(&a, &b), 1.0));
    }

    /// Pins the asymmetric-empty edge case → `0.0`, in both argument orders.
    #[test]
    fn jaccard_one_empty_is_zero() {
        let a = ["x"];
        let b: [&str; 0] = [];
        assert!(approx_eq(Scorer::jaccard_set_similarity(&a, &b), 0.0));
        assert!(approx_eq(Scorer::jaccard_set_similarity(&b, &a), 0.0));
    }

    /// Pins the core formula on a worked example: intersection {y,z} over
    /// union {x,y,z,w} = `0.5`.
    #[test]
    fn jaccard_partial_overlap_is_intersection_over_union() {
        let a = ["x", "y", "z"];
        let b = ["y", "z", "w"];
        let s = Scorer::jaccard_set_similarity(&a, &b);
        assert!((s - 0.5).abs() < 1e-9, "got {s}");
    }

    // ---------- optional_field_score ----------

    /// Pins the both-`None` policy of `optional_field_score` → `1.0`.
    #[test]
    fn optional_field_both_none_is_one() {
        let n: Option<String> = None;
        assert!(approx_eq(
            Scorer::optional_field_score(&n, &n, SimilarityAlgorithm::Exact),
            1.0
        ));
    }

    /// Pins the one-`None` policy → `0.0`, in both argument orders.
    #[test]
    fn optional_field_asymmetric_is_zero() {
        let n: Option<String> = None;
        let s = Some("x".to_string());
        assert!(approx_eq(
            Scorer::optional_field_score(&s, &n, SimilarityAlgorithm::Exact),
            0.0
        ));
        assert!(approx_eq(
            Scorer::optional_field_score(&n, &s, SimilarityAlgorithm::Exact),
            0.0
        ));
    }

    /// Pins that when both sides are `Some`, the selected `SimilarityAlgorithm`
    /// is the one actually applied (each variant gives its expected result).
    #[test]
    fn optional_field_some_some_uses_algorithm() {
        let a = Some("smith".to_string());
        let b = Some("smyth".to_string());
        let jw = Scorer::optional_field_score(&a, &b, SimilarityAlgorithm::JaroWinkler);
        let lv = Scorer::optional_field_score(&a, &b, SimilarityAlgorithm::Levenshtein);
        let ex = Scorer::optional_field_score(&a, &b, SimilarityAlgorithm::Exact);
        let cb = Scorer::optional_field_score(&a, &b, SimilarityAlgorithm::Combined);
        assert!(jw > 0.85);
        assert!(lv >= 0.79);
        assert!(approx_eq(ex, 0.0));
        assert!(cb > 0.8);
    }
}
