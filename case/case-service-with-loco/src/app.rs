//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, background workers, and the truncate/seed
//! lifecycle for `case-service`.

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
    models::_entities::{audit_logs, cases, merge_records},
    tasks,
    workers::downloader::DownloadWorker,
};

/// Blanket auth-enforcement middleware. Reads the `CASE_REQUIRE_AUTH`
/// flag per request via [`auth::require_auth`] and delegates the decision
/// to the pure [`auth::enforce`]: public paths and the disabled flag pass
/// through; otherwise a valid bearer token is required (`401`) and the
/// token's `attrs` claim must satisfy the process-wide ABAC policy
/// ([`auth::policy`]) for the action derived from the method + path
/// (`403` with the deciding rule otherwise). Off by default (see
/// `auth.rs` and `agents/share/authorization-attributes.md`).
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    match auth::enforce(
        auth::require_auth(),
        &method,
        &path,
        req.headers(),
        auth::verifier(),
        auth::policy(),
    ) {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}

/// The loco.rs application hooks for `case-service`.
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
            .add_route(controllers::cases::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        // Seed the process-wide PASETO verifier before the app serves
        // traffic: when `CASE_PASETO_KEYS_URL` is set the published key
        // set is fetched over HTTP once at boot (fetch failure falls back
        // to the `CASE_PASETO_KEYS` env path — the service always boots).
        auth::init().await;
        // Blanket auth enforcement layer (authn + ABAC authz). Added
        // unconditionally; the
        // `CASE_REQUIRE_AUTH` flag is read per request and the layer is a
        // near-noop when the flag is off (the default).
        Ok(router.layer(axum::middleware::from_fn(require_auth_mw)))
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
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, cases::Entity).await?;
        Ok(())
    }
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
