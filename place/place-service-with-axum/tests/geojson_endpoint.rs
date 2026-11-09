//! `GET /api/places/{id}/geojson` answers one place as an RFC 7946
//! `Feature`, and `GET /api/places/search.geojson?bbox=` the places in a
//! box as a paged `FeatureCollection`, both as `application/geo+json`
//! (spec §13 T-7).
//!
//! `#[ignore]`d — run explicitly with
//! `DATABASE_URL=… cargo test --test geojson_endpoint -- --ignored`.

use std::str::FromStr;

use axa::testing::TestApp;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use bigdecimal::BigDecimal;
use place_service::app::App;
use place_service::config::Config;
use place_service::db::{PlaceRepository, create_connection};
use place_service::models::geo::GeoCoordinates;
use place_service::models::place::Place;
use serde_json::Value;
use tower::ServiceExt;

async fn test_router() -> axum::Router {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    App::boot_test()
        .await
        .expect("boot the app: set DATABASE_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router")
}

/// A place at `(lat, lon)` with a name unique to this run.
fn at(tag: &str, lat: &str, lon: &str) -> Place {
    let mut place = Place::new(&format!("GeoJSON {tag} {}", uuid::Uuid::new_v4()));
    place.geo = Some(GeoCoordinates {
        latitude_as_decimal_degrees: BigDecimal::from_str(lat).unwrap(),
        longitude_as_decimal_degrees: BigDecimal::from_str(lon).unwrap(),
        elevation_as_decimal_metres: None,
    });
    place
}

/// Every feature id across all pages of `uri`, following `rel="next"`.
async fn walk(app: &axum::Router, uri: &str) -> Vec<String> {
    let mut ids = Vec::new();
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
            .map(|v| v.to_str().unwrap())
            .and_then(|link| link.strip_prefix('<'))
            .and_then(|link| link.split_once('>'))
            .map(|(target, _)| target.to_owned());
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["type"], "FeatureCollection");
        for feature in body["features"].as_array().unwrap() {
            ids.push(feature["id"].as_str().unwrap().to_owned());
        }
    }
    ids
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Option<String>, Value) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_owned());
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, content_type, body)
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test geojson_endpoint -- --ignored"]
async fn a_stored_place_is_served_as_a_point_feature() {
    let app = test_router().await;
    let config = Config::from_env().expect("load config from env");
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection");
    let repo = place_service::db::PgPlaceRepository::new(db);

    // In the Tasman Sea, far from every other suite's fixtures, so this
    // place never lands on a page another test reads nearest-first.
    let mut place = Place::new(&format!("GeoJSON Fixture {}", uuid::Uuid::new_v4()));
    place.geo = Some(GeoCoordinates {
        latitude_as_decimal_degrees: BigDecimal::from_str("-38.123456").unwrap(),
        longitude_as_decimal_degrees: BigDecimal::from_str("148.765432").unwrap(),
        elevation_as_decimal_metres: None,
    });
    let stored = repo.create(&place).await.expect("seed a located place");
    let unlocated = repo
        .create(&Place::new(&format!(
            "GeoJSON Unlocated {}",
            uuid::Uuid::new_v4()
        )))
        .await
        .expect("seed an unlocated place");

    let (status, content_type, body) =
        get(&app, &format!("/api/places/{}/geojson", stored.id)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(content_type.as_deref(), Some("application/geo+json"));
    assert_eq!(body["type"], "Feature");
    assert_eq!(body["id"], stored.id.to_string());
    assert_eq!(body["geometry"]["type"], "Point");
    // Longitude first, and exactly the digits the plain read answers
    // with (the column's scale pads them), not an f64 rounding of them.
    let (_, _, plain) = get(&app, &format!("/api/places/{}", stored.id)).await;
    assert_eq!(
        body["geometry"]["coordinates"],
        serde_json::json!([
            plain["geo"]["longitude_as_decimal_degrees"],
            plain["geo"]["latitude_as_decimal_degrees"],
        ])
    );
    assert_eq!(
        body["geometry"]["coordinates"][0].as_f64(),
        Some(148.765_432)
    );
    assert_eq!(body["properties"]["name"], stored.name);

    let (status, _, body) = get(&app, &format!("/api/places/{}/geojson", unlocated.id)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body["geometry"].is_null(), "{body}");

    let (status, _, _) = get(
        &app,
        &format!("/api/places/{}/geojson", uuid::Uuid::new_v4()),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test geojson_endpoint -- --ignored"]
async fn a_bbox_pages_every_place_inside_it_and_none_outside() {
    let app = test_router().await;
    let config = Config::from_env().expect("load config from env");
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection");
    let repo = place_service::db::PgPlaceRepository::new(db);

    // A box in the South Atlantic no other test seeds into, and a second
    // across the antimeridian in the Pacific.
    let mut inside = Vec::new();
    for i in 0..5 {
        let p = repo
            .create(&at("Inside", &format!("-47.{i}"), "-23.5"))
            .await
            .unwrap();
        inside.push(p.id.to_string());
    }
    let edge = repo.create(&at("Edge", "-47.9", "-23.0")).await.unwrap();
    inside.push(edge.id.to_string());
    let outside = repo.create(&at("Outside", "-47.5", "-22.9")).await.unwrap();
    let deleted = repo.create(&at("Deleted", "-47.5", "-23.5")).await.unwrap();
    repo.soft_delete(&deleted.id).await.unwrap();
    let east = repo.create(&at("East", "-61.0", "179.5")).await.unwrap();
    let west = repo.create(&at("West", "-61.0", "-179.5")).await.unwrap();
    let middle = repo.create(&at("Middle", "-61.0", "0.0")).await.unwrap();

    // Pages of two: the walk follows every Link and finds each place once.
    // The database is shared across runs and test binaries, so the box
    // may also hold earlier runs' places: check this run's, and that the
    // walk never repeats a feature.
    let found = walk(
        &app,
        "/api/places/search.geojson?bbox=-24,-48,-23,-47&limit=2",
    )
    .await;
    let distinct: std::collections::HashSet<&String> = found.iter().collect();
    assert_eq!(
        distinct.len(),
        found.len(),
        "a feature repeated across pages"
    );
    for id in &inside {
        assert!(found.contains(id), "{id} is in the box, edges included");
    }
    assert!(!found.contains(&outside.id.to_string()));
    assert!(!found.contains(&deleted.id.to_string()), "soft-deleted");

    let across = walk(&app, "/api/places/search.geojson?bbox=179,-62,-179,-60").await;
    assert!(across.contains(&east.id.to_string()));
    assert!(across.contains(&west.id.to_string()));
    assert!(
        !across.contains(&middle.id.to_string()),
        "antimeridian box is not the whole globe"
    );

    let (status, content_type, _) =
        get(&app, "/api/places/search.geojson?bbox=-24,-48,-23,-47").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(content_type.as_deref(), Some("application/geo+json"));

    let (status, _, body) = get(&app, "/api/places/search.geojson?bbox=0,10,1,5").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["code"], "invalid_bbox");
    let (status, _, _) = get(&app, "/api/places/search.geojson?bbox=0,0,1,1&limit=101").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}
