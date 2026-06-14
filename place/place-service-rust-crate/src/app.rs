//! Loco application hooks.
//!
//! The service boots through loco (CLI, `AppContext`, config, migrations,
//! background queue). The existing REST router is built in
//! [`App::after_routes`] and merged onto loco's router, so the
//! hand-written Axum surface keeps working while loco owns the lifecycle.
//! Boot-time singletons (`SearchEngine`, `PlaceMatcher`, domain `Config`)
//! are constructed here and carried in [`AppState`].

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
    api::rest::{ApiDoc, AppState, places_routes},
    config::Config,
    matching::PlaceMatcher,
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
    /// The crate name loco uses in logs and CLI banners; taken from the
    /// `CARGO_CRATE_NAME` build-time env var so it never drifts from `Cargo.toml`.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    /// Human-readable version string: the crate's semver plus a short build
    /// SHA (`BUILD_SHA`/`GITHUB_SHA` from CI, or `"dev"` for local builds), so
    /// a running instance can be tied back to an exact commit.
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    /// Boot the application for the given start mode (server, worker, all).
    /// Delegates to loco's `create_app`, wiring this `App`'s hooks to the
    /// migration crate's `Migrator` so migrations are available through the CLI.
    ///
    /// # Errors
    ///
    /// Propagates any loco boot error (config load, DB connect, migration
    /// check, …).
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: LocoConfig,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// Register the application's controller routes: loco's default routes
    /// plus this service's `places_routes()` (the `/api` surface).
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes().add_route(places_routes())
    }

    /// Post-routing hook: construct boot-time singletons and merge the
    /// hand-written Axum surface onto loco's router.
    ///
    /// Builds the domain `Config` from the environment, opens the Tantivy
    /// `SearchEngine`, constructs the `PlaceMatcher`, and stuffs the resulting
    /// `AppState` into the context's shared store so `FromRef` handler
    /// extraction can reach it. Then mounts Swagger UI and a permissive CORS
    /// layer.
    ///
    /// # Errors
    ///
    /// Returns a loco error if config loading or search-index creation fails
    /// (mapped from the domain `Error` via its string form).
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let config = Config::from_env().map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let matcher = PlaceMatcher::new(config.matching.clone());
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config);
        ctx.shared_store.insert(state);
        let router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .layer(tower_http::cors::CorsLayer::permissive());
        Ok(router)
    }

    /// Register background-queue workers. No-op: this service runs no
    /// background jobs yet (Postgres-backed queue is configured but unused).
    ///
    /// # Errors
    ///
    /// Never errors in the current implementation; the `Result` is required
    /// by the trait.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    /// Register CLI tasks. No-op: this service ships no custom loco tasks.
    fn register_tasks(_tasks: &mut Tasks) {}

    /// Truncate application tables (used by test harnesses between cases).
    /// No-op here; integration tests manage their own fixtures.
    ///
    /// # Errors
    ///
    /// Never errors in the current implementation.
    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }

    /// Seed the database from fixture files. No-op: this service has no seed
    /// data.
    ///
    /// # Errors
    ///
    /// Never errors in the current implementation.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
