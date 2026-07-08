//! Request-level integration tests over the `/api/{collection}`
//! endpoints (spec §6 / §9), in the loco testing style
//! (`loco_rs::testing`, as in the authentication-service sibling).
//!
//! These boot the app against the `test` environment, which needs a
//! reachable PostgreSQL (`config/test.yaml`; override with
//! `DATABASE_URL`). They are `#[ignore]`d so the default `cargo test`
//! stays green on a database-less machine. Run them with:
//!
//! ```sh
//! DATABASE_URL=postgres://loco:loco@localhost:5432/portfolio_service_test \
//!   cargo test -- --ignored
//! ```
//!
//! The blank-name → `422`, kind-mismatch → `422`, and unknown-collection
//! contracts are additionally pinned un-gated by the DB-free unit tests in
//! `src/controllers/work_items.rs`.

use loco_rs::testing::prelude::*;
use portfolio_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

/// A minimal valid project payload (the body *is* `WorkItem`; all fields
/// but `kind` and `name` default).
fn apollo_project() -> Value {
    json!({
        "kind": "Project",
        "name": "Apollo platform migration",
        "code": "PROJ-2026",
        "owner_org_id": "organization:9a2f",
        "goals": [{ "title": "Cut p95 latency" }],
        "keywords": ["infra"],
        "identifiers": [{ "scheme": "JiraProjectKey", "value": "APOLLO" }]
    })
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test -- --ignored`"]
// Pins the create happy path: `POST /api/projects` returns 200, echoes
// the name, and mints a UUID pid; the row is then fetchable.
async fn can_create_and_fetch_a_project() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/api/projects")
            .json(&apollo_project())
            .await;
        assert_eq!(response.status_code(), 200, "create should succeed");
        let body: Value = response.json();
        assert_eq!(body["name"], "Apollo platform migration");
        let pid = body["pid"]
            .as_str()
            .expect("pid in create response")
            .to_string();
        uuid::Uuid::parse_str(&pid).expect("pid should be a UUID");

        let fetched = request.get(&format!("/api/projects/{pid}")).await;
        assert_eq!(fetched.status_code(), 200);
        assert_eq!(fetched.json::<Value>()["name"], "Apollo platform migration");
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
// Pins the validation contract: a blank name is `422`.
async fn create_rejects_blank_name() {
    request::<App, _, _>(|request, _ctx| async move {
        let body = json!({ "kind": "Project", "name": "  " });
        let response = request.post("/api/projects").json(&body).await;
        assert_eq!(response.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
// Pins the kind cross-check: a Product body posted to /projects is `422`.
async fn create_rejects_mismatched_kind() {
    request::<App, _, _>(|request, _ctx| async move {
        let body = json!({ "kind": "Product", "name": "Apollo" });
        let response = request.post("/api/projects").json(&body).await;
        assert_eq!(response.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
// Pins the unknown-collection contract: an unknown segment is `404`.
async fn unknown_collection_is_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/widgets").await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
// Pins the unknown-pid contract on GET.
async fn unknown_pid_is_404() {
    request::<App, _, _>(|request, _ctx| async move {
        let pid = uuid::Uuid::new_v4();
        let response = request.get(&format!("/api/projects/{pid}")).await;
        assert_eq!(response.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
// Pins within-collection duplicate detection: a stored project with a
// shared Jira key is returned by check-duplicates (score 1.0).
async fn check_duplicates_finds_a_deterministic_twin() {
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/projects")
            .json(&apollo_project())
            .await;
        assert_eq!(created.status_code(), 200);

        let query = json!({
            "kind": "Project",
            "name": "Apollo migration",
            "identifiers": [{ "scheme": "JiraProjectKey", "value": "apollo" }]
        });
        let dup = request
            .post("/api/projects/check-duplicates")
            .json(&query)
            .await;
        assert_eq!(dup.status_code(), 200);
        let hits: Value = dup.json();
        assert!(
            hits.as_array().is_some_and(|a| !a.is_empty()),
            "expected a hit"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
// Pins that `/whoami` is `401` without a bearer token.
async fn whoami_requires_a_token() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/projects/whoami").await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}
