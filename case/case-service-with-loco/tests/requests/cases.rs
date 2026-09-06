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
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
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

/// Pagination: `limit` / `offset` window both collection reads, and
/// `X-Total-Count` reports the collection's match count rather than the
/// page — deliberately the collection's, not the caller's view of it,
/// since a caller-specific total would leak how many records
/// concealment is hiding from them.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn list_and_search_are_paginated() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        for i in 0..5 {
            request
                .post("/api/cases")
                .json(&json!({"title": format!("Paging Case {i}"), "agency_id": "dwp"}))
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

        let page = request.get("/api/cases?limit=2&offset=1").await;
        assert_eq!(page.status_code(), 200);
        let body: Value = page.json();
        assert_eq!(body.as_array().expect("array").len(), 2);
        assert_eq!(header!(page, "x-total-count"), "5");
        assert_eq!(header!(page, "x-limit"), "2");
        assert_eq!(header!(page, "x-offset"), "1");

        let all = request.get("/api/cases").await;
        assert_eq!(all.json::<Value>().as_array().expect("array").len(), 5);
        assert_eq!(header!(all, "x-limit"), "100", "the default is the old cap");

        let clamped = request.get("/api/cases?limit=100000").await;
        assert_eq!(header!(clamped, "x-limit"), "500");

        assert_eq!(
            request.get("/api/cases?offset=10001").await.status_code(),
            400
        );

        let hits = request.get("/api/cases/search?q=Paging&limit=2").await;
        assert_eq!(hits.status_code(), 200, "search page: {}", hits.text());
        assert_eq!(hits.json::<Value>().as_array().expect("array").len(), 2);
        assert_eq!(header!(hits, "x-total-count"), "5");
    })
    .await;
}

// Pins the ILIKE title search: `?q=housing` matches only the housing
// case (case-insensitive substring), and a blank `q` is a 400.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn can_search_cases_by_title() {
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
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

        // The survivor's audit trail carries the `merged` action (spec §6.8).
        let main_audit: Value = request
            .get(&format!("/api/cases/{main_pid}/audit"))
            .await
            .json();
        let main_actions: Vec<&str> = main_audit
            .as_array()
            .expect("main audit rows")
            .iter()
            .filter_map(|r| r["action"].as_str())
            .collect();
        assert!(
            main_actions.contains(&"merged"),
            "survivor should have a `merged` audit row"
        );

        // The duplicate's audit trail carries the `merged_into` action: the
        // merge writes a second audit row against the folded-away pid
        // (spec §6.8 / §9), in addition to its earlier `created` row.
        let dup_audit: Value = request
            .get(&format!("/api/cases/{dup_pid}/audit"))
            .await
            .json();
        let dup_actions: Vec<&str> = dup_audit
            .as_array()
            .expect("dup audit rows")
            .iter()
            .filter_map(|r| r["action"].as_str())
            .collect();
        assert!(
            dup_actions.contains(&"merged_into"),
            "duplicate should have a `merged_into` audit row, got {dup_actions:?}"
        );

        // The folded-away duplicate also gets a `deleted` event (spec §9).
        assert!(
            events
                .as_array()
                .unwrap()
                .iter()
                .any(|e| e["kind"] == "deleted" && e["pid"] == dup_pid),
            "duplicate should get a deleted event"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the self-merge guard: main_pid == duplicate_pid is a 422.
async fn merge_with_equal_pids_is_422() {
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
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
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/swagger-ui").await;
        assert_eq!(response.status_code(), 200, "swagger-ui should be served");
        assert!(response.text().contains("/api-docs/openapi.json"));
    })
    .await;
}

// NOTE (QA-flake fix, 2026-07-18): the blanket-enforcement pin
// (`CASE_REQUIRE_AUTH` on ⇒ un-authed /api/* is 401, OpenAPI stays
// public) lives in `tests/enforcement.rs`, its **own test binary** —
// the flag is cached in a process-wide `OnceLock` on first boot, so a
// `set_var` inside this shared binary is a no-op once any sibling
// test has booted the app (the duplicate here was order-dependent and
// failed whenever it didn't run first).

/// **The audit chain, end to end against Postgres.** A digest computed in
/// Rust before an `INSERT` must still match after Postgres has stored the
/// snapshot as `jsonb` (which reorders keys) and returned `created_at` as
/// a `timestamptz`. No unit test can show that; this is why the check is
/// DB-gated.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn audit_chain_survives_a_jsonb_round_trip() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request.post("/api/cases").json(&housing_case()).await;
        assert_eq!(created.status_code(), 200);
        let pid = created.json::<Value>()["pid"]
            .as_str()
            .expect("pid")
            .to_string();

        let mut changed = housing_case();
        changed["keywords"] = json!(["housing", "appeal", "tribunal"]);
        request
            .put(&format!("/api/cases/{pid}"))
            .json(&changed)
            .await;

        let report: Value = request.get("/api/cases/audit/verify").await.json();
        assert_eq!(
            report["verified"], true,
            "the chain must verify after a Postgres round-trip: {report}"
        );
        assert!(report["rows"].as_u64().unwrap_or(0) >= 2, "{report}");
        assert_eq!(report["breaks"].as_array().map(Vec::len), Some(0));
        assert!(report["head"].is_string());
    })
    .await;
}

/// Rewriting an audit row with raw SQL is reported as a `content` break —
/// the property the chain exists to provide. Case data is personal data,
/// so a silently editable trail is the worst failure mode here.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn tampering_with_an_audit_row_breaks_verification() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        request.post("/api/cases").json(&housing_case()).await;
        let report: Value = request.get("/api/cases/audit/verify").await.json();
        assert_eq!(report["verified"], true, "{report}");

        use sea_orm::ConnectionTrait as _;
        ctx.db
            .execute_unprepared(
                r#"UPDATE audit_logs SET snapshot = jsonb_set(snapshot, '{title}', '"Tampered"')
                   WHERE snapshot IS NOT NULL AND redacted_at IS NULL"#,
            )
            .await
            .expect("tamper");

        let report: Value = request.get("/api/cases/audit/verify").await.json();
        assert_eq!(
            report["verified"], false,
            "an edited audit row must break verification: {report}"
        );
        assert!(
            report["breaks"]
                .as_array()
                .is_some_and(|b| b.iter().any(|x| x["kind"] == "content")),
            "{report}"
        );
    })
    .await;
}

/// The §164.528 accounting declares its own completeness rather than
/// returning a reassuring empty list when read-auditing is off.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn disclosure_accounting_declares_its_completeness() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request.post("/api/cases").json(&housing_case()).await;
        let pid = created.json::<Value>()["pid"]
            .as_str()
            .expect("pid")
            .to_string();

        request
            .get(&format!("/api/cases/{pid}"))
            .add_header("x-purpose-of-use", "research")
            .add_header("x-disclosure-recipient", "University of Example")
            .await;

        let body: Value = request
            .get(&format!("/api/cases/{pid}/audit/disclosures"))
            .await
            .json();
        assert_eq!(body["pid"], pid);
        let caveat = body["caveat"].as_str().unwrap_or_default();
        if body["read_auditing_enabled"].as_bool().unwrap_or(false) {
            assert!(caveat.contains("complete for the period"));
        } else {
            assert!(caveat.contains("INCOMPLETE"), "{caveat}");
        }
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// GDPR Art. 17 erasure: the payload is destroyed, every audit row about
// the case is redacted, and — the load-bearing property — the tamper-
// evident hash chain still verifies afterwards.
//
// That last assertion is the whole reason erasure is implemented as
// redaction rather than deletion. Deleting the audit rows would honour
// Art. 17 and destroy §164.312(c) integrity; refusing the erasure would
// do the reverse. Redaction destroys the content while preserving each
// row's `hash` and `prev_hash`, so linkage still checks across it. If
// this test ever fails, the two obligations have stopped being
// simultaneously satisfiable and the design is broken — not the test.
async fn erasure_destroys_content_and_the_chain_still_verifies() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request.post("/api/cases").json(&housing_case()).await;
        let body: Value = created.json();
        let pid = body["pid"].as_str().expect("pid").to_string();

        // Read it once, so there is audit content to destroy.
        request.get(&format!("/api/cases/{pid}")).await;

        let erased = request.post(&format!("/api/cases/{pid}/erase")).await;
        assert_eq!(erased.status_code(), 200);
        let outcome: Value = erased.json();
        assert_eq!(outcome["pid"].as_str().unwrap(), pid);
        assert!(outcome["payload_erased"].as_bool().unwrap());
        assert!(
            outcome["irreversible"].as_bool().unwrap(),
            "the response must not let a caller mistake this for a soft delete"
        );
        assert!(
            outcome["audit_rows_redacted"].as_u64().unwrap() >= 1,
            "the create above must have left audit content to redact: {outcome}"
        );

        // The payload is gone. The identifier deliberately is not, so a
        // reference from another service resolves to "erased" rather than
        // dangling.
        let after = request.get(&format!("/api/cases/{pid}")).await;
        if after.status_code() == 200 {
            let case: Value = after.json();
            assert_eq!(case["title"].as_str().unwrap(), "(erased)");
            assert!(
                case["case_number"].is_null(),
                "case number survived: {case}"
            );
            assert!(
                case["subjects"].as_array().is_none_or(Vec::is_empty),
                "subjects survived: {case}"
            );
        }

        // The chain still verifies across the redacted rows.
        let verify = request.get("/api/cases/audit/verify?limit=1000").await;
        assert_eq!(verify.status_code(), 200);
        let report: Value = verify.json();
        assert!(
            report["verified"].as_bool().unwrap(),
            "redaction must preserve linkage: {report}"
        );
        assert!(
            report["redacted"].as_u64().unwrap() >= 1,
            "the redacted rows must be counted, not hidden: {report}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Erasure is idempotent, and erasing an unknown or already-erased pid is
// still a valid request rather than a `404`.
//
// A subject's right to erasure does not lapse because the record was
// already soft-deleted — the audit content held about it is still
// personal data. Returning `404` would also confirm to a prober which
// pids are unknown.
async fn erasure_is_idempotent_and_answers_for_an_unknown_pid() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request.post("/api/cases").json(&housing_case()).await;
        let body: Value = created.json();
        let pid = body["pid"].as_str().expect("pid").to_string();

        let first = request.post(&format!("/api/cases/{pid}/erase")).await;
        assert_eq!(first.status_code(), 200);

        let second = request.post(&format!("/api/cases/{pid}/erase")).await;
        assert_eq!(second.status_code(), 200, "re-erasing must be safe");
        let outcome: Value = second.json();
        assert!(outcome["irreversible"].as_bool().unwrap());

        let unknown = uuid::Uuid::new_v4();
        let response = request.post(&format!("/api/cases/{unknown}/erase")).await;
        assert_eq!(
            response.status_code(),
            200,
            "an unknown pid must not be distinguishable from a known one"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Row-level record integrity: every write path leaves the record's content
// hash matching its content, and an out-of-band SQL edit is caught.
//
// This closes the gap this service carried alone among the four with an
// audit chain: an attacker editing a stored case and writing no audit row
// left the chain verifying and nothing else looking.
//
// The write-path half matters more than the tamper half. A path that
// forgets to rehash flags an *untouched* record as tampered — a false
// accusation, which is worse than no control at all — and `create` is the
// only path the compiler helps with, since `update_data`, `soft_delete`
// and the erasure all build their `ActiveModel` from `..Default` or an
// existing row and compile happily with a stale digest.
async fn record_integrity_covers_every_write_path_and_catches_tampering() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        use sea_orm::ConnectionTrait as _;

        async fn verify(request: &loco_rs::TestServer) -> Value {
            let response = request.get("/api/cases/records/verify?limit=500").await;
            assert_eq!(response.status_code(), 200);
            response.json()
        }

        // create
        let created = request.post("/api/cases").json(&housing_case()).await;
        let pid = created.json::<Value>()["pid"].as_str().unwrap().to_string();
        let report = verify(&request).await;
        assert!(
            report["verified"].as_bool().unwrap(),
            "after create: {report}"
        );
        assert!(
            report["intact"].as_u64().unwrap() >= 1,
            "nothing is hashed, so this proves nothing: {report}"
        );

        // update
        let mut changed = housing_case();
        changed["title"] = json!("Housing benefit appeal (amended)");
        let updated = request
            .put(&format!("/api/cases/{pid}"))
            .json(&changed)
            .await;
        assert_eq!(updated.status_code(), 200);
        let report = verify(&request).await;
        assert!(
            report["verified"].as_bool().unwrap(),
            "after update: {report}"
        );

        // soft delete
        request.delete(&format!("/api/cases/{pid}")).await;
        let report = verify(&request).await;
        assert!(
            report["verified"].as_bool().unwrap(),
            "after soft delete: {report}"
        );

        // erase — the tombstone is rehashed rather than cleared, because a
        // case's whole payload is one column and so an erased record is
        // still a complete record.
        let erased = request.post("/api/cases").json(&housing_case()).await;
        let erased_pid = erased.json::<Value>()["pid"].as_str().unwrap().to_string();
        request
            .post(&format!("/api/cases/{erased_pid}/erase"))
            .await;
        let report = verify(&request).await;
        assert!(
            report["verified"].as_bool().unwrap(),
            "after erasure: {report}"
        );

        // Now tamper: edit a stored case directly, writing no audit row.
        let victim = request.post("/api/cases").json(&housing_case()).await;
        let victim_pid = victim.json::<Value>()["pid"].as_str().unwrap().to_string();
        ctx.db
            .execute_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "UPDATE cases SET data = jsonb_set(data, '{case_number}', '\"HB-9999-9999\"') \
                 WHERE pid = $1::uuid",
                [victim_pid.clone().into()],
            ))
            .await
            .expect("tamper");

        // The audit chain cannot see it — which is why this control exists.
        let chain = request.get("/api/cases/audit/verify?limit=1000").await;
        assert!(
            chain.json::<Value>()["verified"].as_bool().unwrap(),
            "a record edit writes no audit row, so the chain still verifies"
        );

        // Record integrity does, and names the case.
        let report = verify(&request).await;
        assert!(
            !report["verified"].as_bool().unwrap(),
            "an out-of-band edit must be detected: {report}"
        );
        let flagged: Vec<&str> = report["mismatched"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|m| m["pid"].as_str())
            .collect();
        assert!(
            flagged.contains(&victim_pid.as_str()),
            "the tampered case must be named: {report}"
        );

        // Leave no corrupted record behind: the database is shared with
        // every other DB-gated target in this crate.
        ctx.db
            .execute_raw(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "DELETE FROM cases WHERE pid = $1::uuid",
                [victim_pid.into()],
            ))
            .await
            .expect("purge");
    })
    .await;
}

/// Tantivy search reaches the fields an `ILIKE` over `title` never could
/// — the involved-party subject a case is *about*, an identifier, the
/// agency it was filed with — and tolerates a typo when asked to.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn search_reaches_secondary_fields_and_tolerates_typos() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/cases")
            .json(&json!({
                "title": "Housing benefit appeal",
                "agency_id": "dwp",
                "agency_name": "Department for Work and Pensions",
                "case_number": "HB-2024-0007",
                "subjects": ["person:abc"],
                "identifiers": [{"scheme": "Docket", "value": "CV-2024-001234"}]
            }))
            .await;
        assert_eq!(created.status_code(), 200);

        let hits = |body: Value| body.as_array().map(Vec::len).unwrap_or_default();

        // The defining attribute of a case is who it is about, and it is
        // now searchable, alongside the agency and identifier. Spaces are
        // percent-encoded — a raw space is not a valid request URI.
        for (q, encoded) in [
            ("person:abc", "person:abc"),
            (
                "Department for Work and Pensions",
                "Department%20for%20Work%20and%20Pensions",
            ),
            ("CV-2024-001234", "CV-2024-001234"),
        ] {
            let response = request.get(&format!("/api/cases/search?q={encoded}")).await;
            assert_eq!(response.status_code(), 200);
            assert_eq!(hits(response.json()), 1, "query {q}");
        }

        // A typo misses on exact retrieval and lands on fuzzy.
        assert_eq!(
            hits(request.get("/api/cases/search?q=Housng").await.json()),
            0
        );
        assert_eq!(
            hits(
                request
                    .get("/api/cases/search?q=Housng&fuzzy=true")
                    .await
                    .json()
            ),
            1,
            "fuzzy must tolerate a dropped letter"
        );

        // A soft-deleted case leaves the index.
        let pid = created.json::<Value>()["pid"].as_str().unwrap().to_string();
        request.delete(&format!("/api/cases/{pid}")).await;
        assert_eq!(
            hits(request.get("/api/cases/search?q=Housing").await.json()),
            0,
            "a deleted case must stop being a hit"
        );
    })
    .await;
}

/// The search-blocked candidate detector reaches a case whose title is
/// wholly different but whose identifier matches — the case
/// `check-duplicates` scanning alone would only find inside the old
/// 1000-row cap.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn check_duplicates_blocks_on_identifier_alone() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/cases")
            .json(&json!({
                "title": "Wholly Unrelated Matter",
                "identifiers": [{"scheme": "Docket", "value": "CV-2024-009999"}]
            }))
            .await;
        assert_eq!(created.status_code(), 200);

        let response = request
            .post("/api/cases/check-duplicates")
            .json(&json!({
                "title": "Housing benefit appeal",
                "identifiers": [{"scheme": "Docket", "value": "CV-2024-009999"}]
            }))
            .await;
        assert_eq!(response.status_code(), 200);
        let hits: Value = response.json();
        let created_body: Value = created.json();
        let pid = created_body["pid"].as_str().unwrap();
        assert!(
            hits.as_array().unwrap().iter().any(|h| h["pid"] == pid),
            "the identifier-only match must be found via blocking: {hits}"
        );
    })
    .await;
}

/// `POST /api/cases/deduplicate` (T-7): a stored near-duplicate pair
/// (same docket, different titles) is found via the search-blocked
/// candidate scan, persisted to the `review_queue` at
/// `provenance = "operator"`, and shows up on a subsequent
/// `GET /review-queue` — proving the round trip end to end, not just
/// that the handler responds `200`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn deduplicate_finds_and_queues_a_stored_pair() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let a = request.post("/api/cases").json(&housing_case()).await;
        assert_eq!(a.status_code(), 200);
        let a_pid = a.json::<Value>()["pid"].as_str().unwrap().to_string();

        // A near-duplicate: different display title, same docket — the
        // deterministic short-circuit `can_check_duplicates_against_
        // stored_cases` already pins scores 1.0 / is_match.
        let b = request
            .post("/api/cases")
            .json(&json!({
                "title": "HB appeal — J. Smith",
                "identifiers": [{"scheme": "Docket", "value": "cv-2024-001234"}]
            }))
            .await;
        assert_eq!(b.status_code(), 200);
        let b_pid = b.json::<Value>()["pid"].as_str().unwrap().to_string();

        // An unrelated third case must not be paired with either.
        let c = request
            .post("/api/cases")
            .json(&json!({"title": "Wholly Unrelated Matter"}))
            .await;
        assert_eq!(c.status_code(), 200);

        let response = request
            .post("/api/cases/deduplicate")
            .json(&json!({}))
            .await;
        assert_eq!(response.status_code(), 200, "deduplicate should succeed");
        let body: Value = response.json();
        assert!(
            body["cases_scanned"].as_u64().unwrap() >= 3,
            "all three seeded cases should be scan seeds: {body}"
        );
        assert_eq!(body["duplicates_found"], 1, "exactly one pair: {body}");
        assert_eq!(body["queued_for_review"], 1);

        let items = body["review_items"].as_array().expect("review_items");
        assert_eq!(items.len(), 1);
        let pair: std::collections::HashSet<&str> = [
            items[0]["case_id_a"].as_str().unwrap(),
            items[0]["case_id_b"].as_str().unwrap(),
        ]
        .into_iter()
        .collect();
        assert_eq!(pair, [a_pid.as_str(), b_pid.as_str()].into_iter().collect());
        assert_eq!(items[0]["status"], "pending");
        assert_eq!(items[0]["provenance"], "operator");
        assert_eq!(items[0]["detection_method"], "batch_deduplication");

        // The stored row is visible on the review-queue listing endpoint
        // (T-8) too — the two features actually compose.
        let listed: Value = request.get("/api/cases/review-queue").await.json();
        let listed_items = listed["items"].as_array().expect("items");
        assert_eq!(listed_items.len(), 1);
        assert_eq!(listed_items[0]["id"], items[0]["id"]);

        // Re-running the scan upserts the same row rather than creating
        // a second one (idempotent — the normalized-pair UNIQUE
        // constraint on `review_queue`).
        let again: Value = request
            .post("/api/cases/deduplicate")
            .json(&json!({}))
            .await
            .json();
        assert_eq!(again["review_items"].as_array().unwrap().len(), 1);
        assert_eq!(again["review_items"][0]["id"], items[0]["id"]);
    })
    .await;
}

/// The always-masked view redacts the involved-party fields regardless
/// of caller (enforcement is off in this suite, so no ABAC decision is
/// in play); the descriptive shell — title — is untouched. The export
/// envelope wraps the same content and declares `masked: false` when
/// enforcement is off, since there is no obligation to honour.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn masked_view_and_export_are_served() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created: Value = request
            .post("/api/cases")
            .json(&housing_case())
            .await
            .json();
        let pid = created["pid"].as_str().unwrap().to_string();

        let masked: Value = request
            .get(&format!("/api/cases/{pid}/masked"))
            .await
            .json();
        assert_eq!(masked["title"], "Housing benefit appeal");
        assert!(masked["subjects"].as_array().unwrap().is_empty());
        assert!(masked["case_number"].is_null());

        let export: Value = request
            .get(&format!("/api/cases/{pid}/export"))
            .await
            .json();
        assert_eq!(export["entity"], "case");
        assert_eq!(export["pid"], pid);
        assert_eq!(export["masked"], false, "no ABAC decision, no obligation");
        assert_eq!(export["record"]["case_number"], "HB-2024-0007");
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
                .get(&format!("/api/cases/{pid}/masked"))
                .await
                .status_code(),
            404
        );
        assert_eq!(
            request
                .get(&format!("/api/cases/{pid}/export"))
                .await
                .status_code(),
            404
        );
    })
    .await;
}
