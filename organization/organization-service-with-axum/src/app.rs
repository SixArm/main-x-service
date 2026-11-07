//! Application wiring: configuration, the router, the command line, and
//! the boot used by tests. Boot-time singletons are built in
//! [`App::after_routes`] and carried in shared state.

use axa::opentelemetry::Telemetry;
use axa::{
    AppContext, Environment, Error, Result,
    boot::{BootResult, TestApp},
    cli::{Cli, Command, DbCommand, parse_vars},
    config::{Config as ServiceConfig, Defaults, MB, TimeoutOverride},
    db::truncate_table,
    queue::{Queue, QueueOptions, warn_if_unserved},
    routes::AppRoutes,
    server,
    task::Tasks,
    worker::{BackgroundWorker, WorkerMode},
};
use axum::{
    Router as AxumRouter,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use migration::{Migrator, MigratorTrait};
use std::sync::OnceLock;

use crate::{
    auth, controllers,
    models::_entities::{audit_logs, bulk_jobs, event_outbox, merge_records, organizations},
};

/// OTLP export, started by [`App::init_logger`] when `OTLP_ENDPOINT` is
/// set, so that `after_routes` can measure requests and `on_shutdown` can
/// send what is still queued.
static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

/// Blanket auth-enforcement middleware: authentication (PASETO and/or Keycloak bearer)
/// then ABAC authorization. Reads the flag, verifier, and policy per
/// request (all cached `OnceLock`s), so the layer is wired
/// unconditionally and is a near-noop when `ORGANIZATION_REQUIRE_AUTH`
/// is off. See [`auth::enforce`] for the pure decision.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let decision = auth::enforce_request(
        auth::require_auth(),
        req.method(),
        req.uri().path(),
        req.headers(),
        // Per-request snapshots, so the refresh loop and the policy
        // watcher reach the guard too — not just the handlers.
        &auth::policy().current(),
    )
    .await;
    match decision {
        Ok(()) => next.run(req).await,
        Err((status, reason)) => {
            // The reason goes to the log, never to the caller.
            tracing::warn!(status = status.as_u16(), %reason, "auth_refused");
            let text = if status == axum::http::StatusCode::UNAUTHORIZED {
                "invalid token"
            } else {
                "forbidden"
            };
            (status, text).into_response()
        }
    }
}

/// The axa application type. Carries no state; it exists to implement
/// [`Hooks`], which axa's CLI drives to boot the service, register
/// routes, and run migrations.
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
            axa::opentelemetry::Config::from_env("organization-service", env!("CARGO_PKG_VERSION"));
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

    /// Register the controller route trees on top of axa's defaults
    /// (`/_health`, `/_ping`): the bulk import/export routes (BLK-5), the
    /// organization CRUD/match/audit routes, the OpenAPI/Swagger docs
    /// routes, and the Prometheus metrics route (`/metrics.prom`,
    /// mounted at root like the docs).
    ///
    /// The bulk routes are added **before**
    /// `controllers::organizations::routes()` so their literal paths
    /// (`/import`, `/export`, `/bulk-jobs`) are registered ahead of that
    /// tree's `/{pid}` capture — the same literal-before-dynamic
    /// ordering `controllers::organizations::routes` documents for its
    /// own `/search`, `/merge`, `/whoami`, …
    #[must_use]
    pub fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(crate::bulk::handlers::routes())
            .add_route(controllers::organizations::routes())
            .add_route(controllers::compliance::routes())
            .add_route(controllers::fhir::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    /// Seed the process-wide token verifier (fetching the PASETO key set
    /// over HTTP once when `ORGANIZATION_PASETO_KEYS_URL` is set; env /
    /// reject-all fallback otherwise — the boot never fails on it), then
    /// wrap the assembled router in the blanket auth-enforcement layer.
    ///
    /// # Errors
    ///
    /// Never (returns `Ok`); the signature is axa's.
    pub async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Seed the verifier before serving so `enforce()`/`AuthUser`
        // consult the boot-fetched key set from the first request on.
        auth::init_from_env().await;
        // Key rotation and policy edits without a restart: both loops are
        // no-ops unless their source is configured
        // (`ORGANIZATION_PASETO_KEYS_URL` / `ORGANIZATION_ABAC_POLICY_FILE`).
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        // Durable event bus Phase 3: start the outbox relay loop. A no-op
        // unless `ORGANIZATION_EVENT_TRANSPORT=outbox` AND
        // `ORGANIZATION_EVENT_RELAY` are set, so the default `memory`
        // transport never spawns it.
        crate::relay::spawn(ctx.db.clone());
        // Rebuild the full-text index if it is empty while records exist
        // — the upgrade path for a deployment whose data predates the
        // index, and the recovery path for an index directory that did
        // not survive a restart. A no-op on a normal boot.
        crate::tasks::search::spawn_reindex_if_empty(ctx.db.clone());
        // Blanket auth enforcement, gated by `ORGANIZATION_REQUIRE_AUTH`
        // (off by default). The layer is added unconditionally; the flag
        // is read per request inside the middleware. Header-based API
        // versioning (`Accepts-version`) is layered alongside it.
        let router = router
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn(
                axa::versioning::require_version_mw,
            ));
        // Outermost of these, so the metric and `traceparent` cover
        // versioning and the auth guard too: a 401 or 403 is measured.
        // This is this crate's only router-construction surface.
        // After the auth guard, so the lockout counts its 401s; inside the
        // telemetry layer, so a 429 is measured.
        let router = crate::client_limits::from_env(router)?;
        crate::cursor_key::init_from_env()?;
        Ok(Self::with_telemetry(router, TELEMETRY.get()))
    }

    /// Register background-queue workers: the BLK-5 bulk import/export
    /// worker, which drains `bulk_jobs` under `queue.kind: Postgres`
    /// (`workers.mode: BackgroundQueue` in `config/*.yaml`; the axa
    /// scaffold's unrelated `DownloadWorker` stub was removed — see
    /// entity spec §13 T-12).
    ///
    /// # Errors
    ///
    /// Never (returns `Ok`); the signature is axa's.
    pub fn connect_workers(ctx: &AppContext, queue: &mut Queue) -> Result<()> {
        queue.register(crate::bulk::worker::BulkJobWorker::build(ctx));
        Ok(())
    }

    /// Register CLI tasks. None are registered; the inject marker is kept
    /// so the Loco generator can splice tasks in later.
    #[allow(unused_variables)]
    pub fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::search::SearchReindex);
        tasks.register(crate::tasks::seed_examples::SeedExamples);
        // tasks-inject (do not remove)
    }

    /// Truncate all tables between tests (request-suite setup).
    ///
    /// Order matters only for FK safety; these tables have no FKs, but
    /// children-before-parent order is kept by convention: merge/audit
    /// rows and bulk jobs, then the organizations they reference.
    ///
    /// # Errors
    ///
    /// Propagates any truncate failure.
    pub async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, event_outbox::Entity).await?;
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, bulk_jobs::Entity).await?;
        truncate_table(&ctx.db, organizations::Entity).await?;
        Ok(())
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
        database: "organization_service",
        port: 5150,
        production_port: None,
        body_limit: 2 * MB,
        max_connections: 1,
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
            DbCommand::Truncate => Self::truncate(ctx).await?,
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
        // Every boot starts from an empty, freshly migrated schema, so no
        // table can carry rows from an earlier test.
        Migrator::fresh(&ctx.db).await.map_err(Error::from)?;
        let router = Self::router(&ctx).await?;
        Ok(BootResult {
            app_context: ctx,
            router: Some(router),
        })
    }
}
