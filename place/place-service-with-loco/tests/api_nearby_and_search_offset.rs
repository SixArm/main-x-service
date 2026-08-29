//! REST API integration tests requiring a running `PostgreSQL`.
//!
//! `#[ignore]`d — run explicitly with
//! `DATABASE_URL=… cargo test --test api_nearby_and_search_offset -- --ignored`.
//!
//! This file's reason to exist is **T-9**
//! (`spec/13-tasks.md`): the geo-radius `GET /api/places/nearby` HTTP
//! endpoint and `offset` support on `GET /api/places/search`, both
//! exercised end-to-end against a real database and search index —
//! `tests/integration_geo_radius.rs` pins the `within_radius` primitive
//! in isolation, this file pins the wired HTTP surface.

use std::sync::OnceLock;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use place_service::api::rest::{AppState, create_router};
use place_service::config::Config;
use place_service::db::create_connection;
use place_service::matching::PlaceMatcher;
use place_service::search::SearchEngine;
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;

/// `test_router` mutates the process-wide `SEARCH_INDEX_PATH`
/// environment variable so each test gets its own private search index
/// (see its doc comment). The default test harness runs every
/// `#[tokio::test]` fn in this file concurrently on separate threads,
/// and an environment variable is process state, not per-thread state —
/// two tests racing `test_router()` could each load `Config::from_env()`
/// after the *other's* `set_var`, pointing both at the same directory
/// (or, worse, reading it mid-write). Every test that calls
/// `test_router()` holds this lock first, serializing the whole file on
/// that narrow section; this makes the file's outcome independent of
/// `--test-threads`, not merely correct under the `--test-threads=1`
/// this crate's DB-gated suite happens to run under.
fn search_index_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

/// Build the real router against the environment-configured database and
/// a **fresh, private** temp search index. No auth env vars are touched,
/// so `PLACE_REQUIRE_AUTH` stays at its default (off).
///
/// The environment's default `SEARCH_INDEX_PATH`
/// (`./data/search_index`) is a single directory shared by every local
/// run of every crate test — including other agents/sessions on this
/// same machine running concurrently — and it is never reset between
/// runs. Two writers racing for the Tantivy writer lock on that shared
/// directory silently drops one side's index write (`create_place`
/// discards `index_place`'s error, matching production: an unindexed
/// row still committed to Postgres, so failing the request outright
/// would be worse), which showed up here as a search returning zero
/// hits for rows that really were created. Overriding
/// `SEARCH_INDEX_PATH` to a private [`TempDir`] per test call removes
/// the shared resource entirely — the returned guard must be kept
/// alive for as long as `app` is used, or the directory is deleted out
/// from under it.
async fn test_router() -> (axum::Router, TempDir) {
    let index_dir = TempDir::new().expect("create private search index dir");
    // SAFETY: this test binary runs its tests with `--test-threads=1`
    // (`ci-check.sh test-db`), so no other thread in this process reads
    // the environment concurrently with this write.
    unsafe {
        std::env::set_var("SEARCH_INDEX_PATH", index_dir.path());
    }
    let config = Config::from_env().expect("load config from env");
    let search_engine = SearchEngine::new(&config.search.index_path).expect("search engine");
    let matcher = PlaceMatcher::new(&config.matching);
    let db = create_connection(&config.database)
        .await
        .expect("Postgres connection — set DATABASE_URL to a running, migrated database");
    let app = create_router(AppState::new(db, search_engine, matcher, config));
    (app, index_dir)
}

/// `POST` a hand-built JSON body, returning the status **and** the parsed
/// body.
async fn post_places(app: &axum::Router, body: &Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/places")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, parsed)
}

/// `GET` a path, returning the status, the parsed body, **and** the
/// `X-Total-Count` / `X-Limit` / `X-Offset` pagination headers
/// (`agents/share/restful.md`) as `(total, limit, offset)`, each `None`
/// if the header was absent or unparseable.
async fn get_json(
    app: &axum::Router,
    uri: &str,
) -> (StatusCode, Value, (Option<u64>, Option<u64>, Option<u64>)) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let header = |name: &str| {
        response
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
    };
    let headers = (
        header("x-total-count"),
        header("x-limit"),
        header("x-offset"),
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, parsed, headers)
}

/// A place at a fixed name + coordinate, posted via the real create path
/// (so it goes through validation, persistence, and search indexing
/// exactly like a real client's write).
async fn create_place_at(app: &axum::Router, name: &str, lat: f64, lon: f64) -> Value {
    let (status, body) = post_places(
        app,
        &json!({
            "name": name,
            "geo": { "latitude_as_decimal_degrees": lat, "longitude_as_decimal_degrees": lon },
        }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create failed for {name}: {body}"
    );
    body["data"].clone()
}

/// `GET /api/places/nearby` returns only the places within `radius_km`
/// of `(lat, lon)` — the T-9 acceptance test verbatim: post places, then
/// confirm `nearby` filters to the ones actually inside the radius.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn nearby_returns_only_places_within_radius() {
    let _guard = search_index_lock().lock().await;
    let (app, _index_dir) = test_router().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();

    // Centre: Central Park, NYC. One place a few hundred metres away
    // (inside a 2 km radius), one a continent away (outside).
    let close_name = format!("NearbyCloseA{suffix}");
    let far_name = format!("NearbyFarA{suffix}");
    let close = create_place_at(&app, &close_name, 40.7794, -73.9632).await; // ~0.4 km
    let far = create_place_at(&app, &far_name, 34.0537, -118.2428).await; // continent away

    let (status, body, _headers) = get_json(
        &app,
        "/api/places/nearby?lat=40.7829&lon=-73.9654&radius_km=2",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let names: Vec<String> = body["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect();

    assert!(
        names.contains(&close_name),
        "expected {close_name} within 2 km, got {names:?}"
    );
    assert!(
        !names.contains(&far_name),
        "did not expect {far_name} (continent away) within 2 km, got {names:?}"
    );
    // Sanity: the ids round-trip too, not just the names.
    assert_eq!(close["name"], close_name);
    assert_eq!(far["name"], far_name);
}

/// A place a hair inside the requested radius is included; one a hair
/// outside is excluded — pins the bounding-box pre-filter + exact
/// Haversine check together right at the edge, where a pre-filter bug
/// (too tight a box) would drop a true positive. (The exact-boundary
/// `<=` cutoff itself is pinned precisely, without JSON/validation
/// rounding in the way, by
/// `matching::geo::tests::within_radius_boundary_is_inclusive`.)
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn nearby_near_the_edge_of_the_radius() {
    let _guard = search_index_lock().lock().await;
    let (app, _index_dir) = test_router().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let inside_name = format!("NearbyEdgeInside{suffix}");
    let outside_name = format!("NearbyEdgeOutside{suffix}");

    // 1 degree of latitude is ~111.19 km on the sphere `distance_to`
    // uses. Move due north by 95% / 105% of the requested radius,
    // rounded to 6 decimal places so `MAX_COORDINATE_SCALE` validation
    // never enters into it.
    let lat = 10.0;
    let lon = 0.0;
    let radius_km = 5.0;
    let km_per_degree = std::f64::consts::PI * 6371.0 / 180.0;
    let round6 = |v: f64| (v * 1_000_000.0).round() / 1_000_000.0;
    let inside_lat = round6(lat + 0.95 * radius_km / km_per_degree);
    let outside_lat = round6(lat + 1.05 * radius_km / km_per_degree);

    create_place_at(&app, &inside_name, inside_lat, lon).await;
    create_place_at(&app, &outside_name, outside_lat, lon).await;

    let (status, body, _headers) = get_json(
        &app,
        &format!("/api/places/nearby?lat={lat}&lon={lon}&radius_km={radius_km}&limit=50"),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let names: Vec<String> = body["data"]["results"]
        .as_array()
        .expect("results array")
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect();
    assert!(
        names.contains(&inside_name),
        "expected {inside_name} (95% of radius) to be included, got {names:?}"
    );
    assert!(
        !names.contains(&outside_name),
        "did not expect {outside_name} (105% of radius) to be included, got {names:?}"
    );
}

/// Out-of-range `lat`/`lon`/`radius_km` on `nearby` is a `400`, not a
/// silently-empty result.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn nearby_rejects_out_of_range_coordinates() {
    let _guard = search_index_lock().lock().await;
    let (app, _index_dir) = test_router().await;
    let (status, body, _headers) =
        get_json(&app, "/api/places/nearby?lat=999&lon=0&radius_km=1").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}

/// `GET /api/places/search?offset=` skips the requested number of
/// results — the other T-9 acceptance test verbatim — and the
/// `X-Total-Count` header reports the true total, unaffected by the
/// page window.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn search_offset_skips_results_and_reports_true_total() {
    let _guard = search_index_lock().lock().await;
    let (app, _index_dir) = test_router().await;
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    // A **separator** before the uuid matters: Tantivy's default
    // tokenizer drops any single unbroken token over 40 characters
    // (`search::tests::overlong_single_token_is_not_indexed` pins this),
    // and "Offsetville" directly against a 32-hex-char uuid with no
    // separator is 43 — a real single-word doc name could hit the same
    // cutoff, but this test's *fixture* just needs to not accidentally
    // pick that shape.
    let token = format!("Offsetville-{suffix}");

    // Three places sharing a unique, unindexed-elsewhere token so the
    // search is scoped to exactly this test's rows.
    for n in 0..3 {
        create_place_at(&app, &format!("{token} Park {n}"), 10.0 + f64::from(n), 0.0).await;
    }

    let (status0, body0, headers0) = get_json(
        &app,
        &format!("/api/places/search?q={token}&limit=2&offset=0"),
    )
    .await;
    assert_eq!(status0, StatusCode::OK, "{body0}");
    let page0: Vec<String> = body0["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(page0.len(), 2, "page0={page0:?} full_body={body0}");
    assert_eq!(
        headers0.0,
        Some(3),
        "X-Total-Count should be the true total"
    );
    assert_eq!(headers0.1, Some(2), "X-Limit should echo the applied limit");
    assert_eq!(
        headers0.2,
        Some(0),
        "X-Offset should echo the applied offset"
    );

    let (status2, body2, headers2) = get_json(
        &app,
        &format!("/api/places/search?q={token}&limit=2&offset=2"),
    )
    .await;
    assert_eq!(status2, StatusCode::OK, "{body2}");
    let page2: Vec<String> = body2["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        page2.len(),
        1,
        "the third row, skipping the first two: {page2:?}"
    );
    assert_eq!(headers2.0, Some(3));
    assert_eq!(headers2.2, Some(2));

    // The two pages together cover every row exactly once.
    let mut all: Vec<String> = page0.into_iter().chain(page2).collect();
    all.sort();
    all.dedup();
    assert_eq!(all.len(), 3, "expected all three rows across both pages");
}

/// An `offset` beyond the bound is a `400` (SEC-G7) on both endpoints —
/// the database (or, for `nearby`, the in-process filter) is never asked
/// to materialise and discard an unbounded number of rows.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn offset_beyond_the_bound_is_rejected_on_both_endpoints() {
    let _guard = search_index_lock().lock().await;
    let (app, _index_dir) = test_router().await;

    let (status, body, _headers) =
        get_json(&app, "/api/places/search?q=anything&offset=1000000").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");

    let (status, body, _headers) = get_json(
        &app,
        "/api/places/nearby?lat=0&lon=0&radius_km=1&offset=1000000",
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
}
