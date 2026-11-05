#![warn(clippy::pedantic)]

//! Integration tests for the geo-radius candidate-filtering primitive
//! and an end-to-end matcher-bridge worked example.
//!
//! These pin two things the spec leans on but the per-module unit tests
//! do not exercise as a whole-collection operation:
//!
//! 1. **Geo-radius filtering** (`spec/06-functional-requirements.md §6.3`,
//!    task §13 T-9, landed). The `GET /api/places/nearby` HTTP endpoint
//!    now wires `matching::geo::within_radius` (behind a
//!    `matching::geo::bounding_box` pre-filter) — see
//!    `tests/api_nearby_and_search_offset.rs` for the DB-gated
//!    end-to-end HTTP coverage. This file keeps the primitive-level
//!    pin over a small in-memory collection, independent of the
//!    database.
//! 2. **Matcher-bridge worked example** (`agents/restful.md`). Drives two
//!    service-side records through `adapter::to_matcher_place` and the
//!    canonical `place_matcher::MatchingEngine`, asserting on the runnable
//!    result the docs describe.

use place_service::matching::adapter::to_matcher_place;
use place_service::matching::geo::within_radius;
use place_service::matching::matcher_lib::{Confidence, MatchConfig, MatchingEngine};
use place_service::models::geo::GeoCoordinates;
use place_service::models::place::Place;

/// Attach a fixed geo coordinate to a freshly-named `Place`.
fn place_at(name: &str, lat: f64, lon: f64) -> Place {
    let mut p = Place::new(name);
    p.geo = Some(GeoCoordinates::new(lat, lon));
    p
}

/// `within_radius` keeps only the places whose coordinates fall inside the
/// given radius of the centre — the candidate set the future `nearby`
/// endpoint will return.
#[test]
fn geo_radius_filters_collection_to_nearby_places() {
    // Centre: Central Park, NYC.
    let centre = GeoCoordinates::new(40.7829, -73.9654);

    let collection = [
        place_at("Times Square", 40.7580, -73.9855), // ~3 km away
        place_at("The Met", 40.7794, -73.9632),      // ~0.4 km away
        place_at("Statue of Liberty", 40.6892, -74.0445), // ~12 km away
        place_at("Los Angeles City Hall", 34.0537, -118.2428), // continent away
    ];

    // 2 km radius around the centre.
    let radius_m = 2_000.0;
    let nearby: Vec<&str> = collection
        .iter()
        .filter(|p| {
            p.geo
                .as_ref()
                .is_some_and(|g| within_radius(&centre, g, radius_m))
        })
        .map(|p| p.name.as_str())
        .collect();

    assert_eq!(
        nearby,
        vec!["The Met"],
        "only The Met sits within 2 km of Central Park"
    );
}

/// A wider radius admits more candidates; a tighter radius admits fewer —
/// monotonic in the radius parameter.
#[test]
fn geo_radius_is_monotonic_in_radius() {
    let centre = GeoCoordinates::new(40.7829, -73.9654);
    let collection = [
        place_at("The Met", 40.7794, -73.9632),
        place_at("Times Square", 40.7580, -73.9855),
        place_at("Statue of Liberty", 40.6892, -74.0445),
    ];

    let count_within = |radius_m: f64| {
        collection
            .iter()
            .filter(|p| {
                p.geo
                    .as_ref()
                    .is_some_and(|g| within_radius(&centre, g, radius_m))
            })
            .count()
    };

    let tight = count_within(1_000.0);
    let medium = count_within(5_000.0);
    let wide = count_within(20_000.0);

    assert!(
        tight <= medium,
        "tight {tight} should be <= medium {medium}"
    );
    assert!(medium <= wide, "medium {medium} should be <= wide {wide}");
    assert_eq!(wide, 3, "all three NYC places fall within 20 km");
}

/// Matcher-bridge worked example: two near-duplicate records score as a
/// confident match through the canonical engine. This is the runnable
/// counterpart of the snippet in `agents/restful.md`.
#[test]
fn matcher_bridge_worked_example_scores_near_duplicate() {
    let engine = MatchingEngine::new(MatchConfig::default());

    let mut a = Place::new("Central Park");
    a.geo = Some(GeoCoordinates::new(40.7829, -73.9654));

    let mut b = Place::new("Centrl Park"); // typo
    b.geo = Some(GeoCoordinates::new(40.7830, -73.9655)); // a few metres away

    let result = engine.match_places(&to_matcher_place(&a), &to_matcher_place(&b));

    assert!(
        result.score >= 0.80,
        "near-duplicate score should be high, got {}",
        result.score
    );
    assert!(result.is_match, "near-duplicate should be flagged a match");
    assert!(
        matches!(result.confidence, Confidence::High | Confidence::Medium),
        "confidence should be High or Medium, got {:?}",
        result.confidence
    );
}

/// Two unrelated places (different name, opposite coast) are not a match.
#[test]
fn matcher_bridge_worked_example_rejects_unrelated() {
    let engine = MatchingEngine::new(MatchConfig::default());

    let mut a = Place::new("Central Park");
    a.geo = Some(GeoCoordinates::new(40.7829, -73.9654));

    let mut b = Place::new("Griffith Observatory");
    b.geo = Some(GeoCoordinates::new(34.1184, -118.3004));

    let result = engine.match_places(&to_matcher_place(&a), &to_matcher_place(&b));

    assert!(
        !result.is_match,
        "unrelated places should not match, got score {}",
        result.score
    );
}
