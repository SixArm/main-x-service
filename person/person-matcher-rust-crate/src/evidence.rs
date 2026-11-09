//! Match **evidence**: how much the compared data *proves*, beside the
//! similarity score.
//!
//! [`crate::MatchResult::score`] answers "how alike is the data both
//! records hold?". It renormalises over the fields both records carry, so
//! two sparse records agreeing on name and birth date score `1.0` — and so
//! do twins. [`Evidence`] answers the other question, "how much does the
//! data prove?", as a Fellegi–Sunter log-likelihood ratio in bits, where a
//! missing component contributes exactly `0`.
//!
//! This is the family-wide contract in `agents/share/match-evidence.md`;
//! only the per-component `m`/`u` table ([`COMPONENTS`] and
//! [`PHONETIC`]) is specific to this crate. Spec §12.7.

use serde::{Deserialize, Serialize};

use crate::matcher::MatchBreakdown;

/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;
/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// Default evidence bar in bits ([`crate::MatchConfig::proof_bits`]).
///
/// A million records at one false merge in a thousand needs
/// `log2(1e6) + log2(1e3) ≈ 30` bits.
pub const DEFAULT_PROOF_BITS: f64 = 30.0;

/// Bits a shared value under a globally unique identifier scheme adds,
/// once per pair: `log2(0.99 / 1e-12) ≈ 39.85`.
pub const IDENTIFIER_BITS: f64 = 39.848_637_568_953_24;

/// Bits each contradicted identifier scheme adds:
/// `log2(0.01 / (1 − 1e-12)) ≈ −6.64`.
pub const CONTRADICTION_BITS: f64 = -6.643_856_189_773_282;

/// The weight, in bits, one compared component adds.
///
/// Agreement (`similarity >= AGREE_AT`) adds `log2(m / u)`; disagreement
/// (`similarity <= DISAGREE_AT`) adds `log2((1 − m) / (1 − u))`; between
/// the two the weight is interpolated linearly. A non-finite similarity
/// counts as `0.0` (full disagreement).
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

/// The verdict on a pair (`agents/share/match-evidence.md` §4).
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
    /// What conflicts, e.g. `"identifier:us_ssn"`.
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
    /// The verdict.
    pub strength: MatchStrength,
}

/// `m` / `u` for one non-identifier breakdown component.
#[derive(Debug, Clone, Copy)]
pub struct ComponentWeights {
    /// Breakdown field name, without the `_score` suffix.
    pub name: &'static str,
    /// P(agree | same person).
    pub m: f64,
    /// P(agree | different people).
    pub u: f64,
}

/// The per-component table: one entry per non-identifier
/// [`MatchBreakdown`] component except the phonetic bonus ([`PHONETIC`]).
///
/// Informed estimates, not measurements (`match-evidence.md` §5). The
/// identifier components are not listed: they contribute
/// [`IDENTIFIER_BITS`] once, together, through the identifier rule.
pub const COMPONENTS: &[ComponentWeights] = &[
    // u: about 1% of unrelated pairs share a given name (common names
    // dominate); m < 1 for typos, nicknames and transliteration.
    ComponentWeights {
        name: "given_name",
        m: 0.95,
        u: 0.01,
    },
    // u: family names are more spread out than given names (~0.5%);
    // m is lower because family names change on marriage.
    ComponentWeights {
        name: "family_name",
        m: 0.92,
        u: 0.005,
    },
    // u: 1 in ~10 000 unrelated pairs share a full birth date (365 days
    // over a few decades of birth years, rounded up for clustering).
    ComponentWeights {
        name: "date_of_birth",
        m: 0.97,
        u: 1e-4,
    },
    // u: two unrelated people share a recorded gender about half the time.
    ComponentWeights {
        name: "gender",
        m: 0.98,
        u: 0.5,
    },
    // u: sum of squared ABO+RhD frequencies is about 0.25 (O+ ~37%).
    ComponentWeights {
        name: "blood_type",
        m: 0.99,
        u: 0.25,
    },
    // u: only scored inside a multiple birth; birth order 1 vs 2 is
    // roughly a coin flip.
    ComponentWeights {
        name: "multiple_birth",
        m: 0.98,
        u: 0.5,
    },
    // u: a full postal address is shared by a handful of households in a
    // large population (~1e-4); m is low because people move.
    ComponentWeights {
        name: "address",
        m: 0.70,
        u: 1e-4,
    },
    // u: about 1% of unrelated people share a birth city and country.
    ComponentWeights {
        name: "birth_place",
        m: 0.90,
        u: 0.01,
    },
    // u: like a birth date, a full calendar date of death is rare to share.
    ComponentWeights {
        name: "death_date",
        m: 0.95,
        u: 1e-4,
    },
    // u: about 1% of unrelated people share a death city and country.
    ComponentWeights {
        name: "death_place",
        m: 0.90,
        u: 0.01,
    },
    // u: household landlines are shared (~1e-3); m is low because numbers
    // change.
    ComponentWeights {
        name: "phone",
        m: 0.60,
        u: 1e-3,
    },
    // u: family-shared mailboxes exist but are rare (~1e-4); m is low
    // because people change providers.
    ComponentWeights {
        name: "email",
        m: 0.70,
        u: 1e-4,
    },
    // u: set overlap corroborates rather than identifies — siblings share
    // a parent — so the weight is kept weak.
    ComponentWeights {
        name: "relationships",
        m: 0.60,
        u: 0.10,
    },
    // u: operator tags (cohorts, flags) are shared by many unrelated
    // records; weakest of all.
    ComponentWeights {
        name: "tags",
        m: 0.50,
        u: 0.20,
    },
];

/// The phonetic-name component. It is derived from the two name
/// components, so — exactly as in [`crate::MatchResult::score`] — it only
/// ever adds agreement (when `> 0.9`), never disagreement, and its weight
/// is kept small to limit the double count. u: Soundex codes collide for
/// a large share of unrelated name pairs.
pub const PHONETIC: ComponentWeights = ComponentWeights {
    name: "phonetic_name",
    m: 0.95,
    u: 0.60,
};

/// The non-identifier components of a breakdown, in [`COMPONENTS`] order.
fn component_scores(b: &MatchBreakdown) -> [Option<f64>; 14] {
    [
        b.given_name_score,
        b.family_name_score,
        b.date_of_birth_score,
        b.gender_score,
        b.blood_type_score,
        b.multiple_birth_score,
        b.address_score,
        b.birth_place_score,
        b.death_date_score,
        b.death_place_score,
        b.phone_score,
        b.email_score,
        b.relationships_score,
        b.tags_score,
    ]
}

/// The national-identifier schemes of a breakdown, `(scheme, score)`.
///
/// Each scheme holds at most one value per person, scored `Some(1.0)` /
/// `Some(0.0)` only when both sides parse (the crate's normalisation), so
/// `Some(0.0)` is exactly "both hold a value and the sets are disjoint".
fn national_identifier_scores(b: &MatchBreakdown) -> [(&'static str, Option<f64>); 42] {
    [
        (
            "united_kingdom_national_health_service_number",
            b.united_kingdom_national_health_service_number_score,
        ),
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

/// Compute the [`Evidence`] for a scored pair.
///
/// - A shared value under any national identifier scheme, or a shared
///   `(country, number)` passport pair — the same rules the deterministic
///   short-circuit uses — adds [`IDENTIFIER_BITS`] once. The exact
///   name + birth date + gender tuple, the other deterministic rule, adds
///   no identifier bits.
/// - Each national scheme under which both records parse a value and the
///   values differ is a [`Contradiction`] (`identifier:<scheme>`) and adds
///   [`CONTRADICTION_BITS`]. Passports never contradict: a passport number
///   names a document, and a renewed book legitimately carries a new
///   number, so disjoint passport lists are not a conflict.
/// - Every other component present in the breakdown adds
///   [`component_bits`]; a missing one adds `0` and is not counted.
#[must_use]
pub fn evidence_from_breakdown(b: &MatchBreakdown, is_match: bool, proof_bits: f64) -> Evidence {
    let mut weight_bits = 0.0;
    let mut fields_compared = 0u32;
    let mut contradictions = Vec::new();

    let national = national_identifier_scores(b);
    let shared =
        national.iter().any(|(_, s)| *s == Some(1.0)) || b.passport_book_score == Some(1.0);
    let identifiers_compared =
        national.iter().any(|(_, s)| s.is_some()) || b.passport_book_score.is_some();
    if identifiers_compared {
        fields_compared += 1;
    }
    if shared {
        weight_bits += IDENTIFIER_BITS;
    }
    for (scheme, score) in national {
        if score == Some(0.0) {
            contradictions.push(Contradiction {
                field: format!("identifier:{scheme}"),
            });
            weight_bits += CONTRADICTION_BITS;
        }
    }

    for (weights, score) in COMPONENTS.iter().zip(component_scores(b)) {
        if let Some(s) = score {
            weight_bits += component_bits(s, weights.m, weights.u);
            fields_compared += 1;
        }
    }
    if let Some(s) = b.phonetic_name_score
        && s > 0.9
    {
        weight_bits += component_bits(s, PHONETIC.m, PHONETIC.u);
        fields_compared += 1;
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
    fn identifier_constants_match_their_formulas() {
        assert!((IDENTIFIER_BITS - (0.99_f64 / 1e-12).log2()).abs() < 1e-9);
        assert!((CONTRADICTION_BITS - (0.01_f64 / (1.0 - 1e-12)).log2()).abs() < 1e-9);
    }

    #[test]
    fn component_bits_endpoints_and_interpolation() {
        let (m, u): (f64, f64) = (0.9, 0.1);
        let agree = (m / u).log2();
        let disagree = ((1.0 - m) / (1.0 - u)).log2();
        assert!((component_bits(1.0, m, u) - agree).abs() < 1e-12);
        assert!((component_bits(AGREE_AT, m, u) - agree).abs() < 1e-12);
        assert!((component_bits(0.0, m, u) - disagree).abs() < 1e-12);
        assert!((component_bits(DISAGREE_AT, m, u) - disagree).abs() < 1e-12);
        let mid = component_bits(f64::midpoint(AGREE_AT, DISAGREE_AT), m, u);
        assert!((mid - f64::midpoint(agree, disagree)).abs() < 1e-9);
        assert!((component_bits(f64::NAN, m, u) - disagree).abs() < 1e-12);
    }

    #[test]
    fn every_table_entry_has_sane_probabilities() {
        for w in COMPONENTS.iter().chain(std::iter::once(&PHONETIC)) {
            assert!(w.m > w.u, "{}: m must exceed u", w.name);
            assert!(w.m > 0.0 && w.m < 1.0, "{}", w.name);
            assert!(w.u > 0.0 && w.u < 1.0, "{}", w.name);
        }
    }

    #[test]
    fn strength_round_trips_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&MatchStrength::NonMatch).unwrap(),
            "\"non_match\""
        );
        let back: MatchStrength = serde_json::from_str("\"proven\"").unwrap();
        assert_eq!(back, MatchStrength::Proven);
    }
}
