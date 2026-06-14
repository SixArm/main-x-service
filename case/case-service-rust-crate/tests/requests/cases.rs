//! Request-level integration tests over the `/api/cases` endpoints
//! (spec §6 / §11; entity spec T-4), in the loco testing style
//! (`loco_rs::testing`, as in the authentication-service sibling).
//!
//! These boot the app against the `test` environment, which needs a
//! reachable PostgreSQL (`config/test.yaml`; override with
//! `DATABASE_URL`). They are `#[ignore]`d so the default `cargo test`
//! stays green on a database-less machine. Run them with:
//!
//! ```sh
//! DATABASE_URL=postgres://loco:loco@localhost:5432/case_service_test \
//!   cargo test -- --ignored
//! ```
//!
//! The blank-title → `422` contract is additionally pinned un-gated by
//! the DB-free unit tests in `src/controllers/cases.rs`.

use case_service::app::App;
use loco_rs::testing::prelude::*;
use serde_json::{Value, json};
use serial_test::serial;

/// A minimal valid case payload (the body *is* `case_matcher::Case`; all
/// fields but `title` default). Shared fixture for the request tests; note
/// it is realistic personal-data-shaped (an agency case number, a subject
/// reference, a docket identifier).
fn housing_case() -> Value {
    json!({
        "title": "Housing benefit appeal",
        "agency_id": "dwp",
        "case_number": "HB-2024-0007",
        "subjects": ["person:abc"],
        "keywords": ["housing", "benefit"],
        "identifiers": [{"scheme": "Docket", "value": "CV-2024-001234"}]
    })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the create happy path: `POST /api/cases` with a valid body
// returns 200, echoes the `title`, and mints a UUID `pid`.
async fn can_create_case() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.post("/api/cases").json(&housing_case()).await;
        assert_eq!(response.status_code(), 200, "create should succeed");
        let body: Value = response.json();
        assert_eq!(body["title"], "Housing benefit appeal");
        let pid = body["pid"].as_str().expect("pid in create response");
        uuid::Uuid::parse_str(pid).expect("pid should be a UUID");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the create-time validation contract end to end: a blank title is
// rejected with 422 (OQ-1 / T-2: not 400). The status is also pinned
// un-gated in `src/controllers/cases.rs`.
async fn blank_title_on_create_returns_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/cases")
            .json(&json!({"title": "   "}))
            .await;
        // OQ-1 resolution / T-2: validation failure is 422, not 400.
        assert_eq!(response.status_code(), 422, "blank title should be 422");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins that opened_date validation runs in the create handler: a
// non-ISO-8601 date is a 422, same status as a blank title.
async fn malformed_opened_date_on_create_returns_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let mut payload = housing_case();
        // A value that is not a well-formed ISO-8601 date (spec §6 / T-9).
        payload["opened_date"] = json!("2024-13-99");
        let response = request.post("/api/cases").json(&payload).await;
        assert_eq!(
            response.status_code(),
            422,
            "malformed opened_date should be 422"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins that validation also runs on the update path (PUT), not just
// create: a blank title on update is a 422.
async fn blank_title_on_update_returns_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/cases")
            .json(&housing_case())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();
        let response = request
            .put(&format!("/api/cases/{pid}"))
            .json(&json!({"title": ""}))
            .await;
        assert_eq!(response.status_code(), 422, "blank title should be 422");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the read-back contract: GET /{pid} returns the full stored Case
// (title, case_number, nested identifier) exactly as created.
async fn can_get_case_by_pid() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/cases")
            .json(&housing_case())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();

        let response = request.get(&format!("/api/cases/{pid}")).await;
        assert_eq!(response.status_code(), 200, "get by pid should succeed");
        let body: Value = response.json();
        assert_eq!(body["title"], "Housing benefit appeal");
        assert_eq!(body["case_number"], "HB-2024-0007");
        assert_eq!(body["identifiers"][0]["value"], "CV-2024-001234");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins that a well-formed but absent pid is a 404 (not a 500 or empty
// 200).
async fn unknown_pid_returns_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .get("/api/cases/00000000-0000-4000-8000-000000000000")
            .await;
        assert_eq!(response.status_code(), 404, "unknown pid should be 404");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the list contract: GET /api/cases returns one CaseRef per active
// case (here both created cases appear).
async fn can_list_cases() {
    request::<App, _, _>(|request, _ctx| async move {
        for title in ["Housing benefit appeal", "Tax credit overpayment"] {
            let response = request
                .post("/api/cases")
                .json(&json!({"title": title}))
                .await;
            assert_eq!(response.status_code(), 200);
        }
        let response = request.get("/api/cases").await;
        assert_eq!(response.status_code(), 200, "list should succeed");
        let body: Value = response.json();
        let rows = body.as_array().expect("list returns an array");
        assert_eq!(rows.len(), 2);
        let titles: Vec<&str> = rows.iter().filter_map(|r| r["title"].as_str()).collect();
        assert!(titles.contains(&"Housing benefit appeal"));
        assert!(titles.contains(&"Tax credit overpayment"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the ILIKE title search: `?q=housing` matches only the housing
// case (case-insensitive substring), and a blank `q` is a 400.
async fn can_search_cases_by_title() {
    request::<App, _, _>(|request, _ctx| async move {
        for title in ["Housing benefit appeal", "Tax credit overpayment"] {
            let response = request
                .post("/api/cases")
                .json(&json!({"title": title}))
                .await;
            assert_eq!(response.status_code(), 200);
        }
        // Case-insensitive substring match on `title`.
        let response = request.get("/api/cases/search?q=housing").await;
        assert_eq!(response.status_code(), 200, "search should succeed");
        let body: Value = response.json();
        let rows = body.as_array().expect("search returns an array");
        assert_eq!(rows.len(), 1, "only the housing case matches");
        assert_eq!(rows[0]["title"], "Housing benefit appeal");

        // A blank query is a 400.
        let response = request.get("/api/cases/search?q=").await;
        assert_eq!(response.status_code(), 400, "blank q is rejected");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the stateless /match endpoint: the candidate sharing the query's
// docket (case-insensitively) ranks first with a deterministic 1.0.
async fn can_match_query_against_candidates() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/cases/match")
            .json(&json!({
                "query": housing_case(),
                "candidates": [
                    // Same docket — deterministic short-circuit.
                    {"title": "HB appeal — J. Smith",
                     "identifiers": [{"scheme": "Docket", "value": "cv-2024-001234"}]},
                    {"title": "Tax credit overpayment"}
                ]
            }))
            .await;
        assert_eq!(response.status_code(), 200, "match should succeed");
        let body: Value = response.json();
        let ranked = body.as_array().expect("ranked results array");
        assert!(!ranked.is_empty(), "the docket twin should rank");
        // Best hit is candidate 0 with a deterministic 1.0.
        assert_eq!(ranked[0][0], 0);
        assert_eq!(ranked[0][1]["score"], 1.0);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins /check-duplicates against persisted rows: a near-duplicate (same
// docket, different title) detects the stored twin with score 1.0 and
// is_match true.
async fn can_check_duplicates_against_stored_cases() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.post("/api/cases").json(&housing_case()).await;
        assert_eq!(response.status_code(), 200);

        // Near-duplicate: different display title, same docket.
        let response = request
            .post("/api/cases/check-duplicates")
            .json(&json!({
                "title": "HB appeal — J. Smith",
                "identifiers": [{"scheme": "Docket", "value": "cv-2024-001234"}]
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
        assert_eq!(hits[0]["title"], "Housing benefit appeal");
        assert_eq!(hits[0]["score"], 1.0);
        assert_eq!(hits[0]["is_match"], true);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the full merge workflow: data unions into the survivor, the
// duplicate's title becomes an alternate title, the duplicate is
// soft-deleted (404 afterwards), a merge-history row is written, and a
// `merged` event is published for the survivor.
async fn merge_folds_duplicate_into_survivor() {
    request::<App, _, _>(|request, _ctx| async move {
        // Main: housing case with one docket identifier.
        let main: Value = request
            .post("/api/cases")
            .json(&housing_case())
            .await
            .json();
        let main_pid = main["pid"].as_str().expect("main pid").to_string();

        // Duplicate: different title + an extra keyword and identifier.
        let dup: Value = request
            .post("/api/cases")
            .json(&json!({
                "title": "HB appeal — John Smith",
                "keywords": ["appeal"],
                "identifiers": [{"scheme": "ExternalCaseId", "value": "EXT-99"}]
            }))
            .await
            .json();
        let dup_pid = dup["pid"].as_str().expect("dup pid").to_string();

        // Merge the duplicate into main.
        let response = request
            .post("/api/cases/merge")
            .json(&json!({"main_pid": main_pid, "duplicate_pid": dup_pid, "reason": "confirmed"}))
            .await;
        assert_eq!(response.status_code(), 200, "merge should succeed");
        let body: Value = response.json();
        let merged = &body["main"];
        assert_eq!(merged["title"], "Housing benefit appeal");
        // The duplicate's title is now an alternate title; its data unioned in.
        let alts = merged["alternate_titles"].as_array().expect("alt titles");
        assert!(alts.iter().any(|n| n == "HB appeal — John Smith"));
        assert_eq!(merged["identifiers"].as_array().unwrap().len(), 2);
        assert!(
            merged["keywords"]
                .as_array()
                .unwrap()
                .iter()
                .any(|k| k == "appeal")
        );

        // The duplicate is gone (soft-deleted).
        let response = request.get(&format!("/api/cases/{dup_pid}")).await;
        assert_eq!(
            response.status_code(),
            404,
            "duplicate should be soft-deleted"
        );

        // A merge-history record exists.
        let merges: Value = request.get("/api/cases/merges/recent").await.json();
        let rows = merges.as_array().expect("merge rows");
        assert!(
            rows.iter()
                .any(|r| r["duplicate_pid"].as_str() == Some(dup_pid.as_str()))
        );

        // A Merged event was published for the survivor.
        let events: Value = request.get("/api/cases/events/recent").await.json();
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

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the self-merge guard: main_pid == duplicate_pid is a 422.
async fn merge_with_equal_pids_is_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/cases")
            .json(&housing_case())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();
        let response = request
            .post("/api/cases/merge")
            .json(&json!({"main_pid": pid, "duplicate_pid": pid}))
            .await;
        assert_eq!(response.status_code(), 422, "self-merge must be rejected");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins that merging into an unknown duplicate pid is a 404.
async fn merge_unknown_pid_is_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/cases")
            .json(&housing_case())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();
        let response = request
            .post("/api/cases/merge")
            .json(&json!({
                "main_pid": pid,
                "duplicate_pid": "00000000-0000-4000-8000-000000000000"
            }))
            .await;
        assert_eq!(response.status_code(), 404, "unknown duplicate is 404");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the auditability contract over a create→update→delete cycle:
// three audit rows (created/updated/deleted) with null actor (no token),
// the system-wide audit endpoint is non-empty, and three matching events
// land on the in-memory stream.
async fn crud_writes_audit_log_and_events() {
    request::<App, _, _>(|request, _ctx| async move {
        // Create → update → delete one case.
        let created: Value = request
            .post("/api/cases")
            .json(&housing_case())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();

        let mut updated = housing_case();
        updated["title"] = json!("Housing benefit appeal (rev 2)");
        let response = request
            .put(&format!("/api/cases/{pid}"))
            .json(&updated)
            .await;
        assert_eq!(response.status_code(), 200);

        let response = request.delete(&format!("/api/cases/{pid}")).await;
        assert_eq!(response.status_code(), 200);

        // Three audit rows for this case: created, updated, deleted.
        let entity_audit: Value = request.get(&format!("/api/cases/{pid}/audit")).await.json();
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
        let recent_audit: Value = request.get("/api/cases/audit/recent").await.json();
        assert!(
            !recent_audit
                .as_array()
                .expect("recent audit array")
                .is_empty()
        );

        // The in-memory event stream carries the three events for this pid.
        let events: Value = request.get("/api/cases/events/recent").await.json();
        let mine: Vec<&Value> = events
            .as_array()
            .expect("events array")
            .iter()
            .filter(|e| e["pid"] == pid)
            .collect();
        assert_eq!(mine.len(), 3, "three events published for this case");
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
// Pins that /whoami is the one always-protected route: no token ⇒ 401.
async fn whoami_without_token_is_401() {
    // No JWKS is configured in tests, and no bearer header is sent, so the
    // protected endpoint must reject. The token-accepted path is pinned
    // un-gated by `auth::tests::valid_token_yields_claims`.
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/cases/whoami").await;
        assert_eq!(response.status_code(), 401, "whoami needs a bearer token");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins that the OpenAPI doc is mounted and served (3.0.3, with the
// create path present).
async fn openapi_json_is_served() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api-docs/openapi.json").await;
        assert_eq!(response.status_code(), 200, "openapi.json should be served");
        let body: Value = response.json();
        assert_eq!(body["openapi"], "3.0.3");
        assert!(body["paths"]["/api/cases"]["post"].is_object());
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins that the Swagger UI page is served and points at the OpenAPI JSON.
async fn swagger_ui_is_served() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/swagger-ui").await;
        assert_eq!(response.status_code(), 200, "swagger-ui should be served");
        assert!(response.text().contains("/api-docs/openapi.json"));
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the blanket-enforcement layer: with CASE_REQUIRE_AUTH on, an
// un-authed /api/* request is 401 while the public OpenAPI doc still
// serves 200.
async fn blanket_enforcement_gates_api_but_not_public_paths() {
    // With `CASE_REQUIRE_AUTH` on and no JWKS configured, an un-authed
    // `/api/*` request must 401, while the public OpenAPI doc still serves.
    // `require_auth()` is read once via a `OnceLock`, so set the env before
    // the app boots; `#[serial]` keeps this isolated from other tests.
    // SAFETY: single-threaded under `#[serial]`; no concurrent env access.
    unsafe {
        std::env::set_var("CASE_REQUIRE_AUTH", "1");
    }
    request::<App, _, _>(|request, _ctx| async move {
        let gated = request.get("/api/cases").await;
        assert_eq!(
            gated.status_code(),
            401,
            "un-authed /api/cases should 401 when enforcement is on"
        );

        let doc = request.get("/api-docs/openapi.json").await;
        assert_eq!(
            doc.status_code(),
            200,
            "openapi.json stays public under enforcement"
        );
    })
    .await;
    unsafe {
        std::env::remove_var("CASE_REQUIRE_AUTH");
    }
}
