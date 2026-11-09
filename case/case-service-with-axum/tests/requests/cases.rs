//! Request-level integration tests over the `/api/cases` endpoints
//! (spec §6 / §11; entity spec T-4), in the axa testing style
//! (`axa::testing`, as in the authentication-service sibling).
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

use axa::testing::prelude::*;
use case_service::app::App;
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

/// Pagination: `limit` sizes a page, `Link: rel="next"` carries the
/// signed cursor to the next one until the last, there is no total, and
/// a limit too large or a cursor this service did not sign is a `422`.
/// Whether more pages follow is decided on the fetched batch, never on
/// the caller's concealed view of it.
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

        // Follow `Link: rel="next"` from `first` to the last page, and
        // return each page's length and every pid seen.
        macro_rules! walk {
            ($first:expr) => {{
                let mut url = $first.to_string();
                let mut sizes = Vec::new();
                let mut pids = std::collections::BTreeSet::new();
                loop {
                    let page = request.get(&url).await;
                    assert_eq!(page.status_code(), 200, "{url}: {}", page.text());
                    for gone in ["x-total-count", "x-limit", "x-offset"] {
                        assert!(page.headers().get(gone).is_none(), "no {gone}");
                    }
                    let body: Value = page.json();
                    let rows = body.as_array().expect("array");
                    sizes.push(rows.len());
                    pids.extend(rows.iter().map(|row| row["pid"].to_string()));
                    let Some(link) = page.headers().get("link") else {
                        break;
                    };
                    let link = link.to_str().expect("ascii link");
                    assert!(link.ends_with(r#">; rel="next""#), "{link}");
                    url = link[1..link.find('>').expect("<url>")].to_string();
                }
                (sizes, pids)
            }};
        }

        let (sizes, pids) = walk!("/api/cases?limit=2");
        assert_eq!(
            sizes,
            [2, 2, 1],
            "pages of two, and the last says so by having no link"
        );
        assert_eq!(pids.len(), 5, "every row once");

        // Omitting `limit` is the old cap, which holds all five.
        let (sizes, _) = walk!("/api/cases");
        assert_eq!(sizes, [5]);

        // A limit past the maximum, or not a number, is refused, never
        // clamped and never silently defaulted.
        for refused in ["/api/cases?limit=100000", "/api/cases?limit=lots"] {
            assert_eq!(request.get(refused).await.status_code(), 422, "{refused}");
        }

        // A cursor is the service's own: a made-up one, or one carried to
        // another query, is refused.
        assert_eq!(
            request
                .get("/api/cases?cursor=AQAAAAAAAAAK")
                .await
                .status_code(),
            422
        );
        let first = request.get("/api/cases?limit=2").await;
        let link = first.headers()["link"].to_str().unwrap().to_string();
        let cursor = &link[link.find("cursor=").unwrap() + 7..link.find('>').unwrap()];
        assert_eq!(
            request
                .get(&format!(
                    "/api/cases/search?q=Paging&limit=2&cursor={cursor}"
                ))
                .await
                .status_code(),
            422,
            "a list cursor is no good for a search"
        );

        // Search pages the same way, through the index.
        let (sizes, pids) = walk!("/api/cases/search?q=Paging&limit=2");
        assert_eq!(sizes, [2, 2, 1]);
        assert_eq!(pids.len(), 5, "all five match the query");
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
        // Match evidence rides along (agents/share/match-evidence.md): the
        // shared docket proves the pair.
        assert_eq!(hits[0]["evidence"]["strength"], "proven", "{body}");
        assert!(hits[0]["evidence"]["weight_bits"].as_f64().unwrap() >= 30.0);
        assert!(hits[0]["evidence"]["components"].is_array());
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

        sqlx::query!(
            r#"UPDATE audit_logs SET snapshot = jsonb_set(snapshot, '{title}', '"Tampered"')
               WHERE snapshot IS NOT NULL AND redacted_at IS NULL"#,
        )
        .execute(&ctx.db)
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

/// `(rows, redacted rows, erased rows)` in the audit trail for `pid`.
async fn audit_counts(db: &sqlx::PgPool, pid: &str) -> (i64, i64, i64) {
    let row = sqlx::query!(
        r#"SELECT count(*) AS "rows!",
                  count(*) FILTER (WHERE redacted_at IS NOT NULL) AS "redacted!",
                  count(*) FILTER (WHERE action = 'erased') AS "erased!"
           FROM audit_logs WHERE entity_pid = $1::text::uuid"#,
        pid,
    )
    .fetch_one(db)
    .await
    .expect("audit counts");
    (row.rows, row.redacted, row.erased)
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// CS-G1: GDPR erasure is all or nothing. A trigger makes the **last**
// step — appending the chained `erased` audit row — fail for this one
// case, and every earlier step must then be undone too: the payload not
// tombstoned, the `subject_of` link still active, the audit content not
// redacted, and the chain still verifying with no `erased` row. Before
// the fix the tombstone, the link sweep and the redaction had each
// committed on their own, leaving the subject half erased with no
// accountability row recording that anything happened.
//
// The trigger fires only on an `erased` insert, which only an erasure
// writes. It is installed and dropped around the one request, before any
// assertion, and the suite is `#[serial]`, so it cannot reach another
// test.
async fn erasure_failing_late_leaves_nothing_changed() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let created = request.post("/api/cases").json(&housing_case()).await;
        let pid = created.json::<Value>()["pid"]
            .as_str()
            .expect("pid")
            .to_string();
        let link = request
            .post(&format!("/api/cases/{pid}/links"))
            .json(&json!({
                "kind": "subject_of",
                "to_ref": "person:0c4f1e2a-0000-4000-8000-000000000000"
            }))
            .await;
        assert_eq!(link.status_code(), 200, "link create should succeed");
        let (rows_before, redacted_before, _) = audit_counts(&ctx.db, &pid).await;
        assert!(rows_before >= 1, "the create must leave audit content");
        assert_eq!(redacted_before, 0);

        sqlx::query!(
            "CREATE OR REPLACE FUNCTION cs_g1_fail_erased() RETURNS trigger
                 LANGUAGE plpgsql AS $$
                 BEGIN RAISE EXCEPTION 'CS-G1 test: erased audit row refused'; END $$"
        )
        .execute(&ctx.db)
        .await
        .expect("install the failing function");
        sqlx::query!("DROP TRIGGER IF EXISTS cs_g1_fail_erased ON audit_logs")
            .execute(&ctx.db)
            .await
            .expect("clear any stale trigger");
        sqlx::query!(
            "CREATE TRIGGER cs_g1_fail_erased BEFORE INSERT ON audit_logs
                 FOR EACH ROW WHEN (NEW.action = 'erased')
                 EXECUTE FUNCTION cs_g1_fail_erased()"
        )
        .execute(&ctx.db)
        .await
        .expect("install the failing trigger");

        let erase = request.post(&format!("/api/cases/{pid}/erase")).await;

        sqlx::query!("DROP TRIGGER cs_g1_fail_erased ON audit_logs")
            .execute(&ctx.db)
            .await
            .expect("drop the failing trigger");
        sqlx::query!("DROP FUNCTION cs_g1_fail_erased()")
            .execute(&ctx.db)
            .await
            .expect("drop the failing function");

        assert_eq!(
            erase.status_code(),
            500,
            "a failed audit append must fail the erasure: {}",
            erase.text()
        );

        // Step 1 undone: the case is live and still holds its payload.
        let case = request.get(&format!("/api/cases/{pid}")).await;
        assert_eq!(case.status_code(), 200, "the case must still be live");
        let case: Value = case.json();
        assert_eq!(case["title"], "Housing benefit appeal");
        assert_eq!(case["case_number"], "HB-2024-0007");
        // Step 2 undone: the `subject_of` link is still active.
        let links: Vec<Value> = request.get(&format!("/api/cases/{pid}/links")).await.json();
        assert_eq!(links.len(), 1, "the link sweep must be rolled back");
        // Step 3 undone, and step 4 absent: nothing redacted, no `erased`.
        let (_, redacted, erased) = audit_counts(&ctx.db, &pid).await;
        assert_eq!(redacted, 0, "the redaction must be rolled back");
        assert_eq!(
            erased, 0,
            "no erased row for an erasure that did not happen"
        );
        // The chain still verifies: the rolled-back append left no row.
        let verify: Value = request
            .get("/api/cases/audit/verify?limit=1000")
            .await
            .json();
        assert!(
            verify["verified"].as_bool().unwrap(),
            "the chain must verify after a failed erasure: {verify}"
        );

        // A retry with the fault gone erases fully.
        let retry = request.post(&format!("/api/cases/{pid}/erase")).await;
        assert_eq!(retry.status_code(), 200, "the retry should succeed");
        let outcome: Value = retry.json();
        assert_eq!(outcome["links_withdrawn"], 1);
        // The links endpoint answers `404` for an erased case, so read the
        // table directly.
        let live_links = sqlx::query_scalar!(
            r#"SELECT count(*) AS "n!" FROM entity_links
               WHERE from_pid = $1::text::uuid AND deleted_at IS NULL"#,
            pid,
        )
        .fetch_one(&ctx.db)
        .await
        .expect("count links");
        assert_eq!(live_links, 0, "the retry must withdraw the link");
        let (rows, redacted, erased) = audit_counts(&ctx.db, &pid).await;
        assert_eq!(erased, 1, "exactly one erased row");
        assert_eq!(redacted, rows - 1, "every row but the erased one redacted");
        let title = sqlx::query_scalar!("SELECT title FROM cases WHERE pid = $1::text::uuid", pid)
            .fetch_one(&ctx.db)
            .await
            .expect("the row survives erasure");
        assert_eq!(title, "(erased)");
        let verify: Value = request
            .get("/api/cases/audit/verify?limit=1000")
            .await
            .json();
        assert!(
            verify["verified"].as_bool().unwrap(),
            "the chain must verify after the retry: {verify}"
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
// and the erasure each write the digest columns in their own `UPDATE`
// and would compile happily with a stale digest.
async fn record_integrity_covers_every_write_path_and_catches_tampering() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        async fn verify(request: &axa::TestServer) -> Value {
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
        sqlx::query!(
            r#"UPDATE cases SET data = jsonb_set(data, '{case_number}', '"HB-9999-9999"')
               WHERE pid = $1::text::uuid"#,
            victim_pid,
        )
        .execute(&ctx.db)
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
        sqlx::query!("DELETE FROM cases WHERE pid = $1::text::uuid", victim_pid)
            .execute(&ctx.db)
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
/// that the handler responds `200`. The shared docket makes the
/// evidence `proven` at score 1.0, so the pair is marked `automerged`
/// (match-evidence.md §4).
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
        assert_eq!(body["queued_for_review"], 0);
        assert_eq!(body["auto_merged"], 1);

        let items = body["review_items"].as_array().expect("review_items");
        assert_eq!(items.len(), 1);
        let pair: std::collections::HashSet<&str> = [
            items[0]["case_id_a"].as_str().unwrap(),
            items[0]["case_id_b"].as_str().unwrap(),
        ]
        .into_iter()
        .collect();
        assert_eq!(pair, [a_pid.as_str(), b_pid.as_str()].into_iter().collect());
        assert_eq!(items[0]["status"], "automerged");
        assert_eq!(items[0]["evidence"]["strength"], "proven");
        assert_eq!(items[0]["provenance"], "operator");
        assert_eq!(items[0]["detection_method"], "batch_deduplication");

        // The stored row is visible on the review-queue listing endpoint
        // (T-8) too — the two features actually compose.
        let listed: Value = request.get("/api/cases/review-queue").await.json();
        let listed_items = listed["items"].as_array().expect("items");
        assert_eq!(listed_items.len(), 1);
        assert_eq!(listed_items[0]["id"], items[0]["id"]);
        assert_eq!(listed_items[0]["evidence"]["strength"], "proven");

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

/// Two cases that agree only on their title score above the auto-merge
/// threshold, but nothing proves them one case: the batch scan leaves the
/// pair `pending` with `consistent` evidence, stored and returned
/// (match-evidence.md §4).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn deduplicate_leaves_a_title_only_pair_pending_as_consistent() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        for _ in 0..2 {
            let r = request
                .post("/api/cases")
                .json(&json!({"title": "Housing benefit appeal"}))
                .await;
            assert_eq!(r.status_code(), 200);
        }
        let body: Value = request
            .post("/api/cases/deduplicate")
            .json(&json!({}))
            .await
            .json();
        let items = body["review_items"].as_array().expect("review_items");
        assert_eq!(items.len(), 1, "{body}");
        assert!(items[0]["match_score"].as_f64().unwrap() >= 0.95, "{body}");
        assert_eq!(items[0]["status"], "pending");
        assert_eq!(items[0]["evidence"]["strength"], "consistent");
        assert_eq!(body["auto_merged"], 0);
        assert_eq!(body["queued_for_review"], 1);

        let listed: Value = request.get("/api/cases/review-queue").await.json();
        assert_eq!(listed["items"][0]["status"], "pending");
        assert_eq!(listed["items"][0]["evidence"]["strength"], "consistent");
    })
    .await;
}

/// Two dockets under one scheme contradict each other: the pair stays
/// `pending` with `contradicted` evidence however alike the rest is.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn deduplicate_leaves_a_contradicted_pair_pending() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        for docket in ["CV-2024-1", "CV-2024-2"] {
            let r = request
                .post("/api/cases")
                .json(&json!({
                    "title": "Housing benefit appeal",
                    "identifiers": [{"scheme": "Docket", "value": docket}]
                }))
                .await;
            assert_eq!(r.status_code(), 200);
        }
        let body: Value = request
            .post("/api/cases/deduplicate")
            .json(&json!({}))
            .await
            .json();
        let items = body["review_items"].as_array().expect("review_items");
        assert_eq!(items.len(), 1, "{body}");
        assert_eq!(items[0]["status"], "pending");
        assert_eq!(items[0]["evidence"]["strength"], "contradicted");
        assert_eq!(
            items[0]["evidence"]["contradictions"][0]["field"],
            "identifier:docket"
        );
        assert_eq!(body["auto_merged"], 0);
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

/// Pins that the import route has its own body limit. The service-wide limit is
/// 2 MB, and a file over it must still reach the import rather than being
/// refused at the door with a `413`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn an_import_over_the_default_body_limit_is_accepted() {
    use axum_test::multipart::{MultipartForm, Part};

    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let form = MultipartForm::new().add_text("format", "jsonl").add_part(
            "file",
            Part::bytes(vec![b'\n'; 3_000_000])
                .file_name("import.jsonl")
                .mime_type("application/octet-stream"),
        );
        let response = request.post("/api/cases/import").multipart(form).await;
        assert_eq!(response.status_code(), 202);
    })
    .await;
}

/// The SBOM names every dependency's exact version, so it needs a verified
/// caller with `CASE_REQUIRE_AUTH` off too, when the guard lets everyone by.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn sbom_needs_a_caller_with_enforcement_off() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(request.get("/api/compliance/sbom").await.status_code(), 401);
    })
    .await;
}

/// `pid`s listed under `mismatched` by `GET /api/cases/records/verify`.
async fn mismatched_pids(request: &axa::TestServer) -> Vec<String> {
    let response = request.get("/api/cases/records/verify?limit=1000").await;
    assert_eq!(response.status_code(), 200, "records verify should answer");
    let report: Value = response.json();
    report["mismatched"]
        .as_array()
        .expect("mismatched array")
        .iter()
        .map(|m| m["pid"].as_str().expect("mismatch pid").to_string())
        .collect()
}

/// INT-V1: a record the service wrote must verify. The digests are
/// computed from the payload the caller sent, while verification
/// recomputes them from the row as read back from Postgres, so any
/// normalisation in between (JSONB key order or number form, a server-set
/// column, timestamp precision) would flag every fresh record as tampered.
/// Create, update, merge (survivor and soft-deleted duplicate) all have to
/// verify, and a row edited in SQL must still be flagged.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn a_written_record_verifies() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        // Every field the payload carries, including non-ASCII text and
        // the nested identifiers, so the digest is exercised over the
        // whole stored record, not just `title`.
        let created: Value = request
            .post("/api/cases")
            .json(&json!({
                "title": "Verifiable appeal — Zürich 日本",
                "alternate_titles": ["VA-1", "Appeal (verifiable)"],
                "case_number": "VA-2026-0001",
                "agency_id": "dwp",
                "agency_name": "Department for Work and Pensions",
                "case_type": "Appeal",
                "status": "Open",
                "priority": "High",
                "opened_date": "2026-01-02",
                "subjects": ["person:verifiable"],
                "keywords": ["ledger", "Zürich", "日本"],
                "identifiers": [
                    {"scheme": "Docket", "value": "CV-2026-000999"},
                    {"scheme": "ExternalCaseId", "value": "EXT-VA-1"}
                ],
                "same_as": ["https://example.org/cases/va-1"],
                "in_language": ["en", "de"]
            }))
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();
        // Not vacuous: an unhashed row is never a mismatch, so the record
        // must actually carry both digests for the checks below to mean
        // anything.
        let digests = sqlx::query!(
            "SELECT content_hash, content_hash_sha3 FROM cases WHERE pid = $1::text::uuid",
            pid,
        )
        .fetch_one(&ctx.db)
        .await
        .expect("read digests");
        assert!(
            digests.content_hash.is_some() && digests.content_hash_sha3.is_some(),
            "a new record carries its digests"
        );
        let mismatched = mismatched_pids(&request).await;
        assert!(
            !mismatched.contains(&pid),
            "a freshly created record must verify; mismatched: {mismatched:?}"
        );

        let response = request
            .put(&format!("/api/cases/{pid}"))
            .json(&json!({
                "title": "Verifiable appeal (amended)",
                "agency_id": "dwp",
                "case_number": "VA-2026-0001",
                "keywords": ["ledger"],
                "identifiers": [{"scheme": "Docket", "value": "CV-2026-000999"}]
            }))
            .await;
        assert_eq!(response.status_code(), 200, "update should succeed");
        let mismatched = mismatched_pids(&request).await;
        assert!(
            !mismatched.contains(&pid),
            "an updated record must verify; mismatched: {mismatched:?}"
        );

        // Merge: the survivor is rewritten and the duplicate soft-deleted,
        // and both rows are in the verified set.
        let dup: Value = request
            .post("/api/cases")
            .json(&json!({
                "title": "Unrelated tax credit review",
                "keywords": ["review"],
                "identifiers": [{"scheme": "ExternalCaseId", "value": "EXT-VA-2"}]
            }))
            .await
            .json();
        let dup_pid = dup["pid"].as_str().expect("dup pid").to_string();
        let response = request
            .post("/api/cases/merge")
            .json(&json!({"main_pid": pid, "duplicate_pid": dup_pid, "reason": "confirmed"}))
            .await;
        assert_eq!(response.status_code(), 200, "merge should succeed");
        let mismatched = mismatched_pids(&request).await;
        assert!(
            !mismatched.contains(&pid) && !mismatched.contains(&dup_pid),
            "a merge survivor and its soft-deleted duplicate must verify; mismatched: {mismatched:?}"
        );

        // An edit made behind the service's back must still be caught.
        sqlx::query!(
            "UPDATE cases SET title = title || ' (edited in SQL)' WHERE pid = $1::text::uuid",
            pid,
        )
        .execute(&ctx.db)
        .await
        .expect("raw SQL edit");
        let mismatched = mismatched_pids(&request).await;
        assert!(
            mismatched.contains(&pid),
            "a row edited in SQL must be reported; mismatched: {mismatched:?}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// CS-T1: `db truncate` empties every table the service owns. It used to
// leave `event_outbox`, `bulk_jobs` and `review_queue`, so a truncated
// database still held unsent events, jobs and review pairs for records it
// no longer had — and the relay would go on shipping `created` events for
// cases that were gone.
async fn truncate_clears_outbox_bulk_jobs_and_review_queue() {
    use case_service::bulk::BulkFormat;
    use case_service::models::{bulk_jobs, event_outbox::OutboxInsert, review_queue};

    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let created = request.post("/api/cases").json(&housing_case()).await;
        assert_eq!(created.status_code(), 200);
        let pid = uuid::Uuid::parse_str(created.json::<Value>()["pid"].as_str().unwrap()).unwrap();

        let now: chrono::DateTime<chrono::FixedOffset> = chrono::Utc::now().into();
        OutboxInsert {
            event_id: uuid::Uuid::new_v4(),
            entity: "case".to_string(),
            entity_pid: pid,
            kind: "created".to_string(),
            occurred_at: now,
            actor: None,
            schema_version: 1,
            payload: json!({}),
        }
        .insert_on(&ctx.db)
        .await
        .expect("seed the outbox");
        bulk_jobs::create(
            &ctx.db,
            bulk_jobs::NewBulkJob::import(BulkFormat::Jsonl, json!({}), None),
        )
        .await
        .expect("seed a bulk job");
        review_queue::upsert(
            &ctx.db,
            &[review_queue::NewReviewItem {
                record_id_a: pid,
                record_id_b: uuid::Uuid::new_v4(),
                match_score: 0.9,
                match_quality: "probable".to_string(),
                detection_method: "test".to_string(),
                score_breakdown: None,
                status: "pending".to_string(),
                provenance: "operator".to_string(),
                evidence: None,
            }],
        )
        .await
        .expect("seed the review queue");

        App::truncate(&ctx).await.expect("truncate");

        let left = sqlx::query!(
            r#"SELECT (SELECT count(*) FROM cases) AS "cases!",
                      (SELECT count(*) FROM audit_logs) AS "audit_logs!",
                      (SELECT count(*) FROM entity_links) AS "entity_links!",
                      (SELECT count(*) FROM merge_records) AS "merge_records!",
                      (SELECT count(*) FROM event_outbox) AS "event_outbox!",
                      (SELECT count(*) FROM bulk_jobs) AS "bulk_jobs!",
                      (SELECT count(*) FROM review_queue) AS "review_queue!""#
        )
        .fetch_one(&ctx.db)
        .await
        .expect("count rows");
        assert_eq!(
            [
                left.cases,
                left.audit_logs,
                left.entity_links,
                left.merge_records,
                left.event_outbox,
                left.bulk_jobs,
                left.review_queue,
            ],
            [0; 7],
            "every service table must be empty after truncate"
        );
    })
    .await;
}

/// Erasure removes the case from the full-text index (CS-E1). The index
/// keeps its own copy of the title and subjects, so an erasure that left
/// it there would keep the subject's data at rest after Art. 17 said it
/// was gone, even though search answers resolve against the tombstoned
/// row and so never return it.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (DATABASE_URL); run with: cargo test -- --ignored"]
async fn erasure_removes_the_case_from_the_search_index() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let mut body = housing_case();
        let title = format!("Erasable title {}", uuid::Uuid::new_v4().simple());
        body["title"] = json!(title);
        let created = request.post("/api/cases").json(&body).await;
        let pid = created.json::<Value>()["pid"]
            .as_str()
            .expect("pid")
            .to_string();
        let engine = case_service::search::engine().expect("the search index");
        assert!(
            engine.search(&title, 10).unwrap().contains(&pid),
            "the new case must be indexed"
        );

        let erase = request.post(&format!("/api/cases/{pid}/erase")).await;
        assert_eq!(erase.status_code(), 200, "erase: {}", erase.text());
        engine.reload().ok();
        assert!(
            !engine.search(&title, 10).unwrap().contains(&pid),
            "an erased case must leave the index"
        );
    })
    .await;
}

/// Create two distinct cases through the API and load both rows.
async fn two_cases(
    request: &axa::TestServer,
    db: &sqlx::PgPool,
) -> (
    case_service::models::cases::Model,
    case_service::models::cases::Model,
) {
    use case_service::models::cases::Model as CaseModel;
    let mut pids = Vec::new();
    for title in ["Zoning permit dispute", "Fishing licence renewal"] {
        let created = request
            .post("/api/cases")
            .json(&json!({ "title": format!("{title} {}", uuid::Uuid::new_v4()) }))
            .await;
        assert_eq!(created.status_code(), 200);
        pids.push(created.json::<Value>()["pid"].as_str().unwrap().to_string());
    }
    let main = CaseModel::find_by_pid(db, &pids[0]).await.unwrap();
    let duplicate = CaseModel::find_by_pid(db, &pids[1]).await.unwrap();
    (main, duplicate)
}

/// Retire `pid` the way a concurrent merge or delete would, on `conn`.
async fn retire(conn: &mut sqlx::PgConnection, pid: uuid::Uuid) {
    sqlx::query("UPDATE cases SET active = false, deleted_at = now() WHERE pid = $1")
        .bind(pid)
        .execute(conn)
        .await
        .expect("retire the row");
}

// CS-M1 (SEC-B5): the handler reads both rows unlocked, so by the time
// the merge writes, the duplicate may already have been merged away. The
// merge must re-check liveness under its lock and fail closed, under the
// default `memory` transport as well as `outbox`, rather than fold a
// retired duplicate's data into the survivor.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn merge_refuses_a_duplicate_retired_after_it_was_read() {
    use case_service::models::{ModelError, cases::Model as CaseModel};
    use case_service::streaming;

    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let (main, duplicate) = two_cases(&request, &ctx.db).await;
        let (main_pid, dup_pid) = (main.pid, duplicate.pid);
        let before = main.data.clone();
        let mut folded = main.to_case().unwrap();
        folded
            .keywords
            .push("folded-from-a-retired-duplicate".to_string());

        // Retired after the read, before the merge runs.
        let mut conn = ctx.db.acquire().await.unwrap();
        retire(&mut conn, dup_pid).await;
        drop(conn);

        let outcome = streaming::merge_and_emit(&ctx.db, main, duplicate, &folded, None).await;
        assert!(
            matches!(outcome, Err(ModelError::EntityNotFound)),
            "a merge whose duplicate is gone must fail closed, got {:?}",
            outcome.map(|(m, _, _)| m.pid)
        );
        let survivor = CaseModel::find_by_pid(&ctx.db, &main_pid.to_string())
            .await
            .unwrap();
        assert_eq!(survivor.data, before, "the survivor must be untouched");
    })
    .await;
}

// CS-M1 (SEC-B5), concurrently: a second connection holds the
// duplicate's row lock, as a concurrent merge would. The merge must wait
// for it, and once the holder retires the duplicate and commits, fail
// closed. Deterministic: the test waits until Postgres reports the merge
// blocked on a lock before it lets the holder commit.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn merge_waits_for_a_concurrent_lock_then_fails_closed() {
    use case_service::models::{ModelError, cases::Model as CaseModel};
    use case_service::streaming;
    use sqlx::Connection as _;

    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let (main, duplicate) = two_cases(&request, &ctx.db).await;
        let (main_pid, dup_pid) = (main.pid, duplicate.pid);
        let before = main.data.clone();
        let folded = main.to_case().unwrap();

        // The holder: its own connection, outside the pool.
        let mut holder = sqlx::PgConnection::connect_with(&ctx.db.connect_options())
            .await
            .unwrap();
        let mut held = holder.begin().await.unwrap();
        sqlx::query("SELECT id FROM cases WHERE pid = $1 FOR UPDATE")
            .bind(dup_pid)
            .fetch_one(&mut *held)
            .await
            .unwrap();

        let merging = tokio::spawn({
            let db = ctx.db.clone();
            async move {
                streaming::merge_and_emit(&db, main, duplicate, &folded, None)
                    .await
                    .map(|(m, _, _)| m.pid)
            }
        });

        // Wait until the merge is blocked on a row lock (or, wrongly,
        // already done). Watched from a third connection outside the
        // pool: the merge holds the pool's connection while it waits.
        let mut watcher = sqlx::PgConnection::connect_with(&ctx.db.connect_options())
            .await
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            let waiting: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM pg_stat_activity \
                 WHERE datname = current_database() AND wait_event_type = 'Lock' \
                   AND query ILIKE '%cases%'",
            )
            .fetch_one(&mut watcher)
            .await
            .unwrap();
            if waiting > 0 || merging.is_finished() {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "the merge neither blocked nor finished"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(!merging.is_finished(), "the merge must wait for the lock");

        retire(&mut held, dup_pid).await;
        held.commit().await.unwrap();

        let outcome = merging.await.unwrap();
        assert!(
            matches!(outcome, Err(ModelError::EntityNotFound)),
            "the merge that lost the race must fail closed, got {outcome:?}"
        );
        let survivor = CaseModel::find_by_pid(&ctx.db, &main_pid.to_string())
            .await
            .unwrap();
        assert_eq!(survivor.data, before, "the survivor must be untouched");
    })
    .await;
}
