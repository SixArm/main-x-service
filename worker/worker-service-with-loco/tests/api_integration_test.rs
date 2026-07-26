#![warn(clippy::pedantic)]

//! End-to-end tests for the REST API, driving the real Axum router via
//! `tower`'s `oneshot` against the JSON HTTP surface.
//!
//! Each test builds an app with [`common::create_test_router`], issues a
//! request, and asserts on the status code and response body. They exercise
//! the full CRUD lifecycle plus search and the not-found path. Test data is
//! namespaced with [`common::unique_worker_name`] so concurrent runs do not
//! collide. Requires the test database backing the router.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt; // for `oneshot` and `ready`

use worker_service::{api::ApiResponse, models::Worker};

/// `GET /api/health` returns 200 with the service name and "healthy".
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_health_check() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("healthy"));
    assert!(body_str.contains("worker-service"));
}

/// `POST /api/workers` creates a worker and returns it in the response.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_create_worker() {
    let app = common::create_test_router().await;

    let family_name = common::unique_worker_name("Create");

    let worker_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": family_name,
            "given": ["Integration", "Test"]
        },
        "birth_date": "1990-05-15",
        "gender": "female"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&worker_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        status,
        StatusCode::CREATED,
        "body: {}",
        String::from_utf8_lossy(&body)
    );

    let api_response: ApiResponse<Worker> = serde_json::from_slice(&body).unwrap();
    assert!(api_response.success);

    let worker = api_response.data.unwrap();
    assert_eq!(worker.name.family, family_name);
    assert_eq!(worker.name.given, vec!["Integration", "Test"]);
    assert!(worker.id.to_string() != "00000000-0000-0000-0000-000000000000");
}

/// A created worker can be fetched back by its id via `GET /workers/{id}`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_create_and_get_worker() {
    let app = common::create_test_router().await;

    let family_name = common::unique_worker_name("CreateGet");

    // Create worker
    let worker_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": family_name,
            "given": ["Get", "Test"]
        },
        "birth_date": "1985-03-20",
        "gender": "male"
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&worker_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Worker> = serde_json::from_slice(&create_body).unwrap();
    let created_worker = create_api_response.data.unwrap();
    let worker_id = created_worker.id;

    // Get worker by ID
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/workers/{worker_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let get_body = axum::body::to_bytes(get_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let get_api_response: ApiResponse<Worker> = serde_json::from_slice(&get_body).unwrap();
    assert!(get_api_response.success);

    let retrieved_worker = get_api_response.data.unwrap();
    assert_eq!(retrieved_worker.id, worker_id);
    assert_eq!(retrieved_worker.name.family, family_name);
}

/// `PUT /workers/{id}` updates an existing worker's fields.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_update_worker() {
    let app = common::create_test_router().await;

    let family_name = common::unique_worker_name("Update");

    // Create worker
    let worker_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": family_name,
            "given": ["Update"]
        },
        "birth_date": "1975-11-10",
        "gender": "other"
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&worker_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Worker> = serde_json::from_slice(&create_body).unwrap();
    let mut worker = create_api_response.data.unwrap();

    // Update worker
    worker.name.given = vec!["Update".to_string(), "Modified".to_string()];

    let update_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workers/{}", worker.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&worker).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    let update_body = axum::body::to_bytes(update_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let update_api_response: ApiResponse<Worker> = serde_json::from_slice(&update_body).unwrap();
    let updated_worker = update_api_response.data.unwrap();

    assert_eq!(updated_worker.name.given, vec!["Update", "Modified"]);
}

/// `DELETE /workers/{id}` soft-deletes a worker (subsequent reads 404).
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_delete_worker() {
    let app = common::create_test_router().await;

    let family_name = common::unique_worker_name("Delete");

    // Create worker
    let worker_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": family_name,
            "given": ["Delete"]
        },
        "birth_date": "1988-07-25",
        "gender": "unknown"
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&worker_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Worker> = serde_json::from_slice(&create_body).unwrap();
    let worker = create_api_response.data.unwrap();

    // Delete worker
    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workers/{}", worker.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // Try to get deleted worker - should return None (or 404 depending on implementation)
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/workers/{}", worker.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Soft delete means worker is not returned
    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

/// `GET /workers/search` returns previously created workers by name.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_search_workers() {
    let app = common::create_test_router().await;

    let family_name = common::unique_worker_name("Search");

    // Create a worker to search for
    let worker_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": {
            "use": "official",
            "family": family_name,
            "given": ["Searchable"]
        },
        "birth_date": "1992-04-18",
        "gender": "female"
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&worker_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // Give search engine time to index (in production this would be async)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Search for the worker
    let search_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/workers/search?q={family_name}&limit=10"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(search_response.status(), StatusCode::OK);

    let search_body = axum::body::to_bytes(search_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let body_str = String::from_utf8(search_body.to_vec()).unwrap();

    // Should contain the search term
    assert!(body_str.contains(&family_name));
}

/// `GET /workers/{id}` for an unknown id returns 404.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_get_worker_not_found() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/workers/00000000-0000-0000-0000-000000000001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// The FHIR route `GET /fhir/Practitioner/{id}` is mounted on the router.
///
/// Un-gated: drives a malformed UUID so the `Path<Uuid>` extractor rejects the
/// request with `400 Bad Request` *before* any database access. A `400` proves
/// the path matched a registered handler — a missing route would yield a
/// route-level `404` with an empty body. Pins spec §13 T-9 / T-12 (the
/// FHIR surface is reachable) without a live database.
#[tokio::test]
async fn test_fhir_worker_route_is_mounted() {
    let app = common::create_test_router_no_db();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/fhir/Practitioner/not-a-uuid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 400 from the Path<Uuid> extractor — the route exists and was reached.
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// `GET /fhir/Practitioner/{id}` for an unknown id returns a FHIR `OperationOutcome`.
///
/// DB-gated: a valid-but-absent UUID reaches the handler, which queries the
/// repository and returns a FHIR-conformant `404` (resourceType
/// `OperationOutcome`), confirming the mounted route serves FHIR responses.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_fhir_worker_not_found_returns_operation_outcome() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/fhir/Practitioner/00000000-0000-0000-0000-000000000001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("OperationOutcome"));
}

/// `GET /api/audit/verify` recomputes the audit hash chain and reports a
/// verified window (HIPAA §164.312(c)).
///
/// This drives the mounted route, so it pins the wiring — the handler
/// existing but never reachable was the previous state. The chain covers
/// whatever `audit_log` holds, so the assertion is that verification
/// *runs and reports*, and that a create performed in this test is inside
/// a window it reports as intact.
///
/// Note the limits of this test: it does not exercise **read**-auditing,
/// because that is gated behind `WORKER_AUDIT_READS`, which is cached in a
/// `OnceLock` and defaults off across the whole family. Turning it on from
/// a test would mean mutating the process environment, which is `unsafe`
/// in this edition and racy under a parallel harness. The read path is
/// covered instead at the repository level by
/// `db::audit::chain_tests::read_access_is_chained_and_flagged_as_a_disclosure`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_audit_verify_reports_an_intact_chain() {
    let app = common::create_test_router().await;

    // Write something auditable first, so the window is not vacuously empty.
    let worker_json = json!({
        "name": { "family": common::unique_worker_name("AuditVerify"), "given": ["Chain"] },
        "gender": "unknown"
    });
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&worker_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/audit/verify?limit=1000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: ApiResponse<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    assert!(parsed.success);
    let report = parsed.data.expect("a verification report");

    assert!(
        report["verified"].as_bool().unwrap(),
        "the chain must verify: {report}"
    );
    assert!(
        report["rows"].as_u64().unwrap() >= 1,
        "the create above must be inside the verified window: {report}"
    );
    // `unchained` is deliberately *not* asserted to be zero. The
    // `audit_workers_changes` / `audit_organizations_changes` database
    // triggers (migration 2024122800000005) INSERT into `audit_log`
    // themselves, and a trigger cannot compute the chain — it has no
    // access to the application's hashing or its advisory lock. Those
    // rows therefore land with a NULL `hash`, and verification skips
    // them. The report must keep *surfacing* the count so the gap stays
    // visible rather than being quietly rounded to "verified".
    assert!(
        report["unchained"].is_u64(),
        "the report must state how many rows are outside the chain: {report}"
    );
    // The response must say what it does and does not attest to — the
    // distinction is the whole point of publishing it.
    assert!(
        report["interpretation"]
            .as_str()
            .unwrap()
            .contains("not to the worker records")
    );
}

/// `GET /api/workers/{id}/audit/disclosures` answers the HIPAA
/// §164.528 accounting question, and — critically — says whether it can
/// answer it at all.
///
/// The caveat is the load-bearing assertion. `WORKER_AUDIT_READS`
/// defaults off across the family, so an empty accounting means "reads
/// are not being recorded", not "this record was never disclosed". An
/// endpoint that returned `[]` without saying so would give a false
/// answer to a question a data subject is legally entitled to have
/// answered truthfully.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_disclosure_accounting_states_whether_it_is_complete() {
    let app = common::create_test_router().await;

    let payload = json!({
        // A UUID rather than the shared timestamped helper: that helper
        // produces names sharing a long prefix across runs, which the
        // matcher scores as a duplicate and rejects with `409`.
        "name": {
            "family": format!("Acct{}", uuid::Uuid::new_v4().simple()),
            "given": ["Disclosure"]
        },
        "gender": "unknown"
    });
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(created.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: ApiResponse<Worker> = serde_json::from_slice(&body).unwrap();
    let id = created.data.expect("created record").id;

    // Perform an actual disclosing read: naming a recipient is what makes
    // an access a §164.528 *disclosure* rather than an internal access, so
    // without this the accounting would be trivially empty and the test
    // would pin nothing.
    let read = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/workers/{id}"))
                .header("x-purpose-of-use", "treatment")
                .header("x-disclosure-recipient", "referring-clinic")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(read.status(), StatusCode::OK);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/workers/{id}/audit/disclosures"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: ApiResponse<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    let report = parsed.data.expect("an accounting");

    assert_eq!(report["id"].as_str().unwrap(), id.to_string());
    assert!(report["disclosures"].is_array());
    assert_eq!(
        report["count"].as_u64().unwrap(),
        report["disclosures"].as_array().unwrap().len() as u64
    );

    let enabled = report["read_auditing_enabled"].as_bool().unwrap();
    let caveat = report["caveat"].as_str().unwrap();
    if enabled {
        assert!(caveat.contains("complete"), "caveat: {caveat}");
        // With auditing on, the disclosing read above must appear, and it
        // must carry what §164.528 asks for: to whom, and why.
        let first = &report["disclosures"][0];
        assert!(first["disclosure"].as_bool().unwrap());
        assert_eq!(
            first["context"]["recipient"].as_str().unwrap(),
            "referring-clinic"
        );
        assert_eq!(
            first["context"]["purpose_of_use"].as_str().unwrap(),
            "treatment"
        );
    } else {
        assert!(
            caveat.contains("INCOMPLETE") && caveat.contains("WORKER_AUDIT_READS"),
            "an accounting that cannot see reads must say so, and name the switch: {caveat}"
        );
    }
}

/// The accounting of an unknown record is `404`, not an empty accounting.
///
/// An empty list would tell an unauthenticated prober that the id is
/// valid but never disclosed; `404` says only that there is nothing to
/// answer about.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_disclosure_accounting_for_unknown_record_is_not_found() {
    let app = common::create_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/workers/{}/audit/disclosures",
                    uuid::Uuid::new_v4()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// GDPR Art. 17 erasure destroys the personal data across every table
/// that holds it, and the tamper-evident hash chain still verifies.
///
/// The chain assertion is the load-bearing one. Erasure is implemented as
/// redaction rather than deletion precisely so that Art. 17 and HIPAA
/// §164.312(c) can both be satisfied: deleting the audit rows would
/// honour one and destroy the other. If this ever fails, the two
/// obligations have stopped being simultaneously satisfiable and the
/// design is broken — not the test.
///
/// The child-table assertions matter because a worker is relational.
/// Unlike care-pathway and case, where the whole payload is one JSONB
/// column, a worker's identifiers, addresses, and contacts live in
/// separate tables — so an erasure that only touched the parent row would
/// look successful while leaving the actual personal data in place. The
/// test therefore checks the data is gone through the API rather than
/// trusting the reported counts.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_erasure_destroys_personal_data_and_the_chain_still_verifies() {
    let app = common::create_test_router().await;

    let payload = json!({
        "name": {
            "family": format!("Erase{}", uuid::Uuid::new_v4().simple()),
            "given": ["Right", "ToBeForgotten"]
        },
        "gender": "female",
        "birth_date": "1985-03-02",
        "tax_id": "AB123456C",
        "identifiers": [{
            "identifier_type": "SSN",
            "system": "http://hl7.org/fhir/sid/us-ssn",
            "value": format!("SSN-{}", uuid::Uuid::new_v4())
        }],
        "addresses": [{
            "line1": "12 Erasure Way",
            "city": "Leeds",
            "postal_code": "LS1 1AA",
            "country": "GB"
        }]
    });
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(created.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: ApiResponse<Worker> = serde_json::from_slice(&body).unwrap();
    let worker = created.data.expect("created worker");
    let id = worker.id;
    assert!(
        !worker.identifiers.is_empty(),
        "fixture must have data to erase"
    );
    assert!(!worker.addresses.is_empty());

    attach_and_verify_assessment(&app, id).await;

    let outcome = erase_and_read_outcome(&app, id).await;
    assert_erasure_outcome(&outcome);

    assert_worker_data_destroyed(id).await;

    assert_chain_verifies_with_redactions(app).await;
}

/// The chain still verifies across the redacted rows, and the redactions
/// are counted rather than hidden.
///
/// Split out of the test above so each stays under the line limit, and
/// because this is the single assertion the whole redaction design exists
/// to make true.
async fn assert_chain_verifies_with_redactions(app: axum::Router) {
    let verify = app
        .oneshot(
            Request::builder()
                .uri("/api/audit/verify?limit=1000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(verify.status(), StatusCode::OK);
    let body = axum::body::to_bytes(verify.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: ApiResponse<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    let report = parsed.data.expect("a verification report");
    assert!(
        report["verified"].as_bool().unwrap(),
        "redaction must preserve linkage: {report}"
    );
    assert!(
        report["redacted"].as_u64().unwrap() >= 1,
        "the redacted rows must be counted, not hidden: {report}"
    );
}

/// Erasing an unknown id is a valid request, not a `404`.
///
/// A `404` would confirm to a prober which ids are unknown, and a
/// subject's right to erasure does not lapse merely because no live
/// record remains — audit content held about the id is still personal
/// data.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_erasure_of_an_unknown_id_is_answered_not_refused() {
    let app = common::create_test_router().await;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/workers/{}/erase", uuid::Uuid::new_v4()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: ApiResponse<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    let outcome = parsed.data.expect("an erasure outcome");
    assert!(!outcome["payload_erased"].as_bool().unwrap());
    assert!(outcome["irreversible"].as_bool().unwrap());
}

/// Ground truth for a worker erasure, in SQL rather than through the API.
///
/// Erasure soft-deletes the worker, so every worker-scoped endpoint
/// afterwards returns `404` — including the assessments list. A `404`
/// proves only that the route is unreachable, not that the rows are gone,
/// and "unreachable" is exactly the weaker claim erasure is not allowed
/// to settle for. This assertion previously read the `data` key of a
/// response that is a bare array, so it counted `null` as zero and passed
/// over an intact psychometric profile.
async fn assert_worker_data_destroyed(id: uuid::Uuid) {
    let conn = common::db().await;

    for (table, label) in [
        ("worker_identifiers", "identifiers"),
        ("worker_addresses", "addresses"),
        ("worker_contacts", "contacts"),
        ("worker_documents", "documents"),
        ("worker_photos", "photos"),
        // The one this service has and person does not, and the most
        // sensitive: aptitude / personality / psychometric scores.
        ("worker_assessments", "psychometric assessments"),
    ] {
        let sql = format!("SELECT count(*) AS n FROM {table} WHERE worker_id = $1");
        assert_eq!(
            common::count_rows(&conn, &sql, id).await,
            0,
            "{label} survived erasure"
        );
    }

    assert_eq!(
        common::count_rows(
            &conn,
            "SELECT count(*) AS n FROM worker_names WHERE worker_id = $1 AND family = '(erased)'",
            id
        )
        .await,
        1,
        "the tombstone name must be present"
    );
    assert_eq!(
        common::count_rows(
            &conn,
            "SELECT count(*) AS n FROM worker_names WHERE worker_id = $1",
            id
        )
        .await,
        1,
        "a real name survived alongside the tombstone"
    );

    assert_eq!(
        common::count_rows(
            &conn,
            "SELECT count(*) AS n FROM workers WHERE id = $1 \
             AND tax_id IS NULL AND birth_date IS NULL AND worker_type IS NULL \
             AND gender = 'unknown' AND active = FALSE AND deleted_at IS NOT NULL",
            id
        )
        .await,
        1,
        "the workers row was not fully scrubbed and retired"
    );
}

/// Attach a completed psychometric assessment and pin that it is listable.
///
/// Split out to keep the erasure test under the line limit, and separated
/// from the erasure assertions on purpose: if the fixture never lands, the
/// "no assessments afterwards" assertion passes vacuously, so the setup
/// carries its own verification.
///
/// This is the table worker has and person does not, and the most
/// sensitive data this service holds — an erasure that swept names and
/// addresses but left a psychometric profile keyed to the worker id would
/// miss what a subject is most likely asking about.
async fn attach_and_verify_assessment(app: &axum::Router, id: uuid::Uuid) {
    let assessment = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/workers/{id}/assessments"))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "category": "psychometric",
                        "instrument": "Erasure Inventory",
                        "provider": "Test Publisher",
                        "status": "completed",
                        "administered_on": "2026-01-15",
                        "results": [{
                            "scale": "numerical_reasoning",
                            "raw_score": 41.0,
                            "max_score": 50.0,
                            "percentile": 82.0,
                            "narrative": "strong under time pressure"
                        }]
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = assessment.status();
    let body = axum::body::to_bytes(assessment.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(
        status.is_success(),
        "the assessment fixture must be created, or the erasure assertion is vacuous: {status:?} {}",
        String::from_utf8_lossy(&body)
    );

    // Pin that the assessment is really there before the erasure, so the
    // "zero assessments afterwards" assertion below cannot pass vacuously
    // because the fixture never landed or the list endpoint 404s.
    let before = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/workers/{id}/assessments"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(before.status(), StatusCode::OK);
    let body = axum::body::to_bytes(before.into_body(), usize::MAX)
        .await
        .unwrap();
    let listed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // The assessments endpoint returns a bare JSON array, not the
    // `{data: …}` envelope the other worker endpoints use. Reading
    // `["data"]` yields `null`, which counts as zero — which is exactly
    // how the post-erasure assertion below passed vacuously until this
    // pre-check was added.
    assert_eq!(
        listed
            .as_array()
            .expect("the assessments endpoint returns a bare array")
            .len(),
        1,
        "the assessment fixture must be listable before erasure: {listed}"
    );
}

/// Issue the erasure and return its reported outcome.
async fn erase_and_read_outcome(app: &axum::Router, id: uuid::Uuid) -> serde_json::Value {
    let erased = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/workers/{id}/erase"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(erased.status(), StatusCode::OK);
    let body = axum::body::to_bytes(erased.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: ApiResponse<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    parsed.data.expect("an erasure outcome")
}

/// What the service *claims* it did. Checked separately from
/// [`assert_worker_data_destroyed`], which checks what actually happened —
/// a self-reported count is not evidence.
fn assert_erasure_outcome(outcome: &serde_json::Value) {
    assert!(outcome["irreversible"].as_bool().unwrap());
    assert!(outcome["payload_erased"].as_bool().unwrap());
    assert!(
        outcome["child_rows_deleted"].as_u64().unwrap() >= 4,
        "at least the name, identifier, address, and assessment rows: {outcome}"
    );
    assert!(
        outcome["audit_rows_redacted"].as_u64().unwrap() >= 1,
        "the create above left audit content to redact: {outcome}"
    );
}
