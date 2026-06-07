//! Person matching: algorithms, scoring, and matcher strategies.
//!
//! This is the matching layer's public face. It defines the
//! [`PersonMatcher`](crate::matching::PersonMatcher) trait and two concrete strategies —
//! [`ProbabilisticMatcher`](crate::matching::ProbabilisticMatcher) (weighted fuzzy) and
//! [`DeterministicMatcher`](crate::matching::DeterministicMatcher) (rule-based) — both of which delegate the
//! numeric work to the scorers in [`scoring`](crate::matching::scoring). The per-field algorithms
//! live in [`algorithms`](crate::matching::algorithms) and the Soundex phonetic helper in
//! [`phonetic`](crate::matching::phonetic).
//!
//! [`MatchResult`](crate::matching::MatchResult) pairs a candidate with its overall score and a
//! per-component [`MatchScoreBreakdown`](crate::matching::MatchScoreBreakdown). The crate also re-exports the
//! canonical sibling `person-matcher` library as [`matcher_lib`](crate::matching::matcher_lib); pair
//! it with [`adapter::to_matcher_person`](crate::matching::adapter::to_matcher_person) to score service records
//! through the reference engine.

use crate::models::Person;
use crate::config::MatchingConfig;
use crate::Result;

/// Adapter from the service `Person` to the canonical matcher's `Person`.
pub mod adapter;
/// Per-field comparison algorithms (name, DOB, gender, address, …).
pub mod algorithms;
/// Soundex phonetic encoding and similarity.
pub mod phonetic;
/// Probabilistic and deterministic scoring strategies.
pub mod scoring;

pub use scoring::{ProbabilisticScorer, DeterministicScorer, MatchQuality};

/// Re-export the canonical `person-matcher` library so callers can reach
/// `MatchingEngine`, `MatchConfig`, `MatchResult`, `MatchBreakdown`, the
/// `Person` builder, and all 40+ national-identifier slots without taking
/// a separate dependency. Pair this with [`adapter::to_matcher_person`] to
/// score two service `Person` records through the reference algorithm.
pub use ::person_matcher as matcher_lib;

/// A scored candidate: the matched person plus its overall score and
/// per-component breakdown.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// The candidate person that was scored.
    pub person: Person,
    /// Overall match score in `[0.0, 1.0]`.
    pub score: f64,
    /// Per-component contribution to the overall score.
    pub breakdown: MatchScoreBreakdown,
}

/// The seven per-field scores that feed the overall match score.
///
/// Each field is in `[0.0, 1.0]`. Serialized into API responses so
/// callers can see *why* two records matched.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MatchScoreBreakdown {
    /// Name similarity (family + given + prefix/suffix).
    pub name_score: f64,
    /// Birth-date proximity.
    pub birth_date_score: f64,
    /// Gender agreement.
    pub gender_score: f64,
    /// Best postal-address similarity.
    pub address_score: f64,
    /// Best identifier (type+system+value) match.
    pub identifier_score: f64,
    /// Tax-ID exact match (deterministic signal).
    pub tax_id_score: f64,
    /// Best identity-document (type+number) match.
    pub document_score: f64,
}

impl MatchScoreBreakdown {
    /// Return a comma-joined list of the components that matched
    /// strongly (each above its own confidence cutoff), or
    /// `"no strong matches"` when none did.
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

/// A strategy for scoring one person against others.
///
/// Implemented by [`ProbabilisticMatcher`] and [`DeterministicMatcher`].
/// `Send + Sync` so an `Arc<dyn PersonMatcher>` can be shared across
/// async request handlers.
pub trait PersonMatcher: Send + Sync {
    /// Score `person` against a single `candidate`.
    fn match_persons(&self, person: &Person, candidate: &Person) -> Result<MatchResult>;

    /// Score `person` against every candidate, returning only those that
    /// meet the threshold, sorted by score descending.
    fn find_matches(&self, person: &Person, candidates: &[Person]) -> Result<Vec<MatchResult>>;

    /// Return `true` when a score meets this matcher's threshold.
    fn is_match(&self, score: f64) -> bool;
}

/// [`PersonMatcher`] backed by the weighted-average scorer.
pub struct ProbabilisticMatcher {
    /// The underlying weighted-average scorer.
    scorer: ProbabilisticScorer,
}

impl ProbabilisticMatcher {
    /// Create a probabilistic matcher from a [`MatchingConfig`].
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            scorer: ProbabilisticScorer::new(config),
        }
    }

    /// Return the threshold used for classification.
    ///
    /// Currently a hard-coded `0.85`; wiring this to the config is a
    /// known TODO.
    pub fn threshold(&self) -> f64 {
        0.85 // TODO: expose config properly
    }

    /// Bucket a score into a [`MatchQuality`] via the underlying scorer.
    pub fn classify_match(&self, score: f64) -> MatchQuality {
        self.scorer.classify_match(score)
    }
}

impl PersonMatcher for ProbabilisticMatcher {
    fn match_persons(&self, person: &Person, candidate: &Person) -> Result<MatchResult> {
        Ok(self.scorer.calculate_score(person, candidate))
    }

    fn find_matches(&self, person: &Person, candidates: &[Person]) -> Result<Vec<MatchResult>> {
        let mut matches: Vec<MatchResult> = candidates
            .iter()
            .map(|candidate| self.scorer.calculate_score(person, candidate))
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

/// [`PersonMatcher`] backed by the rule-based scorer.
pub struct DeterministicMatcher {
    /// The underlying rule-based scorer.
    scorer: DeterministicScorer,
}

impl DeterministicMatcher {
    /// Create a deterministic matcher from a [`MatchingConfig`].
    pub fn new(config: MatchingConfig) -> Self {
        Self {
            scorer: DeterministicScorer::new(config),
        }
    }
}

impl PersonMatcher for DeterministicMatcher {
    fn match_persons(&self, person: &Person, candidate: &Person) -> Result<MatchResult> {
        Ok(self.scorer.calculate_score(person, candidate))
    }

    fn find_matches(&self, person: &Person, candidates: &[Person]) -> Result<Vec<MatchResult>> {
        let mut matches: Vec<MatchResult> = candidates
            .iter()
            .map(|candidate| self.scorer.calculate_score(person, candidate))
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

    /// Build a default config with an 0.85 probable threshold.
    fn create_test_config() -> MatchingConfig {
        MatchingConfig {
            threshold_score: 0.85,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        }
    }

    /// Build a minimal male person with the given family/given name and DOB.
    fn create_test_person(family: &str, given: &str, dob: Option<NaiveDate>) -> Person {
        Person {
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

    /// find_matches returns above-threshold candidates, best first.
    #[test]
    fn test_probabilistic_find_matches() {
        let config = MatchingConfig {
            threshold_score: 0.60, // Lower threshold for test (name+dob+gender only = ~0.65)
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let person = create_test_person("Smith", "John", dob);

        let candidates = vec![
            create_test_person("Smith", "John", dob), // Exact match
            create_test_person("Smyth", "John", dob), // Close match
            create_test_person("Johnson", "Bob", NaiveDate::from_ymd_opt(1990, 5, 20)), // No match
        ];

        let matches = matcher.find_matches(&person, &candidates).unwrap();

        // Should find at least one match (the exact match)
        assert!(matches.len() >= 1, "Expected at least 1 match, got {}", matches.len());

        // First match should have highest score
        if matches.len() > 1 {
            assert!(matches[0].score >= matches[1].score);
        }
    }

    /// The deterministic matcher flags identical records as a match.
    #[test]
    fn test_deterministic_matcher() {
        let config = create_test_config();
        let matcher = DeterministicMatcher::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let person1 = create_test_person("Smith", "John", dob);
        let person2 = create_test_person("Smith", "John", dob);

        let result = matcher.match_persons(&person1, &person2).unwrap();

        assert!(matcher.is_match(result.score));
    }

    /// summary() lists the strongly-matching components.
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

    /// An exact match clears a low (0.60) configured threshold.
    #[test]
    fn test_probabilistic_matcher_with_threshold() {
        let config = MatchingConfig {
            threshold_score: 0.60,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let person = create_test_person("Smith", "John", dob);
        let candidate = create_test_person("Smith", "John", dob);

        let result = matcher.match_persons(&person, &candidate).unwrap();
        // Name + DOB + Gender matching should exceed 0.60
        assert!(result.score >= 0.60, "Exact match should exceed threshold 0.60, got {}", result.score);
        assert!(matcher.is_match(result.score));
    }

    /// find_matches results are sorted by descending score.
    #[test]
    fn test_match_result_ordering_by_score() {
        let config = MatchingConfig {
            threshold_score: 0.10, // Very low to catch all
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let matcher = ProbabilisticMatcher::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let person = create_test_person("Smith", "John", dob);

        let candidates = vec![
            create_test_person("Johnson", "Bob", NaiveDate::from_ymd_opt(1995, 5, 20)), // Low match
            create_test_person("Smith", "John", dob), // Exact match
            create_test_person("Smyth", "John", dob), // Close match
        ];

        let matches = matcher.find_matches(&person, &candidates).unwrap();
        assert!(!matches.is_empty(), "Should find at least one match");

        // Results should be sorted descending by score
        for window in matches.windows(2) {
            assert!(window[0].score >= window[1].score,
                "Results should be sorted descending: {} >= {}", window[0].score, window[1].score);
        }
    }

    /// An empty candidate list yields no matches.
    #[test]
    fn test_empty_candidates_list() {
        let config = create_test_config();
        let matcher = ProbabilisticMatcher::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let person = create_test_person("Smith", "John", dob);

        let matches = matcher.find_matches(&person, &[]).unwrap();
        assert!(matches.is_empty(), "Empty candidates should produce empty results");
    }
}
