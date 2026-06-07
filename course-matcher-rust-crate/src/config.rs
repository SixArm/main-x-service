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
    pub fn strict() -> Self {
        Self {
            threshold: 0.95,
            ..Self::default()
        }
    }

    /// Looser threshold for exploratory match-checking (e.g. UI
    /// "find similar courses"). Drops to 0.70.
    pub fn lenient() -> Self {
        Self {
            threshold: 0.70,
            ..Self::default()
        }
    }
}
