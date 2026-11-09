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
//! it, with this crate's per-component `m`/`u` table.
//!
//! ## Rules
//!
//! - A component either record lacks (`None` in the breakdown) adds `0`
//!   bits and is not counted in [`Evidence::fields_compared`].
//! - A similarity `s` maps onto the line between the disagreement weight
//!   `D = log2((1 − m) / (1 − u))` and the agreement weight
//!   `A = log2(m / u)`, via [`component_bits`].
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

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::matcher::MatchBreakdown;
use crate::models::{Event, EventIdScheme};

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

/// Weigh one optional component.
fn add(total: &mut f64, compared: &mut u32, score: Option<f64>, w: ComponentWeights) {
    if let Some(s) = score {
        *total += component_bits(s, w.m, w.u);
        *compared += 1;
    }
}

/// Compute the evidence for a pair from its breakdown, the identifier
/// rule, and the already-decided `is_match`.
#[must_use]
pub fn compute(
    event1: &Event,
    event2: &Event,
    breakdown: &MatchBreakdown,
    is_match: bool,
    proof_bits: f64,
) -> Evidence {
    let mut bits = 0.0;
    let mut compared = 0u32;

    add(&mut bits, &mut compared, breakdown.name_score, NAME);
    add(
        &mut bits,
        &mut compared,
        breakdown.start_date_score,
        START_DATE,
    );
    add(&mut bits, &mut compared, breakdown.end_date_score, END_DATE);
    add(&mut bits, &mut compared, breakdown.location_score, LOCATION);
    add(&mut bits, &mut compared, breakdown.category_score, CATEGORY);
    add(
        &mut bits,
        &mut compared,
        breakdown.country_code_score,
        COUNTRY_CODE,
    );
    add(
        &mut bits,
        &mut compared,
        breakdown.organizer_score,
        ORGANIZER,
    );
    add(
        &mut bits,
        &mut compared,
        breakdown.performers_score,
        PERFORMERS,
    );
    add(&mut bits, &mut compared, breakdown.url_score, URL);
    add(
        &mut bits,
        &mut compared,
        breakdown.relationships_score,
        RELATIONSHIPS,
    );
    add(&mut bits, &mut compared, breakdown.tags_score, TAGS);

    // Identifiers: one shared value anywhere is IDENTIFIER_BITS once; each
    // scheme both sides hold with disjoint values is a contradiction.
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
                bits += CONTRADICTION_BITS;
            } else {
                shared = true;
            }
        }
    }
    if shared {
        bits += IDENTIFIER_BITS;
    }
    if shared || !contradictions.is_empty() {
        compared += 1;
    }

    let strength = if !contradictions.is_empty() {
        MatchStrength::Contradicted
    } else if bits >= proof_bits {
        MatchStrength::Proven
    } else if is_match {
        MatchStrength::Consistent
    } else {
        MatchStrength::NonMatch
    };

    Evidence {
        weight_bits: bits,
        fields_compared: compared,
        contradictions,
        strength,
    }
}

#[cfg(test)]
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
        // Every component agrees fully, so the weight is the sum of the
        // agreement weights: about 67.9 bits.
        let full: f64 = [
            NAME,
            START_DATE,
            END_DATE,
            LOCATION,
            CATEGORY,
            COUNTRY_CODE,
            ORGANIZER,
            PERFORMERS,
            URL,
            RELATIONSHIPS,
            TAGS,
        ]
        .iter()
        .map(|w| (w.m / w.u).log2())
        .sum();
        assert!((r.evidence.weight_bits - full).abs() < 1e-9);
        // Without the URL (the strongest non-identifier field) it still clears the bar.
        let mut a = rich();
        a.url = None;
        let r = engine().match_events(&a, &rich());
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
}
