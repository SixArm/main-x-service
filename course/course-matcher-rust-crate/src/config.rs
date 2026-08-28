//! Matching configuration: per-component weights and the probable-match
//! threshold. [`MatchConfig::default`] pins the canonical weights; the
//! [`MatchConfig::strict`] and [`MatchConfig::lenient`] presets adjust only the
//! threshold.

use serde::{Deserialize, Serialize};

/// Per-component weights + the probable-match threshold. Defaults
/// pin the weights documented in `agents/matching-algorithm.md`.
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

    /// Weight of the relationships (typed-set Jaccard over `(relation,
    /// course_id)` pairs) component. Default 0.05 — a **supporting**
    /// signal only: two records that reference the same related
    /// courses are weakly more likely the same course, but the field
    /// never identifies on its own and does not participate when
    /// either side has no relationships recorded. Sits in the
    /// supporting-signal cluster alongside [`Self::tags_weight`] and is
    /// intentionally NOT part of the six identifying weights that sum
    /// to 1.0 (§7). See spec §5.1 / §7 / §23 T-11.
    pub relationships_weight: f64,

    /// Weight of the tags (plain set Jaccard) component. Default
    /// 0.05 — a **supporting** signal only, analogous to
    /// [`Self::relationships_weight`]: two records sharing the same
    /// operator-applied tags are weakly more likely the same course,
    /// but does not participate when either side has no tags
    /// recorded. See spec §5.2 / §13a / §7 / §23 T-12.
    pub tags_weight: f64,
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
            relationships_weight: 0.05,
            tags_weight: 0.05,
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

    /// Sum of the six **identifying** per-component weights (name,
    /// course code, provider, educational level, keywords, teaches).
    /// The renormalised weighted average doesn't require this to equal
    /// 1.0 (it divides by the weights that actually contributed), but
    /// the documented defaults do — see the `default_weights_sum_to_one`
    /// test. Deliberately excludes `relationships_weight` /
    /// `tags_weight`: those sit in a separate low-weight
    /// **supporting-signal** cluster (§7) and are not part of the
    /// six-weight partition of unity. Exists only under `cfg(test)`
    /// because nothing in the runtime path needs the raw total.
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

/// Unit tests for the weight/threshold defaults and the presets.
#[cfg(test)]
mod tests {
    use super::*;

    // Pins the invariant that the canonical default weights form a
    // partition of unity (sum to 1.0). Not strictly required by the
    // renormaliser, but documented and relied on by readers.
    #[test]
    fn default_weights_sum_to_one() {
        let total = MatchConfig::default().weight_total();
        assert!(
            (total - 1.0).abs() < 1e-9,
            "weights sum to {total}, expected 1.0"
        );
    }

    // Pins every default weight + the threshold to the exact values in
    // the spec, so any unreviewed retuning of the algorithm fails CI.
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
        assert!((c.relationships_weight - 0.05).abs() < 1e-9);
        assert!((c.tags_weight - 0.05).abs() < 1e-9);
    }

    // Pins that the relationships/tags supporting-signal weights are
    // NOT part of the six-weight partition of unity (§7) — adding them
    // to `weight_total()` would break `default_weights_sum_to_one`.
    #[test]
    fn default_relationships_and_tags_weight_is_005_and_excluded_from_weight_total() {
        let c = MatchConfig::default();
        assert!((c.relationships_weight - 0.05).abs() < 1e-9);
        assert!((c.tags_weight - 0.05).abs() < 1e-9);
        assert!((c.weight_total() - 1.0).abs() < 1e-9);
    }

    // Pins that the `strict` preset moves ONLY the threshold (to 0.95)
    // and leaves the component weights untouched.
    #[test]
    fn strict_only_raises_the_threshold() {
        let d = MatchConfig::default();
        let s = MatchConfig::strict();
        assert!((s.threshold - 0.95).abs() < 1e-9);
        // Presets must not disturb the weights — only the threshold.
        assert!((s.weight_total() - d.weight_total()).abs() < 1e-9);
        assert!((s.name_weight - d.name_weight).abs() < 1e-9);
    }

    // Pins that the `lenient` preset moves ONLY the threshold (to 0.70)
    // and leaves the component weights untouched.
    #[test]
    fn lenient_only_lowers_the_threshold() {
        let d = MatchConfig::default();
        let l = MatchConfig::lenient();
        assert!((l.threshold - 0.70).abs() < 1e-9);
        assert!((l.weight_total() - d.weight_total()).abs() < 1e-9);
        assert!((l.teaches_weight - d.teaches_weight).abs() < 1e-9);
    }

    // Pins that the config survives a JSON serialise → deserialise
    // round-trip unchanged — it is part of the service API surface.
    #[test]
    fn config_serde_round_trips() {
        let c = MatchConfig::strict();
        let json = serde_json::to_string(&c).expect("serialize");
        let back: MatchConfig = serde_json::from_str(&json).expect("deserialize");
        assert!((back.threshold - c.threshold).abs() < 1e-9);
        assert!((back.name_weight - c.name_weight).abs() < 1e-9);
    }
}
