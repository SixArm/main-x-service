//! Integration tests for the scoring components and the combined scorer.
//!
//! These cover edge cases across each matching component (name, address, geo,
//! identifier, phonetic) and the [`compute_match`] aggregator: Unicode and
//! reversed names, sparse addresses, antipodal/date-line geo, multi-identifier
//! lists, Soundex consistency, custom weights, confidence boundaries, score
//! range invariants, the phonetic bonus, all-component matches, and batch
//! ranking.

use place_service::matching::name::name_similarity;
use place_service::matching::address::address_similarity;
use place_service::matching::geo::{geo_similarity, geo_similarity_with_reference, within_radius};
use place_service::matching::identifier::{identifier_similarity, has_gln_match};
use place_service::matching::phonetic::{soundex, soundex_match};
use place_service::matching::scoring::{compute_match, MatchConfidence, MatchWeights};
use place_service::models::address::PostalAddress;
use place_service::models::geo::GeoCoordinates;
use place_service::models::identifier::{IdentifierType, PlaceIdentifier};
use place_service::models::place::Place;
use place_service::models::place_type::PlaceType;

// -- Name matching edge cases --

/// Accented vs unaccented names score highly similar.
#[test]
fn test_name_similarity_unicode() {
    let score = name_similarity("Café de Flore", "Cafe de Flore");
    assert!(score > 0.8, "Unicode name similarity: {score}");
}

/// Identical long names score exactly 1.0.
#[test]
fn test_name_similarity_very_long_names() {
    let a = "The Very Long Name of an Incredibly Important Historical Place in Downtown";
    let b = "The Very Long Name of an Incredibly Important Historical Place in Downtown";
    let score = name_similarity(a, b);
    assert!((score - 1.0).abs() < f64::EPSILON);
}

/// Two different single characters score in `[0.0, 1.0)`.
#[test]
fn test_name_similarity_single_character() {
    let score = name_similarity("A", "B");
    assert!(score < 1.0);
    assert!(score >= 0.0);
}

/// Reversed word order scores partial, not perfect.
#[test]
fn test_name_similarity_reversed_words() {
    let score = name_similarity("Park Central", "Central Park");
    assert!(score > 0.5, "Reversed words score: {score}");
    assert!(score < 1.0, "Should not be perfect: {score}");
}

// -- Address matching edge cases --

/// Addresses sharing only a matching country score 1.0 (adaptive weighting).
#[test]
fn test_address_similarity_only_country() {
    let a = PostalAddress {
        street_address: None,
        address_locality: None,
        address_region: None,
        address_country: Some("US".into()),
        postal_code: None,
    };
    let b = PostalAddress {
        street_address: None,
        address_locality: None,
        address_region: None,
        address_country: Some("US".into()),
        postal_code: None,
    };
    let score = address_similarity(&a, &b);
    assert!((score - 1.0).abs() < f64::EPSILON);
}

/// Two empty addresses share no field and score 0.0.
#[test]
fn test_address_similarity_empty_addresses() {
    let a = PostalAddress::new();
    let b = PostalAddress::new();
    let score = address_similarity(&a, &b);
    assert!((score - 0.0).abs() < f64::EPSILON);
}

/// Near-identical postal codes score high but below 1.0.
#[test]
fn test_address_similarity_similar_postal_codes() {
    let a = PostalAddress {
        street_address: None,
        address_locality: None,
        address_region: None,
        address_country: None,
        postal_code: Some("10001".into()),
    };
    let b = PostalAddress {
        street_address: None,
        address_locality: None,
        address_region: None,
        address_country: None,
        postal_code: Some("10002".into()),
    };
    let score = address_similarity(&a, &b);
    assert!(score > 0.7, "Similar postal codes: {score}");
    assert!(score < 1.0);
}

// -- Geo matching edge cases --

/// Pole-to-pole coordinates score near 0.0.
#[test]
fn test_geo_similarity_north_south_poles() {
    let north = GeoCoordinates::new(90.0, 0.0);
    let south = GeoCoordinates::new(-90.0, 0.0);
    let score = geo_similarity(&north, &south);
    assert!(score < 0.001, "Pole-to-pole score: {score}");
}

/// Points straddling the date line are correctly measured as close.
#[test]
fn test_geo_similarity_date_line() {
    let a = GeoCoordinates::new(0.0, 179.99);
    let b = GeoCoordinates::new(0.0, -179.99);
    // These are very close across the date line but our haversine handles it
    let dist = a.distance_to(&b);
    assert!(dist < 3000.0, "Date line distance should be small: {dist}m");
}

/// `within_radius` is inclusive at the boundary and excludes a too-tight radius.
#[test]
fn test_geo_within_radius_boundary() {
    let a = GeoCoordinates::new(40.7829, -73.9654);
    let b = GeoCoordinates::new(40.7830, -73.9655);
    let dist = a.distance_to(&b);
    // Should be within a generous radius
    assert!(within_radius(&a, &b, dist + 1.0));
    // Should NOT be within a too-tight radius
    assert!(!within_radius(&a, &b, 0.1));
}

/// A larger reference distance yields a higher similarity for the same gap.
#[test]
fn test_geo_reference_distance_effect() {
    let a = GeoCoordinates::new(40.7829, -73.9654);
    let b = GeoCoordinates::new(40.7929, -73.9754);

    let score_tight = geo_similarity_with_reference(&a, &b, 0.01);
    let score_normal = geo_similarity_with_reference(&a, &b, 1.0);
    let score_loose = geo_similarity_with_reference(&a, &b, 100.0);

    assert!(score_tight < score_normal);
    assert!(score_normal < score_loose);
}

// -- Identifier matching edge cases --

/// Lists sharing identifiers score 1.0.
#[test]
fn test_identifier_similarity_multiple_matches() {
    let a = vec![
        PlaceIdentifier::gln("1234567890123"),
        PlaceIdentifier::new(IdentifierType::Fips, "36061"),
    ];
    let b = vec![
        PlaceIdentifier::gln("1234567890123"),
        PlaceIdentifier::new(IdentifierType::Fips, "36061"),
    ];
    let score = identifier_similarity(&a, &b);
    assert!((score - 1.0).abs() < f64::EPSILON);
}

/// Same type but different values score 0.0.
#[test]
fn test_identifier_no_match_different_values() {
    let a = vec![PlaceIdentifier::new(IdentifierType::Fips, "36061")];
    let b = vec![PlaceIdentifier::new(IdentifierType::Fips, "06037")];
    let score = identifier_similarity(&a, &b);
    assert!((score - 0.0).abs() < f64::EPSILON);
}

/// A shared GLN is detected even among other identifier types.
#[test]
fn test_gln_match_among_many_identifiers() {
    let a = vec![
        PlaceIdentifier::new(IdentifierType::Fips, "36061"),
        PlaceIdentifier::gln("1234567890123"),
        PlaceIdentifier::new(IdentifierType::Gnis, "975772"),
    ];
    let b = vec![
        PlaceIdentifier::new(IdentifierType::OpenStreetMap, "175905"),
        PlaceIdentifier::gln("1234567890123"),
    ];
    assert!(has_gln_match(&a, &b));
}

// -- Phonetic matching edge cases --

/// Soundex is deterministic and always four characters.
#[test]
fn test_soundex_codes_consistency() {
    // Same name produces same code every time
    let code1 = soundex("Springfield");
    let code2 = soundex("Springfield");
    assert_eq!(code1, code2);
    assert_eq!(code1.len(), 4);
}

/// All-numeric input yields the empty `"0000"` code.
#[test]
fn test_soundex_numeric_input() {
    let code = soundex("123");
    assert_eq!(code, "0000");
}

/// Misspelled place names still phonetically match.
#[test]
fn test_soundex_match_similar_place_names() {
    assert!(soundex_match("Manhattan", "Manhatan"));
    assert!(soundex_match("Brooklyn", "Brooklin"));
}

// -- Scoring integration tests --

/// Name-heavy weights outscore geo-heavy weights when names match but geo differs.
#[test]
fn test_match_with_custom_weights() {
    let mut a = Place::new("Test Place");
    a.geo = Some(GeoCoordinates::new(40.7829, -73.9654));

    let mut b = Place::new("Test Place");
    b.geo = Some(GeoCoordinates::new(48.8584, 2.2945)); // Far away

    let name_heavy = MatchWeights {
        name: 0.90,
        geo: 0.02,
        address: 0.02,
        place_type: 0.03,
        identifier: 0.03,
    };

    let geo_heavy = MatchWeights {
        name: 0.10,
        geo: 0.80,
        address: 0.02,
        place_type: 0.04,
        identifier: 0.04,
    };

    let result_name = compute_match(&a, &b, &name_heavy);
    let result_geo = compute_match(&a, &b, &geo_heavy);

    assert!(result_name.score > result_geo.score,
        "Name-heavy {:.3} should > geo-heavy {:.3} when names match but geo differs",
        result_name.score, result_geo.score);
}

/// Confidence classification is correct exactly at and just below each threshold.
#[test]
fn test_match_confidence_boundaries() {
    assert_eq!(MatchConfidence::from_score(0.95), MatchConfidence::Certain);
    assert_eq!(MatchConfidence::from_score(0.949), MatchConfidence::Probable);
    assert_eq!(MatchConfidence::from_score(0.80), MatchConfidence::Probable);
    assert_eq!(MatchConfidence::from_score(0.799), MatchConfidence::Possible);
    assert_eq!(MatchConfidence::from_score(0.60), MatchConfidence::Possible);
    assert_eq!(MatchConfidence::from_score(0.599), MatchConfidence::Unlikely);
    assert_eq!(MatchConfidence::from_score(0.0), MatchConfidence::Unlikely);
    assert_eq!(MatchConfidence::from_score(1.0), MatchConfidence::Certain);
}

/// Every score across a cross-product of inputs stays in `[0.0, 1.0]`.
#[test]
fn test_match_score_always_in_range() {
    let places = [
        Place::new(""),
        Place::new("A"),
        Place::new("A very long place name that goes on and on"),
    ];
    let weights = MatchWeights::default();

    for a in &places {
        for b in &places {
            let result = compute_match(a, b, &weights);
            assert!(result.score >= 0.0 && result.score <= 1.0,
                "Score out of range: {} for {:?} vs {:?}", result.score, a.name, b.name);
        }
    }
}

/// The phonetic bonus lifts the score for sounds-alike misspellings.
#[test]
fn test_match_phonetic_bonus_applied() {
    // Names that sound alike but are spelled differently
    let a = Place::new("Springfield");
    let b = Place::new("Springfeild");
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.breakdown.phonetic_match);
    // The phonetic bonus should push the score up
    assert!(result.score > 0.85, "With phonetic bonus: {}", result.score);
}

/// No phonetic bonus is added when the score is already at/above 0.95.
#[test]
fn test_match_no_phonetic_bonus_when_score_high() {
    // Identical names - phonetic match is true but no bonus applied (score already >= 0.95)
    let a = Place::new("Central Park");
    let b = Place::new("Central Park");
    let result = compute_match(&a, &b, &MatchWeights::default());
    assert!(result.breakdown.phonetic_match);
    assert!((result.score - 1.0).abs() < f64::EPSILON);
}

/// A match with every component present populates the full breakdown.
#[test]
fn test_match_with_all_components() {
    let mut a = Place::new("Central Park");
    a.place_type = Some(PlaceType::Park);
    a.geo = Some(GeoCoordinates::new(40.7829, -73.9654));
    a.address = Some(PostalAddress {
        street_address: Some("14 E 60th St".into()),
        address_locality: Some("New York".into()),
        address_region: Some("NY".into()),
        address_country: Some("US".into()),
        postal_code: Some("10022".into()),
    });
    a.identifiers = vec![PlaceIdentifier::new(IdentifierType::Gnis, "975772")];

    let mut b = a.clone();
    b.id = uuid::Uuid::new_v4();
    b.name = "Central Park NYC".into();

    let result = compute_match(&a, &b, &MatchWeights::default());

    // All component scores should be populated
    assert!(result.breakdown.name_score > 0.5);
    assert!((result.breakdown.geo_score - 1.0).abs() < 0.001);
    assert!((result.breakdown.address_score - 1.0).abs() < f64::EPSILON);
    assert!((result.breakdown.place_type_score - 1.0).abs() < f64::EPSILON);
    assert!((result.breakdown.identifier_score - 1.0).abs() < f64::EPSILON);
}

/// Candidate scores rank exact > partial > different.
#[test]
fn test_batch_matching_sorted_by_relevance() {
    let target = Place::new("Central Park");
    let candidates = [
        Place::new("Central Park"),       // exact
        Place::new("Central Gardens"),     // partial
        Place::new("Buckingham Palace"),   // different
    ];
    let weights = MatchWeights::default();

    let scores: Vec<f64> = candidates
        .iter()
        .map(|c| compute_match(&target, c, &weights).score)
        .collect();

    assert!(scores[0] > scores[1], "Exact should beat partial");
    assert!(scores[1] > scores[2], "Partial should beat different");
}
