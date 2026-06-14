//! Loco application hooks.
//!
//! The service boots through loco (CLI, `AppContext`, config, migrations,
//! background queue). The existing REST router is built in
//! [`App::after_routes`] and merged onto loco's router, so the
//! hand-written Axum surface keeps working while loco owns the lifecycle.
//! Boot-time singletons (`SearchEngine`, `ProbabilisticMatcher`, domain
//! `Config`) are constructed here and carried in [`AppState`].

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
    api::rest::{ApiDoc, AppState, events_routes, metrics_routes},
    config::Config,
    matching::ProbabilisticMatcher,
    search::SearchEngine,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// The loco application. Owns the boot lifecycle, config, migrations,
/// and the background queue; the REST surface is merged in via
/// [`App::after_routes`].
pub struct App;

#[async_trait]
impl Hooks for App {
    /// Crate name reported by loco (banner, logs). Sourced from Cargo.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    /// Human-readable version string: `pkg-version (build-sha)`.
    ///
    /// The SHA is taken from `BUILD_SHA` or `GITHUB_SHA` at compile
    /// time, falling back to `dev` for local builds — so a deployed
    /// binary can be traced back to its commit.
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    /// Boot the app: run migrations via [`Migrator`] and start in the
    /// requested [`StartMode`]. Delegates to loco's `create_app`.
    ///
    /// # Errors
    ///
    /// Returns a loco error if the database connection, migration run,
    /// or server bind fails.
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: LocoConfig,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// Register loco-native routes: the framework defaults plus this
    /// crate's `events` and `metrics` route groups. The hand-written
    /// Axum surface is merged separately in [`Self::after_routes`].
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(events_routes())
            .add_route(metrics_routes())
    }

    /// Final router assembly. Builds the boot-time singletons
    /// (`SearchEngine`, `ProbabilisticMatcher`, domain `Config`), stashes
    /// the resulting [`AppState`] in loco's shared store so handlers can
    /// retrieve it, then mounts Swagger UI and a permissive CORS layer.
    ///
    /// # Errors
    ///
    /// Returns a loco error if the domain config cannot be read from the
    /// environment or the Tantivy search index cannot be opened/created.
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let config = Config::from_env().map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let matcher = ProbabilisticMatcher::new(config.matching.clone());
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config);
        ctx.shared_store.insert(state);
        let router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .layer(tower_http::cors::CorsLayer::permissive());
        Ok(router)
    }

    /// Register background workers on the queue. None yet — CRUD-side
    /// effects run inline — so this is a no-op.
    ///
    /// # Errors
    ///
    /// Infallible today; signature returns `Result` to satisfy the trait.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    /// Register loco CLI tasks. None defined; no-op.
    fn register_tasks(_tasks: &mut Tasks) {}

    /// Truncate tables (used by the test harness). No-op here; the
    /// integration tests manage their own fixtures.
    ///
    /// # Errors
    ///
    /// Infallible today; signature returns `Result` to satisfy the trait.
    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }

    /// Seed the database from fixtures. No seed data; no-op.
    ///
    /// # Errors
    ///
    /// Infallible today; signature returns `Result` to satisfy the trait.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
