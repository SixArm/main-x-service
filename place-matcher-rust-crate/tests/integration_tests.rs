//! Integration tests for place matcher.
//!
//! These exercise the **public API** as a downstream user would — building
//! [`Place`] records, constructing a [`MatchingEngine`], and inspecting
//! the [`place_matcher::MatchResult`] / [`place_matcher::MatchBreakdown`].
//!
//! Naming follows the AGENTS guide: `test_<thing>_<expected>`.

use place_matcher::{
    Address, Confidence, MatchConfig, MatchingEngine, Normalizer, Place, PlaceCategory, PlaceId,
    PlaceIdScheme, Scorer, SimilarityAlgorithm,
};

// ============================================================
// Perfect / near-perfect matches
// ============================================================

#[test]
fn test_perfect_match_all_fields() {
    let addr = Address::new()
        .with_line1("Champ de Mars")
        .with_city("Paris")
        .with_postcode("75007");
    let p1 = Place::builder()
        .name("Eiffel Tower")
        .add_alternate_name("La Tour Eiffel")
        .latitude(48.858_222)
        .longitude(2.294_500)
        .category(PlaceCategory::Monument)
        .country_code_as_iso_3166_1_alpha_2("FR")
        .address(addr.clone())
        .build();
    let p2 = p1.clone();

    let result = MatchingEngine::default_config().match_places(&p1, &p2);
    assert!(result.is_match);
    assert!(result.score > 0.95);
    assert!(result.breakdown.name_score.unwrap() > 0.99);
    assert_eq!(result.breakdown.coordinates_score, Some(1.0));
    assert_eq!(result.breakdown.category_score, Some(1.0));
    assert_eq!(result.breakdown.country_code_score, Some(1.0));
}

#[test]
fn test_identical_minimal_record_scores_one() {
    let p = Place::builder().name("Big Ben").build();
    let r = MatchingEngine::default_config().match_places(&p, &p.clone());
    assert!(r.is_match);
    assert!(r.score > 0.99);
}

// ============================================================
// Name + alternate-name matching (best of cartesian product)
// ============================================================

#[test]
fn test_name_match_picks_best_across_alternates() {
    let p1 = Place::builder().name("Eiffel Tower").build();
    let p2 = Place::builder()
        .name("La Tour Eiffel")
        .add_alternate_name("Eiffel Tower")
        .build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    let s = r.breakdown.name_score.expect("scored");
    assert!(s > 0.99, "expected best-of cartesian product to pick exact, got {s}");
}

#[test]
fn test_name_match_returns_none_if_either_side_empty() {
    let p1 = Place::builder().build();
    let p2 = Place::builder().name("Big Ben").build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert!(r.breakdown.name_score.is_none());
}

// ============================================================
// Coordinates scoring
// ============================================================

#[test]
fn test_coordinates_score_identical_points() {
    let p1 = Place::builder().name("X").latitude(48.85).longitude(2.29).build();
    let p2 = p1.clone();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert!((r.breakdown.coordinates_score.unwrap() - 1.0).abs() < 1e-9);
}

#[test]
fn test_coordinates_score_far_apart_decays_to_zero() {
    let p1 = Place::builder().name("X").latitude(0.0).longitude(0.0).build();
    let p2 = Place::builder().name("X").latitude(0.0).longitude(10.0).build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert!(r.breakdown.coordinates_score.unwrap() < 1e-3);
}

#[test]
fn test_coordinates_score_none_when_out_of_range() {
    let p1 = Place::builder().name("X").latitude(200.0).longitude(0.0).build();
    let p2 = Place::builder().name("X").latitude(0.0).longitude(0.0).build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert!(r.breakdown.coordinates_score.is_none());
}

// ============================================================
// Category equality
// ============================================================

#[test]
fn test_category_match_equal_scores_one() {
    let p1 = Place::builder().name("X").category(PlaceCategory::Hotel).build();
    let p2 = Place::builder().name("X").category(PlaceCategory::Hotel).build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert_eq!(r.breakdown.category_score, Some(1.0));
}

#[test]
fn test_category_mismatch_other_strings_scores_zero() {
    let p1 = Place::builder()
        .name("X")
        .category(PlaceCategory::Other("foo".into()))
        .build();
    let p2 = Place::builder()
        .name("X")
        .category(PlaceCategory::Other("bar".into()))
        .build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert_eq!(r.breakdown.category_score, Some(0.0));
}

// ============================================================
// Country code equality
// ============================================================

#[test]
fn test_country_code_case_insensitive_match() {
    let p1 = Place::builder()
        .name("X")
        .country_code_as_iso_3166_1_alpha_2("gb")
        .build();
    let p2 = Place::builder()
        .name("X")
        .country_code_as_iso_3166_1_alpha_2("GB")
        .build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert_eq!(r.breakdown.country_code_score, Some(1.0));
}

// ============================================================
// Place IDs
// ============================================================

#[test]
fn test_place_ids_shared_returns_one() {
    let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
    let a = Place::builder().name("Eiffel Tower").add_place_id(id.clone()).build();
    let b = Place::builder().name("Tour Eiffel").add_place_id(id).build();
    let r = MatchingEngine::default_config().match_places(&a, &b);
    assert_eq!(r.breakdown.place_ids_score, Some(1.0));
}

#[test]
fn test_place_ids_scheme_local_equality() {
    let a = Place::builder()
        .name("X")
        .add_place_id(PlaceId::new(PlaceIdScheme::Google, "X").unwrap())
        .build();
    let b = Place::builder()
        .name("X")
        .add_place_id(PlaceId::new(PlaceIdScheme::OsmNode, "X").unwrap())
        .build();
    let r = MatchingEngine::default_config().match_places(&a, &b);
    assert_eq!(r.breakdown.place_ids_score, Some(0.0));
}

// ============================================================
// Deterministic match
// ============================================================

#[test]
fn test_deterministic_via_shared_place_id_returns_true() {
    let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
    let a = Place::builder()
        .name("Anything")
        .add_place_id(id.clone())
        .build();
    let b = Place::builder().name("Other Name").add_place_id(id).build();
    assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
}

#[test]
fn test_deterministic_via_name_and_postcode_returns_true() {
    let addr = Address::new().with_postcode("SW1A 2AA");
    let a = Place::builder()
        .name("10 Downing Street")
        .address(addr.clone())
        .build();
    let b = Place::builder()
        .name("10 Downing Street")
        .address(addr)
        .build();
    assert!(MatchingEngine::default_config().deterministic_match(&a, &b));
}

#[test]
fn test_deterministic_fallback_requires_postcode() {
    let a = Place::builder().name("X").build();
    let b = Place::builder().name("X").build();
    assert!(!MatchingEngine::default_config().deterministic_match(&a, &b));
}

// ============================================================
// Address scoring
// ============================================================

#[test]
fn test_address_postcode_match_dominates() {
    let a = Address::new()
        .with_line1("10 High Street")
        .with_city("Cardiff")
        .with_postcode("CF10 1AA");
    let b = Address::new()
        .with_line1("10 High Road")
        .with_city("Cardiff")
        .with_postcode("CF10 1AA");
    let p1 = Place::builder().name("X").address(a).build();
    let p2 = Place::builder().name("X").address(b).build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert!(r.breakdown.address_score.unwrap() >= 0.7);
}

// ============================================================
// Phone, email
// ============================================================

#[test]
fn test_phone_normalisation_match() {
    let p1 = Place::builder()
        .name("X")
        .phone("+44 7700 900123")
        .build();
    let p2 = Place::builder()
        .name("X")
        .phone("07700 900123")
        .build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert_eq!(r.breakdown.phone_score, Some(1.0));
}

#[test]
fn test_email_match_after_normalisation() {
    let p1 = Place::builder()
        .name("X")
        .email("Info@Example.ORG")
        .build();
    let p2 = Place::builder()
        .name("X")
        .email("info@example.org")
        .build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert_eq!(r.breakdown.email_score, Some(1.0));
}

// ============================================================
// Haversine helpers (Scorer)
// ============================================================

#[test]
fn test_haversine_known_city_pair() {
    let d = Scorer::haversine_metres(51.507_22, -0.127_5, 48.853_0, 2.349_2) / 1000.0;
    assert!(d > 330.0 && d < 355.0, "London-Paris km was {d}");
}

#[test]
fn test_haversine_identical_zero() {
    let d = Scorer::haversine_metres(10.0, 20.0, 10.0, 20.0);
    assert!(d.abs() < 1e-6);
}

#[test]
fn test_haversine_antipode_about_20015km() {
    let d = Scorer::haversine_metres(0.0, 0.0, 0.0, 180.0) / 1000.0;
    assert!((d - 20015.0).abs() < 50.0);
}

#[test]
fn test_haversine_cross_dateline_is_short() {
    let d = Scorer::haversine_metres(0.0, 179.5, 0.0, -179.5) / 1000.0;
    assert!(d < 120.0);
}

#[test]
fn test_coordinates_score_contract_at_zero_scale_three_scale() {
    let scale = 50.0;
    assert!((Scorer::coordinates_score(0.0, scale) - 1.0).abs() < 1e-12);
    let one_e = Scorer::coordinates_score(scale, scale);
    assert!((one_e - (1.0_f64 / std::f64::consts::E)).abs() < 1e-12);
    assert!(Scorer::coordinates_score(3.0 * scale, scale) < 1e-3);
}

// ============================================================
// Config presets and serde
// ============================================================

#[test]
fn test_default_config_round_trips_through_json() {
    let cfg = MatchConfig::default();
    let json = serde_json::to_string(&cfg).expect("serialise");
    let back: MatchConfig = serde_json::from_str(&json).expect("deserialise");
    assert!((cfg.match_threshold - back.match_threshold).abs() < 1e-12);
    assert!((cfg.coordinates_weight - back.coordinates_weight).abs() < 1e-12);
}

#[test]
fn test_strict_preset_requires_deterministic_match() {
    let strict = MatchingEngine::new(MatchConfig::strict());
    let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
    let p1 = Place::builder()
        .name("Eiffel Tower")
        .latitude(48.858)
        .longitude(2.2945)
        .add_place_id(id.clone())
        .build();
    let p2 = p1.clone();
    assert!(strict.match_places(&p1, &p2).is_match);
}

#[test]
fn test_lenient_preset_lowers_threshold() {
    let lenient = MatchingEngine::new(MatchConfig::lenient());
    let p1 = Place::builder()
        .name("Cafe Centrale")
        .latitude(48.0)
        .longitude(2.0)
        .build();
    let p2 = Place::builder()
        .name("Cafe Central")
        .latitude(48.0)
        .longitude(2.0)
        .build();
    let r = lenient.match_places(&p1, &p2);
    assert!(r.score >= 0.65);
}

#[test]
fn test_similarity_algorithm_selectable() {
    let cfg = MatchConfig {
        name_algorithm: SimilarityAlgorithm::Exact,
        ..MatchConfig::default()
    };
    let engine = MatchingEngine::new(cfg);
    let p1 = Place::builder().name("Stephen").build();
    let p2 = Place::builder().name("Steven").build();
    let r = engine.match_places(&p1, &p2);
    assert!(r.breakdown.name_score.unwrap() < 0.9);
}

// ============================================================
// JSON round-trip of new types
// ============================================================

#[test]
fn test_place_with_new_types_round_trips_through_json() {
    let p = Place::builder()
        .name("Eiffel Tower")
        .add_alternate_name("La Tour Eiffel")
        .latitude(48.858_222)
        .longitude(2.294_500)
        .category(PlaceCategory::Monument)
        .add_place_id(PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap())
        .country_code_as_iso_3166_1_alpha_2("FR")
        .build();
    let json = serde_json::to_string(&p).expect("serialise");
    let back: Place = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(p, back);
}

#[test]
fn test_place_id_round_trips() {
    let id = PlaceId::new(PlaceIdScheme::Other("custom".into()), "abc-123").unwrap();
    let json = serde_json::to_string(&id).expect("serialise");
    let back: PlaceId = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(id, back);
}

#[test]
fn test_place_category_round_trips() {
    for c in [
        PlaceCategory::Hotel,
        PlaceCategory::RailwayStation,
        PlaceCategory::Other("ferry-terminal".into()),
    ] {
        let json = serde_json::to_string(&c).expect("serialise");
        let back: PlaceCategory = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(c, back);
    }
}

// ============================================================
// Place::validate
// ============================================================

#[test]
fn test_validate_requires_a_name() {
    assert!(Place::builder().name("Big Ben").build().validate().is_ok());
    assert!(Place::builder().build().validate().is_err());
}

// ============================================================
// Confidence band
// ============================================================

#[test]
fn test_confidence_band_high_for_exact_clone() {
    let p = Place::builder()
        .name("Eiffel Tower")
        .latitude(48.858_222)
        .longitude(2.294_500)
        .build();
    let r = MatchingEngine::default_config().match_places(&p, &p.clone());
    assert_eq!(r.confidence, Confidence::High);
}

// ============================================================
// Missing-field tolerance
// ============================================================

#[test]
fn test_missing_optional_fields_yield_none_in_breakdown() {
    let p1 = Place::builder().name("X").build();
    let p2 = Place::builder().name("X").build();
    let r = MatchingEngine::default_config().match_places(&p1, &p2);
    assert!(r.breakdown.coordinates_score.is_none());
    assert!(r.breakdown.address_score.is_none());
    assert!(r.breakdown.category_score.is_none());
    assert!(r.breakdown.country_code_score.is_none());
    assert!(r.breakdown.place_ids_score.is_none());
    assert!(r.breakdown.phone_score.is_none());
    assert!(r.breakdown.email_score.is_none());
}

// ============================================================
// Normalizer surface
// ============================================================

#[test]
fn test_normalizer_name_strips_diacritics_and_punctuation() {
    assert_eq!(Normalizer::normalize_name("Siân"), "sian");
    assert_eq!(Normalizer::normalize_name("O'Brien"), "obrien");
}

#[test]
fn test_normalizer_postcode_uppercases_and_strips_whitespace() {
    assert_eq!(Normalizer::normalize_postcode("cf10 1aa"), "CF101AA");
}

// ============================================================
// Batch APIs
// ============================================================

#[test]
fn test_match_one_to_many_returns_one_result_per_candidate() {
    let engine = MatchingEngine::default_config();
    let q = Place::builder().name("Eiffel Tower").build();
    let candidates = vec![
        Place::builder().name("Eiffel Tower").build(),
        Place::builder().name("Big Ben").build(),
    ];
    let r = engine.match_one_to_many(&q, &candidates);
    assert_eq!(r.len(), 2);
    assert!(r[0].is_match);
    assert!(!r[1].is_match);
}

#[test]
fn test_rank_one_to_many_orders_by_descending_score() {
    let engine = MatchingEngine::default_config();
    let q = Place::builder().name("Eiffel Tower").build();
    let candidates = vec![
        Place::builder().name("Big Ben").build(),
        q.clone(),
        Place::builder().name("Statue of Liberty").build(),
    ];
    let ranked = engine.rank_one_to_many(&q, &candidates);
    assert_eq!(ranked[0].0, 1);
    for w in ranked.windows(2) {
        assert!(w[0].1.score >= w[1].1.score);
    }
}
