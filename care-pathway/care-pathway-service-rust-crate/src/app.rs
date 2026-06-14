//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, background workers, and the truncate/seed
//! lifecycle for `care-pathway-service`.

use async_trait::async_trait;
use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    Router as AxumRouter,
};
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
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

#[allow(unused_imports)]
use crate::{
    auth, controllers,
    models::_entities::{audit_logs, care_pathways, merge_records},
    tasks,
    workers::downloader::DownloadWorker,
};

/// Blanket `/api/*` JWT-enforcement middleware. Reads the flag and
/// verifier per request and delegates the decision to [`auth::enforce`]:
/// when enforcement is off (the default) or the path is public it is a
/// near-noop; otherwise an absent/invalid bearer token yields `401`.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    match auth::enforce(auth::require_auth(), &path, req.headers(), auth::verifier()) {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}

/// The loco.rs application hooks for `care-pathway-service`.
///
/// Implements [`Hooks`] to register routes, background workers, and the
/// truncate/seed lifecycle; the binary entrypoint drives it via the
/// loco CLI.
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

    /// Register boot-time initializers — none for the MVP.
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    /// Assemble the route table: loco's default routes (`/_health`,
    /// `/_ping`, …) plus the care-pathway controller and the API-docs
    /// controller.
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::care_pathways::routes())
            .add_route(controllers::docs::routes())
    }
    /// Wrap the assembled router in the blanket JWT-enforcement layer.
    ///
    /// Layered unconditionally; [`require_auth_mw`] is a near-noop unless
    /// `CARE_PATHWAY_REQUIRE_AUTH` is on (see [`auth::require_auth`]).
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        // Blanket JWT enforcement, off by default and gated per-request by
        // `CARE_PATHWAY_REQUIRE_AUTH` (see `auth::require_auth`). Wired
        // unconditionally; the flag is the only switch.
        Ok(router.layer(axum::middleware::from_fn(require_auth_mw)))
    }

    /// Register background workers with the queue — currently just the
    /// placeholder [`DownloadWorker`].
    ///
    /// # Errors
    ///
    /// Propagates queue-registration errors.
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }

    /// Register CLI tasks — none for the MVP (the inject marker is kept so
    /// `cargo loco generate task` can splice new tasks in).
    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }
    /// Truncate all tables for the loco test harness, in
    /// foreign-key-safe order: history/audit children first, then the
    /// `care_pathways` parent.
    ///
    /// # Errors
    ///
    /// Propagates DB truncation errors.
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, care_pathways::Entity).await?;
        Ok(())
    }
    /// Seed reference/fixture data — none for the MVP.
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
