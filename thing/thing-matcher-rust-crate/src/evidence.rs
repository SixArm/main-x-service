//! Match evidence: Fellegi–Sunter log-likelihood weights beside the
//! similarity score.
//!
//! [`crate::MatchResult::score`] answers "how alike is the data both
//! records hold?". [`Evidence`] answers a different question: "how much
//! does that data *prove*?". Two sparse records holding only the same
//! name agree perfectly on everything they hold, and score `1.0`, yet a
//! shared name proves little. Auto-merge is gated on evidence, not on
//! similarity.
//!
//! The model, the formula, the result shape, the contradiction rule and
//! the default bar are fixed family-wide by
//! `agents/share/match-evidence.md` (repository root). Only the per-
//! component `m`/`u` table below is specific to this crate. See spec
//! §5.12 / §8.
//!
//! ## Rules this crate applies
//!
//! - A component either record lacks adds `0` bits and is not counted.
//! - **Identity group.** The deterministic rule ([`crate::MatchingEngine::deterministic_match`])
//!   treats a shared `(property_id, value)` identifier, a shared `sameAs`
//!   URL, and an equal canonical `url` as identity. Any of those agreeing
//!   adds [`IDENTIFIER_BITS`] **once** per pair. A `url` or `sameAs` that
//!   is present on both sides but does not agree adds its own
//!   disagreement weight instead.
//! - **Contradictions.** Every `property_id` is a scheme (the deterministic
//!   rule trusts every scheme, so this one does too). A scheme under which
//!   both records hold at least one non-blank value, with no value in
//!   common, is a [`Contradiction`] named `identifier:<scheme lowercase>`
//!   and adds [`CONTRADICTION_BITS`].

use crate::matcher::MatchBreakdown;
use crate::models::Thing;
use crate::normalizer::Normalizer;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;
/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// Weight, in bits, of a value shared under a globally unique scheme:
/// `log2(0.99 / 1e-12) ≈ 39.85`. Clears the default bar alone.
pub const IDENTIFIER_BITS: f64 = 39.848_637_568_953_24;

/// Weight, in bits, of one contradicted scheme:
/// `log2(0.01 / (1 − 1e-12)) ≈ −6.64`.
pub const CONTRADICTION_BITS: f64 = -6.643_856_189_773_282;

/// Default [`crate::MatchConfig::proof_bits`]: a million records at one
/// false merge in a thousand needs `20 + 10 = 30` bits.
pub const DEFAULT_PROOF_BITS: f64 = 30.0;

// ---- Per-component m / u table ----------------------------------------
//
// `m` = P(agree | same thing); `u` = P(agree | different things). These
// are informed starting estimates, not measurements (match-evidence.md
// §5). Each `u` line says where the estimate comes from.

/// Name: `u` — two unrelated things rarely share a normalised name
/// (title, product name, landmark), though generic names ("Chair") exist.
pub const NAME: (f64, f64) = (0.95, 1e-3);
/// Soundex on names: `u` — 4-character codes collide often, and the
/// signal largely repeats `NAME`, so it is kept weak on purpose.
pub const NAME_PHONETIC: (f64, f64) = (0.90, 0.30);
/// Description: `u` — free text near-equal by chance is very rare;
/// `m` is low because descriptions are rewritten between sources.
pub const DESCRIPTION: (f64, f64) = (0.70, 1e-3);
/// Disambiguating description: `u` — short, more formulaic than a
/// description ("1813 novel"), so coincidences are likelier.
pub const DISAMBIGUATING_DESCRIPTION: (f64, f64) = (0.80, 5e-3);
/// Identifiers: `u` is the family-wide unique-scheme coincidence rate
/// (`1e-12`); agreement is [`IDENTIFIER_BITS`], a conflict
/// [`CONTRADICTION_BITS`].
pub const IDENTIFIERS: (f64, f64) = (0.99, 1e-12);
/// Canonical `url`: `u` — unrelated things almost never share a page.
/// Agreement is identity ([`IDENTIFIER_BITS`]); only its disagreement
/// weight is used, and `m` is low because URLs move.
pub const URL: (f64, f64) = (0.70, 1e-6);
/// `sameAs` set: `u` — as for `url`. Any overlap is identity; disjoint
/// sets are weak counter-evidence (sources link different authorities).
pub const SAME_AS: (f64, f64) = (0.50, 1e-6);
/// `image` URL: `u` — placeholder / stock images are shared across many
/// records, so it is far above a page URL's.
pub const IMAGE: (f64, f64) = (0.60, 1e-3);
/// `mainEntityOfPage`: `u` — a page about one thing is rarely the main
/// page of another, but it is not a deterministic rule here.
pub const MAIN_ENTITY_OF_PAGE: (f64, f64) = (0.70, 1e-4);
/// `additionalType` set: `u` — common types (`schema.org/Book`) are shared
/// by a large fraction of any catalogue.
pub const ADDITIONAL_TYPES: (f64, f64) = (0.90, 0.10);
/// Relationship set: `u` — overlapping related-thing sets corroborate but
/// do not identify (siblings share a parent).
pub const RELATIONSHIPS: (f64, f64) = (0.60, 0.05);
/// Tag set: `u` — operator tags come from a small vocabulary and repeat
/// widely.
pub const TAGS: (f64, f64) = (0.60, 0.10);

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

/// How strong the evidence for a match is (match-evidence.md §4).
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
    /// What conflicts, e.g. `"identifier:isbn"`.
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

/// Running total of bits and compared fields.
#[derive(Default)]
struct Tally {
    bits: f64,
    fields: u32,
}

impl Tally {
    fn add(&mut self, similarity: Option<f64>, (m, u): (f64, f64)) {
        if let Some(s) = similarity {
            self.bits += component_bits(s, m, u);
            self.fields += 1;
        }
    }
}

/// Compute the evidence for one compared pair.
///
/// `breakdown` is the pair's [`MatchBreakdown`]; the things are passed
/// too because the identity group and the two set components need the
/// raw values to tell "absent on one side" from "present and different"
/// (the similarity breakdown scores an empty-vs-non-empty `sameAs` or
/// `additionalType` set as `0.0`, which evidence must not count).
#[must_use]
pub(crate) fn compute(
    breakdown: &MatchBreakdown,
    thing1: &Thing,
    thing2: &Thing,
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let mut tally = Tally::default();

    // Plain components: `None` in the breakdown means absent on a side.
    tally.add(breakdown.name_score, NAME);
    tally.add(breakdown.name_phonetic_score, NAME_PHONETIC);
    tally.add(breakdown.description_score, DESCRIPTION);
    tally.add(
        breakdown.disambiguating_description_score,
        DISAMBIGUATING_DESCRIPTION,
    );
    tally.add(
        present_url_pair(thing1.image.as_ref(), thing2.image.as_ref()).and(breakdown.image_score),
        IMAGE,
    );
    tally.add(
        present_url_pair(
            thing1.main_entity_of_page.as_ref(),
            thing2.main_entity_of_page.as_ref(),
        )
        .and(breakdown.main_entity_of_page_score),
        MAIN_ENTITY_OF_PAGE,
    );
    let both_types = !url_set(&thing1.additional_types).is_empty()
        && !url_set(&thing2.additional_types).is_empty();
    tally.add(
        breakdown.additional_types_score.filter(|_| both_types),
        ADDITIONAL_TYPES,
    );
    tally.add(breakdown.relationships_score, RELATIONSHIPS);
    tally.add(breakdown.tags_score, TAGS);

    // Identity group: identifiers, canonical url, sameAs.
    let mut identity = false;

    let contradictions = identifier_contradictions(thing1, thing2);
    let shared_identifier = shares_identifier(thing1, thing2);
    identity |= shared_identifier;
    if shared_identifier || !contradictions.is_empty() {
        // Identifiers count as one field however many schemes compared.
        tally.fields += 1;
    }
    for _ in &contradictions {
        tally.bits += CONTRADICTION_BITS;
    }

    if let Some((a, b)) = normalized_url_pair(thing1.url.as_ref(), thing2.url.as_ref()) {
        if a == b {
            identity = true;
        } else {
            tally.add(Some(0.0), URL);
        }
    }

    let same_as1 = url_set(&thing1.same_as);
    let same_as2 = url_set(&thing2.same_as);
    if !same_as1.is_empty() && !same_as2.is_empty() {
        if same_as1.is_disjoint(&same_as2) {
            tally.add(Some(0.0), SAME_AS);
        } else {
            identity = true;
        }
    }

    if identity {
        tally.bits += IDENTIFIER_BITS;
        // The shared identifier already counted itself above.
        if !shared_identifier && contradictions.is_empty() {
            tally.fields += 1;
        }
    }

    let strength = if !contradictions.is_empty() {
        MatchStrength::Contradicted
    } else if tally.bits >= proof_bits {
        MatchStrength::Proven
    } else if is_match {
        MatchStrength::Consistent
    } else {
        MatchStrength::NonMatch
    };

    Evidence {
        weight_bits: tally.bits,
        fields_compared: tally.fields,
        contradictions,
        strength,
    }
}

/// Both URLs normalised, iff both are present and normalise to non-empty
/// strings. A blank or fragment-only value is absent, not a value.
fn normalized_url_pair(a: Option<&String>, b: Option<&String>) -> Option<(String, String)> {
    let a = Normalizer::normalize_url(a?);
    let b = Normalizer::normalize_url(b?);
    (!a.is_empty() && !b.is_empty()).then_some((a, b))
}

/// `Some(())` iff both URLs are present and non-blank after normalising.
fn present_url_pair(a: Option<&String>, b: Option<&String>) -> Option<()> {
    normalized_url_pair(a, b).map(|_| ())
}

/// The non-empty normalised URLs of a list.
fn url_set(urls: &[String]) -> BTreeSet<String> {
    urls.iter()
        .map(|s| Normalizer::normalize_url(s))
        .filter(|s| !s.is_empty())
        .collect()
}

/// Each record's non-blank identifier values, grouped by trimmed
/// `property_id`. Values are compared exactly, as the deterministic rule
/// compares them.
fn identifier_schemes(thing: &Thing) -> BTreeMap<&str, BTreeSet<&str>> {
    let mut schemes: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for id in &thing.identifiers {
        let scheme = id.property_id.trim();
        if scheme.is_empty() || id.value.trim().is_empty() {
            continue;
        }
        schemes.entry(scheme).or_default().insert(id.value.as_str());
    }
    schemes
}

/// `true` iff some non-blank `(property_id, value)` pair appears on both.
fn shares_identifier(thing1: &Thing, thing2: &Thing) -> bool {
    let a = identifier_schemes(thing1);
    let b = identifier_schemes(thing2);
    a.iter().any(|(scheme, values)| {
        b.get(scheme)
            .is_some_and(|other| !values.is_disjoint(other))
    })
}

/// One contradiction per scheme both records hold with disjoint values.
fn identifier_contradictions(thing1: &Thing, thing2: &Thing) -> Vec<Contradiction> {
    let a = identifier_schemes(thing1);
    let b = identifier_schemes(thing2);
    let mut fields: BTreeSet<String> = BTreeSet::new();
    for (scheme, values) in &a {
        if let Some(other) = b.get(scheme)
            && values.is_disjoint(other)
        {
            fields.insert(format!("identifier:{}", scheme.to_lowercase()));
        }
    }
    fields
        .into_iter()
        .map(|field| Contradiction { field })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Identifier, MatchConfig, MatchingEngine, RelationKind, RelationshipRef};

    fn engine() -> MatchingEngine {
        MatchingEngine::default_config()
    }

    fn id(scheme: &str, value: &str) -> Identifier {
        Identifier::new(scheme, value).unwrap()
    }

    #[test]
    fn constants_match_their_formulas() {
        assert!((IDENTIFIER_BITS - (0.99_f64 / 1e-12).log2()).abs() < 1e-9);
        assert!((CONTRADICTION_BITS - (0.01_f64 / (1.0 - 1e-12)).log2()).abs() < 1e-9);
        let (m, u) = IDENTIFIERS;
        assert!((component_bits(1.0, m, u) - IDENTIFIER_BITS).abs() < 1e-9);
        assert!((component_bits(0.0, m, u) - CONTRADICTION_BITS).abs() < 1e-9);
    }

    #[test]
    fn component_bits_interpolates_and_handles_nan() {
        let (m, u) = NAME;
        let agree = component_bits(1.0, m, u);
        let disagree = component_bits(0.0, m, u);
        assert!(agree > 0.0 && disagree < 0.0);
        let mid = component_bits(0.775, m, u);
        assert!((mid - f64::midpoint(agree, disagree)).abs() < 1e-9);
        assert!((component_bits(f64::NAN, m, u) - disagree).abs() < 1e-12);
    }

    #[test]
    fn missing_component_adds_zero_and_is_not_counted() {
        let a = Thing::builder().name("Eiffel Tower").build();
        let b = Thing::builder()
            .name("Eiffel Tower")
            .description("Wrought-iron lattice tower")
            .build();
        let e = engine().match_things(&a, &b).evidence;
        assert_eq!(e.fields_compared, 1);
        assert!((e.weight_bits - component_bits(1.0, NAME.0, NAME.1)).abs() < 1e-9);
    }

    #[test]
    fn one_sided_same_as_and_types_are_not_counted() {
        let a = Thing::builder()
            .name("Eiffel Tower")
            .add_same_as("https://www.wikidata.org/wiki/Q243")
            .add_additional_type("https://schema.org/LandmarksOrHistoricalBuildings")
            .build();
        let b = Thing::builder().name("Eiffel Tower").build();
        let e = engine().match_things(&a, &b).evidence;
        assert_eq!(e.fields_compared, 1);
    }

    #[test]
    fn identical_sparse_record_is_consistent_not_proven() {
        let a = Thing::builder().name("Eiffel Tower").build();
        let r = engine().match_things(&a, &a.clone());
        assert!(r.is_match);
        assert_eq!(r.evidence.strength, MatchStrength::Consistent);
        assert!(r.evidence.weight_bits < DEFAULT_PROOF_BITS);
    }

    #[test]
    fn shared_unique_identifier_is_proven() {
        let a = Thing::builder()
            .name("Pride and Prejudice")
            .add_identifier(id("isbn", "9780141439518"))
            .build();
        let b = Thing::builder()
            .name("Pride & Prejudice")
            .add_identifier(id("isbn", "9780141439518"))
            .build();
        let e = engine().match_things(&a, &b).evidence;
        assert_eq!(e.strength, MatchStrength::Proven);
        assert!(e.contradictions.is_empty());
        assert_eq!(e.fields_compared, 2);
    }

    #[test]
    fn identity_signals_add_identifier_bits_once() {
        let a = Thing::builder()
            .add_identifier(id("doi", "10.1000/x"))
            .url("https://example.org/x")
            .add_same_as("https://www.wikidata.org/wiki/Q1")
            .build();
        let e = engine().match_things(&a, &a.clone()).evidence;
        assert!((e.weight_bits - IDENTIFIER_BITS).abs() < 1e-9);
        assert_eq!(e.fields_compared, 1);
    }

    #[test]
    fn shared_canonical_url_counts_as_identity() {
        let a = Thing::builder().url("https://example.org/tower").build();
        let e = engine().match_things(&a, &a.clone()).evidence;
        assert_eq!(e.strength, MatchStrength::Proven);
    }

    #[test]
    fn conflicting_same_scheme_identifiers_are_contradicted() {
        let base = || {
            Thing::builder()
                .name("Pride and Prejudice")
                .description("Novel by Jane Austen")
                .url("https://example.org/pp")
        };
        let a = base().add_identifier(id("ISBN", "9780141439518")).build();
        let b = base().add_identifier(id("ISBN", "9780199535569")).build();
        let r = engine().match_things(&a, &b);
        assert_eq!(r.evidence.strength, MatchStrength::Contradicted);
        assert_eq!(
            r.evidence.contradictions,
            vec![Contradiction {
                field: "identifier:isbn".into()
            }]
        );
    }

    #[test]
    fn shared_value_in_a_multi_valued_scheme_is_not_a_contradiction() {
        let a = Thing::builder()
            .add_identifier(id("isbn", "old"))
            .add_identifier(id("isbn", "new"))
            .build();
        let b = Thing::builder().add_identifier(id("isbn", "new")).build();
        let e = engine().match_things(&a, &b).evidence;
        assert!(e.contradictions.is_empty());
        assert_eq!(e.strength, MatchStrength::Proven);
    }

    #[test]
    fn different_schemes_neither_agree_nor_contradict() {
        let a = Thing::builder().add_identifier(id("isbn", "1")).build();
        let b = Thing::builder().add_identifier(id("gtin", "1")).build();
        let e = engine().match_things(&a, &b).evidence;
        assert!(e.contradictions.is_empty());
        assert_eq!(e.fields_compared, 0);
        assert!(e.weight_bits.abs() < 1e-12);
    }

    #[test]
    fn blank_identifier_values_are_ignored() {
        let blank = Identifier {
            property_id: "isbn".into(),
            value: "  ".into(),
        };
        let a = Thing::builder().add_identifier(blank).build();
        let b = Thing::builder().add_identifier(id("isbn", "1")).build();
        let e = engine().match_things(&a, &b).evidence;
        assert!(e.contradictions.is_empty());
    }

    #[test]
    fn contradiction_beats_a_shared_identifier_elsewhere() {
        let a = Thing::builder()
            .add_identifier(id("doi", "10.1/a"))
            .add_identifier(id("isbn", "1"))
            .build();
        let b = Thing::builder()
            .add_identifier(id("doi", "10.1/a"))
            .add_identifier(id("isbn", "2"))
            .build();
        let e = engine().match_things(&a, &b).evidence;
        assert_eq!(e.strength, MatchStrength::Contradicted);
        assert!((e.weight_bits - (IDENTIFIER_BITS + CONTRADICTION_BITS)).abs() < 1e-9);
    }

    #[test]
    fn richly_identical_record_without_identifier_is_proven() {
        let a = Thing::builder()
            .name("Eiffel Tower")
            .description("Wrought-iron lattice tower on the Champ de Mars")
            .disambiguating_description("Landmark in Paris")
            .image("https://example.org/eiffel.jpg")
            .main_entity_of_page("https://example.org/about/eiffel")
            .add_additional_type("https://schema.org/LandmarksOrHistoricalBuildings")
            .add_relationship(RelationshipRef::new(RelationKind::ContainedIn, "paris").unwrap())
            .add_tag("landmark")
            .build();
        let r = engine().match_things(&a, &a.clone());
        assert!(!engine().deterministic_match(&a, &a.clone()));
        assert_eq!(r.evidence.fields_compared, 8);
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    #[test]
    fn clear_disagreement_gives_negative_bits() {
        let a = Thing::builder()
            .name("Eiffel Tower")
            .url("https://example.org/eiffel")
            .build();
        let b = Thing::builder()
            .name("Statue of Liberty")
            .url("https://example.org/liberty")
            .build();
        let r = engine().match_things(&a, &b);
        assert!(r.evidence.weight_bits < 0.0);
        assert_eq!(r.evidence.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn proof_bits_bar_is_configurable() {
        let a = Thing::builder().name("Eiffel Tower").build();
        let low = MatchingEngine::new(MatchConfig {
            proof_bits: 5.0,
            ..MatchConfig::default()
        });
        let e = low.match_things(&a, &a.clone()).evidence;
        assert_eq!(e.strength, MatchStrength::Proven);
    }

    #[test]
    fn strength_serialises_snake_case() {
        let json = serde_json::to_string(&MatchStrength::NonMatch).unwrap();
        assert_eq!(json, "\"non_match\"");
    }

    #[test]
    fn old_json_without_evidence_deserialises() {
        let a = Thing::builder().name("Eiffel Tower").build();
        let r = engine().match_things(&a, &a.clone());
        let mut value = serde_json::to_value(&r).unwrap();
        value.as_object_mut().unwrap().remove("evidence");
        let back: crate::MatchResult = serde_json::from_value(value).unwrap();
        assert_eq!(back.evidence, Evidence::default());
        assert_eq!(back.evidence.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn old_config_json_without_proof_bits_defaults_to_30() {
        let c: MatchConfig = serde_json::from_str(r#"{"match_threshold":0.8}"#).unwrap();
        assert!((c.proof_bits - DEFAULT_PROOF_BITS).abs() < 1e-12);
    }
}
