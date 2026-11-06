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
    api::rest::{ApiDoc, AppState, courses_routes, fhir_routes, metrics_routes},
    config::Config,
    matching::CourseMatcher,
    observability::{self, Telemetry, TelemetryConfig},
    search::SearchEngine,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

/// The installed telemetry providers, so `on_shutdown` can flush what the
/// batch processor is still holding. Set once by [`App::init_logger`].
static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

/// The axa application. Owns the boot lifecycle, config, migrations,
/// and the background queue. The REST surface is registered as native
/// axa controllers in [`App::routes`]; `AppState` (search engine,
/// matcher, repositories, config) is built once and placed in the
/// `AppContext` shared store in [`App::after_routes`].
pub struct App;

impl App {
    /// Install this crate's own logging + `OpenTelemetry` stack (repo
    /// `tasks.md` PRO-H12), returning `true` so axa does **not** also
    /// install its own. Ported from person-service's `App::init_logger`
    /// (itself ported from link-graph-service's) — the `App::init_logger`
    /// seam is identical across crates' shapes despite the differing router
    /// layout (see `src/observability.rs`'s module docs and `after_routes`
    /// below for where this crate's shape actually diverges).
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
        // the filter outright; otherwise axa's module whitelist applies,
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

    /// The service's route table.
    #[must_use]
    pub fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // loco /_health, /_ping
            .add_route(courses_routes())
            .add_route(fhir_routes()) // /fhir/Basic{,/{id}} + /fhir/metadata (non-standard)
            .add_route(metrics_routes()) // GET /metrics.prom (root, public)
    }

    /// Add this service's state and layers to the router.
    ///
    /// # Errors
    ///
    /// Fails when the service's setup fails.
    pub async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Build the boot-time singletons (search engine, matcher, repos,
        // domain config) once and place them in the shared store, where
        // the controllers retrieve them via `FromRef<AppContext>`.
        let config = Config::from_env().map_err(|e| axa::Error::string(&e.to_string()))?;
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| axa::Error::string(&e.to_string()))?;
        let matcher = CourseMatcher::new(config.matching);
        // Finalise the PASETO verifier first: when `COURSE_PASETO_KEYS_URL`
        // is set the key set is fetched over HTTP once, here, in async
        // context (fetch failure falls back to the `COURSE_PASETO_KEYS` env
        // path; the service always boots) — before the shared-store insert
        // and the middleware capture the state, so both router surfaces
        // consult the fetched key set.
        crate::api::rest::auth::init().await;
        // Key rotation and policy edits without a restart: both loops are
        // no-ops unless their source is configured (`COURSE_PASETO_KEYS_URL`
        // / `COURSE_ABAC_POLICY_FILE`).
        crate::api::rest::auth::spawn_key_refresh();
        crate::api::rest::auth::spawn_policy_watcher();
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config);
        ctx.shared_store.insert(state.clone());
        // Durable event bus (Phase 3): spawn the outbox relay worker. A
        // no-op unless the transport is `outbox` and `COURSE_EVENT_RELAY`
        // is truthy, so the default `memory` transport is unaffected.
        crate::relay::spawn(ctx.db.clone());
        // Swagger UI + blanket auth enforcement + permissive CORS layered
        // on top of the controllers.
        let router = router
            // Observe every request on `http_requests_total{path,status}`
            // (T-18), before merging in Swagger UI. `route_layer`, not
            // `layer`: it must run after route matching so `MatchedPath`
            // resolves — see `crate::metrics::track_http_requests_mw`.
            .route_layer(axum::middleware::from_fn(
                crate::metrics::track_http_requests_mw,
            ))
            .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
            // Blanket auth enforcement (default-off; a near-noop unless
            // `COURSE_REQUIRE_AUTH` was truthy at construction; layered
            // inside CORS so preflight requests still pass).
            .layer(axum::middleware::from_fn_with_state(
                state,
                crate::api::rest::auth::require_auth_mw,
            ))
            // Header-based API versioning (`Accepts-version`): negotiates
            // the version for `/api/*` and stamps it on the response
            // (`agents/share/api-versioning.md`).
            .layer(axum::middleware::from_fn(
                crate::api::rest::version::require_version_mw,
            ))
            // Outermost layer runs first, so the request span wraps CORS,
            // versioning, and the auth guard too — a 401/403 is part of the
            // trace, not invisible to it (same reasoning as person-service's
            // and link-graph-service's `after_routes`).
            .layer(axum::middleware::from_fn(observability::trace_mw));
        Ok(router)
    }
}

/// Service defaults. The environment overrides them; see `axa::config`.
fn defaults() -> Defaults {
    Defaults {
        database: "course_service",
        port: 8084,
        production_port: None,
        body_limit: 2 * MB,
        max_connections: 5,
        ..Defaults::default()
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
        // `check` reads settings only, so it runs before anything connects.
        if matches!(cli.command, Command::Check) {
            return axa::cli::check(&environment, &defaults());
        }
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
            Command::Check => Ok(()),
            Command::Start { .. } => {
                let router = Self::router(&ctx).await?;
                server::serve(router, &ctx.config.server).await?;
                Self::on_shutdown(&ctx);
                Ok(())
            }
            Command::Db { command } => Self::db(&ctx, command).await,
            Command::Task { name, params } => {
                let tasks = Tasks::default();
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
