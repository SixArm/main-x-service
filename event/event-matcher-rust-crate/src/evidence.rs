//! Match evidence: how much the compared data **proves**, beside the
//! similarity score.
//!
//! [`crate::MatchResult::score`] answers "how alike is the data both
//! records hold?". Two records holding only the same name and start date
//! agree perfectly on everything they hold, and so do two unrelated
//! events that happen to share a generic title and a common start time.
//! [`Evidence`] answers the other question — "how much does the data
//! prove?" — as a Fellegi–Sunter log-likelihood ratio in bits, plus any
//! hard [`Contradiction`]s. The family-wide contract is
//! `agents/share/match-evidence.md`; this module is this crate's copy of
//! it, with this crate's per-component `m`/`u` table, correlated groups
//! and frequency components.
//!
//! ## Rules
//!
//! - A component either record lacks (`None` in the breakdown) adds `0`
//!   bits and is not counted in [`Evidence::fields_compared`]. So does a
//!   location pair that shares no comparable sub-field (the neutral `0.5`
//!   of spec §6.4, OQ-J): that is absence, not disagreement.
//! - A similarity `s` maps onto the line between the disagreement weight
//!   `D = log2((1 − m) / (1 − u))` and the agreement weight
//!   `A = log2(m / u)`, via [`component_bits`].
//! - Components in one of [`CORRELATED_GROUPS`] are combined by
//!   [`group_bits`]: the strongest in full, the rest at
//!   [`CORRELATED_DISCOUNT`].
//! - An exact agreement on a [`FREQUENCY_COMPONENTS`] value takes its
//!   agreement weight from the value's own share ([`frequency_bits`]),
//!   when the caller's [`ValueFrequencies`] knows it.
//! - A non-blank `(scheme, value)` pair shared across `event_ids` adds
//!   [`IDENTIFIER_BITS`] once per pair. Every [`crate::EventIdScheme`]
//!   (including `Other(_)`) is identity here, because
//!   [`crate::MatchingEngine::deterministic_match`] already short-circuits
//!   on any shared pair. The `event_ids_score` breakdown component is
//!   therefore **not** weighed again as an ordinary component.
//! - The name + start-date deterministic rule is **not** an identifier;
//!   its components are weighed like any others.
//! - Per scheme, when both records hold at least one non-blank value and
//!   the value sets are disjoint, a [`Contradiction`] with field
//!   `identifier:<scheme>` is recorded and [`CONTRADICTION_BITS`] added.
//! - [`Evidence::components`] lists every term; they sum to
//!   [`Evidence::weight_bits`].

use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::matcher::MatchBreakdown;
use crate::models::{Event, EventIdScheme};
use crate::normalizer::Normalizer;

/// Similarity at or above which a component counts as full agreement.
pub const AGREE_AT: f64 = 0.95;
/// Similarity at or below which a component counts as full disagreement.
pub const DISAGREE_AT: f64 = 0.60;

/// Bits a shared value under a globally unique identifier scheme adds:
/// `log2(0.99 / 1e-12)`, about `39.85`.
pub const IDENTIFIER_BITS: f64 = 39.848_637_568_953_24;

/// Bits a contradicted identifier scheme adds:
/// `log2(0.01 / (1 − 1e-12))`, about `−6.64`.
pub const CONTRADICTION_BITS: f64 = -6.643_856_189_773_282;

/// The default evidence bar, in bits, for [`MatchStrength::Proven`]:
/// `log2(N) + log2(1/α)` for a million records at one false merge in a
/// thousand.
pub const DEFAULT_PROOF_BITS: f64 = 30.0;

/// Weight of each member of a correlated group after its strongest.
pub const CORRELATED_DISCOUNT: f64 = 0.5;

/// Smallest share used, so a value seen once cannot give unbounded bits.
pub const MIN_SHARE: f64 = 1e-9;

/// Groups of components that are not independent given the pair's truth
/// (spec §5.5). Within a group the strongest present member counts in
/// full and every other present member at [`CORRELATED_DISCOUNT`].
///
/// - `start_date` / `end_date`: one event's two endpoints; an end date is
///   usually the start plus a standard duration, and both move together
///   when an event is rescheduled.
/// - `location` / `organizer` / `country_code`: a series run by one
///   organiser at one venue agrees on all three for one reason, and the
///   venue fixes the country.
/// - `url` / `name`: event pages are slugged from the title, and a
///   recurring series reuses both its title and its page.
pub const CORRELATED_GROUPS: &[&[&str]] = &[
    &["start_date", "end_date"],
    &["location", "organizer", "country_code"],
    &["url", "name"],
];

/// Components whose exact agreement is frequency-adjusted, keyed by these
/// names in [`ValueFrequencies::share`] and [`frequency_key`]. Public
/// contract (spec §5.5, §9.2).
pub const FREQUENCY_COMPONENTS: &[&str] = &["name"];

/// The `m` / `u` probabilities for one breakdown component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComponentWeights {
    /// Chance the component agrees when the records are the same event.
    pub m: f64,
    /// Chance the component agrees by coincidence for different events.
    pub u: f64,
}

/// `name_score`. `u`: two unrelated events rarely share a full title,
/// but generic titles ("Annual General Meeting", "Open Mic Night") recur,
/// so `1/200` rather than a full-name-of-a-person rarity.
pub const NAME: ComponentWeights = ComponentWeights { m: 0.95, u: 0.005 };
/// `start_date_score`. `u`: full agreement means within ~13 minutes at the
/// default one-hour scale; unrelated events cluster on round hours of
/// busy days, so about `1/1000` rather than one minute in a year.
pub const START_DATE: ComponentWeights = ComponentWeights { m: 0.90, u: 0.001 };
/// `end_date_score`. `u`: end times cluster even more than start times
/// (and correlate with them), so a weaker `1/100`; `m` lower because end
/// times are often missing-then-guessed or revised.
pub const END_DATE: ComponentWeights = ComponentWeights { m: 0.85, u: 0.01 };
/// `location_score`. `u`: two unrelated events sharing a venue / address /
/// coordinates within 100 m — busy venues host many events, so `1/100`.
pub const LOCATION: ComponentWeights = ComponentWeights { m: 0.85, u: 0.01 };
/// `category_score`. `u`: about one in ten unrelated events share a
/// schema.org Event subtype (the enum is short and skewed).
pub const CATEGORY: ComponentWeights = ComponentWeights { m: 0.90, u: 0.10 };
/// `country_code_score`. `u`: registries are dominated by a few
/// countries, so unrelated events share one about a fifth of the time.
pub const COUNTRY_CODE: ComponentWeights = ComponentWeights { m: 0.98, u: 0.20 };
/// `organizer_score`. `u`: an active organiser runs many events, so about
/// `1/100`; `m` lower because organiser names are written inconsistently.
pub const ORGANIZER: ComponentWeights = ComponentWeights { m: 0.85, u: 0.01 };
/// `performers_score` (best-of pair). `u`: touring performers appear at
/// many events, so `1/100`; `m` lower because line-ups change.
pub const PERFORMERS: ComponentWeights = ComponentWeights { m: 0.80, u: 0.01 };
/// `url_score` (exact). `u`: an event page URL is close to unique; shared
/// venue or series pages keep it at `1e-4` rather than identifier-grade.
pub const URL: ComponentWeights = ComponentWeights { m: 0.90, u: 0.0001 };
/// `relationships_score` (Jaccard). Weak: overlapping super-/sub-event
/// references corroborate (sibling sessions share a parent) but do not
/// identify; `u` about one in twenty.
pub const RELATIONSHIPS: ComponentWeights = ComponentWeights { m: 0.70, u: 0.05 };
/// `tags_score` (Jaccard). Weak: operator tags are coarse and shared
/// widely; `u` about one in ten.
pub const TAGS: ComponentWeights = ComponentWeights { m: 0.60, u: 0.10 };

// `name_phonetic_score` is deliberately **not** weighed: it is derived
// from the same names as `name_score`, so counting it would double-count
// one piece of evidence. `event_ids_score` is replaced by the identifier
// rule (IDENTIFIER_BITS / CONTRADICTION_BITS) for the same reason.

/// One hard conflict between the two records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contradiction {
    /// What conflicts, e.g. `"identifier:eventbrite"`.
    pub field: String,
}

/// The evidence verdict. Checked in declaration order.
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

/// One term of the evidence sum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentBits {
    /// The component, e.g. `"name"`, `"identifier"`,
    /// `"contradiction:identifier:eventbrite"`.
    pub component: String,
    /// Bits this term added, after any discount or adjustment.
    pub bits: f64,
    /// Discounted as a non-strongest member of a correlated group.
    pub correlated: bool,
    /// Agreement weight taken from the value's own frequency.
    pub frequency_adjusted: bool,
}

/// How much the compared data proves, beside the similarity score.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Evidence {
    /// Sum of the components' log-likelihood weights, in bits. Always
    /// equal to the sum of [`Evidence::components`]' `bits`, in order.
    pub weight_bits: f64,
    /// Components both records held (identifiers count as one).
    pub fields_compared: u32,
    /// Hard conflicts; any one makes the strength `Contradicted`.
    pub contradictions: Vec<Contradiction>,
    /// The verdict.
    pub strength: MatchStrength,
    /// Every term of the sum. Empty on JSON written before the field.
    #[serde(default)]
    pub components: Vec<ComponentBits>,
}

/// The weight, in bits, one compared component adds.
///
/// ```
/// use event_matcher::evidence::component_bits;
/// assert!(component_bits(1.0, 0.95, 0.005) > 7.0);
/// assert!(component_bits(0.0, 0.95, 0.005) < 0.0);
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

/// Canonical order for a correlated group's members: larger absolute value
/// first, ties to the larger value. Total over finite inputs.
fn stronger(a: f64, b: f64) -> std::cmp::Ordering {
    b.abs().total_cmp(&a.abs()).then_with(|| b.total_cmp(&a))
}

/// Split a group's member bits into its strongest member and the rest,
/// both in the canonical order (absolute value descending, ties to the
/// larger value), so the result does not depend on input order. `None`
/// for an empty slice.
///
/// ```
/// use event_matcher::evidence::strongest_first;
/// let (s, rest) = strongest_first(&[2.0, -5.0, 5.0]).unwrap();
/// assert_eq!(s, 5.0);
/// assert_eq!(rest, vec![-5.0, 2.0]);
/// ```
#[must_use]
pub fn strongest_first(member_bits: &[f64]) -> Option<(f64, Vec<f64>)> {
    let mut sorted = member_bits.to_vec();
    sorted.sort_by(|a, b| stronger(*a, *b));
    let mut iter = sorted.into_iter();
    let strongest = iter.next()?;
    Some((strongest, iter.collect()))
}

/// The bits a correlated group adds, from its present members' bits.
///
/// ```
/// use event_matcher::evidence::group_bits;
/// assert_eq!(group_bits(&[4.0, 6.0]), 8.0);
/// assert_eq!(group_bits(&[]), 0.0);
/// ```
#[must_use]
pub fn group_bits(member_bits: &[f64]) -> f64 {
    let Some((strongest, rest)) = strongest_first(member_bits) else {
        return 0.0;
    };
    strongest + CORRELATED_DISCOUNT * rest.iter().sum::<f64>()
}

/// Agreement bits for a value held by `share` of records.
///
/// A share at or above `m` gives `0`: a value most records hold is no
/// evidence, but agreeing on it is not evidence against either.
///
/// ```
/// use event_matcher::evidence::frequency_bits;
/// assert_eq!(frequency_bits(0.95, 0.99), 0.0);
/// assert!(frequency_bits(0.95, 1e-6) > 19.0);
/// ```
#[must_use]
pub fn frequency_bits(m: f64, share: f64) -> f64 {
    let share = if share.is_finite() {
        share.clamp(MIN_SHARE, m)
    } else {
        m
    };
    (m / share).log2()
}

/// Where the share of records holding a value comes from.
pub trait ValueFrequencies {
    /// The share, in `0.0..=1.0`, of records whose `component` holds
    /// `value` (already in the matcher's normalised form, see
    /// [`frequency_key`]), or `None` when unknown.
    fn share(&self, component: &str, value: &str) -> Option<f64>;
}

/// A frequency source that knows nothing: every share is `None`, so the
/// plain `m`/`u` table applies. [`crate::MatchingEngine::match_events`]
/// uses it.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoFrequencies;

impl ValueFrequencies for NoFrequencies {
    fn share(&self, _component: &str, _value: &str) -> Option<f64> {
        None
    }
}

/// The normalised key the matcher looks up for `component`'s raw value,
/// or `None` when `component` is not in [`FREQUENCY_COMPONENTS`] or the
/// value normalises to empty. For `"name"` this is
/// [`Normalizer::normalize_name`], the form names are compared in.
///
/// ```
/// use event_matcher::evidence::frequency_key;
/// assert_eq!(frequency_key("name", "  Annual General Meeting! ").as_deref(),
///            Some("annual general meeting"));
/// assert_eq!(frequency_key("url", "https://example.org"), None);
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

/// A [`ValueFrequencies`] built from counts. Values are stored under
/// [`frequency_key`], so raw or already-normalised input both work.
#[derive(Debug, Clone, Default)]
pub struct FrequencyTable {
    shares: HashMap<(String, String), f64>,
}

impl FrequencyTable {
    /// An empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a table from `(component, value, count)` triples over a
    /// population of `total` records: each share is `count / total`.
    /// Counts for one key are added. A `total` of `0` gives an empty table.
    ///
    /// ```
    /// use event_matcher::evidence::{FrequencyTable, ValueFrequencies};
    /// let t = FrequencyTable::from_counts([("name", "Open Mic Night", 50_u64)], 1_000);
    /// assert_eq!(t.share("name", "open mic night"), Some(0.05));
    /// ```
    #[must_use]
    pub fn from_counts<C, V, I>(counts: I, total: u64) -> Self
    where
        C: AsRef<str>,
        V: AsRef<str>,
        I: IntoIterator<Item = (C, V, u64)>,
    {
        let mut table = Self::new();
        if total == 0 {
            return table;
        }
        let mut summed: HashMap<(String, String), u64> = HashMap::new();
        for (component, value, count) in counts {
            if let Some(key) = frequency_key(component.as_ref(), value.as_ref()) {
                *summed
                    .entry((component.as_ref().to_string(), key))
                    .or_insert(0) += count;
            }
        }
        // `u64 -> f64` loses precision only above 2^53 records, far beyond
        // any registry, and a share needs no more than f64's precision.
        #[allow(clippy::cast_precision_loss)]
        let total = total as f64;
        for (key, count) in summed {
            #[allow(clippy::cast_precision_loss)]
            let count = count as f64;
            table.shares.insert(key, (count / total).clamp(0.0, 1.0));
        }
        table
    }

    /// Set the share of `component`'s `value` directly. Ignored when the
    /// component is not frequency-adjusted or the value normalises empty.
    pub fn insert(&mut self, component: &str, value: &str, share: f64) {
        if let Some(key) = frequency_key(component, value) {
            self.shares.insert((component.to_string(), key), share);
        }
    }
}

impl ValueFrequencies for FrequencyTable {
    fn share(&self, component: &str, value: &str) -> Option<f64> {
        self.shares
            .get(&(component.to_string(), value.to_string()))
            .copied()
    }
}

/// The lowercase name used in a contradiction's `identifier:<scheme>`.
fn scheme_key(scheme: &EventIdScheme) -> String {
    match scheme {
        EventIdScheme::Wikidata => "wikidata".into(),
        EventIdScheme::Eventbrite => "eventbrite".into(),
        EventIdScheme::Meetup => "meetup".into(),
        EventIdScheme::Ticketmaster => "ticketmaster".into(),
        EventIdScheme::Songkick => "songkick".into(),
        EventIdScheme::Bandsintown => "bandsintown".into(),
        EventIdScheme::Facebook => "facebook".into(),
        EventIdScheme::Luma => "luma".into(),
        EventIdScheme::GoogleCalendar => "googlecalendar".into(),
        EventIdScheme::ICalendarUid => "icalendaruid".into(),
        EventIdScheme::Other(name) => format!("other:{}", name.trim().to_lowercase()),
    }
}

/// Group an event's non-blank identifier values by scheme. Keys are the
/// scheme itself (equality is scheme-scoped, as for the short-circuit);
/// values are trimmed, so a blank value — reachable via serde or a struct
/// literal despite [`crate::EventId::new`]'s guard — is ignored (SEC-M2).
fn ids_by_scheme(event: &Event) -> BTreeMap<String, (EventIdScheme, BTreeSet<&str>)> {
    let mut map: BTreeMap<String, (EventIdScheme, BTreeSet<&str>)> = BTreeMap::new();
    for id in &event.event_ids {
        let value = id.value.trim();
        if value.is_empty() {
            continue;
        }
        // Key by the exact scheme's debug form so `Other("Foo")` and
        // `Other("foo")` stay distinct, exactly as `EventId` equality does.
        let key = format!("{:?}", id.scheme);
        map.entry(key)
            .or_insert_with(|| (id.scheme.clone(), BTreeSet::new()))
            .1
            .insert(value);
    }
    map
}

/// Normalised name keys an event holds (primary plus alternates).
fn name_keys(event: &Event) -> BTreeSet<String> {
    event
        .name
        .iter()
        .chain(event.alternate_names.iter())
        .filter_map(|n| frequency_key("name", n))
        .collect()
}

/// The frequency-adjusted name agreement weight, when the two events
/// share a normalised name whose share is known. Several shared names:
/// the most common is used, the conservative choice.
fn name_frequency_bits(e1: &Event, e2: &Event, freq: &dyn ValueFrequencies) -> Option<f64> {
    let k1 = name_keys(e1);
    let k2 = name_keys(e2);
    k1.intersection(&k2)
        .filter_map(|k| freq.share("name", k))
        .filter(|s| s.is_finite())
        .max_by(f64::total_cmp)
        .map(|share| frequency_bits(NAME.m, share))
}

/// Whether the two events share an identifier value, and the schemes
/// under which both hold values but share none.
fn assess_identifiers(event1: &Event, event2: &Event) -> (bool, Vec<Contradiction>) {
    let ids1 = ids_by_scheme(event1);
    let ids2 = ids_by_scheme(event2);
    let mut shared = false;
    let mut contradictions = Vec::new();
    for (key, (scheme, values1)) in &ids1 {
        if let Some((_, values2)) = ids2.get(key) {
            if values1.is_disjoint(values2) {
                contradictions.push(Contradiction {
                    field: format!("identifier:{}", scheme_key(scheme)),
                });
            } else {
                shared = true;
            }
        }
    }
    (shared, contradictions)
}

/// Everything [`compute`] needs beyond the two events.
pub(crate) struct EvidenceInputs<'a> {
    /// The probabilistic breakdown.
    pub breakdown: &'a MatchBreakdown,
    /// The location similarity for evidence: `None` when either side has
    /// no location **or** the two share no comparable sub-field (OQ-J).
    pub location: Option<f64>,
    /// The already-decided `is_match`.
    pub is_match: bool,
    /// The bar for `Proven`.
    pub proof_bits: f64,
}

/// Compute the evidence for a pair.
pub(crate) fn compute(
    event1: &Event,
    event2: &Event,
    inputs: &EvidenceInputs<'_>,
    freq: &dyn ValueFrequencies,
) -> Evidence {
    let b = inputs.breakdown;

    // 1. Raw per-component terms, in a fixed order.
    let mut terms: Vec<ComponentBits> = Vec::new();
    let mut push = |name: &str, score: Option<f64>, w: ComponentWeights, adjusted: Option<f64>| {
        if let Some(s) = score {
            let plain = component_bits(s, w.m, w.u);
            // Frequency adjustment replaces `u` only on full agreement.
            let (bits, frequency_adjusted) = match adjusted {
                Some(a) if s >= AGREE_AT => (a, true),
                _ => (plain, false),
            };
            terms.push(ComponentBits {
                component: name.to_string(),
                bits,
                correlated: false,
                frequency_adjusted,
            });
        }
    };
    // Exact (normalised) agreement on a name implies `name_score == 1.0`;
    // `name_frequency_bits` is `Some` only for such a shared value.
    push(
        "name",
        b.name_score,
        NAME,
        name_frequency_bits(event1, event2, freq),
    );
    push("start_date", b.start_date_score, START_DATE, None);
    push("end_date", b.end_date_score, END_DATE, None);
    push("location", inputs.location, LOCATION, None);
    push("category", b.category_score, CATEGORY, None);
    push("country_code", b.country_code_score, COUNTRY_CODE, None);
    push("organizer", b.organizer_score, ORGANIZER, None);
    push("performers", b.performers_score, PERFORMERS, None);
    push("url", b.url_score, URL, None);
    push("relationships", b.relationships_score, RELATIONSHIPS, None);
    push("tags", b.tags_score, TAGS, None);
    let mut compared = u32::try_from(terms.len()).unwrap_or(u32::MAX);

    // 2. Correlated groups: every non-strongest present member is
    //    discounted; which one is strongest follows `strongest_first`.
    for group in CORRELATED_GROUPS {
        let members: Vec<usize> = terms
            .iter()
            .enumerate()
            .filter(|(_, t)| group.contains(&t.component.as_str()))
            .map(|(i, _)| i)
            .collect();
        let Some(&strongest) = members
            .iter()
            .min_by(|&&i, &&j| stronger(terms[i].bits, terms[j].bits).then(i.cmp(&j)))
        else {
            continue;
        };
        for &i in &members {
            if i != strongest {
                terms[i].bits *= CORRELATED_DISCOUNT;
                terms[i].correlated = true;
            }
        }
    }

    // 3. Identifiers: one shared value anywhere is IDENTIFIER_BITS once;
    //    each scheme both sides hold with disjoint values is a contradiction.
    let (shared, contradictions) = assess_identifiers(event1, event2);
    if shared {
        terms.push(ComponentBits {
            component: "identifier".into(),
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
    if shared || !contradictions.is_empty() {
        compared += 1;
    }

    // 4. The total is the sum of the listed terms, in order, so the two
    //    can never disagree.
    let weight_bits: f64 = terms.iter().map(|t| t.bits).sum();

    let strength = if !contradictions.is_empty() {
        MatchStrength::Contradicted
    } else if weight_bits >= inputs.proof_bits {
        MatchStrength::Proven
    } else if inputs.is_match {
        MatchStrength::Consistent
    } else {
        MatchStrength::NonMatch
    };

    Evidence {
        weight_bits,
        fields_compared: compared,
        contradictions,
        strength,
        components: terms,
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::many_single_char_names)]
mod tests {
    use super::*;
    use crate::{
        Address, EventCategory, EventId, Location, MatchConfig, MatchResult, MatchingEngine,
        RelationKind, RelationshipRef,
    };

    fn engine() -> MatchingEngine {
        MatchingEngine::default_config()
    }

    fn sparse() -> Event {
        Event::builder()
            .name("RustConf 2024")
            .start_date("2024-09-10T09:00:00Z")
            .build()
    }

    fn rich() -> Event {
        Event::builder()
            .name("Glastonbury Festival 2024")
            .start_date("2024-06-26T09:00:00Z")
            .end_date("2024-06-30T23:59:00Z")
            .location(
                Location::new()
                    .with_venue_name("Worthy Farm")
                    .with_address(
                        Address::new()
                            .with_line1("Worthy Lane")
                            .with_city("Pilton")
                            .with_postcode("BA4 4BY")
                            .with_country("GB"),
                    )
                    .with_latitude(51.1537)
                    .with_longitude(-2.5857),
            )
            .category(EventCategory::Festival)
            .country_code_as_iso_3166_1_alpha_2("GB")
            .organizer("Glastonbury Festivals Ltd")
            .add_performer("Example Band")
            .url("https://example.org/glastonbury-2024")
            .add_relationship(RelationshipRef::new(RelationKind::Inner, "series-glasto").unwrap())
            .add_tag("music")
            .build()
    }

    #[test]
    fn constants_match_their_formulas() {
        assert!((IDENTIFIER_BITS - (0.99_f64 / 1e-12).log2()).abs() < 1e-9);
        assert!((CONTRADICTION_BITS - (0.01_f64 / (1.0 - 1e-12)).log2()).abs() < 1e-9);
    }

    #[test]
    fn component_bits_interpolates_and_handles_nan() {
        let w = NAME;
        let a = component_bits(1.0, w.m, w.u);
        let d = component_bits(0.0, w.m, w.u);
        let mid = component_bits(0.775, w.m, w.u);
        assert!(a > 0.0 && d < 0.0);
        assert!((mid - f64::midpoint(a, d)).abs() < 1e-9);
        assert!((component_bits(f64::NAN, w.m, w.u) - d).abs() < 1e-12);
    }

    #[test]
    fn missing_component_adds_zero_and_is_not_counted() {
        let with = engine().match_events(&sparse(), &sparse());
        let mut b = sparse();
        b.url = Some("https://example.org/x".into());
        // `url` is present on one side only, so it is `None` in the breakdown.
        let without = engine().match_events(&sparse(), &b);
        assert!(without.breakdown.url_score.is_none());
        assert_eq!(with.evidence.fields_compared, 2);
        assert_eq!(without.evidence.fields_compared, 2);
        assert!((with.evidence.weight_bits - without.evidence.weight_bits).abs() < 1e-12);
    }

    #[test]
    fn identical_sparse_records_are_consistent_not_proven() {
        let r = engine().match_events(&sparse(), &sparse());
        assert!(r.is_match);
        assert!(r.score > 0.99);
        assert_eq!(r.evidence.strength, MatchStrength::Consistent);
        assert!(r.evidence.weight_bits < MatchConfig::default().proof_bits);
        let expected = (NAME.m / NAME.u).log2() + (START_DATE.m / START_DATE.u).log2();
        assert!((r.evidence.weight_bits - expected).abs() < 1e-9);
    }

    #[test]
    fn shared_unique_identifier_is_proven() {
        let id = EventId::new(EventIdScheme::Eventbrite, "123456789").unwrap();
        let a = Event::builder()
            .name("RustConf 2024")
            .add_event_id(id.clone())
            .build();
        let b = Event::builder().name("RC '24").add_event_id(id).build();
        let r = engine().match_events(&a, &b);
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
        assert!(r.evidence.weight_bits >= 30.0);
        assert!(r.evidence.contradictions.is_empty());
    }

    #[test]
    fn conflicting_same_scheme_identifiers_are_contradicted() {
        let a = Event {
            event_ids: vec![EventId::new(EventIdScheme::Wikidata, "Q1").unwrap()],
            ..rich()
        };
        let b = Event {
            event_ids: vec![EventId::new(EventIdScheme::Wikidata, "Q2").unwrap()],
            ..rich()
        };
        let r = engine().match_events(&a, &b);
        assert_eq!(r.evidence.strength, MatchStrength::Contradicted);
        assert_eq!(
            r.evidence.contradictions,
            vec![Contradiction {
                field: "identifier:wikidata".into()
            }]
        );
    }

    #[test]
    fn different_schemes_neither_share_nor_contradict() {
        let a = Event::builder()
            .name("X")
            .add_event_id(EventId::new(EventIdScheme::Eventbrite, "1").unwrap())
            .build();
        let b = Event::builder()
            .name("X")
            .add_event_id(EventId::new(EventIdScheme::Meetup, "1").unwrap())
            .build();
        let r = engine().match_events(&a, &b);
        assert!(r.evidence.contradictions.is_empty());
        assert_eq!(r.evidence.fields_compared, 1);
    }

    #[test]
    fn several_values_under_one_scheme_do_not_contradict_a_partner_holding_one() {
        let a = Event::builder()
            .name("X")
            .add_event_id(EventId::new(EventIdScheme::Meetup, "old").unwrap())
            .add_event_id(EventId::new(EventIdScheme::Meetup, "new").unwrap())
            .build();
        let b = Event::builder()
            .name("X")
            .add_event_id(EventId::new(EventIdScheme::Meetup, "new").unwrap())
            .build();
        let r = engine().match_events(&a, &b);
        assert!(r.evidence.contradictions.is_empty());
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    #[test]
    fn blank_identifier_values_are_ignored() {
        let blank = EventId {
            scheme: EventIdScheme::Eventbrite,
            value: "  ".into(),
        };
        let a = Event {
            event_ids: vec![blank.clone()],
            ..sparse()
        };
        let b = Event {
            event_ids: vec![blank],
            ..sparse()
        };
        let r = engine().match_events(&a, &b);
        assert!(r.evidence.contradictions.is_empty());
        assert_eq!(r.evidence.strength, MatchStrength::Consistent);
    }

    #[test]
    fn richly_identical_record_without_identifier_is_proven() {
        let r = engine().match_events(&rich(), &rich());
        assert!(rich().event_ids.is_empty());
        assert_eq!(r.evidence.fields_compared, 11);
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
        // Every component agrees fully. Ungrouped: their agreement weights.
        // Grouped: the strongest in full, the rest at half.
        let a = |w: ComponentWeights| (w.m / w.u).log2();
        let full = group_bits(&[a(START_DATE), a(END_DATE)])
            + group_bits(&[a(LOCATION), a(ORGANIZER), a(COUNTRY_CODE)])
            + group_bits(&[a(URL), a(NAME)])
            + a(CATEGORY)
            + a(PERFORMERS)
            + a(RELATIONSHIPS)
            + a(TAGS);
        assert!((r.evidence.weight_bits - full).abs() < 1e-9);
        assert!((r.evidence.weight_bits - 56.58).abs() < 0.01);
        // Without the URL (the strongest non-identifier field) it still clears the bar.
        let mut no_url = rich();
        no_url.url = None;
        let r = engine().match_events(&no_url, &rich());
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    #[test]
    fn clear_disagreement_gives_negative_bits() {
        let a = sparse();
        let b = Event::builder()
            .name("Totally Different Gathering")
            .start_date("2019-01-01T00:00:00Z")
            .build();
        let r = engine().match_events(&a, &b);
        assert!(r.evidence.weight_bits < 0.0);
        assert_eq!(r.evidence.strength, MatchStrength::NonMatch);
    }

    #[test]
    fn proof_bits_is_configurable() {
        let cfg = MatchConfig {
            proof_bits: 10.0,
            ..MatchConfig::default()
        };
        let r = MatchingEngine::new(cfg).match_events(&sparse(), &sparse());
        assert_eq!(r.evidence.strength, MatchStrength::Proven);
    }

    #[test]
    fn old_json_without_evidence_deserialises() {
        let r = engine().match_events(&sparse(), &sparse());
        let mut v = serde_json::to_value(&r).unwrap();
        v.as_object_mut().unwrap().remove("evidence");
        let back: MatchResult = serde_json::from_value(v).unwrap();
        assert_eq!(back.evidence, Evidence::default());
        assert_eq!(back.evidence.strength, MatchStrength::NonMatch);

        // Evidence written before `components` existed.
        let mut v = serde_json::to_value(&r).unwrap();
        v["evidence"].as_object_mut().unwrap().remove("components");
        let back: MatchResult = serde_json::from_value(v).unwrap();
        assert!(back.evidence.components.is_empty());
        assert!((back.evidence.weight_bits - r.evidence.weight_bits).abs() < 1e-12);

        let cfg: MatchConfig = serde_json::from_str("{}").unwrap();
        assert!((cfg.proof_bits - DEFAULT_PROOF_BITS).abs() < 1e-12);
    }

    #[test]
    fn strength_serialises_snake_case() {
        assert_eq!(
            serde_json::to_string(&MatchStrength::NonMatch).unwrap(),
            "\"non_match\""
        );
    }

    // ---------- phase 2: correlated groups ----------

    fn term<'a>(e: &'a Evidence, name: &str) -> Option<&'a ComponentBits> {
        e.components.iter().find(|t| t.component == name)
    }

    /// The sum the evidence would have had with no group discount.
    fn undiscounted(e: &Evidence) -> f64 {
        e.components
            .iter()
            .map(|t| {
                if t.correlated {
                    t.bits / CORRELATED_DISCOUNT
                } else {
                    t.bits
                }
            })
            .sum()
    }

    #[test]
    fn correlated_agreements_give_less_than_their_sum_in_any_order() {
        let a = (START_DATE.m / START_DATE.u).log2();
        let b = (END_DATE.m / END_DATE.u).log2();
        let g = group_bits(&[a, b]);
        assert!(g < a + b);
        assert!((g - (a + 0.5 * b)).abs() < 1e-12);
        assert_eq!(group_bits(&[b, a]).to_bits(), g.to_bits());
        let three = [1.0, -4.0, 2.5];
        assert_eq!(
            group_bits(&three).to_bits(),
            group_bits(&[2.5, 1.0, -4.0]).to_bits()
        );
        // Ties on absolute value go to the larger value.
        assert_eq!(strongest_first(&[-5.0, 5.0]).unwrap().0, 5.0);
        assert_eq!(strongest_first(&[5.0, -5.0]).unwrap().0, 5.0);
        assert!(strongest_first(&[]).is_none());

        // Through the engine: start in full, end discounted.
        let e = Event::builder()
            .start_date("2024-09-10T09:00:00Z")
            .end_date("2024-09-10T17:00:00Z")
            .build();
        let r = engine().match_events(&e, &e);
        assert!((r.evidence.weight_bits - g).abs() < 1e-12);
        assert!(!term(&r.evidence, "start_date").unwrap().correlated);
        assert!(term(&r.evidence, "end_date").unwrap().correlated);
    }

    #[test]
    fn uncorrelated_fields_still_sum() {
        let e = Event::builder()
            .name("Example Chess Open")
            .category(EventCategory::SportsEvent)
            .add_performer("Example Quartet")
            .add_tag("chess")
            .build();
        let r = engine().match_events(&e, &e);
        let a = |w: ComponentWeights| (w.m / w.u).log2();
        let sum = a(NAME) + a(CATEGORY) + a(PERFORMERS) + a(TAGS);
        assert!((r.evidence.weight_bits - sum).abs() < 1e-12);
        assert!(r.evidence.components.iter().all(|t| !t.correlated));
    }

    #[test]
    fn correlated_disagreements_are_discounted_too() {
        let a = Event::builder()
            .start_date("2024-09-10T09:00:00Z")
            .end_date("2024-09-10T17:00:00Z")
            .build();
        let b = Event::builder()
            .start_date("2023-01-01T09:00:00Z")
            .end_date("2023-01-01T17:00:00Z")
            .build();
        let r = engine().match_events(&a, &b);
        let d = |w: ComponentWeights| ((1.0 - w.m) / (1.0 - w.u)).log2();
        // Start disagrees harder (−3.32) than end (−2.72), so it is the strongest.
        let expected = d(START_DATE) + CORRELATED_DISCOUNT * d(END_DATE);
        assert!((r.evidence.weight_bits - expected).abs() < 1e-12);
        assert!(r.evidence.weight_bits > d(START_DATE) + d(END_DATE));
        assert!(term(&r.evidence, "end_date").unwrap().correlated);
    }

    /// Two different occurrences of one recurring event: same name,
    /// organiser, venue, page, category and country, a week apart.
    #[test]
    fn series_occurrences_are_not_proven() {
        let occurrence = || Event {
            performers: vec![],
            relationships: vec![],
            tags: vec![],
            ..rich()
        };
        let week_one = occurrence();
        let week_two = Event {
            start_date: Some("2024-07-03T09:00:00Z".into()),
            end_date: Some("2024-07-07T23:59:00Z".into()),
            ..occurrence()
        };

        let r = engine().match_events(&week_one, &week_two);
        assert_ne!(r.evidence.strength, MatchStrength::Proven);
        assert!(r.evidence.weight_bits < 30.0);
        assert!((r.evidence.weight_bits - 26.17).abs() < 0.01);
        // Summed as if independent, the same terms would have cleared the bar.
        assert!(undiscounted(&r.evidence) >= 30.0);
    }

    // ---------- phase 2: value frequencies ----------

    fn agm() -> Event {
        Event::builder()
            .name("Annual General Meeting")
            .start_date("2024-09-10T18:00:00Z")
            .build()
    }

    fn name_bits_with_share(share: f64) -> ComponentBits {
        let mut t = FrequencyTable::new();
        t.insert("name", "Annual General Meeting", share);
        let r = engine().match_events_with_frequencies(&agm(), &agm(), &t);
        term(&r.evidence, "name").unwrap().clone()
    }

    #[test]
    fn common_name_gives_fewer_bits_than_default_and_rare_more() {
        let default = (NAME.m / NAME.u).log2();
        let common = name_bits_with_share(0.01);
        let rare = name_bits_with_share(1e-6);
        assert!(common.frequency_adjusted && rare.frequency_adjusted);
        assert!(common.bits < default);
        assert!(rare.bits > default);
        assert!((common.bits - (0.95_f64 / 0.01).log2()).abs() < 1e-12);
    }

    #[test]
    fn agreeing_on_a_very_common_name_proves_little() {
        // One record in five is an "Annual General Meeting".
        let t = name_bits_with_share(0.2);
        assert!(t.bits < 2.5, "{}", t.bits);
        let mut table = FrequencyTable::new();
        table.insert("name", "Annual General Meeting", 0.2);
        let plain = engine().match_events(&agm(), &agm());
        let adjusted = engine().match_events_with_frequencies(&agm(), &agm(), &table);
        assert!(adjusted.evidence.weight_bits < plain.evidence.weight_bits - 5.0);
        assert_eq!(plain.score.to_bits(), adjusted.score.to_bits());
        assert_eq!(plain.is_match, adjusted.is_match);
        assert_eq!(plain.confidence, adjusted.confidence);
    }

    #[test]
    fn share_at_or_above_m_gives_zero_never_negative() {
        assert_eq!(frequency_bits(0.95, 0.95), 0.0);
        assert_eq!(frequency_bits(0.95, 1.0), 0.0);
        assert_eq!(frequency_bits(0.95, f64::NAN), 0.0);
        assert!((frequency_bits(0.95, 0.0) - (0.95 / MIN_SHARE).log2()).abs() < 1e-9);
        let t = name_bits_with_share(0.99);
        assert!(t.frequency_adjusted);
        assert_eq!(t.bits, 0.0);
    }

    #[test]
    fn no_frequency_adjustment_on_partial_agreement_or_disagreement() {
        let mut table = FrequencyTable::new();
        for n in [
            "Annual General Meeting",
            "Annual General Meetings",
            "Chess Club",
        ] {
            table.insert("name", n, 0.2);
        }
        // Near but not equal after normalisation: no shared value to look up.
        let near = Event::builder().name("Annual General Meetings").build();
        let r = engine().match_events_with_frequencies(&agm(), &near, &table);
        let plain = engine().match_events(&agm(), &near);
        assert!(!term(&r.evidence, "name").unwrap().frequency_adjusted);
        assert_eq!(r.evidence, plain.evidence);
        // Disagreement.
        let other = Event::builder().name("Chess Club").build();
        let r = engine().match_events_with_frequencies(&agm(), &other, &table);
        let t = term(&r.evidence, "name").unwrap();
        assert!(!t.frequency_adjusted);
        assert!(t.bits < 0.0);
    }

    #[test]
    fn no_frequencies_equals_match_events() {
        for (a, b) in [(rich(), rich()), (sparse(), rich()), (agm(), agm())] {
            let plain = engine().match_events(&a, &b);
            let none = engine().match_events_with_frequencies(&a, &b, &NoFrequencies);
            assert_eq!(
                serde_json::to_value(&plain).unwrap(),
                serde_json::to_value(&none).unwrap()
            );
        }
    }

    #[test]
    fn frequency_table_from_counts_uses_matcher_keys() {
        let t = FrequencyTable::from_counts(
            [
                ("name", "Open Mic Night", 30_u64),
                ("name", "open mic night!", 20),
                ("url", "https://example.org", 5),
            ],
            1_000,
        );
        assert_eq!(t.share("name", "open mic night"), Some(0.05));
        assert_eq!(t.share("url", "https://example.org"), None);
        assert!(
            FrequencyTable::from_counts([("name", "x", 1_u64)], 0)
                .share("name", "x")
                .is_none()
        );
        assert_eq!(FREQUENCY_COMPONENTS, &["name"]);
        assert_eq!(frequency_key("name", "###"), None);
    }

    // ---------- phase 2: explainability and OQ-J ----------

    #[test]
    fn components_list_every_term_and_sum_to_weight_bits() {
        let id = EventId::new(EventIdScheme::Eventbrite, "1").unwrap();
        let a = Event {
            event_ids: vec![
                id.clone(),
                EventId::new(EventIdScheme::Meetup, "a").unwrap(),
            ],
            ..rich()
        };
        let b = Event {
            event_ids: vec![id, EventId::new(EventIdScheme::Meetup, "b").unwrap()],
            ..rich()
        };
        let e = engine().match_events(&a, &b).evidence;
        assert_eq!(e.strength, MatchStrength::Contradicted);
        assert!((term(&e, "identifier").unwrap().bits - IDENTIFIER_BITS).abs() < 1e-12);
        assert!(
            (term(&e, "contradiction:identifier:meetup").unwrap().bits - CONTRADICTION_BITS).abs()
                < 1e-12
        );
        let sum: f64 = e.components.iter().map(|t| t.bits).sum();
        assert_eq!(sum.to_bits(), e.weight_bits.to_bits());
    }

    #[test]
    fn empty_location_is_missing_not_disagreement() {
        let a = Event {
            location: Some(Location::new()),
            ..sparse()
        };
        let r = engine().match_events(&a, &a);
        // The score path keeps the neutral 0.5 (spec §6.4, unchanged)…
        assert_eq!(r.breakdown.location_score, Some(0.5));
        // …but the evidence treats it as absent.
        assert!(term(&r.evidence, "location").is_none());
        let plain = engine().match_events(&sparse(), &sparse());
        assert_eq!(
            r.evidence.weight_bits.to_bits(),
            plain.evidence.weight_bits.to_bits()
        );
        assert_eq!(r.evidence.fields_compared, plain.evidence.fields_compared);

        // A nested empty address beside a shared venue name is left out of
        // the evidence blend, so the venue alone decides: full agreement.
        let venue = Location::new()
            .with_venue_name("Example Hall")
            .with_address(Address::new());
        let b = Event {
            location: Some(venue),
            ..sparse()
        };
        let r = engine().match_events(&b, &b);
        assert!(r.breakdown.location_score.unwrap() < 0.95);
        let t = term(&r.evidence, "location").unwrap();
        assert!((t.bits - (LOCATION.m / LOCATION.u).log2()).abs() < 1e-12);
    }
}
