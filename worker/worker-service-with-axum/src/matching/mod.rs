//! The worker-matching engine: algorithms, scoring, and matcher strategies.
//!
//! Matching answers the question "do these two worker records describe the
//! same person?" by producing a confidence score in `[0.0, 1.0]`, a
//! per-component breakdown, and the [`MatchEvidence`] that says how much
//! the compared data *proves* (`agents/share/match-evidence.md`). Two
//! strategies are offered behind the [`WorkerMatcher`] trait:
//!
//! - [`ProbabilisticMatcher`] — the service's matcher. It scores through
//!   the canonical `worker-matcher` engine via [`adapter::to_matcher_worker`]:
//!   the score, `is_match`, the quality band, the breakdown and the
//!   evidence are all the engine's. Since spec §13 T-23 closed, a
//!   component neither worker holds is left out of the score rather than
//!   counted as `0`.
//! - [`DeterministicMatcher`] — rule-based scoring with short-circuits (an
//!   exact tax-ID or identifier match pins the score to 1.0), from the
//!   service's own [`DeterministicScorer`]; its breakdown and evidence are
//!   the engine's view of the same pair.
//!
//! Submodules:
//! - [`algorithms`] — the individual component comparison functions.
//! - [`phonetic`] — Soundex phonetic encoding used as a name-score bonus.
//! - [`scoring`] — the service's own [`ProbabilisticScorer`] /
//!   [`DeterministicScorer`] and the [`MatchQuality`] bands.
//! - [`adapter`] — bridges service [`Worker`] records into the canonical
//!   `worker-matcher` crate (re-exported here as [`matcher_lib`]).
//! - [`evidence`] — the wire mirror of the matcher's evidence.
//! - [`frequencies`] — value frequencies counted over the registry.
//!
//! # Examples
//!
//! ```
//! use worker_service::matching::{ProbabilisticMatcher, WorkerMatcher};
//! use worker_service::config::MatchingConfig;
//!
//! let matcher = ProbabilisticMatcher::new(MatchingConfig {
//!     threshold_score: 0.85,
//!     exact_match_score: 1.0,
//!     fuzzy_match_score: 0.8,
//! });
//! // A score at or above the threshold counts as a match.
//! assert!(matcher.is_match(0.90));
//! assert!(!matcher.is_match(0.50));
//! ```

use crate::Result;
use crate::config::MatchingConfig;
use crate::models::Worker;

/// Bridge from service-side [`Worker`] records into the canonical `worker-matcher` crate.
pub mod adapter;
/// Per-component matching algorithms (name, DOB, gender, address, identifier, tax id, document).
pub mod algorithms;
/// Match evidence on the wire (`agents/share/match-evidence.md`).
pub mod evidence;
/// Value frequencies counted over the registry (`match-evidence.md` §7).
pub mod frequencies;
/// Soundex phonetic encoding used as a name-matching bonus.
pub mod phonetic;
/// Probabilistic and deterministic scorers combining the component algorithms.
pub mod scoring;

pub use evidence::{MatchEvidence, MatchStrength};
pub use frequencies::{FrequencyCache, FrequencySource, MIN_FREQUENCY_POPULATION};
pub use scoring::{DeterministicScorer, MatchQuality, ProbabilisticScorer, ScoreResult};

/// Re-export the canonical `worker-matcher` library so callers can reach
/// `MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, the
/// `Worker` builder, and all 40+ national-identifier slots without taking
/// a separate dependency. Pair this with [`adapter::to_matcher_worker`] to
/// score two service `Worker` records through the reference algorithm.
pub use ::worker_matcher as matcher_lib;

use matcher_lib::evidence::{NoFrequencies, ValueFrequencies};

/// The outcome of comparing a query worker against one candidate.
///
/// Everything but [`worker`](Self::worker) comes from the `worker-matcher`
/// engine's result for the pair (except [`DeterministicMatcher`]'s score
/// and `is_match`, which are its own rules').
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// The candidate worker that was scored.
    pub worker: Worker,
    /// Overall confidence score in `[0.0, 1.0]`, renormalised over the
    /// components both workers hold.
    pub score: f64,
    /// Whether the pair clears the matcher's threshold.
    pub is_match: bool,
    /// The matcher's coarse confidence band for [`score`](Self::score).
    pub confidence: matcher_lib::Confidence,
    /// The matcher's per-component scores; `None` means not compared.
    pub breakdown: matcher_lib::MatchBreakdown,
    /// How much the compared data proves (`match-evidence.md`).
    pub evidence: MatchEvidence,
}

impl MatchResult {
    /// Wrap the engine's result for `candidate`.
    #[must_use]
    pub fn from_engine(candidate: Worker, result: &matcher_lib::MatchResult) -> Self {
        Self {
            worker: candidate,
            score: result.score,
            is_match: result.is_match,
            confidence: result.confidence,
            breakdown: result.breakdown.clone(),
            evidence: MatchEvidence::from(&result.evidence),
        }
    }

    /// The wire quality label, from the matcher's confidence band:
    /// `"certain"` (high, score ≥ 0.90), `"probable"` (medium, ≥ 0.75),
    /// else `"possible"`.
    #[must_use]
    pub fn quality(&self) -> &'static str {
        quality_label(self.confidence)
    }

    /// The breakdown as wire JSON: an object of the components both
    /// workers held, keyed by the matcher's field names
    /// (`given_name_score`, `uk_nhs_number_score`, …). A component either
    /// worker lacks is absent, not `0`. Scores only, never values.
    #[must_use]
    pub fn breakdown_json(&self) -> serde_json::Value {
        breakdown_json(&self.breakdown)
    }
}

/// The wire quality label for a matcher confidence band.
#[must_use]
pub fn quality_label(confidence: matcher_lib::Confidence) -> &'static str {
    match confidence {
        matcher_lib::Confidence::High => "certain",
        matcher_lib::Confidence::Medium => "probable",
        matcher_lib::Confidence::Low => "possible",
    }
}

/// A matcher breakdown as a JSON object of its compared components only.
#[must_use]
pub fn breakdown_json(breakdown: &matcher_lib::MatchBreakdown) -> serde_json::Value {
    let mut value = serde_json::to_value(breakdown).unwrap_or_default();
    if let Some(map) = value.as_object_mut() {
        map.retain(|_, v| !v.is_null());
    }
    value
}

/// The individual component scores of the service's own
/// [`ProbabilisticScorer`] / [`DeterministicScorer`] ([`ScoreResult`]).
///
/// Each field is a score in `[0.0, 1.0]` for one comparison axis, and a
/// component neither worker holds scores `0` here. The service's match
/// responses carry the matcher's breakdown instead
/// ([`MatchResult::breakdown_json`]).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MatchScoreBreakdown {
    /// Name similarity (Jaro-Winkler / Levenshtein / Soundex).
    pub name_score: f64,
    /// Birth-date proximity score.
    pub birth_date_score: f64,
    /// Gender agreement score.
    pub gender_score: f64,
    /// Best-pair address similarity.
    pub address_score: f64,
    /// Best identifier (type + system + value) match.
    pub identifier_score: f64,
    /// Tax-ID exact-match score (1.0 or 0.0).
    pub tax_id_score: f64,
    /// Best identity-document (type + number) match.
    pub document_score: f64,
}
impl MatchScoreBreakdown {
    /// Returns a human-readable summary listing the components that scored
    /// strongly, using per-component thresholds (e.g. name ≥ 0.90, address
    /// ≥ 0.80). Returns `"no strong matches"` when nothing clears its bar.
    ///
    /// # Examples
    ///
    /// ```
    /// use worker_service::matching::MatchScoreBreakdown;
    ///
    /// let b = MatchScoreBreakdown {
    ///     name_score: 0.96, birth_date_score: 0.0, gender_score: 1.0,
    ///     address_score: 0.0, identifier_score: 0.0, tax_id_score: 0.0,
    ///     document_score: 0.0,
    /// };
    /// let s = b.summary();
    /// assert!(s.contains("name"));
    /// assert!(s.contains("gender"));
    /// ```
    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

        // Per-component "strong enough to mention" bars. They are tuned per
        // axis rather than shared: identifier/document/tax_id are near-exact
        // signals so they need a high bar (≥0.95 / ==1.0), while the fuzzier
        // name/DOB/gender are listed at ≥0.90 and address — the coarsest — at
        // ≥0.80. These bars are for the human-readable label only; they do not
        // affect the numeric scoring.
        if self.name_score >= 0.90 {
            parts.push("name");
        }
        if self.birth_date_score >= 0.90 {
            parts.push("DOB");
        }
        if self.gender_score >= 0.90 {
            parts.push("gender");
        }
        if self.address_score >= 0.80 {
            parts.push("address");
        }
        if self.identifier_score >= 0.95 {
            parts.push("identifier");
        }
        if self.tax_id_score >= 1.0 {
            parts.push("tax_id");
        }
        if self.document_score >= 0.95 {
            parts.push("document");
        }

        if parts.is_empty() {
            "no strong matches".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// The strategy interface shared by all matchers. Implementors are `Send +
/// Sync` so they can be held in shared application state behind an `Arc`.
pub trait WorkerMatcher: Send + Sync {
    /// Scores `worker` against a single `candidate`, with value
    /// frequencies for the evidence (`match-evidence.md` §7). Frequencies
    /// change only the evidence, never the score.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying scoring fails.
    fn match_workers_with_frequencies(
        &self,
        worker: &Worker,
        candidate: &Worker,
        frequencies: &dyn ValueFrequencies,
    ) -> Result<MatchResult>;

    /// Scores `worker` against a single `candidate` with no value
    /// frequencies.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying scoring fails.
    fn match_workers(&self, worker: &Worker, candidate: &Worker) -> Result<MatchResult> {
        self.match_workers_with_frequencies(worker, candidate, &NoFrequencies)
    }

    /// Scores `worker` against every candidate, keeps the matches
    /// (`is_match`), and returns them sorted by descending score.
    ///
    /// # Errors
    ///
    /// Returns an error if scoring any candidate fails.
    fn find_matches_with_frequencies(
        &self,
        worker: &Worker,
        candidates: &[Worker],
        frequencies: &dyn ValueFrequencies,
    ) -> Result<Vec<MatchResult>> {
        let mut matches = Vec::new();
        for candidate in candidates {
            let result = self.match_workers_with_frequencies(worker, candidate, frequencies)?;
            if result.is_match {
                matches.push(result);
            }
        }
        sort_best_first(&mut matches);
        Ok(matches)
    }

    /// [`find_matches_with_frequencies`](Self::find_matches_with_frequencies)
    /// with no value frequencies.
    ///
    /// # Errors
    ///
    /// Returns an error if scoring any candidate fails.
    fn find_matches(&self, worker: &Worker, candidates: &[Worker]) -> Result<Vec<MatchResult>> {
        self.find_matches_with_frequencies(worker, candidates, &NoFrequencies)
    }

    /// Returns `true` when `score` is at or above this matcher's threshold.
    fn is_match(&self, score: f64) -> bool;
}

/// Sort best-first. `partial_cmp` can yield `None` only for NaN scores,
/// which neither scorer produces; treat any such case as "equal".
fn sort_best_first(matches: &mut [MatchResult]) {
    matches.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// The engine configuration for a service [`MatchingConfig`]: the
/// matcher's defaults, with the service's threshold.
fn engine_config(config: &MatchingConfig) -> matcher_lib::MatchConfig {
    matcher_lib::MatchConfig {
        match_threshold: config.threshold_score,
        ..matcher_lib::MatchConfig::default()
    }
}

/// The service's [`WorkerMatcher`]: scores through the `worker-matcher`
/// engine, taking its score, `is_match`, confidence, breakdown and
/// evidence (spec §13 T-23).
pub struct ProbabilisticMatcher {
    /// The canonical engine, configured with the service's threshold.
    engine: matcher_lib::MatchingEngine,
    /// The service's threshold and [`MatchQuality`] bands
    /// ([`threshold`](Self::threshold), [`classify_match`](Self::classify_match));
    /// it scores nothing on this path.
    bands: ProbabilisticScorer,
}

impl ProbabilisticMatcher {
    /// Builds a probabilistic matcher from the given matching configuration.
    #[must_use]
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            engine: matcher_lib::MatchingEngine::new(engine_config(&config)),
            bands: ProbabilisticScorer::new(config),
        }
    }

    /// Returns the configured match threshold — the same
    /// `MatchingConfig::threshold_score` the engine's `is_match` and
    /// [`classify_match`](Self::classify_match) consult. Spec §13 T-17:
    /// this used to return a hard-coded `0.85` regardless of the matcher's
    /// actual configuration.
    #[must_use]
    pub fn threshold(&self) -> f64 {
        self.bands.threshold_score()
    }

    /// Classifies a raw score into a coarse [`MatchQuality`] band (definite /
    /// probable / possible / unlikely).
    #[must_use]
    pub fn classify_match(&self, score: f64) -> MatchQuality {
        self.bands.classify_match(score)
    }
}

impl WorkerMatcher for ProbabilisticMatcher {
    fn match_workers_with_frequencies(
        &self,
        worker: &Worker,
        candidate: &Worker,
        frequencies: &dyn ValueFrequencies,
    ) -> Result<MatchResult> {
        let result = self.engine.match_workers_with_frequencies(
            &adapter::to_matcher_worker(worker),
            &adapter::to_matcher_worker(candidate),
            frequencies,
        );
        Ok(MatchResult::from_engine(candidate.clone(), &result))
    }

    fn find_matches_with_frequencies(
        &self,
        worker: &Worker,
        candidates: &[Worker],
        frequencies: &dyn ValueFrequencies,
    ) -> Result<Vec<MatchResult>> {
        // Project the query once, not once per candidate.
        let query = adapter::to_matcher_worker(worker);
        let mut matches: Vec<MatchResult> = candidates
            .iter()
            .map(|candidate| {
                let result = self.engine.match_workers_with_frequencies(
                    &query,
                    &adapter::to_matcher_worker(candidate),
                    frequencies,
                );
                MatchResult::from_engine(candidate.clone(), &result)
            })
            .filter(|result| result.is_match)
            .collect();
        sort_best_first(&mut matches);
        Ok(matches)
    }

    fn is_match(&self, score: f64) -> bool {
        self.bands.is_match(score)
    }
}

/// A [`WorkerMatcher`] that applies rule-based deterministic scoring with
/// short-circuits (e.g. an exact tax-ID match pins the score to 1.0).
///
/// The score and `is_match` are the [`DeterministicScorer`]'s; the
/// confidence band, breakdown and evidence are the `worker-matcher`
/// engine's view of the same pair, so a deterministic match still
/// carries the evidence an auto-merge gate needs.
pub struct DeterministicMatcher {
    /// The underlying rule-based scorer.
    scorer: DeterministicScorer,
    /// The engine supplying breakdown and evidence.
    engine: matcher_lib::MatchingEngine,
}

impl DeterministicMatcher {
    /// Builds a deterministic matcher from the given matching configuration.
    #[must_use]
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            engine: matcher_lib::MatchingEngine::new(engine_config(&config)),
            scorer: DeterministicScorer::new(config),
        }
    }
}

impl WorkerMatcher for DeterministicMatcher {
    fn match_workers_with_frequencies(
        &self,
        worker: &Worker,
        candidate: &Worker,
        frequencies: &dyn ValueFrequencies,
    ) -> Result<MatchResult> {
        let scored = self.scorer.calculate_score(worker, candidate);
        let engine = self.engine.match_workers_with_frequencies(
            &adapter::to_matcher_worker(worker),
            &adapter::to_matcher_worker(candidate),
            frequencies,
        );
        let mut result = MatchResult::from_engine(scored.worker, &engine);
        result.score = scored.score;
        result.is_match = self.scorer.is_match(scored.score);
        result.confidence = matcher_lib::Confidence::from_score(scored.score);
        Ok(result)
    }

    fn is_match(&self, score: f64) -> bool {
        self.scorer.is_match(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Gender, HumanName};
    use chrono::NaiveDate;

    /// Builds a baseline matching config with the default 0.85 threshold.
    fn create_test_config() -> MatchingConfig {
        MatchingConfig {
            threshold_score: 0.85,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        }
    }

    /// Builds a minimal male [`Worker`] with the given name parts and DOB.
    fn create_test_worker(family: &str, given: &str, dob: Option<NaiveDate>) -> Worker {
        Worker {
            id: uuid::Uuid::new_v4(),
            identifiers: vec![],
            active: true,
            name: HumanName {
                use_type: None,
                family: family.to_string(),
                given: vec![given.to_string()],
                prefix: vec![],
                suffix: vec![],
            },
            additional_names: vec![],
            telecom: vec![],
            gender: Gender::Male,
            worker_type: None,
            birth_date: dob,
            tax_id: None,
            documents: vec![],
            emergency_contacts: vec![],
            deceased: false,
            deceased_datetime: None,
            addresses: vec![],
            marital_status: None,
            multiple_birth: None,
            photo: vec![],
            managing_organization: None,
            links: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    /// `find_matches` returns the strong candidates, best score first.
    #[test]
    fn test_probabilistic_find_matches() {
        let config = MatchingConfig {
            threshold_score: 0.60, // Lower threshold for test (name+dob+gender only = ~0.65)
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);

        let dob = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap());
        let worker = create_test_worker("Smith", "John", dob);

        let candidates = vec![
            create_test_worker("Smith", "John", dob), // Exact match
            create_test_worker("Smyth", "John", dob), // Close match
            create_test_worker(
                "Johnson",
                "Bob",
                Some(chrono::NaiveDate::from_ymd_opt(1990, 5, 20).unwrap()),
            ), // No match
        ];

        let match_results = matcher.find_matches(&worker, &candidates).unwrap();

        // Should find at least one match (the exact match)
        assert!(
            !match_results.is_empty(),
            "Expected at least 1 match, got {}",
            match_results.len()
        );

        // First match should have highest score
        if match_results.len() > 1 {
            assert!(match_results[0].score >= match_results[1].score);
        }
    }

    /// Two identical workers clear the deterministic match threshold.
    #[test]
    fn test_deterministic_matcher() {
        let config = create_test_config();
        let matcher = DeterministicMatcher::new(config);

        let dob = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap());
        let worker1 = create_test_worker("Smith", "John", dob);
        let worker2 = create_test_worker("Smith", "John", dob);

        let result = matcher.match_workers(&worker1, &worker2).unwrap();

        assert!(matcher.is_match(result.score));
    }

    /// `summary` lists every component that scored above its bar.
    #[test]
    fn test_match_score_breakdown_summary() {
        let breakdown = MatchScoreBreakdown {
            name_score: 0.95,
            birth_date_score: 0.92,
            gender_score: 1.0,
            address_score: 0.70,
            identifier_score: 0.40,
            tax_id_score: 0.0,
            document_score: 0.0,
        };

        let summary = breakdown.summary();
        assert!(summary.contains("name"));
        assert!(summary.contains("DOB"));
        assert!(summary.contains("gender"));
    }

    /// An exact match scores above the threshold and counts as a match.
    #[test]
    fn test_probabilistic_matcher_with_threshold() {
        let config = MatchingConfig {
            threshold_score: 0.60,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);

        let dob = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap());
        let worker = create_test_worker("Smith", "John", dob);
        let candidate = create_test_worker("Smith", "John", dob);

        let result = matcher.match_workers(&worker, &candidate).unwrap();
        // Name + DOB + Gender matching should exceed 0.60
        assert!(
            result.score >= 0.60,
            "Exact match should exceed threshold 0.60, got {}",
            result.score
        );
        assert!(matcher.is_match(result.score));
    }

    /// Spec §13 T-17: `ProbabilisticMatcher::threshold()` must reflect the
    /// matcher's actual configured `threshold_score`, not a hard-coded
    /// `0.85` — a non-default config's threshold must round-trip.
    #[test]
    fn threshold_reflects_the_configured_value() {
        let config = MatchingConfig {
            threshold_score: 0.42,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);
        assert!(
            (matcher.threshold() - 0.42).abs() < f64::EPSILON,
            "expected 0.42, got {}",
            matcher.threshold()
        );
    }

    /// `find_matches` results are always sorted by descending score.
    #[test]
    fn test_match_result_ordering_by_score() {
        let config = MatchingConfig {
            threshold_score: 0.10, // Very low to catch all
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);

        let dob = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap());
        let worker = create_test_worker("Smith", "John", dob);

        let candidates = vec![
            create_test_worker(
                "Johnson",
                "Bob",
                Some(chrono::NaiveDate::from_ymd_opt(1995, 5, 20).unwrap()),
            ), // Low match
            create_test_worker("Smith", "John", dob), // Exact match
            create_test_worker("Smyth", "John", dob), // Close match
        ];

        let match_results = matcher.find_matches(&worker, &candidates).unwrap();
        assert!(!match_results.is_empty(), "Should find at least one match");

        // Results should be sorted descending by score
        for window in match_results.windows(2) {
            assert!(
                window[0].score >= window[1].score,
                "Results should be sorted descending: {} >= {}",
                window[0].score,
                window[1].score
            );
        }
    }

    /// An empty candidate list yields no matches (no panic).
    #[test]
    fn test_empty_candidates_list() {
        let config = create_test_config();
        let matcher = ProbabilisticMatcher::new(config);

        let dob = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap());
        let worker = create_test_worker("Smith", "John", dob);

        let match_results = matcher.find_matches(&worker, &[]).unwrap();
        assert!(
            match_results.is_empty(),
            "Empty candidates should produce empty results"
        );
    }
}
