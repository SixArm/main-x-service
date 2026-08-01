//! Loco application hooks.
//!
//! The service boots through loco (CLI, `AppContext`, config, migrations,
//! background queue). The existing REST router — handlers, `OpenAPI`, CORS
//! — is built in [`App::after_routes`] and merged onto loco's router, so
//! the hand-written Axum surface keeps working while loco owns the
//! lifecycle. The boot-time singletons (`SearchEngine`, `CourseMatcher`,
//! domain `Config`) are constructed here and carried in [`AppState`].

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    Result,
    app::{AppContext, Hooks},
    bgworker::Queue,
    boot::{BootResult, StartMode, create_app},
    config::Config as LocoConfig,
    controller::AppRoutes,
    environment::Environment,
    task::Tasks,
};
use migration::Migrator;
use std::path::Path;

use crate::{
    api::rest::{ApiDoc, AppState, courses_routes, fhir_routes, metrics_routes},
    config::Config,
    matching::CourseMatcher,
    search::SearchEngine,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// The loco application. Owns the boot lifecycle, config, migrations,
/// and the background queue. The REST surface is registered as native
/// loco controllers in [`App::routes`]; `AppState` (search engine,
/// matcher, repositories, config) is built once and placed in the
/// `AppContext` shared store in [`App::after_routes`].
pub struct App;

#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: LocoConfig,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // loco /_health, /_ping
            .add_route(courses_routes())
            .add_route(fhir_routes()) // /fhir/Basic{,/{id}} + /fhir/metadata (non-standard)
            .add_route(metrics_routes()) // GET /metrics.prom (root, public)
    }

    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Build the boot-time singletons (search engine, matcher, repos,
        // domain config) once and place them in the shared store, where
        // the controllers retrieve them via `FromRef<AppContext>`.
        let config = Config::from_env().map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let matcher = CourseMatcher::new(config.matching);
        // Finalise the PASETO verifier first: when `COURSE_PASETO_KEYS_URL`
        // is set the key set is fetched over HTTP once, here, in async
        // context (fetch failure falls back to the `COURSE_PASETO_KEYS` env
        // path; the service always boots) — before the shared-store insert
        // and the middleware capture the state, so both router surfaces
        // consult the fetched key set.
        crate::api::rest::auth::init().await;
        // Key rotation and policy edits without a restart: both loops are
        // no-ops unless their source is configured (`COURSE_PASETO_KEYS_URL`
        // / `COURSE_ABAC_POLICY_FILE`).
        crate::api::rest::auth::spawn_key_refresh();
        crate::api::rest::auth::spawn_policy_watcher();
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config);
        ctx.shared_store.insert(state.clone());
        // Durable event bus (Phase 3): spawn the outbox relay worker. A
        // no-op unless the transport is `outbox` and `COURSE_EVENT_RELAY`
        // is truthy, so the default `memory` transport is unaffected.
        crate::relay::spawn(ctx.db.clone());
        // Swagger UI + blanket auth enforcement + permissive CORS layered
        // on top of the controllers.
        let router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            // Blanket auth enforcement (default-off; a near-noop unless
            // `COURSE_REQUIRE_AUTH` was truthy at construction; layered
            // inside CORS so preflight requests still pass).
            .layer(axum::middleware::from_fn_with_state(
                state,
                crate::api::rest::auth::require_auth_mw,
            ))
            // Header-based API versioning (`Accepts-version`): negotiates
            // the version for `/api/*` and stamps it on the response
            // (`agents/share/api-versioning.md`).
            .layer(axum::middleware::from_fn(
                crate::api::rest::version::require_version_mw,
            ))
            .layer(tower_http::cors::CorsLayer::permissive());
        Ok(router)
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(_tasks: &mut Tasks) {}

    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
