//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes and the truncate lifecycle for `link-graph-service`.
//!
//! Read-only to the world: the graph read controllers, `OpenAPI`/Swagger,
//! and Prometheus metrics are mounted (plus loco's default `/_health` /
//! `/_ping`); `after_routes` adds the blanket read guard and spawns the
//! periodic reconciliation worker when a source is configured. The live
//! **Fluvio bus consumer** is the remaining v1-deferred piece (spec §13).

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

use crate::auth;
use crate::controllers;
use crate::models::_entities::{audit_log, consumer_offsets, edges, entity_presence};
use crate::reconcile;

/// Blanket read-guard middleware (spec §9.4 / T-19). Delegates to the pure
/// [`auth::enforce`]: with `LINK_GRAPH_REQUIRE_AUTH` off, or on a public
/// path, it passes through; otherwise a valid bearer token is required
/// (`401`) and the token's `attrs` must satisfy the ABAC policy for a
/// `read` on the aggregator (`403`). The per-record `case↔person`
/// concealment (§10) stacks on top of this in the handlers.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    match auth::enforce(
        auth::require_auth(),
        &path,
        req.headers(),
        auth::verifier(),
        auth::policy(),
    ) {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}

/// The loco.rs application hooks for `link-graph-service`.
pub struct App;

#[async_trait]
impl Hooks for App {
    /// The loco application name, taken from the crate name at compile time.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    /// Human-readable version string: the crate `version` plus the build
    /// commit SHA (`BUILD_SHA`/`GITHUB_SHA` at compile time, else `dev`).
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    /// Boot the application: delegates to loco's [`create_app`] with this
    /// crate's [`App`] hooks and the database [`Migrator`].
    ///
    /// # Errors
    ///
    /// Propagates any loco boot error (config, DB connection, …).
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// Register boot-time initializers — none for the v1 core.
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    /// Assemble the route table: loco's default routes (`/_health`,
    /// `/_ping`, …) plus the read-only graph controller (spec §9).
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::graph::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    /// Wrap the router in the blanket read-guard layer. The
    /// `LINK_GRAPH_REQUIRE_AUTH` flag is read per request inside
    /// [`require_auth_mw`], so the layer is a no-op until a deployment
    /// activates enforcement.
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Periodic reconciliation (design §8): pull each configured
        // service's authoritative entity_links and repair the read-model.
        // Spawned only when a source is configured
        // (`LINK_GRAPH_RECONCILE_URL_CASE`); a no-op otherwise.
        if let Some(source) = reconcile::HttpAuthoritativeSource::from_env_for("case") {
            tokio::spawn(reconcile::run_periodic(ctx.db.clone(), source));
        }
        Ok(router.layer(axum::middleware::from_fn(require_auth_mw)))
    }

    /// Register background workers — none in the v1 core (the bus
    /// consumer and reconciliation worker are deferred, spec §13).
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    /// Register CLI tasks — none for the v1 core.
    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }

    /// Truncate all read-model tables for the loco test harness. These
    /// tables have no cross-table FKs, so order is unconstrained.
    ///
    /// # Errors
    ///
    /// Propagates DB truncation errors.
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, edges::Entity).await?;
        truncate_table(&ctx.db, entity_presence::Entity).await?;
        truncate_table(&ctx.db, consumer_offsets::Entity).await?;
        truncate_table(&ctx.db, audit_log::Entity).await?;
        Ok(())
    }

    /// Seed reference/fixture data — none (the read-model is derived).
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
