//! Match evidence: Fellegi–Sunter log-likelihood weights beside the
//! similarity score.
//!
//! The similarity `score` answers "how alike is the data both records
//! hold?". It cannot tell "nothing here contradicts a match" from "the
//! data proves a match": two sparse records holding only the same title
//! score `1.0`, and so would two different appeals filed under one
//! template title. [`Evidence`] answers the second question, in bits: each
//! bit doubles the odds that the pair is one case. A missing component
//! contributes exactly `0`. The family-wide contract is
//! `agents/share/match-evidence.md`; this file is this crate's copy of it
//! plus its own `m`/`u` table.

use serde::{Deserialize, Serialize};

use crate::scoring::MatchBreakdown;

/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;
/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// Bits one shared globally-unique identifier contributes, once per pair:
/// `log2(0.99 / 1e-12)` ≈ 39.85. Clears the default `proof_bits` alone.
pub const IDENTIFIER_BITS: f64 = 39.848_637_568_953_24;

/// Bits each contradicted identifier scheme contributes:
/// `log2(0.01 / (1 − 1e-12))` ≈ −6.64.
pub const CONTRADICTION_BITS: f64 = -6.643_856_189_773_282;

/// Default evidence bar for [`MatchStrength::Proven`]: `log2(N) +
/// log2(1/α)` for a million records at one false merge in a thousand.
pub const DEFAULT_PROOF_BITS: f64 = 30.0;

/// Per-component `(m, u)`: `m` is the chance the component agrees when the
/// records are one case, `u` the chance it agrees by coincidence when they
/// are different cases. Informed estimates, not measurements
/// (`agents/share/match-evidence.md` §5).
///
/// Title (Jaro-Winkler). `u`: agencies reuse template titles ("Housing
/// benefit appeal — …"), so a near-identical title for an unrelated case
/// is roughly one in two hundred, not one in a million.
pub const TITLE_M_U: (f64, f64) = (0.95, 0.005);
/// Subjects (Jaccard over involved parties). `u`: a shared party list
/// between unrelated cases is about one in a hundred (the same claimant
/// files several matters); `m` below `0.9` because parties are added and
/// removed over a case's life.
pub const SUBJECTS_M_U: (f64, f64) = (0.85, 0.01);
/// Same-agency case number. `u`: one agency's numbering collides with
/// itself about one time in a thousand (re-keyed or recycled numbers);
/// `m` allows for transcription errors.
pub const CASE_NUMBER_M_U: (f64, f64) = (0.95, 0.001);
/// Case type (categorical). `u`: about a dozen types, unevenly used, so
/// two unrelated cases share one about 15% of the time.
pub const CASE_TYPE_M_U: (f64, f64) = (0.95, 0.15);
/// Status (categorical). `u`: a handful of statuses, most cases open or
/// closed, so about one in four; `m` low because status changes over time.
pub const STATUS_M_U: (f64, f64) = (0.80, 0.25);
/// Keywords (Jaccard). `u`: agencies tag from a small vocabulary, so
/// overlapping tag sets are common (about 5%); `m` low because tags drift.
/// Set overlap corroborates; it does not identify.
pub const KEYWORDS_M_U: (f64, f64) = (0.80, 0.05);

/// How strong the evidence for a match is (contract §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStrength {
    /// A contradiction rules the pair out of any automatic merge.
    Contradicted,
    /// The evidence clears `proof_bits`, with no contradiction.
    Proven,
    /// `is_match`, no contradiction, but the evidence is short of proof.
    Consistent,
    /// Not a match.
    #[default]
    NonMatch,
}

/// One hard conflict between the two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contradiction {
    /// What conflicts, e.g. `"identifier:docket"`.
    pub field: String,
}

/// How much the compared data proves, beside the similarity score.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Evidence {
    /// Sum of the components' log-likelihood weights, in bits.
    pub weight_bits: f64,
    /// Components both records held (identifiers count as one).
    pub fields_compared: u32,
    /// Hard conflicts; any one makes the strength `Contradicted`.
    pub contradictions: Vec<Contradiction>,
    /// The verdict (§4).
    pub strength: MatchStrength,
}

/// The weight, in bits, one compared component adds.
///
/// `similarity` at or above [`AGREE_AT`] adds `log2(m/u)`; at or below
/// [`DISAGREE_AT`] adds `log2((1−m)/(1−u))` (negative); in between it is
/// interpolated linearly. A non-finite similarity counts as `0.0`.
#[must_use]
pub fn component_bits(similarity: f64, m: f64, u: f64) -> f64 {
    let agree = (m / u).log2();
    let disagree = ((1.0 - m) / (1.0 - u)).log2();
    let s = if similarity.is_finite() {
        similarity.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if s >= AGREE_AT {
        agree
    } else if s <= DISAGREE_AT {
        disagree
    } else {
        disagree + (agree - disagree) * (s - DISAGREE_AT) / (AGREE_AT - DISAGREE_AT)
    }
}

/// Assemble the [`Evidence`] for one pair.
///
/// `breakdown` holds the component similarities (`None` adds nothing and
/// is not counted). `identifier_shared` is whether the deterministic
/// short-circuit fired; it adds [`IDENTIFIER_BITS`] once and counts as one
/// field. When it fired, an agreeing case number is the identifier itself
/// (rule R-1) and is not weighed a second time. `contradictions` each add
/// [`CONTRADICTION_BITS`]. `is_match` and `proof_bits` decide the strength.
#[must_use]
pub fn assess(
    breakdown: &MatchBreakdown,
    identifier_shared: bool,
    contradictions: Vec<Contradiction>,
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let case_number = if identifier_shared && breakdown.case_number_score == Some(1.0) {
        None
    } else {
        breakdown.case_number_score
    };
    let components = [
        (breakdown.title_score, TITLE_M_U),
        (breakdown.subjects_score, SUBJECTS_M_U),
        (case_number, CASE_NUMBER_M_U),
        (breakdown.case_type_score, CASE_TYPE_M_U),
        (breakdown.status_score, STATUS_M_U),
        (breakdown.keywords_score, KEYWORDS_M_U),
    ];
    let mut weight_bits = 0.0_f64;
    let mut fields_compared = 0_u32;
    for (similarity, (m, u)) in components {
        if let Some(s) = similarity {
            weight_bits += component_bits(s, m, u);
            fields_compared += 1;
        }
    }
    if identifier_shared {
        weight_bits += IDENTIFIER_BITS;
        fields_compared += 1;
    }
    for _ in &contradictions {
        weight_bits += CONTRADICTION_BITS;
    }
    let strength = if !contradictions.is_empty() {
        MatchStrength::Contradicted
    } else if weight_bits >= proof_bits {
        MatchStrength::Proven
    } else if is_match {
        MatchStrength::Consistent
    } else {
        MatchStrength::NonMatch
    };
    Evidence {
        weight_bits,
        fields_compared,
        contradictions,
        strength,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_their_formulas() {
        assert!((IDENTIFIER_BITS - (0.99_f64 / 1e-12).log2()).abs() < 1e-9);
        assert!((CONTRADICTION_BITS - (0.01_f64 / (1.0 - 1e-12)).log2()).abs() < 1e-9);
    }

    #[test]
    fn component_bits_agree_disagree_and_interpolate() {
        let (m, u) = TITLE_M_U;
        let agree = (m / u).log2();
        let disagree = ((1.0 - m) / (1.0 - u)).log2();
        assert!((component_bits(1.0, m, u) - agree).abs() < 1e-12);
        assert!((component_bits(0.0, m, u) - disagree).abs() < 1e-12);
        assert!(disagree < 0.0 && agree > 0.0);
        let mid = component_bits(0.775, m, u);
        assert!(mid > disagree && mid < agree);
        // Non-finite similarity is treated as full disagreement, not NaN.
        assert!((component_bits(f64::NAN, m, u) - disagree).abs() < 1e-12);
    }

    #[test]
    fn missing_component_adds_zero_and_is_not_counted() {
        let with_title = MatchBreakdown {
            title_score: Some(1.0),
            ..Default::default()
        };
        let with_status_missing = assess(&with_title, false, Vec::new(), true, 30.0);
        assert_eq!(with_status_missing.fields_compared, 1);
        let (m, u) = TITLE_M_U;
        assert!((with_status_missing.weight_bits - (m / u).log2()).abs() < 1e-12);

        let empty = assess(&MatchBreakdown::default(), false, Vec::new(), false, 30.0);
        assert_eq!(empty.fields_compared, 0);
        assert!(empty.weight_bits.abs() < 1e-12);
        assert_eq!(empty.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn contradiction_wins_over_proof() {
        let e = assess(
            &MatchBreakdown::default(),
            true,
            vec![Contradiction {
                field: "identifier:docket".into(),
            }],
            true,
            30.0,
        );
        assert_eq!(e.strength, MatchStrength::Contradicted);
        assert!((e.weight_bits - (IDENTIFIER_BITS + CONTRADICTION_BITS)).abs() < 1e-9);
    }

    #[test]
    fn old_json_without_evidence_deserialises() {
        let json = r#"{"score":0.5,"is_match":false,"confidence":"Low",
            "breakdown":{"title_score":0.5,"subjects_score":null,
            "case_number_score":null,"case_type_score":null,
            "status_score":null,"keywords_score":null,
            "deterministic_match":false}}"#;
        let r: crate::MatchResult = serde_json::from_str(json).expect("old JSON parses");
        assert_eq!(r.evidence, Evidence::default());
        assert_eq!(r.evidence.strength, MatchStrength::NonMatch);

        let cfg: crate::MatchConfig = serde_json::from_str(
            r#"{"threshold":0.85,"title_weight":0.3,"subjects_weight":0.25,
            "case_number_weight":0.15,"case_type_weight":0.1,
            "status_weight":0.05,"keywords_weight":0.15}"#,
        )
        .expect("old config parses");
        assert!((cfg.proof_bits - DEFAULT_PROOF_BITS).abs() < 1e-12);
    }
}
