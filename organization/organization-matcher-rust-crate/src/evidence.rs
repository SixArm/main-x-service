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
}

impl Evidence {
    /// Build the evidence for one pair.
    ///
    /// `breakdown` must carry the computed components (also on the
    /// deterministic path); `identifier_shared` is whether any of the
    /// identifier rules R-0/R-1/R-2 fired; `contradictions` comes from
    /// [`identifier_contradictions`].
    #[must_use]
    pub fn compute(
        breakdown: &MatchBreakdown,
        identifier_shared: bool,
        contradictions: Vec<Contradiction>,
        is_match: bool,
        proof_bits: f64,
    ) -> Self {
        let components = [
            (breakdown.name_score, table::NAME),
            (breakdown.address_score, table::ADDRESS),
            (breakdown.url_score, table::URL),
            (breakdown.jurisdiction_score, table::JURISDICTION),
            (breakdown.founding_date_score, table::FOUNDING_DATE),
            (breakdown.keywords_score, table::KEYWORDS),
            (breakdown.relationships_score, table::RELATIONSHIPS),
            (breakdown.tags_score, table::TAGS),
        ];
        let mut weight_bits = 0.0_f64;
        let mut fields_compared = 0_u32;
        for (similarity, (m, u)) in components {
            // A component either record lacks adds 0 and is not counted.
            if let Some(s) = similarity {
                weight_bits += component_bits(s, m, u);
                fields_compared += 1;
            }
        }
        if identifier_shared {
            weight_bits += IDENTIFIER_BITS;
        }
        for _ in &contradictions {
            weight_bits += CONTRADICTION_BITS;
        }
        if identifier_shared || !contradictions.is_empty() {
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
        Self {
            weight_bits,
            fields_compared,
            contradictions,
            strength,
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
        let e = Evidence::compute(&name_only, false, vec![], true, DEFAULT_PROOF_BITS);
        assert_eq!(e.fields_compared, 1);
        let (m, u) = table::NAME;
        assert!((e.weight_bits - (m / u).log2()).abs() < 1e-12);

        let none = MatchBreakdown::default();
        let e = Evidence::compute(&none, false, vec![], false, DEFAULT_PROOF_BITS);
        assert_eq!(e.fields_compared, 0);
        assert!(e.weight_bits.abs() < 1e-12);
        assert_eq!(e.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn contradiction_wins_over_a_shared_identifier() {
        let e = Evidence::compute(
            &MatchBreakdown::default(),
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
}
