//! Adapter contract test for the `place-matcher` public API.
//!
//! Pins the public surface that downstream `place-service` depends on via
//! its `to_matcher_place` adapter.

use place_matcher::{
    Address, Confidence, MatchConfig, MatchingEngine, Place, PlaceBuilder, PlaceCategory, PlaceId,
    PlaceIdScheme,
};

// =============================================================================
// 1. PlaceBuilder surface
// =============================================================================

#[test]
fn place_builder_demographic_and_contact_surface() {
    let addr = Address::new().with_line1("1 Test St").with_city("Town");
    let pid = PlaceId::new(PlaceIdScheme::Other("GLN".into()), "0614141999996").unwrap();

    let p: Place = Place::builder()
        .name("Central Park")
        .add_alternate_name("The Park")
        .alternate_names(vec!["Manhattan Park".into()])
        .latitude(40.7829)
        .longitude(-73.9654)
        .category(PlaceCategory::Park)
        .add_place_id(pid.clone())
        .place_ids(vec![pid])
        .address(addr)
        .phone("+1-212-310-6600")
        .email("info@example.com")
        .local_id("LOCAL-1")
        .altitude_as_metre(10.0)
        .elevation_as_metre(10.0)
        .area_as_metre_2(8000.0)
        .country_code_as_iso_3166_1_alpha_2("US")
        .maximum_capacity_count(100_000)
        .build();

    assert_eq!(p.name.as_deref(), Some("Central Park"));
    assert_eq!(p.latitude, Some(40.7829));
    assert_eq!(p.longitude, Some(-73.9654));
    assert_eq!(p.category, Some(PlaceCategory::Park));
    assert!(!p.place_ids.is_empty());
    assert!(p.address.is_some());
}

// =============================================================================
// 2. PlaceId / PlaceIdScheme construction
// =============================================================================

#[test]
fn place_id_constructor_surface() {
    // Empty value → None (trimmed).
    assert!(PlaceId::new(PlaceIdScheme::Google, "   ").is_none());

    // Known schemes downstream uses by name.
    let _ = [
        PlaceIdScheme::Google,
        PlaceIdScheme::OsmNode,
        PlaceIdScheme::OsmWay,
        PlaceIdScheme::OsmRelation,
        PlaceIdScheme::GeoNames,
        PlaceIdScheme::Wikidata,
        PlaceIdScheme::Foursquare,
        PlaceIdScheme::Here,
        PlaceIdScheme::Mapbox,
        PlaceIdScheme::Other("Custom".into()),
    ];

    let id = PlaceId::new(PlaceIdScheme::Wikidata, "Q243").unwrap();
    assert_eq!(id.scheme, PlaceIdScheme::Wikidata);
    assert_eq!(id.value, "Q243");
}

// =============================================================================
// 3. PlaceCategory variants downstream maps service PlaceType into
// =============================================================================

#[test]
fn place_category_variants_used_by_adapter_exist() {
    // The service's `map_place_type` routes into these variants; if any are
    // renamed or removed, downstream breaks.
    let _ = [
        PlaceCategory::Hotel,
        PlaceCategory::Restaurant,
        PlaceCategory::Cafe,
        PlaceCategory::Bar,
        PlaceCategory::Shop,
        PlaceCategory::Mall,
        PlaceCategory::Hospital,
        PlaceCategory::School,
        PlaceCategory::University,
        PlaceCategory::Library,
        PlaceCategory::Museum,
        PlaceCategory::Theatre,
        PlaceCategory::Cinema,
        PlaceCategory::Park,
        PlaceCategory::Beach,
        PlaceCategory::Stadium,
        PlaceCategory::Airport,
        PlaceCategory::RailwayStation,
        PlaceCategory::BusStation,
        PlaceCategory::Bank,
        PlaceCategory::PostOffice,
        PlaceCategory::Government,
        PlaceCategory::Monument,
        PlaceCategory::ReligiousBuilding,
        PlaceCategory::Cemetery,
        PlaceCategory::Mountain,
        PlaceCategory::Lake,
        PlaceCategory::River,
        PlaceCategory::City,
        PlaceCategory::Town,
        PlaceCategory::Village,
        PlaceCategory::Neighborhood,
        PlaceCategory::OfficeBuilding,
        PlaceCategory::Residence,
        PlaceCategory::Warehouse,
        PlaceCategory::Other("custom".into()),
    ];
}

// =============================================================================
// 4. Address builder surface
// =============================================================================

#[test]
fn address_builder_surface() {
    let a = Address::new()
        .with_line1("Line 1")
        .with_line2("Line 2")
        .with_city("Town")
        .with_county("Region")
        .with_postcode("AB1 2CD")
        .with_country("GB");
    assert_eq!(a.line1.as_deref(), Some("Line 1"));
    assert_eq!(a.county.as_deref(), Some("Region"));
    assert_eq!(a.postcode.as_deref(), Some("AB1 2CD"));
}

// =============================================================================
// 5. MatchingEngine entry points
// =============================================================================

#[test]
fn matching_engine_constructor_surface() {
    let _: MatchingEngine = MatchingEngine::default_config();
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::default());
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::strict());
    let _: MatchingEngine = MatchingEngine::new(MatchConfig::lenient());
}

#[test]
fn matching_engine_match_places_returns_match_result() {
    let a = Place::builder().name("Central Park").build();
    let b = a.clone();
    let result = MatchingEngine::default_config().match_places(&a, &b);
    let _: f64 = result.score;
    let _: bool = result.is_match;
    let _: Confidence = result.confidence;
    let _ = result.breakdown.name_score;
    let _ = result.breakdown.coordinates_score;
    let _ = result.breakdown.address_score;
    let _ = result.breakdown.category_score;
    let _ = result.breakdown.country_code_score;
    let _ = result.breakdown.place_ids_score;
    let _ = result.breakdown.phone_score;
    let _ = result.breakdown.email_score;
}

#[test]
fn matching_engine_deterministic_match_returns_bool() {
    let id = PlaceId::new(PlaceIdScheme::Other("GLN".into()), "0614141999996").unwrap();
    let a = Place::builder().name("X").add_place_id(id.clone()).build();
    let b = Place::builder().name("Y").add_place_id(id).build();
    let res: bool = MatchingEngine::default_config().deterministic_match(&a, &b);
    assert!(res, "shared place_id must trigger deterministic match");
}

#[test]
fn matching_engine_match_one_to_many_returns_vec() {
    let query = Place::builder().name("Q").build();
    let candidates = vec![query.clone(), query.clone()];
    let results = MatchingEngine::default_config().match_one_to_many(&query, &candidates);
    assert_eq!(results.len(), candidates.len());
}

// =============================================================================
// 6. Confidence + config + round-trip
// =============================================================================

#[test]
fn confidence_variants_exist() {
    let _ = [Confidence::High, Confidence::Medium, Confidence::Low];
}

#[test]
fn match_config_preset_scores_form_monotonic_threshold_ladder() {
    let strict = MatchConfig::strict().match_threshold;
    let default = MatchConfig::default().match_threshold;
    let lenient = MatchConfig::lenient().match_threshold;
    assert!(strict >= default && default >= lenient);
}

#[test]
fn match_result_round_trips_through_json() {
    let a = Place::builder().name("X").build();
    let result = MatchingEngine::default_config().match_places(&a, &a);
    let json = serde_json::to_string(&result).expect("serialize");
    let _: place_matcher::MatchResult = serde_json::from_str(&json).expect("deserialize");
}

#[test]
fn place_builder_is_value_type() {
    fn _check(b: PlaceBuilder) -> PlaceBuilder {
        b.name("ok")
    }
}
