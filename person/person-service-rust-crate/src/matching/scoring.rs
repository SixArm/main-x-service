//! Match scoring calculations.
//!
//! This module turns the per-field scores from
//! [`crate::matching::algorithms`] into an overall match score for a
//! pair of persons. Two strategies live here:
//!
//! - [`ProbabilisticScorer`](crate::matching::scoring::ProbabilisticScorer) — a weighted average over the components
//!   that are *present on both records*, with the absent components
//!   dropped from both numerator and denominator (renormalization), so
//!   sparse records are not unfairly penalized. Tax-ID and document
//!   exact matches short-circuit to near-certain scores.
//! - [`DeterministicScorer`](crate::matching::scoring::DeterministicScorer) — rule-based points: a tax-ID / identifier
//!   / document exact match short-circuits to `1.0`; otherwise it awards
//!   one point each for strong name, DOB, and gender agreement (plus an
//!   optional address point) and divides by the points available.
//!
//! [`MatchQuality`] buckets a numeric score into definite / probable /
//! possible / unlikely for human-facing classification.

use super::algorithms::{
    address_matching, dob_matching, document_matching, gender_matching, identifier_matching,
    name_matching, tax_id_matching,
};
use super::{MatchResult, MatchScoreBreakdown};
use crate::config::MatchingConfig;
use crate::models::Person;

/// Weighted-average ("probabilistic") scoring strategy.
///
/// Combines fuzzy per-field scores into a single confidence value in
/// `[0.0, 1.0]` using the configured threshold to decide matches.
pub struct ProbabilisticScorer {
    /// Configuration for matching thresholds and weights
    config: MatchingConfig,
}

impl ProbabilisticScorer {
    /// Create a new probabilistic scorer from a [`MatchingConfig`].
    pub fn new(config: MatchingConfig) -> Self {
        Self { config }
    }

    /// Score a candidate against a reference person.
    ///
    /// Computes all seven component scores, short-circuits to `1.0` on a
    /// tax-ID exact match or `0.98` on a document exact match, and
    /// otherwise returns the weight-renormalized average over the
    /// components present on both records. The full
    /// [`MatchScoreBreakdown`] is always included.
    pub fn calculate_score(&self, person: &Person, candidate: &Person) -> MatchResult {
        // Calculate individual component scores
        let name_score = name_matching::match_names(&person.name, &candidate.name);

        let birth_date_score =
            dob_matching::match_birth_dates(person.birth_date, candidate.birth_date);

        let gender_score = gender_matching::match_gender(person.gender, candidate.gender);

        let address_score =
            address_matching::match_addresses(&person.addresses, &candidate.addresses);

        let identifier_score =
            identifier_matching::match_identifiers(&person.identifiers, &candidate.identifiers);

        let tax_id_score = tax_id_matching::match_tax_ids(person, candidate);

        let document_score =
            document_matching::match_documents(&person.documents, &candidate.documents);

        // Tax ID exact match is a strong deterministic signal — short-circuit
        if tax_id_score >= 1.0 {
            return MatchResult {
                person: candidate.clone(),
                score: 1.0,
                breakdown: MatchScoreBreakdown {
                    name_score,
                    birth_date_score,
                    gender_score,
                    address_score,
                    identifier_score,
                    tax_id_score,
                    document_score,
                },
            };
        }

        // Document number exact match is also a strong signal — short-circuit
        if document_score >= 1.0 {
            return MatchResult {
                person: candidate.clone(),
                score: 0.98,
                breakdown: MatchScoreBreakdown {
                    name_score,
                    birth_date_score,
                    gender_score,
                    address_score,
                    identifier_score,
                    tax_id_score,
                    document_score,
                },
            };
        }

        // Weight factors for each component (probabilistic). When BOTH
        // sides lack the underlying data for a component we drop the
        // component from the sum AND its weight from the denominator
        // so the final score is the weighted average over the present
        // components rather than penalising absent ones. Two records
        // with identical name+DOB+gender (and no address / identifier /
        // tax_id / documents) therefore score 1.0, not 0.65.
        const NAME_WEIGHT: f64 = 0.30;
        const DOB_WEIGHT: f64 = 0.25;
        const GENDER_WEIGHT: f64 = 0.10;
        const ADDRESS_WEIGHT: f64 = 0.10;
        const IDENTIFIER_WEIGHT: f64 = 0.10;
        const TAX_ID_WEIGHT: f64 = 0.10;
        const DOCUMENT_WEIGHT: f64 = 0.05;

        let mut weighted_sum = 0.0_f64;
        let mut weight_sum = 0.0_f64;

        // Name is required, so it always contributes.
        weighted_sum += name_score * NAME_WEIGHT;
        weight_sum += NAME_WEIGHT;

        if person.birth_date.is_some() && candidate.birth_date.is_some() {
            weighted_sum += birth_date_score * DOB_WEIGHT;
            weight_sum += DOB_WEIGHT;
        }

        // Gender is always set (enum has Unknown), so it always contributes.
        weighted_sum += gender_score * GENDER_WEIGHT;
        weight_sum += GENDER_WEIGHT;

        if !person.addresses.is_empty() && !candidate.addresses.is_empty() {
            weighted_sum += address_score * ADDRESS_WEIGHT;
            weight_sum += ADDRESS_WEIGHT;
        }

        if !person.identifiers.is_empty() && !candidate.identifiers.is_empty() {
            weighted_sum += identifier_score * IDENTIFIER_WEIGHT;
            weight_sum += IDENTIFIER_WEIGHT;
        }

        if person.effective_tax_id().is_some() && candidate.effective_tax_id().is_some() {
            weighted_sum += tax_id_score * TAX_ID_WEIGHT;
            weight_sum += TAX_ID_WEIGHT;
        }

        if !person.documents.is_empty() && !candidate.documents.is_empty() {
            weighted_sum += document_score * DOCUMENT_WEIGHT;
            weight_sum += DOCUMENT_WEIGHT;
        }

        let total_score = if weight_sum > 0.0 {
            weighted_sum / weight_sum
        } else {
            0.0
        };

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
            person: candidate.clone(),
            score: total_score,
            breakdown,
        }
    }

    /// Return `true` when `score` meets the configured threshold.
    pub fn is_match(&self, score: f64) -> bool {
        score >= self.config.threshold_score
    }

    /// Bucket a numeric score into a [`MatchQuality`].
    ///
    /// `>= 0.95` is definite, `>= threshold` is probable, `>= 0.50` is
    /// possible, anything lower is unlikely.
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

/// Rule-based ("deterministic") scoring strategy.
///
/// Awards discrete points for strong field agreement and short-circuits
/// to `1.0` on any exact identifier-class match.
pub struct DeterministicScorer {
    /// Configuration for matching (currently unused; reserved for
    /// future rule tuning, hence the leading underscore).
    _config: MatchingConfig,
}

impl DeterministicScorer {
    /// Create a new deterministic scorer from a [`MatchingConfig`].
    pub fn new(config: MatchingConfig) -> Self {
        Self { _config: config }
    }

    /// Score a candidate against a reference person using strict rules.
    ///
    /// Rule 0/1/1b: an exact tax-ID, identifier, or document match
    /// short-circuits to `1.0`. Otherwise Rule 2 awards one point each
    /// for name `>= 0.90`, DOB `>= 0.95`, and exact gender, and Rule 3
    /// adds an optional address point; the final score is points earned
    /// over points available.
    pub fn calculate_score(&self, person: &Person, candidate: &Person) -> MatchResult {
        let mut total_score = 0.0;
        let mut points_available = 0.0;

        // Rule 0: Tax ID exact match = definite match
        let tax_id_score = tax_id_matching::match_tax_ids(person, candidate);
        if tax_id_score >= 1.0 {
            return MatchResult {
                person: candidate.clone(),
                score: 1.0,
                breakdown: MatchScoreBreakdown {
                    name_score: 0.0,
                    birth_date_score: 0.0,
                    gender_score: 0.0,
                    address_score: 0.0,
                    identifier_score: 0.0,
                    tax_id_score,
                    document_score: 0.0,
                },
            };
        }

        // Rule 1: Exact identifier match = definite match
        let identifier_score =
            identifier_matching::match_identifiers(&person.identifiers, &candidate.identifiers);

        if identifier_score >= 0.98 {
            return MatchResult {
                person: candidate.clone(),
                score: 1.0,
                breakdown: MatchScoreBreakdown {
                    name_score: 0.0,
                    birth_date_score: 0.0,
                    gender_score: 0.0,
                    address_score: 0.0,
                    identifier_score,
                    tax_id_score,
                    document_score: 0.0,
                },
            };
        }

        // Rule 1b: Document number exact match = definite match
        let document_score =
            document_matching::match_documents(&person.documents, &candidate.documents);

        if document_score >= 1.0 {
            return MatchResult {
                person: candidate.clone(),
                score: 1.0,
                breakdown: MatchScoreBreakdown {
                    name_score: 0.0,
                    birth_date_score: 0.0,
                    gender_score: 0.0,
                    address_score: 0.0,
                    identifier_score,
                    tax_id_score,
                    document_score,
                },
            };
        }

        // Rule 2: Name + DOB + Gender must all match
        let name_score = name_matching::match_names(&person.name, &candidate.name);
        let dob_score = dob_matching::match_birth_dates(person.birth_date, candidate.birth_date);
        let gender_score = gender_matching::match_gender(person.gender, candidate.gender);

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
        let address_score =
            address_matching::match_addresses(&person.addresses, &candidate.addresses);

        if !person.addresses.is_empty() && !candidate.addresses.is_empty() {
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
            person: candidate.clone(),
            score: final_score,
            breakdown,
        }
    }

    /// Return `true` when a deterministic score meets the `0.75` bar
    /// (at least three of the four rules satisfied).
    pub fn is_match(&self, score: f64) -> bool {
        score >= 0.75 // Require at least 3/4 rules to match
    }
}

/// Human-facing classification of a numeric match score.
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
    /// Lowercase string name (`"definite"`, `"probable"`, …) for API
    /// responses and review-queue records.
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchQuality::Definite => "definite",
            MatchQuality::Probable => "probable",
            MatchQuality::Possible => "possible",
            MatchQuality::Unlikely => "unlikely",
        }
    }

    /// Return `true` for qualities that count as a match (definite or
    /// probable).
    pub fn is_match(&self) -> bool {
        matches!(self, MatchQuality::Definite | MatchQuality::Probable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Gender, HumanName};
    use jiff::civil::Date;

    /// Build a default config with an 0.85 probable threshold.
    fn create_test_config() -> MatchingConfig {
        MatchingConfig {
            threshold_score: 0.85,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        }
    }

    /// Build a minimal male "John <name>" person with the given DOB.
    fn create_test_person(name: &str, dob: Option<Date>) -> Person {
        Person {
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

    /// Identical name+DOB+gender renormalizes to ~1.0 (definite).
    #[test]
    fn test_exact_match_scores_high() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob = Some(jiff::civil::date(1980, 1, 15));
        let person1 = create_test_person("Smith", dob);
        let person2 = create_test_person("Smith", dob);

        let result = scorer.calculate_score(&person1, &person2);

        // Per spec change 2026-06-03, weights renormalise over present
        // components. Identical name + DOB + gender (no address /
        // identifier / tax_id / document on either side) is
        // (1*0.30 + 1*0.25 + 1*0.10) / (0.30 + 0.25 + 0.10) = 1.0.
        assert!(
            result.score >= 0.99,
            "Exact match on name+DOB+gender should renormalise to 1.0, got {}",
            result.score,
        );
        assert!(scorer.is_match(result.score));
        assert_eq!(scorer.classify_match(result.score), MatchQuality::Definite);
    }

    /// Spelling variant + day-off DOB still scores in the probable band.
    #[test]
    fn test_fuzzy_match_scores_moderate() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob1 = Some(jiff::civil::date(1980, 1, 15));
        let dob2 = Some(jiff::civil::date(1980, 1, 16)); // One day off

        let person1 = create_test_person("Smith", dob1);
        let person2 = create_test_person("Smyth", dob2); // Spelling variant

        let result = scorer.calculate_score(&person1, &person2);

        // Post-renormalisation: name ~0.93 (Jaro-Winkler) + DOB ~0.95
        // (day off by 1) + gender 1.0, weighted average over 0.65 of
        // weight, lands in the probable / certain band rather than the
        // possible band the old non-renormalised sum (~0.55) produced.
        assert!(
            result.score > 0.85,
            "Fuzzy near-match should score > 0.85, got {}",
            result.score
        );
        assert!(result.score < 1.0);
    }

    /// Unrelated name + far-apart DOB scores below 0.50.
    #[test]
    fn test_no_match_scores_low() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob1 = Some(jiff::civil::date(1980, 1, 15));
        let dob2 = Some(jiff::civil::date(1990, 6, 20));

        let person1 = create_test_person("Smith", dob1);
        let person2 = create_test_person("Johnson", dob2);

        let result = scorer.calculate_score(&person1, &person2);

        assert!(
            result.score < 0.50,
            "Non-match should score < 0.50, got {}",
            result.score
        );
        assert!(!scorer.is_match(result.score));
    }

    /// Deterministic scorer meets its 0.75 bar on identical records.
    #[test]
    fn test_deterministic_exact_match() {
        let config = create_test_config();
        let scorer = DeterministicScorer::new(config);

        let dob = Some(jiff::civil::date(1980, 1, 15));
        let person1 = create_test_person("Smith", dob);
        let person2 = create_test_person("Smith", dob);

        let result = scorer.calculate_score(&person1, &person2);

        assert!(
            result.score >= 0.75,
            "Exact match should meet deterministic threshold"
        );
        assert!(scorer.is_match(result.score));
    }

    /// Representative scores map to the expected quality buckets.
    #[test]
    fn test_match_quality_classification() {
        assert_eq!(
            ProbabilisticScorer::new(create_test_config()).classify_match(0.98),
            MatchQuality::Definite
        );

        assert_eq!(
            ProbabilisticScorer::new(create_test_config()).classify_match(0.87),
            MatchQuality::Probable
        );

        assert_eq!(
            ProbabilisticScorer::new(create_test_config()).classify_match(0.60),
            MatchQuality::Possible
        );

        assert_eq!(
            ProbabilisticScorer::new(create_test_config()).classify_match(0.30),
            MatchQuality::Unlikely
        );
    }

    /// Matching name+DOB+gender+address+identifier scores very high.
    #[test]
    fn test_probabilistic_all_fields_match() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob = Some(jiff::civil::date(1980, 1, 15));
        let mut person1 = create_test_person("Smith", dob);
        let mut person2 = create_test_person("Smith", dob);

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
        person1.addresses = vec![addr.clone()];
        person2.addresses = vec![addr];

        // Add matching identifiers
        let id = crate::models::Identifier::mrn("hospital-a".into(), "MRN-001".into());
        person1.identifiers = vec![id.clone()];
        person2.identifiers = vec![id];

        let result = scorer.calculate_score(&person1, &person2);
        assert!(
            result.score > 0.80,
            "All fields matching should score very high, got {}",
            result.score
        );
    }

    /// Divergent name+DOB+gender stays well below threshold.
    #[test]
    fn test_probabilistic_no_fields_match() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let mut person1 = create_test_person("Smith", Some(jiff::civil::date(1980, 1, 15)));
        person1.gender = Gender::Male;
        let mut person2 = create_test_person("Johnson", Some(jiff::civil::date(1995, 8, 22)));
        person2.gender = Gender::Female;

        let result = scorer.calculate_score(&person1, &person2);
        // Post-renormalisation: low name (Smith vs Johnson), low DOB
        // (15 years apart), 0 gender — averaged over 0.65 of weight.
        // The result is still well below the 0.85 threshold, but the
        // renormalisation moves it up from the old ~0.10 to ~0.20.
        assert!(
            result.score < 0.40,
            "Unrelated records should score well below threshold, got {}",
            result.score
        );
        assert!(!scorer.is_match(result.score));
    }

    /// Same name but different DOB lands between 0.30 and 0.80.
    #[test]
    fn test_probabilistic_partial_match() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        // Same name but different DOB
        let person1 = create_test_person("Smith", Some(jiff::civil::date(1980, 1, 15)));
        let person2 = create_test_person("Smith", Some(jiff::civil::date(1990, 6, 20)));

        let result = scorer.calculate_score(&person1, &person2);
        assert!(
            result.score > 0.30,
            "Name match alone should contribute some score, got {}",
            result.score
        );
        assert!(
            result.score < 0.80,
            "Only name match should not score too high, got {}",
            result.score
        );
    }

    /// A shared tax ID short-circuits the deterministic scorer to 1.0.
    #[test]
    fn test_deterministic_tax_id_match_short_circuits() {
        let config = create_test_config();
        let scorer = DeterministicScorer::new(config);

        let mut person1 = create_test_person("Smith", Some(jiff::civil::date(1980, 1, 15)));
        person1.tax_id = Some("123-45-6789".into());
        let mut person2 = create_test_person("Jones", Some(jiff::civil::date(1995, 12, 1)));
        person2.tax_id = Some("123-45-6789".into());

        let result = scorer.calculate_score(&person1, &person2);
        assert_eq!(
            result.score, 1.0,
            "Tax ID match should short-circuit to 1.0"
        );
        assert_eq!(result.breakdown.tax_id_score, 1.0);
    }

    /// A shared exact identifier short-circuits to 1.0.
    #[test]
    fn test_deterministic_identifier_match() {
        let config = create_test_config();
        let scorer = DeterministicScorer::new(config);

        let id = crate::models::Identifier::ssn("123-45-6789".into());
        let mut person1 = create_test_person("Smith", Some(jiff::civil::date(1980, 1, 15)));
        person1.identifiers = vec![id.clone()];
        let mut person2 = create_test_person("Jones", Some(jiff::civil::date(1995, 12, 1)));
        person2.identifiers = vec![id];

        let result = scorer.calculate_score(&person1, &person2);
        assert_eq!(
            result.score, 1.0,
            "Exact identifier match should short-circuit to 1.0"
        );
    }

    /// The 0.95 definite/probable boundary is inclusive at 0.95.
    #[test]
    fn test_score_boundary_0_95() {
        let scorer = ProbabilisticScorer::new(create_test_config());
        assert_eq!(scorer.classify_match(0.95), MatchQuality::Definite);
        assert_eq!(scorer.classify_match(0.949), MatchQuality::Probable);
    }

    /// A 0.70 configured threshold is inclusive at exactly 0.70.
    #[test]
    fn test_score_boundary_0_70() {
        let config = MatchingConfig {
            threshold_score: 0.70,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        };
        let scorer = ProbabilisticScorer::new(config);
        assert!(
            scorer.is_match(0.70),
            "Score at threshold should be a match"
        );
        assert!(
            !scorer.is_match(0.69),
            "Score below threshold should not be a match"
        );
        assert_eq!(scorer.classify_match(0.70), MatchQuality::Probable);
    }
}
