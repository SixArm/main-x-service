//! Match scoring calculations.
//!
//! Combines the per-component scores from
//! [`algorithms`](crate::matching::algorithms) into a single overall match
//! score. Two strategies live here:
//!
//! - [`ProbabilisticScorer`] — a weighted sum across name, birth date,
//!   gender, address, identifier, tax ID, and document, with deterministic
//!   short-circuits for an exact tax-ID or document-number match.
//! - [`DeterministicScorer`] — a rule-based points system where strong
//!   identity signals short-circuit to a perfect score and weaker rules
//!   contribute fractional confidence.
//!
//! Both produce a [`MatchResult`] carrying the score and a full
//! [`MatchScoreBreakdown`]; [`MatchQuality`] buckets a score for display.

use super::algorithms::{
    address_matching, dob_matching, document_matching, gender_matching, identifier_matching,
    name_matching, tax_id_matching,
};
use super::{MatchResult, MatchScoreBreakdown};
use crate::config::MatchingConfig;
use crate::models::Worker;

/// Weighted, fuzzy ("probabilistic") scoring strategy.
///
/// Each component contributes its `[0,1]` score times a fixed weight; the
/// weights sum to 1.0 so the overall score is also in `[0,1]`. Strong
/// deterministic signals (tax ID, document number) bypass the weighting.
pub struct ProbabilisticScorer {
    /// Threshold/score configuration. Only `threshold_score` is consulted at
    /// runtime (by `is_match` / `classify_match`); the component weights are
    /// fixed `const`s in `calculate_score`, not read from here.
    config: MatchingConfig,
}

impl ProbabilisticScorer {
    /// Creates a scorer that uses the given threshold configuration.
    #[must_use]
    pub fn new(config: MatchingConfig) -> Self {
        Self { config }
    }

    /// Scores `candidate` against `worker`, returning the overall score plus
    /// a per-component breakdown.
    ///
    /// An exact tax-ID match short-circuits to 1.0 and an exact document
    /// match to 0.98; otherwise the result is the weighted sum (name 0.30,
    /// birth date 0.25, gender/address/identifier/tax-ID 0.10 each, document
    /// 0.05).
    #[must_use]
    pub fn calculate_score(&self, worker: &Worker, candidate: &Worker) -> MatchResult {
        // Weight factors for each component (probabilistic). These sum to 1.0
        // (0.30 + 0.25 + 0.10 + 0.10 + 0.10 + 0.10 + 0.05) so the weighted total
        // is itself a value in [0, 1]. Name and birth date carry the most weight
        // because together they are the strongest demographic discriminators;
        // gender/address/identifier/tax-ID are corroborating evidence at 0.10
        // each, and document is the lightest at 0.05 (it rarely differs between
        // records that already agree on the others). Keep these in sync with the
        // table in `agents/matching.md`.
        const NAME_WEIGHT: f64 = 0.30;
        const DOB_WEIGHT: f64 = 0.25;
        const GENDER_WEIGHT: f64 = 0.10;
        const ADDRESS_WEIGHT: f64 = 0.10;
        const IDENTIFIER_WEIGHT: f64 = 0.10;
        const TAX_ID_WEIGHT: f64 = 0.10;
        const DOCUMENT_WEIGHT: f64 = 0.05;

        // Calculate individual component scores
        let name_score = name_matching::match_names(&worker.name, &candidate.name);

        let birth_date_score =
            dob_matching::match_birth_dates(worker.birth_date, candidate.birth_date);

        let gender_score = gender_matching::match_gender(worker.gender, candidate.gender);

        let address_score =
            address_matching::match_addresses(&worker.addresses, &candidate.addresses);

        let identifier_score =
            identifier_matching::match_identifiers(&worker.identifiers, &candidate.identifiers);

        let tax_id_score = tax_id_matching::match_tax_ids(worker, candidate);

        let document_score =
            document_matching::match_documents(&worker.documents, &candidate.documents);

        // Tax ID exact match is a strong deterministic signal — short-circuit.
        // A government-issued tax ID is unique per person, so an exact match
        // pins the overall score to a perfect 1.0 regardless of the weaker
        // fuzzy components. The component scores are still recorded in the
        // breakdown for transparency in the API response / review queue.
        if tax_id_score >= 1.0 {
            return MatchResult {
                worker: candidate.clone(),
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

        // Document number exact match is also a strong signal — short-circuit.
        // Capped at 0.98 (not 1.0) because a document number is slightly weaker
        // evidence than a tax ID: numbers can be re-used across document types /
        // issuers and are more prone to transcription collisions, so we leave a
        // small margin below "certain".
        if document_score >= 1.0 {
            return MatchResult {
                worker: candidate.clone(),
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

    /// Returns `true` when `score` meets the configured threshold.
    #[must_use]
    pub fn is_match(&self, score: f64) -> bool {
        score >= self.config.threshold_score
    }

    /// Buckets `score` into a [`MatchQuality`]: Definite (≥0.95), Probable
    /// (≥ threshold), Possible (≥0.50), else Unlikely.
    #[must_use]
    pub fn classify_match(&self, score: f64) -> MatchQuality {
        // Bucket boundaries, checked high-to-low so the first satisfied arm
        // wins: 0.95 is the fixed "certain" line (auto-merge-worthy), the
        // configured threshold (default 0.85) separates Probable from the
        // review-only Possible band, and 0.50 is the floor below which a pair is
        // Unlikely. These bands are surfaced verbatim in the review queue.
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
/// Applies an ordered set of rules: an exact tax ID, identifier, or document
/// match short-circuits to a perfect score, while name+DOB+gender agreement
/// and address agreement each contribute fractional points. The final score
/// is points earned over points available.
pub struct DeterministicScorer {
    /// Held for API symmetry with [`ProbabilisticScorer`]; the deterministic
    /// rules use fixed thresholds rather than the configured score, so the
    /// field is currently unused (hence the leading underscore).
    _config: MatchingConfig,
}

impl DeterministicScorer {
    /// Creates a deterministic scorer. The config is retained but the rule
    /// thresholds are fixed constants.
    #[must_use]
    pub fn new(config: MatchingConfig) -> Self {
        Self { _config: config }
    }

    /// Scores `candidate` against `worker` using the rule set.
    ///
    /// Rules 0/1/1b (tax ID, identifier, document exact match) short-circuit
    /// to 1.0. Otherwise each of name (≥0.90), DOB (≥0.95), and gender (=1.0)
    /// earns one point out of three available, and — when both records have
    /// addresses — a strong address match (≥0.80) earns a fourth point. The
    /// returned score is earned/available.
    #[must_use]
    pub fn calculate_score(&self, worker: &Worker, candidate: &Worker) -> MatchResult {
        // `total_score` accumulates points earned; `points_available` the
        // points in play. The final score is the ratio, so a rule that does not
        // apply (e.g. no addresses on either side) is excluded from both —
        // neither rewarding nor penalising its absence.
        let mut total_score = 0.0;
        let mut points_available = 0.0;

        // Rule 0: Tax ID exact match = definite match. A unique government ID
        // overrides every fuzzy signal, so short-circuit straight to 1.0.
        let tax_id_score = tax_id_matching::match_tax_ids(worker, candidate);
        if tax_id_score >= 1.0 {
            return MatchResult {
                worker: candidate.clone(),
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

        // Rule 1: Exact identifier match = definite match. The 0.98 bar admits
        // both a perfect value match (1.0) and a formatting-only difference
        // (0.98, e.g. "123-45-6789" vs "123456789") — both require type + system
        // to already agree, so either is conclusive enough to short-circuit.
        let identifier_score =
            identifier_matching::match_identifiers(&worker.identifiers, &candidate.identifiers);

        if identifier_score >= 0.98 {
            return MatchResult {
                worker: candidate.clone(),
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

        // Rule 1b: Document number exact match = definite match. Here the bar is
        // a strict 1.0 (same type AND same number AND same issuing country); a
        // same-number/different-country pair scores 0.95 and is intentionally
        // NOT treated as a deterministic short-circuit.
        let document_score =
            document_matching::match_documents(&worker.documents, &candidate.documents);

        if document_score >= 1.0 {
            return MatchResult {
                worker: candidate.clone(),
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

        // Rule 2: Name + DOB + Gender each contribute one point of three. The
        // per-component bars are deliberately strict (a near-miss earns nothing,
        // unlike the probabilistic sum): name ≥ 0.90 (strong fuzzy agreement),
        // DOB ≥ 0.95 (exact or a one-/two-day typo), gender exactly 1.0 (an exact
        // match — "Unknown" scores 0.5 and so never earns the point).
        let name_score = name_matching::match_names(&worker.name, &candidate.name);
        let dob_score = dob_matching::match_birth_dates(worker.birth_date, candidate.birth_date);
        let gender_score = gender_matching::match_gender(worker.gender, candidate.gender);

        // These three rules are always in play, so three points are available.
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

        // Rule 3: Address is optional but adds confidence. Only counted when
        // BOTH records carry an address — otherwise its absence must not dilute
        // the ratio — and a strong (≥ 0.80) match earns the fourth point.
        let address_score =
            address_matching::match_addresses(&worker.addresses, &candidate.addresses);

        if !worker.addresses.is_empty() && !candidate.addresses.is_empty() {
            points_available += 1.0;
            if address_score >= 0.80 {
                total_score += 1.0;
            }
        }

        // Final score is the fraction of available points earned (e.g. 3 of 4 =
        // 0.75). Guard against a zero denominator, which cannot occur here
        // (Rule 2 always adds 3.0) but keeps the division total.
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

    /// Returns `true` when `score` clears the deterministic bar of 0.75
    /// (at least three of four rules satisfied).
    #[must_use]
    pub fn is_match(&self, score: f64) -> bool {
        score >= 0.75 // Require at least 3/4 rules to match
    }
}

/// Coarse confidence bucket for a match score, used in API responses and the
/// review queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchQuality {
    /// Definite match (score >= 0.95). Confident enough to auto-merge.
    Definite,
    /// Probable match (score >= the configured threshold, default 0.85, but
    /// below 0.95). Treated as a match but typically routed for review.
    Probable,
    /// Possible match (score >= 0.50 but below threshold). Not a match;
    /// surfaced as a weak candidate for human review.
    Possible,
    /// Unlikely match (score < 0.50). Effectively a non-match.
    Unlikely,
}

impl MatchQuality {
    /// Returns the lower-case wire string for this quality
    /// ("definite"/"probable"/"possible"/"unlikely").
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            MatchQuality::Definite => "definite",
            MatchQuality::Probable => "probable",
            MatchQuality::Possible => "possible",
            MatchQuality::Unlikely => "unlikely",
        }
    }

    /// Returns `true` for qualities treated as a match (Definite or Probable).
    #[must_use]
    pub fn is_match(&self) -> bool {
        matches!(self, MatchQuality::Definite | MatchQuality::Probable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Gender, HumanName};
    use chrono::NaiveDate;

    /// Builds a matching config with a 0.85 threshold for the tests.
    fn create_test_config() -> MatchingConfig {
        MatchingConfig {
            threshold_score: 0.85,
            exact_match_score: 1.0,
            fuzzy_match_score: 0.8,
        }
    }

    /// Builds a minimal male "John <name>" worker with the given birth date.
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

    /// Matching name/DOB/gender (but nothing else) lands in the Possible band.
    #[test]
    fn test_exact_match_scores_high() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap());
        let worker1 = create_test_worker("Smith", dob);
        let worker2 = create_test_worker("Smith", dob);

        let result = scorer.calculate_score(&worker1, &worker2);

        // With NAME (0.30) + DOB (0.25) + GENDER (0.10) = 0.65
        // No address, identifiers, tax_id, or documents, so those contribute 0
        assert!(
            result.score >= 0.60,
            "Exact match on name/dob/gender should score >= 0.60, got {}",
            result.score
        );
        assert!(!scorer.is_match(result.score)); // 0.65 < threshold of 0.85
        assert_eq!(scorer.classify_match(result.score), MatchQuality::Possible);
    }

    /// A name typo plus a one-day DOB slip yields a moderate score.
    #[test]
    fn test_fuzzy_match_scores_moderate() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob1 = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap());
        let dob2 = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 16).unwrap()); // One day off

        let worker1 = create_test_worker("Smith", dob1);
        let worker2 = create_test_worker("Smyth", dob2); // Spelling variant

        let result = scorer.calculate_score(&worker1, &worker2);

        assert!(
            result.score > 0.60,
            "Fuzzy match should score > 0.60, got {}",
            result.score
        );
        assert!(result.score < 0.80);
    }

    /// Different name and DOB score below the Possible band.
    #[test]
    fn test_no_match_scores_low() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob1 = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap());
        let dob2 = Some(chrono::NaiveDate::from_ymd_opt(1990, 6, 20).unwrap());

        let worker1 = create_test_worker("Smith", dob1);
        let worker2 = create_test_worker("Johnson", dob2);

        let result = scorer.calculate_score(&worker1, &worker2);

        assert!(
            result.score < 0.50,
            "Non-match should score < 0.50, got {}",
            result.score
        );
        assert!(!scorer.is_match(result.score));
    }

    /// Deterministic scoring on a clean name/DOB/gender match clears 0.75.
    #[test]
    fn test_deterministic_exact_match() {
        let config = create_test_config();
        let scorer = DeterministicScorer::new(config);

        let dob = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap());
        let worker1 = create_test_worker("Smith", dob);
        let worker2 = create_test_worker("Smith", dob);

        let result = scorer.calculate_score(&worker1, &worker2);

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

    /// Name, DOB, gender, address, and identifier all matching scores high.
    #[test]
    fn test_probabilistic_all_fields_match() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let dob = Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap());
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
        let id = crate::models::Identifier::mrn("hospital-a", "MRN-001".into());
        worker1.identifiers = vec![id.clone()];
        worker2.identifiers = vec![id];

        let result = scorer.calculate_score(&worker1, &worker2);
        assert!(
            result.score > 0.80,
            "All fields matching should score very high, got {}",
            result.score
        );
    }

    /// Divergent name, DOB, and gender score very low.
    #[test]
    fn test_probabilistic_no_fields_match() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        let mut worker1 = create_test_worker(
            "Smith",
            Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap()),
        );
        worker1.gender = Gender::Male;
        let mut worker2 = create_test_worker(
            "Johnson",
            Some(chrono::NaiveDate::from_ymd_opt(1995, 8, 22).unwrap()),
        );
        worker2.gender = Gender::Female;

        let result = scorer.calculate_score(&worker1, &worker2);
        assert!(
            result.score < 0.30,
            "No matching fields should score very low, got {}",
            result.score
        );
        assert!(!scorer.is_match(result.score));
    }

    /// A name match with a divergent DOB lands in the middle of the range.
    #[test]
    fn test_probabilistic_partial_match() {
        let config = create_test_config();
        let scorer = ProbabilisticScorer::new(config);

        // Same name but different DOB
        let worker1 = create_test_worker(
            "Smith",
            Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap()),
        );
        let worker2 = create_test_worker(
            "Smith",
            Some(chrono::NaiveDate::from_ymd_opt(1990, 6, 20).unwrap()),
        );

        let result = scorer.calculate_score(&worker1, &worker2);
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

    /// A shared tax ID forces a 1.0 score even when names/DOBs differ.
    #[test]
    fn test_deterministic_tax_id_match_short_circuits() {
        let config = create_test_config();
        let scorer = DeterministicScorer::new(config);

        let mut worker1 = create_test_worker(
            "Smith",
            Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap()),
        );
        worker1.tax_id = Some("123-45-6789".into());
        let mut worker2 = create_test_worker(
            "Jones",
            Some(chrono::NaiveDate::from_ymd_opt(1995, 12, 1).unwrap()),
        );
        worker2.tax_id = Some("123-45-6789".into());

        let result = scorer.calculate_score(&worker1, &worker2);
        assert!(
            (result.score - 1.0).abs() < f64::EPSILON,
            "Tax ID match should short-circuit to 1.0"
        );
        assert!((result.breakdown.tax_id_score - 1.0).abs() < f64::EPSILON);
    }

    /// A shared exact identifier short-circuits to a 1.0 score.
    #[test]
    fn test_deterministic_identifier_match() {
        let config = create_test_config();
        let scorer = DeterministicScorer::new(config);

        let id = crate::models::Identifier::ssn("123-45-6789".into());
        let mut worker1 = create_test_worker(
            "Smith",
            Some(chrono::NaiveDate::from_ymd_opt(1980, 1, 15).unwrap()),
        );
        worker1.identifiers = vec![id.clone()];
        let mut worker2 = create_test_worker(
            "Jones",
            Some(chrono::NaiveDate::from_ymd_opt(1995, 12, 1).unwrap()),
        );
        worker2.identifiers = vec![id];

        let result = scorer.calculate_score(&worker1, &worker2);
        assert!(
            (result.score - 1.0).abs() < f64::EPSILON,
            "Exact identifier match should short-circuit to 1.0"
        );
    }

    /// The Definite/Probable boundary sits exactly at 0.95.
    #[test]
    fn test_score_boundary_0_95() {
        let scorer = ProbabilisticScorer::new(create_test_config());
        assert_eq!(scorer.classify_match(0.95), MatchQuality::Definite);
        assert_eq!(scorer.classify_match(0.949), MatchQuality::Probable);
    }

    /// `is_match` is inclusive at the configured threshold (here 0.70).
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
