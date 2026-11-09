//! Match evidence: Fellegi–Sunter log-likelihood weights beside the
//! similarity score.
//!
//! The similarity [`MatchResult::score`](crate::MatchResult::score) answers
//! "how alike is the data both records hold?". It renormalises over the
//! fields present, so two sparse records that agree on a name alone score
//! `1.0`. [`Evidence`] answers a different question: "how much does the
//! data prove?". A missing field adds exactly `0` bits, a shared value under
//! a globally unique external place identifier adds [`IDENTIFIER_BITS`], and
//! conflicting identifiers under the same scheme are recorded as a
//! [`Contradiction`].
//!
//! This is the family-wide contract in `agents/share/match-evidence.md`
//! (repo root); the only crate-specific part is the per-component `m`/`u`
//! table below (spec §5.5).
//!
//! ```
//! use place_matcher::{MatchStrength, MatchingEngine, Place, PlaceId, PlaceIdScheme};
//!
//! let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
//! let a = Place::builder().name("Eiffel Tower").add_place_id(id.clone()).build();
//! let b = Place::builder().name("Tour Eiffel").add_place_id(id).build();
//!
//! let result = MatchingEngine::default_config().match_places(&a, &b);
//! assert_eq!(result.evidence.strength, MatchStrength::Proven);
//! ```

use crate::models::{Place, PlaceIdScheme};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;
/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// Bits one shared globally unique identifier contributes:
/// `log2(0.99 / 1e-12)`, about `39.85`.
pub const IDENTIFIER_BITS: f64 = 39.848_637_568_953_24;

/// Bits one contradicted identifier scheme contributes:
/// `log2(0.01 / (1 - 1e-12))`, about `-6.64`.
pub const CONTRADICTION_BITS: f64 = -6.643_856_189_773_282;

/// Default [`MatchConfig::proof_bits`](crate::MatchConfig::proof_bits):
/// `log2(10^6)` records plus `log2(10^3)` for one false merge in a
/// thousand, rounded to `30`.
pub const DEFAULT_PROOF_BITS: f64 = 30.0;

/// Serde default for [`MatchConfig::proof_bits`](crate::MatchConfig::proof_bits).
#[must_use]
pub fn default_proof_bits() -> f64 {
    DEFAULT_PROOF_BITS
}

/// The `m` / `u` probabilities of one breakdown component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentWeights {
    /// Chance the component agrees when the records are the same place.
    pub m: f64,
    /// Chance the component agrees by coincidence for different places.
    pub u: f64,
}

/// `name_score`. Place names repeat far more than street addresses do
/// (chain branches, "Station Road Café", "St Mary's Church"), so `u` is a
/// few in a thousand rather than one in a million; `m` allows for
/// re-branding and transliteration.
pub const NAME: ComponentWeights = ComponentWeights { m: 0.95, u: 0.005 };

/// `name_phonetic_score`. Soundex over the same names, so it is strongly
/// correlated with `NAME` and collides often (a four-character code):
/// deliberately weak, `u` about one in three.
pub const NAME_PHONETIC: ComponentWeights = ComponentWeights { m: 0.9, u: 0.3 };

/// `coordinates_score`. Full agreement (`s >= 0.95`) at the default 50 m
/// scale means the two points are within about 11 m. Two unrelated places
/// in a registry rarely sit that close (`u` about `1e-4`); `m` allows for
/// geocoder drift and centroid-vs-entrance placement. Strong evidence.
pub const COORDINATES: ComponentWeights = ComponentWeights { m: 0.9, u: 1e-4 };

/// `address_score`. A full postal address (postcode + street + number) is
/// shared by unrelated places mainly inside one building or shopping
/// centre: `u` about one in a thousand. `m` is lower because formatting
/// and partial addresses vary between sources.
pub const ADDRESS: ComponentWeights = ComponentWeights { m: 0.85, u: 0.001 };

/// `category_score`. A handful of categories cover most places, so two
/// unrelated places share one about one time in ten.
pub const CATEGORY: ComponentWeights = ComponentWeights { m: 0.9, u: 0.1 };

/// `country_code_score`. A registry is usually dominated by a few
/// countries, so coincidental agreement is common (`u` about `0.1`); the
/// same place almost never changes country.
pub const COUNTRY_CODE: ComponentWeights = ComponentWeights { m: 0.98, u: 0.1 };

/// `place_ids_score`. Agreement is handled as an identifier
/// ([`IDENTIFIER_BITS`]), disagreement as a [`Contradiction`]; these
/// values document the implied `m`/`u` and are not read by
/// [`component_bits`].
pub const PLACE_IDS: ComponentWeights = ComponentWeights { m: 0.99, u: 1e-12 };

/// `phone_score`. Exact E.164 equality; unrelated places share a number
/// mainly through a chain's central switchboard (`u` about `1e-3`).
pub const PHONE: ComponentWeights = ComponentWeights { m: 0.85, u: 0.001 };

/// `email_score`. Exact normalised equality; chains and venue operators
/// share `info@` addresses, so `u` is a little higher than for phone.
pub const EMAIL: ComponentWeights = ComponentWeights { m: 0.8, u: 0.002 };

/// `local_id_score` (opt-in). A source-system key, unique within one
/// source but not globally, so it is weighed as a component and never as
/// an identifier; `u` about one in a thousand for a single-source caller.
pub const LOCAL_ID: ComponentWeights = ComponentWeights { m: 0.95, u: 0.001 };

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

/// One hard conflict between the two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contradiction {
    /// What conflicts, e.g. `"identifier:wikidata"`.
    pub field: String,
}

/// How strongly the evidence supports the pair being one place.
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

/// The identifier outcome for a pair of places.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct IdentifierAssessment {
    /// At least one non-empty `(scheme, value)` is held by both.
    pub shared: bool,
    /// At least one scheme is held by both (shared or contradicted).
    pub compared: bool,
    /// Schemes both hold with disjoint value sets, as `identifier:<scheme>`.
    pub contradictions: Vec<Contradiction>,
}

/// Lowercase label of a scheme for a [`Contradiction`] field.
fn scheme_label(scheme: &PlaceIdScheme) -> String {
    match scheme {
        PlaceIdScheme::Google => "google".to_string(),
        PlaceIdScheme::OsmNode => "osm_node".to_string(),
        PlaceIdScheme::OsmWay => "osm_way".to_string(),
        PlaceIdScheme::OsmRelation => "osm_relation".to_string(),
        PlaceIdScheme::GeoNames => "geonames".to_string(),
        PlaceIdScheme::Wikidata => "wikidata".to_string(),
        PlaceIdScheme::Foursquare => "foursquare".to_string(),
        PlaceIdScheme::Here => "here".to_string(),
        PlaceIdScheme::Mapbox => "mapbox".to_string(),
        PlaceIdScheme::Other(name) => format!("other:{}", name.trim().to_lowercase()),
    }
}

/// Non-empty identifier values per scheme label. Values are compared
/// trimmed, the same normalisation the deterministic rule applies.
fn ids_by_scheme(place: &Place) -> BTreeMap<String, BTreeSet<&str>> {
    let mut map: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    for id in &place.place_ids {
        let value = id.value.trim();
        if value.is_empty() {
            continue;
        }
        map.entry(scheme_label(&id.scheme))
            .or_default()
            .insert(value);
    }
    map
}

/// Compare the two places' external identifiers scheme by scheme.
///
/// Every [`PlaceIdScheme`] is treated as globally unique within itself —
/// the same assumption the deterministic short-circuit makes, which fires
/// on any shared `(scheme, value)`. `local_id` is not an identifier here.
pub(crate) fn assess_identifiers(p1: &Place, p2: &Place) -> IdentifierAssessment {
    let a = ids_by_scheme(p1);
    let b = ids_by_scheme(p2);
    let mut out = IdentifierAssessment::default();
    for (scheme, values_a) in &a {
        let Some(values_b) = b.get(scheme) else {
            continue;
        };
        out.compared = true;
        if values_a.is_disjoint(values_b) {
            out.contradictions.push(Contradiction {
                field: format!("identifier:{scheme}"),
            });
        } else {
            out.shared = true;
        }
    }
    out
}

/// Assemble [`Evidence`] from the breakdown and the identifier outcome.
///
/// `address_compared` is `false` when the address score is the neutral
/// "no sub-field on both sides" `0.5`, which is absence, not disagreement.
pub(crate) fn assess(
    breakdown: &crate::MatchBreakdown,
    address_compared: bool,
    ids: IdentifierAssessment,
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let mut weight_bits = 0.0_f64;
    let mut fields_compared = 0_u32;
    let address = if address_compared {
        breakdown.address_score
    } else {
        None
    };
    let components = [
        (breakdown.name_score, NAME),
        (breakdown.name_phonetic_score, NAME_PHONETIC),
        (breakdown.coordinates_score, COORDINATES),
        (address, ADDRESS),
        (breakdown.category_score, CATEGORY),
        (breakdown.country_code_score, COUNTRY_CODE),
        (breakdown.phone_score, PHONE),
        (breakdown.email_score, EMAIL),
        (breakdown.local_id_score, LOCAL_ID),
    ];
    for (score, w) in components {
        if let Some(s) = score {
            weight_bits += component_bits(s, w.m, w.u);
            fields_compared += 1;
        }
    }
    // `place_ids_score` is the identifier component: counted once, as
    // IDENTIFIER_BITS on a shared value, never also through the table.
    if ids.compared {
        fields_compared += 1;
    }
    if ids.shared {
        weight_bits += IDENTIFIER_BITS;
    }
    for _ in &ids.contradictions {
        weight_bits += CONTRADICTION_BITS;
    }
    let strength = if !ids.contradictions.is_empty() {
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
        contradictions: ids.contradictions,
        strength,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Address, PlaceCategory, PlaceId};
    use crate::{MatchConfig, MatchResult, MatchingEngine};

    fn engine() -> MatchingEngine {
        MatchingEngine::default_config()
    }

    #[test]
    fn constants_match_their_formulas() {
        assert!((IDENTIFIER_BITS - (0.99_f64 / 1e-12).log2()).abs() < 1e-9);
        assert!((CONTRADICTION_BITS - (0.01_f64 / (1.0 - 1e-12)).log2()).abs() < 1e-9);
        assert!((MatchConfig::default().proof_bits - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn component_bits_interpolates_and_clamps() {
        let w = NAME;
        let agree = (w.m / w.u).log2();
        let disagree = ((1.0 - w.m) / (1.0 - w.u)).log2();
        assert!((component_bits(1.0, w.m, w.u) - agree).abs() < 1e-12);
        assert!((component_bits(0.0, w.m, w.u) - disagree).abs() < 1e-12);
        assert!((component_bits(f64::NAN, w.m, w.u) - disagree).abs() < 1e-12);
        let mid = component_bits(0.775, w.m, w.u);
        assert!(mid > disagree && mid < agree);
    }

    #[test]
    fn missing_component_adds_zero_and_is_not_counted() {
        let a = Place::builder().name("Eiffel Tower").build();
        let b = Place::builder()
            .name("Eiffel Tower")
            .latitude_as_decimal_degrees(48.858)
            .longitude_as_decimal_degrees(2.294)
            .build();
        let only_name = engine().match_places(&a, &a);
        let with_missing = engine().match_places(&a, &b);
        assert_eq!(with_missing.evidence.fields_compared, 1);
        assert!((only_name.evidence.weight_bits - with_missing.evidence.weight_bits).abs() < 1e-12);
    }

    #[test]
    fn empty_addresses_are_absence_not_disagreement() {
        let a = Place::builder()
            .name("Eiffel Tower")
            .address(Address::new())
            .build();
        let r = engine().match_places(&a, &a);
        assert_eq!(r.breakdown.address_score, Some(0.5));
        assert_eq!(r.evidence.fields_compared, 1);
    }

    #[test]
    fn identical_sparse_records_are_consistent_not_proven() {
        let a = Place::builder()
            .name("Eiffel Tower")
            .latitude_as_decimal_degrees(48.858_222)
            .longitude_as_decimal_degrees(2.294_5)
            .build();
        let r = engine().match_places(&a, &a);
        assert!(r.is_match);
        // name 7.57 + coordinates 13.14 = about 20.7 bits, short of 30.
        assert!(r.evidence.weight_bits < 30.0, "{}", r.evidence.weight_bits);
        assert!(r.evidence.weight_bits > 20.0, "{}", r.evidence.weight_bits);
        assert_eq!(r.evidence.strength, MatchStrength::Consistent);
    }

    #[test]
    fn shared_unique_identifier_is_proven() {
        let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
        let a = Place::builder()
            .name("Eiffel Tower")
            .add_place_id(id.clone())
            .build();
        let b = Place::builder()
            .name("Tour Eiffel")
            .add_place_id(id)
            .build();
        let r = engine().match_places(&a, &b);
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
        assert!(r.evidence.weight_bits >= IDENTIFIER_BITS - 10.0);
        assert!(r.evidence.contradictions.is_empty());
    }

    fn rich(builder: crate::models::PlaceBuilder) -> crate::models::PlaceBuilder {
        builder
            .name("Café de Flore")
            .latitude_as_decimal_degrees(48.854_0)
            .longitude_as_decimal_degrees(2.332_6)
            .address(
                Address::new()
                    .with_line1("172 Boulevard Saint-Germain")
                    .with_city("Paris")
                    .with_postcode("75006"),
            )
            .category(PlaceCategory::Cafe)
            .country_code_as_iso_3166_1_alpha_2("FR")
            .phone("+33 1 45 48 55 26")
            .email("contact@example.org")
    }

    #[test]
    fn conflicting_same_scheme_identifiers_are_contradicted() {
        let a = rich(Place::builder())
            .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q1").unwrap())
            .build();
        let b = rich(Place::builder())
            .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q2").unwrap())
            .build();
        let r = engine().match_places(&a, &b);
        assert!(r.is_match);
        assert_eq!(r.evidence.strength, MatchStrength::Contradicted);
        assert_eq!(
            r.evidence.contradictions,
            vec![Contradiction {
                field: "identifier:wikidata".to_string()
            }]
        );
    }

    #[test]
    fn contradiction_wins_over_a_shared_identifier_elsewhere() {
        let shared = PlaceId::new(PlaceIdScheme::GeoNames, "2988507").unwrap();
        let a = Place::builder()
            .name("Paris")
            .add_place_id(shared.clone())
            .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q90").unwrap())
            .build();
        let b = Place::builder()
            .name("Paris")
            .add_place_id(shared)
            .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q91").unwrap())
            .build();
        let r = engine().match_places(&a, &b);
        assert_eq!(r.evidence.strength, MatchStrength::Contradicted);
    }

    #[test]
    fn one_shared_value_among_several_is_not_a_contradiction() {
        let a = Place::builder()
            .name("Paris")
            .add_place_id(PlaceId::new(PlaceIdScheme::OsmNode, "1").unwrap())
            .add_place_id(PlaceId::new(PlaceIdScheme::OsmNode, "2").unwrap())
            .build();
        let b = Place::builder()
            .name("Paris")
            .add_place_id(PlaceId::new(PlaceIdScheme::OsmNode, "2").unwrap())
            .build();
        let r = engine().match_places(&a, &b);
        assert!(r.evidence.contradictions.is_empty());
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    #[test]
    fn different_schemes_neither_prove_nor_contradict() {
        let a = Place::builder()
            .name("Paris")
            .add_place_id(PlaceId::new(PlaceIdScheme::OsmNode, "1").unwrap())
            .build();
        let b = Place::builder()
            .name("Paris")
            .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q90").unwrap())
            .build();
        let r = engine().match_places(&a, &b);
        assert!(r.evidence.contradictions.is_empty());
        assert_eq!(r.evidence.fields_compared, 1);
    }

    #[test]
    fn blank_identifier_values_are_ignored() {
        let blank = PlaceId {
            scheme: PlaceIdScheme::Wikidata,
            value: "  ".to_string(),
        };
        let a = Place::builder().name("Paris").add_place_id(blank).build();
        let b = Place::builder()
            .name("Paris")
            .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q90").unwrap())
            .build();
        let r = engine().match_places(&a, &b);
        assert!(r.evidence.contradictions.is_empty());
    }

    #[test]
    fn richly_identical_record_without_identifier_is_proven() {
        let a = rich(Place::builder()).build();
        let r = engine().match_places(&a, &a);
        // name + coordinates + address + category + country + phone + email
        // = 7.57 + 13.14 + 9.73 + 3.17 + 3.29 + 9.73 + 8.64, about 55.3.
        assert!(r.evidence.weight_bits > 55.0, "{}", r.evidence.weight_bits);
        assert_eq!(r.evidence.fields_compared, 7);
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    #[test]
    fn clear_disagreement_gives_negative_bits() {
        let a = Place::builder()
            .name("Eiffel Tower")
            .country_code_as_iso_3166_1_alpha_2("FR")
            .category(PlaceCategory::Monument)
            .build();
        let b = Place::builder()
            .name("Golden Gate Bridge")
            .country_code_as_iso_3166_1_alpha_2("US")
            .category(PlaceCategory::Park)
            .build();
        let r = engine().match_places(&a, &b);
        assert!(r.evidence.weight_bits < 0.0, "{}", r.evidence.weight_bits);
        assert_eq!(r.evidence.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn json_without_evidence_still_deserialises() {
        let a = Place::builder().name("Eiffel Tower").build();
        let mut value = serde_json::to_value(engine().match_places(&a, &a)).unwrap();
        value.as_object_mut().unwrap().remove("evidence");
        let back: MatchResult = serde_json::from_value(value).unwrap();
        assert_eq!(back.evidence, Evidence::default());
        assert_eq!(back.evidence.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn strength_serialises_snake_case() {
        assert_eq!(
            serde_json::to_string(&MatchStrength::NonMatch).unwrap(),
            "\"non_match\""
        );
    }

    #[test]
    fn proof_bits_is_configurable() {
        let a = rich(Place::builder()).build();
        let cfg = MatchConfig {
            proof_bits: 100.0,
            ..Default::default()
        };
        let r = MatchingEngine::new(cfg).match_places(&a, &a);
        assert_eq!(r.evidence.strength, MatchStrength::Consistent);
    }
}
