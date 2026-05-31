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
//! use place_matcher::Scorer;
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
/// use place_matcher::Scorer;
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
    /// use place_matcher::Scorer;
    /// assert!(Scorer::jaro_winkler_similarity("smith", "smith") > 0.99);
    /// assert!(Scorer::jaro_winkler_similarity("smith", "smyth") > 0.85);
    /// assert_eq!(Scorer::jaro_winkler_similarity("", ""), 1.0);
    /// assert_eq!(Scorer::jaro_winkler_similarity("smith", ""), 0.0);
    /// ```
    pub fn jaro_winkler_similarity(s1: &str, s2: &str) -> f64 {
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
    /// use place_matcher::Scorer;
    /// assert_eq!(Scorer::levenshtein_similarity("smith", "smith"), 1.0);
    /// assert!(Scorer::levenshtein_similarity("smith", "smyth") >= 0.79);
    /// assert!(Scorer::levenshtein_similarity("abc", "xyz") < 0.5);
    /// assert_eq!(Scorer::levenshtein_similarity("", ""), 1.0);
    /// ```
    pub fn levenshtein_similarity(s1: &str, s2: &str) -> f64 {
        if s1.is_empty() && s2.is_empty() {
            return 1.0;
        }
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }

        let distance = levenshtein(s1, s2);
        let max_len = s1.len().max(s2.len());
        1.0 - (distance as f64 / max_len as f64)
    }

    /// Binary exact-match score: `1.0` if `s1 == s2`, else `0.0`.
    ///
    /// Case-sensitive and whitespace-sensitive. Pair with
    /// [`crate::Normalizer`] when comparing user-entered text.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_matcher::Scorer;
    /// assert_eq!(Scorer::exact_match("test", "test"), 1.0);
    /// assert_eq!(Scorer::exact_match("Test", "test"), 0.0);  // case-sensitive
    /// assert_eq!(Scorer::exact_match("a", "b"),       0.0);
    /// ```
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
    /// use place_matcher::Scorer;
    ///
    /// let s = Scorer::combined_similarity("Stephen", "Steven");
    /// assert!(s > 0.80, "combined score for Stephen/Steven was {s}");
    /// ```
    pub fn combined_similarity(s1: &str, s2: &str) -> f64 {
        let jw = Self::jaro_winkler_similarity(s1, s2);
        let lev = Self::levenshtein_similarity(s1, s2);
        0.7 * jw + 0.3 * lev
    }

    /// Great-circle distance in metres between two geographic points,
    /// computed via the Haversine formula on a sphere of mean Earth
    /// radius `6_371_000` m.
    ///
    /// All inputs are decimal degrees. The function is total over `f64`:
    /// non-finite inputs produce `f64::NAN`, but no panic.
    ///
    /// # Examples
    ///
    /// Identical points are zero distance apart:
    ///
    /// ```
    /// use place_matcher::Scorer;
    /// let d = Scorer::haversine_metres(51.5, -0.12, 51.5, -0.12);
    /// assert!(d.abs() < 1e-6, "got {d}");
    /// ```
    ///
    /// London to Paris is about 343 km:
    ///
    /// ```
    /// use place_matcher::Scorer;
    /// // Charing Cross to Notre-Dame.
    /// let d = Scorer::haversine_metres(51.507_22, -0.127_5, 48.853_0, 2.349_2);
    /// let km = d / 1000.0;
    /// assert!(km > 330.0 && km < 355.0, "London-Paris km was {km}");
    /// ```
    ///
    /// Near-antipodes are close to half the Earth's circumference (~20015 km):
    ///
    /// ```
    /// use place_matcher::Scorer;
    /// let d = Scorer::haversine_metres(0.0, 0.0, 0.0, 180.0);
    /// let km = d / 1000.0;
    /// assert!((km - 20015.0).abs() < 50.0, "antipodal km was {km}");
    /// ```
    ///
    /// The formula works across the equator and the dateline:
    ///
    /// ```
    /// use place_matcher::Scorer;
    /// // Cross-equator (1 deg of latitude either side of equator).
    /// let cross_eq = Scorer::haversine_metres(1.0, 0.0, -1.0, 0.0);
    /// let cross_eq_km = cross_eq / 1000.0;
    /// assert!((cross_eq_km - 222.4).abs() < 1.0, "got {cross_eq_km} km");
    ///
    /// // Cross-dateline: 179.5 deg east vs -179.5 deg west at the equator
    /// // is 1 degree of arc, ~111 km — not ~39000 km.
    /// let cross_dl = Scorer::haversine_metres(0.0, 179.5, 0.0, -179.5);
    /// let cross_dl_km = cross_dl / 1000.0;
    /// assert!(cross_dl_km < 120.0, "cross-dateline km was {cross_dl_km}");
    /// ```
    pub fn haversine_metres(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        const EARTH_RADIUS_M: f64 = 6_371_000.0;
        let to_rad = |d: f64| d.to_radians();
        let phi1 = to_rad(lat1);
        let phi2 = to_rad(lat2);
        let dphi = to_rad(lat2 - lat1);
        let dlambda = to_rad(lon2 - lon1);
        let a = (dphi / 2.0).sin().powi(2)
            + phi1.cos() * phi2.cos() * (dlambda / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().clamp(0.0, 1.0).asin();
        EARTH_RADIUS_M * c
    }

    /// Gaussian-decay similarity for a geographic distance.
    ///
    /// Returns `exp(-(d/s)^2)` clamped to `[0.0, 1.0]`.
    ///
    /// Contract:
    ///
    /// - `d == 0` → `1.0`.
    /// - `d == scale` → `1/e` (approximately `0.368`).
    /// - `d == 3 * scale` → approximately `0.0001` (close to zero).
    /// - Negative or non-finite inputs are treated as missing and return `0.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use place_matcher::Scorer;
    ///
    /// let scale = 50.0;
    /// assert!((Scorer::coordinates_score(0.0, scale) - 1.0).abs() < 1e-12);
    ///
    /// let one_e = Scorer::coordinates_score(scale, scale);
    /// assert!((one_e - (1.0_f64 / std::f64::consts::E)).abs() < 1e-12);
    ///
    /// let far = Scorer::coordinates_score(3.0 * scale, scale);
    /// assert!(far < 1e-3, "got {far}");
    /// ```
    pub fn coordinates_score(distance_metres: f64, scale_metres: f64) -> f64 {
        if !distance_metres.is_finite()
            || !scale_metres.is_finite()
            || scale_metres <= 0.0
            || distance_metres < 0.0
        {
            return 0.0;
        }
        let ratio = distance_metres / scale_metres;
        let s = (-(ratio * ratio)).exp();
        s.clamp(0.0, 1.0)
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
    /// use place_matcher::{Scorer, SimilarityAlgorithm};
    ///
    /// let none: Option<String> = None;
    /// let a = Some("hello".to_string());
    /// let b = Some("hello".to_string());
    ///
    /// assert_eq!(Scorer::optional_field_score(&none, &none, SimilarityAlgorithm::Exact), 1.0);
    /// assert_eq!(Scorer::optional_field_score(&a,    &none, SimilarityAlgorithm::Exact), 0.0);
    /// assert_eq!(Scorer::optional_field_score(&a,    &b,    SimilarityAlgorithm::Exact), 1.0);
    /// ```
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
/// use place_matcher::SimilarityAlgorithm;
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

    // ---------- jaro_winkler ----------

    #[test]
    fn jaro_winkler_identical() {
        assert!(Scorer::jaro_winkler_similarity("smith", "smith") > 0.99);
    }

    #[test]
    fn jaro_winkler_close_typo() {
        assert!(Scorer::jaro_winkler_similarity("smith", "smyth") > 0.85);
    }

    #[test]
    fn jaro_winkler_distant() {
        assert!(Scorer::jaro_winkler_similarity("jones", "james") < 0.8);
    }

    #[test]
    fn jaro_winkler_empty_pair_is_one() {
        assert_eq!(Scorer::jaro_winkler_similarity("", ""), 1.0);
    }

    #[test]
    fn jaro_winkler_single_empty_is_zero() {
        assert_eq!(Scorer::jaro_winkler_similarity("smith", ""), 0.0);
        assert_eq!(Scorer::jaro_winkler_similarity("", "smith"), 0.0);
    }

    #[test]
    fn jaro_winkler_in_unit_interval() {
        for (a, b) in [("a", "b"), ("smith", "smyth"), ("abc", "xyz")] {
            let s = Scorer::jaro_winkler_similarity(a, b);
            assert!((0.0..=1.0).contains(&s), "out of range: {s}");
        }
    }

    // ---------- levenshtein ----------

    #[test]
    fn levenshtein_identical() {
        assert_eq!(Scorer::levenshtein_similarity("smith", "smith"), 1.0);
    }

    #[test]
    fn levenshtein_one_edit() {
        // 1 substitution over 5 chars: 1 - 1/5 = 0.8
        let s = Scorer::levenshtein_similarity("smith", "smyth");
        assert!((s - 0.8).abs() < 1e-9, "got {s}");
    }

    #[test]
    fn levenshtein_completely_different() {
        assert!(Scorer::levenshtein_similarity("abc", "xyz") < 0.5);
    }

    #[test]
    fn levenshtein_empty_pair_is_one() {
        assert_eq!(Scorer::levenshtein_similarity("", ""), 1.0);
    }

    #[test]
    fn levenshtein_single_empty_is_zero() {
        assert_eq!(Scorer::levenshtein_similarity("smith", ""), 0.0);
        assert_eq!(Scorer::levenshtein_similarity("", "smith"), 0.0);
    }

    // ---------- exact ----------

    #[test]
    fn exact_match_basic() {
        assert_eq!(Scorer::exact_match("test", "test"), 1.0);
        assert_eq!(Scorer::exact_match("test", "Test"), 0.0);
        assert_eq!(Scorer::exact_match("test", "other"), 0.0);
        assert_eq!(Scorer::exact_match("", ""), 1.0);
    }

    // ---------- combined ----------

    #[test]
    fn combined_identical_is_one() {
        assert!((Scorer::combined_similarity("smith", "smith") - 1.0).abs() < 1e-9);
    }

    #[test]
    fn combined_close_typo_is_high() {
        let s = Scorer::combined_similarity("Stephen", "Steven");
        assert!(s > 0.80, "got {s}");
    }

    #[test]
    fn combined_distant_is_low() {
        assert!(Scorer::combined_similarity("alice", "zachary") < 0.5);
    }

    // ---------- optional_field_score ----------

    #[test]
    fn optional_field_both_none_is_one() {
        let n: Option<String> = None;
        assert_eq!(
            Scorer::optional_field_score(&n, &n, SimilarityAlgorithm::Exact),
            1.0
        );
    }

    #[test]
    fn optional_field_asymmetric_is_zero() {
        let n: Option<String> = None;
        let s = Some("x".to_string());
        assert_eq!(
            Scorer::optional_field_score(&s, &n, SimilarityAlgorithm::Exact),
            0.0
        );
        assert_eq!(
            Scorer::optional_field_score(&n, &s, SimilarityAlgorithm::Exact),
            0.0
        );
    }

    // ---------- haversine ----------

    #[test]
    fn haversine_identical_is_zero() {
        let d = Scorer::haversine_metres(51.5, -0.12, 51.5, -0.12);
        assert!(d.abs() < 1e-6);
    }

    #[test]
    fn haversine_london_paris_about_343km() {
        let d = Scorer::haversine_metres(51.507_22, -0.127_5, 48.853_0, 2.349_2) / 1000.0;
        assert!(d > 330.0 && d < 355.0, "got {d}");
    }

    #[test]
    fn haversine_antipodal_about_half_earth() {
        let d = Scorer::haversine_metres(0.0, 0.0, 0.0, 180.0) / 1000.0;
        assert!((d - 20015.0).abs() < 50.0, "got {d}");
    }

    #[test]
    fn haversine_cross_dateline_short_distance() {
        let d = Scorer::haversine_metres(0.0, 179.5, 0.0, -179.5) / 1000.0;
        assert!(d < 120.0, "got {d}");
    }

    // ---------- coordinates_score ----------

    #[test]
    fn coordinates_score_contract() {
        let scale = 50.0;
        assert!((Scorer::coordinates_score(0.0, scale) - 1.0).abs() < 1e-12);
        let one_e = Scorer::coordinates_score(scale, scale);
        assert!((one_e - (1.0_f64 / std::f64::consts::E)).abs() < 1e-12);
        let far = Scorer::coordinates_score(3.0 * scale, scale);
        assert!(far < 1e-3);
    }

    #[test]
    fn coordinates_score_rejects_pathological_inputs() {
        assert_eq!(Scorer::coordinates_score(f64::NAN, 50.0), 0.0);
        assert_eq!(Scorer::coordinates_score(10.0, 0.0), 0.0);
        assert_eq!(Scorer::coordinates_score(-1.0, 50.0), 0.0);
    }

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
        assert_eq!(ex, 0.0);
        assert!(cb > 0.8);
    }
}
