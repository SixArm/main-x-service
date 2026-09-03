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
use std::sync::OnceLock;

use crate::{
    api::rest::{ApiDoc, AppState, metrics_routes, things_routes},
    config::Config,
    matching::ProbabilisticMatcher,
    observability::{self, Telemetry, TelemetryConfig},
    search::SearchEngine,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// The installed telemetry providers, so `on_shutdown` can flush what the
/// batch processor is still holding. Set once by [`App::init_logger`].
static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

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

    /// Install this crate's own logging + `OpenTelemetry` stack (repo
    /// `tasks.md` PRO-H12), returning `true` so loco does **not** also
    /// install its own. Ported from person-service's `App::init_logger`
    /// (itself ported from link-graph-service's) — the `Hooks::init_logger`
    /// seam is identical across crates' shapes despite the differing router
    /// layout (see `src/observability.rs`'s module docs and `after_routes`
    /// below for where this crate's shape actually diverges).
    ///
    /// # Errors
    ///
    /// Returns an error only if an OTLP exporter cannot be **built** (a
    /// malformed endpoint). A collector that is merely unreachable is not an
    /// error: the gRPC channel connects lazily, so boot is unaffected.
    fn init_logger(ctx: &AppContext) -> Result<bool> {
        let telemetry_config = TelemetryConfig::from_env();
        if !telemetry_config.export_enabled() {
            return Ok(false);
        }
        let logger_config = &ctx.config.logger;
        // An operator who set `RUST_LOG` or `logger.override_filter` owns
        // the filter outright; otherwise loco's module whitelist applies,
        // and the whitelist has no `opentelemetry*` entry — so a failing
        // export would be silent unless widened.
        let operator_supplied =
            std::env::var("RUST_LOG").is_ok() || logger_config.override_filter.is_some();
        let env_filter = observability::with_exporter_diagnostics(
            loco_rs::logger::init_env_filter::<Self>(
                logger_config.override_filter.as_ref(),
                &logger_config.level,
            ),
            operator_supplied,
        );
        // `logger.enable: false` means "no stdout layer" — honour it rather
        // than quietly turning logging on for anyone who enables export.
        let fmt_layer = logger_config
            .enable
            .then(|| loco_rs::logger::init_layer(std::io::stdout, &logger_config.format, true));
        let telemetry = observability::init(&telemetry_config, env_filter, fmt_layer)
            .map_err(|error| loco_rs::Error::Message(format!("OTLP init failed: {error}")))?;
        tracing::info!(
            service.name = %telemetry_config.service_name,
            endpoint = %telemetry_config.endpoint.as_deref().unwrap_or_default(),
            "OpenTelemetry OTLP export enabled"
        );
        let _ = TELEMETRY.set(telemetry);
        Ok(true)
    }

    /// Flush and tear down the OTLP providers on graceful shutdown, so the
    /// last batch of spans is not lost with the process.
    async fn on_shutdown(_ctx: &AppContext) {
        if let Some(telemetry) = TELEMETRY.get() {
            telemetry.shutdown();
        }
    }

    /// Register the loco route table: the framework defaults plus the
    /// hand-written `things_routes()` group and the root-level
    /// `metrics_routes()` Prometheus scrape route.
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(things_routes())
            .add_route(crate::controllers::fhir::routes())
            .add_route(metrics_routes())
    }

    /// Post-routing hook where the hand-written Axum surface is attached.
    ///
    /// Builds the boot-time singletons (`Config`, `SearchEngine`,
    /// `ProbabilisticMatcher`), stuffs the assembled [`AppState`] into loco's shared
    /// store so the legacy handlers can extract it, then mounts Swagger UI,
    /// the blanket auth-enforcement middleware
    /// (`auth::require_auth_mw` — a near-noop unless `THING_REQUIRE_AUTH`
    /// was truthy at construction; layered inside CORS so preflight
    /// requests still pass), and a permissive CORS layer onto the router.
    ///
    /// The PASETO verifier is finalised **first** via
    /// [`crate::api::rest::state::boot_verifier`] — when
    /// `THING_PASETO_KEYS_URL` is set the key set is fetched over HTTP
    /// once, here, in async context (fetch failure falls back to the
    /// `THING_PASETO_KEYS` env path; the service always boots) —
    /// **before** the shared-store insert and the middleware capture
    /// the state, so both router surfaces consult the fetched key set.
    ///
    /// # Errors
    ///
    /// Returns a loco error if config load or search-index open fails.
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let config = Config::from_env().map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let matcher = ProbabilisticMatcher::new(&config.matching);
        crate::api::rest::auth::init().await;
        // Key rotation and policy edits without a restart: both loops are
        // no-ops unless their source is configured (`THING_PASETO_KEYS_URL`
        // / `THING_ABAC_POLICY_FILE`).
        crate::api::rest::auth::spawn_key_refresh();
        crate::api::rest::auth::spawn_policy_watcher();
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config);
        ctx.shared_store.insert(state.clone());
        // Durable event bus Phase 3: start the outbox relay loop (a no-op
        // unless the `outbox` transport and `THING_EVENT_RELAY` are set).
        crate::relay::spawn(ctx.db.clone());
        let router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .layer(axum::middleware::from_fn_with_state(
                state,
                crate::api::rest::auth::require_auth_mw,
            ))
            .layer(axum::middleware::from_fn(
                crate::api::rest::version::require_version_mw,
            ))
            .layer(tower_http::cors::CorsLayer::permissive())
            // Outermost layer runs first, so the request span wraps CORS,
            // versioning, and the auth guard too — a 401/403 is part of the
            // trace, not invisible to it (same reasoning as person-service's
            // and link-graph-service's `after_routes`).
            .layer(axum::middleware::from_fn(observability::trace_mw));
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
