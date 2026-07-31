//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, the blanket auth guard, the API-version
//! middleware, and the truncate lifecycle for
//! `content-management-system-service`.

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

/// Blanket auth-enforcement middleware: reads `CMS_REQUIRE_AUTH` per
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
/// `content-management-system-service`.
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
            .add_route(controllers::sites::routes())
            .add_route(controllers::types::routes())
            .add_route(controllers::entries::routes())
            .add_route(controllers::assets::routes())
            .add_route(controllers::workflow::routes())
            .add_route(controllers::localization::routes())
            .add_route(controllers::routing::routes())
            .add_route(controllers::delivery::routes())
            .add_route(controllers::insights::routes())
            .add_route(controllers::preview::routes())
            .add_route(controllers::webhooks::routes())
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
        tasks.register(tasks::schedule_sweep::ScheduleSweep);
        tasks.register(tasks::webhook_dispatch::WebhookDispatch);
        // tasks-inject (do not remove)
    }

    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, EventOutbox).await?;
        truncate_table(&ctx.db, AuditLogs).await?;
        truncate_table(&ctx.db, WebhookDeliveries).await?;
        truncate_table(&ctx.db, Webhooks).await?;
        truncate_table(&ctx.db, PreviewTokens).await?;
        truncate_table(&ctx.db, AudienceRules).await?;
        truncate_table(&ctx.db, Menus).await?;
        truncate_table(&ctx.db, Redirects).await?;
        truncate_table(&ctx.db, Routes).await?;
        truncate_table(&ctx.db, Renditions).await?;
        truncate_table(&ctx.db, Assets).await?;
        truncate_table(&ctx.db, ContentReferences).await?;
        truncate_table(&ctx.db, Revisions).await?;
        truncate_table(&ctx.db, EntryVariants).await?;
        truncate_table(&ctx.db, Entries).await?;
        truncate_table(&ctx.db, ContentTypes).await?;
        truncate_table(&ctx.db, Templates).await?;
        truncate_table(&ctx.db, Sites).await?;
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
