#![warn(clippy::pedantic)]

//! Match evidence through the real router and a real database
//! (`agents/share/match-evidence.md`; spec §13 T-24).
//!
//! The batch scan marks a pair auto-merged only when it clears
//! `auto_merge_threshold` **and** its evidence is `proven`; the duplicate
//! check carries evidence beside every candidate, and that evidence holds
//! component names and bits, never a value. Each test names its workers
//! with a fresh UUID, so a run finds its own records among whatever
//! earlier runs left in a shared database.
//!
//! `#[ignore]`d — run with
//! `DATABASE_URL=… cargo test --test match_evidence_db -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;
use worker_service::db::AuditContext;
use worker_service::models::Worker;

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

/// A worker body with a family name unique to the run and, optionally, a
/// tax id (which the adapter routes to the matcher's unique `us_ssn`
/// scheme).
fn worker(family: &str, tax_id: Option<&str>) -> Value {
    let mut body = json!({
        "name": { "family": family, "given": ["Margaret"] },
        "gender": "female",
        "birth_date": "1979-04-12",
    });
    if let Some(t) = tax_id {
        body["tax_id"] = json!(t);
    }
    body
}

/// A valid-shaped US SSN unique enough to the run.
fn ssn() -> String {
    let n = Uuid::new_v4().as_u128();
    format!("536-{:02}-{:04}", 10 + n % 89, 1000 + (n >> 8) % 9000)
}

/// Seed two workers through the repository (past the create path's 409).
async fn seed_pair(a: &Value, b: &Value) -> (Worker, Worker) {
    let state = common::create_test_app_state().await;
    let ctx = AuditContext::default();
    let a: Worker = serde_json::from_value(a.clone()).unwrap();
    let b: Worker = serde_json::from_value(b.clone()).unwrap();
    let a = state.worker_repository.create(&a, &ctx).await.unwrap();
    let b = state.worker_repository.create(&b, &ctx).await.unwrap();
    (a, b)
}

/// Run a scan and return the pair's item.
async fn scan_for(app: &axum::Router, body: &Value, a: &Worker, b: &Worker) -> Value {
    let (status, scan) = call(app, "POST", "/api/workers/deduplicate", Some(body)).await;
    assert_eq!(status, StatusCode::OK, "{scan}");
    let (ida, idb) = (a.id.to_string(), b.id.to_string());
    scan["review_items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| {
            let ids = [i["worker_id_a"].as_str(), i["worker_id_b"].as_str()];
            ids.contains(&Some(ida.as_str())) && ids.contains(&Some(idb.as_str()))
        })
        .cloned()
        .unwrap_or_else(|| panic!("the scan should pair the two seeded workers: {scan}"))
}

/// A pair sharing a unique identifier is proven, and over the threshold
/// it is marked auto-merged.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_pair_sharing_a_unique_identifier_is_automerged_as_proven() {
    let app = common::create_test_router().await;
    let family = format!("Proven{}", Uuid::new_v4().simple());
    let shared = ssn();
    let (a, b) = seed_pair(
        &worker(&family, Some(&shared)),
        &worker(&family, Some(&shared)),
    )
    .await;

    let item = scan_for(&app, &json!({ "max_candidates": 1000 }), &a, &b).await;
    assert!(item["match_score"].as_f64().unwrap() >= 0.95, "{item}");
    assert_eq!(item["evidence"]["strength"], "proven", "{item}");
    assert_eq!(item["status"], "automerged", "{item}");
    assert!(
        item["evidence"]["weight_bits"].as_f64().unwrap() >= 30.0,
        "{item}"
    );
}

/// A pair holding two different values under one unique scheme is
/// contradicted, and stays pending even over the auto-merge threshold.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn a_pair_with_conflicting_identifiers_stays_pending_as_contradicted() {
    let app = common::create_test_router().await;
    let family = format!("Contra{}", Uuid::new_v4().simple());
    let (a, b) = seed_pair(
        &worker(&family, Some(&ssn())),
        &worker(&family, Some(&ssn())),
    )
    .await;

    // The conflicting SSN drags the score to about 0.68; ask for pairs
    // over 0.5 and set the auto-merge threshold under the pair's score,
    // so only the evidence keeps it out of an automatic merge.
    let item = scan_for(
        &app,
        &json!({ "threshold": 0.5, "max_candidates": 1000, "auto_merge_threshold": 0.6 }),
        &a,
        &b,
    )
    .await;
    assert!(item["match_score"].as_f64().unwrap() >= 0.6, "{item}");
    assert_eq!(item["evidence"]["strength"], "contradicted", "{item}");
    assert_eq!(item["status"], "pending", "{item}");
    assert_eq!(
        item["evidence"]["contradictions"][0]["field"], "identifier:us_ssn",
        "{item}"
    );
}

/// `check-duplicates` carries evidence beside each candidate, and that
/// evidence holds no value either worker holds.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DATABASE_URL"]
async fn check_duplicates_carries_evidence_without_values() {
    let app = common::create_test_router().await;
    // Under 40 characters: the search index drops a longer token, and the
    // duplicate check finds its candidates through the index.
    let family = format!("Zbigniewska{}", &Uuid::new_v4().simple().to_string()[..16]);
    let tax_id = ssn();
    let body = worker(&family, Some(&tax_id));
    let (status, created) = call(&app, "POST", "/api/workers", Some(&body)).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");

    let (status, check) = call(&app, "POST", "/api/workers/check-duplicates", Some(&body)).await;
    assert_eq!(status, StatusCode::OK, "{check}");
    assert_eq!(check["has_duplicates"], true, "{check}");
    let candidate = check["potential_matches"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["worker"]["id"] == created["id"])
        .unwrap_or_else(|| panic!("the created worker is a candidate: {check}"));
    let evidence = &candidate["evidence"];
    assert_eq!(evidence["strength"], "proven", "{candidate}");
    assert!(
        evidence["fields_compared"].as_u64().unwrap() >= 4,
        "{candidate}"
    );

    // Every string in the evidence and the breakdown, keys included: the
    // numbers are bits and scores, so the strings are where a value would
    // show.
    let mut strings = Vec::new();
    collect_strings(evidence, &mut strings);
    collect_strings(&candidate["score_breakdown"], &mut strings);
    let serial = &tax_id[tax_id.len() - 4..];
    for value in [family.to_lowercase().as_str(), "margaret", "1979", serial] {
        assert!(
            strings.iter().all(|s| !s.to_lowercase().contains(value)),
            "{value} leaked into {strings:?}"
        );
    }
}

/// Every object key and string value in `value`.
fn collect_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|v| collect_strings(v, out)),
        Value::Object(map) => {
            for (k, v) in map {
                out.push(k.clone());
                collect_strings(v, out);
            }
        }
        _ => {}
    }
}
