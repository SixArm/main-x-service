//! `GET /api/places/{id}/geojson` answers one place as an RFC 7946
//! `Feature`, as `application/geo+json` (spec §13 T-7).
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

    let mut place = Place::new(&format!("GeoJSON Fixture {}", uuid::Uuid::new_v4()));
    place.geo = Some(GeoCoordinates {
        latitude_as_decimal_degrees: BigDecimal::from_str("40.785091").unwrap(),
        longitude_as_decimal_degrees: BigDecimal::from_str("-73.968285").unwrap(),
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
        Some(-73.968_285)
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
