//! `GET /api/places/nearby` filters, orders and pages in the database
//! (spec §13 T-21): following `next` reaches every place in the radius,
//! nearest first and then by `id`, and a circle across the antimeridian
//! finds both sides.
//!
//! Each run seeds around a centre of its own, picked at random away from
//! the poles and the antimeridian, so the walks can be compared exactly
//! with what the run seeded even though the database is shared.
//!
//! `#[ignore]`d — run explicitly with
//! `DATABASE_URL=… cargo test --test nearby_paging -- --ignored`.

use std::str::FromStr;

use axa_test::TestApp;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use bigdecimal::BigDecimal;
use place_service::app::App;
use place_service::config::Config;
use place_service::db::{PgPlaceRepository, PlaceRepository, create_connection};
use place_service::models::geo::GeoCoordinates;
use place_service::models::place::Place;
use serde_json::Value;
use tower::ServiceExt;

/// Kilometres per degree of latitude on the service's 6 371 km sphere.
const KM_PER_DEGREE: f64 = std::f64::consts::PI * 6371.0 / 180.0;

async fn setup() -> (axum::Router, PgPlaceRepository) {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    let app = App::boot_test()
        .await
        .expect("boot the app: set DATABASE_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router");
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection");
    (app, PgPlaceRepository::new(db))
}

/// Round to the six decimal places validation allows.
fn round6(v: f64) -> f64 {
    (v * 1_000_000.0).round() / 1_000_000.0
}

async fn seed(repo: &PgPlaceRepository, lat: f64, lon: f64) -> String {
    let mut place = Place::new(&format!("Nearby {}", uuid::Uuid::new_v4()));
    place.geo = Some(GeoCoordinates {
        latitude_as_decimal_degrees: BigDecimal::from_str(&round6(lat).to_string()).unwrap(),
        longitude_as_decimal_degrees: BigDecimal::from_str(&round6(lon).to_string()).unwrap(),
        elevation_as_decimal_metres: None,
    });
    repo.create(&place).await.expect("seed").id.to_string()
}

/// Every result across all pages of `uri`, as `(id, lat, lon)`.
async fn walk(app: &axum::Router, uri: &str) -> Vec<(String, f64, f64)> {
    let mut out = Vec::new();
    let mut next = Some(uri.to_owned());
    while let Some(uri) = next.take() {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{uri}");
        next = response
            .headers()
            .get(header::LINK)
            .and_then(|v| v.to_str().ok())
            .and_then(|link| link.strip_prefix('<'))
            .and_then(|link| link.split_once('>'))
            .map(|(target, _)| target.to_owned());
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        for p in body["results"].as_array().unwrap() {
            let num = |v: &Value| v.as_f64().unwrap();
            out.push((
                p["id"].as_str().unwrap().to_owned(),
                num(&p["geo"]["latitude_as_decimal_degrees"]),
                num(&p["geo"]["longitude_as_decimal_degrees"]),
            ));
        }
    }
    out
}

fn distance_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    GeoCoordinates::new(lat1, lon1).distance_to(&GeoCoordinates::new(lat2, lon2))
}

/// A pseudo-random value in `0..1` from a fresh UUID.
fn unit() -> f64 {
    let bytes = uuid::Uuid::new_v4().into_bytes();
    f64::from(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])) / f64::from(u32::MAX)
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test nearby_paging -- --ignored"]
async fn following_next_reaches_every_place_in_the_radius_nearest_first() {
    let (app, repo) = setup().await;
    let lat = round6(-50.0 + 100.0 * unit());
    let lon = round6(-170.0 + 340.0 * unit());

    // Rings at 0.5, 1, 1.5 … km due north, four places on the 2 km ring
    // so distance ties are broken by id, and two beyond the 5 km radius.
    let mut inside = Vec::new();
    for ring in 1..=8 {
        let km = f64::from(ring) * 0.5;
        inside.push(seed(&repo, lat + km / KM_PER_DEGREE, lon).await);
    }
    let dlon = 2.0 / (KM_PER_DEGREE * lat.to_radians().cos());
    for _ in 0..3 {
        inside.push(seed(&repo, lat, lon + dlon).await);
    }
    let outside = [
        seed(&repo, lat + 5.5 / KM_PER_DEGREE, lon).await,
        seed(&repo, lat - 7.0 / KM_PER_DEGREE, lon).await,
    ];

    let found = walk(
        &app,
        &format!("/api/places/nearby?lat={lat}&lon={lon}&radius_km=5&limit=3"),
    )
    .await;

    let mut found_ids: Vec<String> = found.iter().map(|(id, _, _)| id.clone()).collect();
    let mut expected = inside.clone();
    found_ids.sort();
    expected.sort();
    assert_eq!(found_ids, expected, "every place within 5 km, once");
    for id in &outside {
        assert!(!found_ids.contains(id), "{id} is beyond the radius");
    }

    // Nearest first, and equal distances in id order.
    for pair in found.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let (da, db) = (
            distance_m(lat, lon, a.1, a.2),
            distance_m(lat, lon, b.1, b.2),
        );
        assert!(da <= db + 1e-6, "{a:?} at {da} m before {b:?} at {db} m");
        if (da - db).abs() < 1e-6 {
            assert!(a.0 < b.0, "a tie is in id order: {a:?} {b:?}");
        }
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test nearby_paging -- --ignored"]
async fn a_circle_across_the_antimeridian_finds_both_sides() {
    let (app, repo) = setup().await;
    // A random latitude, so each run's places are its own.
    let lat = round6(-40.0 + 80.0 * unit());
    let dlon = 3.0 / (KM_PER_DEGREE * lat.to_radians().cos());
    let east = seed(&repo, lat, 179.999).await;
    let west = seed(&repo, lat, -180.0 + dlon).await;
    let far = seed(&repo, lat, 170.0).await;

    let found: Vec<String> = walk(
        &app,
        &format!("/api/places/nearby?lat={lat}&lon=179.999&radius_km=5&limit=50"),
    )
    .await
    .into_iter()
    .map(|(id, _, _)| id)
    .collect();
    assert!(found.contains(&east), "the centre's own side");
    assert!(found.contains(&west), "across the antimeridian, ~3 km away");
    assert!(!found.contains(&far));
}
