#![warn(clippy::pedantic)]

//! Property-based tests.
//!
//! Each property generates many random inputs via `proptest` and checks an
//! invariant that should hold for **every** input. The point is to catch
//! the failure modes that example-based tests miss: weird Unicode in
//! names, edge-case dates, sparse / dense `Event` records.

use event_matcher::{
    Address, Confidence, Event, EventCategory, EventId, EventIdScheme, Location, MatchConfig,
    MatchingEngine, Normalizer, RelationKind, RelationshipRef,
};
use proptest::prelude::*;

// ---------- Strategies ----------

/// A reasonable name string for proptest: arbitrary Unicode constrained
/// to lengths we'd plausibly see in practice. Skips strings that
/// normalise to empty so `validate()` will still pass on builders that
/// only carry a name.
fn name_strategy() -> impl Strategy<Value = String> {
    "[\\PC]{1,40}".prop_filter("normalises to empty", |s| {
        !Normalizer::normalize_name(s).is_empty()
    })
}

/// An ISO 8601 datetime sampled from a wide but bounded year range.
fn iso8601_datetime_strategy() -> impl Strategy<Value = String> {
    (
        1900i32..=2100,
        1u32..=12,
        1u32..=28,
        0u32..=23,
        0u32..=59,
        0u32..=59,
    )
        .prop_map(|(y, m, d, h, mi, s)| format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z"))
}

/// A reasonable category for proptest.
fn category_strategy() -> impl Strategy<Value = EventCategory> {
    prop_oneof![
        Just(EventCategory::MusicEvent),
        Just(EventCategory::ComedyEvent),
        Just(EventCategory::Festival),
        Just(EventCategory::ConferenceEvent),
        Just(EventCategory::SportsEvent),
        Just(EventCategory::SocialEvent),
    ]
}

/// A short URL-like string for proptest (`url`, `virtual_url`).
fn url_strategy() -> impl Strategy<Value = String> {
    "[a-z]{1,8}".prop_map(|s| format!("https://example.org/{s}"))
}

/// A two-letter uppercase country-code-shaped string for proptest
/// (`country_code_as_iso_3166_1_alpha_2`). Not validated against the
/// real ISO 3166-1 list — see spec/10-open-questions.md OQ-B — just the
/// shape the field is documented to carry.
fn country_code_strategy() -> impl Strategy<Value = String> {
    "[A-Z]{2}"
}

/// A reasonable `EventIdScheme` for proptest, covering every named
/// variant plus the `Other(String)` catch-all.
fn event_id_scheme_strategy() -> impl Strategy<Value = EventIdScheme> {
    prop_oneof![
        Just(EventIdScheme::Wikidata),
        Just(EventIdScheme::Eventbrite),
        Just(EventIdScheme::Meetup),
        Just(EventIdScheme::Ticketmaster),
        Just(EventIdScheme::Songkick),
        Just(EventIdScheme::Bandsintown),
        Just(EventIdScheme::Facebook),
        Just(EventIdScheme::Luma),
        Just(EventIdScheme::GoogleCalendar),
        Just(EventIdScheme::ICalendarUid),
        "[a-z]{1,8}".prop_map(EventIdScheme::Other),
    ]
}

/// A well-formed `EventId` for proptest — `EventId::new` only rejects an
/// empty-after-trim value, so a short alphanumeric value always builds.
fn event_id_strategy() -> impl Strategy<Value = EventId> {
    (event_id_scheme_strategy(), "[a-zA-Z0-9]{1,12}")
        .prop_map(|(scheme, value)| EventId::new(scheme, value).expect("non-empty value"))
}

/// A postal address for proptest, each sub-field independently present
/// or absent.
fn address_strategy() -> impl Strategy<Value = Address> {
    (
        prop::option::of(name_strategy()),
        prop::option::of(name_strategy()),
        prop::option::of(name_strategy()),
        prop::option::of(name_strategy()),
        prop::option::of(name_strategy()),
        prop::option::of(country_code_strategy()),
    )
        .prop_map(|(line1, line2, city, county, postcode, country)| {
            let mut a = Address::new();
            if let Some(v) = line1 {
                a = a.with_line1(v);
            }
            if let Some(v) = line2 {
                a = a.with_line2(v);
            }
            if let Some(v) = city {
                a = a.with_city(v);
            }
            if let Some(v) = county {
                a = a.with_county(v);
            }
            if let Some(v) = postcode {
                a = a.with_postcode(v);
            }
            if let Some(v) = country {
                a = a.with_country(v);
            }
            a
        })
}

/// A `Location` for proptest, each sub-field independently present or
/// absent (including a nested `Address`).
fn location_strategy() -> impl Strategy<Value = Location> {
    (
        prop::option::of(name_strategy()),
        prop::option::of(address_strategy()),
        prop::option::of(-90.0f64..=90.0),
        prop::option::of(-180.0f64..=180.0),
        prop::option::of(url_strategy()),
    )
        .prop_map(|(venue, addr, lat, lon, virtual_url)| {
            let mut loc = Location::new();
            if let Some(v) = venue {
                loc = loc.with_venue_name(v);
            }
            if let Some(v) = addr {
                loc = loc.with_address(v);
            }
            if let Some(v) = lat {
                loc = loc.with_latitude(v);
            }
            if let Some(v) = lon {
                loc = loc.with_longitude(v);
            }
            if let Some(v) = virtual_url {
                loc = loc.with_virtual_url(v);
            }
            loc
        })
}

/// A `RelationKind` for proptest, covering every variant.
fn relation_kind_strategy() -> impl Strategy<Value = RelationKind> {
    prop_oneof![
        Just(RelationKind::Outer),
        Just(RelationKind::Inner),
        Just(RelationKind::ImmediatelyBefore),
        Just(RelationKind::ImmediatelyAfter),
    ]
}

/// A well-formed `RelationshipRef` for proptest — `RelationshipRef::new`
/// only rejects an empty-after-trim `event_id`, so a short alphanumeric
/// id always builds.
fn relationship_strategy() -> impl Strategy<Value = RelationshipRef> {
    (relation_kind_strategy(), "[a-zA-Z0-9]{1,12}")
        .prop_map(|(relation, id)| RelationshipRef::new(relation, id).expect("non-empty event_id"))
}

/// An `Event` carrying enough data to make `validate()` pass and the
/// matching engine produce a non-trivial score. Populates every
/// scoreable field (spec/10-open-questions.md T-1), not just the five
/// the original strategy covered.
fn event_strategy() -> impl Strategy<Value = Event> {
    (
        (
            name_strategy(),
            prop::collection::vec(name_strategy(), 0..3),
            prop::option::of(iso8601_datetime_strategy()),
            prop::option::of(iso8601_datetime_strategy()),
            prop::option::of(category_strategy()),
        ),
        (
            prop::option::of(location_strategy()),
            prop::collection::vec(event_id_strategy(), 0..3),
            prop::option::of(name_strategy()),
            prop::collection::vec(name_strategy(), 0..3),
        ),
        (
            prop::option::of(url_strategy()),
            prop::option::of(country_code_strategy()),
            prop::collection::vec(relationship_strategy(), 0..3),
            prop::collection::vec(name_strategy(), 0..3),
        ),
    )
        .prop_map(|(core, place, extra)| {
            let (name, alts, start, end, cat) = core;
            let (location, event_ids, organizer, performers) = place;
            let (url, country_code, relationships, tags) = extra;
            let mut b = Event::builder()
                .name(name)
                .alternate_names(alts)
                .event_ids(event_ids)
                .performers(performers)
                .relationships(relationships)
                .tags(tags);
            if let Some(s) = start {
                b = b.start_date(s);
            }
            if let Some(e) = end {
                b = b.end_date(e);
            }
            if let Some(c) = cat {
                b = b.category(c);
            }
            if let Some(l) = location {
                b = b.location(l);
            }
            if let Some(o) = organizer {
                b = b.organizer(o);
            }
            if let Some(u) = url {
                b = b.url(u);
            }
            if let Some(cc) = country_code {
                b = b.country_code_as_iso_3166_1_alpha_2(cc);
            }
            b.build()
        })
}

/// True unless `loc` is `Some` but every sub-field it (and any nested
/// `Address`) carries is `None` — a "present but fully vacuous"
/// `Location`. Per spec §6.4 (and the same rule one level down for
/// `Address`), two such locations score the documented neutral `0.5`
/// rather than being skipped as absent evidence, which can pull a
/// self-match below `High` confidence for a reason unrelated to
/// matching correctness. See spec/10-open-questions.md OQ-J.
fn location_is_meaningful(loc: Option<&Location>) -> bool {
    // `compare_addresses` scores only postcode/city/line1 (spec §6.4);
    // `line2`/`county`/`country` contribute nothing to it.
    let address_scores_something =
        |a: &Address| a.line1.is_some() || a.city.is_some() || a.postcode.is_some();
    match loc {
        None => true,
        Some(l) => {
            // A nested `Address` present but scoring nothing is itself
            // vacuous-but-present: `compare_addresses` returns a neutral
            // 0.5 for it, which — because `compare_locations` counts any
            // *present* address as contributing weight regardless of its
            // own content — drags the location average down even when a
            // sibling field (e.g. `virtual_url`) genuinely self-matches.
            // So a present-but-vacuous address disqualifies the whole
            // location, not just its own sub-score.
            let address_ok = l.address.as_ref().is_none_or(address_scores_something);
            let has_signal = l.venue_name.is_some()
                // Coordinates only fire in `compare_locations` when BOTH
                // latitude AND longitude are present — a lone latitude
                // (or longitude) contributes nothing.
                || (l.latitude_as_decimal_degrees.is_some()
                    && l.longitude_as_decimal_degrees.is_some())
                || l.virtual_url.is_some()
                || l.address.as_ref().is_some_and(address_scores_something);
            address_ok && has_signal
        }
    }
}

// ---------- Properties ----------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 500,
        .. ProptestConfig::default()
    })]

    /// `normalize_name` MUST be idempotent.
    #[test]
    fn normalize_name_is_idempotent(s in "\\PC{0,80}") {
        let once = Normalizer::normalize_name(&s);
        let twice = Normalizer::normalize_name(&once);
        prop_assert_eq!(once, twice);
    }

    /// `normalize_name` MUST always be lowercase and whitespace-trimmed.
    #[test]
    fn normalize_name_has_no_uppercase_or_leading_whitespace(s in "\\PC{0,80}") {
        let n = Normalizer::normalize_name(&s);
        prop_assert!(!n.chars().any(|c| c.is_ascii_uppercase()));
        prop_assert!(!n.starts_with(' '));
        prop_assert!(!n.ends_with(' '));
    }

    /// Probabilistic `score` MUST always land in `[0.0, 1.0]`.
    #[test]
    fn score_is_bounded_unit_interval(p1 in event_strategy(), p2 in event_strategy()) {
        let engine = MatchingEngine::default_config();
        let r = engine.match_events(&p1, &p2);
        prop_assert!(r.score >= 0.0, "score < 0.0: {}", r.score);
        prop_assert!(r.score <= 1.0, "score > 1.0: {}", r.score);
    }

    /// Self-match MUST produce `is_match == true` for any validating `Event`.
    #[test]
    fn self_match_is_true(p in event_strategy()) {
        prop_assume!(p.validate().is_ok());
        let r = MatchingEngine::default_config().match_events(&p, &p);
        prop_assert!(r.is_match, "self-match failed for {:?}: score={}", p, r.score);
    }

    /// Self-match MUST yield `High` confidence.
    ///
    /// Excludes the one input class (spec/10-open-questions.md OQ-J)
    /// where the invariant provably does not hold *by the crate's own
    /// documented design*, not by a bug this test should catch: a
    /// `Location` present on both sides but carrying no comparable
    /// sub-field (see [`location_is_meaningful`]) scores the spec §6.4
    /// neutral `0.5` rather than being skipped, which can pull a
    /// self-match below `High`. This is a narrower exclusion than
    /// dropping the property — every other input, including a `None`
    /// location or one with any single field set, still exercises it.
    #[test]
    fn self_match_confidence_is_high(p in event_strategy().prop_filter(
        "present-but-fully-vacuous Location scores a documented neutral \
         0.5 per spec §6.4, not None — OQ-J",
        |p| location_is_meaningful(p.location.as_ref()),
    )) {
        prop_assume!(p.validate().is_ok());
        let r = MatchingEngine::default_config().match_events(&p, &p);
        prop_assert_eq!(r.confidence, Confidence::High);
    }

    /// `match_events` MUST be symmetric.
    #[test]
    fn matching_is_symmetric(p1 in event_strategy(), p2 in event_strategy()) {
        let engine = MatchingEngine::default_config();
        let forward = engine.match_events(&p1, &p2);
        let reverse = engine.match_events(&p2, &p1);
        prop_assert!(
            (forward.score - reverse.score).abs() < 1e-9,
            "score asymmetric: {} vs {}",
            forward.score,
            reverse.score
        );
        prop_assert_eq!(forward.is_match, reverse.is_match);
        prop_assert_eq!(forward.confidence, reverse.confidence);
    }

    /// `deterministic_match` MUST also be symmetric.
    #[test]
    fn deterministic_match_is_symmetric(p1 in event_strategy(), p2 in event_strategy()) {
        let engine = MatchingEngine::default_config();
        prop_assert_eq!(
            engine.deterministic_match(&p1, &p2),
            engine.deterministic_match(&p2, &p1)
        );
    }

    /// `MatchConfig` MUST survive a JSON round-trip without value drift.
    #[test]
    fn match_config_default_round_trips_through_json(_ignored in any::<u8>()) {
        let original = MatchConfig::default();
        let json = serde_json::to_string(&original).expect("serialise");
        let back: MatchConfig = serde_json::from_str(&json).expect("deserialise");
        prop_assert!((original.match_threshold - back.match_threshold).abs() < 1e-12);
        // Every `MatchConfig` weight field (spec/10-open-questions.md T-2)
        // — not just the 3 the original assertion happened to cover.
        prop_assert!((original.name_weight - back.name_weight).abs() < 1e-12);
        prop_assert!((original.start_date_weight - back.start_date_weight).abs() < 1e-12);
        prop_assert!((original.end_date_weight - back.end_date_weight).abs() < 1e-12);
        prop_assert!((original.location_weight - back.location_weight).abs() < 1e-12);
        prop_assert!((original.category_weight - back.category_weight).abs() < 1e-12);
        prop_assert!((original.country_code_weight - back.country_code_weight).abs() < 1e-12);
        prop_assert!((original.event_ids_weight - back.event_ids_weight).abs() < 1e-12);
        prop_assert!((original.organizer_weight - back.organizer_weight).abs() < 1e-12);
        prop_assert!((original.performers_weight - back.performers_weight).abs() < 1e-12);
        prop_assert!((original.url_weight - back.url_weight).abs() < 1e-12);
        prop_assert!(
            (original.relationships_weight - back.relationships_weight).abs() < 1e-12
        );
        prop_assert!((original.tags_weight - back.tags_weight).abs() < 1e-12);
        prop_assert_eq!(original.strict_mode, back.strict_mode);
    }

    /// `Event` MUST survive a JSON round-trip.
    #[test]
    fn event_round_trips_through_json(p in event_strategy()) {
        let json = serde_json::to_string(&p).expect("serialise");
        let back: Event = serde_json::from_str(&json).expect("deserialise");
        prop_assert_eq!(&p.name, &back.name);
        prop_assert_eq!(&p.alternate_names, &back.alternate_names);
        prop_assert_eq!(&p.start_date, &back.start_date);
        prop_assert_eq!(&p.end_date, &back.end_date);
        prop_assert_eq!(&p.category, &back.category);
    }

    /// `start_date` sub-score MUST be commutative.
    #[test]
    fn start_date_subscore_is_symmetric(
        s1 in iso8601_datetime_strategy(),
        s2 in iso8601_datetime_strategy(),
    ) {
        let engine = MatchingEngine::default_config();
        let e1 = Event::builder().name("X").start_date(&s1).build();
        let e2 = Event::builder().name("X").start_date(&s2).build();
        let forward = engine.match_events(&e1, &e2).breakdown.start_date_score;
        let reverse = engine.match_events(&e2, &e1).breakdown.start_date_score;
        match (forward, reverse) {
            (Some(a), Some(b)) => prop_assert!((a - b).abs() < 1e-9),
            (None, None) => {}
            _ => prop_assert!(false, "asymmetric None"),
        }
    }

    /// spec/10-open-questions.md T-3: `relationships_score` MUST be
    /// bounded in `[0.0, 1.0]` when present, and symmetric.
    #[test]
    fn relationships_score_is_bounded_and_symmetric(
        a in prop::collection::vec(relationship_strategy(), 0..4),
        b in prop::collection::vec(relationship_strategy(), 0..4),
    ) {
        let engine = MatchingEngine::default_config();
        let e1 = Event::builder().name("X").relationships(a).build();
        let e2 = Event::builder().name("X").relationships(b).build();
        let forward = engine.match_events(&e1, &e2).breakdown.relationships_score;
        let reverse = engine.match_events(&e2, &e1).breakdown.relationships_score;
        match (forward, reverse) {
            (Some(f), Some(r)) => {
                prop_assert!((0.0..=1.0).contains(&f), "forward {f} out of range");
                prop_assert!((0.0..=1.0).contains(&r), "reverse {r} out of range");
                prop_assert!((f - r).abs() < 1e-9, "asymmetric: {f} vs {r}");
            }
            (None, None) => {}
            _ => prop_assert!(false, "asymmetric None"),
        }
    }

    /// T-3: identical non-empty `relationships` sets score exactly `1.0`.
    #[test]
    fn relationships_score_identical_sets_is_one(
        a in prop::collection::vec(relationship_strategy(), 1..4),
    ) {
        let engine = MatchingEngine::default_config();
        let e1 = Event::builder().name("X").relationships(a.clone()).build();
        let e2 = Event::builder().name("X").relationships(a).build();
        let score = engine.match_events(&e1, &e2).breakdown.relationships_score;
        prop_assert_eq!(score, Some(1.0));
    }

    /// T-3: disjoint non-empty `relationships` sets score exactly `0.0`.
    /// The `a-`/`b-` id prefixes guarantee disjointness deterministically
    /// rather than relying on two independently-sampled sets happening
    /// not to collide.
    #[test]
    fn relationships_score_disjoint_sets_is_zero(
        a in prop::collection::vec(relationship_strategy(), 1..4),
        b in prop::collection::vec(relationship_strategy(), 1..4),
    ) {
        let disjoint = |prefix: &str, refs: Vec<RelationshipRef>| -> Vec<RelationshipRef> {
            refs.into_iter()
                .map(|r| RelationshipRef::new(r.relation, format!("{prefix}{}", r.event_id)).unwrap())
                .collect()
        };
        let engine = MatchingEngine::default_config();
        let e1 = Event::builder().name("X").relationships(disjoint("a-", a)).build();
        let e2 = Event::builder().name("X").relationships(disjoint("b-", b)).build();
        let score = engine.match_events(&e1, &e2).breakdown.relationships_score;
        prop_assert_eq!(score, Some(0.0));
    }

    /// T-3: `relationships_score` is `None` when either side has no
    /// relationships recorded at all.
    #[test]
    fn relationships_score_either_side_empty_is_none(
        a in prop::collection::vec(relationship_strategy(), 1..4),
    ) {
        let engine = MatchingEngine::default_config();
        let e1 = Event::builder().name("X").relationships(a).build();
        let e2 = Event::builder().name("X").build();
        prop_assert_eq!(
            engine.match_events(&e1, &e2).breakdown.relationships_score,
            None
        );
    }

    /// T-3: `tags_score` MUST be bounded in `[0.0, 1.0]` when present,
    /// and symmetric.
    #[test]
    fn tags_score_is_bounded_and_symmetric(
        a in prop::collection::vec(name_strategy(), 0..4),
        b in prop::collection::vec(name_strategy(), 0..4),
    ) {
        let engine = MatchingEngine::default_config();
        let e1 = Event::builder().name("X").tags(a).build();
        let e2 = Event::builder().name("X").tags(b).build();
        let forward = engine.match_events(&e1, &e2).breakdown.tags_score;
        let reverse = engine.match_events(&e2, &e1).breakdown.tags_score;
        match (forward, reverse) {
            (Some(f), Some(r)) => {
                prop_assert!((0.0..=1.0).contains(&f), "forward {f} out of range");
                prop_assert!((0.0..=1.0).contains(&r), "reverse {r} out of range");
                prop_assert!((f - r).abs() < 1e-9, "asymmetric: {f} vs {r}");
            }
            (None, None) => {}
            _ => prop_assert!(false, "asymmetric None"),
        }
    }

    /// T-3: identical non-empty `tags` sets score exactly `1.0`.
    #[test]
    fn tags_score_identical_sets_is_one(a in prop::collection::vec(name_strategy(), 1..4)) {
        let engine = MatchingEngine::default_config();
        let e1 = Event::builder().name("X").tags(a.clone()).build();
        let e2 = Event::builder().name("X").tags(a).build();
        let score = engine.match_events(&e1, &e2).breakdown.tags_score;
        prop_assert_eq!(score, Some(1.0));
    }

    /// T-3: disjoint non-empty `tags` sets score exactly `0.0`. The
    /// `a-`/`b-` prefixes guarantee disjointness deterministically (tags
    /// are compared case-insensitively, but the prefix still separates
    /// the sets since neither side's tags share it).
    #[test]
    fn tags_score_disjoint_sets_is_zero(
        a in prop::collection::vec(name_strategy(), 1..4),
        b in prop::collection::vec(name_strategy(), 1..4),
    ) {
        let prefixed = |prefix: &str, tags: Vec<String>| -> Vec<String> {
            tags.into_iter().map(|t| format!("{prefix}{t}")).collect()
        };
        let engine = MatchingEngine::default_config();
        let e1 = Event::builder().name("X").tags(prefixed("a-", a)).build();
        let e2 = Event::builder().name("X").tags(prefixed("b-", b)).build();
        let score = engine.match_events(&e1, &e2).breakdown.tags_score;
        prop_assert_eq!(score, Some(0.0));
    }

    /// T-3: `tags_score` is `None` when either side has no tags recorded
    /// at all.
    #[test]
    fn tags_score_either_side_empty_is_none(a in prop::collection::vec(name_strategy(), 1..4)) {
        let engine = MatchingEngine::default_config();
        let e1 = Event::builder().name("X").tags(a).build();
        let e2 = Event::builder().name("X").build();
        prop_assert_eq!(engine.match_events(&e1, &e2).breakdown.tags_score, None);
    }

    /// `parse_iso8601_unix_seconds` MUST be idempotent under
    /// canonicalisation: parsing back the string form of a Unix-second
    /// timestamp must return the same number.
    ///
    /// Sampled within a bounded range to keep the test fast.
    #[test]
    fn parse_iso8601_round_trip(s in iso8601_datetime_strategy()) {
        let parsed = Normalizer::parse_iso8601_unix_seconds(&s);
        prop_assert!(parsed.is_some(), "failed to parse {s}");
    }

    /// `Confidence::from_score` MUST be monotonic.
    #[test]
    fn confidence_is_monotonic(a in 0.0f64..=1.0, b in 0.0f64..=1.0) {
        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
        let rank = |c: Confidence| match c {
            Confidence::Low => 0u8,
            Confidence::Medium => 1,
            Confidence::High => 2,
        };
        let ra = rank(Confidence::from_score(lo));
        let rb = rank(Confidence::from_score(hi));
        prop_assert!(
            rb >= ra,
            "score {} -> {:?}, score {} -> {:?}",
            lo,
            Confidence::from_score(lo),
            hi,
            Confidence::from_score(hi)
        );
    }
}
