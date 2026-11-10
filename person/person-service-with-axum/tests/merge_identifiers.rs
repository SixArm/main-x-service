#![warn(clippy::pedantic)]

//! A merge of a duplicate holding an identifier succeeds, and the survivor
//! holds the identifier afterwards. `person_identifiers` is unique on
//! `(system, value)` across the table and the merge wrote the survivor's
//! copy while the duplicate's row was still there, so every such merge
//! violated the key and answered `500` (spec §13 T-39). A create carrying
//! an identifier another person holds answered `500` the same way.
//!
//! `#[ignore]`d — run with
//! `DB_URL=… cargo test --test merge_identifiers -- --ignored`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use person_service::db::AuditContext;
use person_service::models::Person;
use serde_json::{Value, json};
use tower::ServiceExt;

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

#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn merging_a_duplicate_with_an_identifier_moves_it_to_the_survivor() {
    let state = common::create_test_app_state().await;
    let app = common::create_test_router().await;
    let run = uuid::Uuid::new_v4().simple().to_string();
    let person = |mrn: &str| -> Person {
        serde_json::from_value(json!({
            "name": { "family": format!("Merge{run}"), "given": ["Ann"] },
            "gender": "female",
            "birth_date": "1980-01-01",
            "identifiers": [{ "identifier_type": "MRN", "system": "urn:example:mrn", "value": mrn }],
        }))
        .unwrap()
    };
    let (mrn_a, mrn_b) = (format!("{run}-A"), format!("{run}-B"));
    let ctx = AuditContext::default();
    let a = state
        .person_repository
        .create(&person(&mrn_a), &ctx)
        .await
        .unwrap();
    let b = state
        .person_repository
        .create(&person(&mrn_b), &ctx)
        .await
        .unwrap();

    let (status, merged) = call(
        &app,
        "POST",
        "/api/persons/merge",
        Some(&json!({ "main_person_id": a.id, "duplicate_person_id": b.id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{merged}");

    let (status, _) = call(&app, "GET", &format!("/api/persons/{}", b.id), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, survivor) = call(&app, "GET", &format!("/api/persons/{}", a.id), None).await;
    assert_eq!(status, StatusCode::OK);
    let values: Vec<&str> = survivor["identifiers"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["value"].as_str())
        .collect();
    assert!(
        values.contains(&mrn_a.as_str()) && values.contains(&mrn_b.as_str()),
        "the survivor holds both identifiers: {survivor}"
    );
}

/// A person carrying an identifier another person already holds is a
/// `409` naming the key, not a `500`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL via DB_URL"]
async fn an_identifier_another_person_holds_is_a_409_not_a_500() {
    let app = common::create_test_router().await;
    let mrn = format!("MRN-{}", uuid::Uuid::new_v4().simple());
    let body = |family: &str, given: &str, birth: &str| {
        json!({
            "name": { "family": family, "given": [given] },
            "gender": "female",
            "birth_date": birth,
            "identifiers": [{ "identifier_type": "MRN", "system": "urn:example:mrn", "value": mrn }],
        })
    };
    let run = uuid::Uuid::new_v4().simple().to_string();
    let (status, first) = call(
        &app,
        "POST",
        "/api/persons",
        Some(&body(&format!("Holder{run}"), "Ann", "1980-01-01")),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    let (status, second) = call(
        &app,
        "POST",
        "/api/persons",
        Some(&body(&format!("Other{run}"), "Theodore", "1951-11-30")),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{second}");
    assert_eq!(second["code"], "conflict", "{second}");
}
