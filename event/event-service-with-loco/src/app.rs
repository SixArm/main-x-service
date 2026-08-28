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
    api::rest::{ApiDoc, AppState, events_routes, metrics_routes},
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

    /// Install this crate's own logging + `OpenTelemetry` stack (repo
    /// `tasks.md` PRO-H9), returning `true` so loco does **not** also
    /// install its own. Ported verbatim from person-service's
    /// `App::init_logger` (itself from link-graph-service) — the
    /// `Hooks::init_logger` seam is identical across all three crates'
    /// shapes despite the differing router layout (see
    /// `src/observability.rs`'s module docs and `after_routes` below for
    /// where this crate's shape genuinely diverges).
    ///
    /// The stack is loco's — its [`EnvFilter`](loco_rs::logger::init_env_filter)
    /// policy and its formatted layer, so `RUST_LOG`, `logger.level`,
    /// `logger.format` and `logger.override_filter` keep behaving exactly as
    /// they did — **plus** the `tracing-opentelemetry` bridge over an
    /// OTLP/gRPC exporter.
    ///
    /// When OTLP export is off (`OTLP_ENDPOINT=""`) this returns `false` and
    /// loco's untouched `logger::init` runs, so the disabled path is not a
    /// re-implementation that could drift from it.
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

    /// Register loco-native routes: the framework defaults plus this
    /// crate's `events`, FHIR `Appointment`, and `metrics` route groups.
    /// The hand-written Axum surface is merged separately in
    /// [`Self::after_routes`].
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(events_routes())
            .add_route(crate::controllers::fhir::routes())
            .add_route(metrics_routes())
    }

    /// Final router assembly. Builds the boot-time singletons
    /// (`SearchEngine`, `ProbabilisticMatcher`, domain `Config`), stashes
    /// the resulting [`AppState`] in loco's shared store so handlers can
    /// retrieve it, then mounts Swagger UI, the blanket auth-enforcement
    /// middleware (`auth::require_auth_mw` — a near-noop unless
    /// `EVENT_REQUIRE_AUTH` was truthy at construction; layered inside
    /// CORS so preflight requests still pass), and a permissive CORS
    /// layer.
    ///
    /// The PASETO verifier is finalised **first** via
    /// [`crate::api::rest::state::boot_verifier`] — when
    /// `EVENT_PASETO_KEYS_URL` is set the key set is fetched over HTTP
    /// once, here, in async context (fetch failure falls back to the
    /// `EVENT_PASETO_KEYS` env path; the service always boots) —
    /// **before** the shared-store insert and the middleware capture
    /// the state, so both router surfaces consult the fetched key set.
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
        crate::api::rest::auth::init().await;
        // Key rotation and policy edits without a restart: both loops are
        // no-ops unless their source is configured (`EVENT_PASETO_KEYS_URL`
        // / `EVENT_ABAC_POLICY_FILE`).
        crate::api::rest::auth::spawn_key_refresh();
        crate::api::rest::auth::spawn_policy_watcher();
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config);
        ctx.shared_store.insert(state.clone());
        // Durable event bus Phase 3: start the outbox relay loop. A no-op
        // unless `EVENT_EVENT_TRANSPORT=outbox` AND `EVENT_EVENT_RELAY` are
        // set, so the default `memory` transport never spawns it.
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
            // trace, not invisible to it (PRO-H9; same reasoning as
            // person-service's/link-graph-service's `after_routes`).
            .layer(axum::middleware::from_fn(observability::trace_mw));
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
