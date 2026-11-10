#![warn(clippy::pedantic)]

//! Duplicate detection, merge and privacy through the real router and a
//! real database (spec §13 T-5).
//!
//! Each test names its workers with a fresh UUID, so a run finds its own
//! records among whatever earlier runs left in a shared database.
//!
//! `#[ignore]`d — run with
//! `DB_URL=… cargo test --test dedup_merge_privacy -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<&Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(b) => {
            builder = builder.header("content-type", "application/json");
            Body::from(serde_json::to_vec(b).unwrap())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// A worker body: a family name unique to this run, a fixed given name,
/// birth date and gender, and one MRN. `worker_identifiers` is unique on
/// `(system, value)` across the table, so each MRN is unique to the run.
fn worker(family: &str, mrn: &str) -> Value {
    json!({
        "name": { "family": family, "given": ["Margaret"] },
        "gender": "female",
        "birth_date": "1979-04-12",
        "identifiers": [{ "identifier_type": "MRN", "system": "urn:example:mrn", "value": mrn }],
    })
}

/// The second create of an identical worker is refused by the duplicate
/// check, before the insert. Since T-23 the score is the matcher's,
/// renormalised over the components both workers hold, so identical name,
/// birth date and gender score `1.0` and the check flags them; it used to
/// count every component neither worker has as `0`, score `0.75`, miss
/// them, and leave the identifier's unique key to refuse the insert. The
/// `409` carries the candidate's evidence: consistent, not proven, because
/// the MRN routes to no unique scheme and a name and birth date are not
/// proof (`agents/share/match-evidence.md`).
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn creating_the_same_worker_twice_is_a_409() {
    let app = common::create_test_router().await;
    let family = format!("Dedupe{}", Uuid::new_v4().simple());
    let body = worker(&family, &format!("MRN-{}", Uuid::new_v4().simple()));

    let (status, first) = call(&app, "POST", "/api/workers", Some(&body)).await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let (status, second) = call(&app, "POST", "/api/workers", Some(&body)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{second}");
    assert_eq!(second["code"], "duplicate_detected", "{second}");
    let candidate = second["errors"]["potential_matches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["worker"]["id"] == first["id"])
        .unwrap_or_else(|| panic!("the first worker is a candidate: {second}"));
    assert!(candidate["score"].as_f64().unwrap() > 0.99, "{candidate}");
    assert_eq!(candidate["quality"], "certain", "{candidate}");
    assert_eq!(
        candidate["evidence"]["strength"], "consistent",
        "{candidate}"
    );
    assert!(
        candidate["evidence"]["components"]
            .as_array()
            .unwrap()
            .len()
            >= 3,
        "{candidate}"
    );
}

/// A different worker carrying an identifier another worker already holds
/// is a `409` naming the key, not a `500`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn an_identifier_another_worker_holds_is_a_409_not_a_500() {
    let app = common::create_test_router().await;
    let mrn = format!("MRN-{}", Uuid::new_v4().simple());
    let first = worker(&format!("Holder{}", Uuid::new_v4().simple()), &mrn);
    let (status, _) = call(&app, "POST", "/api/workers", Some(&first)).await;
    assert_eq!(status, StatusCode::CREATED);
    let mut other = worker(&format!("Other{}", Uuid::new_v4().simple()), &mrn);
    other["name"]["given"] = json!(["Theodore"]);
    other["birth_date"] = json!("1951-11-30");
    let (status, body) = call(&app, "POST", "/api/workers", Some(&other)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "conflict", "{body}");
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn the_batch_scan_finds_a_pair_and_merge_folds_one_into_the_other() {
    let app = common::create_test_router().await;
    let state = common::create_test_app_state().await;
    let family = format!("Batch{}", Uuid::new_v4().simple());
    // Seeded through the repository, past the create path's 409.
    let a: worker_service::models::Worker =
        serde_json::from_value(worker(&family, &format!("{family}-A"))).unwrap();
    let b: worker_service::models::Worker =
        serde_json::from_value(worker(&family, &format!("{family}-B"))).unwrap();
    let ctx = worker_service::db::AuditContext::default();
    let a = state.worker_repository.create(&a, &ctx).await.unwrap();
    let b = state.worker_repository.create(&b, &ctx).await.unwrap();

    // The request's defaults: threshold 0.7, auto-merge at 0.95.
    let (status, scan) = call(
        &app,
        "POST",
        "/api/workers/deduplicate",
        Some(&json!({ "max_candidates": 1000 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{scan}");
    let (ida, idb) = (a.id.to_string(), b.id.to_string());
    let is_pair = |i: &&Value| {
        let ids = [i["worker_id_a"].as_str(), i["worker_id_b"].as_str()];
        ids.contains(&Some(ida.as_str())) && ids.contains(&Some(idb.as_str()))
    };
    let item = scan["review_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(is_pair)
        .unwrap_or_else(|| panic!("the scan should pair the two seeded workers: {scan}"));
    // The pair agrees on everything it holds, so it scores 1.0, well over
    // the auto-merge threshold (it scored 0.65 before T-23 closed). But
    // name, birth date and gender are consistent with one worker, not
    // proof of one: twins hold them too. So it is queued, not auto-merged.
    assert!(item["match_score"].as_f64().unwrap() >= 0.95, "{item}");
    assert_eq!(item["status"], "pending", "{item}");
    assert_eq!(item["evidence"]["strength"], "consistent", "{item}");

    // The stored queue returns the same evidence.
    let (status, queue) = call(&app, "GET", "/api/workers/review-queue?limit=500", None).await;
    assert_eq!(status, StatusCode::OK, "{queue}");
    let stored = queue["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(is_pair)
        .unwrap_or_else(|| panic!("the pair is stored: {queue}"));
    assert_eq!(stored["evidence"]["strength"], "consistent", "{stored}");
    assert_eq!(stored["evidence"], item["evidence"], "{stored}");

    let (status, merged) = call(
        &app,
        "POST",
        "/api/workers/merge",
        Some(&json!({ "main_worker_id": a.id, "duplicate_worker_id": b.id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{merged}");
    let (status, _) = call(&app, "GET", &format!("/api/workers/{}", b.id), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, survivor) = call(&app, "GET", &format!("/api/workers/{}", a.id), None).await;
    assert_eq!(status, StatusCode::OK);
    let values: Vec<&str> = survivor["identifiers"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["value"].as_str())
        .collect();
    let (want_a, want_b) = (format!("{family}-A"), format!("{family}-B"));
    assert!(
        values.contains(&want_a.as_str()) && values.contains(&want_b.as_str()),
        "the survivor holds both identifiers: {survivor}"
    );
}

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn the_masked_view_hides_what_the_export_returns_whole() {
    let app = common::create_test_router().await;
    let state = common::create_test_app_state().await;
    let family = format!("Private{}", Uuid::new_v4().simple());
    let mut body = worker(&family, &format!("{family}-A"));
    body["tax_id"] = json!("123-45-6789");
    body["telecom"] = json!([{ "system": "phone", "value": "+44 7700 900456" }]);
    body["emergency_contacts"] = json!([{
        "name": "Sam Contact",
        "relationship": "sibling",
        "telecom": [{ "system": "phone", "value": "+44 7700 900789" }],
        "is_primary": true,
    }]);
    let w: worker_service::models::Worker = serde_json::from_value(body).unwrap();
    let stored = state
        .worker_repository
        .create(&w, &worker_service::db::AuditContext::default())
        .await
        .unwrap();

    let (status, masked) = call(
        &app,
        "GET",
        &format!("/api/workers/{}/masked", stored.id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{masked}");
    assert_eq!(masked["tax_id"], "***-**-6789");
    assert_eq!(masked["telecom"][0]["value"], "+** **** **0456");
    assert_eq!(
        masked["emergency_contacts"][0]["telecom"][0]["value"], "+** **** **0789",
        "{masked}"
    );

    let (status, export) = call(
        &app,
        "GET",
        &format!("/api/workers/{}/export", stored.id),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{export}");
    assert_eq!(export["tax_id"], "123-45-6789");
    assert_eq!(export["telecom"][0]["value"], "+44 7700 900456");
    assert_eq!(
        export["emergency_contacts"][0]["telecom"][0]["value"],
        "+44 7700 900789"
    );
}
