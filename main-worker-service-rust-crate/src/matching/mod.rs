//! Worker matching algorithms and scoring

use crate::models::Worker;
use crate::config::MatchingConfig;
use crate::Result;

pub mod algorithms;
pub mod phonetic;
pub mod scoring;

pub use scoring::{ProbabilisticScorer, DeterministicScorer, MatchQuality};

/// Match result containing a worker and their match score
#[derive(Debug, Clone)]
pub struct MatchResult {
    pub worker: Worker,
    pub score: f64,
    pub breakdown: MatchScoreBreakdown,
}

/// Breakdown of match score components
#[derive(Debug, Clone, serde::Serialize)]
pub struct MatchScoreBreakdown {
    pub name_score: f64,
    pub birth_date_score: f64,
    pub gender_score: f64,
    pub address_score: f64,
    pub identifier_score: f64,
    pub tax_id_score: f64,
    pub document_score: f64,
}

impl MatchScoreBreakdown {
    /// Get a summary of which components matched well
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

/// Worker matcher trait
pub trait WorkerMatcher: Send + Sync {
    /// Match a worker against a candidate
    fn match_workers(&self, worker: &Worker, candidate: &Worker) -> Result<MatchResult>;

    /// Find potential matches for a worker
    fn find_matches(&self, worker: &Worker, candidates: &[Worker]) -> Result<Vec<MatchResult>>;

    /// Check if a score meets the matching threshold
    fn is_match(&self, score: f64) -> bool;
}

/// Probabilistic matching strategy
pub struct ProbabilisticMatcher {
    scorer: ProbabilisticScorer,
}

impl ProbabilisticMatcher {
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            scorer: ProbabilisticScorer::new(config),
        }
    }

    /// Get the configured threshold (not implemented yet)
    pub fn threshold(&self) -> f64 {
        0.85 // TODO: expose config properly
    }

    /// Classify match quality
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
            .filter(|result| self.is_match(result.score))
            .collect();

        // Sort by score descending
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

/// Deterministic matching strategy
pub struct DeterministicMatcher {
    scorer: DeterministicScorer,
}

impl DeterministicMatcher {
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
            .filter(|result| self.is_match(result.score))
            .collect();

        // Sort by score descending
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
    use crate::models::{HumanName, Gender};
    use chrono::NaiveDate;

    fn create_test_config() -> MatchingConfig {
        MatchingConfig {
            threshold_score: 0.85,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        }
    }

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

    #[test]
    fn test_probabilistic_find_matches() {
        let config = MatchingConfig {
            threshold_score: 0.60, // Lower threshold for test (name+dob+gender only = ~0.65)
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let worker = create_test_worker("Smith", "John", dob);

        let candidates = vec![
            create_test_worker("Smith", "John", dob), // Exact match
            create_test_worker("Smyth", "John", dob), // Close match
            create_test_worker("Johnson", "Bob", NaiveDate::from_ymd_opt(1990, 5, 20)), // No match
        ];

        let matches = matcher.find_matches(&worker, &candidates).unwrap();

        // Should find at least one match (the exact match)
        assert!(matches.len() >= 1, "Expected at least 1 match, got {}", matches.len());

        // First match should have highest score
        if matches.len() > 1 {
            assert!(matches[0].score >= matches[1].score);
        }
    }

    #[test]
    fn test_deterministic_matcher() {
        let config = create_test_config();
        let matcher = DeterministicMatcher::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let worker1 = create_test_worker("Smith", "John", dob);
        let worker2 = create_test_worker("Smith", "John", dob);

        let result = matcher.match_workers(&worker1, &worker2).unwrap();

        assert!(matcher.is_match(result.score));
    }

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

    #[test]
    fn test_probabilistic_matcher_with_threshold() {
        let config = MatchingConfig {
            threshold_score: 0.60,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let worker = create_test_worker("Smith", "John", dob);
        let candidate = create_test_worker("Smith", "John", dob);

        let result = matcher.match_workers(&worker, &candidate).unwrap();
        // Name + DOB + Gender matching should exceed 0.60
        assert!(result.score >= 0.60, "Exact match should exceed threshold 0.60, got {}", result.score);
        assert!(matcher.is_match(result.score));
    }

    #[test]
    fn test_match_result_ordering_by_score() {
        let config = MatchingConfig {
            threshold_score: 0.10, // Very low to catch all
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let worker = create_test_worker("Smith", "John", dob);

        let candidates = vec![
            create_test_worker("Johnson", "Bob", NaiveDate::from_ymd_opt(1995, 5, 20)), // Low match
            create_test_worker("Smith", "John", dob), // Exact match
            create_test_worker("Smyth", "John", dob), // Close match
        ];

        let matches = matcher.find_matches(&worker, &candidates).unwrap();
        assert!(!matches.is_empty(), "Should find at least one match");

        // Results should be sorted descending by score
        for window in matches.windows(2) {
            assert!(window[0].score >= window[1].score,
                "Results should be sorted descending: {} >= {}", window[0].score, window[1].score);
        }
    }

    #[test]
    fn test_empty_candidates_list() {
        let config = create_test_config();
        let matcher = ProbabilisticMatcher::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let worker = create_test_worker("Smith", "John", dob);

        let matches = matcher.find_matches(&worker, &[]).unwrap();
        assert!(matches.is_empty(), "Empty candidates should produce empty results");
    }
}
