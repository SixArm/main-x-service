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

use worker_service::{api::rest::links::LinkView, models::Worker};

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

    let api_response: Worker = serde_json::from_slice(&body).unwrap();

    let worker = api_response;
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

    let create_api_response: Worker = serde_json::from_slice(&create_body).unwrap();
    let created_worker = create_api_response;
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

    let get_api_response: Worker = serde_json::from_slice(&get_body).unwrap();

    let retrieved_worker = get_api_response;
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

    let create_api_response: Worker = serde_json::from_slice(&create_body).unwrap();
    let mut worker = create_api_response;

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

    let update_api_response: Worker = serde_json::from_slice(&update_body).unwrap();
    let updated_worker = update_api_response;

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

    let create_api_response: Worker = serde_json::from_slice(&create_body).unwrap();
    let worker = create_api_response;

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

/// `GET /api/workers` is a genuine database-backed "list everything"
/// endpoint — distinct from `/workers/search`, and added specifically
/// because `/workers/search?q=*` is **not** a reliable way to enumerate a
/// collection (a live investigation on the sibling person-service found
/// the Tantivy index can carry stale entries the database no longer has;
/// see `person-service`'s `CHANGELOG.md`). This test creates several
/// workers with genuinely distinct family names (not just distinct
/// suffixes of a shared prefix, which real-time duplicate detection would
/// legitimately 409 on) and pages through the **entire** collection with a
/// `limit` smaller than the created count, proving every created worker is
/// found exactly once across the pages.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_list_workers_paginates_every_created_record_exactly_once() {
    let app = common::create_test_router().await;

    let run_suffix = chrono::Utc::now().timestamp_micros();
    let surnames = [
        "Abernathy",
        "Blackwood",
        "Cavendish",
        "Dunmore",
        "Ellsworth",
        "Fitzgerald",
        "Greenhalgh",
    ];
    let created_count = surnames.len();
    let mut created_ids = std::collections::HashSet::new();

    for (i, surname) in surnames.iter().enumerate() {
        let family_name = format!("{surname}{run_suffix}");
        let worker_json = json!({
            "id": "00000000-0000-0000-0000-000000000000",
            "name": { "family": family_name, "given": [format!("Page{i}")] },
            "gender": "unknown",
        });
        let response = app
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
        assert_eq!(response.status(), StatusCode::CREATED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created: Worker = serde_json::from_slice(&body).unwrap();
        created_ids.insert(created.id);
    }
    assert_eq!(created_ids.len(), created_count, "each POST made a new id");

    // Page through the WHOLE collection (other tests' workers may
    // coexist) with a limit smaller than what we created, exactly
    // mirroring how link-graph's suggestion job (T-31) pages
    // `GET /api/workers?limit=&offset=`.
    let page_limit = 3usize;
    let mut offset = 0usize;
    let mut seen_ids: Vec<uuid::Uuid> = Vec::new();
    loop {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/workers?limit={page_limit}&offset={offset}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let page: worker_service::api::rest::handlers::ListResponse =
            serde_json::from_slice(&body).unwrap();
        let data = page;
        let page_len = data.workers.len();
        for w in data.workers {
            seen_ids.push(w.id);
        }
        if page_len < page_limit {
            break;
        }
        offset += page_limit;
        assert!(offset < 100_000, "paginated far past any plausible total");
    }

    let seen_unique: std::collections::HashSet<_> = seen_ids.iter().copied().collect();
    assert_eq!(
        seen_ids.len(),
        seen_unique.len(),
        "a record was returned on more than one page — pagination is not stable"
    );
    for id in &created_ids {
        assert!(
            seen_unique.contains(id),
            "created worker {id} was never seen while paginating /api/workers"
        );
    }
}

/// SEC-G7-style bound: `GET /api/workers` with an out-of-bound pagination
/// `offset` is rejected with `400`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_list_workers_rejects_out_of_bound_offset() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/workers?offset=1000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "an offset past the bound must be rejected"
    );
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
async fn test_fhir_practitioner_route_is_mounted() {
    let app = common::create_test_router_no_db().await;

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
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let report = parsed;

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
    let created: Worker = serde_json::from_slice(&body).unwrap();
    let id = created.id;

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
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let report = parsed;

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
    let created: Worker = serde_json::from_slice(&body).unwrap();
    let worker = created;
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
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let report = parsed;
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
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let outcome = parsed;
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

    for (sql, label) in [
        (
            "SELECT count(*) AS n FROM worker_identifiers WHERE worker_id = $1",
            "identifiers",
        ),
        (
            "SELECT count(*) AS n FROM worker_addresses WHERE worker_id = $1",
            "addresses",
        ),
        (
            "SELECT count(*) AS n FROM worker_contacts WHERE worker_id = $1",
            "contacts",
        ),
        (
            "SELECT count(*) AS n FROM worker_documents WHERE worker_id = $1",
            "documents",
        ),
        (
            "SELECT count(*) AS n FROM worker_photos WHERE worker_id = $1",
            "photos",
        ),
        // The one this service has and person does not, and the most
        // sensitive: aptitude / personality / psychometric scores.
        (
            "SELECT count(*) AS n FROM worker_assessments WHERE worker_id = $1",
            "psychometric assessments",
        ),
    ] {
        assert_eq!(
            common::count_rows(&conn, sql, id).await,
            0,
            "{label} survived erasure"
        );
    }

    assert_eq!(
        common::count_rows(
            &conn,
            "SELECT count(*) AS n FROM worker_names \
             WHERE worker_id = $1 AND family = '(erased)' AND is_primary",
            id
        )
        .await,
        1,
        "the tombstone name must be present, and primary (WK-E3)"
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
    // The assessments endpoint returns a bare JSON array. When the other
    // worker endpoints still wrapped theirs in a `{data: …}` envelope,
    // reading `["data"]` here yielded `null`, which counts as zero — which
    // is exactly how the post-erasure assertion below passed vacuously
    // until this pre-check was added.
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
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    parsed
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

/// After `m20260726_000003_drop_audit_triggers`, **no writer produces an
/// unchained audit row**: a full create/update/delete cycle leaves every
/// row carrying a chain digest.
///
/// This is the pin for the trigger removal. The
/// `audit_workers_changes` / `audit_organizations_changes` triggers
/// wrote into `audit_log` from the database, where the application's
/// hashing and advisory lock are unreachable, so their rows landed with a
/// NULL `hash`. Verification skipped them, which meant an inserted row
/// with a NULL `hash` did not register as a break and a trigger row could
/// be deleted without breaking linkage either — roughly half the trail
/// was a log rather than evidence.
///
/// Asserted on a **fresh** database, which is what CI provides: rows
/// written before the migration keep their NULL `hash` deliberately
/// (deleting them would destroy history, and they carry no digest to
/// invalidate), so this assertion is about writers, not about history.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_no_writer_produces_unchained_audit_rows() {
    let app = common::create_test_router().await;

    let payload = json!({
        "name": {
            "family": format!("Chained{}", uuid::Uuid::new_v4().simple()),
            "given": ["No", "Triggers"]
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
    let created: Worker = serde_json::from_slice(&body).unwrap();
    let id = created.id;

    // A delete exercises the third trigger branch (`TG_OP = 'DELETE'`,
    // which a soft delete reached as an UPDATE).
    let deleted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workers/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(deleted.status().is_success() || deleted.status() == StatusCode::NO_CONTENT);

    // Ground truth: no row anywhere in the trail lacks a digest.
    let conn = common::db().await;
    let unchained = common::count_rows(
        &conn,
        "SELECT count(*) AS n FROM audit_log WHERE hash IS NULL AND $1 IS NOT NULL",
        id,
    )
    .await;
    assert_eq!(
        unchained, 0,
        "a writer is still appending unchained audit rows; \
     the database triggers should have been dropped"
    );

    // And the trigger's own entity_type must be absent: nothing writes it.
    let legacy = common::count_rows(
        &conn,
        "SELECT count(*) AS n FROM audit_log WHERE entity_type = 'worker' \
     AND $1 IS NOT NULL",
        id,
    )
    .await;
    assert_eq!(legacy, 0, "the dropped trigger is still writing rows");
}

/// The per-entity audit endpoint returns the record's rows, including the
/// **read** rows the disclosure path writes.
///
/// Before 2026-07-26 the two writers disagreed on `entity_type` — mutations
/// wrote `"Worker"`, read-auditing wrote `"worker"` — and this endpoint
/// filtered on one spelling, so it silently omitted every read. A short
/// audit answer is worse than an error: nothing in the response says it is
/// incomplete.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_entity_audit_covers_every_entity_type_spelling() {
    let app = common::create_test_router().await;

    let payload = json!({
        "name": {
            "family": format!("Spelling{}", uuid::Uuid::new_v4().simple()),
            "given": ["Audit"]
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
    let created: Worker = serde_json::from_slice(&body).unwrap();
    let id = created.id;

    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/workers/{id}/audit"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
    let rows = parsed;
    assert!(
        !rows.is_empty(),
        "the create must appear in this record's audit history"
    );

    // Whatever spelling a row carries, the endpoint must have returned it:
    // compare against the database rather than trusting the filter.
    let conn = common::db().await;
    let total = common::count_rows(
        &conn,
        "SELECT count(*) AS n FROM audit_log WHERE entity_id = $1",
        id,
    )
    .await;
    assert_eq!(
        i64::try_from(rows.len()).expect("row count fits in i64"),
        total,
        "the endpoint returned {} of {total} audit rows for this record — \
     a spelling is being filtered out",
        rows.len()
    );
}

/// Row-level record integrity: **every** write path leaves the record's
/// content hash matching its content.
///
/// This is the test the feature lives or dies by. The hash is only useful
/// if a mismatch means tampering — so a write path that forgets to rehash
/// produces a false accusation, which is worse than having no control at
/// all. The compiler catches only one of the four paths (`create`, whose
/// initializer names every column); `update`, `merge`, and `delete` build
/// their `ActiveModel` with `..Default::default()` and compile happily
/// while silently leaving a stale digest. So each is exercised here.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_every_write_path_rehashes_the_record() {
    let app = common::create_test_router().await;

    let create = |suffix: &str| {
        json!({
            "name": {
                "family": format!("Integ{}{}", suffix, uuid::Uuid::new_v4().simple()),
                "given": ["Hash"]
            },
            "gender": "female",
            "tax_id": "AB123456C"
        })
    };

    // 1. create
    let id = create_worker(&app, &create("Create")).await;
    assert_records_verify(&app, "after create").await;

    // 2. update
    let mut updated = create("Update");
    updated["id"] = json!(id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workers/{id}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&updated).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "update failed: {:?}",
        response.status()
    );
    assert_records_verify(&app, "after update").await;

    // 3. delete (soft) — changes the lifecycle state the digest binds
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workers/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(response.status().is_success() || response.status() == StatusCode::NO_CONTENT);
    assert_records_verify(&app, "after soft delete").await;

    // 4. erase — clears the hash rather than recomputing one over a
    //    half-destroyed record, so it must report as a gap, never a
    //    mismatch.
    let erased = create_worker(&app, &create("Erase")).await;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/workers/{erased}/erase"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_records_verify(&app, "after erasure").await;
}

/// An out-of-band SQL edit to a **child table** is detected.
///
/// This is the gap the dropped database triggers never closed. The edit
/// writes no audit row, so the audit chain still verifies — and it touches
/// `worker_identifiers`, not `workers`, so a digest over the parent row
/// alone would not have noticed either.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_out_of_band_sql_edit_to_a_child_table_is_detected() {
    let app = common::create_test_router().await;
    let payload = json!({
        "name": {
            "family": format!("Tamper{}", uuid::Uuid::new_v4().simple()),
            "given": ["Detect"]
        },
        "gender": "male",
        "identifiers": [{
            "identifier_type": "SSN",
            "system": "http://hl7.org/fhir/sid/us-ssn",
            "value": format!("SSN-{}", uuid::Uuid::new_v4())
        }]
    });
    let id = create_worker(&app, &payload).await;
    assert_records_verify(&app, "before tampering").await;

    // Edit the stored identifier directly, writing no audit row.
    let conn = common::db().await;
    sqlx::query("UPDATE worker_identifiers SET value = 'TAMPERED' WHERE worker_id = $1")
        .bind(id)
        .execute(&conn)
        .await
        .expect("tamper");

    // The audit chain is untouched — that is the point.
    let verify = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/audit/verify?limit=1000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(verify.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        parsed["verified"].as_bool().unwrap(),
        "the audit chain cannot see a record edit — that is why this control exists"
    );

    // Record integrity does see it, and names the record.
    let report = record_integrity_report(&app).await;
    assert!(
        !report["verified"].as_bool().unwrap(),
        "an out-of-band child-table edit must be detected: {report}"
    );
    let flagged: Vec<&str> = report["mismatched"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["id"].as_str())
        .collect();
    assert!(
        flagged.contains(&id.to_string().as_str()),
        "the tampered record must be named: {report}"
    );

    // Leave no deliberately-corrupted record behind: the database is
    // shared with every other DB-gated target in this crate.
    common::purge_record(&conn, id).await;
}

/// Create a worker through the API and return its id.
async fn create_worker(app: &axum::Router, payload: &serde_json::Value) -> uuid::Uuid {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: Worker = serde_json::from_slice(&body).unwrap();
    parsed.id
}

/// Fetch the record-integrity report.
async fn record_integrity_report(app: &axum::Router) -> serde_json::Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/records/verify?limit=500")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    parsed
}

/// Assert no record reports as tampered.
async fn assert_records_verify(app: &axum::Router, stage: &str) {
    let report = record_integrity_report(app).await;
    assert!(
        report["verified"].as_bool().unwrap(),
        "{stage}: a write path did not rehash its record — \
         this is a false tamper report, not a real one: {report}"
    );
    // Guard against passing vacuously: if nothing were ever hashed, every
    // record would count as `unhashed` and `verified` would still be true,
    // so the assertion above would hold while the feature did nothing.
    assert!(
        report["intact"].as_u64().unwrap() >= 1,
        "{stage}: no record carries a verified hash, so this proves nothing: {report}"
    );
}

/// An out-of-band SQL edit to a **psychometric score** is detected.
///
/// `worker_assessments` is the most sensitive table this service holds and
/// was, until now, the one place record integrity did not reach: it is not
/// part of the assembled `Worker`, so `workers.content_hash` never covered
/// it. A changed score band is exactly the edit someone would want to make
/// and exactly the one nothing would have caught.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_out_of_band_edit_to_an_assessment_score_is_detected() {
    let app = common::create_test_router().await;
    let id = create_worker(
        &app,
        &json!({
            "name": {
                "family": format!("Score{}", uuid::Uuid::new_v4().simple()),
                "given": ["Tamper"]
            },
            "gender": "unknown"
        }),
    )
    .await;
    attach_and_verify_assessment(&app, id).await;
    assert_records_verify(&app, "before tampering with a score").await;

    // Raise the percentile directly in SQL, writing no audit row.
    let conn = common::db().await;
    let affected = sqlx::query(
        "UPDATE worker_assessments \
         SET results = jsonb_set(results, '{0,percentile}', '99.0') \
         WHERE worker_id = $1",
    )
    .bind(id)
    .execute(&conn)
    .await
    .expect("tamper")
    .rows_affected();
    assert_eq!(affected, 1, "the fixture assessment must have been edited");

    // The audit chain cannot see it — that is why this control exists.
    let verify = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/audit/verify?limit=1000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(verify.into_body(), usize::MAX)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(
        parsed["verified"].as_bool().unwrap(),
        "an assessment edit writes no audit row, so the chain still verifies"
    );

    // Record integrity does see it.
    let report = record_integrity_report(&app).await;
    assert!(
        !report["verified"].as_bool().unwrap(),
        "an edited psychometric score must be detected: {report}"
    );

    // Leave no deliberately-corrupted record behind.
    common::purge_record(&conn, id).await;
}

/// `GET /api/compliance` is mounted and reports build provenance.
///
/// Mounted, not merely compiled — and worth pinning here in particular:
/// this crate is mid-conversion and registers its routes twice, once as
/// an axum `Router` and once as an axa `Routes`, so a handler added to
/// only one of them compiles and serves 404 from the other.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn test_compliance_identification_is_mounted() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/compliance")
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

    assert!(body_str.contains("worker-service"));
    // The absent medical-device classification is stated, not implied.
    assert!(body_str.contains("IEC 62304"));
}

/// `GET /api/compliance/sbom` names every dependency's exact version, so
/// it needs a verified caller even with `WORKER_REQUIRE_AUTH` off. The
/// document itself is checked in `tests/enforcement.rs`, which has a token.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn test_compliance_sbom_needs_a_caller_with_enforcement_off() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/compliance/sbom")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

/// The cross-service link endpoints (`api/rest/links.rs`) answer bare JSON,
/// like every other worker REST endpoint (`spec/restful` §1.3): a created
/// link is the `LinkView`, a list is the array, and a withdrawal is `{}`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn test_links_endpoints_answer_bare_json() {
    let app = common::create_test_router().await;

    let family_name = common::unique_worker_name("Links");
    let worker_json = json!({
        "id": "00000000-0000-0000-0000-000000000000",
        "name": { "family": family_name, "given": ["Envelope", "Test"] },
        "gender": "female"
    });
    let response = app
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
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: Worker = serde_json::from_slice(&body).unwrap();
    let worker_id = created.id;

    // Create a same_identity link — the response is the link itself.
    let to_ref = format!("person:{}", uuid::Uuid::new_v4());
    let link_json = json!({ "kind": "same_identity", "to_ref": to_ref });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/workers/{worker_id}/links"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&link_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let link: LinkView = serde_json::from_slice(&body).unwrap();
    assert_eq!(link.to_ref, to_ref);

    // List — the bare array.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/workers/{worker_id}/links"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let listed: Vec<LinkView> = serde_json::from_slice(&body).unwrap();
    assert_eq!(listed.len(), 1);

    // Withdraw — an empty object.
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/workers/{worker_id}/links/{}", link.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let deleted: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(deleted, json!({}));
}

/// Whether `/api/records/verify` names `id` as mismatched.
async fn is_mismatched(app: &axum::Router, id: uuid::Uuid) -> bool {
    let report = record_integrity_report(app).await;
    report["mismatched"]
        .as_array()
        .expect("a mismatched list")
        .iter()
        .any(|m| m["id"].as_str() == Some(id.to_string().as_str()))
}

/// A worker payload carrying every child collection that can be written,
/// for [`a_written_record_verifies`].
fn rich_worker_payload() -> serde_json::Value {
    json!({
        "name": {
            "family": format!("Verifies{}", uuid::Uuid::new_v4().simple()),
            "given": ["Written", "Stored"],
            "prefix": ["Dr"],
            "suffix": ["Jr"]
        },
        "additional_names": [
            { "family": "Before", "given": ["Ann"] },
            { "family": "Short", "given": ["Annie"] }
        ],
        "gender": "female",
        "worker_type": "nurse",
        "birth_date": "1950-02-03",
        "tax_id": "AB123456C",
        "deceased": true,
        "deceased_datetime": "2020-01-02T03:04:05.123456789Z",
        "marital_status": "M",
        "multiple_birth": false,
        "identifiers": [{
            "identifier_type": "NPI",
            "system": "http://hl7.org/fhir/sid/us-npi",
            "value": format!("NPI-{}", uuid::Uuid::new_v4()),
            "assigner": "CMS"
        }],
        "addresses": [
            { "use_type": "home", "line1": "1 Main St", "line2": "Flat 2", "city": "Springfield", "state": "IL", "postal_code": "12345", "country": "US" },
            { "use_type": "work", "line1": "2 Side St", "city": "Shelbyville", "country": "US" }
        ],
        "documents": [{
            "document_type": "PASSPORT",
            "number": "P123",
            "issuing_country": "US",
            "issuing_authority": "State Dept",
            "issue_date": "2010-01-01",
            "expiry_date": "2030-01-01",
            "verified": true
        }],
        "emergency_contacts": [{
            "name": "Bob",
            "relationship": "spouse",
            "telecom": [{ "system": "phone", "value": "+15550101", "use_type": "mobile" }],
            "address": { "use_type": "home", "city": "Springfield" },
            "is_primary": true
        }],
        "photo": ["https://example.org/a.png"],
        "created_at": "2001-01-01T00:00:00.987654321Z",
        "updated_at": "2001-01-01T00:00:00.987654321Z"
    })
}

/// INT-V1: a record verifies as it is **stored**, not as the caller sent
/// it. A fresh record and an updated one both verify, and a row edited in
/// SQL is still flagged, so the first two checks cannot pass by verifying
/// nothing.
///
/// The payload carries what the stored record may not keep exactly as
/// sent: address `use_type`s, sub-microsecond timestamps, and every child
/// collection that can be written, plus an assessment. That includes
/// `telecom`, `links`, and every name and identifier use, which until
/// WK-D1 was fixed could not be stored at all ([`enum_columns`]).
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn a_written_record_verifies() {
    let app = common::create_test_router().await;
    let other = create_worker(
        &app,
        &json!({
            "name": { "family": format!("VerifiesTarget{}", uuid::Uuid::new_v4().simple()), "given": ["T"] },
            "gender": "unknown"
        }),
    )
    .await;
    let mut payload = rich_worker_payload();
    with_enum_columns(&mut payload, other);
    // Keep the NPI as well, so the identifiers mix a known type with the
    // `OTHER` and `ODS` ones.
    let npi = rich_worker_payload()["identifiers"][0].clone();
    payload["identifiers"].as_array_mut().unwrap().push(npi);
    let id = create_worker(&app, &payload).await;
    assert!(
        !is_mismatched(&app, id).await,
        "a fresh record must verify: {}",
        record_integrity_report(&app).await
    );
    attach_and_verify_assessment(&app, id).await;
    assert!(
        !is_mismatched(&app, id).await,
        "a record with an assessment must verify: {}",
        record_integrity_report(&app).await
    );

    let mut changed = payload.clone();
    changed["id"] = json!(id);
    changed["marital_status"] = json!("W");
    changed["deceased_datetime"] = json!("2021-05-06T07:08:09.987654321Z");
    changed["addresses"][0]["use_type"] = json!("temp");
    changed["telecom"][0]["use_type"] = json!("work");
    changed["links"].as_array_mut().unwrap().truncate(2);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/workers/{id}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&changed).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(
        response.status().is_success(),
        "update failed: {:?}",
        response.status()
    );
    assert!(
        !is_mismatched(&app, id).await,
        "an updated record must verify: {}",
        record_integrity_report(&app).await
    );

    // And verification still catches a change made behind the service's
    // back, so the checks above cannot pass by verifying nothing.
    let conn = common::db().await;
    sqlx::query(
        "UPDATE worker_names SET family = family || ' (edited in SQL)' \
         WHERE worker_id = $1 AND is_primary",
    )
    .bind(id)
    .execute(&conn)
    .await
    .expect("tamper with the row");
    assert!(
        is_mismatched(&app, id).await,
        "a row edited in SQL must be flagged"
    );
    // Leave no deliberately-corrupted record behind: the database is
    // shared with every other DB-gated target in this crate.
    common::purge_record(&conn, id).await;
    common::purge_record(&conn, other).await;
}

/// `GET /api/workers/{id}` as JSON.
async fn get_worker_json(app: &axum::Router, id: uuid::Uuid) -> serde_json::Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/workers/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

/// `POST /api/workers`, returning the status and the body as JSON.
async fn post_worker(app: &axum::Router, payload: &serde_json::Value) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8_lossy(&body).into_owned())
}

/// The name, identifier, contact and link uses and types that the
/// tables' CHECK constraints spell differently from Rust's `Debug`, for
/// [`enum_columns_round_trip`] and [`a_written_record_verifies`].
///
/// Every name use, every identifier use, the `OTHER` and `ODS`
/// identifier types, every contact system and use, and every link type
/// to `other`.
fn enum_columns(other: uuid::Uuid) -> serde_json::Value {
    let ident = |identifier_type: &str, use_type: &str| {
        json!({
            "identifier_type": identifier_type,
            "use_type": use_type,
            "system": "urn:test",
            "value": format!("{identifier_type}-{use_type}-{}", uuid::Uuid::new_v4().simple()),
        })
    };
    let additional_names: Vec<serde_json::Value> =
        ["usual", "temp", "nickname", "anonymous", "old", "maiden"]
            .iter()
            .map(|u| json!({ "use_type": u, "family": format!("Alias{u}"), "given": ["A"] }))
            .collect();
    let links: Vec<serde_json::Value> = ["replacedby", "replaces", "refer", "seealso"]
        .iter()
        .map(|t| json!({ "other_worker_id": other, "link_type": t }))
        .collect();
    json!({
        "name_use": "official",
        "additional_names": additional_names,
        "identifiers": [
            ident("OTHER", "usual"),
            ident("ODS", "official"),
            ident("OTHER", "temp"),
            ident("ODS", "secondary"),
            ident("OTHER", "old"),
        ],
        "telecom": [
            { "system": "phone", "value": "+15550100", "use_type": "home" },
            { "system": "fax", "value": "+15550101", "use_type": "work" },
            { "system": "email", "value": "a@example.org", "use_type": "temp" },
            { "system": "pager", "value": "+15550102", "use_type": "old" },
            { "system": "url", "value": "https://example.org", "use_type": "work" },
            { "system": "sms", "value": "+15550103", "use_type": "mobile" },
            { "system": "other", "value": "handle" }
        ],
        "links": links,
    })
}

/// Lay [`enum_columns`] onto a worker payload.
fn with_enum_columns(payload: &mut serde_json::Value, other: uuid::Uuid) {
    let columns = enum_columns(other);
    payload["name"]["use_type"] = columns["name_use"].clone();
    for key in ["additional_names", "identifiers", "telecom", "links"] {
        payload[key] = columns[key].clone();
    }
}

/// `values` sorted by their JSON text, so two lists compare regardless of
/// the order the database returned their rows in.
fn sorted(values: &serde_json::Value) -> Vec<String> {
    let mut texts: Vec<String> = values
        .as_array()
        .expect("an array")
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    texts.sort();
    texts
}

/// WK-D1: the name and identifier uses, the identifier type, a contact's
/// system and use, and a link's type are stored in the lowercase tokens
/// the tables' CHECK constraints take, and read back as they were sent.
///
/// They used to be written in their Rust `Debug` spelling (`Official`,
/// `Phone`, `Seealso`, `Other`), which the constraints refuse, so this
/// create answered `500`, and no worker could carry telecom or links.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn enum_columns_round_trip() {
    let app = common::create_test_router().await;
    let other = create_worker(
        &app,
        &json!({
            "name": { "family": format!("LinkTarget{}", uuid::Uuid::new_v4().simple()), "given": ["T"] },
            "gender": "unknown"
        }),
    )
    .await;
    let mut payload = json!({
        "name": { "family": format!("EnumCols{}", uuid::Uuid::new_v4().simple()), "given": ["E"] },
        "gender": "other"
    });
    with_enum_columns(&mut payload, other);

    let (status, body) = post_worker(&app, &payload).await;
    assert_eq!(status, StatusCode::CREATED, "create failed: {body}");
    let created: serde_json::Value = serde_json::from_str(&body).unwrap();
    let id: uuid::Uuid = created["id"].as_str().unwrap().parse().unwrap();

    let stored = get_worker_json(&app, id).await;
    assert_eq!(stored["name"]["use_type"], json!("official"));
    for key in ["additional_names", "identifiers", "telecom", "links"] {
        assert_eq!(
            sorted(&stored[key]),
            sorted(&created[key]),
            "{key} must read back as written"
        );
    }
    // And as sent: the wire tokens are unchanged by the trip.
    let sent = enum_columns(other);
    assert_eq!(
        sorted(&stored["links"]),
        sorted(&sent["links"]),
        "links as sent"
    );
    let pairs = |list: &serde_json::Value| -> Vec<String> {
        let mut v: Vec<String> = list
            .as_array()
            .unwrap()
            .iter()
            .map(|x| format!("{} {}", x["identifier_type"], x["use_type"]))
            .collect();
        v.sort();
        v
    };
    assert_eq!(
        pairs(&stored["identifiers"]),
        pairs(&sent["identifiers"]),
        "identifiers as sent"
    );
    let uses = |key: &str, field: &str| -> Vec<String> {
        let mut v: Vec<String> = stored[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x[field].to_string())
            .collect();
        v.sort();
        v
    };
    let tokens = |items: &[&str]| -> Vec<String> {
        let mut v: Vec<String> = items.iter().map(|s| format!("\"{s}\"")).collect();
        v.sort();
        v
    };
    assert_eq!(
        uses("additional_names", "use_type"),
        tokens(&["usual", "temp", "nickname", "anonymous", "old", "maiden"])
    );
    assert_eq!(
        uses("telecom", "system"),
        tokens(&["phone", "fax", "email", "pager", "url", "sms", "other"])
    );

    // The database holds the CHECK constraints' spellings.
    let conn = common::db().await;
    let link_types: Vec<String> = sqlx::query_scalar(
        "SELECT link_type::text FROM worker_links WHERE worker_id = $1 ORDER BY link_type",
    )
    .bind(id)
    .fetch_all(&conn)
    .await
    .unwrap();
    assert_eq!(
        link_types,
        ["refer", "replaced_by", "replaces", "seealso"],
        "link types as the CHECK constraint spells them"
    );

    common::purge_record(&conn, id).await;
    common::purge_record(&conn, other).await;
}

/// `POST /api/workers/{id}/assessments`, returning the new assessment's id.
async fn post_assessment(
    app: &axum::Router,
    worker: uuid::Uuid,
    body: &serde_json::Value,
) -> uuid::Uuid {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/workers/{worker}/assessments"))
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
    assert!(
        status.is_success(),
        "create assessment: {status:?} {}",
        String::from_utf8_lossy(&bytes)
    );
    let created: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    created["id"].as_str().unwrap().parse().unwrap()
}

/// A worker with nothing but a unique name, for the assessment tests.
async fn plain_worker(app: &axum::Router, prefix: &str) -> uuid::Uuid {
    create_worker(
        app,
        &json!({
            "name": { "family": format!("{prefix}{}", uuid::Uuid::new_v4().simple()), "given": ["A"] },
            "gender": "unknown"
        }),
    )
    .await
}

/// WK-A1: a worker's assessments list most recently administered first,
/// with undated ones **last**, as the API spec and the query's own doc
/// say. `ORDER BY administered_on DESC` put them first, since Postgres
/// sorts `NULL` first under `DESC`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn undated_assessments_list_last() {
    let app = common::create_test_router().await;
    let id = plain_worker(&app, "AssessOrder").await;
    let older = json!({
        "category": "aptitude", "instrument": "Older", "status": "completed",
        "administered_on": "2025-01-01",
        "results": [{ "scale": "numerical_reasoning", "raw_score": 30.0, "max_score": 50.0 }]
    });
    let newer = json!({
        "category": "aptitude", "instrument": "Newer", "status": "completed",
        "administered_on": "2026-01-01",
        "results": [{ "scale": "numerical_reasoning", "raw_score": 40.0, "max_score": 50.0 }]
    });
    // Created last, so a creation-ordered list would also put it first.
    let undated = json!({ "category": "aptitude", "instrument": "Undated", "status": "scheduled" });
    for body in [&older, &newer, &undated] {
        post_assessment(&app, id, body).await;
    }

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/workers/{id}/assessments"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let listed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let instruments: Vec<&str> = listed
        .as_array()
        .expect("a bare array")
        .iter()
        .map(|a| a["instrument"].as_str().unwrap())
        .collect();
    assert_eq!(instruments, ["Newer", "Older", "Undated"]);

    common::purge_record(&common::db().await, id).await;
}

/// WK-A2: a capped `/api/records/verify` checks the **newest**
/// assessments. With no `ORDER BY` it checked whichever rows the heap
/// scan reached first, so an edit to the newest assessment, made after
/// a dozen older ones, went unchecked under `limit=3`.
///
/// The workers half of the window is unaffected: it already orders by
/// `updated_at`.
#[tokio::test]
#[ignore = "requires a running PostgreSQL database"]
async fn records_verify_checks_the_newest_assessments() {
    const ROWS: usize = 12;
    let app = common::create_test_router().await;
    let id = plain_worker(&app, "AssessNewest").await;
    let mut newest = uuid::Uuid::nil();
    for i in 0..ROWS {
        newest = post_assessment(
            &app,
            id,
            &json!({ "category": "aptitude", "instrument": format!("Window{i}"), "status": "scheduled" }),
        )
        .await;
    }

    // Make the heap order the insertion order. Without this, the edit
    // below can reuse a line pointer freed by an earlier test's purge and
    // land the newest row first in a scan, where the unordered query
    // would happen to check it. Rewritten compactly, the edit's new row
    // version goes after every other row, which is the case the old
    // query missed.
    let conn = common::db().await;
    sqlx::query("VACUUM FULL worker_assessments")
        .execute(&conn)
        .await
        .expect("rewrite the table in insertion order");
    let edited = sqlx::query(
        "UPDATE worker_assessments SET instrument = instrument || ' (edited in SQL)' WHERE id = $1",
    )
    .bind(newest)
    .execute(&conn)
    .await
    .expect("tamper")
    .rows_affected();
    assert_eq!(edited, 1);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/records/verify?limit=3")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    // Leave no deliberately-corrupted record behind, before asserting.
    common::purge_record(&conn, id).await;

    let caught = report["mismatched"]
        .as_array()
        .expect("a mismatched list")
        .iter()
        .any(|m| m["id"].as_str() == Some(newest.to_string().as_str()));
    assert!(
        caught,
        "an edit to the newest assessment must be in a capped window: {report}"
    );
}
