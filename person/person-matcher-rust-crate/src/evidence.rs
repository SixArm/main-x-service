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
//! This is the family-wide contract in `agents/share/match-evidence.md`.
//! Specific to this crate: the per-component `m`/`u` table
//! ([`COMPONENTS`]), the correlated groups ([`CORRELATED_GROUPS`]) and the
//! frequency-adjusted components ([`FREQUENCY_COMPONENTS`], keyed by
//! [`frequency_key`]). Spec §12.7.

use std::collections::HashMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::matcher::MatchBreakdown;
use crate::models::Person;
use crate::normalizer::Normalizer;

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
    /// Every term of the sum, in order; the `bits` add up to
    /// `weight_bits` exactly. Empty on JSON that pre-dates the field.
    #[serde(default)]
    pub components: Vec<ComponentBits>,
}

/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentBits {
    /// The component, e.g. `"family_name"`, `"identifier"`,
    /// `"contradiction:identifier:us_ssn"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}

// ── Correlated components (match-evidence.md §6) ──

/// Weight of each member of a correlated group after its strongest.
pub const CORRELATED_DISCOUNT: f64 = 0.5;

/// This crate's correlated groups, by [`COMPONENTS`] name. Disjoint: a
/// component is in at most one group.
///
/// - `address` + `phone`: a household's address and landline are recorded
///   together and move together, so two people in one house agree on
///   both for one reason.
/// - `death_date` + `death_place`: both are determined by one fact, the
///   death, and are usually recorded from the same certificate.
///
/// `birth_place` is deliberately **not** grouped with `address`: one is
/// fixed at birth and the other is the current residence, so for
/// different people they agree for different reasons, and putting
/// `address` in two groups would discount it twice.
pub const CORRELATED_GROUPS: &[&[&str]] = &[&["address", "phone"], &["death_date", "death_place"]];

/// The index of the strongest of `member_bits`: largest absolute value,
/// ties broken by the larger value, then by the earlier index (whose
/// value is then identical, so the result does not depend on order).
fn strongest_index(member_bits: &[f64]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, &b) in member_bits.iter().enumerate() {
        best = match best {
            None => Some(i),
            Some(j) => {
                let c = member_bits[j];
                let order = b.abs().total_cmp(&c.abs()).then(b.total_cmp(&c));
                if order == std::cmp::Ordering::Greater {
                    Some(i)
                } else {
                    Some(j)
                }
            }
        };
    }
    best
}

/// Split `member_bits` into its strongest member (largest absolute value,
/// ties to the larger value) and the rest, or `None` when empty.
#[must_use]
pub fn strongest_first(member_bits: &[f64]) -> Option<(f64, Vec<f64>)> {
    let i = strongest_index(member_bits)?;
    let rest = member_bits
        .iter()
        .enumerate()
        .filter(|(j, _)| *j != i)
        .map(|(_, b)| *b)
        .collect();
    Some((member_bits[i], rest))
}

/// The bits a correlated group adds, from its present members' bits.
#[must_use]
pub fn group_bits(member_bits: &[f64]) -> f64 {
    let Some((strongest, rest)) = strongest_first(member_bits) else {
        return 0.0;
    };
    strongest + CORRELATED_DISCOUNT * rest.iter().sum::<f64>()
}

// ── Common and rare values (match-evidence.md §7) ──

/// Where the share of records holding a value comes from.
pub trait ValueFrequencies {
    /// The share, in `0.0..=1.0`, of records whose `component` holds
    /// `value` (already in the matcher's normalised form, see
    /// [`frequency_key`]), or `None` when unknown.
    fn share(&self, component: &str, value: &str) -> Option<f64>;
}

/// A frequency source that knows nothing: every agreement uses the
/// table's `u`. What [`crate::MatchingEngine::match_persons`] uses.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFrequencies;

impl ValueFrequencies for NoFrequencies {
    fn share(&self, _component: &str, _value: &str) -> Option<f64> {
        None
    }
}

/// An in-memory frequency source keyed by `(component, value)`, where
/// `value` is a [`frequency_key`]. Values are stored as given.
#[derive(Debug, Clone, Default)]
pub struct FrequencyTable {
    /// `(component, key)` → share of records.
    shares: HashMap<(String, String), f64>,
}

impl FrequencyTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a table from `(component, key, count)` entries over a
    /// population of `total` records: each share is `count / total`. A
    /// `total` of `0` gives an empty table.
    #[must_use]
    pub fn from_counts<I, C, V>(counts: I, total: u64) -> Self
    where
        I: IntoIterator<Item = (C, V, u64)>,
        C: Into<String>,
        V: Into<String>,
    {
        let mut table = Self::new();
        if total == 0 {
            return table;
        }
        for (component, value, count) in counts {
            table.insert(component, value, count as f64 / total as f64);
        }
        table
    }

    /// Set the share of records whose `component` holds `value`.
    pub fn insert(&mut self, component: impl Into<String>, value: impl Into<String>, share: f64) {
        self.shares.insert((component.into(), value.into()), share);
    }
}

impl ValueFrequencies for FrequencyTable {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        self.shares
            .get(&(component.to_owned(), value.to_owned()))
            .copied()
    }
}

/// Smallest share used, so a value seen once cannot give unbounded bits.
pub const MIN_SHARE: f64 = 1e-9;

/// Agreement bits for a value held by `share` of records.
#[must_use]
pub fn frequency_bits(m: f64, share: f64) -> f64 {
    let share = if share.is_finite() {
        share.clamp(MIN_SHARE, m)
    } else {
        m
    };
    (m / share).log2()
}

/// The components whose exact agreement is weighted by the value's own
/// frequency. Public contract: a service counts values under these names.
pub const FREQUENCY_COMPONENTS: &[&str] = &["given_name", "family_name", "date_of_birth"];

/// The key the matcher looks up for `raw` under `component`, or `None`
/// when `component` is not in [`FREQUENCY_COMPONENTS`] or `raw` has no
/// usable form.
///
/// - `given_name`, `family_name`: [`Normalizer::normalize_name`] (the form
///   name scoring compares); `None` when it normalises to empty.
/// - `date_of_birth`: ISO 8601 `YYYY-MM-DD`; `raw` must parse as one.
///
/// ```
/// use person_matcher::evidence::frequency_key;
/// assert_eq!(frequency_key("family_name", "  SMITH ").as_deref(), Some("smith"));
/// assert_eq!(frequency_key("date_of_birth", "1900-01-01").as_deref(), Some("1900-01-01"));
/// assert_eq!(frequency_key("phone", "123"), None);
/// ```
#[must_use]
pub fn frequency_key(component: &str, raw: &str) -> Option<String> {
    match component {
        "given_name" | "family_name" => {
            let n = Normalizer::normalize_name(raw);
            (!n.is_empty()).then_some(n)
        }
        "date_of_birth" => NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
            .ok()
            .map(|d| d.format("%Y-%m-%d").to_string()),
        _ => None,
    }
}

/// The key both persons share under a frequency component, or `None` when
/// either lacks it or the keys differ (no exact agreement).
fn shared_key(component: &str, p1: &Person, p2: &Person) -> Option<String> {
    let raw = |p: &Person| -> Option<String> {
        match component {
            "given_name" => p.given_name.clone(),
            "family_name" => p.family_name.clone(),
            "date_of_birth" => p.date_of_birth.map(|d| d.format("%Y-%m-%d").to_string()),
            _ => None,
        }
    };
    let k1 = frequency_key(component, &raw(p1)?)?;
    let k2 = frequency_key(component, &raw(p2)?)?;
    (k1 == k2).then_some(k1)
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
/// [`MatchBreakdown`] component except `phonetic_name`, which is derived
/// from the two name components and so is left out of the sum entirely
/// (match-evidence.md §6).
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

/// Compute the [`Evidence`] for a scored pair with no value
/// frequencies: every agreement uses the table's `u`.
///
/// See [`evidence_for`] for the rules.
#[must_use]
pub fn evidence_from_breakdown(b: &MatchBreakdown, is_match: bool, proof_bits: f64) -> Evidence {
    evidence_with_shares(b, &|_| None, is_match, proof_bits)
}

/// Compute the [`Evidence`] for `p1` / `p2`, whose breakdown is `b`.
///
/// - A shared value under any national identifier scheme, or a shared
///   `(country, number)` passport pair — the same rules the deterministic
///   short-circuit uses — adds [`IDENTIFIER_BITS`] once (term
///   `"identifier"`). The exact name + birth date + gender tuple, the
///   other deterministic rule, adds no identifier bits.
/// - Each national scheme under which both records parse a value and the
///   values differ is a [`Contradiction`] (`identifier:<scheme>`) and adds
///   [`CONTRADICTION_BITS`] (term `"contradiction:identifier:<scheme>"`).
///   Passports never contradict: a passport number names a document, and
///   a renewed book legitimately carries a new number.
/// - Every other component present in the breakdown adds
///   [`component_bits`]; a missing one adds `0` and is not counted. A
///   [`FREQUENCY_COMPONENTS`] member on which both records hold the same
///   [`frequency_key`] and that `freqs` knows uses
///   [`frequency_bits`] in place of the table's agreement weight.
/// - Within each [`CORRELATED_GROUPS`] group, the strongest present member
///   counts in full and the others at [`CORRELATED_DISCOUNT`].
#[must_use]
pub fn evidence_for(
    b: &MatchBreakdown,
    p1: &Person,
    p2: &Person,
    freqs: &dyn ValueFrequencies,
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let shares = |component: &str| {
        let key = shared_key(component, p1, p2)?;
        freqs.share(component, &key)
    };
    evidence_with_shares(b, &shares, is_match, proof_bits)
}

/// The rules of [`evidence_for`], with `shares(component)` giving the
/// frequency of the value both records agree on exactly, when known.
fn evidence_with_shares(
    b: &MatchBreakdown,
    shares: &dyn Fn(&str) -> Option<f64>,
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let mut fields_compared = 0u32;
    let mut contradictions = Vec::new();
    let mut components = Vec::new();

    let national = national_identifier_scores(b);
    let shared =
        national.iter().any(|(_, s)| *s == Some(1.0)) || b.passport_book_score == Some(1.0);
    let identifiers_compared =
        national.iter().any(|(_, s)| s.is_some()) || b.passport_book_score.is_some();
    if identifiers_compared {
        fields_compared += 1;
    }
    if shared {
        components.push(ComponentBits {
            component: "identifier".into(),
            bits: IDENTIFIER_BITS,
            correlated: false,
            frequency_adjusted: false,
        });
    }
    for (scheme, score) in national {
        if score == Some(0.0) {
            let field = format!("identifier:{scheme}");
            components.push(ComponentBits {
                component: format!("contradiction:{field}"),
                bits: CONTRADICTION_BITS,
                correlated: false,
                frequency_adjusted: false,
            });
            contradictions.push(Contradiction { field });
        }
    }

    // Raw per-component terms, before the correlated-group discount.
    let first_component = components.len();
    for (weights, score) in COMPONENTS.iter().zip(component_scores(b)) {
        let Some(s) = score else { continue };
        fields_compared += 1;
        let share =
            if s.is_finite() && s >= AGREE_AT && FREQUENCY_COMPONENTS.contains(&weights.name) {
                shares(weights.name)
            } else {
                None
            };
        let (bits, frequency_adjusted) = match share {
            Some(share) => (frequency_bits(weights.m, share), true),
            None => (component_bits(s, weights.m, weights.u), false),
        };
        components.push(ComponentBits {
            component: weights.name.into(),
            bits,
            correlated: false,
            frequency_adjusted,
        });
    }

    for group in CORRELATED_GROUPS {
        let members: Vec<usize> = (first_component..components.len())
            .filter(|&i| group.contains(&components[i].component.as_str()))
            .collect();
        let bits: Vec<f64> = members.iter().map(|&i| components[i].bits).collect();
        let Some(strongest) = strongest_index(&bits) else {
            continue;
        };
        for (k, &i) in members.iter().enumerate() {
            if k != strongest {
                components[i].bits *= CORRELATED_DISCOUNT;
                components[i].correlated = true;
            }
        }
    }

    let weight_bits: f64 = components.iter().map(|c| c.bits).sum();

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
        components,
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
        for w in COMPONENTS {
            assert!(w.m > w.u, "{}: m must exceed u", w.name);
            assert!(w.m > 0.0 && w.m < 1.0, "{}", w.name);
            assert!(w.u > 0.0 && w.u < 1.0, "{}", w.name);
        }
    }

    #[test]
    fn groups_and_frequency_components_name_table_entries() {
        let names: Vec<&str> = COMPONENTS.iter().map(|w| w.name).collect();
        let mut seen = Vec::new();
        for group in CORRELATED_GROUPS {
            assert!(group.len() >= 2);
            for member in *group {
                assert!(names.contains(member), "{member} not in COMPONENTS");
                assert!(!seen.contains(member), "{member} in two groups");
                seen.push(*member);
            }
        }
        for c in FREQUENCY_COMPONENTS {
            assert!(names.contains(c), "{c} not in COMPONENTS");
        }
    }

    #[test]
    fn correlated_agreements_give_less_than_their_sum_in_any_order() {
        let (a, p) = (12.0, 9.0);
        let g = group_bits(&[a, p]);
        assert!(g < a + p);
        assert!((g - (a + CORRELATED_DISCOUNT * p)).abs() < 1e-12);
        assert!((group_bits(&[p, a]) - g).abs() < 1e-12);
        assert!((group_bits(&[]) - 0.0).abs() < 1e-12);
        assert!((group_bits(&[a]) - a).abs() < 1e-12);
    }

    #[test]
    fn correlated_disagreements_are_discounted_too() {
        let g = group_bits(&[-4.0, -1.5]);
        assert!(g > -5.5, "discounted against");
        assert!((g - (-4.0 - CORRELATED_DISCOUNT * 1.5)).abs() < 1e-12);
        assert!((group_bits(&[-1.5, -4.0]) - g).abs() < 1e-12);
    }

    #[test]
    fn strongest_first_orders_by_absolute_value_ties_to_larger() {
        assert_eq!(strongest_first(&[3.0, -5.0]), Some((-5.0, vec![3.0])));
        assert_eq!(strongest_first(&[-5.0, 5.0]), Some((5.0, vec![-5.0])));
        assert_eq!(strongest_first(&[5.0, -5.0]), Some((5.0, vec![-5.0])));
        assert_eq!(strongest_first(&[]), None);
    }

    #[test]
    fn frequency_bits_common_rare_and_capped() {
        let (m, u): (f64, f64) = (0.92, 0.005); // family_name
        let default = (m / u).log2();
        assert!(frequency_bits(m, 0.01) < default, "common name");
        assert!(frequency_bits(m, 1e-6) > default, "rare name");
        // A share at or above m is no evidence, never evidence against.
        assert!(frequency_bits(m, m).abs() < 1e-12);
        assert!(frequency_bits(m, 0.99).abs() < 1e-12);
        assert!(frequency_bits(m, f64::NAN).abs() < 1e-12);
        // MIN_SHARE bounds a value seen once.
        assert!((frequency_bits(m, 0.0) - (m / MIN_SHARE).log2()).abs() < 1e-9);
    }

    #[test]
    fn frequency_table_from_counts_and_insert() {
        let mut t = FrequencyTable::from_counts([("family_name", "smith", 10_u64)], 1000);
        assert_eq!(t.share("family_name", "smith"), Some(0.01));
        assert_eq!(t.share("family_name", "jones"), None);
        t.insert("date_of_birth", "1900-01-01", 0.2);
        assert_eq!(t.share("date_of_birth", "1900-01-01"), Some(0.2));
        assert_eq!(
            FrequencyTable::from_counts([("x", "y", 1_u64)], 0).share("x", "y"),
            None
        );
        assert_eq!(NoFrequencies.share("family_name", "smith"), None);
    }

    #[test]
    fn frequency_key_normalises_like_the_matcher() {
        assert_eq!(
            frequency_key("given_name", "Zoë").as_deref(),
            Some(Normalizer::normalize_name("Zoë").as_str())
        );
        assert_eq!(frequency_key("family_name", " ... "), None);
        assert_eq!(frequency_key("date_of_birth", "01/01/1900"), None);
        assert_eq!(frequency_key("tags", "vip"), None);
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
