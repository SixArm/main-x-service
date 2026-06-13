//! Matching configuration: per-component weights and the probable-match
//! threshold. [`MatchConfig::default`] pins the canonical weights; the
//! [`MatchConfig::strict`] and [`MatchConfig::lenient`] presets adjust only the
//! threshold.

use serde::{Deserialize, Serialize};

/// Per-component weights + the probable-match threshold. Defaults
/// pin the weights documented in `AGENTS/matching-algorithm.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchConfig {
    /// Probable-match cutoff (probabilistic strategy). Default 0.85.
    pub threshold: f64,

    /// Weight of the name component (Jaro-Winkler). Default 0.35.
    pub name_weight: f64,
    /// Weight of the same-provider course-code component. Default 0.15.
    pub course_code_weight: f64,
    /// Weight of the provider component. Default 0.15.
    pub provider_weight: f64,
    /// Weight of the educational-level component. Default 0.10.
    pub educational_level_weight: f64,
    /// Weight of the keywords (Jaccard) component. Default 0.10.
    pub keywords_weight: f64,
    /// Weight of the teaches / competencies (Jaccard) component. Default 0.15.
    pub teaches_weight: f64,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            threshold: 0.85,
            name_weight: 0.35,
            course_code_weight: 0.15,
            provider_weight: 0.15,
            educational_level_weight: 0.10,
            keywords_weight: 0.10,
            teaches_weight: 0.15,
        }
    }
}

impl MatchConfig {
    /// Tighter threshold for high-stakes deduplication (e.g. batch
    /// auto-merge). Bumps the threshold to 0.95 — only Definite-grade
    /// matches pass.
    ///
    /// # Examples
    ///
    /// ```
    /// use course_matcher::MatchConfig;
    ///
    /// let config = MatchConfig::strict();
    /// assert_eq!(config.threshold, 0.95);
    /// ```
    #[must_use]
    pub fn strict() -> Self {
        Self {
            threshold: 0.95,
            ..Self::default()
        }
    }

    /// Looser threshold for exploratory match-checking (e.g. UI
    /// "find similar courses"). Drops to 0.70.
    ///
    /// # Examples
    ///
    /// ```
    /// use course_matcher::MatchConfig;
    ///
    /// let config = MatchConfig::lenient();
    /// assert_eq!(config.threshold, 0.70);
    /// ```
    #[must_use]
    pub fn lenient() -> Self {
        Self {
            threshold: 0.70,
            ..Self::default()
        }
    }

    /// Sum of every per-component weight. The renormalised weighted
    /// average doesn't require this to equal 1.0, but the documented
    /// default weights do — see `weights_sum_to_one`.
    #[cfg(test)]
    fn weight_total(&self) -> f64 {
        self.name_weight
            + self.course_code_weight
            + self.provider_weight
            + self.educational_level_weight
            + self.keywords_weight
            + self.teaches_weight
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
    fn default_threshold_and_weights_match_spec() {
        let c = MatchConfig::default();
        assert!((c.threshold - 0.85).abs() < 1e-9);
        assert!((c.name_weight - 0.35).abs() < 1e-9);
        assert!((c.course_code_weight - 0.15).abs() < 1e-9);
        assert!((c.provider_weight - 0.15).abs() < 1e-9);
        assert!((c.educational_level_weight - 0.10).abs() < 1e-9);
        assert!((c.keywords_weight - 0.10).abs() < 1e-9);
        assert!((c.teaches_weight - 0.15).abs() < 1e-9);
    }

    #[test]
    fn strict_only_raises_the_threshold() {
        let d = MatchConfig::default();
        let s = MatchConfig::strict();
        assert!((s.threshold - 0.95).abs() < 1e-9);
        // Presets must not disturb the weights — only the threshold.
        assert!((s.weight_total() - d.weight_total()).abs() < 1e-9);
        assert!((s.name_weight - d.name_weight).abs() < 1e-9);
    }

    #[test]
    fn lenient_only_lowers_the_threshold() {
        let d = MatchConfig::default();
        let l = MatchConfig::lenient();
        assert!((l.threshold - 0.70).abs() < 1e-9);
        assert!((l.weight_total() - d.weight_total()).abs() < 1e-9);
        assert!((l.teaches_weight - d.teaches_weight).abs() < 1e-9);
    }

    #[test]
    fn config_serde_round_trips() {
        let c = MatchConfig::strict();
        let json = serde_json::to_string(&c).expect("serialize");
        let back: MatchConfig = serde_json::from_str(&json).expect("deserialize");
        assert!((back.threshold - c.threshold).abs() < 1e-9);
        assert!((back.name_weight - c.name_weight).abs() < 1e-9);
    }
}
