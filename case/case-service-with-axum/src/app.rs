//! axa application wiring: the [`App`] `Hooks` implementation that
//! registers routes, background workers, and the truncate/seed
//! lifecycle for `case-service`.

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

#[allow(unused_imports)]
use crate::{
    auth, controllers,
    models::_entities::{audit_logs, cases, entity_links, merge_records},
    tasks,
};

/// OTLP export, started by [`App::init_logger`] when `OTLP_ENDPOINT` is
/// set, so that `after_routes` can measure requests and `on_shutdown` can
/// send what is still queued.
static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

/// Blanket auth-enforcement middleware. Reads the `CASE_REQUIRE_AUTH`
/// flag per request via [`auth::require_auth`] and delegates the decision
/// to [`auth::enforce_request`]: public paths and the disabled flag pass
/// through; otherwise a valid bearer token is required (`401`) and the
/// token's `attrs` claim must satisfy the process-wide ABAC policy
/// ([`auth::policy`]) for the action derived from the method + path
/// (`403` with the deciding rule otherwise). Off by default (see
/// `auth.rs` and `agents/share/authorization-attributes.md`).
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    // Snapshot the current (hot-reloadable) policy for this request; the
    // verifiers (PASETO and/or Keycloak) are read live by `request_claims`.
    let policy = auth::policy().current();
    match auth::enforce_request(auth::require_auth(), &method, &path, req.headers(), &policy).await
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

/// The axa application hooks for `case-service`.
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
            axa::opentelemetry::Config::from_env("case-service", env!("CARGO_PKG_VERSION"));
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
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::cases::routes())
            .add_route(crate::bulk::handlers::routes())
            .add_route(controllers::links::routes())
            .add_route(controllers::compliance::routes())
            .add_route(controllers::fhir::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    /// Add this service's state and layers to the router.
    ///
    /// # Errors
    ///
    /// Fails when the service's setup fails.
    pub async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Seed the process-wide PASETO verifier before the app serves
        // traffic: when `CASE_PASETO_KEYS_URL` is set the published key
        // set is fetched over HTTP once at boot (fetch failure falls back
        // to the `CASE_PASETO_KEYS` env path — the service always boots).
        auth::init().await;
        // Periodically re-fetch the published key set so a key rotation
        // is picked up without a restart (no-op unless
        // `CASE_PASETO_KEYS_URL` is set; disable with
        // `CASE_PASETO_KEYS_REFRESH_SECS=0`).
        auth::spawn_key_refresh();
        // Hot-reload the ABAC policy when its file changes (no-op unless
        // `CASE_ABAC_POLICY_FILE` is set) so operators can edit the
        // policy without a restart.
        auth::spawn_policy_watcher();
        // Durable event bus Phase 3: start the outbox relay loop. A no-op
        // unless `CASE_EVENT_TRANSPORT=outbox` AND `CASE_EVENT_RELAY` are
        // set, so the default `memory` transport never spawns it.
        crate::relay::spawn(ctx.db.clone());
        // Rebuild the full-text index if it is empty while records exist
        // — the upgrade path for a deployment whose data predates the
        // index, and the recovery path for an index directory that did
        // not survive a restart. A no-op on a normal boot.
        crate::tasks::search::spawn_reindex_if_empty(ctx.db.clone());
        // Blanket auth enforcement layer (authn + ABAC authz). Added
        // unconditionally; the
        // `CASE_REQUIRE_AUTH` flag is read per request and the layer is a
        // near-noop when the flag is off (the default).
        let router = router
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn(
                axa::versioning::require_version_mw,
            ));
        // Outermost of these, so the metric and `traceparent` cover
        // versioning and the auth guard too: a 401 or 403 is measured.
        Ok(Self::with_telemetry(router, TELEMETRY.get()))
    }

    /// Register background workers.
    ///
    /// # Errors
    ///
    /// Fails when a worker cannot be registered.
    pub fn connect_workers(ctx: &AppContext, queue: &mut Queue) -> Result<()> {
        queue.register(crate::bulk::worker::BulkJobWorker::build(ctx));
        Ok(())
    }

    #[allow(unused_variables)]
    /// Register command-line tasks.
    pub fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::search::SearchReindex);
        tasks.register(crate::tasks::integrity_key::IntegrityKey);
        tasks.register(crate::tasks::integrity_resign::IntegrityResign);
        tasks.register(crate::tasks::seed_examples::SeedExamples);
        // tasks-inject (do not remove)
    }

    /// Empty the tables.
    ///
    /// # Errors
    ///
    /// Propagates the database error.
    pub async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, entity_links::Entity).await?;
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, cases::Entity).await?;
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
        database: "case_service",
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
