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
    api::rest::{ApiDoc, AppState, auth, fhir_routes, metrics_routes, workers_routes},
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

    /// Install this crate's own logging + `OpenTelemetry` stack (repo
    /// `tasks.md` PRO-H9), returning `true` so loco does **not** also
    /// install its own. Ported from person-service's `App::init_logger`
    /// (itself ported verbatim from link-graph-service's) — the
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
        // Boot-time PASETO key-set fetch (`WORKER_PASETO_KEYS_URL`, spec
        // §13 T-1b fetch item): resolve the verifier — fetched key set
        // when the URL is set and reachable, env key set otherwise —
        // BEFORE the enforcement middleware and the shared-store state
        // are built, so both router surfaces (the enforcement layer,
        // which captures the verifier here, and the handlers' `AuthUser`
        // extractor, which reads it from the shared-store `AppState`)
        // verify against the same key set. Fetch failure falls back to
        // the env path — the service always boots.
        auth::init().await;
        // Key rotation and policy edits without a restart: both loops are
        // no-ops unless their source is configured
        // (`WORKER_PASETO_KEYS_URL` / `WORKER_ABAC_POLICY_FILE`).
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        // Grabbed before `config` moves into `AppState::new` below.
        let grpc_config = config.server.clone();
        // Bundle DB handle + singletons into shared application state.
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config);
        // PRO-H11: the real gRPC server, spawned alongside the REST
        // router rather than blocking boot on it. Shares this exact
        // `AppState` (cloned — the REST router below takes the
        // original), so both surfaces see one database pool, one
        // search index, one matcher. A bind/serve failure is logged,
        // not fatal: the REST surface still comes up even if the gRPC
        // port is unavailable, matching this crate's existing
        // "always boot" posture for other best-effort subsystems (key
        // refresh, policy watch, the outbox relay).
        let grpc_state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::api::grpc::serve(grpc_config, grpc_state).await {
                tracing::error!("gRPC server failed to start or exited: {e}");
            }
        });
        // Make the state retrievable by request handlers via the shared store.
        ctx.shared_store.insert(state);
        // Durable event bus Phase 3: spawn the outbox relay loop. A no-op
        // unless `WORKER_EVENT_TRANSPORT=outbox` AND `WORKER_EVENT_RELAY`
        // are both set, so the default `memory` transport is unchanged.
        crate::relay::spawn(ctx.db.clone());
        // Mount Swagger UI, then the blanket auth-enforcement middleware
        // (spec §13 T-1b: default-off, gated by `WORKER_REQUIRE_AUTH` read
        // here at construction — restart to change; the ABAC policy from
        // `WORKER_ABAC_POLICY`/`_FILE`, else the built-in default, is
        // captured alongside it), then a permissive CORS layer (outermost,
        // so preflight `OPTIONS` is answered before enforcement runs).
        let router = router
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()));
        let router = auth::apply_enforcement(router, auth::require_auth_from_env())
            .layer(axum::middleware::from_fn(
                crate::api::rest::version::require_version_mw,
            ))
            .layer(tower_http::cors::CorsLayer::permissive())
            // Outermost layer runs first, so the request span wraps CORS,
            // versioning, and the auth guard too — a 401/403 is part of the
            // trace, not invisible to it (PRO-H9, same reasoning as
            // person-service's / link-graph-service's `after_routes`).
            .layer(axum::middleware::from_fn(observability::trace_mw));
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
    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::integrity_key::IntegrityKey);
        tasks.register(crate::tasks::integrity_resign::IntegrityResign);
    }

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
