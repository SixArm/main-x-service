//! Matching configuration: per-component weights, the timeframe decay
//! width, and the probable-match threshold. [`MatchConfig::default`]
//! pins the canonical weights; the [`MatchConfig::strict`] and
//! [`MatchConfig::lenient`] presets adjust only the threshold.

use serde::{Deserialize, Serialize};

/// Per-component weights, the timeframe Gaussian width, and the
/// probable-match threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchConfig {
    /// Probable-match cutoff (probabilistic strategy). Default 0.85.
    pub threshold: f64,

    /// Weight of the name component (Jaro-Winkler). Default 0.30.
    pub name_weight: f64,
    /// Weight of the goals (Jaccard over titles) component. Default 0.15.
    pub goals_weight: f64,
    /// Weight of the owner-scoped code component. Default 0.15.
    pub code_weight: f64,
    /// Weight of the owner-org exact component. Default 0.10.
    pub owner_org_weight: f64,
    /// Weight of the parent plan exact component (`parent_ref`).
    /// Default 0.08.
    pub parent_weight: f64,
    /// Weight of the timeframe (date-proximity) component. Default 0.07.
    pub timeframe_weight: f64,
    /// Weight of the keywords (Jaccard) component. Default 0.05.
    pub keywords_weight: f64,
    /// Weight of the relationships (typed-set Jaccard) component.
    /// Default 0.05.
    pub relationships_weight: f64,
    /// Weight of the tags (Jaccard) component. Default 0.05.
    pub tags_weight: f64,

    /// Gaussian decay width (σ, in days) for the timeframe component: a
    /// day gap of σ scores ≈0.61, 2σ ≈0.14. Default 90.0.
    pub timeframe_sigma_days: f64,
}

impl Default for MatchConfig {
    /// The canonical configuration. The nine component weights sum to
    /// `1.0` (so the renormalised average lands in `[0.0, 1.0]` even when
    /// every component is present) and are ordered by identity strength:
    /// name dominates; tags / relationships are the weakest standalone
    /// signals.
    fn default() -> Self {
        Self {
            // Probable-match cutoff. 0.85 is deliberately demanding: a
            // strong name alone (~0.9) can clear it, but a weak name needs
            // corroboration from other components.
            threshold: 0.85,
            // Name is the primary identity signal → heaviest weight.
            name_weight: 0.30,
            // Shared goal titles strongly corroborate "same initiative".
            goals_weight: 0.15,
            // Owner-scoped code: precise but only within the same owner.
            code_weight: 0.15,
            // Same sponsoring organisation: useful exact signal.
            owner_org_weight: 0.10,
            // Same parent plan (`parent_ref`): a corroborating
            // containment signal in the unified recursive tree.
            parent_weight: 0.08,
            // Timeframe proximity: corroborating, decays with the day gap.
            timeframe_weight: 0.07,
            // Free-form keywords: moderate corroboration.
            keywords_weight: 0.05,
            // Typed within-entity relationships: supporting only.
            relationships_weight: 0.05,
            // Operator tags: supporting only.
            tags_weight: 0.05,
            // Default Gaussian width for the timeframe decay.
            timeframe_sigma_days: 90.0,
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
    /// use project_portfolio_management_matcher::MatchConfig;
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
    /// use project_portfolio_management_matcher::MatchConfig;
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

    /// Sum of every per-component weight (`1.0` for the documented
    /// defaults). Test-only invariant check.
    #[cfg(test)]
    fn weight_total(&self) -> f64 {
        self.name_weight
            + self.goals_weight
            + self.code_weight
            + self.owner_org_weight
            + self.parent_weight
            + self.timeframe_weight
            + self.keywords_weight
            + self.relationships_weight
            + self.tags_weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pins the core invariant: the default weights sum to exactly 1.0.
    #[test]
    fn default_weights_sum_to_one() {
        let total = MatchConfig::default().weight_total();
        assert!(
            (total - 1.0).abs() < 1e-9,
            "weights sum to {total}, expected 1.0"
        );
    }

    // Pins that `strict`/`lenient` only move the threshold.
    #[test]
    fn presets_change_only_threshold() {
        let d = MatchConfig::default();
        let s = MatchConfig::strict();
        let l = MatchConfig::lenient();
        assert!((s.threshold - 0.95).abs() < 1e-9);
        assert!((l.threshold - 0.70).abs() < 1e-9);
        assert!((s.weight_total() - d.weight_total()).abs() < 1e-9);
        assert!((l.name_weight - d.name_weight).abs() < 1e-9);
        assert!((l.timeframe_sigma_days - d.timeframe_sigma_days).abs() < 1e-9);
    }
}
