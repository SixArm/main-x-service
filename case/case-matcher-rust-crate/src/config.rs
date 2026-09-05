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
    /// The canonical configuration. The six weights sum to `1.0` (so the
    /// renormalised average lands in `[0.0, 1.0]` even when every
    /// component is present) and are ordered by identity strength: title
    /// and subjects dominate; status is the weakest standalone signal.
    fn default() -> Self {
        Self {
            // Probable-match cutoff. 0.85 is deliberately demanding: a
            // strong title alone (~0.9) can clear it, but a weak title
            // needs corroboration from other components.
            threshold: 0.85,
            // Title is the primary identity signal → heaviest weight.
            title_weight: 0.30,
            // Shared subjects (involved parties) strongly corroborate.
            subjects_weight: 0.25,
            // Agency-scoped case number: precise but only when present.
            case_number_weight: 0.15,
            // Categorical type: useful but coarse.
            case_type_weight: 0.10,
            // Status: weakest — the same matter changes status over time.
            status_weight: 0.05,
            // Free-form keywords/tags: moderate corroboration.
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

    /// Validate this config's weights and threshold, returning it
    /// unchanged on success. Every field on `MatchConfig` is `pub` and
    /// directly settable — the plain struct literal is still how the
    /// presets and the common case build a config — but a caller
    /// assembling one from untrusted input (e.g. deserialized) can call
    /// this additive, opt-in check, where a negative or non-finite
    /// weight reaching [`crate::scoring::weighted_average`] unchecked
    /// could push a score outside `[0.0, 1.0]` or produce `NaN`,
    /// breaking the crate's own "scores stay bounded and finite"
    /// invariant (spec §19/§21). Same shape as the sibling
    /// `organization-matcher`/`care-pathway-matcher` crates' identical
    /// `MatchConfig`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidConfig`] naming the first offending
    /// field if any weight is negative, `NaN`, or infinite, or if
    /// `threshold` is outside `[0.0, 1.0]`.
    pub fn validated(self) -> Result<Self> {
        let weights = [
            ("title_weight", self.title_weight),
            ("subjects_weight", self.subjects_weight),
            ("case_number_weight", self.case_number_weight),
            ("case_type_weight", self.case_type_weight),
            ("status_weight", self.status_weight),
            ("keywords_weight", self.keywords_weight),
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
    /// defaults).
    ///
    /// Test-only invariant check: confirms the presets keep the weights
    /// summing to one and only move the threshold. Returns the total of
    /// all six component weights.
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

    // Pins the core invariant: the default weights sum to exactly 1.0
    // (within float tolerance), which the renormalised average relies on.
    #[test]
    fn default_weights_sum_to_one() {
        let total = MatchConfig::default().weight_total();
        assert!(
            (total - 1.0).abs() < 1e-9,
            "weights sum to {total}, expected 1.0"
        );
    }

    // Pins that `strict`/`lenient` only move the threshold (to 0.95 /
    // 0.70) and leave every component weight identical to the default.
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
            title_weight: -0.1,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());
    }

    /// `NaN` and infinite weights are rejected, not silently propagated
    /// into `weighted_average`.
    #[test]
    fn validated_rejects_nan_and_infinite_weights() {
        let c = MatchConfig {
            subjects_weight: f64::NAN,
            ..MatchConfig::default()
        };
        assert!(c.validated().is_err());

        let c = MatchConfig {
            case_number_weight: f64::INFINITY,
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
