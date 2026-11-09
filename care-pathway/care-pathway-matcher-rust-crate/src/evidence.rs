//! Match evidence — Fellegi–Sunter log-likelihood weights beside the
//! similarity score.
//!
//! The similarity `score` answers "how alike is the data both records
//! hold?". It cannot tell "nothing here contradicts a match" from "the data
//! proves a match": two pathway records holding only the same title score
//! `1.0`, and so would two trusts' independently written stroke pathways.
//! [`Evidence`] answers the second question, in bits, so a service can
//! gate an automatic merge on proof rather than similarity.
//!
//! This module implements the family contract in
//! `agents/share/match-evidence.md` verbatim; only the per-component
//! `m`/`u` table, the correlated groups and the frequency components
//! below are specific to this crate.

use serde::{Deserialize, Serialize};

use crate::care_pathway::{CarePathway, IdentifierScheme};
use crate::normalize;
use crate::scoring::MatchBreakdown;

/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;
/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// Bits a value shared under a globally unique scheme contributes, once
/// per pair: `log2(0.99 / 1e-12)` ≈ 39.85. Clears the default
/// [`crate::MatchConfig::proof_bits`] alone, which is what the
/// deterministic short-circuit means.
pub const IDENTIFIER_BITS: f64 = 39.848_637_568_953_24;

/// Bits each contradicted scheme adds: `log2(0.01 / (1 − 1e-12))` ≈ −6.64.
pub const CONTRADICTION_BITS: f64 = -6.643_856_189_773_282;

/// One component's Fellegi–Sunter probabilities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentProbabilities {
    /// Chance the component agrees when the records are the same pathway.
    pub m: f64,
    /// Chance the component agrees by coincidence for different pathways.
    pub u: f64,
}

// ── The per-component m/u table ─────────────────────────────────────
// An informed starting estimate, not a measurement (match-evidence.md §5,
// §6). A deployment that estimates its own from review-queue decisions
// should replace these.

/// Name (Jaro-Winkler + Soundex). `u`: pathway titles are short and
/// formulaic ("Acute Stroke Pathway"), so coincidental agreement is far
/// likelier than for a person's full name — about 1 in 200.
pub const NAME: ComponentProbabilities = ComponentProbabilities { m: 0.90, u: 0.005 };
/// Target condition codes (Jaccard). `u`: a registry holds several
/// pathways per condition (one per trust, per setting), so two unrelated
/// pathways share their codes about 1 time in 50.
pub const CONDITION: ComponentProbabilities = ComponentProbabilities { m: 0.90, u: 0.02 };
/// Provider-scoped pathway code. Listed for completeness and **not
/// summed**: an agreeing code under one provider is the R-1 identifier
/// (counted as [`IDENTIFIER_BITS`]) and a differing one is the
/// `identifier:pathway_code` contradiction, so adding a component weight
/// as well would count the same fact twice. `u`: one code per provider.
pub const PATHWAY_CODE: ComponentProbabilities = ComponentProbabilities { m: 0.95, u: 0.001 };
/// Care setting (exact enum equality). `u`: a handful of settings with
/// acute/inpatient dominant, so about 1 in 4 by coincidence.
pub const CARE_SETTING: ComponentProbabilities = ComponentProbabilities { m: 0.90, u: 0.25 };
/// Interventions (Jaccard). `u`: overlapping intervention lists are common
/// across pathways for related conditions — about 1 in 10. Weak: sets
/// corroborate, they do not identify.
pub const INTERVENTIONS: ComponentProbabilities = ComponentProbabilities { m: 0.80, u: 0.10 };
/// Keywords (Jaccard). `u`: free-text keywords drift between editions and
/// recur across a specialty — about 1 in 10.
pub const KEYWORDS: ComponentProbabilities = ComponentProbabilities { m: 0.80, u: 0.10 };
/// Relationships (typed `(relation, pathway_id)` Jaccard). `u`: sharing a
/// reference to the same specific pathway is uncommon for unrelated
/// records — about 1 in 20; `m` is low because the field is sparse and
/// edited independently.
pub const RELATIONSHIPS: ComponentProbabilities = ComponentProbabilities { m: 0.70, u: 0.05 };
/// Tags (folded Jaccard). `u`: operator tags are a small, reused
/// vocabulary — about 1 in 5.
pub const TAGS: ComponentProbabilities = ComponentProbabilities { m: 0.70, u: 0.20 };

/// The weight, in bits, one compared component adds.
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

/// The verdict on how strongly the evidence supports a match.
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
    /// What conflicts, e.g. `"identifier:doi"`.
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
    /// The verdict (match-evidence.md §4).
    pub strength: MatchStrength,
    /// Every term of `weight_bits`, in order; they sum to it exactly
    /// (match-evidence.md §8). Absent in JSON written before the field.
    #[serde(default)]
    pub components: Vec<ComponentBits>,
}

/// The deterministic schemes, in a fixed order so the contradiction list
/// is the same whichever record is `a`, each with its field label.
const SCHEMES: [(IdentifierScheme, &str); 5] = [
    (IdentifierScheme::Doi, "identifier:doi"),
    (IdentifierScheme::Wikidata, "identifier:wikidata"),
    (IdentifierScheme::GuidelineId, "identifier:guidelineid"),
    (IdentifierScheme::Uri, "identifier:uri"),
    (IdentifierScheme::Uuid, "identifier:uuid"),
];

/// Hard conflicts between `a` and `b`: a deterministic scheme under which
/// both hold at least one non-empty value and the sets share none, and a
/// shared provider whose two non-empty pathway codes differ.
#[must_use]
pub(crate) fn contradictions(a: &CarePathway, b: &CarePathway, cap: usize) -> Vec<Contradiction> {
    let mut out = Vec::new();
    for (scheme, field) in &SCHEMES {
        let values = |p: &CarePathway| -> Vec<String> {
            p.identifiers
                .iter()
                .take(cap)
                .filter(|i| i.scheme == *scheme)
                .map(|i| normalize::fold(&i.value))
                .filter(|v| !v.is_empty())
                .collect()
        };
        let av = values(a);
        let bv = values(b);
        if !av.is_empty() && !bv.is_empty() && !av.iter().any(|v| bv.contains(v)) {
            out.push(Contradiction {
                field: (*field).to_string(),
            });
        }
    }
    // Same provider, different non-empty pathway codes: the provider says
    // these are two of its pathways (or one record carries a wrong code).
    if let (Some(ap), Some(bp), Some(ac), Some(bc)) = (
        a.provider_id.as_deref(),
        b.provider_id.as_deref(),
        a.pathway_code.as_deref(),
        b.pathway_code.as_deref(),
    ) && !ap.is_empty()
        && ap == bp
    {
        let an = normalize::pathway_code(ac);
        let bn = normalize::pathway_code(bc);
        if !an.is_empty() && !bn.is_empty() && an != bn {
            out.push(Contradiction {
                field: "identifier:pathway_code".to_string(),
            });
        }
    }
    out
}

// ── Correlated components (match-evidence.md §6) ────────────────────

/// Weight of each member of a correlated group after its strongest.
pub const CORRELATED_DISCOUNT: f64 = 0.5;

/// This crate's correlated groups, by component name. Public contract.
///
/// - `condition` + `interventions`: a pathway's target condition largely
///   determines its interventions (every stroke pathway lists
///   thrombolysis and a CT head), so the two agree for one reason.
/// - `keywords` + `tags`: both are free labels drawn from the same
///   specialty vocabulary, often applied by the same editor.
///
/// `care_setting` is deliberately in no group. The same condition and
/// interventions are run in acute, community and home settings, so the
/// setting is a separate fact about the pathway, and putting it beside
/// `interventions` would also make one component a member of two groups.
pub const CORRELATED_GROUPS: &[&[&str]] = &[&["condition", "interventions"], &["keywords", "tags"]];

/// Index of the strongest of `bits`: largest absolute value, ties to the
/// larger value, exact ties to the first.
fn strongest_index(bits: &[f64]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, &b) in bits.iter().enumerate() {
        best = match best {
            None => Some(i),
            Some(j) => {
                let c = bits[j];
                // Larger absolute value, then larger value; exact ties keep
                // the earlier member.
                if b.abs().total_cmp(&c.abs()).then(b.total_cmp(&c)) == std::cmp::Ordering::Greater
                {
                    Some(i)
                } else {
                    Some(j)
                }
            }
        };
    }
    best
}

/// The strongest of `member_bits` (largest absolute value, ties to the
/// larger value) and the rest, the rest in descending order so the result
/// does not depend on the order the members were given in.
#[must_use]
pub fn strongest_first(member_bits: &[f64]) -> Option<(f64, Vec<f64>)> {
    let i = strongest_index(member_bits)?;
    let mut rest: Vec<f64> = member_bits
        .iter()
        .enumerate()
        .filter(|(j, _)| *j != i)
        .map(|(_, b)| *b)
        .collect();
    rest.sort_by(|x, y| y.total_cmp(x));
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

// ── Common and rare values (match-evidence.md §7) ───────────────────

/// Where the share of records holding a value comes from.
pub trait ValueFrequencies {
    /// The share, in `0.0..=1.0`, of records whose `component` holds
    /// `value` (already in the matcher's normalised form), or `None`
    /// when unknown.
    fn share(&self, component: &str, value: &str) -> Option<f64>;
}

/// No frequencies: every agreement uses the table's `u`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFrequencies;

impl ValueFrequencies for NoFrequencies {
    fn share(&self, _component: &str, _value: &str) -> Option<f64> {
        None
    }
}

/// An in-memory `(component, value) → share` table. Values should be
/// keyed with [`frequency_key`], the form the matcher looks up.
#[derive(Debug, Clone, Default)]
pub struct FrequencyTable {
    shares: std::collections::HashMap<(String, String), f64>,
}

impl FrequencyTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from `(component, value, count)` entries over `total`
    /// records: each share is `count / total`. A `total` of `0` gives an
    /// empty table.
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
            // u64 → f64 loses precision only past 2^53 records.
            #[allow(clippy::cast_precision_loss)]
            let share = count as f64 / total as f64;
            table.insert(component, value, share);
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

/// The components whose agreement weight adjusts to the value's own
/// frequency. Public contract: pathway titles such as "Stroke Pathway"
/// recur across trusts, so agreeing on one proves far less than agreeing
/// on a distinctive title.
pub const FREQUENCY_COMPONENTS: &[&str] = &["name"];

/// The key the matcher looks up for `component`'s raw value: the same
/// normalised form it compares. `name` is the case-folded, NFKC-normalised
/// primary name ([`normalize::fold`]). `None` for a component not in
/// [`FREQUENCY_COMPONENTS`] or a value that normalises to empty.
#[must_use]
pub fn frequency_key(component: &str, raw: &str) -> Option<String> {
    match component {
        "name" => {
            let key = normalize::fold(raw);
            if key.is_empty() { None } else { Some(key) }
        }
        _ => None,
    }
}

// ── Explainability (match-evidence.md §8) ───────────────────────────

/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentBits {
    /// The component, e.g. `"name"`, `"identifier"`,
    /// `"contradiction:identifier:doi"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}

/// Assemble the [`Evidence`] for one pair.
///
/// `components` is the probabilistic breakdown, computed even when the
/// short-circuit fired; `name_share` is the share of records holding the
/// pair's shared name, when the two primary names normalise to one key
/// and the caller's frequencies know it; `identifier_shared` is whether a
/// deterministic rule (R-0, R-1 or R-2) fired. `pathway_code_score` is
/// ignored, see [`PATHWAY_CODE`].
#[must_use]
pub(crate) fn assess(
    components: &MatchBreakdown,
    name_share: Option<f64>,
    identifier_shared: bool,
    contradictions: Vec<Contradiction>,
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let table = [
        ("name", components.name_score, NAME),
        ("condition", components.condition_score, CONDITION),
        ("care_setting", components.care_setting_score, CARE_SETTING),
        (
            "interventions",
            components.interventions_score,
            INTERVENTIONS,
        ),
        ("keywords", components.keywords_score, KEYWORDS),
        (
            "relationships",
            components.relationships_score,
            RELATIONSHIPS,
        ),
        ("tags", components.tags_score, TAGS),
    ];
    let mut terms: Vec<ComponentBits> = Vec::new();
    for (name, similarity, p) in table {
        // A component either record lacks adds exactly 0 and is not listed.
        let Some(s) = similarity else { continue };
        // Only an exact agreement on a frequency component adjusts.
        let share = if FREQUENCY_COMPONENTS.contains(&name) && s >= AGREE_AT {
            if name == "name" { name_share } else { None }
        } else {
            None
        };
        let (bits, frequency_adjusted) = match share {
            Some(share) => (frequency_bits(p.m, share), true),
            None => (component_bits(s, p.m, p.u), false),
        };
        terms.push(ComponentBits {
            component: name.to_string(),
            bits,
            correlated: false,
            frequency_adjusted,
        });
    }
    let fields_compared_components = u32::try_from(terms.len()).unwrap_or(u32::MAX);

    // Correlated groups: the strongest present member counts in full, the
    // others at CORRELATED_DISCOUNT.
    for group in CORRELATED_GROUPS {
        let members: Vec<usize> = terms
            .iter()
            .enumerate()
            .filter(|(_, t)| group.contains(&t.component.as_str()))
            .map(|(i, _)| i)
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

    if identifier_shared {
        terms.push(ComponentBits {
            component: "identifier".to_string(),
            bits: IDENTIFIER_BITS,
            correlated: false,
            frequency_adjusted: false,
        });
    }
    for c in &contradictions {
        terms.push(ComponentBits {
            component: format!("contradiction:{}", c.field),
            bits: CONTRADICTION_BITS,
            correlated: false,
            frequency_adjusted: false,
        });
    }
    // The terms are the sum, by construction.
    let weight_bits: f64 = terms.iter().map(|t| t.bits).sum();
    // Identifiers count as one field, whether shared or contradicted.
    let fields_compared =
        fields_compared_components + u32::from(identifier_shared || !contradictions.is_empty());
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

    #[test]
    fn identifier_and_contradiction_constants_match_their_formulas() {
        assert!((IDENTIFIER_BITS - (0.99_f64 / 1e-12).log2()).abs() < 1e-9);
        assert!((CONTRADICTION_BITS - (0.01_f64 / (1.0 - 1e-12)).log2()).abs() < 1e-9);
    }

    #[test]
    fn component_bits_interpolates_between_disagree_and_agree() {
        let p = NAME;
        let agree = (p.m / p.u).log2();
        let disagree = ((1.0 - p.m) / (1.0 - p.u)).log2();
        assert!((component_bits(1.0, p.m, p.u) - agree).abs() < 1e-12);
        assert!((component_bits(AGREE_AT, p.m, p.u) - agree).abs() < 1e-12);
        assert!((component_bits(0.0, p.m, p.u) - disagree).abs() < 1e-12);
        assert!((component_bits(DISAGREE_AT, p.m, p.u) - disagree).abs() < 1e-12);
        let mid = component_bits(0.775, p.m, p.u);
        assert!((mid - f64::midpoint(agree, disagree)).abs() < 1e-9);
        // Non-finite similarity is treated as full disagreement.
        assert!((component_bits(f64::NAN, p.m, p.u) - disagree).abs() < 1e-12);
    }

    #[test]
    fn every_table_entry_has_positive_agreement_and_negative_disagreement() {
        for p in [
            NAME,
            CONDITION,
            PATHWAY_CODE,
            CARE_SETTING,
            INTERVENTIONS,
            KEYWORDS,
            RELATIONSHIPS,
            TAGS,
        ] {
            assert!(component_bits(1.0, p.m, p.u) > 0.0);
            assert!(component_bits(0.0, p.m, p.u) < 0.0);
        }
    }

    #[test]
    fn a_missing_component_adds_zero_and_is_not_counted() {
        let name_only = MatchBreakdown {
            name_score: Some(1.0),
            ..MatchBreakdown::default()
        };
        let e = assess(&name_only, None, false, Vec::new(), true, 30.0);
        assert_eq!(e.fields_compared, 1);
        assert!((e.weight_bits - component_bits(1.0, NAME.m, NAME.u)).abs() < 1e-12);

        let none = assess(
            &MatchBreakdown::default(),
            None,
            false,
            Vec::new(),
            false,
            30.0,
        );
        assert_eq!(none.fields_compared, 0);
        assert!(none.weight_bits.abs() < 1e-12);
        assert_eq!(none.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn pathway_code_component_is_not_summed() {
        let b = MatchBreakdown {
            pathway_code_score: Some(1.0),
            ..MatchBreakdown::default()
        };
        let e = assess(&b, None, false, Vec::new(), false, 30.0);
        assert_eq!(e.fields_compared, 0);
        assert!(e.weight_bits.abs() < 1e-12);
    }

    #[test]
    fn a_contradiction_beats_a_shared_identifier() {
        let e = assess(
            &MatchBreakdown::default(),
            None,
            true,
            vec![Contradiction {
                field: "identifier:doi".into(),
            }],
            true,
            30.0,
        );
        assert_eq!(e.strength, MatchStrength::Contradicted);
        assert_eq!(e.fields_compared, 1);
        assert!((e.weight_bits - (IDENTIFIER_BITS + CONTRADICTION_BITS)).abs() < 1e-9);
    }

    #[test]
    fn strength_serialises_snake_case() {
        let json = serde_json::to_string(&MatchStrength::NonMatch).unwrap_or_default();
        assert_eq!(json, "\"non_match\"");
    }

    // Two agreeing correlated fields give less than their sum, and the
    // result does not depend on the order the members are given in.
    #[test]
    fn correlated_agreements_give_less_than_their_sum_in_any_order() {
        let c = component_bits(1.0, CONDITION.m, CONDITION.u);
        let i = component_bits(1.0, INTERVENTIONS.m, INTERVENTIONS.u);
        let g = group_bits(&[c, i]);
        assert!(g < c + i);
        assert!((g - (c + CORRELATED_DISCOUNT * i)).abs() < 1e-12);
        assert_eq!(g.to_bits(), group_bits(&[i, c]).to_bits());
        let three = [1.0, -3.0, 2.0];
        assert_eq!(
            group_bits(&three).to_bits(),
            group_bits(&[2.0, 1.0, -3.0]).to_bits()
        );
        assert!((group_bits(&three) - (-3.0 + 0.5 * 3.0)).abs() < 1e-12);
        // Ties on absolute value go to the larger value.
        assert_eq!(strongest_first(&[-2.0, 2.0]).map(|(s, _)| s), Some(2.0));
        assert!(group_bits(&[]).abs() < 1e-12);
    }

    // Correlated disagreements are one piece of evidence against, too.
    #[test]
    fn correlated_disagreements_are_discounted() {
        let b = MatchBreakdown {
            condition_score: Some(0.0),
            interventions_score: Some(0.0),
            ..MatchBreakdown::default()
        };
        let e = assess(&b, None, false, Vec::new(), false, 30.0);
        let c = component_bits(0.0, CONDITION.m, CONDITION.u);
        let i = component_bits(0.0, INTERVENTIONS.m, INTERVENTIONS.u);
        assert!(e.weight_bits < 0.0);
        assert!(e.weight_bits > c + i, "{} vs {}", e.weight_bits, c + i);
        assert!((e.weight_bits - group_bits(&[c, i])).abs() < 1e-12);
        assert_eq!(e.components.iter().filter(|t| t.correlated).count(), 1);
    }

    // Components in different groups, or in none, still sum in full.
    #[test]
    fn uncorrelated_fields_still_sum() {
        let b = MatchBreakdown {
            name_score: Some(1.0),
            condition_score: Some(1.0),
            keywords_score: Some(1.0),
            care_setting_score: Some(1.0),
            ..MatchBreakdown::default()
        };
        let e = assess(&b, None, false, Vec::new(), true, 30.0);
        let expected = component_bits(1.0, NAME.m, NAME.u)
            + component_bits(1.0, CONDITION.m, CONDITION.u)
            + component_bits(1.0, KEYWORDS.m, KEYWORDS.u)
            + component_bits(1.0, CARE_SETTING.m, CARE_SETTING.u);
        assert!((e.weight_bits - expected).abs() < 1e-12);
        assert!(e.components.iter().all(|t| !t.correlated));
    }

    #[test]
    fn frequency_bits_clamps_to_zero_and_to_min_share() {
        assert!(frequency_bits(0.9, 0.9).abs() < 1e-12);
        assert!(frequency_bits(0.9, 1.0).abs() < 1e-12);
        assert!(frequency_bits(0.9, f64::NAN).abs() < 1e-12);
        assert!((frequency_bits(0.9, 0.0) - (0.9 / MIN_SHARE).log2()).abs() < 1e-9);
    }

    #[test]
    fn frequency_key_and_table() {
        assert_eq!(
            frequency_key("name", "  Stroke PATHWAY "),
            frequency_key("name", "stroke pathway")
        );
        assert_eq!(frequency_key("name", "   "), None);
        assert_eq!(frequency_key("keywords", "stroke"), None);
        let t = FrequencyTable::from_counts([("name", "stroke pathway", 25_u64)], 100);
        assert_eq!(t.share("name", "stroke pathway"), Some(0.25));
        assert_eq!(t.share("name", "other"), None);
        assert!(
            FrequencyTable::from_counts([("name", "x", 1_u64)], 0)
                .share("name", "x")
                .is_none()
        );
        assert_eq!(NoFrequencies.share("name", "stroke pathway"), None);
    }

    // A share is used only on exact agreement of a frequency component.
    #[test]
    fn share_is_ignored_below_agreement() {
        let b = MatchBreakdown {
            name_score: Some(0.8),
            ..MatchBreakdown::default()
        };
        let e = assess(&b, Some(1e-6), false, Vec::new(), false, 30.0);
        assert!(!e.components[0].frequency_adjusted);
        assert!((e.weight_bits - component_bits(0.8, NAME.m, NAME.u)).abs() < 1e-12);
    }
}
