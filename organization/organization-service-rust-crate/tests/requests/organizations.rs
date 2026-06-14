//! Request-level integration tests for the organizations API.
//!
//! These boot the real loco app against the `test` environment config
//! (`config/test.yaml`), so they require a reachable PostgreSQL
//! instance (default `postgres://loco:loco@localhost:5432/organization_service_test`,
//! overridable via `DATABASE_URL`). Following the family convention
//! (see person-service `tests/api_integration_test.rs`) they are
//! `#[ignore]`d so the default `cargo test` run stays green without a
//! database; run them with:
//!
//! ```bash
//! cargo test -- --ignored
//! ```
//!
//! The blank-name → 422 contract is additionally pinned DB-free by a
//! unit test in `src/controllers/organizations.rs`.

use loco_rs::testing::prelude::*;
use organization_service::app::App;
use serde_json::json;
use serial_test::serial;

/// A representative create payload (the body is the
/// `organization_matcher::Organization` shape, snake_case wire format).
fn acme() -> serde_json::Value {
    json!({
        "name": "Acme, Inc.",
        "legal_name": "Acme Incorporated",
        "url": "https://acme.com",
        "same_as": ["https://www.wikidata.org/wiki/Q42"],
        "jurisdiction": "US",
        "founding_date": "1985-04-01",
        "keywords": ["anvils"]
    })
}

/// Create happy path: POST returns a `{pid, name}` ref, and the stored
/// payload round-trips verbatim through `GET /{pid}`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn can_create_and_round_trip() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.post("/api/organizations").json(&acme()).await;
        assert_eq!(response.status_code(), 200, "create should succeed");
        let body: serde_json::Value = response.json();
        let pid = body["pid"].as_str().expect("pid in create response");
        assert_eq!(body["name"], "Acme, Inc.");

        let fetched = request.get(&format!("/api/organizations/{pid}")).await;
        assert_eq!(fetched.status_code(), 200);
        let org: serde_json::Value = fetched.json();
        // Snake_case wire format, round-tripped verbatim (OQ-1).
        assert_eq!(org["name"], "Acme, Inc.");
        assert_eq!(org["legal_name"], "Acme Incorporated");
        assert_eq!(org["same_as"][0], "https://www.wikidata.org/wiki/Q42");
        assert_eq!(org["founding_date"], "1985-04-01");
        assert_eq!(org["jurisdiction"], "US");
    })
    .await;
}

/// Blank `name` is a validation failure: `422 Unprocessable Entity`
/// (family convention; spec §6/§9, entity spec T-2).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn blank_name_returns_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/organizations")
            .json(&json!({"name": " "}))
            .await;
        assert_eq!(response.status_code(), 422, "blank name must be 422");
    })
    .await;
}

/// Updating with a blank `name` is the same validation failure.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn update_with_blank_name_returns_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: serde_json::Value = request
            .post("/api/organizations")
            .json(&acme())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid");

        let response = request
            .put(&format!("/api/organizations/{pid}"))
            .json(&json!({"name": ""}))
            .await;
        assert_eq!(response.status_code(), 422);
    })
    .await;
}

/// Unknown pid → `404 Not Found`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn get_unknown_pid_returns_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .get("/api/organizations/00000000-0000-4000-8000-000000000000")
            .await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}

/// Case-insensitive name search finds the seeded record; blank `q`
/// is a `400`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn can_search_by_name() {
    request::<App, _, _>(|request, _ctx| async move {
        request.post("/api/organizations").json(&acme()).await;
        request
            .post("/api/organizations")
            .json(&json!({"name": "Globex Corporation"}))
            .await;

        let response = request.get("/api/organizations/search?q=acme").await;
        assert_eq!(response.status_code(), 200);
        let hits: serde_json::Value = response.json();
        let hits = hits.as_array().expect("array of refs");
        assert_eq!(hits.len(), 1, "only Acme should match");
        assert_eq!(hits[0]["name"], "Acme, Inc.");

        let blank = request.get("/api/organizations/search?q=%20").await;
        assert_eq!(blank.status_code(), 400, "blank q is a bad request");
    })
    .await;
}

/// check-duplicates scores the query against stored organizations and
/// returns hits above the threshold.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn can_check_duplicates() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: serde_json::Value = request
            .post("/api/organizations")
            .json(&acme())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid");
        request
            .post("/api/organizations")
            .json(&json!({"name": "Completely Unrelated Widgets Ltd"}))
            .await;

        // Same sameAs URL → deterministic short-circuit match.
        let response = request
            .post("/api/organizations/check-duplicates")
            .json(&acme())
            .await;
        assert_eq!(response.status_code(), 200);
        let hits: serde_json::Value = response.json();
        let hits = hits.as_array().expect("array of scored refs");
        assert!(!hits.is_empty(), "the stored Acme must be reported");
        assert_eq!(hits[0]["pid"], pid);
        assert_eq!(hits[0]["is_match"], true);
        assert!(hits[0]["score"].as_f64().expect("score") >= 0.95);
    })
    .await;
}

/// Merge folds a duplicate into a survivor: list fields union, the
/// duplicate's name becomes an alternate name, the duplicate is
/// soft-deleted, and a merge-history row + `merged` event are written.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn merge_folds_duplicate_into_survivor() {
    request::<App, _, _>(|request, _ctx| async move {
        let main: serde_json::Value = request
            .post("/api/organizations")
            .json(&acme())
            .await
            .json();
        let main_pid = main["pid"].as_str().expect("main pid").to_string();

        let dup: serde_json::Value = request
            .post("/api/organizations")
            .json(&json!({
                "name": "Acme Incorporated",
                "keywords": ["hardware"],
                "identifiers": [{"scheme": "Duns", "value": "150483782"}]
            }))
            .await
            .json();
        let dup_pid = dup["pid"].as_str().expect("dup pid").to_string();

        let response = request
            .post("/api/organizations/merge")
            .json(&json!({"main_pid": main_pid, "duplicate_pid": dup_pid, "reason": "confirmed"}))
            .await;
        assert_eq!(response.status_code(), 200, "merge should succeed");
        let merged = response.json::<serde_json::Value>()["main"].clone();
        assert_eq!(merged["name"], "Acme, Inc.");
        let alts = merged["alternate_names"].as_array().expect("alt names");
        assert!(alts.iter().any(|n| n == "Acme Incorporated"));
        assert!(
            merged["keywords"]
                .as_array()
                .unwrap()
                .iter()
                .any(|k| k == "hardware")
        );
        assert_eq!(merged["identifiers"].as_array().unwrap().len(), 1);

        // Duplicate is soft-deleted.
        let gone = request.get(&format!("/api/organizations/{dup_pid}")).await;
        assert_eq!(gone.status_code(), 404, "duplicate should be soft-deleted");

        // Merge-history row exists.
        let merges: serde_json::Value =
            request.get("/api/organizations/merges/recent").await.json();
        assert!(
            merges
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["duplicate_pid"].as_str() == Some(dup_pid.as_str()))
        );

        // Merged event published for the survivor.
        let events: serde_json::Value =
            request.get("/api/organizations/events/recent").await.json();
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

/// Self-merge (equal pids) is rejected with `422`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn merge_with_equal_pids_is_422() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: serde_json::Value = request
            .post("/api/organizations")
            .json(&acme())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();
        let response = request
            .post("/api/organizations/merge")
            .json(&json!({"main_pid": pid, "duplicate_pid": pid}))
            .await;
        assert_eq!(response.status_code(), 422, "self-merge must be rejected");
    })
    .await;
}

/// The JWT-protected `whoami` rejects a request without a bearer token.
/// The token-accepted path is pinned un-gated by `auth::tests`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn whoami_without_token_is_401() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/organizations/whoami").await;
        assert_eq!(response.status_code(), 401, "whoami needs a bearer token");
    })
    .await;
}

/// With blanket enforcement on (`ORGANIZATION_REQUIRE_AUTH=1`), an
/// un-authenticated `GET /api/organizations` is `401`, while the public
/// OpenAPI doc still returns `200`. The flag is set inside the test and
/// removed afterwards; `#[serial]` avoids env-var races with the other
/// request tests (which run with the flag unset / off).
///
/// Note: `require_auth()` caches the flag in a `OnceLock` on first read,
/// so this test sets the env var *before* the app boots; if another test
/// in this process already triggered enforcement to cache as `false`,
/// this assertion would not hold — hence `#[serial]` and a dedicated
/// scope. In CI this suite runs with the flag wired (see family contract).
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn require_auth_gate_blocks_unauthed_list_but_allows_openapi() {
    // SAFETY: single-threaded within this #[serial] test.
    unsafe {
        std::env::set_var("ORGANIZATION_REQUIRE_AUTH", "1");
    }
    request::<App, _, _>(|request, _ctx| async move {
        let protected = request.get("/api/organizations").await;
        assert_eq!(
            protected.status_code(),
            401,
            "un-authed list must be 401 when enforcement is on"
        );

        let openapi = request.get("/api-docs/openapi.json").await;
        assert_eq!(
            openapi.status_code(),
            200,
            "public OpenAPI doc must stay reachable"
        );
    })
    .await;
    unsafe {
        std::env::remove_var("ORGANIZATION_REQUIRE_AUTH");
    }
}

/// Merging an unknown duplicate is a `404`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with: cargo test -- --ignored"]
async fn merge_unknown_pid_is_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let created: serde_json::Value = request
            .post("/api/organizations")
            .json(&acme())
            .await
            .json();
        let pid = created["pid"].as_str().expect("pid").to_string();
        let response = request
            .post("/api/organizations/merge")
            .json(&json!({
                "main_pid": pid,
                "duplicate_pid": "00000000-0000-4000-8000-000000000000"
            }))
            .await;
        assert_eq!(response.status_code(), 404, "unknown duplicate is 404");
    })
    .await;
}
