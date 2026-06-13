//! Request-level integration tests over the seven `/api/care-pathways`
//! endpoints (spec §6 / §11; entity spec T-4), in the loco testing style
//! (`loco_rs::testing`, as in the authentication-service sibling).
//!
//! These boot the app against the `test` environment, which needs a
//! reachable PostgreSQL (`config/test.yaml`; override with
//! `DATABASE_URL`). They are `#[ignore]`d so the default `cargo test`
//! stays green on a database-less machine. Run them with:
//!
//! ```sh
//! DATABASE_URL=postgres://loco:loco@localhost:5432/care-pathway-service_test \
//!   cargo test -- --ignored
//! ```
//!
//! The blank-name → `422` contract is additionally pinned un-gated by
//! the DB-free unit tests in `src/controllers/care_pathways.rs`.

use care_pathway_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{json, Value};
use serial_test::serial;

/// A minimal valid pathway payload (the body *is*
/// `care_pathway_matcher::CarePathway`; all fields but `name` default).
fn stroke_pathway() -> Value {
    json!({
        "name": "Acute Stroke Care Pathway",
        "provider_id": "trust-1",
        "pathway_code": "STROKE-01",
        "condition_codes": [{"system": "Icd10", "code": "I63"}],
        "identifiers": [{"scheme": "GuidelineId", "value": "NICE-NG128"}]
    })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_create_care_pathway() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/care-pathways")
            .json(&stroke_pathway())
            .await;
        assert_eq!(response.status_code(), 200, "create should succeed");
        let body: Value = response.json();
        assert_eq!(body["name"], "Acute Stroke Care Pathway");
        let pid = body["pid"].as_str().expect("pid in create response");
        uuid::Uuid::parse_str(pid).expect("pid should be a UUID");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn blank_name_on_create_returns_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/care-pathways")
            .json(&json!({"name": "   "}))
            .await;
        // OQ-1 resolution / T-2: validation failure is 422, not 400.
        assert_eq!(response.status_code(), 422, "blank name should be 422");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn malformed_condition_code_on_create_returns_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut payload = stroke_pathway();
        // A code that is not a well-formed ICD-10 code (spec §6 / T-9).
        payload["condition_codes"] = json!([{"system": "Icd10", "code": "not-a-code"}]);
        let response = request.post("/api/care-pathways").json(&payload).await;
        assert_eq!(
            response.status_code(),
            422,
            "malformed condition code should be 422"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn blank_name_on_update_returns_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/care-pathways")
            .json(&stroke_pathway())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();
        let response = request
            .put(&format!("/api/care-pathways/{pid}"))
            .json(&json!({"name": ""}))
            .await;
        assert_eq!(response.status_code(), 422, "blank name should be 422");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_get_care_pathway_by_pid() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/care-pathways")
            .json(&stroke_pathway())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();

        let response = request.get(&format!("/api/care-pathways/{pid}")).await;
        assert_eq!(response.status_code(), 200, "get by pid should succeed");
        let body: Value = response.json();
        assert_eq!(body["name"], "Acute Stroke Care Pathway");
        assert_eq!(body["pathway_code"], "STROKE-01");
        assert_eq!(body["condition_codes"][0]["code"], "I63");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn unknown_pid_returns_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .get("/api/care-pathways/00000000-0000-4000-8000-000000000000")
            .await;
        assert_eq!(response.status_code(), 404, "unknown pid should be 404");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_list_care_pathways() {
    request::<App, _, _>(|request, _ctx| async move {
        for name in ["Acute Stroke Care Pathway", "Sepsis Care Pathway"] {
            let response = request
                .post("/api/care-pathways")
                .json(&json!({"name": name}))
                .await;
            assert_eq!(response.status_code(), 200);
        }
        let response = request.get("/api/care-pathways").await;
        assert_eq!(response.status_code(), 200, "list should succeed");
        let body: Value = response.json();
        let rows = body.as_array().expect("list returns an array");
        assert_eq!(rows.len(), 2);
        let names: Vec<&str> = rows.iter().filter_map(|r| r["name"].as_str()).collect();
        assert!(names.contains(&"Acute Stroke Care Pathway"));
        assert!(names.contains(&"Sepsis Care Pathway"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_match_query_against_candidates() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/care-pathways/match")
            .json(&json!({
                "query": stroke_pathway(),
                "candidates": [
                    // Same NICE guideline id — deterministic short-circuit.
                    {"name": "Cerebrovascular accident pathway",
                     "identifiers": [{"scheme": "GuidelineId", "value": "nice-ng128"}]},
                    {"name": "Hip Fracture Care Pathway"}
                ]
            }))
            .await;
        assert_eq!(response.status_code(), 200, "match should succeed");
        let body: Value = response.json();
        let ranked = body.as_array().expect("ranked results array");
        assert!(!ranked.is_empty(), "the guideline-id twin should rank");
        // Best hit is candidate 0 with a deterministic 1.0.
        assert_eq!(ranked[0][0], 0);
        assert_eq!(ranked[0][1]["score"], 1.0);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_check_duplicates_against_stored_pathways() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/care-pathways")
            .json(&stroke_pathway())
            .await;
        assert_eq!(response.status_code(), 200);

        // Near-duplicate: different display name, same guideline id.
        let response = request
            .post("/api/care-pathways/check-duplicates")
            .json(&json!({
                "name": "Cerebrovascular accident pathway",
                "identifiers": [{"scheme": "GuidelineId", "value": "nice-ng128"}]
            }))
            .await;
        assert_eq!(
            response.status_code(),
            200,
            "check-duplicates should succeed"
        );
        let body: Value = response.json();
        let hits = body.as_array().expect("hits array");
        assert_eq!(hits.len(), 1, "the stored twin should be detected");
        assert_eq!(hits[0]["name"], "Acute Stroke Care Pathway");
        assert_eq!(hits[0]["score"], 1.0);
        assert_eq!(hits[0]["is_match"], true);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn crud_writes_audit_log_and_events() {
    request::<App, _, _>(|request, _ctx| async move {
        // Create → update → delete one pathway.
        let created: Value = request
            .post("/api/care-pathways")
            .json(&stroke_pathway())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();

        let mut updated = stroke_pathway();
        updated["name"] = json!("Acute Stroke Pathway (rev 2)");
        let response = request
            .put(&format!("/api/care-pathways/{pid}"))
            .json(&updated)
            .await;
        assert_eq!(response.status_code(), 200);

        let response = request.delete(&format!("/api/care-pathways/{pid}")).await;
        assert_eq!(response.status_code(), 200);

        // Three audit rows for this pathway: created, updated, deleted.
        let entity_audit: Value = request
            .get(&format!("/api/care-pathways/{pid}/audit"))
            .await
            .json();
        let rows = entity_audit.as_array().expect("audit rows array");
        assert_eq!(rows.len(), 3, "create + update + delete should each audit");
        let actions: Vec<&str> = rows.iter().filter_map(|r| r["action"].as_str()).collect();
        assert!(actions.contains(&"created"));
        assert!(actions.contains(&"updated"));
        assert!(actions.contains(&"deleted"));
        // No bearer token was sent, so the actor is recorded as null
        // (populated once JWT auth is enforced — T-7).
        assert!(rows.iter().all(|r| r["actor"].is_null()));

        // System-wide recent-audit endpoint returns entries too.
        let recent_audit: Value = request.get("/api/care-pathways/audit/recent").await.json();
        assert!(!recent_audit
            .as_array()
            .expect("recent audit array")
            .is_empty());

        // The in-memory event stream carries the three events for this pid.
        let events: Value = request.get("/api/care-pathways/events/recent").await.json();
        let mine: Vec<&Value> = events
            .as_array()
            .expect("events array")
            .iter()
            .filter(|e| e["pid"] == pid)
            .collect();
        assert_eq!(mine.len(), 3, "three events published for this pathway");
        let kinds: Vec<&str> = mine.iter().filter_map(|e| e["kind"].as_str()).collect();
        assert!(kinds.contains(&"created"));
        assert!(kinds.contains(&"updated"));
        assert!(kinds.contains(&"deleted"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn whoami_without_token_is_401() {
    // No JWKS is configured in tests, and no bearer header is sent, so the
    // protected endpoint must reject. The token-accepted path is pinned
    // un-gated by `auth::tests::valid_token_yields_claims`.
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/care-pathways/whoami").await;
        assert_eq!(response.status_code(), 401, "whoami needs a bearer token");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn openapi_json_is_served() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api-docs/openapi.json").await;
        assert_eq!(response.status_code(), 200, "openapi.json should be served");
        let body: Value = response.json();
        assert_eq!(body["openapi"], "3.0.3");
        assert!(body["paths"]["/api/care-pathways"]["post"].is_object());
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn swagger_ui_is_served() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/swagger-ui").await;
        assert_eq!(response.status_code(), 200, "swagger-ui should be served");
        assert!(response.text().contains("/api-docs/openapi.json"));
    })
    .await;
}
