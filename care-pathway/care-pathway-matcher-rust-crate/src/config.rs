//! Matching configuration: per-component weights and the probable-match
//! threshold. [`MatchConfig::default`] pins the canonical weights; the
//! [`MatchConfig::strict`] and [`MatchConfig::lenient`] presets adjust
//! only the threshold.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Per-component weights + the probable-match threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchConfig {
    /// Probable-match cutoff (probabilistic strategy). Default 0.85.
    pub threshold: f64,

    /// Weight of the name component (Jaro-Winkler). Default 0.30.
    pub name_weight: f64,
    /// Weight of the condition-codes (Jaccard) component. Default 0.25.
    pub condition_weight: f64,
    /// Weight of the same-provider pathway-code component. Default 0.15.
    pub pathway_code_weight: f64,
    /// Weight of the care-setting component. Default 0.10.
    pub care_setting_weight: f64,
    /// Weight of the interventions (Jaccard) component. Default 0.10.
    pub interventions_weight: f64,
    /// Weight of the keywords (Jaccard) component. Default 0.10.
    pub keywords_weight: f64,

    /// Weight of the relationship-set similarity component: typed-set
    /// Jaccard over `(relation, pathway_id)` pairs (see
    /// [`crate::RelationshipRef`]). Default `0.05` — a **supporting**
    /// signal only: two records referencing the same related pathways
    /// are weakly more likely the same pathway, but the field never
    /// identifies on its own and does not participate when either side
    /// has no relationships recorded. See spec §13.1 / §23.
    pub relationships_weight: f64,
    /// Weight of the tag-set similarity component: set Jaccard over the
    /// case-insensitively normalised tag sets. Default `0.05` — a
    /// **supporting** signal only, analogous to
    /// [`Self::relationships_weight`]: two records sharing the same
    /// operator-applied tags are weakly more likely the same pathway,
    /// but does not participate when either side has no tags recorded.
    /// See spec §13.2 / §23.
    pub tags_weight: f64,
}

impl Default for MatchConfig {
    /// Canonical configuration. Weights are chosen to sum to `1.0` so the
    /// raw weighted average sits in `[0.0, 1.0]` when every component is
    /// present; the renormalisation in `weighted_average` keeps that range
    /// even when some are absent. Relative magnitudes encode evidence
    /// strength: name (0.30) and condition codes (0.25) carry the most
    /// identity signal, pathway code (0.15) next, with setting /
    /// interventions / keywords (0.10 each) as corroboration.
    fn default() -> Self {
        Self {
            threshold: 0.85, // probable-match cutoff for `is_match`
            name_weight: 0.30,
            condition_weight: 0.25,
            pathway_code_weight: 0.15,
            care_setting_weight: 0.10,
            interventions_weight: 0.10,
            keywords_weight: 0.10,
            relationships_weight: 0.05,
            tags_weight: 0.05,
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
    /// use care_pathway_matcher::MatchConfig;
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
    /// use care_pathway_matcher::MatchConfig;
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

    /// Validate this config's weights and threshold (CPM-T1), returning
    /// it unchanged on success. Every field on `MatchConfig` is `pub`
    /// and directly settable — the plain struct literal is still how
    /// the presets and the common case build a config — but a caller
    /// assembling one from untrusted input (e.g. deserialized) can call
    /// this additive, opt-in check, where a negative or non-finite
    /// weight reaching [`crate::scoring::weighted_average`] unchecked
    /// could push a score outside `[0.0, 1.0]` or produce `NaN`,
    /// breaking the crate's own "scores stay bounded and finite"
    /// invariant (spec §24) and the [`crate::Confidence`] banding built
    /// on it. Same shape as the sibling `organization-matcher` crate's
    /// identical `MatchConfig`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] naming the first offending
    /// field if any weight is negative, `NaN`, or infinite, or if
    /// `threshold` is outside `[0.0, 1.0]`.
    pub fn validated(self) -> Result<Self> {
        let weights = [
            ("name_weight", self.name_weight),
            ("condition_weight", self.condition_weight),
            ("pathway_code_weight", self.pathway_code_weight),
            ("care_setting_weight", self.care_setting_weight),
            ("interventions_weight", self.interventions_weight),
            ("keywords_weight", self.keywords_weight),
            ("relationships_weight", self.relationships_weight),
            ("tags_weight", self.tags_weight),
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

    /// Sum of every per-component weight (1.0 for the documented
    /// defaults).
    #[cfg(test)]
    fn weight_total(&self) -> f64 {
        self.name_weight
            + self.condition_weight
            + self.pathway_code_weight
            + self.care_setting_weight
            + self.interventions_weight
            + self.keywords_weight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Pins the load-bearing invariant that the default weights sum to
    // exactly 1.0 (within float tolerance), so the documented range and
    // weighting story hold.
    #[test]
    fn default_weights_sum_to_one() {
        let total = MatchConfig::default().weight_total();
        assert!(
            (total - 1.0).abs() < 1e-9,
            "weights sum to {total}, expected 1.0"
        );
    }

    // Pins that `strict`/`lenient` move only the threshold (to 0.95/0.70)
    // and leave every weight identical to the default — they change the
    // match cutoff, never the score.
    #[test]
    fn presets_change_only_threshold() {
        let d = MatchConfig::default();
        let s = MatchConfig::strict();
        let l = MatchConfig::lenient();
        assert!((s.threshold - 0.95).abs() < 1e-9);
        assert!((l.threshold - 0.70).abs() < 1e-9);
        assert!((s.weight_total() - d.weight_total()).abs() < 1e-9);
        assert!((l.name_weight - d.name_weight).abs() < 1e-9);
    }

    // The two supporting-signal weights (§13.1/§13.2, §23) default to
    // 0.05 each and layer on top of the core six, which is why
    // `weight_total` (the core-six sum) still pins to exactly 1.0 above.
    #[test]
    fn default_relationships_and_tags_weight_is_005() {
        let c = MatchConfig::default();
        assert!((c.relationships_weight - 0.05).abs() < 1e-9);
        assert!((c.tags_weight - 0.05).abs() < 1e-9);
    }

    /// CPM-T1: the default config, and the two presets, all pass
    /// validation unchanged — `validated` must never reject a
    /// well-formed config.
    #[test]
    fn validated_accepts_the_defaults_and_presets() {
        assert!(MatchConfig::default().validated().is_ok());
        assert!(MatchConfig::strict().validated().is_ok());
        assert!(MatchConfig::lenient().validated().is_ok());
    }

    /// CPM-T1: a negative weight on any single field is rejected.
    #[test]
    fn validated_rejects_a_negative_weight() {
        let c = MatchConfig {
            name_weight: -0.1,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());
    }

    /// CPM-T1: `NaN` and infinite weights are rejected, not silently
    /// propagated into `weighted_average`.
    #[test]
    fn validated_rejects_nan_and_infinite_weights() {
        let c = MatchConfig {
            condition_weight: f64::NAN,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());

        let c = MatchConfig {
            pathway_code_weight: f64::INFINITY,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());
    }

    /// CPM-T1: a threshold outside `[0.0, 1.0]` (including `NaN`) is
    /// rejected even when every weight is well-formed.
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
