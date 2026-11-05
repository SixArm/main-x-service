//! Application wiring: configuration, the router, the command line, and
//! the boot used by tests. Boot-time singletons are built in
//! [`App::after_routes`] and carried in shared state.

use axa::{
    AppContext, Environment, Error, Result,
    boot::{BootResult, TestApp},
    cli::{Cli, Command, DbCommand, parse_vars},
    config::{Config as ServiceConfig, Defaults, MB},
    routes::AppRoutes,
    server,
    task::Tasks,
};
use axum::Router as AxumRouter;
use migration::{Migrator, MigratorTrait};
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

impl App {
    /// Install this crate's own logging + `OpenTelemetry` stack (repo
    /// `tasks.md` PRO-H9), returning `true` so loco does **not** also
    /// install its own. Ported from person-service's `App::init_logger`
    /// (itself ported verbatim from link-graph-service's) — the
    /// `Hooks::init_logger` seam is identical across all three crates'
    /// shapes despite the differing router layout (see
    /// `src/observability.rs`'s module docs and `after_routes` below for
    /// where this crate's shape genuinely diverges).
    ///
    /// The stack is loco's — its [`EnvFilter`](axa::logger::init_env_filter)
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
    pub fn init_logger(ctx: &AppContext) -> Result<bool> {
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
            axa::logger::init_env_filter(
                env!("CARGO_CRATE_NAME"),
                logger_config.override_filter.as_ref(),
                &logger_config.level,
            ),
            operator_supplied,
        );
        // `logger.enable: false` means "no stdout layer" — honour it rather
        // than quietly turning logging on for anyone who enables export.
        let fmt_layer = logger_config
            .enable
            .then(|| axa::logger::init_layer(std::io::stdout, &logger_config.format, true));
        let telemetry = observability::init(&telemetry_config, env_filter, fmt_layer)
            .map_err(|error| axa::Error::Message(format!("OTLP init failed: {error}")))?;
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
    pub fn on_shutdown(_ctx: &AppContext) {
        if let Some(telemetry) = TELEMETRY.get() {
            telemetry.shutdown();
        }
    }

    /// Registers the hand-written Axum route groups on top of loco's default
    /// routes: worker CRUD/search/match (`workers_routes`), FHIR R5
    /// (`fhir_routes`), and Prometheus exposition (`metrics_routes`).
    #[must_use]
    pub fn routes(_ctx: &AppContext) -> AppRoutes {
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
    /// into [`axa::Error::string`].
    pub async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Load domain config (env + defaults); map domain error to loco error.
        let config = Config::from_env().map_err(|e| axa::Error::string(&e.to_string()))?;
        // Open/create the Tantivy index at the configured path.
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| axa::Error::string(&e.to_string()))?;
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
            // Outermost layer runs first, so the request span wraps CORS,
            // versioning, and the auth guard too — a 401/403 is part of the
            // trace, not invisible to it (PRO-H9, same reasoning as
            // person-service's / link-graph-service's `after_routes`).
            .layer(axum::middleware::from_fn(observability::trace_mw));
        Ok(router)
    }

    /// CLI-task registration hook. No custom tasks are registered.
    pub fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::integrity_key::IntegrityKey);
        tasks.register(crate::tasks::integrity_resign::IntegrityResign);
    }
}

/// Service defaults. The environment overrides them; see `axa::config`.
fn defaults() -> Defaults {
    Defaults {
        database: "worker_service",
        port: 5150,
        production_port: Some(8080),
        body_limit: 2 * MB,
        max_connections: 5,
    }
}

impl App {
    /// Connect the database and build the context.
    ///
    /// # Errors
    ///
    /// Fails when the database cannot be reached.
    pub async fn context(environment: Environment, config: ServiceConfig) -> Result<AppContext> {
        let db = axa::db::connect(&config.database).await?;
        Ok(AppContext::new(environment, db, config))
    }

    /// Build the full router: the routes under the standard middleware,
    /// then this service's own layers.
    ///
    /// # Errors
    ///
    /// Fails when the service's setup fails.
    pub async fn router(ctx: &AppContext) -> Result<AxumRouter> {
        let router = Self::routes(ctx).into_app(ctx.clone())?;
        Self::after_routes(router, ctx).await
    }

    /// Run a database command.
    async fn db(ctx: &AppContext, command: DbCommand) -> Result<()> {
        match command {
            DbCommand::Migrate => Migrator::up(&ctx.db, None).await?,
            DbCommand::Down { steps } => Migrator::down(&ctx.db, Some(steps)).await?,
            DbCommand::Reset => Migrator::fresh(&ctx.db).await?,
            DbCommand::Status => Migrator::status(&ctx.db).await?,
            DbCommand::Truncate => return Err(Error::string("this service has no truncate")),
        }
        Ok(())
    }

    /// Run the command line.
    ///
    /// # Errors
    ///
    /// Fails when boot, a command, or the server fails.
    pub async fn run(cli: Cli) -> Result<()> {
        let environment = cli
            .environment
            .as_deref()
            .map_or_else(Environment::from_env, Environment::parse);
        let mut config = ServiceConfig::from_env(&environment, &defaults())?;
        if let Command::Start { binding, port, .. } = &cli.command {
            if let Some(binding) = binding {
                config.server.binding.clone_from(binding);
            }
            if let Some(port) = port {
                config.server.port = *port;
            }
        }
        let ctx = Self::context(environment, config).await?;
        if !Self::init_logger(&ctx)? {
            axa::logger::init(env!("CARGO_CRATE_NAME"), &ctx.config.logger);
        }
        if ctx.config.database.auto_migrate && !matches!(cli.command, Command::Db { .. }) {
            Migrator::up(&ctx.db, None).await?;
        }
        match cli.command {
            Command::Start { .. } => {
                let router = Self::router(&ctx).await?;
                server::serve(router, &ctx.config.server).await?;
                Self::on_shutdown(&ctx);
                Ok(())
            }
            Command::Db { command } => Self::db(&ctx, command).await,
            Command::Task { name, params } => {
                let mut tasks = Tasks::default();
                Self::register_tasks(&mut tasks);
                let Some(name) = name else {
                    for task in tasks.list() {
                        println!("{}\t{}", task.name, task.detail);
                    }
                    return Ok(());
                };
                tasks.run(&name, &ctx, &parse_vars(&params)?).await
            }
        }
    }
}

impl TestApp for App {
    async fn boot_test() -> Result<BootResult> {
        let environment = Environment::Test;
        let config = ServiceConfig::from_env(&environment, &defaults())?;
        let ctx = Self::context(environment, config).await?;
        Migrator::up(&ctx.db, None).await.map_err(Error::from)?;
        let router = Self::router(&ctx).await?;
        Ok(BootResult {
            app_context: ctx,
            router: Some(router),
        })
    }
}
