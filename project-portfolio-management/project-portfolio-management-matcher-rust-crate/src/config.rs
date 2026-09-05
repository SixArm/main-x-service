//! Matching configuration: per-component weights, the timeframe decay
//! width, and the probable-match threshold. [`MatchConfig::default`]
//! pins the canonical weights; the [`MatchConfig::strict`] and
//! [`MatchConfig::lenient`] presets adjust only the threshold.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

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

    /// Validate this config's weights, `timeframe_sigma_days`, and
    /// threshold, returning it unchanged on success. Every field on
    /// `MatchConfig` is `pub` and directly settable — the plain struct
    /// literal is still how the presets and the common case build a
    /// config — but a caller assembling one from untrusted input (e.g.
    /// deserialized config) can call this additive, opt-in check.
    ///
    /// `timeframe_score`'s own Gaussian decay already falls back to a
    /// 1-day width for a non-positive `timeframe_sigma_days` (including
    /// `NaN`, since `sigma > 0.0` is `false` for `NaN`), so a bad sigma
    /// can never itself produce an unbounded or `NaN` score — but it
    /// would otherwise be **silently ignored** rather than reported.
    /// Validating it here surfaces the caller's mistake instead of
    /// quietly substituting a different value than the one they set.
    /// Every weight, by contrast, has no such internal fallback: an
    /// unchecked negative or non-finite weight reaching
    /// [`crate::scoring::weighted_average`] can push the returned score
    /// outside `[0.0, 1.0]` or produce `NaN`, breaking the crate's own
    /// "scores stay bounded and finite" invariant (spec §19/§24) and
    /// the [`crate::Confidence`] banding built on it. Same shape as the
    /// sibling `organization-matcher`/`care-pathway-matcher`/
    /// `case-matcher` crates' `MatchConfig::validated`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] naming the first offending
    /// field if any weight or `timeframe_sigma_days` is negative,
    /// `NaN`, or infinite, or if `threshold` is outside `[0.0, 1.0]`.
    pub fn validated(self) -> Result<Self> {
        let weights = [
            ("name_weight", self.name_weight),
            ("goals_weight", self.goals_weight),
            ("code_weight", self.code_weight),
            ("owner_org_weight", self.owner_org_weight),
            ("parent_weight", self.parent_weight),
            ("timeframe_weight", self.timeframe_weight),
            ("keywords_weight", self.keywords_weight),
            ("relationships_weight", self.relationships_weight),
            ("tags_weight", self.tags_weight),
            ("timeframe_sigma_days", self.timeframe_sigma_days),
        ];
        for (name, weight) in weights {
            if !weight.is_finite() || weight < 0.0 {
                return Err(Error::InvalidConfig(format!(
                    "{name} must be a finite, non-negative number, got {weight}"
                )));
            }
        }
        if !self.threshold.is_finite() || !(0.0..=1.0).contains(&self.threshold) {
            return Err(Error::InvalidConfig(format!(
                "threshold must be finite and within [0.0, 1.0], got {}",
                self.threshold
            )));
        }
        Ok(self)
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

    /// The default config, and the two presets, all pass validation
    /// unchanged — `validated` must never reject a well-formed config.
    #[test]
    fn validated_accepts_the_defaults_and_presets() {
        assert!(MatchConfig::default().validated().is_ok());
        assert!(MatchConfig::strict().validated().is_ok());
        assert!(MatchConfig::lenient().validated().is_ok());
    }

    /// A negative weight on any single field is rejected.
    #[test]
    fn validated_rejects_a_negative_weight() {
        let c = MatchConfig {
            name_weight: -0.1,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());
    }

    /// `NaN` and infinite weights are rejected, not silently propagated
    /// into `weighted_average`.
    #[test]
    fn validated_rejects_nan_and_infinite_weights() {
        let c = MatchConfig {
            goals_weight: f64::NAN,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());

        let c = MatchConfig {
            code_weight: f64::INFINITY,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());
    }

    /// A negative or non-finite `timeframe_sigma_days` is rejected too,
    /// even though `timeframe_score` would otherwise silently substitute
    /// a 1-day fallback rather than propagate it into an unbounded score.
    #[test]
    fn validated_rejects_a_bad_timeframe_sigma() {
        let c = MatchConfig {
            timeframe_sigma_days: -1.0,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());

        let c = MatchConfig {
            timeframe_sigma_days: f64::NAN,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());
    }

    /// A threshold outside `[0.0, 1.0]` (including `NaN`) is rejected
    /// even when every weight is well-formed.
    #[test]
    fn validated_rejects_an_out_of_range_threshold() {
        let c = MatchConfig {
            threshold: 1.5,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());

        let c = MatchConfig {
            threshold: -0.01,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());

        let c = MatchConfig {
            threshold: f64::NAN,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());
    }
}
