//! Match-result shape + the renormalised weighted-sum helper.

use serde::{Deserialize, Serialize};

/// The outcome of matching two care pathways.
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
    /// use care_pathway_matcher::Confidence;
    ///
    /// assert_eq!(Confidence::classify(0.99), Confidence::High);
    /// assert_eq!(Confidence::classify(0.80), Confidence::Medium);
    /// assert_eq!(Confidence::classify(0.40), Confidence::Low);
    /// ```
    #[must_use]
    pub fn classify(score: f64) -> Self {
        // Bands are independent of the configurable `is_match` threshold;
        // they are fixed reporting buckets. Boundaries are inclusive lower
        // bounds, so a score exactly on 0.95 / 0.70 lands in the higher band.
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
    /// Condition-codes (Jaccard) overlap.
    pub condition_score: Option<f64>,
    /// Same-provider pathway-code similarity.
    pub pathway_code_score: Option<f64>,
    /// Care-setting similarity.
    pub care_setting_score: Option<f64>,
    /// Interventions (Jaccard) similarity.
    pub interventions_score: Option<f64>,
    /// Keywords (Jaccard) similarity.
    pub keywords_score: Option<f64>,
    /// Relationship-set similarity: typed-set Jaccard over `(relation,
    /// pathway_id)` pairs, `|A ∩ B| / |A ∪ B|`. `None` when either side
    /// has no relationships recorded. See
    /// [`crate::RelationshipRef`]; spec §13.1.
    #[serde(default)]
    pub relationships_score: Option<f64>,
    /// Tag-set similarity: set Jaccard over the case-insensitively
    /// normalised tag sets, `|A ∩ B| / |A ∪ B|`. `None` when either side
    /// has no tags recorded. Spec §13.2.
    #[serde(default)]
    pub tags_score: Option<f64>,
    /// True when the deterministic short-circuit fired.
    pub deterministic_match: bool,
}

/// Compute a renormalised weighted average over `Some` components only.
/// Skipped (`None`) components don't pull the score down: the divisor is
/// the sum of the weights that actually contributed. Returns `0.0` when
/// no component is present.
#[must_use]
pub fn weighted_average(components: &[(Option<f64>, f64)]) -> f64 {
    let mut weighted_sum = 0.0_f64; // Σ (score · weight) over present components
    let mut weight_sum = 0.0_f64; // Σ weight over present components (the divisor)
    for (score, weight) in components {
        // Only present (`Some`) components contribute to BOTH sums; this
        // is the renormalisation — a skipped component never inflates the
        // divisor, so absent data neither helps nor hurts the score.
        if let Some(s) = score {
            weighted_sum += s * weight;
            weight_sum += weight;
        }
    }
    if weight_sum > 0.0 {
        weighted_sum / weight_sum
    } else {
        // No component was present ⇒ nothing to average; define as 0.0.
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two perfect components and two absent ones average to 1.0 — pins
    // that `None` entries are excluded from the divisor (renormalisation).
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

    // No components at all ⇒ defined as 0.0 (divisor would be zero).
    #[test]
    fn weighted_average_empty_is_zero() {
        assert!(weighted_average(&[]).abs() < 1e-9);
    }

    // Spot-checks one score in each band well clear of the boundaries.
    #[test]
    fn confidence_thresholds() {
        assert_eq!(Confidence::classify(0.99), Confidence::High);
        assert_eq!(Confidence::classify(0.85), Confidence::Medium);
        assert_eq!(Confidence::classify(0.50), Confidence::Low);
    }

    // Pins that boundaries are inclusive lower bounds: exactly-on lands in
    // the higher band, a hair below drops to the lower band.
    #[test]
    fn confidence_boundaries_are_inclusive_lower_bounds() {
        // Exactly on a boundary classifies into the higher band.
        assert_eq!(Confidence::classify(0.95), Confidence::High);
        assert_eq!(Confidence::classify(0.70), Confidence::Medium);
        // Just below flips to the lower band.
        assert_eq!(Confidence::classify(0.949_999), Confidence::Medium);
        assert_eq!(Confidence::classify(0.699_999), Confidence::Low);
    }

    // The endpoints of the score range map to the outer bands.
    #[test]
    fn confidence_extremes() {
        assert_eq!(Confidence::classify(0.0), Confidence::Low);
        assert_eq!(Confidence::classify(1.0), Confidence::High);
    }

    // Worked renormalisation example: a present 0.0 component DOES count
    // against the score (unlike an absent one), giving 0.7 here.
    #[test]
    fn weighted_average_renormalises_over_present_weights() {
        // name 1.0 @ 0.35 and provider 0.0 @ 0.15; others absent.
        // (1.0*0.35 + 0.0*0.15) / (0.35+0.15) = 0.7.
        let score = weighted_average(&[(Some(1.0), 0.35), (Some(0.0), 0.15), (None, 0.10)]);
        assert!((score - 0.7).abs() < 1e-9, "got {score}");
    }

    // All-absent components ⇒ 0.0 (the zero-divisor guard path).
    #[test]
    fn weighted_average_all_none_is_zero() {
        let score = weighted_average(&[(None, 0.35), (None, 0.15)]);
        assert!(score.abs() < 1e-9);
    }

    // The `Default` impl yields a Low, non-match, non-deterministic,
    // zero-score result — the neutral baseline reused by the engine.
    #[test]
    fn default_match_result_is_low_non_match() {
        let r = MatchResult::default();
        assert!(r.score.abs() < 1e-9);
        assert!(!r.is_match);
        assert_eq!(r.confidence, Confidence::Low);
        assert!(!r.breakdown.deterministic_match);
    }

    // Pins the derived `Default` for `Confidence` (the `#[default]` variant).
    #[test]
    fn confidence_default_is_low() {
        assert_eq!(Confidence::default(), Confidence::Low);
    }
}
