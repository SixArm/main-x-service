//! Loco application hooks.
//!
//! The service boots through loco (CLI, `AppContext`, config, migrations,
//! background queue). The existing REST router is built in
//! [`App::after_routes`] and merged onto loco's router, so the
//! hand-written Axum surface keeps working while loco owns the lifecycle.
//! Boot-time singletons (`SearchEngine`, `ThingMatcher`, domain `Config`)
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
    api::rest::{ApiDoc, AppState, metrics_routes, things_routes},
    config::Config,
    matching::ThingMatcher,
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
    /// Service name loco uses in logs and banners; taken from the crate
    /// name at compile time so it never drifts from `Cargo.toml`.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    /// Human-readable version string: package version plus a short build
    /// id. The build id prefers `BUILD_SHA`, falls back to GitHub Actions'
    /// `GITHUB_SHA`, and is `"dev"` for local builds.
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    /// Boot the loco application for the given mode/environment, delegating
    /// to loco's `create_app` with this crate's `Migrator` so migrations run
    /// as part of boot.
    ///
    /// # Errors
    ///
    /// Propagates any loco boot error (config load, DB connect, migration).
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: LocoConfig,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// Register the loco route table: the framework defaults plus the
    /// hand-written `things_routes()` group and the root-level
    /// `metrics_routes()` Prometheus scrape route.
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(things_routes())
            .add_route(metrics_routes())
    }

    /// Post-routing hook where the hand-written Axum surface is attached.
    ///
    /// Builds the boot-time singletons (`Config`, `SearchEngine`,
    /// `ThingMatcher`), stuffs the assembled [`AppState`] into loco's shared
    /// store so the legacy handlers can extract it, then mounts Swagger UI
    /// and a permissive CORS layer onto the router.
    ///
    /// # Errors
    ///
    /// Returns a loco error if config load or search-index open fails.
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let config = Config::from_env().map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let matcher = ThingMatcher::new(config.matching.clone());
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config);
        ctx.shared_store.insert(state);
        let router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .layer(tower_http::cors::CorsLayer::permissive());
        Ok(router)
    }

    /// Worker-registration hook. No background workers are registered yet,
    /// so this is a no-op.
    ///
    /// # Errors
    ///
    /// Never errors in the current implementation.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    /// CLI-task registration hook. No custom tasks are registered yet.
    fn register_tasks(_tasks: &mut Tasks) {}

    /// Test-support hook to wipe tables. Intentionally a no-op here; the
    /// service relies on soft-delete and has no truncate workflow.
    ///
    /// # Errors
    ///
    /// Never errors in the current implementation.
    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }

    /// Seed-data hook. No fixtures are loaded; a fresh database starts empty.
    ///
    /// # Errors
    ///
    /// Never errors in the current implementation.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
