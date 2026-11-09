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
    /// Every term of the sum, in order; the terms add up to
    /// `weight_bits` exactly. Empty on JSON written before the field.
    #[serde(default)]
    pub components: Vec<ComponentBits>,
}

/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentBits {
    /// The component, e.g. `"family_name"`, `"identifier"`,
    /// `"contradiction:identifier:uk_nhs_number"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
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
/// Family name. `u`: about 1 in 200 unrelated workers share a surname —
/// the sum of squared surname shares, which the most common surnames (at
/// about 1% each) dominate.
pub const FAMILY_NAME: MU = MU { m: 0.95, u: 0.005 };
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
/// Soundex of given + family name. Recorded for reference only: the
/// phonetic component is **derived** from the two name components, so it
/// is left out of the evidence sum entirely rather than discounted
/// (`agents/share/match-evidence.md` §6). It still lifts `score`.
pub const PHONETIC_NAME: MU = MU { m: 0.90, u: 0.50 };

/// The per-component table by name: every non-identifier breakdown
/// component the evidence sums, named as its breakdown field without the
/// `_score` suffix. These names are what [`CORRELATED_GROUPS`],
/// [`FREQUENCY_COMPONENTS`] and [`ComponentBits::component`] use.
pub const COMPONENTS: [(&str, MU); 14] = [
    ("given_name", GIVEN_NAME),
    ("family_name", FAMILY_NAME),
    ("date_of_birth", DATE_OF_BIRTH),
    ("gender", GENDER),
    ("blood_type", BLOOD_TYPE),
    ("multiple_birth", MULTIPLE_BIRTH),
    ("address", ADDRESS),
    ("birth_place", BIRTH_PLACE),
    ("death_date", DEATH_DATE),
    ("death_place", DEATH_PLACE),
    ("phone", PHONE),
    ("email", EMAIL),
    ("relationships", RELATIONSHIPS),
    ("tags", TAGS),
];

/// The `m` / `u` of a component in [`COMPONENTS`], by name.
#[must_use]
pub fn component_weights(component: &str) -> Option<MU> {
    COMPONENTS
        .iter()
        .find(|(name, _)| *name == component)
        .map(|(_, mu)| *mu)
}

// ---------- correlated components (match-evidence.md §6) ----------

/// Weight of each member of a correlated group after its strongest.
pub const CORRELATED_DISCOUNT: f64 = 0.5;

/// Groups of components that are not independent given the match status.
/// Within a group the strongest present member (by absolute bits) counts
/// in full and every other present member at [`CORRELATED_DISCOUNT`].
///
/// - `address` + `phone`: a household's landline is recorded with, and
///   moves with, its address; two people in one house agree on both for
///   one reason.
/// - `death_date` + `death_place`: both are recorded from one death
///   registration, so they agree or disagree together.
///
/// `birth_place` + `address` is deliberately **not** a group: where a
/// worker was born and where they live now are recorded separately and
/// at different times, and the address comparison is postcode-and-street
/// led, which a birth city does not predict. The groups are disjoint, so
/// each component is in at most one.
pub const CORRELATED_GROUPS: &[&[&str]] = &[&["address", "phone"], &["death_date", "death_place"]];

/// `true` when `a` is stronger than `b`: larger absolute value, ties to
/// the larger value.
fn stronger(a: f64, b: f64) -> bool {
    match a.abs().total_cmp(&b.abs()) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => a > b,
    }
}

/// Index of the strongest of `member_bits`, or `None` when empty.
fn strongest_index(member_bits: &[f64]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, &b) in member_bits.iter().enumerate() {
        match best {
            Some(j) if !stronger(b, member_bits[j]) => {}
            _ => best = Some(i),
        }
    }
    best
}

/// Split `member_bits` into its strongest member (largest absolute value,
/// ties to the larger value) and the rest, in their original order.
/// `None` when there are no members.
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

// ---------- common and rare values (match-evidence.md §7) ----------

/// Where the share of records holding a value comes from.
pub trait ValueFrequencies {
    /// The share, in `0.0..=1.0`, of records whose `component` holds
    /// `value` (already in the matcher's normalised form, see
    /// [`frequency_key`]), or `None` when unknown.
    fn share(&self, component: &str, value: &str) -> Option<f64>;
}

/// No frequencies: every share is unknown, so the plain `m`/`u` table
/// applies. [`crate::MatchingEngine::match_workers`] uses this.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoFrequencies;

impl ValueFrequencies for NoFrequencies {
    fn share(&self, _component: &str, _value: &str) -> Option<f64> {
        None
    }
}

/// A frequency source held in memory, keyed by `(component, value)`.
///
/// ```
/// use worker_matcher::evidence::{frequency_key, FrequencyTable, ValueFrequencies};
///
/// let key = frequency_key("family_name", "Smith").unwrap();
/// let table = FrequencyTable::from_counts([("family_name", key.as_str(), 120)], 10_000);
/// assert_eq!(table.share("family_name", "smith"), Some(0.012));
/// assert_eq!(table.share("family_name", "jones"), None);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrequencyTable {
    shares: std::collections::HashMap<(String, String), f64>,
}

impl FrequencyTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a table from `(component, value, count)` triples over a
    /// population of `total` records: each share is `count / total`.
    /// A `total` of `0` yields an empty table.
    #[must_use]
    pub fn from_counts<'a, I>(counts: I, total: u64) -> Self
    where
        I: IntoIterator<Item = (&'a str, &'a str, u64)>,
    {
        let mut table = Self::new();
        if total == 0 {
            return table;
        }
        #[allow(clippy::cast_precision_loss)]
        let total = total as f64;
        for (component, value, count) in counts {
            #[allow(clippy::cast_precision_loss)]
            let count = count as f64;
            table.insert(component, value, count / total);
        }
        table
    }

    /// Set the share of records whose `component` holds `value`. `value`
    /// should be the [`frequency_key`] form.
    pub fn insert(&mut self, component: &str, value: &str, share: f64) {
        self.shares
            .insert((component.to_string(), value.to_string()), share);
    }
}

impl ValueFrequencies for FrequencyTable {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        self.shares
            .get(&(component.to_string(), value.to_string()))
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

/// The components whose agreement weight adjusts to the agreed value's
/// frequency. Public contract, with [`frequency_key`]'s value forms.
pub const FREQUENCY_COMPONENTS: [&str; 3] = ["given_name", "family_name", "date_of_birth"];

/// The key the matcher looks up for `raw` under `component`, in the same
/// normalised form it compares, so a service can count values the same
/// way. `None` for a component not in [`FREQUENCY_COMPONENTS`] or a value
/// with no usable form.
///
/// - `given_name`, `family_name`: [`crate::Normalizer::normalize_name`]
///   (lowercase, diacritics and punctuation stripped, whitespace
///   collapsed); `None` when that is empty.
/// - `date_of_birth`: ISO 8601 `YYYY-MM-DD`; `raw` must parse as one.
///
/// ```
/// use worker_matcher::evidence::frequency_key;
/// assert_eq!(frequency_key("family_name", "  O'Brien ").as_deref(), Some("obrien"));
/// assert_eq!(frequency_key("date_of_birth", "1900-01-01").as_deref(), Some("1900-01-01"));
/// assert_eq!(frequency_key("date_of_birth", "01/01/1900"), None);
/// assert_eq!(frequency_key("email", "a@example.org"), None);
/// ```
#[must_use]
pub fn frequency_key(component: &str, raw: &str) -> Option<String> {
    match component {
        "given_name" | "family_name" => {
            let n = crate::Normalizer::normalize_name(raw);
            (!n.is_empty()).then_some(n)
        }
        "date_of_birth" => chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
            .ok()
            .map(|d| d.format("%Y-%m-%d").to_string()),
        _ => None,
    }
}

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

/// The non-identifier component scores, in [`COMPONENTS`] order.
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

/// Compute the [`Evidence`] for one pair from its breakdown, with no
/// frequency adjustment. See [`evaluate_adjusted`].
#[must_use]
pub fn evaluate(breakdown: &MatchBreakdown, is_match: bool, proof_bits: f64) -> Evidence {
    evaluate_adjusted(breakdown, is_match, proof_bits, &[])
}

/// Compute the [`Evidence`] for one pair from its breakdown.
///
/// - A value shared under any national-identifier scheme, or a shared
///   `(country, number)` passport pair, adds [`IDENTIFIER_BITS`] once, as
///   the term `"identifier"`.
/// - Each national-identifier scheme both sides parse under, with
///   different values, is a [`Contradiction`] and adds
///   [`CONTRADICTION_BITS`], as the term
///   `"contradiction:identifier:<scheme>"`. Passport books never
///   contradict: a passport number changes on every renewal.
/// - Every other component in [`COMPONENTS`] that is `Some` adds
///   [`component_bits`], or the bits given for it in `agreement_overrides`
///   (a frequency-adjusted exact agreement, computed by the caller), then
///   [`CORRELATED_GROUPS`] discount their non-strongest members.
/// - The name + DOB + gender deterministic tuple is **not** an
///   identifier, and the phonetic component is derived from the names, so
///   neither adds anything of its own.
///
/// `weight_bits` is the sum of the listed terms.
#[must_use]
pub fn evaluate_adjusted(
    breakdown: &MatchBreakdown,
    is_match: bool,
    proof_bits: f64,
    agreement_overrides: &[(&str, f64)],
) -> Evidence {
    let mut fields_compared = 0_u32;
    let mut contradictions = Vec::new();
    let mut terms: Vec<ComponentBits> = Vec::new();
    let term = |component: String, bits: f64, frequency_adjusted: bool| ComponentBits {
        component,
        bits,
        correlated: false,
        frequency_adjusted,
    };

    let mut identifier_compared = false;
    let mut identifier_shared = false;
    let mut contradiction_terms = Vec::new();
    for (scheme, score) in identifier_scores(breakdown) {
        if let Some(s) = score {
            identifier_compared = true;
            if s >= 1.0 {
                identifier_shared = true;
            } else {
                let field = format!("identifier:{scheme}");
                contradiction_terms.push(term(
                    format!("contradiction:{field}"),
                    CONTRADICTION_BITS,
                    false,
                ));
                contradictions.push(Contradiction { field });
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
        terms.push(term("identifier".to_string(), IDENTIFIER_BITS, false));
    }
    terms.extend(contradiction_terms);

    let first_component = terms.len();
    for ((name, mu), score) in COMPONENTS.iter().zip(component_scores(breakdown)) {
        if let Some(s) = score {
            fields_compared += 1;
            let adjusted = agreement_overrides
                .iter()
                .find(|(c, _)| c == name)
                .map(|(_, bits)| *bits);
            let bits = adjusted.unwrap_or_else(|| component_bits(s, mu.m, mu.u));
            terms.push(term((*name).to_string(), bits, adjusted.is_some()));
        }
    }

    for group in CORRELATED_GROUPS {
        let members: Vec<usize> = (first_component..terms.len())
            .filter(|&i| group.contains(&terms[i].component.as_str()))
            .collect();
        let bits: Vec<f64> = members.iter().map(|&i| terms[i].bits).collect();
        if let Some(strongest) = strongest_index(&bits) {
            for (k, &i) in members.iter().enumerate() {
                if k != strongest {
                    terms[i].bits *= CORRELATED_DISCOUNT;
                    terms[i].correlated = true;
                }
            }
        }
    }

    let weight_bits: f64 = terms.iter().map(|t| t.bits).sum();

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
        components: terms,
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
        for (_, mu) in COMPONENTS {
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

    // ---------- correlated groups (§6) ----------

    fn bits_of(e: &Evidence, component: &str) -> f64 {
        e.components
            .iter()
            .find(|t| t.component == component)
            .map_or(0.0, |t| t.bits)
    }

    fn assert_terms_sum(e: &Evidence) {
        let sum: f64 = e.components.iter().map(|t| t.bits).sum();
        approx(sum, e.weight_bits);
    }

    #[test]
    fn strongest_first_orders_by_absolute_value_ties_to_larger() {
        assert_eq!(strongest_first(&[]), None);
        assert_eq!(
            strongest_first(&[1.0, -3.0, 2.0]),
            Some((-3.0, vec![1.0, 2.0]))
        );
        assert_eq!(strongest_first(&[-2.0, 2.0]), Some((2.0, vec![-2.0])));
        assert_eq!(strongest_first(&[2.0, -2.0]), Some((2.0, vec![-2.0])));
    }

    #[test]
    fn group_bits_is_order_independent() {
        approx(group_bits(&[4.0, 10.0]), 12.0);
        approx(group_bits(&[10.0, 4.0]), 12.0);
        approx(group_bits(&[-2.0, 2.0]), group_bits(&[2.0, -2.0]));
        approx(group_bits(&[]), 0.0);
    }

    #[test]
    fn correlated_agreements_give_less_than_their_sum() {
        let b = MatchBreakdown {
            address_score: Some(1.0),
            phone_score: Some(1.0),
            ..MatchBreakdown::default()
        };
        let e = evaluate(&b, false, DEFAULT_PROOF_BITS);
        let address = component_bits(1.0, ADDRESS.m, ADDRESS.u);
        let phone = component_bits(1.0, PHONE.m, PHONE.u);
        assert!(e.weight_bits < address + phone);
        approx(e.weight_bits, group_bits(&[address, phone]));
        approx(e.weight_bits, group_bits(&[phone, address]));
        // Phone (13.05) is stronger than address (12.97): address is the
        // discounted member.
        let addr_term = e
            .components
            .iter()
            .find(|t| t.component == "address")
            .unwrap();
        assert!(addr_term.correlated);
        assert!(
            !e.components
                .iter()
                .find(|t| t.component == "phone")
                .unwrap()
                .correlated
        );
        assert_terms_sum(&e);
    }

    #[test]
    fn uncorrelated_fields_still_sum() {
        let b = MatchBreakdown {
            address_score: Some(1.0),
            email_score: Some(1.0),
            ..MatchBreakdown::default()
        };
        let e = evaluate(&b, false, DEFAULT_PROOF_BITS);
        approx(
            e.weight_bits,
            component_bits(1.0, ADDRESS.m, ADDRESS.u) + component_bits(1.0, EMAIL.m, EMAIL.u),
        );
        assert!(e.components.iter().all(|t| !t.correlated));
    }

    #[test]
    fn correlated_disagreements_are_discounted_too() {
        let b = MatchBreakdown {
            death_date_score: Some(0.0),
            death_place_score: Some(0.0),
            ..MatchBreakdown::default()
        };
        let e = evaluate(&b, false, DEFAULT_PROOF_BITS);
        let date = component_bits(0.0, DEATH_DATE.m, DEATH_DATE.u);
        let place = component_bits(0.0, DEATH_PLACE.m, DEATH_PLACE.u);
        assert!(e.weight_bits < 0.0);
        assert!(e.weight_bits > date + place, "less negative than the sum");
        approx(e.weight_bits, date + CORRELATED_DISCOUNT * place);
        assert_terms_sum(&e);
    }

    #[test]
    fn identifier_and_contradiction_are_terms() {
        let b = MatchBreakdown {
            uk_nhs_number_score: Some(1.0),
            us_ssn_score: Some(0.0),
            gender_score: Some(1.0),
            ..MatchBreakdown::default()
        };
        let e = evaluate(&b, true, DEFAULT_PROOF_BITS);
        approx(bits_of(&e, "identifier"), IDENTIFIER_BITS);
        approx(
            bits_of(&e, "contradiction:identifier:us_ssn"),
            CONTRADICTION_BITS,
        );
        assert_eq!(e.components.len(), 3);
        assert_terms_sum(&e);
    }

    #[test]
    fn phonetic_component_is_not_summed() {
        let b = MatchBreakdown {
            phonetic_name_score: Some(1.0),
            ..MatchBreakdown::default()
        };
        let e = evaluate(&b, true, DEFAULT_PROOF_BITS);
        approx(e.weight_bits, 0.0);
        assert_eq!(e.fields_compared, 0);
    }

    #[test]
    fn correlated_groups_name_real_disjoint_components() {
        let mut seen = std::collections::HashSet::new();
        for group in CORRELATED_GROUPS {
            assert!(group.len() >= 2);
            for c in *group {
                assert!(component_weights(c).is_some(), "{c}");
                assert!(seen.insert(*c), "{c} is in two groups");
            }
        }
        for c in FREQUENCY_COMPONENTS {
            assert!(component_weights(c).is_some(), "{c}");
        }
    }

    // ---------- value frequencies (§7) ----------

    #[test]
    fn common_value_gives_fewer_bits_than_default_and_rare_more() {
        let default = component_bits(1.0, FAMILY_NAME.m, FAMILY_NAME.u);
        let common = frequency_bits(FAMILY_NAME.m, 0.01);
        let rare = frequency_bits(FAMILY_NAME.m, 1e-6);
        assert!(common < default, "{common} !< {default}");
        assert!(rare > default, "{rare} !> {default}");
    }

    #[test]
    fn share_at_or_above_m_gives_zero_never_negative() {
        approx(frequency_bits(0.95, 0.95), 0.0);
        approx(frequency_bits(0.95, 1.0), 0.0);
        approx(frequency_bits(0.95, f64::NAN), 0.0);
        approx(frequency_bits(0.95, 0.0), (0.95 / MIN_SHARE).log2());
    }

    #[test]
    fn frequency_table_from_counts_and_insert() {
        let mut t = FrequencyTable::from_counts([("family_name", "smith", 50)], 1000);
        assert_eq!(t.share("family_name", "smith"), Some(0.05));
        assert_eq!(t.share("given_name", "smith"), None);
        t.insert("date_of_birth", "1900-01-01", 0.2);
        assert_eq!(t.share("date_of_birth", "1900-01-01"), Some(0.2));
        assert_eq!(
            FrequencyTable::from_counts([("a", "b", 1)], 0),
            FrequencyTable::new()
        );
        assert_eq!(NoFrequencies.share("family_name", "smith"), None);
    }

    #[test]
    fn override_marks_the_term_frequency_adjusted() {
        let b = MatchBreakdown {
            family_name_score: Some(1.0),
            ..MatchBreakdown::default()
        };
        let e = evaluate_adjusted(&b, true, DEFAULT_PROOF_BITS, &[("family_name", 1.25)]);
        assert_eq!(e.components.len(), 1);
        assert!(e.components[0].frequency_adjusted);
        approx(e.weight_bits, 1.25);
    }

    #[test]
    fn evidence_without_components_deserialises() {
        let e: Evidence = serde_json::from_str(
            r#"{"weight_bits":1.5,"fields_compared":2,"contradictions":[],"strength":"proven"}"#,
        )
        .unwrap();
        assert!(e.components.is_empty());
    }
}
