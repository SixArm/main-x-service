//! Loco application hooks.
//!
//! The service boots through loco (CLI, `AppContext`, config, migrations,
//! background queue). The existing REST router (including the FHIR
//! surface mounted within it) is built in [`App::after_routes`] and
//! merged onto loco's router, so the hand-written Axum surface keeps
//! working while loco owns the lifecycle. Boot-time singletons
//! (`SearchEngine`, `ProbabilisticMatcher`, domain `Config`) are
//! constructed here and carried in [`AppState`].

use async_trait::async_trait;
use axum::Router as AxumRouter;
use loco_rs::{
    Result,
    app::{AppContext, Hooks},
    bgworker::{BackgroundWorker, Queue},
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
    api::rest::{ApiDoc, AppState, auth, metrics_routes, persons_routes},
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
        config: LocoConfig,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// Install this crate's own logging + `OpenTelemetry` stack (repo
    /// `tasks.md` PRO-H9), returning `true` so loco does **not** also
    /// install its own. Ported verbatim from link-graph-service's
    /// `App::init_logger` — the `Hooks::init_logger` seam is identical
    /// across both crates' shapes despite the differing router layout
    /// (see `src/observability.rs`'s module docs and `after_routes` below
    /// for where the two crates genuinely diverge).
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

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(persons_routes())
            .add_route(crate::api::fhir::handlers::routes())
            .add_route(metrics_routes())
    }

    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let config = Config::from_env().map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        let matcher = ProbabilisticMatcher::new(config.matching.clone());
        // Boot-time PASETO key-set fetch (`PERSON_PASETO_KEYS_URL`, spec
        // §13 T-1c): resolve the verifier — fetched key set when the URL
        // is set and reachable, env key set otherwise — BEFORE the
        // enforcement middleware and the shared-store state are built,
        // so both router surfaces (the enforcement layer, which
        // snapshots the verifier here, and the loco handlers' `AuthUser`
        // extractor, which reads it from the shared-store `AppState`)
        // verify against the same key set. Fetch failure falls back to
        // the env path — the service always boots.
        auth::init().await;
        // Key rotation and policy edits without a restart: both loops are
        // no-ops unless their source is configured (`PERSON_PASETO_KEYS_URL`
        // / `PERSON_ABAC_POLICY_FILE`).
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config)
            .await
            .map_err(|e| loco_rs::Error::string(&e.to_string()))?;
        // Blanket auth enforcement (default-off, `PERSON_REQUIRE_AUTH`),
        // snapshotted here at boot — changing the env var requires a
        // restart. Layered unconditionally; the flag is the only switch.
        // The verifier and policy are read live per request.
        let enforcement = auth::Enforcement::from_env();
        ctx.shared_store.insert(state);
        // Durable event bus Phase 3: start the outbox relay loop. Gated
        // internally — a no-op unless the transport is `outbox`
        // (`PERSON_EVENT_TRANSPORT`) and `PERSON_EVENT_RELAY` is truthy —
        // so default (`memory`) boots behave exactly as before.
        crate::relay::spawn(ctx.db.clone());
        let router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            .layer(axum::middleware::from_fn_with_state(
                enforcement,
                auth::require_auth_middleware,
            ))
            // Header-based API versioning (`Accepts-version`): negotiates
            // the version for `/api/*` and stamps it on the response
            // (`agents/share/api-versioning.md`).
            .layer(axum::middleware::from_fn(
                crate::api::rest::version::require_version_mw,
            ))
            .layer(tower_http::cors::CorsLayer::permissive())
            // Outermost layer runs first, so the request span wraps CORS,
            // versioning, and the auth guard too — a 401/403 is part of the
            // trace, not invisible to it (same reasoning as
            // link-graph-service's `after_routes`).
            .layer(axum::middleware::from_fn(observability::trace_mw));
        Ok(router)
    }

    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        // Bulk import/export jobs drain on the Postgres-backed queue
        // (`bg_pg`); the worker is a thin adapter over `crate::bulk::pipeline`.
        queue
            .register(crate::bulk::worker::BulkJobWorker::build(ctx))
            .await?;
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::bulk_artifact_sweep::BulkArtifactSweep);
        tasks.register(crate::tasks::integrity_key::IntegrityKey);
        tasks.register(crate::tasks::integrity_resign::IntegrityResign);
        tasks.register(crate::tasks::seed_examples::SeedExamples);
    }

    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
