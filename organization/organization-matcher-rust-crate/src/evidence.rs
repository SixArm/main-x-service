//! Match evidence: Fellegi–Sunter log-likelihood weights beside the
//! similarity score.
//!
//! The similarity `score` answers "how alike is the data both records
//! hold?". It cannot tell "nothing here contradicts a match" from "the
//! data proves a match": two records holding only the same name score
//! `1.0`, and so do two unrelated organizations that happen to share a
//! name. [`Evidence`] answers the second question, in bits, so a service
//! can gate an automatic merge on proof rather than on similarity.
//!
//! The contract is the family-wide design in
//! `agents/share/match-evidence.md`; only the per-component `m`/`u`
//! table below is specific to this crate.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::normalize;
use crate::organization::{IdentifierScheme, Organization};
use crate::scoring::MatchBreakdown;

/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;
/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// Default [`crate::MatchConfig::proof_bits`]: a million records at one
/// false merge in a thousand needs `log2(1e6) + log2(1e3) ≈ 30` bits.
pub const DEFAULT_PROOF_BITS: f64 = 30.0;

/// The weight, in bits, a shared value under a globally unique
/// identifier scheme adds once per pair: `log2(0.99 / 1e-12) ≈ 39.85`.
/// Clears the default bar alone, which is the meaning of the
/// deterministic short-circuit.
pub const IDENTIFIER_BITS: f64 = 39.848_637_568_953_24;

/// The weight, in bits, each contradicted identifier scheme adds:
/// `log2(0.01 / (1 − 1e-12)) ≈ −6.64`.
pub const CONTRADICTION_BITS: f64 = -6.643_856_189_773_282;

/// Per-component `(m, u)` probabilities. An informed starting estimate,
/// not a measurement (`match-evidence.md` §5, §6).
///
/// - `m` — chance the component agrees when the records are one entity.
/// - `u` — chance it agrees by coincidence when they are not.
pub mod table {
    /// Name, after legal-suffix stripping. `u`: unrelated organizations
    /// share a bare name ("acme", "national bank") far more often than
    /// two people share a full name — about one pair in a thousand.
    pub const NAME: (f64, f64) = (0.95, 1e-3);
    /// Postal address, field by field. `m` is lowered for relocations;
    /// `u`: two unrelated organizations at the same street address is
    /// rare, but shared office buildings exist — about 1 in 10,000.
    pub const ADDRESS: (f64, f64) = (0.85, 1e-4);
    /// Registered domain. `u`: a domain names one organization, except
    /// that parents and subsidiaries share one — about 1 in 2,000.
    pub const URL: (f64, f64) = (0.90, 5e-4);
    /// Jurisdiction (country). `u`: most registries are dominated by a
    /// few countries, so a chance agreement is common — about 1 in 5.
    pub const JURISDICTION: (f64, f64) = (0.98, 0.20);
    /// Founding year. `u`: two unrelated organizations founded in the
    /// same year out of roughly a century of plausible years — 1 in 100.
    pub const FOUNDING_DATE: (f64, f64) = (0.90, 0.01);
    /// Keywords (Jaccard). `u`: overlapping keyword sets mostly signal
    /// the same sector, not the same entity — about 1 in 20.
    pub const KEYWORDS: (f64, f64) = (0.70, 0.05);
    /// Relationships (typed Jaccard). `u`: sibling subsidiaries share a
    /// parent reference, so overlap corroborates weakly — about 1 in 20.
    pub const RELATIONSHIPS: (f64, f64) = (0.70, 0.05);
    /// Tags (Jaccard). `u`: operator tags are coarse labels ("vendor")
    /// that many unrelated records carry — about 1 in 10.
    pub const TAGS: (f64, f64) = (0.60, 0.10);
}

/// The weight, in bits, one compared component adds.
///
/// Full agreement (`similarity >= AGREE_AT`) adds `log2(m / u)`; full
/// disagreement (`similarity <= DISAGREE_AT`) adds
/// `log2((1 − m) / (1 − u))`, which is negative; in between is linear.
/// A non-finite similarity counts as full disagreement.
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

/// The verdict on how much the compared data proves (`match-evidence.md` §4).
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
    /// What conflicts, e.g. `"identifier:lei"`.
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
    /// Every term of the sum, in order; they add up to `weight_bits`.
    /// Defaults to empty for JSON written before the field existed.
    #[serde(default)]
    pub components: Vec<ComponentBits>,
}

/// One term of the evidence sum (`match-evidence.md` §8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentBits {
    /// The component, e.g. `"name"`, `"identifier"`,
    /// `"contradiction:identifier:lei"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}

// ─── Correlated components (match-evidence.md §6) ───────────────────

/// Weight of each member of a correlated group after its strongest.
pub const CORRELATED_DISCOUNT: f64 = 0.5;

/// Groups of components that are not independent given the match
/// status; within a group the strongest counts in full and the rest at
/// [`CORRELATED_DISCOUNT`]. Public contract.
///
/// - `address` + `jurisdiction` — the country is determined by the
///   registered address; a subsidiary sharing its parent's office
///   agrees on both for one reason.
/// - `keywords` + `tags` — both are descriptive labels drawn from the
///   same sector vocabulary, so they agree (or not) together.
/// - `name` + `url` — a domain is usually the name, and a brand family
///   (parent, subsidiaries, franchises) shares both.
pub const CORRELATED_GROUPS: &[&[&str]] = &[
    &["address", "jurisdiction"],
    &["keywords", "tags"],
    &["name", "url"],
];

/// Index of the strongest member: largest absolute value, ties broken
/// by the larger value, then by the earlier position.
fn strongest_index(member_bits: &[f64]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, &b) in member_bits.iter().enumerate() {
        best = match best {
            None => Some(i),
            Some(j) => {
                let c = member_bits[j];
                let better = b.abs().total_cmp(&c.abs()).then(b.total_cmp(&c)).is_gt();
                Some(if better { i } else { j })
            }
        };
    }
    best
}

/// Split a group's present members into the strongest and the rest,
/// ordering by absolute value with ties to the larger value, so the
/// result does not depend on component order. `None` when empty.
#[must_use]
pub fn strongest_first(member_bits: &[f64]) -> Option<(f64, Vec<f64>)> {
    let i = strongest_index(member_bits)?;
    let rest = member_bits
        .iter()
        .enumerate()
        .filter(|&(j, _)| j != i)
        .map(|(_, &b)| b)
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

// ─── Common and rare values (match-evidence.md §7) ──────────────────

/// Where the share of records holding a value comes from.
pub trait ValueFrequencies {
    /// The share, in `0.0..=1.0`, of records whose `component` holds
    /// `value` (already in the matcher's normalised form, see
    /// [`frequency_key`]), or `None` when unknown.
    fn share(&self, component: &str, value: &str) -> Option<f64>;
}

/// A frequency source that knows nothing: the plain §2 table applies.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFrequencies;

impl ValueFrequencies for NoFrequencies {
    fn share(&self, _component: &str, _value: &str) -> Option<f64> {
        None
    }
}

/// An in-memory [`ValueFrequencies`] keyed by `(component, value)`,
/// where `value` is a [`frequency_key`].
#[derive(Debug, Clone, Default)]
pub struct FrequencyTable {
    shares: BTreeMap<(String, String), f64>,
}

impl FrequencyTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from `(component, value, count)` rows over a population of
    /// `total` records: each share is `count / total`. A `total` of `0`
    /// yields an empty table.
    #[must_use]
    pub fn from_counts<C, V>(counts: impl IntoIterator<Item = (C, V, u64)>, total: u64) -> Self
    where
        C: Into<String>,
        V: Into<String>,
    {
        let mut table = Self::new();
        if total == 0 {
            return table;
        }
        // u64 → f64 is lossy only beyond 2^53 records, far past any registry.
        #[allow(clippy::cast_precision_loss)]
        let total = total as f64;
        for (component, value, count) in counts {
            #[allow(clippy::cast_precision_loss)]
            let share = count as f64 / total;
            table.insert(component, value, share);
        }
        table
    }

    /// Record the share of records whose `component` holds `value`.
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

/// Agreement bits for a value held by `share` of records. A share at or
/// above `m` gives `0`, never negative; a non-finite share gives `0`.
#[must_use]
pub fn frequency_bits(m: f64, share: f64) -> f64 {
    let share = if share.is_finite() {
        share.clamp(MIN_SHARE, m)
    } else {
        m
    };
    (m / share).log2()
}

/// Components whose exact agreement is weighed by the value's own
/// frequency. Public contract: names such as "ABC Ltd" or "City Council"
/// recur across jurisdictions, so agreeing on one proves little.
pub const FREQUENCY_COMPONENTS: &[&str] = &["name"];

/// The key the matcher looks up for `raw` under `component`, in the
/// same normalised form it compares: for `"name"`, the legal-suffix-
/// stripped name ([`normalize::legal_name`]), used for `name`,
/// `legal_name` and each alternate name alike. `None` for a component
/// not in [`FREQUENCY_COMPONENTS`] or a value that normalises to empty.
#[must_use]
pub fn frequency_key(component: &str, raw: &str) -> Option<String> {
    match component {
        "name" => {
            let key = normalize::legal_name(raw);
            (!key.is_empty()).then_some(key)
        }
        _ => None,
    }
}

impl Evidence {
    /// Build the evidence for one pair.
    ///
    /// `breakdown` must carry the computed components (also on the
    /// deterministic path). `frequency_shares` gives, per component in
    /// [`FREQUENCY_COMPONENTS`], the share of the value both records
    /// hold exactly; the caller supplies one only on exact agreement.
    /// `identifier_shared` is whether R-0/R-1/R-2 fired; `contradictions`
    /// comes from [`identifier_contradictions`].
    #[must_use]
    pub fn compute(
        breakdown: &MatchBreakdown,
        frequency_shares: &[(&str, f64)],
        identifier_shared: bool,
        contradictions: Vec<Contradiction>,
        is_match: bool,
        proof_bits: f64,
    ) -> Self {
        let present = [
            ("name", breakdown.name_score, table::NAME),
            ("address", breakdown.address_score, table::ADDRESS),
            ("url", breakdown.url_score, table::URL),
            (
                "jurisdiction",
                breakdown.jurisdiction_score,
                table::JURISDICTION,
            ),
            (
                "founding_date",
                breakdown.founding_date_score,
                table::FOUNDING_DATE,
            ),
            ("keywords", breakdown.keywords_score, table::KEYWORDS),
            (
                "relationships",
                breakdown.relationships_score,
                table::RELATIONSHIPS,
            ),
            ("tags", breakdown.tags_score, table::TAGS),
        ];
        let mut components: Vec<ComponentBits> = Vec::new();
        for (name, similarity, (m, u)) in present {
            // A component either record lacks adds 0 and is not listed.
            let Some(s) = similarity else { continue };
            let share = frequency_shares
                .iter()
                .find(|(c, _)| *c == name && FREQUENCY_COMPONENTS.contains(c))
                .map(|&(_, share)| share);
            // Only an exact agreement is adjusted (§7).
            let (bits, frequency_adjusted) = match share {
                Some(share) if s >= AGREE_AT => (frequency_bits(m, share), true),
                _ => (component_bits(s, m, u), false),
            };
            components.push(ComponentBits {
                component: name.into(),
                bits,
                correlated: false,
                frequency_adjusted,
            });
        }
        let fields_compared = u32::try_from(components.len()).unwrap_or(u32::MAX)
            + u32::from(identifier_shared || !contradictions.is_empty());

        // Correlated groups: the strongest present member counts in full,
        // the rest are discounted (§6).
        for group in CORRELATED_GROUPS {
            let members: Vec<usize> = components
                .iter()
                .enumerate()
                .filter(|(_, c)| group.contains(&c.component.as_str()))
                .map(|(i, _)| i)
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

        if identifier_shared {
            components.push(ComponentBits {
                component: "identifier".into(),
                bits: IDENTIFIER_BITS,
                correlated: false,
                frequency_adjusted: false,
            });
        }
        for c in &contradictions {
            components.push(ComponentBits {
                component: format!("contradiction:{}", c.field),
                bits: CONTRADICTION_BITS,
                correlated: false,
                frequency_adjusted: false,
            });
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
        Self {
            weight_bits,
            fields_compared,
            contradictions,
            strength,
            components,
        }
    }
}

/// Lowercase name of a deterministic scheme, for a contradiction field.
fn scheme_name(scheme: &IdentifierScheme) -> &'static str {
    match scheme {
        IdentifierScheme::Lei => "lei",
        IdentifierScheme::Duns => "duns",
        IdentifierScheme::Iso6523 => "iso6523",
        IdentifierScheme::Gln => "gln",
        IdentifierScheme::Wikidata => "wikidata",
        IdentifierScheme::Ror => "ror",
        IdentifierScheme::Isni => "isni",
        IdentifierScheme::Vat => "vat",
        IdentifierScheme::TaxId => "tax_id",
        IdentifierScheme::Naics => "naics",
        IdentifierScheme::IsicV4 => "isic_v4",
        IdentifierScheme::Sic => "sic",
        IdentifierScheme::Custom(_) => "custom",
    }
}

/// Non-empty folded values per scheme accepted by `keep`, keyed by the
/// scheme's lowercase name (a `BTreeMap`, so output order is stable).
fn values_by_scheme(
    o: &Organization,
    keep: impl Fn(&IdentifierScheme) -> bool,
) -> BTreeMap<&'static str, BTreeSet<String>> {
    let mut out: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    for id in &o.identifiers {
        if !keep(&id.scheme) {
            continue;
        }
        let v = normalize::fold(&id.value);
        // Blank values carry no identity and cannot contradict (SEC-M2).
        if v.is_empty() {
            continue;
        }
        out.entry(scheme_name(&id.scheme)).or_default().insert(v);
    }
    out
}

/// Hard identifier conflicts between two records.
///
/// One contradiction per deterministic scheme under which both records
/// hold at least one non-empty value and the two value sets share none
/// (`identifier:<scheme>`), plus `identifier:tax_id` when both records
/// sit in the same jurisdiction and hold disjoint non-empty tax ids.
#[must_use]
pub fn identifier_contradictions(a: &Organization, b: &Organization) -> Vec<Contradiction> {
    let mut out = Vec::new();
    let av = values_by_scheme(a, IdentifierScheme::is_deterministic);
    let bv = values_by_scheme(b, IdentifierScheme::is_deterministic);
    for (scheme, a_set) in &av {
        if let Some(b_set) = bv.get(scheme)
            && a_set.is_disjoint(b_set)
        {
            out.push(Contradiction {
                field: format!("identifier:{scheme}"),
            });
        }
    }

    // A tax id is only unique within its jurisdiction, so it can only
    // contradict when both records name the same, non-empty one.
    let same_jurisdiction = match (a.jurisdiction.as_deref(), b.jurisdiction.as_deref()) {
        (Some(aj), Some(bj)) => {
            let (aj, bj) = (normalize::fold(aj), normalize::fold(bj));
            !aj.is_empty() && aj == bj
        }
        _ => false,
    };
    if same_jurisdiction {
        let is_tax = |s: &IdentifierScheme| *s == IdentifierScheme::TaxId;
        let at = values_by_scheme(a, is_tax);
        let bt = values_by_scheme(b, is_tax);
        if let (Some(a_set), Some(b_set)) = (at.get("tax_id"), bt.get("tax_id"))
            && a_set.is_disjoint(b_set)
        {
            out.push(Contradiction {
                field: "identifier:tax_id".into(),
            });
        }
    }
    out
}

/// Unit tests for the evidence arithmetic and the contradiction rule.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::OrgIdentifier;

    fn ident(scheme: IdentifierScheme, value: &str) -> OrgIdentifier {
        OrgIdentifier {
            scheme,
            value: value.into(),
        }
    }

    #[test]
    fn identifier_bits_match_the_contract() {
        assert!((IDENTIFIER_BITS - (0.99_f64 / 1e-12).log2()).abs() < 1e-9);
        assert!((CONTRADICTION_BITS - (0.01_f64 / (1.0 - 1e-12)).log2()).abs() < 1e-9);
        const { assert!(IDENTIFIER_BITS >= DEFAULT_PROOF_BITS) };
    }

    #[test]
    fn component_bits_endpoints_and_interpolation() {
        let (m, u) = table::NAME;
        let agree = (m / u).log2();
        let disagree = ((1.0 - m) / (1.0 - u)).log2();
        assert!((component_bits(1.0, m, u) - agree).abs() < 1e-12);
        assert!((component_bits(AGREE_AT, m, u) - agree).abs() < 1e-12);
        assert!((component_bits(0.0, m, u) - disagree).abs() < 1e-12);
        assert!((component_bits(DISAGREE_AT, m, u) - disagree).abs() < 1e-12);
        let mid = component_bits(f64::midpoint(AGREE_AT, DISAGREE_AT), m, u);
        assert!((mid - f64::midpoint(agree, disagree)).abs() < 1e-9);
        assert!((component_bits(f64::NAN, m, u) - disagree).abs() < 1e-12);
        assert!(disagree < 0.0);
    }

    #[test]
    fn every_table_entry_is_a_valid_probability_pair() {
        for (m, u) in [
            table::NAME,
            table::ADDRESS,
            table::URL,
            table::JURISDICTION,
            table::FOUNDING_DATE,
            table::KEYWORDS,
            table::RELATIONSHIPS,
            table::TAGS,
        ] {
            assert!(0.0 < u && u < m && m < 1.0, "bad pair ({m}, {u})");
        }
    }

    #[test]
    fn missing_component_adds_zero_and_is_not_counted() {
        let name_only = MatchBreakdown {
            name_score: Some(1.0),
            ..Default::default()
        };
        let e = Evidence::compute(&name_only, &[], false, vec![], true, DEFAULT_PROOF_BITS);
        assert_eq!(e.fields_compared, 1);
        let (m, u) = table::NAME;
        assert!((e.weight_bits - (m / u).log2()).abs() < 1e-12);

        let none = MatchBreakdown::default();
        let e = Evidence::compute(&none, &[], false, vec![], false, DEFAULT_PROOF_BITS);
        assert_eq!(e.fields_compared, 0);
        assert!(e.weight_bits.abs() < 1e-12);
        assert_eq!(e.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn contradiction_wins_over_a_shared_identifier() {
        let e = Evidence::compute(
            &MatchBreakdown::default(),
            &[],
            true,
            vec![Contradiction {
                field: "identifier:duns".into(),
            }],
            true,
            DEFAULT_PROOF_BITS,
        );
        assert_eq!(e.strength, MatchStrength::Contradicted);
        assert_eq!(e.fields_compared, 1);
    }

    #[test]
    fn multiple_values_under_one_scheme_do_not_contradict_a_partner_holding_one() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("A");
        a.identifiers.push(ident(IdentifierScheme::Duns, "111"));
        a.identifiers.push(ident(IdentifierScheme::Duns, "222"));
        b.identifiers.push(ident(IdentifierScheme::Duns, "222"));
        assert!(identifier_contradictions(&a, &b).is_empty());
    }

    #[test]
    fn blank_values_and_one_sided_schemes_never_contradict() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("A");
        a.identifiers.push(ident(IdentifierScheme::Lei, "   "));
        b.identifiers.push(ident(IdentifierScheme::Lei, "X"));
        a.identifiers.push(ident(IdentifierScheme::Gln, "1"));
        b.identifiers.push(ident(IdentifierScheme::Naics, "1"));
        assert!(identifier_contradictions(&a, &b).is_empty());
    }

    #[test]
    fn tax_ids_contradict_only_within_one_jurisdiction() {
        let mut a = Organization::new("A");
        let mut b = Organization::new("A");
        a.identifiers.push(ident(IdentifierScheme::TaxId, "1"));
        b.identifiers.push(ident(IdentifierScheme::TaxId, "2"));
        assert!(identifier_contradictions(&a, &b).is_empty());
        a.jurisdiction = Some("US".into());
        b.jurisdiction = Some("GB".into());
        assert!(identifier_contradictions(&a, &b).is_empty());
        b.jurisdiction = Some("us".into());
        assert_eq!(
            identifier_contradictions(&a, &b),
            vec![Contradiction {
                field: "identifier:tax_id".into()
            }]
        );
    }

    #[test]
    fn strength_serialises_snake_case() {
        let json = serde_json::to_string(&MatchStrength::NonMatch).expect("serialise");
        assert_eq!(json, "\"non_match\"");
    }

    // ---------- correlated groups (§6) ----------

    fn agree((m, u): (f64, f64)) -> f64 {
        (m / u).log2()
    }

    fn disagree((m, u): (f64, f64)) -> f64 {
        ((1.0 - m) / (1.0 - u)).log2()
    }

    fn term<'a>(e: &'a Evidence, name: &str) -> &'a ComponentBits {
        e.components
            .iter()
            .find(|c| c.component == name)
            .expect("term present")
    }

    #[test]
    fn agreeing_correlated_fields_give_less_than_their_sum_in_any_order() {
        let (addr, jur) = (agree(table::ADDRESS), agree(table::JURISDICTION));
        let g = group_bits(&[addr, jur]);
        assert!(g < addr + jur);
        assert!((g - (addr + CORRELATED_DISCOUNT * jur)).abs() < 1e-12);
        assert!((group_bits(&[jur, addr]) - g).abs() < 1e-12);

        let b = MatchBreakdown {
            name_score: Some(0.0),
            address_score: Some(1.0),
            jurisdiction_score: Some(1.0),
            ..Default::default()
        };
        let e = Evidence::compute(&b, &[], false, vec![], false, DEFAULT_PROOF_BITS);
        assert!(!term(&e, "address").correlated);
        assert!(term(&e, "jurisdiction").correlated);
        let expected = disagree(table::NAME) + g;
        assert!((e.weight_bits - expected).abs() < 1e-12);
    }

    #[test]
    fn uncorrelated_fields_still_sum() {
        let b = MatchBreakdown {
            name_score: Some(1.0),
            founding_date_score: Some(1.0),
            relationships_score: Some(1.0),
            ..Default::default()
        };
        let e = Evidence::compute(&b, &[], false, vec![], true, DEFAULT_PROOF_BITS);
        let sum = agree(table::NAME) + agree(table::FOUNDING_DATE) + agree(table::RELATIONSHIPS);
        assert!((e.weight_bits - sum).abs() < 1e-12);
        assert!(e.components.iter().all(|c| !c.correlated));
    }

    #[test]
    fn correlated_disagreements_are_discounted_too() {
        let b = MatchBreakdown {
            name_score: Some(1.0),
            keywords_score: Some(0.0),
            tags_score: Some(0.0),
            ..Default::default()
        };
        let e = Evidence::compute(&b, &[], false, vec![], false, DEFAULT_PROOF_BITS);
        let (k, t) = (disagree(table::KEYWORDS), disagree(table::TAGS));
        // Keywords disagree more strongly, so they count in full.
        assert!(k.abs() > t.abs());
        assert!(term(&e, "tags").correlated);
        let expected = agree(table::NAME) + k + CORRELATED_DISCOUNT * t;
        assert!((e.weight_bits - expected).abs() < 1e-12);
        assert!(e.weight_bits > agree(table::NAME) + k + t);
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
        assert!(group_bits(&[]).abs() < 1e-12);
    }

    #[test]
    fn every_group_names_real_components() {
        let known = [
            "name",
            "address",
            "url",
            "jurisdiction",
            "founding_date",
            "keywords",
            "relationships",
            "tags",
        ];
        for group in CORRELATED_GROUPS {
            assert!(group.len() >= 2);
            assert!(group.iter().all(|c| known.contains(c)), "{group:?}");
        }
        assert!(FREQUENCY_COMPONENTS.iter().all(|c| known.contains(c)));
    }

    // ---------- value frequencies (§7) ----------

    #[test]
    fn common_name_gives_fewer_bits_than_default_and_rare_gives_more() {
        let (m, _) = table::NAME;
        let default = agree(table::NAME);
        assert!(frequency_bits(m, 0.01) < default);
        assert!(frequency_bits(m, 1e-6) > default);
    }

    #[test]
    fn share_at_or_above_m_gives_zero_never_negative() {
        let (m, _) = table::NAME;
        assert!(frequency_bits(m, m).abs() < 1e-12);
        assert!(frequency_bits(m, 1.0).abs() < 1e-12);
        assert!(frequency_bits(m, f64::NAN).abs() < 1e-12);
        assert!((frequency_bits(m, 0.0) - (m / MIN_SHARE).log2()).abs() < 1e-9);
    }

    #[test]
    fn frequency_adjusts_only_exact_agreement() {
        let shares = [("name", 0.01)];
        let exact = MatchBreakdown {
            name_score: Some(1.0),
            ..Default::default()
        };
        let e = Evidence::compute(&exact, &shares, false, vec![], true, DEFAULT_PROOF_BITS);
        assert!(term(&e, "name").frequency_adjusted);
        assert!((e.weight_bits - frequency_bits(table::NAME.0, 0.01)).abs() < 1e-12);

        for s in [0.8, 0.1] {
            let partial = MatchBreakdown {
                name_score: Some(s),
                ..Default::default()
            };
            let with = Evidence::compute(&partial, &shares, false, vec![], false, 30.0);
            let without = Evidence::compute(&partial, &[], false, vec![], false, 30.0);
            assert_eq!(with, without, "similarity {s} was adjusted");
            assert!(!term(&with, "name").frequency_adjusted);
        }
    }

    #[test]
    fn shares_for_components_outside_the_list_are_ignored() {
        let b = MatchBreakdown {
            name_score: Some(1.0),
            url_score: Some(1.0),
            ..Default::default()
        };
        let with = Evidence::compute(&b, &[("url", 0.5)], false, vec![], true, 30.0);
        let without = Evidence::compute(&b, &[], false, vec![], true, 30.0);
        assert_eq!(with, without);
    }

    #[test]
    fn frequency_key_is_the_compared_name_form() {
        assert_eq!(frequency_key("name", "ABC Ltd"), Some("abc".into()));
        assert_eq!(
            frequency_key("name", "ABC Ltd"),
            frequency_key("name", "abc, Limited")
        );
        assert_eq!(frequency_key("url", "https://abc.example"), None);
        assert_eq!(frequency_key("name", "   "), None);
    }

    #[test]
    fn frequency_table_from_counts_and_insert() {
        let mut t = FrequencyTable::from_counts([("name", "abc", 50_u64)], 1000);
        assert_eq!(t.share("name", "abc"), Some(0.05));
        assert_eq!(t.share("name", "xyz"), None);
        t.insert("name", "xyz", 0.2);
        assert_eq!(t.share("name", "xyz"), Some(0.2));
        assert_eq!(NoFrequencies.share("name", "abc"), None);
        let empty = FrequencyTable::from_counts([("name", "abc", 1_u64)], 0);
        assert_eq!(empty.share("name", "abc"), None);
    }

    // ---------- explainability (§8) ----------

    #[test]
    fn components_list_identifier_and_contradiction_terms_and_sum() {
        let b = MatchBreakdown {
            name_score: Some(1.0),
            address_score: Some(1.0),
            jurisdiction_score: Some(1.0),
            ..Default::default()
        };
        let c = vec![Contradiction {
            field: "identifier:lei".into(),
        }];
        let e = Evidence::compute(&b, &[], true, c, true, 30.0);
        assert!((term(&e, "identifier").bits - IDENTIFIER_BITS).abs() < 1e-12);
        assert!((term(&e, "contradiction:identifier:lei").bits - CONTRADICTION_BITS).abs() < 1e-12);
        let sum: f64 = e.components.iter().map(|c| c.bits).sum();
        assert!((sum - e.weight_bits).abs() < f64::EPSILON);
    }

    #[test]
    fn old_evidence_json_without_components_deserialises() {
        let old = r#"{"weight_bits":9.89,"fields_compared":1,"contradictions":[],
            "strength":"consistent"}"#;
        let e: Evidence = serde_json::from_str(old).expect("old evidence JSON");
        assert!(e.components.is_empty());
        assert_eq!(e.strength, MatchStrength::Consistent);
    }
}
