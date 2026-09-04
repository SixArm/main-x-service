//! Match-result shape + the renormalised weighted-sum helper.

use serde::{Deserialize, Serialize};

/// The outcome of matching two courses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    /// Overall score in `[0.0, 1.0]`.
    pub score: f64,
    /// True when `score` meets the configured probable-match threshold.
    pub is_match: bool,
    /// Coarse confidence band derived from `score`.
    pub confidence: Confidence,
    /// Per-component score breakdown.
    pub breakdown: MatchBreakdown,
}

impl Default for MatchResult {
    /// The "no-match" zero value: score `0.0`, `is_match` false,
    /// `Confidence::Low`, and an all-`None` breakdown. Used as the base
    /// for the deterministic short-circuit result (which then flips only
    /// the fields it owns) and anywhere a neutral starting point helps.
    fn default() -> Self {
        Self {
            score: 0.0,
            is_match: false,
            confidence: Confidence::Low,
            breakdown: MatchBreakdown::default(),
        }
    }
}

/// Coarse confidence band for a match score.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Confidence {
    /// Definite match (score >= 0.95).
    High,
    /// Likely match (score >= 0.70).
    Medium,
    /// Unlikely match (score < 0.70).
    #[default]
    Low,
}

impl Confidence {
    /// Classify a score into a coarse band. Mirrors the service-side
    /// `MatchQuality` for cross-crate consistency.
    ///
    /// # Examples
    ///
    /// ```
    /// use course_matcher::Confidence;
    ///
    /// assert_eq!(Confidence::classify(0.99), Confidence::High);
    /// assert_eq!(Confidence::classify(0.80), Confidence::Medium);
    /// assert_eq!(Confidence::classify(0.40), Confidence::Low);
    /// ```
    #[must_use]
    pub fn classify(score: f64) -> Self {
        // Bands are lower-bound-inclusive. 0.95 is the "Definite" cutoff
        // shared with the deterministic short-circuit (a phonetic bonus
        // is capped just below it so it cannot mint a High alone). 0.70
        // is the "likely duplicate, worth a human look" floor.
        if score >= 0.95 {
            Confidence::High
        } else if score >= 0.70 {
            Confidence::Medium
        } else {
            Confidence::Low
        }
    }
}

/// Per-component scores. `None` means the component was skipped because one or
/// both records lacked the data, so it did not contribute to the weighted sum.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchBreakdown {
    /// Name similarity (Jaro-Winkler).
    pub name_score: Option<f64>,
    /// Same-provider course-code similarity.
    pub course_code_score: Option<f64>,
    /// Provider similarity.
    pub provider_score: Option<f64>,
    /// Educational-level similarity.
    pub educational_level_score: Option<f64>,
    /// Keywords (Jaccard) similarity.
    pub keywords_score: Option<f64>,
    /// Teaches / competencies (Jaccard) similarity.
    pub teaches_score: Option<f64>,
    /// Relationship-set similarity: typed-set Jaccard over `(relation,
    /// course_id)` pairs, `|A ∩ B| / |A ∪ B|`. `None` when either side
    /// has no relationships recorded. See [`crate::RelationshipRef`];
    /// spec §5.1 / §23 T-11.
    #[serde(default)]
    pub relationships_score: Option<f64>,
    /// Tag-set similarity: plain set Jaccard over the case-
    /// insensitively normalised tag sets, `|A ∩ B| / |A ∪ B|`. `None`
    /// when either side has no tags recorded. Spec §5.2 / §13a /
    /// §23 T-12.
    #[serde(default)]
    pub tags_score: Option<f64>,
    /// True when the deterministic short-circuit fired.
    pub deterministic_match: bool,
}

/// Compute a renormalised weighted average over `Some` components only.
///
/// Skipped (`None`) components don't pull the score down: the divisor is
/// the sum of the weights that *actually contributed*, not the sum of
/// all configured weights. This is what lets a pair sharing only a name
/// and a provider still score `1.0` — absent data is treated as "no
/// evidence", never as "negative evidence".
///
/// `components` is a slice of `(optional score, weight)` pairs; each
/// score, when present, must already lie in `[0.0, 1.0]`.
///
/// # Returns
///
/// The renormalised weighted average in `[0.0, 1.0]`, or `0.0` when no
/// component is present (every entry is `None`, so the divisor would be
/// zero) — **provided every present component's `weight` is
/// non-negative.** [`MatchConfig`](crate::MatchConfig)'s weight fields
/// are `pub` and intentionally unvalidated (spec §22: "Setting weights
/// to negative — not validated today, caller contract"), so a caller
/// that sets a negative weight can push this outside `[0.0, 1.0]` —
/// see `weighted_average_negative_weight_breaks_the_unit_interval_bound`
/// below, which pins this as the documented consequence of that caller
/// contract rather than a bug in this function.
#[must_use]
pub fn weighted_average(components: &[(Option<f64>, f64)]) -> f64 {
    let mut weighted_sum = 0.0_f64;
    let mut weight_sum = 0.0_f64;
    for (score, weight) in components {
        // Only present components accumulate into BOTH numerator and
        // denominator — that is the renormalisation.
        if let Some(s) = score {
            weighted_sum += s * weight;
            weight_sum += weight;
        }
    }
    if weight_sum > 0.0 {
        weighted_sum / weight_sum
    } else {
        // No component contributed; avoid a divide-by-zero and report
        // "no evidence" as the neutral 0.0.
        0.0
    }
}

/// Unit tests for the confidence bands and the renormalising average.
#[cfg(test)]
mod tests {
    use super::*;

    // Pins that `None` components are excluded from both numerator and
    // denominator, so two perfect present components yield exactly 1.0.
    #[test]
    fn weighted_average_ignores_none() {
        let score = weighted_average(&[
            (Some(1.0), 0.30),
            (Some(1.0), 0.25),
            (None, 0.10),
            (None, 0.10),
        ]);
        assert!((score - 1.0).abs() < 1e-9);
    }

    // Pins the empty-slice base case: no components ⇒ 0.0 (no divide-by-zero).
    #[test]
    fn weighted_average_empty_is_zero() {
        assert!(weighted_average(&[]).abs() < 1e-9);
    }

    // Pins representative points in each confidence band.
    #[test]
    fn confidence_thresholds() {
        assert_eq!(Confidence::classify(0.99), Confidence::High);
        assert_eq!(Confidence::classify(0.85), Confidence::Medium);
        assert_eq!(Confidence::classify(0.50), Confidence::Low);
    }

    #[test]
    fn confidence_boundaries_are_inclusive_lower_bounds() {
        // Exactly on a boundary classifies into the higher band.
        assert_eq!(Confidence::classify(0.95), Confidence::High);
        assert_eq!(Confidence::classify(0.70), Confidence::Medium);
        // Just below flips to the lower band.
        assert_eq!(Confidence::classify(0.949_999), Confidence::Medium);
        assert_eq!(Confidence::classify(0.699_999), Confidence::Low);
    }

    // Pins the endpoints of the score range: 0.0 ⇒ Low, 1.0 ⇒ High.
    #[test]
    fn confidence_extremes() {
        assert_eq!(Confidence::classify(0.0), Confidence::Low);
        assert_eq!(Confidence::classify(1.0), Confidence::High);
    }

    #[test]
    fn weighted_average_renormalises_over_present_weights() {
        // name 1.0 @ 0.35 and provider 0.0 @ 0.15; others absent.
        // (1.0*0.35 + 0.0*0.15) / (0.35+0.15) = 0.7.
        let score = weighted_average(&[(Some(1.0), 0.35), (Some(0.0), 0.15), (None, 0.10)]);
        assert!((score - 0.7).abs() < 1e-9, "got {score}");
    }

    // Pins that an all-`None` slice (every component skipped) is 0.0,
    // not NaN from a 0/0 division.
    #[test]
    fn weighted_average_all_none_is_zero() {
        let score = weighted_average(&[(None, 0.35), (None, 0.15)]);
        assert!(score.abs() < 1e-9);
    }

    // spec §22 anti-patterns / §23 T-13: a negative `MatchConfig` weight
    // is documented as "not validated today — caller contract", and this
    // pins the documented consequence rather than leaving it implicit.
    // `MatchConfig`'s weight fields are `pub`, so this is reachable from
    // a real caller, not merely an internal edge case: name @ 1.0 with
    // weight 1.0, provider @ 0.0 with weight -0.5 renormalises to
    // (1.0*1.0 + 0.0*-0.5) / (1.0 + -0.5) = 1.0/0.5 = 2.0, outside the
    // function's normal [0.0, 1.0] guarantee. A future change that adds
    // validation to `MatchConfig` (making this test start failing) would
    // be a deliberate, documented behaviour change — not this test
    // regressing silently.
    #[test]
    fn weighted_average_negative_weight_breaks_the_unit_interval_bound() {
        let score = weighted_average(&[(Some(1.0), 1.0), (Some(0.0), -0.5)]);
        assert!((score - 2.0).abs() < 1e-9, "got {score}");
        assert!(
            !(0.0..=1.0).contains(&score),
            "expected this negative-weight case to fall outside [0.0, 1.0]; got {score}"
        );
    }

    // Pins the neutral default: zero score, non-match, Low, no
    // deterministic flag.
    #[test]
    fn default_match_result_is_low_non_match() {
        let r = MatchResult::default();
        assert!(r.score.abs() < 1e-9);
        assert!(!r.is_match);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(!r.breakdown.deterministic_match);
    }

    // Pins the derived `Default` for `Confidence` to `Low` (the
    // `#[default]` variant), the safe fallback for an absent score.
    #[test]
    fn confidence_default_is_low() {
        assert_eq!(Confidence::default(), Confidence::Low);
    }
}
