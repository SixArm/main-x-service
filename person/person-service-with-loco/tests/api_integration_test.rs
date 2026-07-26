#![warn(clippy::pedantic)]

//! Integration tests for the REST API endpoints.
//!
//! These tests build a real [`Router`](axum::Router) via
//! [`common::create_test_router`] (which opens a database connection and
//! search index from the environment config) and drive it with
//! `tower::ServiceExt::oneshot` requests, asserting on status codes and
//! decoded [`ApiResponse`] bodies. They require a reachable `PostgreSQL`
//! instance (see `docker-compose.test.yml`) and exercise the full
//! create / get / update / delete / search lifecycle plus the
//! not-found path.

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use tower::ServiceExt; // for `oneshot` and `ready`

use person_service::{api::ApiResponse, models::Person};

/// `GET /api/health` returns 200 and identifies the service.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
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
    assert!(body_str.contains("person-service"));
}

/// `POST /api/persons` creates a person and assigns a fresh UUID
/// (ignoring the all-zero id in the payload).
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn test_create_person() {
    let app = common::create_test_router().await;

    let family_name = common::unique_person_name("Create");

    let person_json = json!({
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
                .uri("/api/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let api_response: ApiResponse<Person> = serde_json::from_slice(&body).unwrap();
    assert!(api_response.success);

    let person = api_response.data.unwrap();
    assert_eq!(person.name.family, family_name);
    assert_eq!(person.name.given, vec!["Integration", "Test"]);
    assert!(person.id.to_string() != "00000000-0000-0000-0000-000000000000");
}

/// Create a person, then `GET /api/persons/{id}` returns the same
/// record.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn test_create_and_get_person() {
    let app = common::create_test_router().await;

    let family_name = common::unique_person_name("CreateGet");

    // Create person
    let person_json = json!({
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
                .uri("/api/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Person> = serde_json::from_slice(&create_body).unwrap();
    let created_person = create_api_response.data.unwrap();
    let person_id = created_person.id;

    // Get person by ID
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/persons/{person_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let get_body = axum::body::to_bytes(get_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let get_api_response: ApiResponse<Person> = serde_json::from_slice(&get_body).unwrap();
    assert!(get_api_response.success);

    let retrieved_person = get_api_response.data.unwrap();
    assert_eq!(retrieved_person.id, person_id);
    assert_eq!(retrieved_person.name.family, family_name);
}

/// Create a person, then `PUT /api/persons/{id}` persists a changed
/// given-name list.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn test_update_person() {
    let app = common::create_test_router().await;

    let family_name = common::unique_person_name("Update");

    // Create person
    let person_json = json!({
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
                .uri("/api/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Person> = serde_json::from_slice(&create_body).unwrap();
    let mut person = create_api_response.data.unwrap();

    // Update person
    person.name.given = vec!["Update".to_string(), "Modified".to_string()];

    let update_response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/persons/{}", person.id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_response.status(), StatusCode::OK);

    let update_body = axum::body::to_bytes(update_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let update_api_response: ApiResponse<Person> = serde_json::from_slice(&update_body).unwrap();
    let updated_person = update_api_response.data.unwrap();

    assert_eq!(updated_person.name.given, vec!["Update", "Modified"]);
}

/// Create a person, soft-delete it (204), then confirm a subsequent GET
/// returns 404.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn test_delete_person() {
    let app = common::create_test_router().await;

    let family_name = common::unique_person_name("Delete");

    // Create person
    let person_json = json!({
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
                .uri("/api/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let create_body = axum::body::to_bytes(create_response.into_body(), usize::MAX)
        .await
        .unwrap();

    let create_api_response: ApiResponse<Person> = serde_json::from_slice(&create_body).unwrap();
    let person = create_api_response.data.unwrap();

    // Delete person
    let delete_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/persons/{}", person.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    // Try to get deleted person - should return None (or 404 depending on implementation)
    let get_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/persons/{}", person.id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Soft delete means person is not returned
    assert_eq!(get_response.status(), StatusCode::NOT_FOUND);
}

/// Create a person, then `GET /api/persons/search` finds it by family
/// name (after a brief indexing delay).
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn test_search_persons() {
    let app = common::create_test_router().await;

    let family_name = common::unique_person_name("Search");

    // Create a person to search for
    let person_json = json!({
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
                .uri("/api/persons")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&person_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // Give search engine time to index (in production this would be async)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Search for the person
    let search_response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/persons/search?q={family_name}&limit=10"))
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

/// SEC-G7: `GET /api/persons/search` with an out-of-bound pagination
/// `offset` is rejected with `400` — the search engine is never asked to
/// materialise `offset + limit` hits for an unbounded offset.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn test_search_rejects_out_of_bound_offset() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/persons/search?q=anything&offset=1000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "an offset past the SEC-G7 cap must be rejected before searching"
    );
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        body_str.contains("OFFSET_TOO_LARGE"),
        "expected the OFFSET_TOO_LARGE error code, got: {body_str}"
    );
}

/// `GET /api/persons/{id}` for an unknown id returns 404.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn test_get_person_not_found() {
    let app = common::create_test_router().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/persons/00000000-0000-0000-0000-000000000001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// SEC-B5: `POST /api/persons/merge` with `main == duplicate` is rejected
/// with `422` — merging a record into itself would apply the survivor and
/// then soft-delete the same id, destroying the record. The guard runs
/// before any fetch, so no rows need to exist.
#[tokio::test]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with `cargo test --test api_integration_test -- --ignored`"]
async fn test_merge_into_self_is_rejected() {
    let app = common::create_test_router().await;

    let same = "11111111-1111-1111-1111-111111111111";
    let merge_json = json!({
        "main_person_id": same,
        "duplicate_person_id": same,
        "merge_reason": "self-merge guard test"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/persons/merge")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&merge_json).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        body_str.contains("INVALID_MERGE") || body_str.contains("must differ"),
        "expected the self-merge rejection reason, got: {body_str}"
    );
}

/// `GET /api/persons/{id}/audit/disclosures` answers the HIPAA
/// §164.528 accounting question, and — critically — says whether it can
/// answer it at all.
///
/// The caveat is the load-bearing assertion. `PERSON_AUDIT_READS`
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
                .uri("/api/persons")
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
    let created: ApiResponse<Person> = serde_json::from_slice(&body).unwrap();
    let id = created.data.expect("created record").id;

    // Perform an actual disclosing read: naming a recipient is what makes
    // an access a §164.528 *disclosure* rather than an internal access, so
    // without this the accounting would be trivially empty and the test
    // would pin nothing.
    let read = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/persons/{id}"))
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
                .uri(format!("/api/persons/{id}/audit/disclosures"))
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
            caveat.contains("INCOMPLETE") && caveat.contains("PERSON_AUDIT_READS"),
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
                    "/api/persons/{}/audit/disclosures",
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
/// The child-table assertions matter because a person is relational.
/// Unlike care-pathway and case, where the whole payload is one JSONB
/// column, a person's identifiers, addresses, and contacts live in
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
                .uri("/api/persons")
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
    let created: ApiResponse<Person> = serde_json::from_slice(&body).unwrap();
    let person = created.data.expect("created person");
    let id = person.id;
    assert!(
        !person.identifiers.is_empty(),
        "fixture must have data to erase"
    );
    assert!(!person.addresses.is_empty());

    let erased = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/persons/{id}/erase"))
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
    let outcome = parsed.data.expect("an erasure outcome");
    assert!(outcome["irreversible"].as_bool().unwrap());
    assert!(outcome["payload_erased"].as_bool().unwrap());
    assert!(
        outcome["child_rows_deleted"].as_u64().unwrap() >= 3,
        "at least the name, identifier, and address rows: {outcome}"
    );
    assert!(
        outcome["audit_rows_redacted"].as_u64().unwrap() >= 1,
        "the create above left audit content to redact: {outcome}"
    );

    assert_person_data_destroyed(id).await;

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
                .uri(format!("/api/persons/{}/erase", uuid::Uuid::new_v4()))
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

/// Ground truth for a person erasure, in SQL rather than through the API.
///
/// Erasure soft-deletes the record, so a subsequent `GET` may return
/// `404` — and an assertion guarded by "if the read succeeded" would then
/// pass without checking anything at all. A `404` proves the record is
/// unreachable, not that the data is gone, and "unreachable" is exactly
/// the weaker claim erasure is not allowed to settle for.
async fn assert_person_data_destroyed(id: uuid::Uuid) {
    let conn = common::db().await;

    for (table, label) in [
        ("person_identifiers", "identifiers"),
        ("person_addresses", "addresses"),
        ("person_contacts", "contacts"),
        ("person_documents", "documents"),
        ("person_photos", "photos"),
    ] {
        let sql = format!("SELECT count(*) AS n FROM {table} WHERE person_id = $1");
        assert_eq!(
            common::count_rows(&conn, &sql, id).await,
            0,
            "{label} survived erasure"
        );
    }

    // Exactly one name row remains, and it is the tombstone: read paths
    // assume a person has at least one name, so leaving none would make an
    // erased record a landmine rather than a clean degradation.
    assert_eq!(
        common::count_rows(
            &conn,
            "SELECT count(*) AS n FROM person_names WHERE person_id = $1 AND family = '(erased)'",
            id
        )
        .await,
        1,
        "the tombstone name must be present"
    );
    assert_eq!(
        common::count_rows(
            &conn,
            "SELECT count(*) AS n FROM person_names WHERE person_id = $1",
            id
        )
        .await,
        1,
        "a real name survived alongside the tombstone"
    );

    // The parent row itself is scrubbed and retired.
    assert_eq!(
        common::count_rows(
            &conn,
            "SELECT count(*) AS n FROM persons WHERE id = $1 \
             AND tax_id IS NULL AND birth_date IS NULL AND gender = 'unknown' \
             AND active = FALSE AND deleted_at IS NOT NULL",
            id
        )
        .await,
        1,
        "the persons row was not fully scrubbed and retired"
    );
}
