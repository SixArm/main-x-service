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
use std::cmp::Ordering;
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

// `name_phonetic_score` has no entry and adds no term: the Soundex code is
// derived from the names, so summing it would count `NAME` twice
// (match-evidence §6). It still feeds `score` as a bonus.

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
    /// Every term of the sum, in a fixed order; they add up to
    /// `weight_bits` exactly. Empty on JSON written before the field.
    #[serde(default)]
    pub components: Vec<ComponentBits>,
}

/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentBits {
    /// The component, e.g. `"name"`, `"identifier"`,
    /// `"contradiction:identifier:wikidata"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}

// ---- Correlated components (match-evidence §6) -------------------------

/// Weight of each member of a correlated group after its strongest.
pub const CORRELATED_DISCOUNT: f64 = 0.5;

/// Groups of components that are not independent given the pair's truth.
///
/// - `coordinates` / `address`: a geocoded address is the same fact twice
///   (and a building's address and its point agree for every tenant).
/// - `address` / `country_code`: the address determines the country.
/// - `phone` / `email`: contact details recorded together, and shared
///   together by every branch of a chain or tenant of a serviced office.
///
/// Groups that share a member are merged before discounting, so
/// `coordinates`, `address` and `country_code` form one group: each
/// component is discounted at most once and appears once in
/// [`Evidence::components`].
pub const CORRELATED_GROUPS: &[&[&str]] = &[
    &["coordinates", "address"],
    &["address", "country_code"],
    &["phone", "email"],
];

/// Index of the member with the largest absolute value, ties to the larger
/// value (then to the earlier index, which gives the same result).
fn strongest_index(member_bits: &[f64]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for (i, &b) in member_bits.iter().enumerate() {
        best = match best {
            None => Some(i),
            Some(j) => {
                let c = member_bits[j];
                if b.abs().total_cmp(&c.abs()).then(b.total_cmp(&c)) == Ordering::Greater {
                    Some(i)
                } else {
                    Some(j)
                }
            }
        };
    }
    best
}

/// The strongest member (by absolute value, ties to the larger value) and
/// the others, or `None` when there are no members.
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

/// [`CORRELATED_GROUPS`] with overlapping groups merged.
fn merged_groups() -> Vec<BTreeSet<&'static str>> {
    let mut merged: Vec<BTreeSet<&'static str>> = Vec::new();
    for group in CORRELATED_GROUPS {
        let mut set: BTreeSet<&'static str> = group.iter().copied().collect();
        let mut i = 0;
        while i < merged.len() {
            if merged[i].is_disjoint(&set) {
                i += 1;
            } else {
                set.extend(merged.swap_remove(i));
                i = 0;
            }
        }
        merged.push(set);
    }
    merged
}

// ---- Common and rare values (match-evidence §7) ------------------------

/// Where the share of records holding a value comes from.
pub trait ValueFrequencies {
    /// The share, in `0.0..=1.0`, of records whose `component` holds
    /// `value` (already in the matcher's normalised form), or `None`
    /// when unknown.
    fn share(&self, component: &str, value: &str) -> Option<f64>;
}

/// No frequencies: every share is unknown, so the plain table applies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoFrequencies;

impl ValueFrequencies for NoFrequencies {
    fn share(&self, _component: &str, _value: &str) -> Option<f64> {
        None
    }
}

/// A [`ValueFrequencies`] built from counts.
///
/// Keys are `(component, value)` with `value` in the form
/// [`frequency_key`] returns.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrequencyTable {
    shares: BTreeMap<(String, String), f64>,
}

impl FrequencyTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The share `count / total`, or `None` when `total` is zero.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn from_counts(count: u64, total: u64) -> Option<f64> {
        if total == 0 {
            None
        } else {
            Some(count as f64 / total as f64)
        }
    }

    /// Record the share of records whose `component` holds `value`
    /// (already normalised, see [`frequency_key`]).
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

/// Components whose agreement weight adjusts to the value's frequency.
///
/// `name`: chain and generic names ("Starbucks", "Main Street Surgery",
/// "St Mary's Church") are held by many places; a rare name is strong.
/// Each of a place's names (primary and alternates) is a value.
pub const FREQUENCY_COMPONENTS: &[&str] = &["name"];

/// The key the matcher looks up for `raw` under `component`: the same
/// normalised form it compares. `None` for a component not in
/// [`FREQUENCY_COMPONENTS`] or a value that normalises to empty.
///
/// ```
/// use place_matcher::evidence::frequency_key;
/// assert_eq!(frequency_key("name", "  Café  de Flore! ").as_deref(), Some("cafe de flore"));
/// assert_eq!(frequency_key("name", "..."), None);
/// assert_eq!(frequency_key("phone", "+44 20 7946 0000"), None);
/// ```
#[must_use]
pub fn frequency_key(component: &str, raw: &str) -> Option<String> {
    match component {
        "name" => {
            let key = crate::Normalizer::normalize_name(raw);
            (!key.is_empty()).then_some(key)
        }
        _ => None,
    }
}

/// Normalised names held by both places, the values an exact name
/// agreement can rest on.
fn shared_name_keys(p1: &Place, p2: &Place) -> BTreeSet<String> {
    let keys = |p: &Place| -> BTreeSet<String> {
        p.name
            .iter()
            .chain(p.alternate_names.iter())
            .filter_map(|n| frequency_key("name", n))
            .collect()
    };
    let a = keys(p1);
    let b = keys(p2);
    a.intersection(&b).cloned().collect()
}

/// The share to use for an exact name agreement: the **largest** known
/// share among the shared names (the conservative choice when several
/// coincide), or `None` when none is known.
pub(crate) fn name_share(p1: &Place, p2: &Place, freqs: &dyn ValueFrequencies) -> Option<f64> {
    shared_name_keys(p1, p2)
        .iter()
        .filter_map(|k| freqs.share("name", k))
        .filter(|s| s.is_finite())
        .fold(None, |acc: Option<f64>, s| {
            Some(acc.map_or(s, |a| a.max(s)))
        })
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
/// `name_share` is the frequency of the shared name, when known.
pub(crate) fn assess(
    breakdown: &crate::MatchBreakdown,
    address_compared: bool,
    ids: IdentifierAssessment,
    name_share: Option<f64>,
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let address = if address_compared {
        breakdown.address_score
    } else {
        None
    };
    let table = [
        ("name", breakdown.name_score, NAME),
        ("coordinates", breakdown.coordinates_score, COORDINATES),
        ("address", address, ADDRESS),
        ("category", breakdown.category_score, CATEGORY),
        ("country_code", breakdown.country_code_score, COUNTRY_CODE),
        ("phone", breakdown.phone_score, PHONE),
        ("email", breakdown.email_score, EMAIL),
        ("local_id", breakdown.local_id_score, LOCAL_ID),
    ];
    let mut components: Vec<ComponentBits> = Vec::new();
    for (name, score, w) in table {
        let Some(s) = score else { continue };
        // §7: only an exact agreement on a frequency component adjusts.
        let adjusted = if FREQUENCY_COMPONENTS.contains(&name) && s >= AGREE_AT {
            if name == "name" { name_share } else { None }
        } else {
            None
        };
        let (bits, frequency_adjusted) = match adjusted {
            Some(share) => (frequency_bits(w.m, share), true),
            None => (component_bits(s, w.m, w.u), false),
        };
        components.push(ComponentBits {
            component: name.to_string(),
            bits,
            correlated: false,
            frequency_adjusted,
        });
    }
    // §6: within each (merged) group the strongest counts in full and the
    // rest at CORRELATED_DISCOUNT.
    for group in merged_groups() {
        let members: Vec<usize> = components
            .iter()
            .enumerate()
            .filter(|(_, c)| group.contains(c.component.as_str()))
            .map(|(i, _)| i)
            .collect();
        let bits: Vec<f64> = members.iter().map(|&i| components[i].bits).collect();
        if let Some(k) = strongest_index(&bits) {
            for (pos, &i) in members.iter().enumerate() {
                if pos != k {
                    components[i].bits *= CORRELATED_DISCOUNT;
                    components[i].correlated = true;
                }
            }
        }
    }
    let mut fields_compared = u32::try_from(components.len()).unwrap_or(u32::MAX);
    // `place_ids_score` is the identifier component: counted once, as
    // IDENTIFIER_BITS on a shared value, never also through the table.
    if ids.compared {
        fields_compared += 1;
    }
    if ids.shared {
        components.push(ComponentBits {
            component: "identifier".to_string(),
            bits: IDENTIFIER_BITS,
            correlated: false,
            frequency_adjusted: false,
        });
    }
    for c in &ids.contradictions {
        components.push(ComponentBits {
            component: format!("contradiction:{}", c.field),
            bits: CONTRADICTION_BITS,
            correlated: false,
            frequency_adjusted: false,
        });
    }
    let weight_bits: f64 = components.iter().map(|c| c.bits).sum();
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
        components,
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
        // name 7.57 + {coordinates 13.14, address 9.73/2, country 3.29/2}
        // + category 3.17 + {phone 9.73, email 8.64/2}, about 44.4 bits
        // (55.3 before the correlated-group discount).
        let w = r.evidence.weight_bits;
        assert!(w > 44.0 && w < 45.0, "{w}");
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

    fn term<'a>(e: &'a Evidence, name: &str) -> &'a ComponentBits {
        e.components
            .iter()
            .find(|c| c.component == name)
            .unwrap_or_else(|| panic!("no term {name}: {:?}", e.components))
    }

    fn terms_sum(e: &Evidence) -> f64 {
        e.components.iter().map(|c| c.bits).sum()
    }

    // ---------- correlated groups (§6) ----------

    #[test]
    fn phonetic_score_is_derived_and_adds_no_term() {
        let a = Place::builder().name("Smith Hall").build();
        let b = Place::builder().name("Smyth Hall").build();
        let on = MatchingEngine::new(MatchConfig::lenient()).match_places(&a, &b);
        let off = MatchingEngine::new(MatchConfig {
            use_phonetic_matching: false,
            ..MatchConfig::lenient()
        })
        .match_places(&a, &b);
        assert!(on.breakdown.name_phonetic_score.is_some());
        assert!(
            on.evidence
                .components
                .iter()
                .all(|c| c.component != "name_phonetic")
        );
        assert_eq!(on.evidence, off.evidence);
        assert_eq!(on.evidence.fields_compared, 1);
    }

    #[test]
    fn group_bits_is_less_than_the_sum_and_order_independent() {
        let a = component_bits(1.0, COORDINATES.m, COORDINATES.u);
        let b = component_bits(1.0, ADDRESS.m, ADDRESS.u);
        let g = group_bits(&[a, b]);
        assert!(g < a + b);
        assert!((g - (a + 0.5 * b)).abs() < 1e-12);
        assert!((group_bits(&[b, a]) - g).abs() < 1e-12);
        let c = 3.0;
        assert!((group_bits(&[a, b, c]) - group_bits(&[c, b, a])).abs() < 1e-12);
        assert!(group_bits(&[]).abs() < f64::EPSILON);
    }

    #[test]
    fn strongest_first_orders_by_absolute_value_ties_to_larger() {
        assert_eq!(
            strongest_first(&[2.0, -5.0, 1.0]),
            Some((-5.0, vec![2.0, 1.0]))
        );
        assert_eq!(strongest_first(&[-3.0, 3.0]), Some((3.0, vec![-3.0])));
        assert_eq!(strongest_first(&[3.0, -3.0]), Some((3.0, vec![-3.0])));
        assert_eq!(strongest_first(&[]), None);
    }

    #[test]
    fn overlapping_groups_merge() {
        let merged = merged_groups();
        assert!(merged.iter().any(|g| g.len() == 3
            && g.contains("coordinates")
            && g.contains("address")
            && g.contains("country_code")));
        // Every component appears in at most one merged group.
        let total: usize = merged.iter().map(BTreeSet::len).sum();
        let distinct: BTreeSet<_> = merged.iter().flatten().collect();
        assert_eq!(total, distinct.len());
    }

    #[test]
    fn agreeing_correlated_fields_are_discounted_in_the_match() {
        let a = Place::builder()
            .latitude_as_decimal_degrees(51.5)
            .longitude_as_decimal_degrees(-0.12)
            .address(Address::new().with_postcode("SW1A 1AA"))
            .build();
        let e = engine().match_places(&a, &a).evidence;
        let coords = component_bits(1.0, COORDINATES.m, COORDINATES.u);
        let addr = component_bits(1.0, ADDRESS.m, ADDRESS.u);
        assert!((e.weight_bits - (coords + 0.5 * addr)).abs() < 1e-9);
        assert!(!term(&e, "coordinates").correlated);
        assert!(term(&e, "address").correlated);
    }

    #[test]
    fn uncorrelated_fields_still_sum() {
        let a = Place::builder()
            .name("Eiffel Tower")
            .category(PlaceCategory::Monument)
            .build();
        let e = engine().match_places(&a, &a).evidence;
        let expected =
            component_bits(1.0, NAME.m, NAME.u) + component_bits(1.0, CATEGORY.m, CATEGORY.u);
        assert!((e.weight_bits - expected).abs() < 1e-12);
        assert!(e.components.iter().all(|c| !c.correlated));
    }

    #[test]
    fn correlated_disagreements_are_discounted_too() {
        let a = Place::builder()
            .phone("+44 20 7946 0000")
            .email("a@example.org")
            .build();
        let b = Place::builder()
            .phone("+44 20 7946 0999")
            .email("b@example.org")
            .build();
        let e = engine().match_places(&a, &b).evidence;
        let phone = component_bits(0.0, PHONE.m, PHONE.u);
        let email = component_bits(0.0, EMAIL.m, EMAIL.u);
        // phone (-2.74) is the stronger; email (-2.32) counts at half.
        assert!((e.weight_bits - (phone + 0.5 * email)).abs() < 1e-12);
        assert!(e.weight_bits > phone + email);
        assert!(term(&e, "email").correlated);
    }

    #[test]
    fn two_businesses_in_one_building_are_not_proven() {
        let building = |name: &str| {
            Place::builder()
                .name(name)
                .latitude_as_decimal_degrees(51.507_4)
                .longitude_as_decimal_degrees(-0.127_8)
                .address(
                    Address::new()
                        .with_line1("1 Example Street")
                        .with_city("London")
                        .with_postcode("WC2N 5DU"),
                )
                .country_code_as_iso_3166_1_alpha_2("GB")
                .build()
        };
        let a = building("Harbour Dental Practice");
        let b = building("Quill and Ink Solicitors");
        let e = engine().match_places(&a, &b).evidence;
        // name -4.31 + {coordinates 13.14, address 9.73/2, country 3.29/2}
        // = about 15.3 bits; it would be 21.85 summed independently.
        assert!(e.weight_bits < 20.0, "{}", e.weight_bits);
        assert_ne!(e.strength, MatchStrength::Proven);
    }

    // ---------- value frequencies (§7) ----------

    fn freqs(name: &str, share: f64) -> FrequencyTable {
        let mut t = FrequencyTable::new();
        t.insert("name", frequency_key("name", name).unwrap(), share);
        t
    }

    #[test]
    fn common_name_gives_fewer_bits_and_rare_name_more() {
        let default_bits = component_bits(1.0, NAME.m, NAME.u);
        assert!(frequency_bits(NAME.m, 0.01) < default_bits);
        assert!(frequency_bits(NAME.m, 1e-6) > default_bits);
        let a = Place::builder().name("Starbucks").build();
        let e = engine()
            .match_places_with_frequencies(&a, &a, &freqs("Starbucks", 0.01))
            .evidence;
        let t = term(&e, "name");
        assert!(t.frequency_adjusted);
        assert!((t.bits - (0.95_f64 / 0.01).log2()).abs() < 1e-12);
    }

    #[test]
    fn share_at_or_above_m_gives_zero_never_negative() {
        assert!(frequency_bits(NAME.m, NAME.m).abs() < 1e-12);
        assert!(frequency_bits(NAME.m, 1.0).abs() < 1e-12);
        assert!(frequency_bits(NAME.m, f64::NAN).abs() < 1e-12);
        assert!((frequency_bits(NAME.m, 0.0) - (NAME.m / MIN_SHARE).log2()).abs() < 1e-9);
    }

    #[test]
    fn no_frequency_adjustment_on_partial_agreement_or_disagreement() {
        let query = Place::builder().name("Starbucks Coffee").build();
        let typo = Place::builder().name("Starbucks Cofee").build();
        let partial = Place::builder().name("Starbucks Kitchen").build();
        let other_name = Place::builder().name("Big Ben").build();
        let table = freqs("Starbucks Coffee", 0.5);
        // `typo` is near-exact (its similarity can clear AGREE_AT), `partial`
        // a partial agreement (about 0.82), `other_name` a disagreement:
        // none holds the same
        // normalised value, so none is adjusted.
        let partial_score = engine()
            .match_places(&query, &partial)
            .breakdown
            .name_score
            .unwrap();
        assert!(partial_score > DISAGREE_AT && partial_score < AGREE_AT);
        assert!(
            engine()
                .match_places(&query, &other_name)
                .breakdown
                .name_score
                .unwrap()
                <= DISAGREE_AT
        );
        for other in [&typo, &partial, &other_name] {
            let plain = engine().match_places(&query, other);
            let adjusted = engine().match_places_with_frequencies(&query, other, &table);
            assert_eq!(plain.evidence, adjusted.evidence);
            assert!(!term(&adjusted.evidence, "name").frequency_adjusted);
        }
    }

    #[test]
    fn no_frequencies_equals_match_places() {
        let a = rich(Place::builder()).build();
        let b = rich(Place::builder()).name("Cafe de Flore Paris").build();
        let plain = engine().match_places(&a, &b);
        let none = engine().match_places_with_frequencies(&a, &b, &NoFrequencies);
        assert_eq!(plain.evidence, none.evidence);
        assert_eq!(plain.score.to_bits(), none.score.to_bits());
        assert_eq!(plain.is_match, none.is_match);
        assert_eq!(plain.confidence, none.confidence);
    }

    #[test]
    fn chain_branches_nearby_are_not_proven_on_the_name() {
        // Two branches of one chain, about 25 m apart.
        let branch = |lat: f64| {
            Place::builder()
                .name("Starbucks")
                .latitude_as_decimal_degrees(lat)
                .longitude_as_decimal_degrees(-0.127_8)
                .category(PlaceCategory::Cafe)
                .country_code_as_iso_3166_1_alpha_2("GB")
                .build()
        };
        let a = branch(51.507_400);
        let b = branch(51.507_625);
        let f = freqs("Starbucks", 0.02);
        let plain = engine().match_places(&a, &b);
        let adjusted = engine().match_places_with_frequencies(&a, &b, &f);
        assert_eq!(plain.score.to_bits(), adjusted.score.to_bits());
        assert!(term(&adjusted.evidence, "name").bits < term(&plain.evidence, "name").bits);
        assert!(adjusted.evidence.weight_bits < plain.evidence.weight_bits);
        assert_ne!(plain.evidence.strength, MatchStrength::Proven);
        assert_ne!(adjusted.evidence.strength, MatchStrength::Proven);
    }

    #[test]
    fn frequency_table_from_counts() {
        assert_eq!(FrequencyTable::from_counts(1, 4), Some(0.25));
        assert_eq!(FrequencyTable::from_counts(1, 0), None);
        let t = freqs("Starbucks", 0.25);
        assert_eq!(t.share("name", "starbucks"), Some(0.25));
        assert_eq!(t.share("name", "Starbucks"), None);
        assert_eq!(NoFrequencies.share("name", "starbucks"), None);
    }

    #[test]
    fn alternate_name_agreement_uses_largest_known_share() {
        let a = Place::builder()
            .name("Tour Eiffel")
            .add_alternate_name("Eiffel Tower")
            .build();
        let b = Place::builder()
            .name("Eiffel Tower")
            .add_alternate_name("Tour Eiffel")
            .build();
        let mut t = freqs("Eiffel Tower", 1e-6);
        t.insert("name", "tour eiffel", 1e-3);
        let e = engine().match_places_with_frequencies(&a, &b, &t).evidence;
        assert!((term(&e, "name").bits - (0.95_f64 / 1e-3).log2()).abs() < 1e-9);
    }

    // ---------- explainability (§8) ----------

    #[test]
    fn terms_sum_to_weight_bits_including_identifiers_and_contradictions() {
        let a = rich(Place::builder())
            .add_place_id(PlaceId::new(PlaceIdScheme::GeoNames, "1").unwrap())
            .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q1").unwrap())
            .build();
        let b = rich(Place::builder())
            .add_place_id(PlaceId::new(PlaceIdScheme::GeoNames, "1").unwrap())
            .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q2").unwrap())
            .build();
        let e = engine().match_places(&a, &b).evidence;
        assert!((term(&e, "identifier").bits - IDENTIFIER_BITS).abs() < f64::EPSILON);
        assert!(
            (term(&e, "contradiction:identifier:wikidata").bits - CONTRADICTION_BITS).abs()
                < f64::EPSILON
        );
        assert_eq!(terms_sum(&e).to_bits(), e.weight_bits.to_bits());
    }

    #[test]
    fn json_without_components_still_deserialises() {
        let a = Place::builder().name("Eiffel Tower").build();
        let mut value = serde_json::to_value(engine().match_places(&a, &a).evidence).unwrap();
        value.as_object_mut().unwrap().remove("components");
        let back: Evidence = serde_json::from_value(value).unwrap();
        assert!(back.components.is_empty());
    }
}
