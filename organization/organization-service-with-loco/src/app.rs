//! Loco application hooks for the organization service.
//!
//! The `App` type implements loco's `Hooks`, the seam the CLI drives to boot the
//! service, register routes (`routes`), layer middleware (`after_routes`),
//! and truncate tables between tests (`truncate`). It carries no state.
//! The blanket auth enforcement layer (PASETO authentication + ABAC
//! authorization) is wired here via `require_auth_mw`.

use async_trait::async_trait;
use axum::{
    Router as AxumRouter,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use loco_rs::{
    Result,
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{BootResult, StartMode, create_app},
    config::Config,
    controller::AppRoutes,
    db::truncate_table,
    environment::Environment,
    task::Tasks,
};
use migration::Migrator;
use std::path::Path;

use crate::{
    auth, controllers,
    models::_entities::{audit_logs, event_outbox, merge_records, organizations},
};

/// Blanket auth-enforcement middleware: authentication (PASETO bearer)
/// then ABAC authorization. Reads the flag, verifier, and policy per
/// request (all cached `OnceLock`s), so the layer is wired
/// unconditionally and is a near-noop when `ORGANIZATION_REQUIRE_AUTH`
/// is off. See [`auth::enforce`] for the pure decision.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let decision = auth::enforce(
        auth::require_auth(),
        req.method(),
        req.uri().path(),
        req.headers(),
        auth::verifier(),
        auth::policy(),
    );
    match decision {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}

/// The loco application type. Carries no state; it exists to implement
/// [`Hooks`], which loco's CLI drives to boot the service, register
/// routes, and run migrations.
pub struct App;
#[async_trait]
impl Hooks for App {
    /// The app name loco uses in logs and the CLI banner — the crate name.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    /// Human-readable version: the crate version plus the build SHA when
    /// available (`BUILD_SHA`/`GITHUB_SHA`), else `dev` for local builds.
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    /// Boot the app: delegate to loco's `create_app`, wiring this `App`'s
    /// hooks and the migration `Migrator` for the given start mode and
    /// environment.
    ///
    /// # Errors
    ///
    /// Propagates loco boot failures (config load, DB connect, migration).
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// App initializers run at boot. None are needed here; auth and the
    /// event publisher are process-wide `OnceLock`s, not loco state.
    ///
    /// # Errors
    ///
    /// Never (returns `Ok`); the signature is loco's.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    /// Register the controller route trees on top of loco's defaults
    /// (`/_health`, `/_ping`): the organization CRUD/match/audit routes,
    /// the OpenAPI/Swagger docs routes, and the Prometheus metrics route
    /// (`/metrics.prom`, mounted at root like the docs).
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::organizations::routes())
            .add_route(controllers::compliance::routes())
            .add_route(controllers::fhir::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }
    /// Seed the process-wide token verifier (fetching the PASETO key set
    /// over HTTP once when `ORGANIZATION_PASETO_KEYS_URL` is set; env /
    /// reject-all fallback otherwise — the boot never fails on it), then
    /// wrap the assembled router in the blanket auth-enforcement layer.
    ///
    /// # Errors
    ///
    /// Never (returns `Ok`); the signature is loco's.
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Seed the verifier before serving so `enforce()`/`AuthUser`
        // consult the boot-fetched key set from the first request on.
        auth::init_from_env().await;
        // Durable event bus Phase 3: start the outbox relay loop. A no-op
        // unless `ORGANIZATION_EVENT_TRANSPORT=outbox` AND
        // `ORGANIZATION_EVENT_RELAY` are set, so the default `memory`
        // transport never spawns it.
        crate::relay::spawn(ctx.db.clone());
        // Blanket auth enforcement, gated by `ORGANIZATION_REQUIRE_AUTH`
        // (off by default). The layer is added unconditionally; the flag
        // is read per request inside the middleware. Header-based API
        // versioning (`Accepts-version`) is layered alongside it.
        Ok(router
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn(
                crate::version::require_version_mw,
            )))
    }

    /// Register background-queue workers. None are registered.
    ///
    /// # Errors
    ///
    /// Never (returns `Ok`); the signature is loco's.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        // No background workers (the loco scaffold's DownloadWorker stub
        // was removed; see entity spec §13 T-12).
        Ok(())
    }

    /// Register CLI tasks. None are registered; the inject marker is kept
    /// so the loco generator can splice tasks in later.
    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }
    /// Truncate all tables between tests (request-suite setup).
    ///
    /// Order matters only for FK safety; these tables have no FKs, but
    /// children-before-parent order is kept by convention: merge/audit
    /// rows, then the organizations they reference.
    ///
    /// # Errors
    ///
    /// Propagates any truncate failure.
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, event_outbox::Entity).await?;
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, organizations::Entity).await?;
        Ok(())
    }
    /// Seed the database. No seed data for this service.
    ///
    /// # Errors
    ///
    /// Never (returns `Ok`); the signature is loco's.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
