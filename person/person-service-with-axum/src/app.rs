//! Application wiring: configuration, the router, the command line, and
//! the boot used by tests. Boot-time singletons are built in
//! [`App::after_routes`] and carried in shared state.

use axa::opentelemetry::Telemetry;
use axa::{
    AppContext, Environment, Error, Result,
    boot::{BootResult, TestApp},
    cli::{Cli, Command, DbCommand, parse_vars},
    config::{Config as ServiceConfig, Defaults, MB, TimeoutOverride},
    queue::{Queue, QueueOptions, warn_if_unserved},
    routes::AppRoutes,
    server,
    task::Tasks,
    worker::{BackgroundWorker, WorkerMode},
};
use axum::Router as AxumRouter;
use migration::{Migrator, MigratorTrait};
use std::sync::OnceLock;

use crate::{
    api::rest::{ApiDoc, AppState, auth, metrics_routes, persons_routes},
    config::Config,
    matching::ProbabilisticMatcher,
    search::SearchEngine,
};

/// OTLP export, started by [`App::init_logger`] when `OTLP_ENDPOINT` is
/// set, so that `after_routes` can measure requests and `on_shutdown` can
/// send what is still queued.
static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

/// The axa application. Owns the boot lifecycle, config, migrations,
/// and the background queue; the REST surface is merged in via
/// [`App::after_routes`].
pub struct App;

impl App {
    /// Start OTLP export through `axa::opentelemetry`, and install axa's
    /// logger with its layers. Returns `false`, so that axa installs its
    /// plain logger, when `OTLP_ENDPOINT` is unset, which means export is
    /// off.
    ///
    /// `OTLP_ENDPOINT` must be `https`, or loopback, or have
    /// `OTLP_ALLOW_INSECURE` set, because spans carry what the logs do.
    ///
    /// # Errors
    ///
    /// When the endpoint is not a URL, or is plain `http` to another host
    /// without `OTLP_ALLOW_INSECURE`. A collector that is down is not an
    /// error: nothing connects until the first export.
    pub fn init_logger(ctx: &AppContext) -> Result<bool> {
        let config =
            axa::opentelemetry::Config::from_env("person-service", env!("CARGO_PKG_VERSION"));
        let telemetry = axa::opentelemetry::start(&config)
            .map_err(|error| Error::Message(format!("OTLP: {error}")))?;
        if !telemetry.exporting() {
            return Ok(false);
        }
        axa::logger::init_with(
            env!("CARGO_CRATE_NAME"),
            &ctx.config.logger,
            telemetry.layers(),
            axa::opentelemetry::DIAGNOSTIC_DIRECTIVES,
        );
        tracing::info!(
            service.name = %config.service_name,
            endpoint = %config.endpoint.as_deref().unwrap_or_default(),
            "otlp_export_started"
        );
        let _ = TELEMETRY.set(telemetry);
        Ok(true)
    }

    /// Measure each request and put `traceparent` on its response, when
    /// export is on. It goes inside `axa::middleware::apply`, whose request
    /// span it reports on.
    pub fn with_telemetry(router: AxumRouter, telemetry: Option<&Telemetry>) -> AxumRouter {
        match telemetry {
            Some(telemetry) => router.layer(axum::middleware::from_fn_with_state(
                telemetry.http_metrics(),
                axa::opentelemetry::middleware,
            )),
            None => router,
        }
    }

    /// Send what OTLP export still holds, on graceful shutdown, so the last
    /// spans are not lost with the process.
    pub fn on_shutdown(_ctx: &AppContext) {
        if let Some(telemetry) = TELEMETRY.get() {
            telemetry.shutdown();
        }
    }

    /// The service's route table.
    #[must_use]
    pub fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(persons_routes())
            .add_route(crate::api::fhir::handlers::routes())
            .add_route(metrics_routes())
    }

    /// The boot-time singletons, built once and kept in the shared store. A job reads them
    /// from there, so a process that serves the queue must call this before its first job
    /// can be claimed: `start --worker` never builds the router, and `start --all` claims
    /// jobs while it does.
    ///
    /// # Errors
    ///
    /// Fails when a setting is invalid or the search index cannot be opened.
    async fn shared_state(ctx: &AppContext) -> Result<AppState> {
        if let Some(state) = ctx.shared_store.get::<AppState>() {
            return Ok(state.clone());
        }
        let config = Config::from_env().map_err(|e| axa::Error::string(&e.to_string()))?;
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| axa::Error::string(&e.to_string()))?;
        let matcher = ProbabilisticMatcher::new(config.matching.clone());
        // Boot-time PASETO key-set fetch (`PERSON_PASETO_KEYS_URL`, spec
        // §13 T-1c): resolve the verifier — fetched key set when the URL
        // is set and reachable, env key set otherwise — BEFORE the
        // enforcement middleware and the shared-store state are built,
        // so both router surfaces (the enforcement layer, which
        // snapshots the verifier here, and the axa handlers' `AuthUser`
        // extractor, which reads it from the shared-store `AppState`)
        // verify against the same key set. Fetch failure falls back to
        // the env path — the service always boots.
        auth::init().await;
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config)
            .await
            .map_err(|e| axa::Error::string(&e.to_string()))?;
        ctx.shared_store.insert(state.clone());
        Ok(state)
    }

    /// Add this service's state and layers to the router.
    ///
    /// # Errors
    ///
    /// Fails when the service's setup fails.
    pub async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let state = Self::shared_state(ctx).await?;
        // Key rotation and policy edits without a restart: both loops are
        // no-ops unless their source is configured (`PERSON_PASETO_KEYS_URL`
        // / `PERSON_ABAC_POLICY_FILE`).
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        // Blanket auth enforcement (default-off, `PERSON_REQUIRE_AUTH`),
        // snapshotted here at boot — changing the env var requires a
        // restart. Layered unconditionally; the flag is the only switch.
        // The verifier and policy are read live per request.
        let enforcement = auth::Enforcement::from_env();
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
        let grpc_config = state.config.server.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::api::grpc::serve(grpc_config, grpc_state).await {
                tracing::error!("gRPC server failed to start or exited: {e}");
            }
        });
        // Durable event bus Phase 3: start the outbox relay loop. Gated
        // internally — a no-op unless the transport is `outbox`
        // (`PERSON_EVENT_TRANSPORT`) and `PERSON_EVENT_RELAY` is truthy —
        // so default (`memory`) boots behave exactly as before.
        crate::relay::spawn(ctx.db.clone());
        let router = router
            .merge(axa::openapi::router(
                axa::openapi::spec_of::<ApiDoc>(),
                "Person Service API",
            ))
            .layer(axum::middleware::from_fn_with_state(
                enforcement,
                auth::require_auth_middleware,
            ))
            // Header-based API versioning (`Accepts-version`): negotiates
            // the version for `/api/*` and stamps it on the response
            // (`agents/share/api-versioning.md`).
            .layer(axum::middleware::from_fn(
                axa::versioning::require_version_mw,
            ));
        // Outermost of these, so the metric and `traceparent` cover
        // versioning and the auth guard too: a 401 or 403 is measured.
        // After the auth guard, so the lockout counts its 401s; inside the
        // telemetry layer, so a 429 is measured.
        let router = crate::client_limits::from_env(router)?;
        Ok(Self::with_telemetry(router, TELEMETRY.get()))
    }

    /// Register background workers.
    ///
    /// # Errors
    ///
    /// Fails when a worker cannot be registered.
    pub fn connect_workers(ctx: &AppContext, queue: &mut Queue) -> Result<()> {
        // Bulk import/export jobs drain on the Postgres-backed queue
        // (`bg_pg`); the worker is a thin adapter over `crate::bulk::pipeline`.
        queue.register(crate::bulk::worker::BulkJobWorker::build(ctx));
        Ok(())
    }

    /// Register command-line tasks.
    pub fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::bulk_artifact_sweep::BulkArtifactSweep);
        tasks.register(crate::tasks::integrity_key::IntegrityKey);
        tasks.register(crate::tasks::integrity_resign::IntegrityResign);
        tasks.register(crate::tasks::seed_examples::SeedExamples);
    }
}

/// How long a bulk import may take. A large upload on a slow link legitimately
/// runs well past the normal request timeout.
const IMPORT_TIMEOUT_SECONDS: u64 = 600;

/// Service configuration: the defaults below, the environment, and the routes
/// with their own limits.
fn service_config(environment: &Environment) -> Result<ServiceConfig> {
    let mut config = ServiceConfig::from_env(environment, &defaults())?;
    config.http.timeout_overrides.push(TimeoutOverride {
        path_suffix: "/import".to_string(),
        seconds: IMPORT_TIMEOUT_SECONDS,
    });
    Ok(config)
}

/// Service defaults. The environment overrides them; see `axa::config`.
fn defaults() -> Defaults {
    Defaults {
        database: "person_service",
        port: 5150,
        production_port: Some(8080),
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
        let mut ctx = AppContext::new(environment, db, config);
        if ctx.config.workers == WorkerMode::BackgroundQueue {
            let mut queue =
                Queue::connect(&ctx.config.database.uri, QueueOptions::default()).await?;
            Self::connect_workers(&ctx, &mut queue)?;
            ctx.queue_provider = Some(std::sync::Arc::new(queue));
        }
        Ok(ctx)
    }

    /// Build the full router: the routes, this service's own layers, then
    /// the standard middleware outside them, so that a refusal from a guard
    /// still gets a request id, security headers, CORS, and the panic and
    /// timeout protection.
    ///
    /// # Errors
    ///
    /// Fails when the service's setup fails.
    pub async fn router(ctx: &AppContext) -> Result<AxumRouter> {
        let router = Self::routes(ctx).into_router(ctx.clone());
        let router = Self::after_routes(router, ctx).await?;
        axa::middleware::apply(router, &ctx.config.http)
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
        let mut config = service_config(&environment)?;
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
            Command::Start { worker, all, .. } => {
                warn_if_unserved(&ctx, worker || all);
                crate::bulk::reconcile::spawn(&ctx, worker || all);
                crate::compliance::address_retention::spawn(ctx.db.clone())?;
                if worker || all {
                    Self::shared_state(&ctx).await?;
                }
                let (stop, stopped) = tokio::sync::watch::channel(false);
                let jobs = (worker || all)
                    .then(|| ctx.queue_provider.clone())
                    .flatten()
                    .map(|queue| tokio::spawn(queue.run(stopped)));
                if worker {
                    server::shutdown_signal().await;
                } else {
                    let router = Self::router(&ctx).await?;
                    server::serve(router, &ctx.config.server).await?;
                }
                let _ = stop.send(true);
                if let Some(jobs) = jobs {
                    let _ = jobs.await;
                }
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
        let config = service_config(&environment)?;
        let ctx = Self::context(environment, config).await?;
        Migrator::up(&ctx.db, None).await.map_err(Error::from)?;
        let router = Self::router(&ctx).await?;
        Ok(BootResult {
            app_context: ctx,
            router: Some(router),
        })
    }
}
