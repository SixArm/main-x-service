//! Match evidence: Fellegi–Sunter log-likelihood weights beside the
//! similarity score.
//!
//! [`crate::MatchResult::score`] answers "how alike is the data both
//! records hold?". It renormalises over the components both sides carry,
//! so two sparse records that agree on everything they hold score `1.0`
//! — and so do twins. [`Evidence`] answers the other question: "how much
//! does the data *prove*?". Each compared component adds
//! `log2(m / u)` bits on agreement and `log2((1 − m) / (1 − u))` bits on
//! disagreement; a component either record lacks adds exactly `0`.
//!
//! The family-wide contract is `agents/share/match-evidence.md` in the
//! monorepo; this module is this crate's copy of it plus the worker
//! matcher's own `m`/`u` table.
//!
//! ```
//! use worker_matcher::{MatchingEngine, MatchStrength, Worker};
//! use chrono::NaiveDate;
//!
//! // Same name, birth date and gender: consistent with one worker, but
//! // twins would look exactly the same. Not proof.
//! let a = Worker::builder()
//!     .given_name("Carys")
//!     .family_name("Pritchard")
//!     .date_of_birth(NaiveDate::from_ymd_opt(1985, 1, 1).unwrap())
//!     .build();
//! let r = MatchingEngine::default_config().match_workers(&a, &a.clone());
//! assert!(r.is_match);
//! assert_eq!(r.evidence.strength, MatchStrength::Consistent);
//!
//! // A shared NHS number is proof.
//! let b = Worker::builder().uk_nhs_number("943 476 5919").build();
//! let c = Worker::builder().uk_nhs_number("9434765919").build();
//! let r = MatchingEngine::default_config().match_workers(&b, &c);
//! assert_eq!(r.evidence.strength, MatchStrength::Proven);
//! ```

use crate::matcher::MatchBreakdown;
use serde::{Deserialize, Serialize};

/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;

/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// Bits a value shared under a globally unique identifier scheme adds,
/// once per pair: `log2(0.99 / 1e-12)`.
pub const IDENTIFIER_BITS: f64 = 39.848_637_568_953_24;

/// Bits each contradicted identifier scheme adds:
/// `log2(0.01 / (1 − 1e-12))`.
pub const CONTRADICTION_BITS: f64 = -6.643_856_189_773_281;

/// Default [`crate::MatchConfig::proof_bits`]: a million records at one
/// false merge in a thousand needs `log2(1e6) + log2(1e3) ≈ 30` bits.
pub const DEFAULT_PROOF_BITS: f64 = 30.0;

/// The weight, in bits, one compared component adds.
///
/// A similarity at or above [`AGREE_AT`] gives the full agreement weight
/// `log2(m / u)`; at or below [`DISAGREE_AT`] the full (negative)
/// disagreement weight `log2((1 − m) / (1 − u))`; in between, a linear
/// interpolation. A non-finite similarity counts as `0.0`.
///
/// ```
/// use worker_matcher::evidence::component_bits;
/// assert!((component_bits(1.0, 0.9, 0.1) - 3.0_f64.log2() * 2.0).abs() < 1e-9);
/// assert!(component_bits(0.0, 0.9, 0.1) < 0.0);
/// ```
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

/// One hard conflict between the two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contradiction {
    /// What conflicts, e.g. `"identifier:uk_nhs_number"`.
    pub field: String,
}

/// The verdict the evidence supports.
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

/// How much the compared data proves, beside the similarity score.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Evidence {
    /// Sum of the components' log-likelihood weights, in bits.
    pub weight_bits: f64,
    /// Components both records held (identifiers count as one).
    pub fields_compared: u32,
    /// Hard conflicts; any one makes the strength `Contradicted`.
    pub contradictions: Vec<Contradiction>,
    /// The verdict.
    pub strength: MatchStrength,
}

/// The `m` / `u` pair for one breakdown component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MU {
    /// Chance the component agrees when the records are the same worker.
    pub m: f64,
    /// Chance the component agrees by coincidence for different workers.
    pub u: f64,
}

/// Given name (middle name blended in). `u`: about 1 in 50 unrelated
/// workers share a given name; common names dominate a workforce.
pub const GIVEN_NAME: MU = MU { m: 0.95, u: 0.02 };
/// Family name. `u`: about 1 in 100 unrelated workers share a surname.
pub const FAMILY_NAME: MU = MU { m: 0.95, u: 0.01 };
/// Date of birth. `u`: one calendar day across ~40 working-age birth
/// years, `1 / (365 × 40)`.
pub const DATE_OF_BIRTH: MU = MU {
    m: 0.97,
    u: 1.0 / 14_600.0,
};
/// Gender. `u`: roughly half of unrelated pairs share it.
pub const GENDER: MU = MU { m: 0.98, u: 0.5 };
/// ABO + `RhD` blood type. `u`: sum of squared type frequencies, about a
/// quarter (O+ and A+ dominate).
pub const BLOOD_TYPE: MU = MU { m: 0.98, u: 0.25 };
/// Multiple-birth order. `u`: recorded orders are mostly 1 or 2, so about
/// half of unrelated pairs with the field share it.
pub const MULTIPLE_BIRTH: MU = MU { m: 0.98, u: 0.5 };
/// Address (best of current and previous). `u`: unrelated workers rarely
/// share a postcode-plus-street, about 1 in 10 000; `m` is low because
/// people move.
pub const ADDRESS: MU = MU { m: 0.80, u: 1e-4 };
/// Place of birth (city + country). `u`: many people are born in the same
/// large city, about 1 in 20.
pub const BIRTH_PLACE: MU = MU { m: 0.90, u: 0.05 };
/// Date of death. `u`: one calendar day across ~100 years.
pub const DEATH_DATE: MU = MU {
    m: 0.97,
    u: 1.0 / 36_500.0,
};
/// Place of death (city + country). `u`: as for place of birth.
pub const DEATH_PLACE: MU = MU { m: 0.90, u: 0.05 };
/// Phone (E.164). `u`: unrelated workers share a number about 1 in 10 000
/// (shared landlines, switchboards); `m` is low because numbers change.
pub const PHONE: MU = MU { m: 0.85, u: 1e-4 };
/// Email (canonicalised). `u`: about 1 in 100 000 (shared role mailboxes);
/// `m` is low because addresses change with employers.
pub const EMAIL: MU = MU { m: 0.85, u: 1e-5 };
/// Relationship set (Jaccard). `u`: overlapping colleague / manager sets
/// are common inside one organisation, about 1 in 10. Corroborates only.
pub const RELATIONSHIPS: MU = MU { m: 0.60, u: 0.10 };
/// Tag set (Jaccard). `u`: operator tags are a small shared vocabulary,
/// about 1 in 5. Corroborates only.
pub const TAGS: MU = MU { m: 0.60, u: 0.20 };
/// Soundex of given + family name. `u`: Soundex codes collide often, and
/// the component is correlated with the name components above, so it is
/// kept deliberately weak (about 1 in 2).
pub const PHONETIC_NAME: MU = MU { m: 0.90, u: 0.50 };

/// The national-identifier breakdown entries, named by scheme. Every one
/// is globally unique within its scheme and drives the deterministic
/// short-circuit. Each is single-valued on [`crate::Worker`], and its
/// breakdown score is `Some(1.0)` when both sides parse to the same value,
/// `Some(0.0)` when both parse and differ, `None` otherwise — so the
/// score alone decides shared vs. contradicted.
fn identifier_scores(b: &MatchBreakdown) -> [(&'static str, Option<f64>); 42] {
    [
        ("uk_nhs_number", b.uk_nhs_number_score),
        ("fr_nir", b.fr_nir_score),
        ("es_tsi", b.es_tsi_score),
        ("ie_ihi", b.ie_ihi_score),
        ("uk_hc_number", b.uk_hc_number_score),
        ("us_ssn", b.us_ssn_score),
        ("au_ihi", b.au_ihi_score),
        ("de_kvnr", b.de_kvnr_score),
        ("it_cf", b.it_cf_score),
        ("nl_bsn", b.nl_bsn_score),
        ("se_personnummer", b.se_personnummer_score),
        ("uk_chi_number", b.uk_chi_number_score),
        ("be_nn", b.be_nn_score),
        ("bg_egn", b.bg_egn_score),
        ("cz_rc", b.cz_rc_score),
        ("dk_cpr", b.dk_cpr_score),
        ("ee_ik", b.ee_ik_score),
        ("es_dni", b.es_dni_score),
        ("fi_hetu", b.fi_hetu_score),
        ("hr_oib", b.hr_oib_score),
        ("is_kt", b.is_kt_score),
        ("lt_ak", b.lt_ak_score),
        ("lv_pk", b.lv_pk_score),
        ("mt_id", b.mt_id_score),
        ("no_fnr", b.no_fnr_score),
        ("pl_pesel", b.pl_pesel_score),
        ("ro_cnp", b.ro_cnp_score),
        ("si_emso", b.si_emso_score),
        ("sk_rc", b.sk_rc_score),
        ("uk_nino", b.uk_nino_score),
        ("gr_dss", b.gr_dss_score),
        ("li_id", b.li_id_score),
        ("nl_id", b.nl_id_score),
        ("pl_nip", b.pl_nip_score),
        ("pt_nif", b.pt_nif_score),
        ("br_cpf", b.br_cpf_score),
        ("cn_rrn", b.cn_rrn_score),
        ("in_aadhaar", b.in_aadhaar_score),
        ("jp_my_number", b.jp_my_number_score),
        ("mx_curp", b.mx_curp_score),
        ("nz_nhi", b.nz_nhi_score),
        ("za_id", b.za_id_score),
    ]
}

/// The non-identifier components, each with its `m` / `u`.
fn field_components(b: &MatchBreakdown) -> [(Option<f64>, MU); 15] {
    [
        (b.given_name_score, GIVEN_NAME),
        (b.family_name_score, FAMILY_NAME),
        (b.date_of_birth_score, DATE_OF_BIRTH),
        (b.gender_score, GENDER),
        (b.blood_type_score, BLOOD_TYPE),
        (b.multiple_birth_score, MULTIPLE_BIRTH),
        (b.address_score, ADDRESS),
        (b.birth_place_score, BIRTH_PLACE),
        (b.death_date_score, DEATH_DATE),
        (b.death_place_score, DEATH_PLACE),
        (b.phone_score, PHONE),
        (b.email_score, EMAIL),
        (b.relationships_score, RELATIONSHIPS),
        (b.tags_score, TAGS),
        (b.phonetic_name_score, PHONETIC_NAME),
    ]
}

/// Compute the [`Evidence`] for one pair from its breakdown.
///
/// - A value shared under any national-identifier scheme, or a shared
///   `(country, number)` passport pair, adds [`IDENTIFIER_BITS`] once.
/// - Each national-identifier scheme both sides parse under, with
///   different values, is a [`Contradiction`] and adds
///   [`CONTRADICTION_BITS`]. Passport books never contradict: a passport
///   number changes on every renewal, so two disjoint book lists are
///   ordinary, not a conflict.
/// - Every other component that is `Some` adds [`component_bits`].
/// - The name + DOB + gender deterministic tuple is **not** an
///   identifier and adds nothing beyond its components.
///
/// `is_match` is the result's own `is_match`; `proof_bits` the configured
/// bar.
#[must_use]
pub fn evaluate(breakdown: &MatchBreakdown, is_match: bool, proof_bits: f64) -> Evidence {
    let mut weight_bits = 0.0_f64;
    let mut fields_compared = 0_u32;
    let mut contradictions = Vec::new();

    let mut identifier_compared = false;
    let mut identifier_shared = false;
    for (scheme, score) in identifier_scores(breakdown) {
        if let Some(s) = score {
            identifier_compared = true;
            if s >= 1.0 {
                identifier_shared = true;
            } else {
                weight_bits += CONTRADICTION_BITS;
                contradictions.push(Contradiction {
                    field: format!("identifier:{scheme}"),
                });
            }
        }
    }
    if let Some(s) = breakdown.passport_book_score {
        identifier_compared = true;
        if s >= 1.0 {
            identifier_shared = true;
        }
    }
    if identifier_compared {
        fields_compared += 1;
    }
    if identifier_shared {
        weight_bits += IDENTIFIER_BITS;
    }

    for (score, mu) in field_components(breakdown) {
        if let Some(s) = score {
            weight_bits += component_bits(s, mu.m, mu.u);
            fields_compared += 1;
        }
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

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    #[test]
    fn constants_match_their_formulas() {
        approx(IDENTIFIER_BITS, (0.99_f64 / 1e-12).log2());
        approx(CONTRADICTION_BITS, (0.01_f64 / (1.0 - 1e-12)).log2());
    }

    #[test]
    fn component_bits_endpoints_and_interpolation() {
        let (m, u) = (0.9_f64, 0.1_f64);
        let agree = (m / u).log2();
        let disagree = ((1.0 - m) / (1.0 - u)).log2();
        approx(component_bits(1.0, m, u), agree);
        approx(component_bits(AGREE_AT, m, u), agree);
        approx(component_bits(0.0, m, u), disagree);
        approx(component_bits(DISAGREE_AT, m, u), disagree);
        let mid = f64::midpoint(AGREE_AT, DISAGREE_AT);
        approx(component_bits(mid, m, u), f64::midpoint(agree, disagree));
        approx(component_bits(f64::NAN, m, u), disagree);
        approx(component_bits(7.0, m, u), agree);
    }

    #[test]
    fn every_mu_is_a_probability_with_m_above_u() {
        let b = MatchBreakdown::default();
        for (_, mu) in field_components(&b) {
            assert!(mu.m > 0.0 && mu.m < 1.0);
            assert!(mu.u > 0.0 && mu.u < 1.0);
            assert!(mu.m > mu.u);
        }
    }

    #[test]
    fn empty_breakdown_is_zero_bits_and_nothing_compared() {
        let e = evaluate(&MatchBreakdown::default(), false, DEFAULT_PROOF_BITS);
        approx(e.weight_bits, 0.0);
        assert_eq!(e.fields_compared, 0);
        assert!(e.contradictions.is_empty());
        assert_eq!(e.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn missing_component_adds_zero_and_is_not_counted() {
        let with_gender = MatchBreakdown {
            given_name_score: Some(1.0),
            gender_score: Some(1.0),
            ..MatchBreakdown::default()
        };
        let without_gender = MatchBreakdown {
            gender_score: None,
            ..with_gender.clone()
        };
        let a = evaluate(&with_gender, true, DEFAULT_PROOF_BITS);
        let b = evaluate(&without_gender, true, DEFAULT_PROOF_BITS);
        assert_eq!(a.fields_compared, 2);
        assert_eq!(b.fields_compared, 1);
        approx(
            a.weight_bits - b.weight_bits,
            component_bits(1.0, GENDER.m, GENDER.u),
        );
    }

    #[test]
    fn identifiers_count_as_one_field_and_add_identifier_bits_once() {
        let b = MatchBreakdown {
            uk_nhs_number_score: Some(1.0),
            fr_nir_score: Some(1.0),
            passport_book_score: Some(1.0),
            ..MatchBreakdown::default()
        };
        let e = evaluate(&b, true, DEFAULT_PROOF_BITS);
        assert_eq!(e.fields_compared, 1);
        approx(e.weight_bits, IDENTIFIER_BITS);
        assert_eq!(e.strength, MatchStrength::Proven);
    }

    #[test]
    fn contradiction_beats_a_shared_identifier() {
        let b = MatchBreakdown {
            uk_nhs_number_score: Some(1.0),
            us_ssn_score: Some(0.0),
            ..MatchBreakdown::default()
        };
        let e = evaluate(&b, true, DEFAULT_PROOF_BITS);
        assert_eq!(e.strength, MatchStrength::Contradicted);
        assert_eq!(
            e.contradictions,
            vec![Contradiction {
                field: "identifier:us_ssn".into()
            }]
        );
        approx(e.weight_bits, IDENTIFIER_BITS + CONTRADICTION_BITS);
    }

    #[test]
    fn disjoint_passports_do_not_contradict() {
        let b = MatchBreakdown {
            passport_book_score: Some(0.0),
            ..MatchBreakdown::default()
        };
        let e = evaluate(&b, false, DEFAULT_PROOF_BITS);
        assert!(e.contradictions.is_empty());
        assert_eq!(e.fields_compared, 1);
        approx(e.weight_bits, 0.0);
    }

    #[test]
    fn strength_serialises_snake_case() {
        assert_eq!(
            serde_json::to_string(&MatchStrength::NonMatch).unwrap(),
            "\"non_match\""
        );
        let e: Evidence = serde_json::from_str(
            r#"{"weight_bits":1.5,"fields_compared":2,"contradictions":[],"strength":"proven"}"#,
        )
        .unwrap();
        assert_eq!(e.strength, MatchStrength::Proven);
    }
}
