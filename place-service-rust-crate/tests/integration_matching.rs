use place_service::models::address::PostalAddress;
use place_service::models::geo::GeoCoordinates;
use place_service::models::identifier::PlaceIdentifier;
use place_service::models::place::Place;
use place_service::models::place_type::PlaceType;
use place_service::matching::scoring::{compute_match, MatchConfidence, MatchWeights};

fn make_place(
    name: &str,
    place_type: Option<PlaceType>,
    lat: Option<f64>,
    lon: Option<f64>,
    locality: Option<&str>,
    country: Option<&str>,
) -> Place {
    let mut p = Place::new(name);
    p.place_type = place_type;
    if let (Some(lat), Some(lon)) = (lat, lon) {
        p.geo = Some(GeoCoordinates::new(lat, lon));
    }
    if locality.is_some() || country.is_some() {
        p.address = Some(PostalAddress {
            street_address: None,
            address_locality: locality.map(String::from),
            address_region: None,
            address_country: country.map(String::from),
            postal_code: None,
        });
    }
    p
}

#[test]
fn test_exact_duplicate_detection() {
    let a = make_place("Central Park", Some(PlaceType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US"));
    let b = make_place("Central Park", Some(PlaceType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US"));
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score > 0.95, "Expected near-perfect match, got {}", result.score);
    assert_eq!(result.confidence, MatchConfidence::Certain);
}

#[test]
fn test_typo_in_name_still_matches() {
    let a = make_place("Central Park", Some(PlaceType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US"));
    let b = make_place("Centrl Park", Some(PlaceType::Park), Some(40.7830), Some(-73.9655), Some("New York"), Some("US"));
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score > 0.7, "Expected probable match, got {}", result.score);
}

#[test]
fn test_completely_different_places() {
    let a = make_place("Central Park", Some(PlaceType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US"));
    let b = make_place("Eiffel Tower", Some(PlaceType::CivicStructure), Some(48.8584), Some(2.2945), Some("Paris"), Some("FR"));
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score < 0.3, "Expected low match, got {}", result.score);
    assert_eq!(result.confidence, MatchConfidence::Unlikely);
}

#[test]
fn test_same_name_different_city() {
    let a = make_place("Main Street Cafe", Some(PlaceType::Restaurant), Some(40.7128), Some(-74.0060), Some("New York"), Some("US"));
    let b = make_place("Main Street Cafe", Some(PlaceType::Restaurant), Some(34.0522), Some(-118.2437), Some("Los Angeles"), Some("US"));
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score < 0.9, "Score: {}", result.score);
}

#[test]
fn test_gln_deterministic_overrides_name_mismatch() {
    let mut a = Place::new("Store Alpha");
    a.identifiers = vec![PlaceIdentifier::gln("1234567890123")];
    let mut b = Place::new("Store Beta");
    b.identifiers = vec![PlaceIdentifier::gln("1234567890123")];
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!((result.score - 1.0).abs() < f64::EPSILON);
    assert!(result.breakdown.deterministic_match);
}

#[test]
fn test_matching_with_name_only() {
    let a = Place::new("Golden Gate Bridge");
    let b = Place::new("Golden Gate Bridge");
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score > 0.95, "Score: {}", result.score);
}

#[test]
fn test_batch_matching_multiple_candidates() {
    let target = make_place("Central Park", Some(PlaceType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US"));
    let candidates = [
        make_place("Central Park", Some(PlaceType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US")),
        make_place("Central Park Zoo", Some(PlaceType::LocalBusiness), Some(40.7678), Some(-73.9718), Some("New York"), Some("US")),
        make_place("Hyde Park", Some(PlaceType::Park), Some(51.5073), Some(-0.1657), Some("London"), Some("GB")),
    ];

    let mut results: Vec<_> = candidates
        .iter()
        .map(|c| compute_match(&target, c, &MatchWeights::default()))
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    assert!(results[0].score > 0.95);
    assert!(results[1].score > results[2].score);
}
