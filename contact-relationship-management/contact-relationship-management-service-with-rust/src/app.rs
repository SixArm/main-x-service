//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, the blanket auth guard, the API-version
//! middleware, and the truncate lifecycle for
//! `contact-relationship-management-service`.

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

use crate::{auth, controllers, models::_entities::prelude::*, tasks};

/// Blanket auth-enforcement middleware: reads `CRM_REQUIRE_AUTH` per
/// request and delegates to the pure [`auth::enforce`]. Off by
/// default — see `auth.rs` and `agents/share/security.md` §4.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let policy = auth::policy().current();
    let verifier = auth::verifier().current();
    match auth::enforce(
        auth::require_auth(),
        &method,
        &path,
        req.headers(),
        &verifier,
        &policy,
    ) {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}

/// The loco.rs application hooks for
/// `contact-relationship-management-service`.
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
        AppRoutes::with_default_routes()
            .add_route(controllers::relationships::routes())
            .add_route(controllers::sales::routes())
            .add_route(controllers::marketing::routes())
            .add_route(controllers::support::routes())
            .add_route(controllers::dashboards::routes())
            .add_route(controllers::insights::routes())
            .add_route(controllers::audits::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        auth::init().await;
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        Ok(router
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn(
                crate::version::require_version_mw,
            )))
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::seed::Seed);
        // tasks-inject (do not remove)
    }

    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, EventOutbox).await?;
        truncate_table(&ctx.db, AuditLogs).await?;
        truncate_table(&ctx.db, Articles).await?;
        truncate_table(&ctx.db, Tickets).await?;
        truncate_table(&ctx.db, SlaPolicies).await?;
        truncate_table(&ctx.db, NurtureEnrollments).await?;
        truncate_table(&ctx.db, NurtureSteps).await?;
        truncate_table(&ctx.db, NurtureSequences).await?;
        truncate_table(&ctx.db, Campaigns).await?;
        truncate_table(&ctx.db, Segments).await?;
        truncate_table(&ctx.db, ForecastSnapshots).await?;
        truncate_table(&ctx.db, Deals).await?;
        truncate_table(&ctx.db, PipelineStages).await?;
        truncate_table(&ctx.db, Pipelines).await?;
        truncate_table(&ctx.db, Leads).await?;
        truncate_table(&ctx.db, ConsentEvents).await?;
        truncate_table(&ctx.db, Activities).await?;
        truncate_table(&ctx.db, Contacts).await?;
        truncate_table(&ctx.db, Accounts).await?;
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
