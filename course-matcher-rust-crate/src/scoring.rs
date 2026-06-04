//! Match-result shape + the renormalised weighted-sum helper.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    pub score: f64,
    pub is_match: bool,
    pub confidence: Confidence,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Confidence {
    High,
    Medium,
    #[default]
    Low,
}

impl Confidence {
    /// Classify a score into a coarse band. Mirrors the service-side
    /// `MatchQuality` for cross-crate consistency.
    pub fn classify(score: f64) -> Self {
        if score >= 0.95 {
            Confidence::High
        } else if score >= 0.70 {
            Confidence::Medium
        } else {
            Confidence::Low
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MatchBreakdown {
    pub name_score: Option<f64>,
    pub course_code_score: Option<f64>,
    pub provider_score: Option<f64>,
    pub educational_level_score: Option<f64>,
    pub keywords_score: Option<f64>,
    pub teaches_score: Option<f64>,
    /// True when the deterministic short-circuit fired.
    pub deterministic_match: bool,
}

/// Compute a renormalised weighted average over Some components only.
/// `Skipped` (None) components don't pull the score down.
pub fn weighted_average(components: &[(Option<f64>, f64)]) -> f64 {
    let mut weighted_sum = 0.0_f64;
    let mut weight_sum = 0.0_f64;
    for (score, weight) in components {
        if let Some(s) = score {
            weighted_sum += s * weight;
            weight_sum += weight;
        }
    }
    if weight_sum > 0.0 { weighted_sum / weight_sum } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn weighted_average_empty_is_zero() {
        assert_eq!(weighted_average(&[]), 0.0);
    }

    #[test]
    fn confidence_thresholds() {
        assert_eq!(Confidence::classify(0.99), Confidence::High);
        assert_eq!(Confidence::classify(0.85), Confidence::Medium);
        assert_eq!(Confidence::classify(0.50), Confidence::Low);
    }
}
