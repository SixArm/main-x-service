use main_thing_service::models::address::PostalAddress;
use main_thing_service::models::geo::GeoCoordinates;
use main_thing_service::models::identifier::ThingIdentifier;
use main_thing_service::models::thing::Thing;
use main_thing_service::models::thing_type::ThingType;
use main_thing_service::matching::scoring::{compute_match, MatchConfidence, MatchWeights};

fn make_thing(
    name: &str,
    thing_type: Option<ThingType>,
    lat: Option<f64>,
    lon: Option<f64>,
    locality: Option<&str>,
    country: Option<&str>,
) -> Thing {
    let mut p = Thing::new(name);
    p.thing_type = thing_type;
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
    let a = make_thing("Central Park", Some(ThingType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US"));
    let b = make_thing("Central Park", Some(ThingType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US"));
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score > 0.95, "Expected near-perfect match, got {}", result.score);
    assert_eq!(result.confidence, MatchConfidence::Certain);
}

#[test]
fn test_typo_in_name_still_matches() {
    let a = make_thing("Central Park", Some(ThingType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US"));
    let b = make_thing("Centrl Park", Some(ThingType::Park), Some(40.7830), Some(-73.9655), Some("New York"), Some("US"));
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score > 0.7, "Expected probable match, got {}", result.score);
}

#[test]
fn test_completely_different_things() {
    let a = make_thing("Central Park", Some(ThingType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US"));
    let b = make_thing("Eiffel Tower", Some(ThingType::CivicStructure), Some(48.8584), Some(2.2945), Some("Paris"), Some("FR"));
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score < 0.3, "Expected low match, got {}", result.score);
    assert_eq!(result.confidence, MatchConfidence::Unlikely);
}

#[test]
fn test_same_name_different_city() {
    let a = make_thing("Main Street Cafe", Some(ThingType::Restaurant), Some(40.7128), Some(-74.0060), Some("New York"), Some("US"));
    let b = make_thing("Main Street Cafe", Some(ThingType::Restaurant), Some(34.0522), Some(-118.2437), Some("Los Angeles"), Some("US"));
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score < 0.9, "Score: {}", result.score);
}

#[test]
fn test_gln_deterministic_overrides_name_mismatch() {
    let mut a = Thing::new("Store Alpha");
    a.identifiers = vec![ThingIdentifier::gln("1234567890123")];
    let mut b = Thing::new("Store Beta");
    b.identifiers = vec![ThingIdentifier::gln("1234567890123")];
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!((result.score - 1.0).abs() < f64::EPSILON);
    assert!(result.breakdown.deterministic_match);
}

#[test]
fn test_matching_with_name_only() {
    let a = Thing::new("Golden Gate Bridge");
    let b = Thing::new("Golden Gate Bridge");
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.score > 0.95, "Score: {}", result.score);
}

#[test]
fn test_batch_matching_multiple_candidates() {
    let target = make_thing("Central Park", Some(ThingType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US"));
    let candidates = [
        make_thing("Central Park", Some(ThingType::Park), Some(40.7829), Some(-73.9654), Some("New York"), Some("US")),
        make_thing("Central Park Zoo", Some(ThingType::LocalBusiness), Some(40.7678), Some(-73.9718), Some("New York"), Some("US")),
        make_thing("Hyde Park", Some(ThingType::Park), Some(51.5073), Some(-0.1657), Some("London"), Some("GB")),
    ];

    let mut results: Vec<_> = candidates
        .iter()
        .map(|c| compute_match(&target, c, &MatchWeights::default()))
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    assert!(results[0].score > 0.95);
    assert!(results[1].score > results[2].score);
}
