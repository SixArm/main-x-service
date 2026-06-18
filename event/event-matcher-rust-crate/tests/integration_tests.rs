#![warn(clippy::pedantic)]

//! Integration tests for Event matcher.
//!
//! These exercise the **public API** as a downstream user would — building
//! [`Event`] records, constructing a [`MatchingEngine`], and inspecting
//! the [`event_matcher::MatchResult`] / [`event_matcher::MatchBreakdown`].
//!
//! Naming follows the AGENTS guide: `test_<thing>_<expected>`.

use event_matcher::{
    Address, Confidence, Event, EventCategory, EventId, EventIdScheme, Location, MatchConfig,
    MatchingEngine, Normalizer, Scorer, SimilarityAlgorithm,
};

// ============================================================
// Perfect / near-perfect matches
// ============================================================

#[test]
fn test_perfect_match_all_fields() {
    let loc = Location::new()
        .with_venue_name("Worthy Farm")
        .with_address(
            Address::new()
                .with_line1("Worthy Farm")
                .with_city("Pilton")
                .with_postcode("BA4 4BY"),
        )
        .with_latitude(51.150_3)
        .with_longitude(-2.586_2);
    let p1 = Event::builder()
        .name("Glastonbury Festival 2024")
        .add_alternate_name("Glasto 2024")
        .start_date("2024-06-26T09:00:00Z")
        .end_date("2024-06-30T23:59:00Z")
        .category(EventCategory::Festival)
        .country_code_as_iso_3166_1_alpha_2("GB")
        .location(loc)
        .build();
    let p2 = p1.clone();

    let result = MatchingEngine::default_config().match_events(&p1, &p2);
    assert!(result.is_match);
    assert!(result.score > 0.95);
    assert!(result.breakdown.name_score.unwrap() > 0.99);
    assert_eq!(result.breakdown.start_date_score, Some(1.0));
    assert_eq!(result.breakdown.category_score, Some(1.0));
    assert_eq!(result.breakdown.country_code_score, Some(1.0));
}

#[test]
fn test_identical_minimal_record_scores_one() {
    let p = Event::builder().name("RustConf 2024").build();
    let r = MatchingEngine::default_config().match_events(&p, &p.clone());
    assert!(r.is_match);
    assert!(r.score > 0.99);
}

// ============================================================
// Name + alternate-name matching (best of cartesian product)
// ============================================================

#[test]
fn test_name_match_picks_best_across_alternates() {
    let p1 = Event::builder().name("RustConf 2024").build();
    let p2 = Event::builder()
        .name("Rust Conference 2024")
        .add_alternate_name("RustConf 2024")
        .build();
    let r = MatchingEngine::default_config().match_events(&p1, &p2);
    let s = r.breakdown.name_score.expect("scored");
    assert!(
        s > 0.99,
        "expected best-of cartesian product to pick exact, got {s}"
    );
}

#[test]
fn test_name_match_returns_none_if_either_side_empty() {
    let p1 = Event::builder().build();
    let p2 = Event::builder().name("RustConf").build();
    let r = MatchingEngine::default_config().match_events(&p1, &p2);
    assert!(r.breakdown.name_score.is_none());
}

// ============================================================
// Start-date scoring
// ============================================================

#[test]
fn test_start_date_score_identical_instants() {
    let p1 = Event::builder()
        .name("X")
        .start_date("2024-06-26T09:00:00Z")
        .build();
    let p2 = p1.clone();
    let r = MatchingEngine::default_config().match_events(&p1, &p2);
    assert!((r.breakdown.start_date_score.unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn test_start_date_score_far_apart_decays_to_zero() {
    let p1 = Event::builder()
        .name("X")
        .start_date("2024-01-01T00:00:00Z")
        .build();
    let p2 = Event::builder()
        .name("X")
        .start_date("2024-06-01T00:00:00Z")
        .build();
    let r = MatchingEngine::default_config().match_events(&p1, &p2);
    assert!(r.breakdown.start_date_score.unwrap() < 1e-3);
}

#[test]
fn test_start_date_score_none_when_unparseable() {
    let p1 = Event::builder().name("X").start_date("not-a-date").build();
    let p2 = Event::builder()
        .name("X")
        .start_date("2024-06-26T09:00:00Z")
        .build();
    let r = MatchingEngine::default_config().match_events(&p1, &p2);
    assert!(r.breakdown.start_date_score.is_none());
}

#[test]
fn test_start_date_score_accepts_equivalent_offsets() {
    // 09:00 UTC == 11:00 +02:00 — both denote the same instant.
    let p1 = Event::builder()
        .name("X")
        .start_date("2024-06-26T09:00:00Z")
        .build();
    let p2 = Event::builder()
        .name("X")
        .start_date("2024-06-26T11:00:00+02:00")
        .build();
    let r = MatchingEngine::default_config().match_events(&p1, &p2);
    assert!((r.breakdown.start_date_score.unwrap() - 1.0).abs() < 1e-9);
}

// ============================================================
// Category equality
// ============================================================

#[test]
fn test_category_match_equal_scores_one() {
    let p1 = Event::builder()
        .name("X")
        .category(EventCategory::MusicEvent)
        .build();
    let p2 = Event::builder()
        .name("X")
        .category(EventCategory::MusicEvent)
        .build();
    let r = MatchingEngine::default_config().match_events(&p1, &p2);
    assert_eq!(r.breakdown.category_score, Some(1.0));
}

#[test]
fn test_category_mismatch_other_strings_scores_zero() {
    let p1 = Event::builder()
        .name("X")
        .category(EventCategory::Other("foo".into()))
        .build();
    let p2 = Event::builder()
        .name("X")
        .category(EventCategory::Other("bar".into()))
        .build();
    let r = MatchingEngine::default_config().match_events(&p1, &p2);
    assert_eq!(r.breakdown.category_score, Some(0.0));
}

// ============================================================
// Event IDs
// ============================================================

#[test]
fn test_event_id_shared_scores_one_and_is_deterministic_match() {
    let id = EventId::new(EventIdScheme::Eventbrite, "12345").unwrap();
    let p1 = Event::builder()
        .name("RustConf")
        .add_event_id(id.clone())
        .build();
    let p2 = Event::builder()
        .name("Wholly Different")
        .add_event_id(id)
        .build();
    let engine = MatchingEngine::default_config();
    let r = engine.match_events(&p1, &p2);
    assert_eq!(r.breakdown.event_ids_score, Some(1.0));
    assert!(engine.deterministic_match(&p1, &p2));
}

#[test]
fn test_event_id_distinct_schemes_do_not_match() {
    let a = Event::builder()
        .name("X")
        .add_event_id(EventId::new(EventIdScheme::Eventbrite, "X").unwrap())
        .build();
    let b = Event::builder()
        .name("X")
        .add_event_id(EventId::new(EventIdScheme::Meetup, "X").unwrap())
        .build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    assert_eq!(r.breakdown.event_ids_score, Some(0.0));
}

// ============================================================
// Location scoring
// ============================================================

#[test]
fn test_location_postcode_match_dominates_address_subscore() {
    let l1 = Location::new().with_address(Address::new().with_postcode("BA4 4BY"));
    let l2 = Location::new().with_address(Address::new().with_postcode("BA4 4BY"));
    let a = Event::builder().name("X").location(l1).build();
    let b = Event::builder().name("X").location(l2).build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    assert!((r.breakdown.location_score.unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn test_location_score_none_when_one_side_has_no_location() {
    let a = Event::builder()
        .name("X")
        .location(Location::new().with_venue_name("Worthy Farm"))
        .build();
    let b = Event::builder().name("X").build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    assert!(r.breakdown.location_score.is_none());
}

// ============================================================
// Organiser / performers / url
// ============================================================

#[test]
fn test_organizer_match_after_normalisation() {
    let a = Event::builder()
        .name("X")
        .organizer("Rust Foundation")
        .build();
    let b = Event::builder()
        .name("X")
        .organizer("RUST  Foundation")
        .build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    assert!(r.breakdown.organizer_score.unwrap() > 0.99);
}

#[test]
fn test_performers_match_picks_best_across_cartesian_product() {
    let a = Event::builder()
        .name("X")
        .add_performer("The Rolling Stones")
        .add_performer("Coldplay")
        .build();
    let b = Event::builder()
        .name("X")
        .add_performer("Coldplay")
        .add_performer("Radiohead")
        .build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    assert!(r.breakdown.performers_score.unwrap() > 0.99);
}

#[test]
fn test_url_match_is_exact_after_trim() {
    let a = Event::builder()
        .name("X")
        .url("https://rustconf.com")
        .build();
    let b = Event::builder()
        .name("X")
        .url("  https://rustconf.com  ")
        .build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    assert_eq!(r.breakdown.url_score, Some(1.0));
}

// ============================================================
// Deterministic match
// ============================================================

#[test]
fn test_deterministic_via_name_and_start_date() {
    let a = Event::builder()
        .name("RustConf 2024")
        .start_date("2024-09-10T09:00:00Z")
        .build();
    let b = Event::builder()
        .name("RustConf 2024")
        .start_date("2024-09-10T09:00:00Z")
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
}

#[test]
fn test_deterministic_rejects_when_start_date_missing() {
    let a = Event::builder().name("RustConf 2024").build();
    let b = Event::builder().name("RustConf 2024").build();
    assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
}

// ============================================================
// Strict mode
// ============================================================

#[test]
fn test_strict_mode_requires_deterministic_signal() {
    let engine = MatchingEngine::new(MatchConfig {
        match_threshold: 0.50,
        strict_mode: true,
        ..MatchConfig::default()
    });
    let a = Event::builder()
        .name("Cafe Centrale Concert")
        .start_date("2024-09-10T09:00:00Z")
        .build();
    let b = Event::builder()
        .name("Cafe Central Concert")
        .start_date("2024-09-10T09:00:00Z")
        .build();
    let r = engine.match_events(&a, &b);
    assert!(r.score >= 0.50);
    assert!(!engine.deterministic_match(&a, &b));
    assert!(!r.is_match);
}

// ============================================================
// Batch APIs
// ============================================================

#[test]
fn test_match_one_to_many_preserves_order() {
    let engine = MatchingEngine::default_config();
    let q = Event::builder().name("RustConf").build();
    let cs = vec![
        Event::builder().name("PyConf").build(),
        Event::builder().name("RustConf").build(),
        Event::builder().name("GoConf").build(),
    ];
    let r = engine.match_one_to_many(&q, &cs);
    assert_eq!(r.len(), 3);
    assert!(r[1].is_match);
}

#[test]
fn test_rank_one_to_many_sorts_by_score_descending() {
    let engine = MatchingEngine::default_config();
    let q = Event::builder().name("RustConf 2024").build();
    let cs = vec![
        Event::builder().name("PyConf 2024").build(),
        q.clone(),
        Event::builder().name("GoConf 2024").build(),
    ];
    let ranked = engine.rank_one_to_many(&q, &cs);
    assert_eq!(ranked[0].0, 1);
    for w in ranked.windows(2) {
        assert!(w[0].1.score >= w[1].1.score);
    }
}

// ============================================================
// Confidence
// ============================================================

#[test]
fn test_confidence_bands_are_inclusive_on_low_side() {
    assert_eq!(Confidence::from_score(0.90), Confidence::High);
    assert_eq!(Confidence::from_score(0.75), Confidence::Medium);
    assert_eq!(Confidence::from_score(0.0), Confidence::Low);
}

// ============================================================
// Scorer / Normalizer surface
// ============================================================

#[test]
fn test_scorer_combined_similarity_in_unit_interval() {
    let s = Scorer::combined_similarity("Stephen", "Steven");
    assert!((0.0..=1.0).contains(&s));
    assert!(s > 0.80);
}

#[test]
fn test_normalizer_parse_iso8601_round_trips_through_seconds() {
    let s = Normalizer::parse_iso8601_unix_seconds("2024-06-26T09:00:00Z").unwrap();
    assert_eq!(s, 1_719_392_400);
}

#[test]
fn test_similarity_algorithm_exact_round_trips_through_json() {
    let json = serde_json::to_string(&SimilarityAlgorithm::Exact).unwrap();
    let back: SimilarityAlgorithm = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, SimilarityAlgorithm::Exact));
}

// ============================================================
// Location — coordinates sub-score (§6.4) and neutral fallback (§3.3/§6.4)
// ============================================================

#[test]
fn test_location_coordinates_close_scores_high() {
    // Two coordinates ~30 m apart at the default scale of 100 m: the
    // Gaussian decay keeps the location score well above 0.9.
    let l1 = Location::new().with_latitude(51.150_30).with_longitude(-2.586_20);
    let l2 = Location::new().with_latitude(51.150_55).with_longitude(-2.586_20);
    let a = Event::builder().name("X").location(l1).build();
    let b = Event::builder().name("X").location(l2).build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    let s = r.breakdown.location_score.expect("both sides have coordinates");
    assert!(s > 0.9, "expected close coordinates to score high, got {s}");
}

#[test]
fn test_location_coordinates_far_apart_scores_low() {
    // London vs Paris: hundreds of km apart, far beyond the 100 m scale.
    let l1 = Location::new().with_latitude(51.507_22).with_longitude(-0.127_5);
    let l2 = Location::new().with_latitude(48.853_0).with_longitude(2.349_2);
    let a = Event::builder().name("X").location(l1).build();
    let b = Event::builder().name("X").location(l2).build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    let s = r.breakdown.location_score.expect("both sides have coordinates");
    assert!(s < 1e-3, "expected far coordinates to decay to ~0, got {s}");
}

#[test]
fn test_location_no_shared_subcomponent_is_neutral_half() {
    // Both sides carry a Location, but they share no populated sub-component:
    // one has only coordinates, the other only a venue name. §6.4 specifies a
    // neutral 0.5 fallback rather than None (location_score is None only when a
    // side has no Location at all).
    let l1 = Location::new().with_latitude(51.150_3).with_longitude(-2.586_2);
    let l2 = Location::new().with_venue_name("Worthy Farm");
    let a = Event::builder().name("X").location(l1).build();
    let b = Event::builder().name("X").location(l2).build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    assert_eq!(r.breakdown.location_score, Some(0.5));
}

#[test]
fn test_location_both_empty_is_neutral_half() {
    // Both sides carry a present-but-empty Location: neutral 0.5, not None.
    let a = Event::builder().name("X").location(Location::new()).build();
    let b = Event::builder().name("X").location(Location::new()).build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    assert_eq!(r.breakdown.location_score, Some(0.5));
}

// ============================================================
// Location — address line-1 house-number-weighted blend (§6.4 / §4.5)
// ============================================================

#[test]
fn test_location_line1_same_street_different_house_number_below_perfect() {
    // Same street, different house number: with house numbers present on both
    // sides §6.4 blends 0.6 * jaro_winkler(street) + 0.4 * (house_number_eq).
    // Equal streets (jw = 1.0) but unequal house numbers => 0.6, dragging the
    // line-1-only address sub-score (and hence the location score) below 1.0.
    let l1 = Location::new().with_address(Address::new().with_line1("10 Main Street"));
    let l2 = Location::new().with_address(Address::new().with_line1("22 Main Street"));
    let a = Event::builder().name("X").location(l1).build();
    let b = Event::builder().name("X").location(l2).build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    let s = r.breakdown.location_score.expect("both sides have a line1");
    assert!(s < 0.99, "house-number mismatch should lower line1 blend, got {s}");
    assert!(s > 0.3, "matching street should keep the blend well above zero, got {s}");
}

#[test]
fn test_location_line1_same_house_number_beats_different() {
    // The same-house-number pair must score strictly higher than the
    // different-house-number pair on an otherwise identical street, pinning the
    // 0.4 house-number term direction.
    let street_a = Location::new().with_address(Address::new().with_line1("10 Main Street"));
    let street_b_same =
        Location::new().with_address(Address::new().with_line1("10 Main Street"));
    let street_b_diff =
        Location::new().with_address(Address::new().with_line1("22 Main Street"));
    let a = Event::builder().name("X").location(street_a.clone()).build();
    let b_same = Event::builder().name("X").location(street_b_same).build();
    let b_diff = Event::builder().name("X").location(street_b_diff).build();
    let engine = MatchingEngine::default_config();
    let same = engine.match_events(&a, &b_same).breakdown.location_score.unwrap();
    let diff = engine.match_events(&a, &b_diff).breakdown.location_score.unwrap();
    assert!(same > diff, "same house number should beat different: {same} vs {diff}");
}

// ============================================================
// Phonetic bonus direction (§6.2 / §5.2.2)
// ============================================================

#[test]
fn test_phonetic_bonus_nudges_up_and_never_down() {
    // A borderline name pair that is phonetically equal (Soundex match) but not
    // textually identical. Per §6.2 the bonus is gated > 0.9 and MUST NOT lower
    // the score: turning phonetic matching on must not decrease the overall
    // score, and where the bonus fires it must increase it.
    let a = Event::builder().name("Catherine Festival").build();
    let b = Event::builder().name("Katharine Festival").build();

    let off = MatchConfig {
        use_phonetic_matching: false,
        ..MatchConfig::default()
    };
    let on = MatchConfig {
        use_phonetic_matching: true,
        ..MatchConfig::default()
    };
    let r_off = MatchingEngine::new(off).match_events(&a, &b);
    let r_on = MatchingEngine::new(on).match_events(&a, &b);

    assert!(r_off.breakdown.name_phonetic_score.is_none());
    assert!(
        r_on.score >= r_off.score - 1e-12,
        "phonetic bonus must never lower the score: on={} off={}",
        r_on.score,
        r_off.score
    );
    // When the phonetic gate clears, the bonus strictly raises the score.
    if r_on.breakdown.name_phonetic_score == Some(1.0) {
        assert!(
            r_on.score > r_off.score,
            "a fired phonetic bonus must nudge the score up: on={} off={}",
            r_on.score,
            r_off.score
        );
    }
}

// ============================================================
// Worked example §11.1 — pin the renormalised Glastonbury score
// ============================================================

#[test]
fn test_glastonbury_worked_example_score_matches_spec_11_1() {
    // spec/11-worked-examples.md §11.1 hand-computes ≈ 0.976. This guards the
    // worked example against silent weight drift.
    let a = Event::builder()
        .name("Glastonbury Festival 2024")
        .add_alternate_name("Glasto 2024")
        .start_date("2024-06-26T09:00:00Z")
        .end_date("2024-06-30T23:59:00Z")
        .category(EventCategory::Festival)
        .country_code_as_iso_3166_1_alpha_2("GB")
        .build();
    let b = Event::builder()
        .name("Glasto 2024")
        .start_date("2024-06-26T09:15:00Z")
        .end_date("2024-06-30T23:59:00Z")
        .category(EventCategory::Festival)
        .country_code_as_iso_3166_1_alpha_2("GB")
        .build();
    let r = MatchingEngine::default_config().match_events(&a, &b);
    assert!(r.is_match);
    assert_eq!(r.confidence, Confidence::High);
    assert!(
        (r.score - 0.976).abs() < 0.01,
        "expected the §11.1 worked example to renormalise to ~0.976, got {}",
        r.score
    );
}
