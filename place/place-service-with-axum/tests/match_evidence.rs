//! Match evidence through the service (`agents/share/match-evidence.md`
//! §4, §10 step 4; spec §13 T-23).
//!
//! - The batch scan stores and returns the matcher's evidence with each
//!   review item, and marks a pair `automerged` only when its score clears
//!   `auto_merge_threshold` **and** its evidence is `proven`. A pair that
//!   agrees only on its name, or whose identifiers conflict under one
//!   scheme, stays `pending` at any score.
//! - `check-duplicates` and the create-time `409` carry evidence, masked or
//!   not, and the evidence holds no field values.
//!
//! Seeds sit at a random remote centre and carry a per-run unique name, so
//! rows that accumulate in the shared test database across runs never
//! pair with these.
//!
//! `#[ignore]`d — run explicitly with
//! `DATABASE_URL=… cargo test --test match_evidence -- --ignored`.

use axa::testing::TestApp;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use place_service::app::App;
use place_service::config::Config;
use place_service::db::{PgPlaceRepository, PlaceRepository, create_connection};
use place_service::matching::scoring::{MatchWeights, compute_match};
use place_service::models::address::PostalAddress;
use place_service::models::geo::GeoCoordinates;
use place_service::models::identifier::{IdentifierType, PlaceIdentifier};
use place_service::models::place::Place;
use place_service::models::place_type::PlaceType;
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

/// The thresholds every scan here uses. 0.9 keeps the scan from pairing
/// the shared database's short names (see `review_queue_score_breakdown`).
const THRESHOLD: f64 = 0.9;
const AUTO_MERGE: f64 = 0.9;

async fn test_router() -> axum::Router {
    let config = Config::from_env().expect("load config from env");
    std::fs::create_dir_all(&config.search.index_path).expect("create search index dir");
    App::boot_test()
        .await
        .expect("boot the app: set DATABASE_URL to a running PostgreSQL")
        .router
        .expect("boot_test builds the router")
}

async fn repo() -> PgPlaceRepository {
    let config = Config::from_env().expect("load config from env");
    PgPlaceRepository::new(
        create_connection(&config.database)
            .await
            .expect("Postgres connection"),
    )
}

async fn send(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let builder = Request::builder().method(method).uri(uri);
    let request = match body {
        Some(b) => builder
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(b).unwrap())),
        None => builder.body(Body::empty()),
    }
    .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// A random point in the Southern Ocean, far from any fixture elsewhere.
fn remote_geo() -> GeoCoordinates {
    let b = Uuid::new_v4().into_bytes();
    let lat = -50.0 - f64::from(b[0]) / 255.0 * 10.0;
    let lon = 100.0 + f64::from(b[1]) / 255.0 * 60.0;
    GeoCoordinates {
        latitude_as_decimal_degrees: format!("{lat:.6}").parse().unwrap(),
        longitude_as_decimal_degrees: format!("{lon:.6}").parse().unwrap(),
        elevation_as_decimal_metres: None,
    }
}

fn unique_name(label: &str) -> String {
    format!("Evidence {label} {}", Uuid::new_v4().simple())
}

/// Thirteen digits that are unique per run (not a valid check digit; the
/// matcher compares values, the repository does not validate).
fn unique_gln() -> String {
    let n = Uuid::new_v4().as_u128() % 10_000_000_000_000;
    format!("{n:013}")
}

fn is_pair(item: &Value, a: Uuid, b: Uuid) -> bool {
    let (a, b) = (a.to_string(), b.to_string());
    let ia = item["place_id_a"].as_str().unwrap_or_default();
    let ib = item["place_id_b"].as_str().unwrap_or_default();
    (ia == a && ib == b) || (ia == b && ib == a)
}

/// Seed `a` and `b`, run the scan, and return the scan's item and the
/// stored item for their pair.
async fn scan_pair(app: &axum::Router, a: &Place, b: &Place) -> (Value, Value) {
    let repo = repo().await;
    let a = repo.create(a).await.expect("seed A");
    let b = repo.create(b).await.expect("seed B");
    let (status, body) = send(
        app,
        "POST",
        "/api/places/deduplicate",
        Some(&json!({
            "threshold": THRESHOLD,
            "auto_merge_threshold": AUTO_MERGE,
            "max_candidates": 1000
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let scanned = body["review_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| is_pair(i, a.id, b.id))
        .cloned()
        .unwrap_or_else(|| panic!("seeded pair not in the scan: {body}"));
    let (status, body) = send(app, "GET", "/api/places/review-queue?limit=500", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let stored = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| is_pair(i, a.id, b.id))
        .cloned()
        .unwrap_or_else(|| panic!("seeded pair not in the stored queue: {body}"));
    (scanned, stored)
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test match_evidence -- --ignored"]
async fn a_name_only_pair_above_auto_merge_stays_pending_and_consistent() {
    let app = test_router().await;
    let name = unique_name("Consistent");
    let (a, b) = (Place::new(&name), Place::new(&name));
    let score = compute_match(&a, &b, &MatchWeights::default()).score;
    assert!(
        score >= AUTO_MERGE,
        "fixture must clear auto-merge: {score}"
    );

    let (scanned, stored) = scan_pair(&app, &a, &b).await;
    for item in [&scanned, &stored] {
        assert_eq!(item["status"], "pending", "{item}");
        assert_eq!(item["evidence"]["strength"], "consistent", "{item}");
        assert!(
            item["evidence"]["weight_bits"].as_f64().unwrap() < 30.0,
            "{item}"
        );
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test match_evidence -- --ignored"]
async fn a_shared_gln_pair_is_automerged_and_proven() {
    let app = test_router().await;
    let name = unique_name("Proven");
    let gln = unique_gln();
    let mut a = Place::new(&name);
    a.identifiers = vec![PlaceIdentifier::gln(&gln)];
    let mut b = Place::new(&name);
    b.identifiers = vec![PlaceIdentifier::gln(&gln)];

    let (scanned, stored) = scan_pair(&app, &a, &b).await;
    for item in [&scanned, &stored] {
        assert_eq!(item["status"], "automerged", "{item}");
        assert_eq!(item["evidence"]["strength"], "proven", "{item}");
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test match_evidence -- --ignored"]
async fn conflicting_osm_ids_stay_pending_and_contradicted() {
    let app = test_router().await;
    let name = unique_name("Contradicted");
    let geo = remote_geo();
    let address = PostalAddress {
        street_address: Some("1 Evidence Way".into()),
        address_locality: Some("Nowhere".into()),
        address_region: None,
        address_country: Some("AQ".into()),
        postal_code: Some("0000".into()),
    };
    let make = |osm: &str| {
        let mut p = Place::new(&name);
        p.geo = Some(geo.clone());
        p.address = Some(address.clone());
        p.place_type = Some(PlaceType::Park);
        p.identifiers = vec![PlaceIdentifier::new(IdentifierType::OpenStreetMap, osm)];
        p
    };
    let a = make(&format!("node:{}", Uuid::new_v4().simple()));
    let b = make(&format!("node:{}", Uuid::new_v4().simple()));
    let score = compute_match(&a, &b, &MatchWeights::default()).score;
    assert!(
        score >= AUTO_MERGE,
        "fixture must clear auto-merge: {score}"
    );

    let (scanned, stored) = scan_pair(&app, &a, &b).await;
    for item in [&scanned, &stored] {
        assert_eq!(item["status"], "pending", "{item}");
        assert_eq!(item["evidence"]["strength"], "contradicted", "{item}");
        assert_eq!(
            item["evidence"]["contradictions"][0]["field"], "identifier:osm_node",
            "{item}"
        );
    }
}

#[tokio::test]
#[ignore = "requires PostgreSQL; run with DATABASE_URL=… cargo test --test match_evidence -- --ignored"]
async fn check_duplicates_and_the_409_carry_value_free_evidence() {
    let app = test_router().await;
    let name = unique_name("Probe");
    let gln = unique_gln();
    let mut seed = Place::new(&name);
    seed.identifiers = vec![PlaceIdentifier::gln(&gln)];
    seed.telephone = Some("+1-555-010-4242".into());
    let seed_json = serde_json::to_value(&seed).unwrap();
    let (status, body) = send(&app, "POST", "/api/places", Some(&seed_json)).await;
    assert_eq!(status, StatusCode::CREATED, "{body}");

    let mut probe = Place::new(&name);
    probe.identifiers = vec![PlaceIdentifier::gln(&gln)];
    let probe_json = serde_json::to_value(&probe).unwrap();

    for uri in [
        "/api/places/check-duplicates",
        "/api/places/check-duplicates?mask_sensitive=true",
    ] {
        let (status, body) = send(&app, "POST", uri, Some(&probe_json)).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let hit = body["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["place"]["name"] == name.as_str())
            .cloned()
            .unwrap_or_else(|| panic!("seed not among candidates: {body}"));
        assert_eq!(hit["evidence"]["strength"], "proven", "{hit}");
        let evidence = hit["evidence"].to_string();
        assert!(
            !evidence.contains(&gln) && !evidence.contains(&name) && !evidence.contains("4242")
        );
    }

    let (status, body) = send(
        &app,
        "POST",
        "/api/places?mask_sensitive=true",
        Some(&probe_json),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let hit = &body["errors"][0];
    assert_eq!(hit["evidence"]["strength"], "proven", "{body}");
    assert!(!hit["evidence"].to_string().contains(&gln));
}
