//! Scoring algorithms for string similarity, geographic distance, and
//! temporal proximity.
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
//! ## Geographic and temporal primitives
//!
//! Two Gaussian-decay scorers exist for domain-specific distances:
//!
//! - [`Scorer::coordinates_score`] turns a [`Scorer::haversine_metres`]
//!   distance into a similarity, parameterised by a metre scale.
//! - [`Scorer::start_date_score`] turns a [`Scorer::seconds_between`]
//!   difference into a similarity, parameterised by a seconds scale.
//!
//! ## Example
//!
//! ```
//! use event_matcher::Scorer;
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

use crate::normalizer::Normalizer;

/// Stateless namespace for similarity scorers.
///
/// Like [`crate::Normalizer`], `Scorer` is a unit type with no fields;
/// every method is associated.
///
/// ```
/// use event_matcher::Scorer;
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
    /// use event_matcher::Scorer;
    /// assert!(Scorer::jaro_winkler_similarity("smith", "smith") > 0.99);
    /// assert!(Scorer::jaro_winkler_similarity("smith", "smyth") > 0.85);
    /// assert_eq!(Scorer::jaro_winkler_similarity("", ""), 1.0);
    /// assert_eq!(Scorer::jaro_winkler_similarity("smith", ""), 0.0);
    /// ```
    #[must_use]
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
    /// use event_matcher::Scorer;
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
        let max_len = s1.len().max(s2.len());
        // `distance` and `max_len` are string-length counts. A length of
        // `u32::MAX` is a 4 GiB string; saturating there keeps the cast
        // lossless for every realistic input while avoiding precision loss.
        let distance = f64::from(u32::try_from(distance).unwrap_or(u32::MAX));
        let max_len = f64::from(u32::try_from(max_len).unwrap_or(u32::MAX));
        1.0 - (distance / max_len)
    }

    /// Binary exact-match score: `1.0` if `s1 == s2`, else `0.0`.
    ///
    /// Case-sensitive and whitespace-sensitive. Pair with
    /// [`crate::Normalizer`] when comparing user-entered text.
    ///
    /// # Examples
    ///
    /// ```
    /// use event_matcher::Scorer;
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
    /// use event_matcher::Scorer;
    ///
    /// let s = Scorer::combined_similarity("Stephen", "Steven");
    /// assert!(s > 0.80, "combined score for Stephen/Steven was {s}");
    /// ```
    #[must_use]
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
    /// use event_matcher::Scorer;
    /// let d = Scorer::haversine_metres(51.5, -0.12, 51.5, -0.12);
    /// assert!(d.abs() < 1e-6, "got {d}");
    /// ```
    ///
    /// London to Paris is about 343 km:
    ///
    /// ```
    /// use event_matcher::Scorer;
    /// let d = Scorer::haversine_metres(51.507_22, -0.127_5, 48.853_0, 2.349_2);
    /// let km = d / 1000.0;
    /// assert!(km > 330.0 && km < 355.0, "London-Paris km was {km}");
    /// ```
    #[must_use]
    pub fn haversine_metres(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        const EARTH_RADIUS_M: f64 = 6_371_000.0;
        let to_rad = |d: f64| d.to_radians();
        let phi1 = to_rad(lat1);
        let phi2 = to_rad(lat2);
        let dphi = to_rad(lat2 - lat1);
        let dlambda = to_rad(lon2 - lon1);
        let a =
            (dphi / 2.0).sin().powi(2) + phi1.cos() * phi2.cos() * (dlambda / 2.0).sin().powi(2);
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
    /// use event_matcher::Scorer;
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
    #[must_use]
    pub fn coordinates_score(distance_metres: f64, scale_metres: f64) -> f64 {
        gaussian_decay(distance_metres, scale_metres)
    }

    /// Absolute difference in seconds between two ISO-8601 timestamps.
    ///
    /// Both inputs are passed through [`Normalizer::parse_iso8601_unix_seconds`].
    /// Returns `None` if either string fails to parse.
    ///
    /// Dates without a time component are anchored to `00:00:00 UTC`.
    /// Local-time-only strings (no timezone) are interpreted as UTC for
    /// the sole purpose of computing a difference — callers who care about
    /// absolute wall-clock equivalence should pre-normalise to a common
    /// timezone before scoring.
    ///
    /// # Examples
    ///
    /// ```
    /// use event_matcher::Scorer;
    ///
    /// let same = Scorer::seconds_between("2024-06-26T09:00:00Z", "2024-06-26T09:00:00Z");
    /// assert_eq!(same, Some(0));
    ///
    /// let one_hour = Scorer::seconds_between(
    ///     "2024-06-26T09:00:00Z",
    ///     "2024-06-26T10:00:00Z",
    /// );
    /// assert_eq!(one_hour, Some(3600));
    ///
    /// // Antisymmetric in absolute value.
    /// let other_way = Scorer::seconds_between(
    ///     "2024-06-26T10:00:00Z",
    ///     "2024-06-26T09:00:00Z",
    /// );
    /// assert_eq!(other_way, Some(3600));
    ///
    /// assert!(Scorer::seconds_between("not-a-date", "2024-06-26").is_none());
    /// ```
    #[must_use]
    pub fn seconds_between(t1: &str, t2: &str) -> Option<i64> {
        let s1 = Normalizer::parse_iso8601_unix_seconds(t1)?;
        let s2 = Normalizer::parse_iso8601_unix_seconds(t2)?;
        Some((s1 - s2).abs())
    }

    /// Gaussian-decay similarity for a temporal difference.
    ///
    /// Returns `exp(-(d/s)^2)` clamped to `[0.0, 1.0]`, where `d` is the
    /// absolute difference in seconds and `s` is the configured scale in
    /// seconds.
    ///
    /// Contract:
    ///
    /// - `d == 0` → `1.0`.
    /// - `d == scale` → `1/e` (approximately `0.368`).
    /// - `d == 3 * scale` → approximately `0.0001`.
    /// - Negative or non-finite inputs are treated as missing and return `0.0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use event_matcher::Scorer;
    ///
    /// let scale_seconds = 3600.0; // one hour
    /// assert!((Scorer::start_date_score(0.0, scale_seconds) - 1.0).abs() < 1e-12);
    ///
    /// let one_e = Scorer::start_date_score(scale_seconds, scale_seconds);
    /// assert!((one_e - (1.0_f64 / std::f64::consts::E)).abs() < 1e-12);
    ///
    /// let far = Scorer::start_date_score(3.0 * scale_seconds, scale_seconds);
    /// assert!(far < 1e-3, "got {far}");
    /// ```
    #[must_use]
    pub fn start_date_score(difference_seconds: f64, scale_seconds: f64) -> f64 {
        gaussian_decay(difference_seconds, scale_seconds)
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
    /// use event_matcher::{Scorer, SimilarityAlgorithm};
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

/// Shared Gaussian (bell-curve) decay kernel backing both
/// [`Scorer::coordinates_score`] and [`Scorer::start_date_score`].
///
/// Computes `exp(-(distance/scale)^2)`, a smooth similarity that equals
/// `1.0` at `distance == 0`, `1/e ~= 0.368` at `distance == scale`, and
/// tends rapidly to `0.0` as the distance grows past a few multiples of
/// `scale`. The squared ratio (versus a plain exponential `exp(-d/s)`) is
/// chosen so that small differences are penalised very gently while large
/// ones are penalised sharply — the desired shape for "near enough is a
/// match, far apart is plainly not".
///
/// `distance` and `scale` are unitless here; callers supply metres (geo)
/// or seconds (temporal) for both, so the ratio is dimensionless.
///
/// Pathological inputs are treated as **missing** rather than allowed to
/// produce a misleading score: a non-finite `distance` or `scale`, a
/// non-positive `scale` (division by zero / sign flip), or a negative
/// `distance` (distances are absolute by construction) all return `0.0`.
/// The final [`f64::clamp`] guards against any floating-point excursion
/// outside `[0.0, 1.0]`.
fn gaussian_decay(distance: f64, scale: f64) -> f64 {
    // Reject inputs that cannot yield a meaningful similarity. Returning
    // 0.0 (not NaN, not a panic) keeps the crate's panic-free contract.
    if !distance.is_finite() || !scale.is_finite() || scale <= 0.0 || distance < 0.0 {
        return 0.0;
    }
    let ratio = distance / scale;
    // Square the ratio so the curve is the Gaussian bell, not a sharp spike.
    let s = (-(ratio * ratio)).exp();
    // Defensive clamp: exp() of a finite negative is already in (0, 1], but
    // pin the bounds so the contract holds even under rounding.
    s.clamp(0.0, 1.0)
}

/// Algorithm selector for name comparison in [`crate::MatchConfig`].
///
/// The enum is `Copy`, so it is cheap to embed in a config struct or to
/// pass through scoring helpers.
///
/// ```
/// use event_matcher::SimilarityAlgorithm;
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

    /// Approximate float equality for assertions on sentinel scores.
    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < f64::EPSILON
    }

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
        assert!(approx_eq(Scorer::jaro_winkler_similarity("", ""), 1.0));
    }

    #[test]
    fn jaro_winkler_single_empty_is_zero() {
        assert!(approx_eq(Scorer::jaro_winkler_similarity("smith", ""), 0.0));
        assert!(approx_eq(Scorer::jaro_winkler_similarity("", "smith"), 0.0));
    }

    #[test]
    fn levenshtein_identical() {
        assert!(approx_eq(
            Scorer::levenshtein_similarity("smith", "smith"),
            1.0
        ));
    }

    #[test]
    fn levenshtein_one_edit() {
        let s = Scorer::levenshtein_similarity("smith", "smyth");
        assert!((s - 0.8).abs() < 1e-9, "got {s}");
    }

    #[test]
    fn levenshtein_empty_pair_is_one() {
        assert!(approx_eq(Scorer::levenshtein_similarity("", ""), 1.0));
    }

    #[test]
    fn exact_match_basic() {
        assert!(approx_eq(Scorer::exact_match("test", "test"), 1.0));
        assert!(approx_eq(Scorer::exact_match("test", "Test"), 0.0));
        assert!(approx_eq(Scorer::exact_match("test", "other"), 0.0));
        assert!(approx_eq(Scorer::exact_match("", ""), 1.0));
    }

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
        assert!(approx_eq(Scorer::coordinates_score(f64::NAN, 50.0), 0.0));
        assert!(approx_eq(Scorer::coordinates_score(10.0, 0.0), 0.0));
        assert!(approx_eq(Scorer::coordinates_score(-1.0, 50.0), 0.0));
    }

    #[test]
    fn seconds_between_identical_is_zero() {
        let d = Scorer::seconds_between("2024-06-26T09:00:00Z", "2024-06-26T09:00:00Z");
        assert_eq!(d, Some(0));
    }

    #[test]
    fn seconds_between_one_hour() {
        let d = Scorer::seconds_between("2024-06-26T09:00:00Z", "2024-06-26T10:00:00Z");
        assert_eq!(d, Some(3600));
    }

    #[test]
    fn seconds_between_is_symmetric_absolute() {
        let a = Scorer::seconds_between("2024-06-26T09:00:00Z", "2024-06-26T10:00:00Z");
        let b = Scorer::seconds_between("2024-06-26T10:00:00Z", "2024-06-26T09:00:00Z");
        assert_eq!(a, b);
    }

    #[test]
    fn seconds_between_one_day() {
        let d = Scorer::seconds_between("2024-06-26", "2024-06-27");
        assert_eq!(d, Some(86_400));
    }

    #[test]
    fn seconds_between_rejects_garbage() {
        assert!(Scorer::seconds_between("not-a-date", "2024-06-26").is_none());
        assert!(Scorer::seconds_between("2024-06-26", "also-not-a-date").is_none());
    }

    #[test]
    fn start_date_score_contract() {
        let scale = 3600.0;
        assert!((Scorer::start_date_score(0.0, scale) - 1.0).abs() < 1e-12);
        let one_e = Scorer::start_date_score(scale, scale);
        assert!((one_e - (1.0_f64 / std::f64::consts::E)).abs() < 1e-12);
        let far = Scorer::start_date_score(3.0 * scale, scale);
        assert!(far < 1e-3);
    }

    #[test]
    fn start_date_score_rejects_pathological_inputs() {
        assert!(approx_eq(Scorer::start_date_score(f64::NAN, 3600.0), 0.0));
        assert!(approx_eq(Scorer::start_date_score(10.0, 0.0), 0.0));
        assert!(approx_eq(Scorer::start_date_score(-1.0, 3600.0), 0.0));
    }

    #[test]
    fn optional_field_both_none_is_one() {
        let n: Option<String> = None;
        assert!(approx_eq(
            Scorer::optional_field_score(&n, &n, SimilarityAlgorithm::Exact),
            1.0
        ));
    }

    #[test]
    fn optional_field_asymmetric_is_zero() {
        let n: Option<String> = None;
        let s = Some("x".to_string());
        assert!(approx_eq(
            Scorer::optional_field_score(&s, &n, SimilarityAlgorithm::Exact),
            0.0
        ));
    }
}
