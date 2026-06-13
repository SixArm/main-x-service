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

    /// Weight of the name component (Jaro-Winkler). Default 0.35.
    pub name_weight: f64,
    /// Weight of the postal-address component. Default 0.20.
    pub address_weight: f64,
    /// Weight of the url / domain component. Default 0.15.
    pub url_weight: f64,
    /// Weight of the jurisdiction (country) component. Default 0.10.
    pub jurisdiction_weight: f64,
    /// Weight of the founding-date component. Default 0.10.
    pub founding_date_weight: f64,
    /// Weight of the keywords (Jaccard) component. Default 0.10.
    pub keywords_weight: f64,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            threshold: 0.85,
            name_weight: 0.35,
            address_weight: 0.20,
            url_weight: 0.15,
            jurisdiction_weight: 0.10,
            founding_date_weight: 0.10,
            keywords_weight: 0.10,
        }
    }
}

impl MatchConfig {
    /// Tighter threshold for high-stakes deduplication (e.g. batch
    /// auto-merge). Bumps the threshold to 0.95.
    ///
    /// # Examples
    ///
    /// ```
    /// use organization_matcher::MatchConfig;
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

    /// Looser threshold for exploratory match-checking (e.g. a UI
    /// "find similar organizations"). Drops to 0.70.
    ///
    /// # Examples
    ///
    /// ```
    /// use organization_matcher::MatchConfig;
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

    /// Sum of every per-component weight (1.0 for the documented
    /// defaults).
    #[cfg(test)]
    fn weight_total(&self) -> f64 {
        self.name_weight
            + self.address_weight
            + self.url_weight
            + self.jurisdiction_weight
            + self.founding_date_weight
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
    fn default_threshold_and_weights_match_spec() {
        let c = MatchConfig::default();
        assert!((c.threshold - 0.85).abs() < 1e-9);
        assert!((c.name_weight - 0.35).abs() < 1e-9);
        assert!((c.address_weight - 0.20).abs() < 1e-9);
        assert!((c.url_weight - 0.15).abs() < 1e-9);
    }

    #[test]
    fn strict_and_lenient_change_only_threshold() {
        let d = MatchConfig::default();
        let s = MatchConfig::strict();
        let l = MatchConfig::lenient();
        assert!((s.threshold - 0.95).abs() < 1e-9);
        assert!((l.threshold - 0.70).abs() < 1e-9);
        assert!((s.weight_total() - d.weight_total()).abs() < 1e-9);
        assert!((l.name_weight - d.name_weight).abs() < 1e-9);
    }
}
