//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, background workers, and the truncate/seed
//! lifecycle for `project-portfolio-management-service`.

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
    bgworker::{BackgroundWorker, Queue},
    boot::{BootResult, StartMode, create_app},
    config::Config,
    controller::AppRoutes,
    db::truncate_table,
    environment::Environment,
    task::Tasks,
};
use migration::Migrator;
use std::path::Path;

#[allow(unused_imports)]
use crate::{
    auth, controllers,
    models::_entities::{
        allocations, audit_logs, automation_runs, automations, benefits, budget_lines,
        event_outbox, gate_reviews, ideas, merge_records, milestones, notifications,
        objective_links, objectives, plan_dependencies, plans, proposals, report_definitions,
        reviews, risks, scenarios, scheduled_actions,
    },
    tasks,
    workers::downloader::DownloadWorker,
};

/// Blanket auth-enforcement middleware: PASETO authentication then ABAC
/// authorization. Reads the `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` flag, the verifier,
/// and the ABAC policy per request (all cached `OnceLock`s) and delegates
/// to the pure [`auth::enforce`]: public paths and the disabled flag pass
/// through; otherwise a valid bearer token is required (`401`) and the
/// derived action is checked against the policy (`403`). Off by default
/// (see `auth.rs`).
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

/// The loco.rs application hooks for `project-portfolio-management-service`.
///
/// Implements [`Hooks`] to register routes, background workers, and the
/// truncate/seed lifecycle; the binary entrypoint drives it via the
/// loco CLI.
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
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::plans::routes())
            .add_route(controllers::compliance::routes())
            .add_route(controllers::governance::routes())
            .add_route(controllers::visibility::routes())
            .add_route(controllers::insights::routes())
            .add_route(controllers::oversight::routes())
            .add_route(controllers::engineering::routes())
            .add_route(controllers::strategy::routes())
            .add_route(controllers::collaboration::routes())
            .add_route(controllers::automation::routes())
            .add_route(controllers::prioritisation::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Seed the process-wide PASETO verifier before the app serves
        // traffic: when `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS_URL` is set the published
        // key set is fetched over HTTP once at boot (fetch failure falls
        // back to the `PROJECT_PORTFOLIO_MANAGEMENT_PASETO_KEYS` env path — the service
        // always boots).
        auth::init().await;
        // Durable event bus Phase 3: start the outbox relay loop. A no-op
        // unless `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_TRANSPORT=outbox` AND `PROJECT_PORTFOLIO_MANAGEMENT_EVENT_RELAY`
        // are set, so the default `memory` transport never spawns it.
        crate::relay::spawn(ctx.db.clone());
        // Optional estate-snapshot ticker (env-gated, default off).
        crate::snapshots::spawn(ctx.db.clone());
        // Set-and-forget: the optional scheduled-action sweep ticker
        // (env-gated, default off — the sweep endpoint always works).
        crate::scheduler::spawn(ctx.clone());
        // Blanket JWT enforcement layer. Added unconditionally; the
        // `PROJECT_PORTFOLIO_MANAGEMENT_REQUIRE_AUTH` flag is read per request and the layer is a
        // near-noop when the flag is off (the default).
        Ok(router
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn(
                crate::version::require_version_mw,
            )))
    }
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, notifications::Entity).await?;
        truncate_table(&ctx.db, automation_runs::Entity).await?;
        truncate_table(&ctx.db, automations::Entity).await?;
        truncate_table(&ctx.db, scheduled_actions::Entity).await?;
        truncate_table(&ctx.db, reviews::Entity).await?;
        truncate_table(&ctx.db, benefits::Entity).await?;
        truncate_table(&ctx.db, objective_links::Entity).await?;
        truncate_table(&ctx.db, objectives::Entity).await?;
        truncate_table(&ctx.db, scenarios::Entity).await?;
        truncate_table(&ctx.db, ideas::Entity).await?;
        truncate_table(&ctx.db, report_definitions::Entity).await?;
        truncate_table(&ctx.db, allocations::Entity).await?;
        truncate_table(&ctx.db, milestones::Entity).await?;
        truncate_table(&ctx.db, plan_dependencies::Entity).await?;
        truncate_table(&ctx.db, budget_lines::Entity).await?;
        truncate_table(&ctx.db, risks::Entity).await?;
        truncate_table(&ctx.db, gate_reviews::Entity).await?;
        truncate_table(&ctx.db, proposals::Entity).await?;
        truncate_table(&ctx.db, event_outbox::Entity).await?;
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, plans::Entity).await?;
        Ok(())
    }
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
