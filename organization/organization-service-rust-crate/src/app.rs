use async_trait::async_trait;
use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    Router as AxumRouter,
};
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::truncate_table,
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;

use crate::{
    auth, controllers,
    models::_entities::{audit_logs, merge_records, organizations},
};

/// Blanket JWT-enforcement middleware. Reads the flag and verifier per
/// request (both are cached `OnceLock`s), so the layer is wired
/// unconditionally and is a near-noop when `ORGANIZATION_REQUIRE_AUTH`
/// is off. See [`auth::enforce`] for the pure decision.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    match auth::enforce(auth::require_auth(), &path, req.headers(), auth::verifier()) {
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
            .add_route(controllers::organizations::routes())
            .add_route(controllers::docs::routes())
    }
    async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        // Blanket JWT enforcement, gated by `ORGANIZATION_REQUIRE_AUTH`
        // (off by default). The layer is added unconditionally; the flag
        // is read per request inside the middleware.
        Ok(router.layer(axum::middleware::from_fn(require_auth_mw)))
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        // No background workers (the loco scaffold's DownloadWorker stub
        // was removed; see entity spec §13 T-12).
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, organizations::Entity).await?;
        Ok(())
    }
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
