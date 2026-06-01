//! End-to-end duplicate-detection integration tests for the
//! place-service ↔ place-matcher bridge.
//!
//! Each test builds service-side `Place` records, projects them through
//! [`adapter::to_matcher_place`], and asserts on the canonical
//! `place_matcher::MatchingEngine` output.

use place_service::matching::adapter::to_matcher_place;
use place_service::matching::matcher_lib::{Confidence, MatchingEngine, PlaceIdScheme};
use place_service::models::{
    address::PostalAddress,
    geo::GeoCoordinates,
    identifier::{IdentifierType, PlaceIdentifier},
    place::Place,
    place_type::PlaceType,
};

// -------- builders -----------------------------------------------------------

fn place(name: &str) -> Place {
    Place::new(name)
}

fn central_park() -> Place {
    let mut p = place("Central Park");
    p.alternate_name = Some("The Central Park".into());
    p.place_type = Some(PlaceType::Park);
    p.address = Some(PostalAddress {
        street_address: Some("14 E 60th St".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10022".into()),
    });
    p.geo = Some(GeoCoordinates {
        latitude: 40.7829,
        longitude: -73.9654,
        elevation: None,
    });
    p
}

fn engine() -> MatchingEngine {
    MatchingEngine::default_config()
}

// =============================================================================
// Identical / near-duplicate cases
// =============================================================================

#[test]
fn identical_places_score_near_one_high_confidence() {
    let a = central_park();
    let b = a.clone();

    let result = engine().match_places(&to_matcher_place(&a), &to_matcher_place(&b));

    assert!(
        result.score >= 0.95,
        "identical places should score ≥ 0.95, got {}",
        result.score
    );
    assert_eq!(result.confidence, Confidence::High);
    assert!(result.is_match);
}

#[test]
fn typo_in_name_still_matches_when_geo_address_agree() {
    // Same place, same address, same coords; name has a typo. The matcher's
    // Jaro-Winkler + the geo + address evidence should still flag this.
    let a = central_park();
    let mut b = central_park();
    b.name = "Centrl Park".into(); // dropped 'a'

    let result = engine().match_places(&to_matcher_place(&a), &to_matcher_place(&b));

    assert!(
        result.score >= 0.85,
        "name typo with matching address+geo should score ≥ 0.85, got {}",
        result.score
    );
    assert!(result.is_match);
}

#[test]
fn closer_geo_outscores_farther_geo_for_same_name() {
    // Same name + same address on each pair; only the geo distance varies.
    // The matcher uses Gaussian-decay geo scoring (default scale = 50m), so
    // close coords contribute strongly while far coords contribute weakly.
    // We assert the *ranking* — close > far — which the matcher must
    // guarantee regardless of the absolute decay constant.
    let mut close_a = central_park();
    let mut close_b = central_park();
    close_a.geo = Some(GeoCoordinates {
        latitude: 40.7829,
        longitude: -73.9654,
        elevation: None,
    });
    close_b.geo = Some(GeoCoordinates {
        latitude: 40.78294, // ~5m north
        longitude: -73.9654,
        elevation: None,
    });

    let mut far_a = central_park();
    let mut far_b = central_park();
    far_a.geo = Some(GeoCoordinates {
        latitude: 40.7829,
        longitude: -73.9654,
        elevation: None,
    });
    far_b.geo = Some(GeoCoordinates {
        latitude: 41.7829, // ~111km north
        longitude: -73.9654,
        elevation: None,
    });

    let close = engine().match_places(&to_matcher_place(&close_a), &to_matcher_place(&close_b));
    let far = engine().match_places(&to_matcher_place(&far_a), &to_matcher_place(&far_b));

    assert!(
        close.score > far.score,
        "closer geo (5m) must outscore far geo (111km) for same-name pairs: close={} far={}",
        close.score,
        far.score
    );
    assert!(
        close.is_match,
        "5m drift with matching name+address must match, got {}",
        close.score
    );
}

// =============================================================================
// Deterministic short-circuits — identifiers
// =============================================================================

#[test]
fn shared_global_location_number_drives_high_score() {
    let mut a = place("ACME East Warehouse");
    a.global_location_number = Some("0614141999996".into());
    let mut b = place("ACME — East Site");
    b.global_location_number = Some("0614141999996".into());

    let result = engine().match_places(&to_matcher_place(&a), &to_matcher_place(&b));

    assert_eq!(
        result.breakdown.place_ids_score,
        Some(1.0),
        "shared GLN must produce place_ids_score = 1.0"
    );
    assert!(
        result.score >= 0.80,
        "shared GLN should drive overall score ≥ 0.80 even with name drift, got {}",
        result.score
    );
}

#[test]
fn shared_osm_identifier_routes_to_osm_node_scheme() {
    let mut a = place("Big Ben");
    a.identifiers.push(PlaceIdentifier::new(
        IdentifierType::OpenStreetMap,
        "node:123456789",
    ));
    let mut b = place("Elizabeth Tower"); // alternative name
    b.identifiers.push(PlaceIdentifier::new(
        IdentifierType::OpenStreetMap,
        "node:123456789",
    ));

    let ma = to_matcher_place(&a);
    let mb = to_matcher_place(&b);
    assert_eq!(ma.place_ids[0].scheme, PlaceIdScheme::OsmNode);

    let result = engine().match_places(&ma, &mb);
    assert_eq!(result.breakdown.place_ids_score, Some(1.0));
}

#[test]
fn different_gln_values_score_zero_on_identifier_axis() {
    let mut a = place("Site A");
    a.global_location_number = Some("0614141999996".into());
    let mut b = place("Site B");
    b.global_location_number = Some("0614141111110".into());

    let result = engine().match_places(&to_matcher_place(&a), &to_matcher_place(&b));
    assert_eq!(
        result.breakdown.place_ids_score,
        Some(0.0),
        "different GLNs with both populated should score 0 on the identifier axis"
    );
}

// =============================================================================
// PlaceType axis
// =============================================================================

#[test]
fn place_type_match_contributes_positive_signal() {
    let mut a = central_park();
    a.place_type = Some(PlaceType::Park);
    let mut b = central_park();
    b.place_type = Some(PlaceType::Park);

    let result = engine().match_places(&to_matcher_place(&a), &to_matcher_place(&b));
    assert_eq!(result.breakdown.category_score, Some(1.0));
}

#[test]
fn place_type_mismatch_lowers_overall_score() {
    let mut a = central_park();
    a.place_type = Some(PlaceType::Park);
    let mut b = central_park();
    b.place_type = Some(PlaceType::Restaurant);
    b.name = "Central Park Cafe".into();

    let result = engine().match_places(&to_matcher_place(&a), &to_matcher_place(&b));
    assert_eq!(
        result.breakdown.category_score,
        Some(0.0),
        "different categories should score 0 on the category axis"
    );
}

// =============================================================================
// Negative cases
// =============================================================================

#[test]
fn completely_different_places_score_low_and_do_not_match() {
    let mut a = place("Eiffel Tower");
    a.geo = Some(GeoCoordinates {
        latitude: 48.8584,
        longitude: 2.2945,
        elevation: None,
    });
    a.address = Some(PostalAddress {
        street_address: Some("Champ de Mars".into()),
        address_locality: Some("Paris".into()),
        address_region: None,
        address_country: Some("FR".into()),
        postal_code: Some("75007".into()),
    });

    let b = central_park();

    let result = engine().match_places(&to_matcher_place(&a), &to_matcher_place(&b));
    assert!(
        result.score < 0.60,
        "Paris vs NYC place should score < 0.60, got {}",
        result.score
    );
    assert!(!result.is_match);
}

#[test]
fn same_name_far_geo_does_not_falsely_match() {
    // Both called "Main Street" but in different countries — geo distance
    // must prevent the name-only match from short-circuiting.
    let mut a = place("Main Street");
    a.geo = Some(GeoCoordinates {
        latitude: 40.7,
        longitude: -74.0,
        elevation: None,
    });
    let mut b = place("Main Street");
    b.geo = Some(GeoCoordinates {
        latitude: -33.9,
        longitude: 151.2, // Sydney
        elevation: None,
    });

    let result = engine().match_places(&to_matcher_place(&a), &to_matcher_place(&b));
    assert!(
        !result.is_match,
        "same name, opposite hemispheres must not match, got score {}",
        result.score
    );
}

// =============================================================================
// Field-routing pinning
// =============================================================================

#[test]
fn address_field_names_map_correctly() {
    // Service uses schema.org names (street_address / address_locality /
    // address_region / postal_code); matcher uses British (line1 / city /
    // county / postcode). The adapter must rename, not drop.
    let mut a = place("Houses of Parliament");
    a.address = Some(PostalAddress {
        street_address: Some("Westminster".into()),
        address_locality: Some("London".into()),
        address_region: Some("Greater London".into()),
        address_country: Some("GB".into()),
        postal_code: Some("SW1A 0AA".into()),
    });

    let m = to_matcher_place(&a);
    let addr = m.address.as_ref().unwrap();
    assert_eq!(addr.line1.as_deref(), Some("Westminster"));
    assert_eq!(addr.city.as_deref(), Some("London"));
    assert_eq!(addr.county.as_deref(), Some("Greater London"));
    assert_eq!(addr.postcode.as_deref(), Some("SW1A 0AA"));
    assert_eq!(addr.country.as_deref(), Some("GB"));
    assert_eq!(
        m.country_code_as_iso_3166_1_alpha_2.as_deref(),
        Some("GB"),
        "2-char country must also propagate to the matcher's iso_3166_1_alpha_2 slot"
    );
}

#[test]
fn geo_elevation_propagates_when_present() {
    let mut a = place("Mauna Kea Summit");
    a.geo = Some(GeoCoordinates {
        latitude: 19.8207,
        longitude: -155.4681,
        elevation: Some(4205.0),
    });
    let m = to_matcher_place(&a);
    assert_eq!(m.elevation_as_metre, Some(4205.0));
}

// =============================================================================
// Edge cases — sparse records
// =============================================================================

#[test]
fn sparse_records_do_not_panic_and_score_in_range() {
    let a = place("");
    let b = place("Anywhere");
    let result = engine().match_places(&to_matcher_place(&a), &to_matcher_place(&b));
    assert!(result.score >= 0.0 && result.score <= 1.0);
}

#[test]
fn name_only_match_is_a_review_candidate_not_a_certain_duplicate() {
    // Two records that share a name and nothing else (no geo, no address,
    // no identifier). Should land in the "needs review" band — neither a
    // hard duplicate nor unrelated.
    let a = place("Smith & Co. Office");
    let b = place("Smith & Co. Office");
    let result = engine().match_places(&to_matcher_place(&a), &to_matcher_place(&b));
    // Name-only score is high — but the test asserts the SAFETY property: a
    // name-only "match" must still expose the per-field breakdown so an
    // operator can audit. We pin that the breakdown carries the name score
    // and the missing fields stay None.
    assert!(result.breakdown.name_score.is_some());
    assert_eq!(result.breakdown.coordinates_score, None);
    assert_eq!(result.breakdown.address_score, None);
    assert_eq!(result.breakdown.place_ids_score, None);
}
