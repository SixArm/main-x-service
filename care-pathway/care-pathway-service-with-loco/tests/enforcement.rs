//! Blanket-enforcement pin (spec §13 T-7; family contract
//! `agents/share/jwt-enforcement.md`): with `CARE_PATHWAY_REQUIRE_AUTH`
//! on and no token, a protected `/api/*` route is `401` while the
//! public OpenAPI doc still serves `200`.
//!
//! Its **own test binary** (the case / patient-flow pattern): the flag
//! is cached in a process-wide `OnceLock` on first boot, so it must be
//! set before the one and only boot in this process — inside the
//! shared request binary it was order-dependent (QA-CP-FLAKE, fixed
//! 2026-07-18).
//!
//! `#[ignore]`d: boots the app (needs PostgreSQL via
//! `config/test.yaml` / `DATABASE_URL`). Run with
//! `cargo test --test enforcement -- --ignored`.

use care_pathway_service::app::App;
use loco_rs::testing::prelude::*;
use serial_test::serial;

#[tokio::test]
#[serial]
#[ignore = "requires PostgreSQL (config/test.yaml); run with `cargo test --test enforcement -- --ignored`"]
async fn require_auth_gates_api_but_not_openapi() {
    // SAFETY: single-threaded test setup; this binary's only test.
    unsafe {
        std::env::set_var("CARE_PATHWAY_REQUIRE_AUTH", "1");
    }
    request::<App, _, _>(|request, _ctx| async move {
        let protected = request.get("/api/care-pathways").await;
        assert_eq!(
            protected.status_code(),
            401,
            "un-authed /api/care-pathways must be 401 when enforcement is on"
        );
        let openapi = request.get("/api-docs/openapi.json").await;
        assert_eq!(
            openapi.status_code(),
            200,
            "public openapi.json stays 200 even when enforcement is on"
        );
    })
    .await;
}
