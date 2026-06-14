//! Match-result shape + the renormalised weighted-sum helper.

use serde::{Deserialize, Serialize};

/// The outcome of matching two cases.
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
    /// The "no match" result: zero score, `is_match = false`,
    /// [`Confidence::Low`], and an empty breakdown. Used as the base for
    /// the deterministic-hit result (which overrides the score/flags) and
    /// anywhere a neutral starting value is needed.
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
    /// use case_matcher::Confidence;
    ///
    /// assert_eq!(Confidence::classify(0.99), Confidence::High);
    /// assert_eq!(Confidence::classify(0.80), Confidence::Medium);
    /// assert_eq!(Confidence::classify(0.40), Confidence::Low);
    /// ```
    #[must_use]
    pub fn classify(score: f64) -> Self {
        // Bands are independent of `MatchConfig::threshold`: they describe
        // the score's strength, while the threshold decides `is_match`.
        // Boundaries are inclusive lower bounds (0.95 → High, 0.70 →
        // Medium), matching the service-side `MatchQuality` cut points.
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
    /// Title similarity (Jaro-Winkler).
    pub title_score: Option<f64>,
    /// Subjects (Jaccard) overlap.
    pub subjects_score: Option<f64>,
    /// Same-agency case-number similarity.
    pub case_number_score: Option<f64>,
    /// Case-type similarity.
    pub case_type_score: Option<f64>,
    /// Status similarity.
    pub status_score: Option<f64>,
    /// Keywords (Jaccard) similarity.
    pub keywords_score: Option<f64>,
    /// True when the deterministic short-circuit fired.
    pub deterministic_match: bool,
}

/// Compute a renormalised weighted average over `Some` components only.
/// Skipped (`None`) components don't pull the score down: the divisor is
/// the sum of the weights that actually contributed. Returns `0.0` when
/// no component is present.
///
/// Renormalisation is the core fairness rule: a record missing a field
/// should not be penalised as though it scored zero there. Dividing by
/// the *present* weight sum (rather than the fixed total of `1.0`)
/// rescales the result back into `[0.0, 1.0]` over whatever components
/// were available.
///
/// `components` pairs each component's optional score with its weight.
/// Returns the renormalised average, or `0.0` when every component is
/// `None` (avoiding a divide-by-zero).
#[must_use]
pub fn weighted_average(components: &[(Option<f64>, f64)]) -> f64 {
    let mut weighted_sum = 0.0_f64;
    let mut weight_sum = 0.0_f64;
    for (score, weight) in components {
        // Only present components contribute to BOTH sums, so absent ones
        // are invisible to the average rather than counting as zero.
        if let Some(s) = score {
            weighted_sum += s * weight;
            weight_sum += weight;
        }
    }
    // Guard the division: with no present components the divisor is 0.
    if weight_sum > 0.0 {
        weighted_sum / weight_sum
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pins that `None` components are excluded: two perfect present
    // components average to 1.0 regardless of the absent ones' weights.
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

    // Pins the empty-input guard: no components → 0.0 (no divide-by-zero).
    #[test]
    fn weighted_average_empty_is_zero() {
        assert!(weighted_average(&[]).abs() < 1e-9);
    }

    // Pins representative classifications across the three bands.
    #[test]
    fn confidence_thresholds() {
        assert_eq!(Confidence::classify(0.99), Confidence::High);
        assert_eq!(Confidence::classify(0.85), Confidence::Medium);
        assert_eq!(Confidence::classify(0.50), Confidence::Low);
    }

    // Pins the boundary semantics: exactly-on-boundary lands in the
    // higher band; just-below drops to the lower band.
    #[test]
    fn confidence_boundaries_are_inclusive_lower_bounds() {
        // Exactly on a boundary classifies into the higher band.
        assert_eq!(Confidence::classify(0.95), Confidence::High);
        assert_eq!(Confidence::classify(0.70), Confidence::Medium);
        // Just below flips to the lower band.
        assert_eq!(Confidence::classify(0.949_999), Confidence::Medium);
        assert_eq!(Confidence::classify(0.699_999), Confidence::Low);
    }

    // Pins the extremes: 0.0 → Low, 1.0 → High.
    #[test]
    fn confidence_extremes() {
        assert_eq!(Confidence::classify(0.0), Confidence::Low);
        assert_eq!(Confidence::classify(1.0), Confidence::High);
    }

    // Pins renormalisation arithmetic: (1.0*0.30 + 0.0*0.15)/(0.30+0.15)
    // = 0.666…, i.e. the absent component's weight is excluded.
    #[test]
    fn weighted_average_renormalises_over_present_weights() {
        // title 1.0 @ 0.30 and case_number 0.0 @ 0.15; others absent.
        // (1.0*0.30 + 0.0*0.15) / (0.30+0.15) = 0.666….
        let score = weighted_average(&[(Some(1.0), 0.30), (Some(0.0), 0.15), (None, 0.10)]);
        assert!((score - (0.30 / 0.45)).abs() < 1e-9, "got {score}");
    }

    // Pins that all-`None` components (even with non-zero weights) → 0.0.
    #[test]
    fn weighted_average_all_none_is_zero() {
        let score = weighted_average(&[(None, 0.30), (None, 0.15)]);
        assert!(score.abs() < 1e-9);
    }

    // Pins the `MatchResult::default` shape: zero / non-match / Low / no
    // deterministic flag.
    #[test]
    fn default_match_result_is_low_non_match() {
        let r = MatchResult::default();
        assert!(r.score.abs() < 1e-9);
        assert!(!r.is_match);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(!r.breakdown.deterministic_match);
    }

    // Pins the derived `Default` for `Confidence` resolves to `Low`.
    #[test]
    fn confidence_default_is_low() {
        assert_eq!(Confidence::default(), Confidence::Low);
    }
}
