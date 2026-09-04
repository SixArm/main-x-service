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
//! DATABASE_URL=postgres://loco:loco@localhost:5432/care_pathway_service_test \
//!   cargo test -- --ignored
//! ```
//!
//! The blank-name → `422` contract is additionally pinned un-gated by
//! the DB-free unit tests in `src/controllers/care_pathways.rs`.

use care_pathway_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
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

/// `POST` create returns `200` with a `{pid, name}` ref whose `pid` is a
/// UUID and whose `name` echoes the request.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_create_care_pathway() {
    super::isolate_search_index();
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

/// A blank `name` on create is rejected `422` (OQ-1 / T-2 family rule).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn blank_name_on_create_returns_422() {
    super::isolate_search_index();
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

/// A malformed `condition_codes` entry on create is rejected `422`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn malformed_condition_code_on_create_returns_422() {
    super::isolate_search_index();
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

/// A `Uuid`-scheme identifier with a non-UUID value is rejected `422`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn malformed_identifier_on_create_returns_422() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let mut payload = stroke_pathway();
        // A Uuid-scheme identifier whose value is not a canonical UUID
        // (spec §6 / validation.rs identifier rules).
        payload["identifiers"] = json!([{"scheme": "Uuid", "value": "not-a-uuid"}]);
        let response = request.post("/api/care-pathways").json(&payload).await;
        assert_eq!(
            response.status_code(),
            422,
            "malformed UUID identifier should be 422"
        );
    })
    .await;
}

/// A `same_as` entry that does not parse as an `http(s)://` URL is
/// rejected `422` (spec §6 / spec/13-tasks.md CP-T3).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn malformed_same_as_url_on_create_returns_422() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let mut payload = stroke_pathway();
        payload["same_as"] = json!(["not-a-url"]);
        let response = request.post("/api/care-pathways").json(&payload).await;
        assert_eq!(
            response.status_code(),
            422,
            "malformed same_as URL should be 422"
        );
    })
    .await;
}

/// A blank `name` on update (`PUT`) is rejected `422`, same as create.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn blank_name_on_update_returns_422() {
    super::isolate_search_index();
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

/// `GET /{pid}` returns the full stored `CarePathway` (round-tripped from
/// JSONB), including nested fields like `condition_codes`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_get_care_pathway_by_pid() {
    super::isolate_search_index();
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

/// `GET` of an unknown `pid` is `404`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn unknown_pid_returns_404() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .get("/api/care-pathways/00000000-0000-4000-8000-000000000000")
            .await;
        assert_eq!(response.status_code(), 404, "unknown pid should be 404");
    })
    .await;
}

/// `PUT` of an unknown `pid` is `404` (the payload is valid, so the
/// `404` comes from `find_by_pid`, not validation).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn update_unknown_pid_returns_404() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .put("/api/care-pathways/00000000-0000-4000-8000-000000000000")
            .json(&stroke_pathway())
            .await;
        assert_eq!(
            response.status_code(),
            404,
            "update of unknown pid should be 404"
        );
    })
    .await;
}

/// `DELETE` of an unknown `pid` is `404`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn delete_unknown_pid_returns_404() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .delete("/api/care-pathways/00000000-0000-4000-8000-000000000000")
            .await;
        assert_eq!(
            response.status_code(),
            404,
            "delete of unknown pid should be 404"
        );
    })
    .await;
}

/// `GET` list returns all active rows created in the test.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_list_care_pathways() {
    super::isolate_search_index();
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

/// Pagination: `limit` / `offset` window both collection reads, and
/// `X-Total-Count` reports the whole match set rather than the page.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn list_and_search_are_paginated() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        for i in 0..5 {
            request
                .post("/api/care-pathways")
                .json(&json!({"name": format!("Paging Pathway {i}")}))
                .await;
        }
        macro_rules! header {
            ($r:expr, $name:expr) => {
                $r.headers()
                    .get($name)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_string()
            };
        }

        let page = request.get("/api/care-pathways?limit=2&offset=1").await;
        assert_eq!(page.status_code(), 200);
        let body: Value = page.json();
        assert_eq!(body.as_array().expect("array").len(), 2);
        assert_eq!(
            header!(page, "x-total-count"),
            "5",
            "the total ignores the window"
        );
        assert_eq!(header!(page, "x-limit"), "2");
        assert_eq!(header!(page, "x-offset"), "1");

        // Omitting both parameters is the pre-pagination behaviour.
        let all = request.get("/api/care-pathways").await;
        let body: Value = all.json();
        assert_eq!(body.as_array().expect("array").len(), 5);
        assert_eq!(header!(all, "x-limit"), "100", "the default is the old cap");

        // Clamped, not refused.
        let clamped = request.get("/api/care-pathways?limit=100000").await;
        assert_eq!(header!(clamped, "x-limit"), "500");

        // An out-of-bound offset is a 400 (SEC-G7).
        assert_eq!(
            request
                .get("/api/care-pathways?offset=10001")
                .await
                .status_code(),
            400
        );

        // Search pages the same way, and its total is the match count.
        let hits = request
            .get("/api/care-pathways/search?q=Paging&limit=2")
            .await;
        assert_eq!(hits.status_code(), 200, "search page: {}", hits.text());
        let body: Value = hits.json();
        assert_eq!(body.as_array().expect("array").len(), 2);
        assert_eq!(header!(hits, "x-total-count"), "5");
    })
    .await;
}

/// `GET /search?q=` does a case-insensitive substring match on `name`,
/// and a blank `q` is a `400`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_search_pathways_by_name() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        for name in ["Acute Stroke Care Pathway", "Sepsis Care Pathway"] {
            let response = request
                .post("/api/care-pathways")
                .json(&json!({"name": name}))
                .await;
            assert_eq!(response.status_code(), 200);
        }
        // Case-insensitive substring match on `name`.
        let response = request.get("/api/care-pathways/search?q=stroke").await;
        assert_eq!(response.status_code(), 200, "search should succeed");
        let body: Value = response.json();
        let rows = body.as_array().expect("search returns an array");
        assert_eq!(rows.len(), 1, "only the stroke pathway matches");
        assert_eq!(rows[0]["name"], "Acute Stroke Care Pathway");

        // A blank query is a 400.
        let response = request.get("/api/care-pathways/search?q=").await;
        assert_eq!(response.status_code(), 400, "blank q is rejected");
    })
    .await;
}

/// `POST /match` ranks an explicit candidate list; the guideline-id twin
/// wins with a deterministic `1.0` (no persistence involved).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_match_query_against_candidates() {
    super::isolate_search_index();
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

/// `POST /check-duplicates` scans stored rows and detects the near-twin
/// of a created pathway (same guideline id) with score `1.0`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_check_duplicates_against_stored_pathways() {
    super::isolate_search_index();
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

/// `POST /merge` folds the duplicate into the survivor end to end: title
/// becomes an alternate name, lists union, the duplicate is soft-deleted
/// (`404`), a merge-history row is written, and a `Merged` event fires.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn merge_folds_duplicate_into_survivor() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        // Main: stroke pathway with one ICD-10 code.
        let main: Value = request
            .post("/api/care-pathways")
            .json(&stroke_pathway())
            .await
            .json();
        let main_pid = main["pid"].as_str().expect("main pid").to_string();

        // Duplicate: different title + an extra keyword and code.
        let dup: Value = request
            .post("/api/care-pathways")
            .json(&json!({
                "name": "Cerebrovascular Accident Pathway",
                "keywords": ["acute"],
                "condition_codes": [{"system": "Snomed", "code": "422504002"}]
            }))
            .await
            .json();
        let dup_pid = dup["pid"].as_str().expect("dup pid").to_string();

        // Merge the duplicate into main.
        let response = request
            .post("/api/care-pathways/merge")
            .json(&json!({"main_pid": main_pid, "duplicate_pid": dup_pid, "reason": "confirmed"}))
            .await;
        assert_eq!(response.status_code(), 200, "merge should succeed");
        let body: Value = response.json();
        let merged = &body["main"];
        assert_eq!(merged["name"], "Acute Stroke Care Pathway");
        // The duplicate's title is now an alternate name; its data unioned in.
        let alts = merged["alternate_names"].as_array().expect("alt names");
        assert!(alts.iter().any(|n| n == "Cerebrovascular Accident Pathway"));
        assert_eq!(merged["condition_codes"].as_array().unwrap().len(), 2);
        assert!(
            merged["keywords"]
                .as_array()
                .unwrap()
                .iter()
                .any(|k| k == "acute")
        );

        // The duplicate is gone (soft-deleted).
        let response = request.get(&format!("/api/care-pathways/{dup_pid}")).await;
        assert_eq!(
            response.status_code(),
            404,
            "duplicate should be soft-deleted"
        );

        // A merge-history record exists.
        let merges: Value = request.get("/api/care-pathways/merges/recent").await.json();
        let rows = merges.as_array().expect("merge rows");
        assert!(
            rows.iter()
                .any(|r| r["duplicate_pid"].as_str() == Some(dup_pid.as_str()))
        );

        // A Merged event was published for the survivor.
        let events: Value = request.get("/api/care-pathways/events/recent").await.json();
        assert!(
            events
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "merged" && e["pid"] == main_pid)
        );
    })
    .await;
}

/// `POST /merge` with equal main/duplicate pids is rejected `422`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn merge_with_equal_pids_is_422() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/care-pathways")
            .json(&stroke_pathway())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();
        let response = request
            .post("/api/care-pathways/merge")
            .json(&json!({"main_pid": pid, "duplicate_pid": pid}))
            .await;
        assert_eq!(response.status_code(), 422, "self-merge must be rejected");
    })
    .await;
}

/// `POST /merge` with an unknown duplicate pid is `404`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn merge_unknown_pid_is_404() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/care-pathways")
            .json(&stroke_pathway())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();
        let response = request
            .post("/api/care-pathways/merge")
            .json(&json!({
                "main_pid": pid,
                "duplicate_pid": "00000000-0000-4000-8000-000000000000"
            }))
            .await;
        assert_eq!(response.status_code(), 404, "unknown duplicate is 404");
    })
    .await;
}

/// Create → update → delete writes three audit rows (with null actor,
/// pre-JWT) and publishes three matching events for the pathway.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn crud_writes_audit_log_and_events() {
    super::isolate_search_index();
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
        assert!(
            !recent_audit
                .as_array()
                .expect("recent audit array")
                .is_empty()
        );

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

/// `GET /whoami` without a bearer token is `401` (no JWKS configured in
/// tests). The token-accepted path is pinned un-gated in `auth::tests`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn whoami_without_token_is_401() {
    // No JWKS is configured in tests, and no bearer header is sent, so the
    // protected endpoint must reject. The token-accepted path is pinned
    // un-gated by `auth::tests::valid_token_yields_claims`.
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/care-pathways/whoami").await;
        assert_eq!(response.status_code(), 401, "whoami needs a bearer token");
    })
    .await;
}

/// `GET /api-docs/openapi.json` serves the hand-written OpenAPI 3 doc.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn openapi_json_is_served() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api-docs/openapi.json").await;
        assert_eq!(response.status_code(), 200, "openapi.json should be served");
        let body: Value = response.json();
        assert_eq!(body["openapi"], "3.0.3");
        assert!(body["paths"]["/api/care-pathways"]["post"].is_object());
    })
    .await;
}

/// `GET /swagger-ui` serves the UI page wired to the OpenAPI doc.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn swagger_ui_is_served() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/swagger-ui").await;
        assert_eq!(response.status_code(), 200, "swagger-ui should be served");
        assert!(response.text().contains("/api-docs/openapi.json"));
    })
    .await;
}

// NOTE (QA-CP-FLAKE fix, 2026-07-18): the blanket-enforcement pin
// moved to `tests/enforcement.rs`, its **own test binary** — the
// `CARE_PATHWAY_REQUIRE_AUTH` flag is cached in a process-wide
// `OnceLock` on first boot, so a `set_var` inside this shared binary
// was a no-op once any sibling test had booted the app (the test was
// order-dependent and failed whenever it didn't run first).

/// Tantivy search reaches the fields an `ILIKE` over `name` never could
/// — the condition code a pathway is *about*, an intervention, an
/// identifier — and tolerates a typo when asked to.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn search_reaches_secondary_fields_and_tolerates_typos() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/care-pathways")
            .json(&json!({
                "name": "Acute Stroke Care Pathway",
                "provider_id": "trust-1",
                "condition_codes": [{"system": "Icd10", "code": "I63"}],
                "interventions": ["thrombolysis"],
                "identifiers": [{"scheme": "GuidelineId", "value": "NICE-NG128"}]
            }))
            .await;
        assert_eq!(created.status_code(), 200);

        let hits = |body: Value| body.as_array().map(Vec::len).unwrap_or_default();

        // The defining attribute of a pathway is its condition code, and
        // it is now searchable.
        for q in ["I63", "thrombolysis", "NICE-NG128"] {
            let response = request
                .get(&format!("/api/care-pathways/search?q={q}"))
                .await;
            assert_eq!(response.status_code(), 200);
            assert_eq!(hits(response.json()), 1, "query {q}");
        }

        // A typo misses on exact retrieval and lands on fuzzy.
        assert_eq!(
            hits(
                request
                    .get("/api/care-pathways/search?q=Strok")
                    .await
                    .json()
            ),
            0
        );
        assert_eq!(
            hits(
                request
                    .get("/api/care-pathways/search?q=Strok&fuzzy=true")
                    .await
                    .json()
            ),
            1,
            "fuzzy must tolerate a dropped letter"
        );

        // A soft-deleted pathway leaves the index.
        let pid = created.json::<Value>()["pid"].as_str().unwrap().to_string();
        request.delete(&format!("/api/care-pathways/{pid}")).await;
        assert_eq!(
            hits(
                request
                    .get("/api/care-pathways/search?q=Stroke")
                    .await
                    .json()
            ),
            0,
            "a deleted pathway must stop being a hit"
        );
    })
    .await;
}

/// The always-masked view redacts provider identity regardless of
/// caller (enforcement is off in this suite, so this is the
/// no-policy-needed path); the clinical content — name, condition
/// codes — is untouched. The export envelope wraps the same content and
/// declares `masked: false` when enforcement is off, since with no ABAC
/// decision in play there is no obligation to honour.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn masked_view_and_export_are_served() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/care-pathways")
            .json(&stroke_pathway())
            .await
            .json();
        let pid = created["pid"].as_str().unwrap().to_string();

        let masked: Value = request
            .get(&format!("/api/care-pathways/{pid}/masked"))
            .await
            .json();
        assert_eq!(masked["name"], "Acute Stroke Care Pathway");
        assert_eq!(masked["condition_codes"][0]["code"], "I63");
        let provider_id = masked["provider_id"].as_str().unwrap();
        assert_ne!(provider_id, "trust-1", "must be redacted");
        assert!(provider_id.contains('*'));

        let export: Value = request
            .get(&format!("/api/care-pathways/{pid}/export"))
            .await
            .json();
        assert_eq!(export["entity"], "care_pathway");
        assert_eq!(export["pid"], pid);
        assert_eq!(export["masked"], false, "no ABAC decision, no obligation");
        assert_eq!(export["record"]["provider_id"], "trust-1");
    })
    .await;
}

/// `GET /{pid}/masked` and `/export` are `404` for an unknown pid, same
/// as the ordinary `GET /{pid}`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn masked_view_and_export_are_404_for_unknown_pid() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let pid = "00000000-0000-4000-8000-000000000000";
        assert_eq!(
            request
                .get(&format!("/api/care-pathways/{pid}/masked"))
                .await
                .status_code(),
            404
        );
        assert_eq!(
            request
                .get(&format!("/api/care-pathways/{pid}/export"))
                .await
                .status_code(),
            404
        );
    })
    .await;
}
