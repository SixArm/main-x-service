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
    api::rest::{ApiDoc, AppState, fhir_routes, metrics_routes, workers_routes},
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
    /// Logical application name reported by loco. Sourced from the Cargo
    /// crate name at compile time.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    /// Human-readable version string: the crate version plus a build SHA.
    /// The SHA comes from `BUILD_SHA`, falling back to `GITHUB_SHA`, then to
    /// `"dev"` for local builds where neither is set.
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    /// Boot hook: hands off to loco's [`create_app`] with this crate's
    /// [`Migrator`], so loco drives mode selection, `AppContext`
    /// construction, config loading, and migration running.
    ///
    /// # Errors
    ///
    /// Returns a loco [`Result`] error if app creation fails (e.g. database
    /// connection or migration failure).
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: LocoConfig,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// Registers the hand-written Axum route groups on top of loco's default
    /// routes: worker CRUD/search/match (`workers_routes`), FHIR R5
    /// (`fhir_routes`), and Prometheus exposition (`metrics_routes`).
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(workers_routes())
            .add_route(fhir_routes())
            .add_route(metrics_routes())
    }

    /// Post-routing hook: builds boot-time singletons, injects shared state,
    /// and layers Swagger UI + permissive CORS onto loco's router.
    ///
    /// Constructs the domain [`Config`], the Tantivy [`SearchEngine`], and
    /// the [`ProbabilisticMatcher`], bundles them with the loco DB handle
    /// into [`AppState`], and stashes that in `ctx.shared_store` so handlers
    /// can retrieve it. Then merges the Swagger UI (served at `/swagger-ui`,
    /// spec at `/api-docs/openapi.json`) and a permissive CORS layer.
    ///
    /// # Errors
    ///
    /// Returns a loco [`Result`] error if configuration loading or
    /// search-engine initialization fails; domain errors are stringified
    /// into [`loco_rs::Error::string`].
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Load domain config (env + defaults); map domain error to loco error.
        let config = Config::from_env().map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        // Open/create the Tantivy index at the configured path.
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        // Build the probabilistic matcher from the matching thresholds.
        let matcher = ProbabilisticMatcher::new(config.matching.clone());
        // Bundle DB handle + singletons into shared application state.
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config);
        // Make the state retrievable by request handlers via the shared store.
        ctx.shared_store.insert(state);
        // Mount Swagger UI and a permissive CORS layer on the router.
        let router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .layer(tower_http::cors::CorsLayer::permissive());
        Ok(router)
    }

    /// Background-worker registration hook. No queue workers are wired up
    /// yet, so this is a no-op that returns `Ok`.
    ///
    /// # Errors
    ///
    /// Returns a loco [`Result`] error if worker registration fails; the
    /// current no-op implementation never errors.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    /// CLI-task registration hook. No custom tasks are registered.
    fn register_tasks(_tasks: &mut Tasks) {}

    /// Test-support hook to truncate tables between test runs. No-op here.
    ///
    /// # Errors
    ///
    /// Returns a loco [`Result`] error if truncation fails; the current
    /// no-op implementation never errors.
    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }

    /// Seed hook to load fixture data from `_base`. No seed data is loaded.
    ///
    /// # Errors
    ///
    /// Returns a loco [`Result`] error if seeding fails; the current no-op
    /// implementation never errors.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
