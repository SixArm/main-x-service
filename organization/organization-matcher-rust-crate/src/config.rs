//! Matching configuration: per-component weights and the probable-match
//! threshold. [`MatchConfig::default`] pins the canonical weights; the
//! [`MatchConfig::strict`] and [`MatchConfig::lenient`] presets adjust
//! only the threshold.

use serde::{Deserialize, Serialize};

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
}
