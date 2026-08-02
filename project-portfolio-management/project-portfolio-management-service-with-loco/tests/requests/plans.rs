//! Request-level integration tests over the `/api/plans`
//! endpoints (spec §6 / §9), in the loco testing style
//! (`loco_rs::testing`, as in the authentication-service sibling).
//!
//! These boot the app against the `test` environment, which needs a
//! reachable PostgreSQL (`config/test.yaml`; override with
//! `DATABASE_URL`). They are `#[ignore]`d so the default `cargo test`
//! stays green on a database-less machine. Run them with:
//!
//! ```sh
//! DATABASE_URL=postgres://loco:loco@localhost:5432/project_portfolio_management_service_test \
//!   cargo test -- --ignored
//! ```
//!
//! The blank-name → `422` contract is additionally pinned un-gated by the
//! DB-free unit tests in `src/controllers/plans.rs`.

use loco_rs::testing::prelude::*;
use project_portfolio_management_service::app::App;
use serde_json::{Value, json};
use serial_test::serial;

/// A minimal valid project payload (the body *is* `Plan`; all fields
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
// Pins the create happy path: `POST /api/plans` returns 200, echoes
// the name, and mints a UUID pid; the row is then fetchable.
async fn can_create_and_fetch_a_project() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.post("/api/plans").json(&apollo_project()).await;
        assert_eq!(response.status_code(), 200, "create should succeed");
        let body: Value = response.json();
        assert_eq!(body["name"], "Apollo platform migration");
        let pid = body["pid"]
            .as_str()
            .expect("pid in create response")
            .to_string();
        uuid::Uuid::parse_str(&pid).expect("pid should be a UUID");

        let fetched = request.get(&format!("/api/plans/{pid}")).await;
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
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let body = json!({ "kind": "Project", "name": "  " });
        let response = request.post("/api/plans").json(&body).await;
        assert_eq!(response.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
// Pins the unified model: `kind` is an optional label, so any kind (or
// none) is accepted — no collection cross-check.
async fn create_accepts_any_optional_kind() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        // A kind label is optional and free — accepted.
        let with_kind = json!({ "kind": "Product", "name": "Apollo" });
        assert_eq!(
            request
                .post("/api/plans")
                .json(&with_kind)
                .await
                .status_code(),
            200
        );
        // No kind at all is also accepted.
        let no_kind = json!({ "name": "Zephyr" });
        assert_eq!(
            request
                .post("/api/plans")
                .json(&no_kind)
                .await
                .status_code(),
            200
        );
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
// Pins the containment cycle guard: pointing a plan's `parent_ref` at
// itself is rejected `422`.
async fn self_parent_is_rejected() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/plans")
            .json(&json!({ "name": "Root" }))
            .await;
        assert_eq!(created.status_code(), 200);
        let pid = created.json::<Value>()["pid"].as_str().unwrap().to_string();
        // Update it to be its own parent → cycle → 422.
        let body = json!({ "name": "Root", "parent_ref": pid });
        let response = request.put(&format!("/api/plans/{pid}")).json(&body).await;
        assert_eq!(response.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
// Pins the unknown-pid contract on GET.
async fn unknown_pid_is_404() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let pid = uuid::Uuid::new_v4();
        let response = request.get(&format!("/api/plans/{pid}")).await;
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
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request.post("/api/plans").json(&apollo_project()).await;
        assert_eq!(created.status_code(), 200);

        let query = json!({
            "kind": "Project",
            "name": "Apollo migration",
            "identifiers": [{ "scheme": "JiraProjectKey", "value": "apollo" }]
        });
        let dup = request
            .post("/api/plans/check-duplicates")
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
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/plans/whoami").await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
// Pins pagination on the plan collection reads: `limit`/`offset` window
// the rows, `X-Total-Count` reports the whole match set, the limit
// clamps, and an out-of-bound offset is a 400. The parent scope applies
// to the count as well as the page, so a child listing's total describes
// the children rather than every plan.
async fn list_and_search_are_paginated() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        for i in 0..5 {
            request
                .post("/api/plans")
                .json(&json!({"name": format!("Paging Plan {i}")}))
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

        let page = request.get("/api/plans?limit=2&offset=1").await;
        assert_eq!(page.status_code(), 200);
        assert_eq!(page.json::<Value>().as_array().expect("array").len(), 2);
        assert_eq!(header!(page, "x-total-count"), "5");
        assert_eq!(header!(page, "x-limit"), "2");
        assert_eq!(header!(page, "x-offset"), "1");

        let all = request.get("/api/plans").await;
        assert_eq!(all.json::<Value>().as_array().expect("array").len(), 5);
        assert_eq!(header!(all, "x-limit"), "100", "the default is the old cap");

        let clamped = request.get("/api/plans?limit=100000").await;
        assert_eq!(header!(clamped, "x-limit"), "500");

        assert_eq!(
            request.get("/api/plans?offset=10001").await.status_code(),
            400
        );

        let hits = request.get("/api/plans/search?q=Paging&limit=2").await;
        assert_eq!(hits.status_code(), 200, "search page: {}", hits.text());
        assert_eq!(hits.json::<Value>().as_array().expect("array").len(), 2);
        assert_eq!(header!(hits, "x-total-count"), "5");
    })
    .await;
}

/// Tantivy search reaches the fields an `ILIKE` over `name` never could
/// — the goal a plan is trying to achieve, the owner org, an
/// identifier — tolerates a typo when asked to, and a `?kind=` filter
/// narrows without being required.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
async fn search_reaches_secondary_fields_tolerates_typos_and_filters_by_kind() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request.post("/api/plans").json(&apollo_project()).await;
        assert_eq!(created.status_code(), 200);

        let hits = |body: Value| body.as_array().map(Vec::len).unwrap_or_default();

        // The defining attribute of a plan is what it is trying to
        // achieve, and it is now searchable, alongside the identifier.
        for q in ["Cut%20p95%20latency", "APOLLO"] {
            let response = request.get(&format!("/api/plans/search?q={q}")).await;
            assert_eq!(response.status_code(), 200);
            assert_eq!(hits(response.json()), 1, "query {q}");
        }

        // A typo misses on exact retrieval and lands on fuzzy.
        assert_eq!(
            hits(request.get("/api/plans/search?q=Apoll").await.json()),
            0
        );
        assert_eq!(
            hits(
                request
                    .get("/api/plans/search?q=Apoll&fuzzy=true")
                    .await
                    .json()
            ),
            1,
            "fuzzy must tolerate a dropped letter"
        );

        // `?kind=project` matches (case-insensitively); `?kind=program`
        // finds nothing — an opt-in narrowing, not a gate.
        assert_eq!(
            hits(
                request
                    .get("/api/plans/search?q=Apollo&kind=Project")
                    .await
                    .json()
            ),
            1
        );
        assert_eq!(
            hits(
                request
                    .get("/api/plans/search?q=Apollo&kind=program")
                    .await
                    .json()
            ),
            0
        );

        // A soft-deleted plan leaves the index.
        let pid = created.json::<Value>()["pid"].as_str().unwrap().to_string();
        request.delete(&format!("/api/plans/{pid}")).await;
        assert_eq!(
            hits(request.get("/api/plans/search?q=Apollo").await.json()),
            0,
            "a deleted plan must stop being a hit"
        );
    })
    .await;
}

/// The search-blocked candidate detector reaches a plan whose name is
/// wholly different but whose identifier matches, and does not care that
/// the query and the stored plan carry different `kind` labels — the
/// matcher is kind-agnostic, so blocking must not silently reintroduce
/// the boundary the data model removed.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
async fn check_duplicates_blocks_on_identifier_alone_and_ignores_kind() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let created = request
            .post("/api/plans")
            .json(&json!({
                "kind": "Program",
                "name": "Wholly Unrelated Plan",
                "identifiers": [{ "scheme": "JiraProjectKey", "value": "APOLLO" }]
            }))
            .await;
        assert_eq!(created.status_code(), 200);

        let response = request
            .post("/api/plans/check-duplicates")
            .json(&json!({
                "kind": "Project",
                "name": "Apollo platform migration",
                "identifiers": [{ "scheme": "JiraProjectKey", "value": "APOLLO" }]
            }))
            .await;
        assert_eq!(response.status_code(), 200);
        let hits: Value = response.json();
        let created_body: Value = created.json();
        let pid = created_body["pid"].as_str().unwrap();
        assert!(
            hits.as_array().unwrap().iter().any(|h| h["pid"] == pid),
            "the identifier-only, differently-kinded match must be found via blocking: {hits}"
        );
    })
    .await;
}

/// The always-masked view drops `lead_ref` and redacts the owner org
/// fields regardless of caller (enforcement is off in this suite, so
/// no ABAC decision is in play); the descriptive shell — name, code —
/// is untouched. The export envelope wraps the same content and
/// declares `masked: false` when enforcement is off, since there is no
/// obligation to honour.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
async fn masked_view_and_export_are_served() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let mut project = apollo_project();
        project["owner_org_id"] = json!("organization:9a2f");
        project["lead_ref"] = json!("person:0c4f1e2a-0000-4000-8000-000000000000");
        let created: Value = request.post("/api/plans").json(&project).await.json();
        let pid = created["pid"].as_str().unwrap().to_string();

        let masked: Value = request
            .get(&format!("/api/plans/{pid}/masked"))
            .await
            .json();
        assert_eq!(masked["name"], "Apollo platform migration");
        assert_eq!(masked["code"], "PROJ-2026");
        assert!(masked["lead_ref"].is_null());
        assert_ne!(masked["owner_org_id"], "organization:9a2f");

        let export: Value = request
            .get(&format!("/api/plans/{pid}/export"))
            .await
            .json();
        assert_eq!(export["entity"], "plan");
        assert_eq!(export["pid"], pid);
        assert_eq!(export["masked"], false, "no ABAC decision, no obligation");
        assert_eq!(export["record"]["owner_org_id"], "organization:9a2f");
    })
    .await;
}

/// `GET /{pid}/masked` and `/export` are `404` for an unknown pid, same
/// as the ordinary `GET /{pid}`.
#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL"]
async fn masked_view_and_export_are_404_for_unknown_pid() {
    super::isolate_search_index();
    request::<App, _, _>(|request, _ctx| async move {
        let pid = "00000000-0000-4000-8000-000000000000";
        assert_eq!(
            request
                .get(&format!("/api/plans/{pid}/masked"))
                .await
                .status_code(),
            404
        );
        assert_eq!(
            request
                .get(&format!("/api/plans/{pid}/export"))
                .await
                .status_code(),
            404
        );
    })
    .await;
}
