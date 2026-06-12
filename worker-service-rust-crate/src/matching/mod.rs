//! The worker-matching engine: algorithms, scoring, and matcher strategies.
//!
//! Matching answers the question "do these two worker records describe the
//! same person?" by producing a confidence score in `[0.0, 1.0]` plus a
//! per-component [`MatchScoreBreakdown`](crate::matching::MatchScoreBreakdown).
//! Two strategies are offered behind the
//! [`WorkerMatcher`](crate::matching::WorkerMatcher) trait:
//!
//! - [`ProbabilisticMatcher`](crate::matching::ProbabilisticMatcher) — a
//!   weighted, fuzzy combination of name, birth date, gender, address,
//!   identifier, tax-ID, and document scores.
//! - [`DeterministicMatcher`](crate::matching::DeterministicMatcher) —
//!   rule-based scoring with short-circuits (an exact tax-ID or identifier
//!   match pins the score to 1.0).
//!
//! Submodules:
//! - [`algorithms`](crate::matching::algorithms) — the individual component
//!   comparison functions.
//! - [`phonetic`](crate::matching::phonetic) — Soundex phonetic encoding used
//!   as a name-score bonus.
//! - [`scoring`](crate::matching::scoring) — the
//!   [`ProbabilisticScorer`](crate::matching::ProbabilisticScorer) /
//!   [`DeterministicScorer`](crate::matching::DeterministicScorer) that
//!   combine the component scores and classify
//!   [`MatchQuality`](crate::matching::MatchQuality).
//! - [`adapter`](crate::matching::adapter) — bridges service
//!   [`Worker`](crate::models::Worker) records into the canonical
//!   `worker-matcher` crate (re-exported here as
//!   [`matcher_lib`](crate::matching::matcher_lib)).
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

pub mod adapter;
pub mod algorithms;
pub mod phonetic;
pub mod scoring;

pub use scoring::{DeterministicScorer, MatchQuality, ProbabilisticScorer};

/// Re-export the canonical `worker-matcher` library so callers can reach
/// `MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, the
/// `Worker` builder, and all 40+ national-identifier slots without taking
/// a separate dependency. Pair this with [`adapter::to_matcher_worker`] to
/// score two service `Worker` records through the reference algorithm.
pub use ::worker_matcher as matcher_lib;

/// The outcome of comparing a query worker against one candidate: the
/// candidate, the overall [`score`](Self::score), and the per-component
/// [`breakdown`](Self::breakdown) that produced it.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// The candidate worker that was scored.
    pub worker: Worker,
    /// Overall confidence score in `[0.0, 1.0]`.
    pub score: f64,
    /// Per-component scores that were combined into [`score`](Self::score).
    pub breakdown: MatchScoreBreakdown,
}

/// The individual component scores that make up a [`MatchResult::score`].
///
/// Each field is a score in `[0.0, 1.0]` for one comparison axis. Serializable
/// so the breakdown can be surfaced verbatim in API responses and review-queue
/// items.
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
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();

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
    /// Scores `worker` against a single `candidate`, returning the candidate,
    /// the overall score, and the component breakdown.
    fn match_workers(&self, worker: &Worker, candidate: &Worker) -> Result<MatchResult>;

    /// Scores `worker` against every candidate, keeps those that clear the
    /// threshold, and returns them sorted by descending score.
    fn find_matches(&self, worker: &Worker, candidates: &[Worker]) -> Result<Vec<MatchResult>>;

    /// Returns `true` when `score` is at or above this matcher's threshold.
    fn is_match(&self, score: f64) -> bool;
}

/// A [`WorkerMatcher`] that combines weighted, fuzzy component scores into an
/// overall probabilistic confidence.
pub struct ProbabilisticMatcher {
    /// The underlying scorer holding the weights and threshold config.
    scorer: ProbabilisticScorer,
}

impl ProbabilisticMatcher {
    /// Builds a probabilistic matcher from the given matching configuration.
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            scorer: ProbabilisticScorer::new(config),
        }
    }

    /// Returns the match threshold.
    ///
    /// Currently hard-coded to `0.85`; wiring this through to the config is
    /// tracked as a TODO in the source.
    pub fn threshold(&self) -> f64 {
        0.85 // TODO: expose config properly
    }

    /// Classifies a raw score into a coarse [`MatchQuality`] band (definite /
    /// probable / possible / unlikely).
    pub fn classify_match(&self, score: f64) -> MatchQuality {
        self.scorer.classify_match(score)
    }
}

impl WorkerMatcher for ProbabilisticMatcher {
    fn match_workers(&self, worker: &Worker, candidate: &Worker) -> Result<MatchResult> {
        Ok(self.scorer.calculate_score(worker, candidate))
    }

    fn find_matches(&self, worker: &Worker, candidates: &[Worker]) -> Result<Vec<MatchResult>> {
        let mut matches: Vec<MatchResult> = candidates
            .iter()
            .map(|candidate| self.scorer.calculate_score(worker, candidate))
            // Drop anything below the configured threshold up front.
            .filter(|result| self.is_match(result.score))
            .collect();

        // Sort best-first. `partial_cmp` can yield `None` only for NaN scores,
        // which the scorer never produces; treat any such case as "equal".
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(matches)
    }

    fn is_match(&self, score: f64) -> bool {
        self.scorer.is_match(score)
    }
}

/// A [`WorkerMatcher`] that applies rule-based deterministic scoring with
/// short-circuits (e.g. an exact tax-ID match pins the score to 1.0).
pub struct DeterministicMatcher {
    /// The underlying rule-based scorer.
    scorer: DeterministicScorer,
}

impl DeterministicMatcher {
    /// Builds a deterministic matcher from the given matching configuration.
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            scorer: DeterministicScorer::new(config),
        }
    }
}

impl WorkerMatcher for DeterministicMatcher {
    fn match_workers(&self, worker: &Worker, candidate: &Worker) -> Result<MatchResult> {
        Ok(self.scorer.calculate_score(worker, candidate))
    }

    fn find_matches(&self, worker: &Worker, candidates: &[Worker]) -> Result<Vec<MatchResult>> {
        let mut matches: Vec<MatchResult> = candidates
            .iter()
            .map(|candidate| self.scorer.calculate_score(worker, candidate))
            // Keep only candidates that satisfy the deterministic threshold.
            .filter(|result| self.is_match(result.score))
            .collect();

        // Sort best-first (see the probabilistic impl for the NaN rationale).
        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(matches)
    }

    fn is_match(&self, score: f64) -> bool {
        self.scorer.is_match(score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Gender, HumanName};
    use jiff::civil::Date;

    /// Builds a baseline matching config with the default 0.85 threshold.
    fn create_test_config() -> MatchingConfig {
        MatchingConfig {
            threshold_score: 0.85,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        }
    }

    /// Builds a minimal male [`Worker`] with the given name parts and DOB.
    fn create_test_worker(family: &str, given: &str, dob: Option<Date>) -> Worker {
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
            created_at: jiff::Timestamp::now(),
            updated_at: jiff::Timestamp::now(),
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

        let dob = Some(jiff::civil::date(1980, 1, 15));
        let worker = create_test_worker("Smith", "John", dob);

        let candidates = vec![
            create_test_worker("Smith", "John", dob), // Exact match
            create_test_worker("Smyth", "John", dob), // Close match
            create_test_worker("Johnson", "Bob", Some(jiff::civil::date(1990, 5, 20))), // No match
        ];

        let matches = matcher.find_matches(&worker, &candidates).unwrap();

        // Should find at least one match (the exact match)
        assert!(
            matches.len() >= 1,
            "Expected at least 1 match, got {}",
            matches.len()
        );

        // First match should have highest score
        if matches.len() > 1 {
            assert!(matches[0].score >= matches[1].score);
        }
    }

    /// Two identical workers clear the deterministic match threshold.
    #[test]
    fn test_deterministic_matcher() {
        let config = create_test_config();
        let matcher = DeterministicMatcher::new(config);

        let dob = Some(jiff::civil::date(1980, 1, 15));
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

        let dob = Some(jiff::civil::date(1980, 1, 15));
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

    /// `find_matches` results are always sorted by descending score.
    #[test]
    fn test_match_result_ordering_by_score() {
        let config = MatchingConfig {
            threshold_score: 0.10, // Very low to catch all
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);

        let dob = Some(jiff::civil::date(1980, 1, 15));
        let worker = create_test_worker("Smith", "John", dob);

        let candidates = vec![
            create_test_worker("Johnson", "Bob", Some(jiff::civil::date(1995, 5, 20))), // Low match
            create_test_worker("Smith", "John", dob), // Exact match
            create_test_worker("Smyth", "John", dob), // Close match
        ];

        let matches = matcher.find_matches(&worker, &candidates).unwrap();
        assert!(!matches.is_empty(), "Should find at least one match");

        // Results should be sorted descending by score
        for window in matches.windows(2) {
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

        let dob = Some(jiff::civil::date(1980, 1, 15));
        let worker = create_test_worker("Smith", "John", dob);

        let matches = matcher.find_matches(&worker, &[]).unwrap();
        assert!(
            matches.is_empty(),
            "Empty candidates should produce empty results"
        );
    }
}
