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
//! - **Correlated groups** ([`CORRELATED_GROUPS`]): `image` with
//!   `main_entity_of_page`, and `description` with
//!   `disambiguating_description`; non-strongest members count at
//!   [`CORRELATED_DISCOUNT`]. `name_phonetic_score` is not summed.
//! - **Frequencies** ([`FREQUENCY_COMPONENTS`]): an exact shared name
//!   uses its own share via [`frequency_bits`], when a
//!   [`ValueFrequencies`] source knows it.
//! - **Listing.** [`Evidence::components`] holds every term, identity and
//!   contradiction terms included; [`Evidence::weight_bits`] is their sum.

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
/// Soundex on names: `u` — 4-character codes collide often. **Not
/// summed**: the code is derived from the name, so it would count the
/// name twice (match-evidence.md §6). Kept for the table's completeness.
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
    /// Sum of the components' log-likelihood weights, in bits. Always the
    /// sum of [`Self::components`]' `bits`, in list order.
    pub weight_bits: f64,
    /// Components both records held (identifiers count as one).
    pub fields_compared: u32,
    /// Hard conflicts; any one makes the strength `Contradicted`.
    pub contradictions: Vec<Contradiction>,
    /// The verdict (§4).
    pub strength: MatchStrength,
    /// Every term of the sum (match-evidence.md §8). Empty on JSON that
    /// pre-dates the field.
    #[serde(default)]
    pub components: Vec<ComponentBits>,
}

/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentBits {
    /// The component, e.g. `"name"`, `"identifier"`,
    /// `"contradiction:identifier:isbn"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}

// ---- Correlated components (match-evidence.md §6) ----------------------

/// Weight of each member of a correlated group after its strongest.
pub const CORRELATED_DISCOUNT: f64 = 0.5;

/// This crate's correlated groups, by component name. Within a group the
/// member with the largest absolute weight counts in full and every other
/// present member at [`CORRELATED_DISCOUNT`].
///
/// - `image` + `main_entity_of_page`: both are scraped from the same web
///   page, so two records harvested from one page (one catalogue listing
///   several products) agree on both for one reason.
/// - `description` + `disambiguating_description`: both are free text
///   written by one source, often the second abridged from the first, so
///   a copied blurb agrees on both for one reason.
///
/// `name_phonetic` is not in a group: it is derived from the name and is
/// left out of the sum entirely (§6).
pub const CORRELATED_GROUPS: &[&[&str]] = &[
    &["image", "main_entity_of_page"],
    &["description", "disambiguating_description"],
];

/// Index of the strongest member: largest absolute value, ties to the
/// larger value, exact ties to the earliest. `None` when empty.
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

/// Split a group's present members into its strongest (by absolute
/// value, ties to the larger value) and the rest. `None` when empty.
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

// ---- Common and rare values (match-evidence.md §7) ---------------------

/// Where the share of records holding a value comes from.
pub trait ValueFrequencies {
    /// The share, in `0.0..=1.0`, of records whose `component` holds
    /// `value` (already in the matcher's normalised form, see
    /// [`frequency_key`]), or `None` when unknown.
    fn share(&self, component: &str, value: &str) -> Option<f64>;
}

/// No frequencies: every share is unknown, so the plain `m`/`u` table
/// applies. What [`crate::MatchingEngine::match_things`] uses.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFrequencies;

impl ValueFrequencies for NoFrequencies {
    fn share(&self, _component: &str, _value: &str) -> Option<f64> {
        None
    }
}

/// Shares held in memory, keyed by component and [`frequency_key`].
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

    /// Build from `(component, key, count)` rows over `total` records:
    /// each share is `count / total`. A `total` of `0` gives an empty
    /// table. Keys should come from [`frequency_key`].
    #[must_use]
    pub fn from_counts<C, K, I>(counts: I, total: u64) -> Self
    where
        C: Into<String>,
        K: Into<String>,
        I: IntoIterator<Item = (C, K, u64)>,
    {
        let mut table = Self::new();
        if total == 0 {
            return table;
        }
        #[allow(clippy::cast_precision_loss)]
        let total = total as f64;
        for (component, key, count) in counts {
            #[allow(clippy::cast_precision_loss)]
            let share = count as f64 / total;
            table.insert(component, key, share);
        }
        table
    }

    /// Set one share. The key should come from [`frequency_key`].
    pub fn insert(&mut self, component: impl Into<String>, key: impl Into<String>, share: f64) {
        self.shares.insert((component.into(), key.into()), share);
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

/// The components whose exact agreement is frequency-adjusted. Public
/// contract: a service counts values under these names.
///
/// - `name`: the primary `name` and every `alternateName`, each keyed by
///   [`Normalizer::normalize_name`]. Generic names ("USB Cable") are held
///   by many unrelated products.
pub const FREQUENCY_COMPONENTS: &[&str] = &["name"];

/// The key the matcher looks up for `raw` under `component`: the same
/// normalised form it compares. `None` for a component not in
/// [`FREQUENCY_COMPONENTS`] or a value that normalises to empty.
///
/// ```
/// use thing_matcher::evidence::frequency_key;
/// assert_eq!(frequency_key("name", "  USB Cable ").as_deref(), Some("usb cable"));
/// assert_eq!(frequency_key("description", "x"), None);
/// assert_eq!(frequency_key("name", "   "), None);
/// ```
#[must_use]
pub fn frequency_key(component: &str, raw: &str) -> Option<String> {
    match component {
        "name" => {
            let key = Normalizer::normalize_name(raw);
            (!key.is_empty()).then_some(key)
        }
        _ => None,
    }
}

// ---- Computation -------------------------------------------------------

/// One raw term before correlation discounts.
struct Term {
    component: String,
    bits: f64,
    frequency_adjusted: bool,
}

/// Running list of terms and compared fields.
#[derive(Default)]
struct Tally {
    terms: Vec<Term>,
    fields: u32,
}

impl Tally {
    fn push(&mut self, component: impl Into<String>, bits: f64, frequency_adjusted: bool) {
        self.terms.push(Term {
            component: component.into(),
            bits,
            frequency_adjusted,
        });
    }

    fn add(&mut self, component: &str, similarity: Option<f64>, (m, u): (f64, f64)) {
        if let Some(s) = similarity {
            self.push(component, component_bits(s, m, u), false);
            self.fields += 1;
        }
    }
}

/// Every normalised, non-empty name a thing holds (primary + alternates).
fn name_keys(thing: &Thing) -> BTreeSet<String> {
    thing
        .name
        .iter()
        .chain(thing.alternate_names.iter())
        .filter_map(|n| frequency_key("name", n))
        .collect()
}

/// Name bits: frequency-adjusted on an exact shared normalised name,
/// the plain table otherwise. With several shared names the strongest
/// one counts (a rare shared alternate name is real evidence even when
/// the primary is generic).
fn name_term(
    similarity: f64,
    thing1: &Thing,
    thing2: &Thing,
    frequencies: &dyn ValueFrequencies,
) -> (f64, bool) {
    let (m, u) = NAME;
    let shared: Vec<String> = name_keys(thing1)
        .intersection(&name_keys(thing2))
        .cloned()
        .collect();
    if shared.is_empty() {
        return (component_bits(similarity, m, u), false);
    }
    let plain = component_bits(1.0, m, u);
    let mut best: Option<(f64, bool)> = None;
    for key in &shared {
        let candidate = match frequencies.share("name", key) {
            Some(share) => (frequency_bits(m, share), true),
            None => (plain, false),
        };
        if best.is_none_or(|(b, _)| candidate.0 > b) {
            best = Some(candidate);
        }
    }
    best.unwrap_or((plain, false))
}

/// Compute the evidence for one compared pair.
///
/// `breakdown` is the pair's [`MatchBreakdown`]; the things are passed
/// too because the identity group, the set components and the name's
/// frequency adjustment need the raw values (the similarity breakdown
/// scores an empty-vs-non-empty `sameAs` or `additionalType` set as
/// `0.0`, which evidence must not count).
#[must_use]
pub(crate) fn compute(
    breakdown: &MatchBreakdown,
    thing1: &Thing,
    thing2: &Thing,
    is_match: bool,
    proof_bits: f64,
    frequencies: &dyn ValueFrequencies,
) -> Evidence {
    let mut tally = Tally::default();

    // Plain components: `None` in the breakdown means absent on a side.
    // `name_phonetic_score` is derived from the name and not summed (§6).
    if let Some(s) = breakdown.name_score {
        let (bits, adjusted) = name_term(s, thing1, thing2, frequencies);
        tally.push("name", bits, adjusted);
        tally.fields += 1;
    }
    tally.add("description", breakdown.description_score, DESCRIPTION);
    tally.add(
        "disambiguating_description",
        breakdown.disambiguating_description_score,
        DISAMBIGUATING_DESCRIPTION,
    );
    tally.add(
        "image",
        present_url_pair(thing1.image.as_ref(), thing2.image.as_ref()).and(breakdown.image_score),
        IMAGE,
    );
    tally.add(
        "main_entity_of_page",
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
        "additional_types",
        breakdown.additional_types_score.filter(|_| both_types),
        ADDITIONAL_TYPES,
    );
    tally.add(
        "relationships",
        breakdown.relationships_score,
        RELATIONSHIPS,
    );
    tally.add("tags", breakdown.tags_score, TAGS);

    // Identity group: identifiers, canonical url, sameAs.
    let mut identity = false;
    let contradictions = identifier_contradictions(thing1, thing2);
    let shared_identifier = shares_identifier(thing1, thing2);
    identity |= shared_identifier;
    if shared_identifier || !contradictions.is_empty() {
        // Identifiers count as one field however many schemes compared.
        tally.fields += 1;
    }

    if let Some((a, b)) = normalized_url_pair(thing1.url.as_ref(), thing2.url.as_ref()) {
        if a == b {
            identity = true;
        } else {
            tally.add("url", Some(0.0), URL);
        }
    }

    let same_as1 = url_set(&thing1.same_as);
    let same_as2 = url_set(&thing2.same_as);
    if !same_as1.is_empty() && !same_as2.is_empty() {
        if same_as1.is_disjoint(&same_as2) {
            tally.add("same_as", Some(0.0), SAME_AS);
        } else {
            identity = true;
        }
    }

    if identity {
        tally.push("identifier", IDENTIFIER_BITS, false);
        // The shared identifier already counted itself above.
        if !shared_identifier && contradictions.is_empty() {
            tally.fields += 1;
        }
    }
    for c in &contradictions {
        tally.push(
            format!("contradiction:{}", c.field),
            CONTRADICTION_BITS,
            false,
        );
    }

    let components = apply_groups(tally.terms);
    let weight_bits = components.iter().map(|c| c.bits).sum::<f64>();

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
        fields_compared: tally.fields,
        contradictions,
        strength,
        components,
    }
}

/// Discount every non-strongest present member of each correlated group.
fn apply_groups(terms: Vec<Term>) -> Vec<ComponentBits> {
    let mut out: Vec<ComponentBits> = terms
        .into_iter()
        .map(|t| ComponentBits {
            component: t.component,
            bits: t.bits,
            correlated: false,
            frequency_adjusted: t.frequency_adjusted,
        })
        .collect();
    for group in CORRELATED_GROUPS {
        let members: Vec<usize> = out
            .iter()
            .enumerate()
            .filter(|(_, c)| group.contains(&c.component.as_str()))
            .map(|(i, _)| i)
            .collect();
        let bits: Vec<f64> = members.iter().map(|&i| out[i].bits).collect();
        let Some(strongest) = strongest_index(&bits) else {
            continue;
        };
        for (k, &i) in members.iter().enumerate() {
            if k != strongest {
                out[i].bits *= CORRELATED_DISCOUNT;
                out[i].correlated = true;
            }
        }
    }
    out
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

    // ---- Phase 2: correlated groups, frequencies, per-term listing ----

    fn bits_of(e: &Evidence, component: &str) -> f64 {
        e.components
            .iter()
            .find(|c| c.component == component)
            .map_or(0.0, |c| c.bits)
    }

    fn components_sum(e: &Evidence) -> f64 {
        e.components.iter().map(|c| c.bits).sum()
    }

    #[test]
    fn correlated_agreement_gives_less_than_the_sum_in_any_order() {
        let img = component_bits(1.0, IMAGE.0, IMAGE.1);
        let mep = component_bits(1.0, MAIN_ENTITY_OF_PAGE.0, MAIN_ENTITY_OF_PAGE.1);
        let ab = group_bits(&[img, mep]);
        let ba = group_bits(&[mep, img]);
        assert!((ab - ba).abs() < 1e-12);
        assert!(ab < img + mep);
        assert!((ab - (mep + CORRELATED_DISCOUNT * img)).abs() < 1e-12);

        let a = Thing::builder()
            .image("https://example.org/p.jpg")
            .main_entity_of_page("https://example.org/p")
            .build();
        let e = engine().match_things(&a, &a.clone()).evidence;
        assert!((e.weight_bits - ab).abs() < 1e-9);
        let image = e
            .components
            .iter()
            .find(|c| c.component == "image")
            .unwrap();
        assert!(image.correlated);
        let page = e
            .components
            .iter()
            .find(|c| c.component == "main_entity_of_page")
            .unwrap();
        assert!(!page.correlated);
    }

    #[test]
    fn strongest_first_orders_by_absolute_value_ties_to_larger() {
        assert_eq!(strongest_first(&[]), None);
        let (s, rest) = strongest_first(&[1.0, -3.0, 2.0]).unwrap();
        assert!((s + 3.0).abs() < 1e-12);
        assert_eq!(rest, vec![1.0, 2.0]);
        let (s, _) = strongest_first(&[-2.0, 2.0]).unwrap();
        assert!((s - 2.0).abs() < 1e-12);
        let (s, _) = strongest_first(&[2.0, -2.0]).unwrap();
        assert!((s - 2.0).abs() < 1e-12);
        assert!(group_bits(&[]).abs() < 1e-12);
    }

    #[test]
    fn uncorrelated_fields_still_sum() {
        let a = Thing::builder()
            .name("Eiffel Tower")
            .add_tag("landmark")
            .build();
        let e = engine().match_things(&a, &a.clone()).evidence;
        let expected = component_bits(1.0, NAME.0, NAME.1) + component_bits(1.0, TAGS.0, TAGS.1);
        assert!((e.weight_bits - expected).abs() < 1e-9);
        assert!(e.components.iter().all(|c| !c.correlated));
    }

    #[test]
    fn correlated_disagreements_are_discounted_too() {
        let a = Thing::builder()
            .image("https://example.org/a.jpg")
            .main_entity_of_page("https://example.org/a")
            .build();
        let b = Thing::builder()
            .image("https://example.org/b.jpg")
            .main_entity_of_page("https://example.org/b")
            .build();
        let e = engine().match_things(&a, &b).evidence;
        let img = component_bits(0.0, IMAGE.0, IMAGE.1);
        let mep = component_bits(0.0, MAIN_ENTITY_OF_PAGE.0, MAIN_ENTITY_OF_PAGE.1);
        assert!(e.weight_bits < 0.0);
        assert!(e.weight_bits > img + mep);
        assert!((e.weight_bits - group_bits(&[img, mep])).abs() < 1e-9);
    }

    #[test]
    fn same_source_page_with_different_names_is_not_proven() {
        // Two different products scraped from one catalogue page: the
        // page and its hero image are shared, the names are not.
        let page = || {
            Thing::builder()
                .image("https://shop.example.com/cables/hero.jpg")
                .main_entity_of_page("https://shop.example.com/cables")
                .add_additional_type("https://schema.org/Product")
        };
        let a = page().name("Braided Lightning Cable").build();
        let b = page().name("Ethernet Patch Lead").build();
        let e = engine().match_things(&a, &b).evidence;
        assert!(e.weight_bits < DEFAULT_PROOF_BITS, "bits {}", e.weight_bits);
        assert_ne!(e.strength, MatchStrength::Proven);
        assert!(bits_of(&e, "name") < 0.0);
    }

    #[test]
    fn generic_name_and_description_are_not_proven() {
        let product = || {
            Thing::builder()
                .name("USB Cable")
                .description("A USB cable for charging and data")
                .add_additional_type("https://schema.org/Product")
                .add_tag("cable")
        };
        let (a, b) = (product().build(), product().build());
        let mut table = FrequencyTable::new();
        table.insert("name", frequency_key("name", "USB Cable").unwrap(), 0.01);
        let plain = engine().match_things(&a, &b).evidence;
        let e = engine()
            .match_things_with_frequencies(&a, &b, &table)
            .evidence;
        assert!(e.weight_bits < plain.weight_bits);
        assert_ne!(e.strength, MatchStrength::Proven);
        let name = e.components.iter().find(|c| c.component == "name").unwrap();
        assert!(name.frequency_adjusted);
    }

    #[test]
    fn common_value_gives_fewer_bits_and_rare_value_more() {
        let (m, u) = NAME;
        let default = component_bits(1.0, m, u);
        assert!(frequency_bits(m, 0.01) < default);
        assert!(frequency_bits(m, 1e-6) > default);

        let a = Thing::builder().name("USB Cable").build();
        let key = frequency_key("name", "USB Cable").unwrap();
        let mut common = FrequencyTable::new();
        common.insert("name", key.clone(), 0.01);
        let mut rare = FrequencyTable::new();
        rare.insert("name", key, 1e-6);
        let e_common = engine()
            .match_things_with_frequencies(&a, &a, &common)
            .evidence;
        let e_rare = engine()
            .match_things_with_frequencies(&a, &a, &rare)
            .evidence;
        assert!(e_common.weight_bits < default);
        assert!(e_rare.weight_bits > default);
    }

    #[test]
    fn share_at_or_above_m_gives_zero_never_negative() {
        let (m, _) = NAME;
        assert!(frequency_bits(m, m).abs() < 1e-12);
        assert!(frequency_bits(m, 1.0).abs() < 1e-12);
        assert!(frequency_bits(m, f64::NAN).abs() < 1e-12);
        assert!(frequency_bits(m, 0.0) <= (m / MIN_SHARE).log2() + 1e-9);
    }

    #[test]
    fn no_frequency_adjustment_on_partial_agreement_or_disagreement() {
        let mut table = FrequencyTable::new();
        for raw in ["Eiffel Tower", "Eiffel Towers", "Big Ben"] {
            table.insert("name", frequency_key("name", raw).unwrap(), 1e-6);
        }
        let a = Thing::builder().name("Eiffel Tower").build();
        for other in ["Eiffel Towers", "Big Ben"] {
            let b = Thing::builder().name(other).build();
            let plain = engine().match_things(&a, &b).evidence;
            let e = engine()
                .match_things_with_frequencies(&a, &b, &table)
                .evidence;
            assert_eq!(e, plain, "{other}");
            assert!(e.components.iter().all(|c| !c.frequency_adjusted));
        }
    }

    #[test]
    fn no_frequencies_equals_match_things() {
        let a = Thing::builder()
            .name("Eiffel Tower")
            .description("Lattice tower")
            .image("https://example.org/e.jpg")
            .add_identifier(id("wikidata", "Q243"))
            .build();
        let b = Thing::builder()
            .name("Tour Eiffel")
            .description("Lattice tower in Paris")
            .add_identifier(id("wikidata", "Q9"))
            .build();
        for (x, y) in [(&a, &b), (&a, &a)] {
            let plain = engine().match_things(x, y);
            let none = engine().match_things_with_frequencies(x, y, &NoFrequencies);
            assert_eq!(plain.score.to_bits(), none.score.to_bits());
            assert_eq!(plain.is_match, none.is_match);
            assert_eq!(plain.confidence, none.confidence);
            assert_eq!(plain.evidence, none.evidence);
        }
    }

    #[test]
    fn frequencies_never_change_score_or_is_match() {
        let a = Thing::builder().name("USB Cable").build();
        let mut table = FrequencyTable::new();
        table.insert("name", "usb cable", 0.5);
        let plain = engine().match_things(&a, &a);
        let adj = engine().match_things_with_frequencies(&a, &a, &table);
        assert_eq!(plain.score.to_bits(), adj.score.to_bits());
        assert_eq!(plain.is_match, adj.is_match);
        assert_eq!(plain.confidence, adj.confidence);
    }

    #[test]
    fn components_sum_to_weight_bits_including_identity_terms() {
        let a = Thing::builder()
            .name("Pride and Prejudice")
            .description("Novel")
            .disambiguating_description("1813 novel")
            .add_identifier(id("doi", "10.1/a"))
            .add_identifier(id("isbn", "1"))
            .build();
        let b = Thing::builder()
            .name("Pride and Prejudice")
            .description("Novel")
            .disambiguating_description("1813 novel")
            .add_identifier(id("doi", "10.1/a"))
            .add_identifier(id("isbn", "2"))
            .build();
        let e = engine().match_things(&a, &b).evidence;
        assert_eq!(e.weight_bits.to_bits(), components_sum(&e).to_bits());
        assert!((bits_of(&e, "identifier") - IDENTIFIER_BITS).abs() < 1e-12);
        assert!((bits_of(&e, "contradiction:identifier:isbn") - CONTRADICTION_BITS).abs() < 1e-12);
    }

    #[test]
    fn phonetic_score_is_not_summed() {
        let lenient = MatchingEngine::new(MatchConfig::lenient());
        let a = Thing::builder().name("Eiffel Tower").build();
        let r = lenient.match_things(&a, &a.clone());
        assert!(r.breakdown.name_phonetic_score.is_some());
        assert_eq!(r.evidence.components.len(), 1);
        assert_eq!(r.evidence.fields_compared, 1);
    }

    #[test]
    fn frequency_table_from_counts() {
        let t = FrequencyTable::from_counts([("name", "usb cable", 50_u64)], 1000);
        assert!((t.share("name", "usb cable").unwrap() - 0.05).abs() < 1e-12);
        assert_eq!(t.share("name", "other"), None);
        let empty = FrequencyTable::from_counts([("name", "x", 1_u64)], 0);
        assert_eq!(empty, FrequencyTable::new());
    }

    #[test]
    fn frequency_key_matches_the_compared_form() {
        assert_eq!(
            frequency_key("name", "Café  Tower!"),
            Some(Normalizer::normalize_name("Café  Tower!"))
        );
        assert_eq!(frequency_key("tags", "x"), None);
        assert!(
            FREQUENCY_COMPONENTS
                .iter()
                .all(|c| frequency_key(c, "abc").is_some())
        );
    }

    #[test]
    fn old_evidence_json_without_components_deserialises() {
        let json = r#"{"weight_bits":1.5,"fields_compared":1,"contradictions":[],"strength":"consistent"}"#;
        let e: Evidence = serde_json::from_str(json).unwrap();
        assert!(e.components.is_empty());
        assert_eq!(e.strength, MatchStrength::Consistent);
    }
}
