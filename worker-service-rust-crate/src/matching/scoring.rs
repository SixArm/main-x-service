//! Match scoring calculations
//!
//! This module combines individual matching algorithm scores into
//! overall match scores using configurable weights.

use crate::models::Worker;
use crate::config::MatchingConfig;
use super::{MatchResult, MatchScoreBreakdown};
use super::algorithms::{
    name_matching, dob_matching, gender_matching,
    address_matching, identifier_matching,
    tax_id_matching, document_matching,
};

/// Probabilistic scoring strategy
pub struct ProbabilisticScorer {
    /// Configuration for matching thresholds and weights
    config: MatchingConfig,
}

impl ProbabilisticScorer {
    /// Create a new probabilistic scorer with configuration
    pub fn new(config: MatchingConfig) -> Self {
        Self { config }
    }

    /// Calculate match score between two workers
    pub fn calculate_score(
        &self,
        worker: &Worker,
        candidate: &Worker,
    ) -> MatchResult {
        // Calculate individual component scores
        let name_score = name_matching::match_names(&worker.name, &candidate.name);

        let birth_date_score = dob_matching::match_birth_dates(
            worker.birth_date,
            candidate.birth_date,
        );

        let gender_score = gender_matching::match_gender(
            worker.gender,
            candidate.gender,
        );

        let address_score = address_matching::match_addresses(
            &worker.addresses,
            &candidate.addresses,
        );

        let identifier_score = identifier_matching::match_identifiers(
            &worker.identifiers,
            &candidate.identifiers,
        );

        let tax_id_score = tax_id_matching::match_tax_ids(worker, candidate);

        let document_score = document_matching::match_documents(
            &worker.documents,
            &candidate.documents,
        );

        // Tax ID exact match is a strong deterministic signal — short-circuit
        if tax_id_score >= 1.0 {
            return MatchResult {
                worker: candidate.clone(),
                score: 1.0,
                breakdown: MatchScoreBreakdown {
                    name_score, birth_date_score, gender_score,
                    address_score, identifier_score, tax_id_score, document_score,
                },
            };
        }

        // Document number exact match is also a strong signal — short-circuit
        if document_score >= 1.0 {
            return MatchResult {
                worker: candidate.clone(),
                score: 0.98,
                breakdown: MatchScoreBreakdown {
                    name_score, birth_date_score, gender_score,
                    address_score, identifier_score, tax_id_score, document_score,
                },
            };
        }

        // Weight factors for each component (probabilistic)
        const NAME_WEIGHT: f64 = 0.30;
        const DOB_WEIGHT: f64 = 0.25;
        const GENDER_WEIGHT: f64 = 0.10;
        const ADDRESS_WEIGHT: f64 = 0.10;
        const IDENTIFIER_WEIGHT: f64 = 0.10;
        const TAX_ID_WEIGHT: f64 = 0.10;
        const DOCUMENT_WEIGHT: f64 = 0.05;

        // Calculate weighted total score
        let total_score = (name_score * NAME_WEIGHT)
            + (birth_date_score * DOB_WEIGHT)
            + (gender_score * GENDER_WEIGHT)
            + (address_score * ADDRESS_WEIGHT)
            + (identifier_score * IDENTIFIER_WEIGHT)
            + (tax_id_score * TAX_ID_WEIGHT)
            + (document_score * DOCUMENT_WEIGHT);

        let breakdown = MatchScoreBreakdown {
            name_score,
            birth_date_score,
            gender_score,
            address_score,
            identifier_score,
            tax_id_score,
            document_score,
        };

        MatchResult {
            worker: candidate.clone(),
            score: total_score,
            breakdown,
        }
    }

    /// Check if a match score meets the threshold
    pub fn is_match(&self, score: f64) -> bool {
        score >= self.config.threshold_score
    }

    /// Classify match quality
    pub fn classify_match(&self, score: f64) -> MatchQuality {
        if score >= 0.95 {
            MatchQuality::Definite
        } else if score >= self.config.threshold_score {
            MatchQuality::Probable
        } else if score >= 0.50 {
            MatchQuality::Possible
        } else {
            MatchQuality::Unlikely
        }
    }
}

/// Deterministic scoring strategy
pub struct DeterministicScorer {
    /// Configuration for matching
    _config: MatchingConfig,
}

impl DeterministicScorer {
    /// Create a new deterministic scorer
    pub fn new(config: MatchingConfig) -> Self {
        Self { _config: config }
    }

    /// Calculate match score using strict rules
    pub fn calculate_score(
        &self,
        worker: &Worker,
        candidate: &Worker,
    ) -> MatchResult {
        let mut total_score = 0.0;
        let mut points_available = 0.0;

        // Rule 0: Tax ID exact match = definite match
        let tax_id_score = tax_id_matching::match_tax_ids(worker, candidate);
        if tax_id_score >= 1.0 {
            return MatchResult {
                worker: candidate.clone(),
                score: 1.0,
                breakdown: MatchScoreBreakdown {
                    name_score: 0.0, birth_date_score: 0.0, gender_score: 0.0,
                    address_score: 0.0, identifier_score: 0.0,
                    tax_id_score, document_score: 0.0,
                },
            };
        }

        // Rule 1: Exact identifier match = definite match
        let identifier_score = identifier_matching::match_identifiers(
            &worker.identifiers,
            &candidate.identifiers,
        );

        if identifier_score >= 0.98 {
            return MatchResult {
                worker: candidate.clone(),
                score: 1.0,
                breakdown: MatchScoreBreakdown {
                    name_score: 0.0, birth_date_score: 0.0, gender_score: 0.0,
                    address_score: 0.0, identifier_score,
                    tax_id_score, document_score: 0.0,
                },
            };
        }

        // Rule 1b: Document number exact match = definite match
        let document_score = document_matching::match_documents(
            &worker.documents,
            &candidate.documents,
        );

        if document_score >= 1.0 {
            return MatchResult {
                worker: candidate.clone(),
                score: 1.0,
                breakdown: MatchScoreBreakdown {
                    name_score: 0.0, birth_date_score: 0.0, gender_score: 0.0,
                    address_score: 0.0, identifier_score,
                    tax_id_score, document_score,
                },
            };
        }

        // Rule 2: Name + DOB + Gender must all match
        let name_score = name_matching::match_names(&worker.name, &candidate.name);
        let dob_score = dob_matching::match_birth_dates(
            worker.birth_date,
            candidate.birth_date,
        );
        let gender_score = gender_matching::match_gender(
            worker.gender,
            candidate.gender,
        );

        points_available += 3.0;

        if name_score >= 0.90 {
            total_score += 1.0;
        }

        if dob_score >= 0.95 {
            total_score += 1.0;
        }

        if gender_score >= 1.0 {
            total_score += 1.0;
        }

        // Rule 3: Address is optional but adds confidence
        let address_score = address_matching::match_addresses(
            &worker.addresses,
            &candidate.addresses,
        );

        if !worker.addresses.is_empty() && !candidate.addresses.is_empty() {
            points_available += 1.0;
            if address_score >= 0.80 {
                total_score += 1.0;
            }
        }

        // Calculate final score as percentage of available points
        let final_score = if points_available > 0.0 {
            total_score / points_available
        } else {
            0.0
        };

        let breakdown = MatchScoreBreakdown {
            name_score,
            birth_date_score: dob_score,
            gender_score,
            address_score,
            identifier_score,
            tax_id_score,
            document_score,
        };

        MatchResult {
            worker: candidate.clone(),
            score: final_score,
            breakdown,
        }
    }

    /// Check if a match score meets deterministic criteria
    pub fn is_match(&self, score: f64) -> bool {
        score >= 0.75 // Require at least 3/4 rules to match
    }
}

/// Match quality classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchQuality {
    /// Definite match (score >= 0.95)
    Definite,
    /// Probable match (score >= threshold)
    Probable,
    /// Possible match (score >= 0.50)
    Possible,
    /// Unlikely match (score < 0.50)
    Unlikely,
}

impl MatchQuality {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchQuality::Definite => "definite",
            MatchQuality::Probable => "probable",
            MatchQuality::Possible => "possible",
            MatchQuality::Unlikely => "unlikely",
        }
    }

    /// Check if this quality indicates a match
    pub fn is_match(&self) -> bool {
        matches!(self, MatchQuality::Definite | MatchQuality::Probable)
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

    fn create_test_worker(name: &str, dob: Option<NaiveDate>) -> Worker {
        Worker {
            id: uuid::Uuid::new_v4(),
            identifiers: vec![],
            active: true,
            name: HumanName {
                use_type: None,
                family: name.to_string(),
                given: vec!["John".to_string()],
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
    fn test_exact_match_scores_high() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let worker1 = create_test_worker("Smith", dob);
        let worker2 = create_test_worker("Smith", dob);

        let result = scorer.calculate_score(&worker1, &worker2);

        // With NAME (0.30) + DOB (0.25) + GENDER (0.10) = 0.65
        // No address, identifiers, tax_id, or documents, so those contribute 0
        assert!(result.score >= 0.60, "Exact match on name/dob/gender should score >= 0.60, got {}", result.score);
        assert!(!scorer.is_match(result.score)); // 0.65 < threshold of 0.85
        assert_eq!(scorer.classify_match(result.score), MatchQuality::Possible);
    }

    #[test]
    fn test_fuzzy_match_scores_moderate() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob1 = NaiveDate::from_ymd_opt(1980, 1, 15);
        let dob2 = NaiveDate::from_ymd_opt(1980, 1, 16); // One day off

        let worker1 = create_test_worker("Smith", dob1);
        let worker2 = create_test_worker("Smyth", dob2); // Spelling variant

        let result = scorer.calculate_score(&worker1, &worker2);

        assert!(result.score > 0.60, "Fuzzy match should score > 0.60, got {}", result.score);
        assert!(result.score < 0.80);
    }

    #[test]
    fn test_no_match_scores_low() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob1 = NaiveDate::from_ymd_opt(1980, 1, 15);
        let dob2 = NaiveDate::from_ymd_opt(1990, 6, 20);

        let worker1 = create_test_worker("Smith", dob1);
        let worker2 = create_test_worker("Johnson", dob2);

        let result = scorer.calculate_score(&worker1, &worker2);

        assert!(result.score < 0.50, "Non-match should score < 0.50, got {}", result.score);
        assert!(!scorer.is_match(result.score));
    }

    #[test]
    fn test_deterministic_exact_match() {
        let config = create_test_config();
        let scorer = DeterministicScorer::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let worker1 = create_test_worker("Smith", dob);
        let worker2 = create_test_worker("Smith", dob);

        let result = scorer.calculate_score(&worker1, &worker2);

        assert!(result.score >= 0.75, "Exact match should meet deterministic threshold");
        assert!(scorer.is_match(result.score));
    }

    #[test]
    fn test_match_quality_classification() {
        assert_eq!(ProbabilisticScorer::new(create_test_config())
            .classify_match(0.98), MatchQuality::Definite);

        assert_eq!(ProbabilisticScorer::new(create_test_config())
            .classify_match(0.87), MatchQuality::Probable);

        assert_eq!(ProbabilisticScorer::new(create_test_config())
            .classify_match(0.60), MatchQuality::Possible);

        assert_eq!(ProbabilisticScorer::new(create_test_config())
            .classify_match(0.30), MatchQuality::Unlikely);
    }

    #[test]
    fn test_probabilistic_all_fields_match() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob = NaiveDate::from_ymd_opt(1980, 1, 15);
        let mut worker1 = create_test_worker("Smith", dob);
        let mut worker2 = create_test_worker("Smith", dob);

        // Add matching addresses
        let addr = crate::models::Address {
            use_type: None,
            line1: Some("123 Main St".into()),
            line2: None,
            city: Some("Springfield".into()),
            state: Some("IL".into()),
            postal_code: Some("62701".into()),
            country: None,
        };
        worker1.addresses = vec![addr.clone()];
        worker2.addresses = vec![addr];

        // Add matching identifiers
        let id = crate::models::Identifier::mrn("hospital-a".into(), "MRN-001".into());
        worker1.identifiers = vec![id.clone()];
        worker2.identifiers = vec![id];

        let result = scorer.calculate_score(&worker1, &worker2);
        assert!(result.score > 0.80, "All fields matching should score very high, got {}", result.score);
    }

    #[test]
    fn test_probabilistic_no_fields_match() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let mut worker1 = create_test_worker("Smith", NaiveDate::from_ymd_opt(1980, 1, 15));
        worker1.gender = Gender::Male;
        let mut worker2 = create_test_worker("Johnson", NaiveDate::from_ymd_opt(1995, 8, 22));
        worker2.gender = Gender::Female;

        let result = scorer.calculate_score(&worker1, &worker2);
        assert!(result.score < 0.30, "No matching fields should score very low, got {}", result.score);
        assert!(!scorer.is_match(result.score));
    }

    #[test]
    fn test_probabilistic_partial_match() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        // Same name but different DOB
        let worker1 = create_test_worker("Smith", NaiveDate::from_ymd_opt(1980, 1, 15));
        let worker2 = create_test_worker("Smith", NaiveDate::from_ymd_opt(1990, 6, 20));

        let result = scorer.calculate_score(&worker1, &worker2);
        assert!(result.score > 0.30, "Name match alone should contribute some score, got {}", result.score);
        assert!(result.score < 0.80, "Only name match should not score too high, got {}", result.score);
    }

    #[test]
    fn test_deterministic_tax_id_match_short_circuits() {
        let config = create_test_config();
        let scorer = DeterministicScorer::new(config);

        let mut worker1 = create_test_worker("Smith", NaiveDate::from_ymd_opt(1980, 1, 15));
        worker1.tax_id = Some("123-45-6789".into());
        let mut worker2 = create_test_worker("Jones", NaiveDate::from_ymd_opt(1995, 12, 1));
        worker2.tax_id = Some("123-45-6789".into());

        let result = scorer.calculate_score(&worker1, &worker2);
        assert_eq!(result.score, 1.0, "Tax ID match should short-circuit to 1.0");
        assert_eq!(result.breakdown.tax_id_score, 1.0);
    }

    #[test]
    fn test_deterministic_identifier_match() {
        let config = create_test_config();
        let scorer = DeterministicScorer::new(config);

        let id = crate::models::Identifier::ssn("123-45-6789".into());
        let mut worker1 = create_test_worker("Smith", NaiveDate::from_ymd_opt(1980, 1, 15));
        worker1.identifiers = vec![id.clone()];
        let mut worker2 = create_test_worker("Jones", NaiveDate::from_ymd_opt(1995, 12, 1));
        worker2.identifiers = vec![id];

        let result = scorer.calculate_score(&worker1, &worker2);
        assert_eq!(result.score, 1.0, "Exact identifier match should short-circuit to 1.0");
    }

    #[test]
    fn test_score_boundary_0_95() {
        let scorer = ProbabilisticScorer::new(create_test_config());
        assert_eq!(scorer.classify_match(0.95), MatchQuality::Definite);
        assert_eq!(scorer.classify_match(0.949), MatchQuality::Probable);
    }

    #[test]
    fn test_score_boundary_0_70() {
        let config = MatchingConfig {
            threshold_score: 0.70,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let scorer = ProbabilisticScorer::new(config);
        assert!(scorer.is_match(0.70), "Score at threshold should be a match");
        assert!(!scorer.is_match(0.69), "Score below threshold should not be a match");
        assert_eq!(scorer.classify_match(0.70), MatchQuality::Probable);
    }
}
