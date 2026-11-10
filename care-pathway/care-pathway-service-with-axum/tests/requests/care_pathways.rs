//! Request-level integration tests over the seven `/api/care-pathways`
//! endpoints (spec §6 / §11; entity spec T-4), in the axa testing style
//! (`axa_test`, as in the authentication-service sibling).
//!
//! These boot the app against the `test` environment, which needs a
//! reachable PostgreSQL (`config/test.yaml`; override with
//! `DB_URL`). They are `#[ignore]`d so the default `cargo test`
//! stays green on a database-less machine. Run them with:
//!
//! ```sh
//! DB_URL=postgres://loco:loco@localhost:5432/care_pathway_service_test \
//!   cargo test -- --ignored
//! ```
//!
//! The blank-name → `422` contract is additionally pinned un-gated by
//! the DB-free unit tests in `src/controllers/care_pathways.rs`.

use axa_test::prelude::*;
use care_pathway_service::app::App;
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

/// Pagination: `limit` sizes a page, `Link: rel="next"` carries the
/// signed cursor to the next one until the last, there is no total, and
/// a limit too large or a cursor this service did not sign is a `422`.
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
                    assert!(page.headers().get("x-total-count").is_none(), "no total");
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

        let (sizes, pids) = walk!("/api/care-pathways?limit=2");
        assert_eq!(
            sizes,
            [2, 2, 1],
            "pages of two, and the last says so by having no link"
        );
        assert_eq!(pids.len(), 5, "every row once");

        // Omitting `limit` is the old cap, which holds all five.
        let (sizes, _) = walk!("/api/care-pathways");
        assert_eq!(sizes, [5]);

        // A limit past the maximum is refused, never clamped.
        assert_eq!(
            request
                .get("/api/care-pathways?limit=100000")
                .await
                .status_code(),
            422
        );

        // A cursor is the service's own: a made-up one, or one carried to
        // another query, is refused.
        assert_eq!(
            request
                .get("/api/care-pathways?cursor=AQAAAAAAAAAK")
                .await
                .status_code(),
            422
        );
        let first = request.get("/api/care-pathways?limit=2").await;
        let link = first.headers()["link"].to_str().unwrap().to_string();
        let cursor = &link[link.find("cursor=").unwrap() + 7..link.find('>').unwrap()];
        assert_eq!(
            request
                .get(&format!(
                    "/api/care-pathways/search?q=Paging&limit=2&cursor={cursor}"
                ))
                .await
                .status_code(),
            422,
            "a list cursor is no good for a search"
        );

        // Search pages the same way, through the index.
        let (sizes, pids) = walk!("/api/care-pathways/search?q=Paging&limit=2");
        assert_eq!(sizes, [2, 2, 1]);
        assert_eq!(pids.len(), 5, "all five match the query");
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
        // Evidence rides beside the score (match-evidence.md): a shared
        // guideline id proves the pair, and the evidence names
        // components, never values.
        assert_eq!(
            hits[0]["evidence"]["strength"], "proven",
            "hit: {:?}",
            hits[0]
        );
        assert!(hits[0]["evidence"]["weight_bits"].as_f64().unwrap() >= 30.0);
        let terms = hits[0]["evidence"]["components"]
            .as_array()
            .expect("components");
        assert!(terms.iter().any(|t| t["component"] == "identifier"));
        assert!(!hits[0]["evidence"].to_string().contains("nice-ng128"));
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

/// The thirteen tables `db truncate` must empty (every table the
/// service's migrations create), each with its row-count query.
const SERVICE_TABLES: [(&str, &str); 13] = [
    ("care_pathways", "SELECT count(*) FROM care_pathways"),
    ("audit_logs", "SELECT count(*) FROM audit_logs"),
    ("merge_records", "SELECT count(*) FROM merge_records"),
    ("entity_links", "SELECT count(*) FROM entity_links"),
    ("event_outbox", "SELECT count(*) FROM event_outbox"),
    ("bulk_jobs", "SELECT count(*) FROM bulk_jobs"),
    ("review_queue", "SELECT count(*) FROM review_queue"),
    (
        "pathway_instances",
        "SELECT count(*) FROM pathway_instances",
    ),
    ("instance_steps", "SELECT count(*) FROM instance_steps"),
    ("instance_team", "SELECT count(*) FROM instance_team"),
    ("instance_events", "SELECT count(*) FROM instance_events"),
    (
        "instance_measures",
        "SELECT count(*) FROM instance_measures",
    ),
    (
        "instance_segments",
        "SELECT count(*) FROM instance_segments",
    ),
];

// CP-T1: `db truncate` empties every table the service owns. It used to
// clear only `merge_records`, `audit_logs` and `care_pathways`, so a
// truncated database still held the instances, their steps, team,
// events, measures and segments, the links, unsent outbox events, bulk
// jobs and review pairs, all for pathways it no longer had.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn truncate_clears_every_service_table() {
    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let created = request
            .post("/api/care-pathways")
            .json(&stroke_pathway())
            .await;
        assert_eq!(created.status_code(), 200);
        let pathway =
            uuid::Uuid::parse_str(created.json::<Value>()["pid"].as_str().unwrap()).unwrap();
        let instance = uuid::Uuid::new_v4();

        // One row in every table the API above did not already write.
        // Plain statements rather than the model helpers, so the seed
        // does not depend on each sub-resource's own validation rules.
        let seeds: [&str; 11] = [
            "INSERT INTO pathway_instances (pid, pathway_pid, subject_ref) \
             VALUES ($1, $2, 'person:0c4f1e2a-0000-4000-8000-000000000001')",
            "INSERT INTO instance_steps (pid, instance_pid, label, position) \
             VALUES (gen_random_uuid(), $1, 'Assess', 0)",
            "INSERT INTO instance_team (pid, instance_pid, member_ref, role) \
             VALUES (gen_random_uuid(), $1, 'worker:x', 'lead')",
            "INSERT INTO instance_events (pid, instance_pid, kind) \
             VALUES (gen_random_uuid(), $1, 'note')",
            "INSERT INTO instance_measures (pid, instance_pid, name) \
             VALUES (gen_random_uuid(), $1, 'nihss')",
            "INSERT INTO instance_segments (pid, instance_pid, label, stage, category, started_at) \
             VALUES (gen_random_uuid(), $1, 'Triage', 'arrival', 'value_adding', now())",
            "INSERT INTO entity_links (id, from_pid, kind, to_ref, provenance) \
             VALUES (gen_random_uuid(), $1, 'continues_as', 'case:x', 'operator')",
            "INSERT INTO event_outbox (event_id, entity, entity_pid, kind, occurred_at, \
             schema_version, payload) \
             VALUES (gen_random_uuid(), 'care_pathway', $1, 'created', now(), 1, '{}')",
            "INSERT INTO bulk_jobs (id, kind, entity, format, status, params, created_at, updated_at) \
             VALUES (gen_random_uuid(), 'import', 'care_pathway', 'jsonl', 'queued', '{}', now(), now())",
            "INSERT INTO review_queue (id, record_id_a, record_id_b, match_score, match_quality, \
             detection_method) VALUES (gen_random_uuid(), $1, gen_random_uuid(), 0.9, 'probable', 'test')",
            "INSERT INTO merge_records (main_pid, duplicate_pid) VALUES ($1, gen_random_uuid())",
        ];
        for (i, sql) in seeds.into_iter().enumerate() {
            let q = sqlx::query(sql);
            let q = match i {
                0 => q.bind(instance).bind(pathway),
                1..=5 => q.bind(instance),
                8 => q,
                _ => q.bind(pathway),
            };
            q.execute(&ctx.db).await.unwrap_or_else(|e| panic!("seed {i}: {e}"));
        }
        for (table, count) in SERVICE_TABLES {
            let n: i64 = sqlx::query_scalar(count)
                .fetch_one(&ctx.db)
                .await
                .expect("count rows");
            assert!(n > 0, "the seed must leave a row in {table}");
        }

        App::truncate(&ctx).await.expect("truncate");

        let mut left = Vec::new();
        for (table, count) in SERVICE_TABLES {
            let n: i64 = sqlx::query_scalar(count)
                .fetch_one(&ctx.db)
                .await
                .expect("count rows");
            if n > 0 {
                left.push((table, n));
            }
        }
        assert!(
            left.is_empty(),
            "every service table must be empty after truncate; left: {left:?}"
        );
    })
    .await;
}

/// Create two distinct pathways through the API and load both rows.
async fn two_pathways(
    request: &axa_test::TestServer,
    db: &sqlx::PgPool,
) -> (
    care_pathway_service::models::care_pathways::Model,
    care_pathway_service::models::care_pathways::Model,
) {
    use care_pathway_service::models::care_pathways::Model as PathwayModel;
    let mut pids = Vec::new();
    for name in ["Merge Lock Survivor", "Merge Lock Duplicate"] {
        let created = request
            .post("/api/care-pathways")
            .json(&json!({ "name": format!("{name} {}", uuid::Uuid::new_v4()) }))
            .await;
        assert_eq!(created.status_code(), 200);
        pids.push(created.json::<Value>()["pid"].as_str().unwrap().to_string());
    }
    let main = PathwayModel::find_by_pid(db, &pids[0]).await.unwrap();
    let duplicate = PathwayModel::find_by_pid(db, &pids[1]).await.unwrap();
    (main, duplicate)
}

/// Retire `pid` the way a concurrent merge or delete would, on `conn`.
async fn retire(conn: &mut sqlx::PgConnection, pid: uuid::Uuid) {
    sqlx::query("UPDATE care_pathways SET active = false, deleted_at = now() WHERE pid = $1")
        .bind(pid)
        .execute(conn)
        .await
        .expect("retire the row");
}

// CP-M1 (SEC-B5): the handler reads both rows unlocked, so by the time
// the merge writes, the duplicate may already have been merged away. The
// merge must re-check liveness under its lock and fail closed, rather
// than fold a retired duplicate's data into the survivor.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn merge_refuses_a_duplicate_retired_after_it_was_read() {
    use care_pathway_service::models::{ModelError, care_pathways::Model as PathwayModel};
    use care_pathway_service::streaming;

    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let (main, duplicate) = two_pathways(&request, &ctx.db).await;
        let (main_pid, dup_pid) = (main.pid, duplicate.pid);
        let before = main.data.clone();
        let mut folded = main.to_pathway().unwrap();
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
        let survivor = PathwayModel::find_by_pid(&ctx.db, &main_pid.to_string())
            .await
            .unwrap();
        assert_eq!(survivor.data, before, "the survivor must be untouched");
    })
    .await;
}

// CP-M1 (SEC-B5), concurrently: a second connection holds the
// duplicate's row lock, as a concurrent merge would. The merge must wait
// for it, and once the holder retires the duplicate and commits, fail
// closed. Deterministic: the test waits until Postgres reports the merge
// blocked on a lock before it lets the holder commit.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
async fn merge_waits_for_a_concurrent_lock_then_fails_closed() {
    use care_pathway_service::models::{ModelError, care_pathways::Model as PathwayModel};
    use care_pathway_service::streaming;
    use sqlx::Connection as _;

    super::isolate_search_index();
    request::<App, _, _>(|request, ctx| async move {
        let (main, duplicate) = two_pathways(&request, &ctx.db).await;
        let (main_pid, dup_pid) = (main.pid, duplicate.pid);
        let before = main.data.clone();
        let folded = main.to_pathway().unwrap();

        // The holder: its own connection, outside the pool.
        let mut holder = sqlx::PgConnection::connect_with(&ctx.db.connect_options())
            .await
            .unwrap();
        let mut held = holder.begin().await.unwrap();
        sqlx::query("SELECT id FROM care_pathways WHERE pid = $1 FOR UPDATE")
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
                   AND query ILIKE '%care_pathways%'",
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
        let survivor = PathwayModel::find_by_pid(&ctx.db, &main_pid.to_string())
            .await
            .unwrap();
        assert_eq!(survivor.data, before, "the survivor must be untouched");
    })
    .await;
}
