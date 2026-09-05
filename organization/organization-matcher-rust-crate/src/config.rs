//! Matching configuration: per-component weights and the probable-match
//! threshold. [`MatchConfig::default`] pins the canonical weights; the
//! [`MatchConfig::strict`] and [`MatchConfig::lenient`] presets adjust
//! only the threshold.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Per-component weights + the probable-match threshold.
///
/// The original six components (`name` through `keywords`) sum to
/// `1.0` by convention. `relationships_weight` and `tags_weight` are
/// **additive supporting weights** on top of that total, not part of
/// it — the weighted average ([`crate::scoring::weighted_average`]) is
/// renormalised over whichever components actually scored on both
/// records, so a combined sum greater than `1.0` does not push any
/// score outside `[0.0, 1.0]`; it only sets the two new fields'
/// relative share when they participate.
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

    /// Weight of the relationships component: typed-set Jaccard over
    /// `(relation, organization_id)` pairs (see
    /// [`crate::RelationshipRef`]). Default `0.05` — a **supporting**
    /// signal only: two records referencing the same related
    /// organizations are weakly more likely the same organization, but
    /// the field never identifies on its own and does not participate
    /// when either side has no relationships recorded. This weight is
    /// additive on top of the original six components (§7); see
    /// [`MatchConfig`] docs for why the combined total need not sum to
    /// `1.0`. Spec §14a / §23.
    pub relationships_weight: f64,

    /// Weight of the tags component: plain set Jaccard over the
    /// case-insensitively normalised tag sets. Default `0.05` — a
    /// **supporting** signal only, analogous to
    /// [`Self::relationships_weight`]: two records sharing the same
    /// operator-applied tags are weakly more likely the same
    /// organization, but does not participate when either side has no
    /// tags recorded. Spec §14b / §23.
    pub tags_weight: f64,
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
            relationships_weight: 0.05,
            tags_weight: 0.05,
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

    /// Validate this config's weights and threshold (ORGM-T1),
    /// returning it unchanged on success. Every field on `MatchConfig`
    /// is `pub` and directly settable, and the plain struct literal
    /// keeps working for the common case — this is an **additive**,
    /// opt-in check for a config built from untrusted input (e.g.
    /// deserialized), where a negative or non-finite weight reaching
    /// [`crate::scoring::weighted_average`] unchecked could push a
    /// score outside `[0.0, 1.0]` or produce `NaN`, breaking the
    /// crate's own "scores stay bounded and finite" invariant (spec
    /// §24) and the [`crate::Confidence`] banding built on it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] naming the first offending
    /// field if any weight is negative, `NaN`, or infinite, or if
    /// `threshold` is outside `[0.0, 1.0]`.
    pub fn validated(self) -> Result<Self> {
        let weights = [
            ("name_weight", self.name_weight),
            ("address_weight", self.address_weight),
            ("url_weight", self.url_weight),
            ("jurisdiction_weight", self.jurisdiction_weight),
            ("founding_date_weight", self.founding_date_weight),
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

    /// Sum of the original six per-component weights (1.0 for the
    /// documented defaults) — deliberately **excludes**
    /// `relationships_weight` / `tags_weight`, which are additive
    /// supporting weights layered on top (see the [`MatchConfig`] docs).
    /// Test-only invariant check; not part of the public API.
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

/// Unit tests for the configuration presets and weight invariants.
#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the key invariant: the default weights sum to exactly 1.0,
    /// so a fully-populated record scores on the documented scale.
    #[test]
    fn default_weights_sum_to_one() {
        let total = MatchConfig::default().weight_total();
        assert!(
            (total - 1.0).abs() < 1e-9,
            "weights sum to {total}, expected 1.0"
        );
    }

    /// Pins the documented default threshold and the headline weights
    /// (name/address/url) against the spec, guarding silent drift.
    #[test]
    fn default_threshold_and_weights_match_spec() {
        let c = MatchConfig::default();
        assert!((c.threshold - 0.85).abs() < 1e-9);
        assert!((c.name_weight - 0.35).abs() < 1e-9);
        assert!((c.address_weight - 0.20).abs() < 1e-9);
        assert!((c.url_weight - 0.15).abs() < 1e-9);
    }

    /// Pins that `strict`/`lenient` move ONLY the threshold (to 0.95 /
    /// 0.70) and leave every component weight untouched.
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

    /// Pins the two new supporting weights (relationships, tags) to
    /// their documented default of `0.05` each (spec §14a / §14b).
    /// These are additive on top of the original six components' `1.0`
    /// total — see the [`MatchConfig`] docs for why a combined
    /// sum-to-`1.0` across all eight is not required for correctness.
    #[test]
    fn default_relationships_and_tags_weight_is_005() {
        let c = MatchConfig::default();
        assert!((c.relationships_weight - 0.05).abs() < 1e-9);
        assert!((c.tags_weight - 0.05).abs() < 1e-9);
    }

    /// ORGM-T1: the default config, and the two presets, all pass
    /// validation unchanged — `validated` must never reject a
    /// well-formed config.
    #[test]
    fn validated_accepts_the_defaults_and_presets() {
        assert!(MatchConfig::default().validated().is_ok());
        assert!(MatchConfig::strict().validated().is_ok());
        assert!(MatchConfig::lenient().validated().is_ok());
    }

    /// ORGM-T1: a negative weight on any single field is rejected.
    #[test]
    fn validated_rejects_a_negative_weight() {
        let c = MatchConfig {
            name_weight: -0.1,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());
    }

    /// ORGM-T1: `NaN` and infinite weights are rejected, not silently
    /// propagated into `weighted_average`.
    #[test]
    fn validated_rejects_nan_and_infinite_weights() {
        let c = MatchConfig {
            address_weight: f64::NAN,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());

        let c = MatchConfig {
            url_weight: f64::INFINITY,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());
    }

    /// ORGM-T1: a threshold outside `[0.0, 1.0]` (including `NaN`) is
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
