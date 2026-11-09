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
//! `m`/`u` table below is specific to this crate.

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

/// Assemble the [`Evidence`] for one pair.
///
/// `components` is the probabilistic breakdown, computed even when the
/// short-circuit fired; `identifier_shared` is whether a deterministic
/// rule (R-0, R-1 or R-2) fired. `pathway_code_score` is ignored, see
/// [`PATHWAY_CODE`].
#[must_use]
pub(crate) fn assess(
    components: &MatchBreakdown,
    identifier_shared: bool,
    contradictions: Vec<Contradiction>,
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let table = [
        (components.name_score, NAME),
        (components.condition_score, CONDITION),
        (components.care_setting_score, CARE_SETTING),
        (components.interventions_score, INTERVENTIONS),
        (components.keywords_score, KEYWORDS),
        (components.relationships_score, RELATIONSHIPS),
        (components.tags_score, TAGS),
    ];
    let mut weight_bits = 0.0_f64;
    let mut fields_compared = 0_u32;
    for (similarity, p) in table {
        // A component either record lacks adds exactly 0 and is not counted.
        if let Some(s) = similarity {
            weight_bits += component_bits(s, p.m, p.u);
            fields_compared += 1;
        }
    }
    if identifier_shared {
        weight_bits += IDENTIFIER_BITS;
    }
    for _ in &contradictions {
        weight_bits += CONTRADICTION_BITS;
    }
    // Identifiers count as one field, whether shared or contradicted.
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
        let e = assess(&name_only, false, Vec::new(), true, 30.0);
        assert_eq!(e.fields_compared, 1);
        assert!((e.weight_bits - component_bits(1.0, NAME.m, NAME.u)).abs() < 1e-12);

        let none = assess(&MatchBreakdown::default(), false, Vec::new(), false, 30.0);
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
        let e = assess(&b, false, Vec::new(), false, 30.0);
        assert_eq!(e.fields_compared, 0);
        assert!(e.weight_bits.abs() < 1e-12);
    }

    #[test]
    fn a_contradiction_beats_a_shared_identifier() {
        let e = assess(
            &MatchBreakdown::default(),
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
}
