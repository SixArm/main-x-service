//! axa application wiring: the [`App`] `Hooks` implementation that
//! registers routes, background workers, and the truncate/seed
//! lifecycle for `care-pathway-service`.

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
use axum::{
    Router as AxumRouter,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::OnceLock;

#[allow(unused_imports)]
use crate::{auth, bulk, controllers, tasks, workers::bulk_export::BulkExportWorker};

/// OTLP export, started by [`App::init_logger`] when `OTLP_ENDPOINT` is
/// set, so that `after_routes` can measure requests and `on_shutdown` can
/// send what is still queued.
static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

/// Blanket `/api/*` auth-enforcement middleware. Reads the flag,
/// verifier, and ABAC policy per request and delegates the decision to
/// [`auth::enforce_request`]: when enforcement is off (the default) or the path
/// is public it is a near-noop; otherwise an absent/invalid bearer
/// token yields `401`, and a valid token the ABAC policy denies yields
/// `403` (see `agents/share/authorization-attributes.md`).
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    match auth::enforce_request(
        auth::require_auth(),
        req.method(),
        &path,
        req.headers(),
        // Per-request snapshot, so the policy watcher reaches the guard
        // too — not just the handlers.
        &auth::policy().current(),
    )
    .await
    {
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

/// The axa application hooks for `care-pathway-service`.
///
/// Implements [`Hooks`] to register routes, background workers, and the
/// truncate/seed lifecycle; the binary entrypoint drives it via the
/// axa CLI.
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
            axa::opentelemetry::Config::from_env("care-pathway-service", env!("CARGO_PKG_VERSION"));
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

    /// Assemble the route table: axa's default routes (`/_health`,
    /// `/_ping`, …) plus the care-pathway controller, the API-docs
    /// controller, and the root-level Prometheus `/metrics.prom` endpoint.
    #[must_use]
    pub fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::insights::routes())
            .add_route(controllers::tba::pathway_routes())
            .add_route(controllers::instances::pathway_routes())
            .add_route(controllers::exports::routes())
            .add_route(controllers::data_quality::routes())
            .add_route(controllers::links::routes())
            .add_route(controllers::tba::routes())
            .add_route(controllers::instances::routes())
            // Native bulk import/export (§13 T-10) + the review-queue
            // surface it feeds: mounted before care_pathways::routes()
            // so their literal paths (/import, /export, /bulk-jobs,
            // /review-queue) register ahead of the /{pid} capture.
            .add_route(bulk::handlers::routes())
            .add_route(controllers::review_queue::routes())
            .add_route(controllers::care_pathways::routes())
            .add_route(controllers::fhir::routes())
            .add_route(controllers::compliance::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    /// Seed the process-wide token verifier (fetching the PASETO key set
    /// over HTTP once when `CARE_PATHWAY_PASETO_KEYS_URL` is set; env /
    /// reject-all fallback otherwise — the boot never fails on it), then
    /// wrap the assembled router in the blanket auth-enforcement layer.
    ///
    /// Layered unconditionally; [`require_auth_mw`] is a near-noop unless
    /// `CARE_PATHWAY_REQUIRE_AUTH` is on (see [`auth::require_auth`]).
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is axa's.
    pub async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Seed the verifier before serving so `enforce()`/`AuthUser`
        // consult the boot-fetched key set from the first request on.
        auth::init_from_env().await;
        // Key rotation and policy edits without a restart: both loops are
        // no-ops unless their source is configured
        // (`CARE_PATHWAY_PASETO_KEYS_URL` / `CARE_PATHWAY_ABAC_POLICY_FILE`).
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        // Time-based-analysis gauges: a no-op unless
        // `CARE_PATHWAY_FLOW_METRICS_SECS` is set (see `flow_metrics`).
        crate::flow_metrics::spawn(ctx);
        // Durable event bus Phase 3: spawn the outbox relay (drain → sink →
        // mark published + retention purge). A no-op unless the transport is
        // `outbox` and `CARE_PATHWAY_EVENT_RELAY` is truthy.
        crate::relay::spawn(ctx.db.clone());
        // Rebuild the full-text index if it is empty while records exist
        // — the upgrade path for a deployment whose data predates the
        // index, and the recovery path for an index directory that did
        // not survive a restart. A no-op on a normal boot.
        crate::tasks::search::spawn_reindex_if_empty(ctx.db.clone());
        // Blanket auth enforcement, off by default and gated per-request by
        // `CARE_PATHWAY_REQUIRE_AUTH` (see `auth::require_auth`). Wired
        // unconditionally; the flag is the only switch. The version
        // middleware negotiates `Accepts-version` for `/api/*` and is
        // orthogonal to the auth guard.
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

    /// Register background workers with the queue: [`BulkExportWorker`]
    /// (FHIR Bulk Data `$export`) and [`bulk::worker::BulkJobWorker`]
    /// (the native bulk import/export API, §13 T-10).
    ///
    /// # Errors
    ///
    /// Propagates queue-registration errors.
    pub fn connect_workers(ctx: &AppContext, queue: &mut Queue) -> Result<()> {
        queue.register(BulkExportWorker::build(ctx));
        queue.register(bulk::worker::BulkJobWorker::build(ctx));
        Ok(())
    }

    /// Register CLI tasks — none for the MVP (the inject marker is kept so
    /// `cargo axa generate task` can splice new tasks in).
    #[allow(unused_variables)]
    pub fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::search::SearchReindex);
        tasks.register(crate::tasks::integrity_key::IntegrityKey);
        tasks.register(crate::tasks::integrity_resign::IntegrityResign);
        tasks.register(crate::tasks::journeys_seed::JourneysSeed);
        // tasks-inject (do not remove)
    }

    /// Truncate all tables for the axa test harness, in
    /// foreign-key-safe order: history/audit children first, then the
    /// `care_pathways` parent.
    ///
    /// # Errors
    ///
    /// Propagates DB truncation errors.
    pub async fn truncate(ctx: &AppContext) -> Result<()> {
        crate::models::delete_all_rows(&ctx.db).await?;
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

/// A failed migration, as axa's error. `MigrateError` has no `From` into
/// it, so its message is carried instead.
#[allow(clippy::needless_pass_by_value)]
fn migrate_error(error: axa::sqlx::sqlx::migrate::MigrateError) -> Error {
    Error::Message(format!("migration: {error}"))
}

/// Service defaults. The environment overrides them; see `axa::config`.
fn defaults() -> Defaults {
    Defaults {
        database: "care_pathway_service",
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

    /// Apply pending migrations ([`crate::models::MIGRATOR`]).
    ///
    /// # Errors
    ///
    /// Fails when a migration fails, or an applied one no longer matches
    /// its file.
    pub async fn migrate(ctx: &AppContext) -> Result<()> {
        axa::sqlx::migrate(&ctx.db, &crate::models::MIGRATOR)
            .await
            .map_err(migrate_error)
    }

    /// Run a database command. `down`, `reset` and `truncate` destroy
    /// data, so `axa::cli::parse` has already refused them outside
    /// development and test.
    async fn db(ctx: &AppContext, command: DbCommand) -> Result<()> {
        let migrator = &crate::models::MIGRATOR;
        match command {
            DbCommand::Migrate => Self::migrate(ctx).await?,
            DbCommand::Down { steps } => axa::sqlx::revert(&ctx.db, migrator, steps)
                .await
                .map_err(migrate_error)?,
            DbCommand::Reset => axa::sqlx::dangerously_recreate(&ctx.db, migrator)
                .await
                .map_err(migrate_error)?,
            DbCommand::Status => {
                for migration in axa::sqlx::status(&ctx.db, migrator).await? {
                    let state = if migration.applied {
                        "applied"
                    } else {
                        "pending"
                    };
                    println!("{}\t{state}\t{}", migration.version, migration.description);
                }
            }
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
            Self::migrate(&ctx).await?;
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
        axa::sqlx::dangerously_recreate(&ctx.db, &crate::models::MIGRATOR)
            .await
            .map_err(migrate_error)?;
        let router = Self::router(&ctx).await?;
        Ok(BootResult {
            app_context: ctx,
            router: Some(router),
        })
    }
}
