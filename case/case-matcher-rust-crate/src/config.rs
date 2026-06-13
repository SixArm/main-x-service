//! Matching configuration: per-component weights and the probable-match
//! threshold. [`MatchConfig::default`] pins the canonical weights; the
//! [`MatchConfig::strict`] and [`MatchConfig::lenient`] presets adjust
//! only the threshold.

use serde::{Deserialize, Serialize};

/// Per-component weights + the probable-match threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchConfig {
    /// Probable-match cutoff (probabilistic strategy). Default 0.85.
    pub threshold: f64,

    /// Weight of the title component (Jaro-Winkler). Default 0.30.
    pub title_weight: f64,
    /// Weight of the subjects (Jaccard) component. Default 0.25.
    pub subjects_weight: f64,
    /// Weight of the same-agency case-number component. Default 0.15.
    pub case_number_weight: f64,
    /// Weight of the case-type component. Default 0.10.
    pub case_type_weight: f64,
    /// Weight of the status component. Default 0.05.
    pub status_weight: f64,
    /// Weight of the keywords (Jaccard) component. Default 0.15.
    pub keywords_weight: f64,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            threshold: 0.85,
            title_weight: 0.30,
            subjects_weight: 0.25,
            case_number_weight: 0.15,
            case_type_weight: 0.10,
            status_weight: 0.05,
            keywords_weight: 0.15,
        }
    }
}

impl MatchConfig {
    /// Tighter threshold for high-stakes deduplication. Bumps the
    /// threshold to 0.95.
    ///
    /// # Examples
    ///
    /// ```
    /// use case_matcher::MatchConfig;
    ///
    /// assert_eq!(MatchConfig::strict().threshold, 0.95);
    /// ```
    #[must_use]
    pub fn strict() -> Self {
        Self {
            threshold: 0.95,
            ..Self::default()
        }
    }

    /// Looser threshold for exploratory match-checking. Drops to 0.70.
    ///
    /// # Examples
    ///
    /// ```
    /// use case_matcher::MatchConfig;
    ///
    /// assert_eq!(MatchConfig::lenient().threshold, 0.70);
    /// ```
    #[must_use]
    pub fn lenient() -> Self {
        Self {
            threshold: 0.70,
            ..Self::default()
        }
    }

    /// Sum of every per-component weight (1.0 for the documented
    /// defaults).
    #[cfg(test)]
    fn weight_total(&self) -> f64 {
        self.title_weight
            + self.subjects_weight
            + self.case_number_weight
            + self.case_type_weight
            + self.status_weight
            + self.keywords_weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_weights_sum_to_one() {
        let total = MatchConfig::default().weight_total();
        assert!(
            (total - 1.0).abs() < 1e-9,
            "weights sum to {total}, expected 1.0"
        );
    }

    #[test]
    fn presets_change_only_threshold() {
        let d = MatchConfig::default();
        let s = MatchConfig::strict();
        let l = MatchConfig::lenient();
        assert!((s.threshold - 0.95).abs() < 1e-9);
        assert!((l.threshold - 0.70).abs() < 1e-9);
        assert!((s.weight_total() - d.weight_total()).abs() < 1e-9);
        assert!((l.title_weight - d.title_weight).abs() < 1e-9);
    }
}
