//! axa application wiring: the [`App`] `Hooks` implementation that
//! registers routes, background workers, and the truncate/seed
//! lifecycle for `care-pathway-service`.

use axa::{
    AppContext, Environment, Error, Result,
    boot::{BootResult, TestApp},
    cli::{Cli, Command, DbCommand, parse_vars},
    config::{Config as ServiceConfig, Defaults, MB},
    db::truncate_table,
    queue::{Queue, QueueOptions},
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
    auth, bulk, controllers,
    models::_entities::{audit_logs, care_pathways, merge_records},
    observability::{self, Telemetry, TelemetryConfig},
    tasks,
    workers::bulk_export::BulkExportWorker,
};

/// The installed telemetry providers, so `on_shutdown` can flush what the
/// batch processor is still holding. Set once by [`App::init_logger`].
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
        Err((status, msg)) => (status, msg).into_response(),
    }
}

/// The axa application hooks for `care-pathway-service`.
///
/// Implements [`Hooks`] to register routes, background workers, and the
/// truncate/seed lifecycle; the binary entrypoint drives it via the
/// axa CLI.
pub struct App;
impl App {
    /// Install this crate's own logging + `OpenTelemetry` stack (repo
    /// `tasks.md` PRO-H12), returning `true` so axa does **not** also
    /// install its own. Ported from organization-service's
    /// `App::init_logger` (itself course's, itself person's) — the
    /// `App::init_logger` seam is identical across crates' shapes
    /// despite the differing router layout (see `src/observability.rs`'s
    /// module docs and `after_routes` below for where this crate's shape
    /// actually diverges).
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
        Ok(router
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn(
                crate::version::require_version_mw,
            ))
            // Outermost layer runs first, so the request span wraps
            // versioning and the auth guard too — a 401/403 is part of the
            // trace, not invisible to it. This is this crate's only
            // router-construction surface — see `src/observability.rs`'s
            // module docs.
            .layer(axum::middleware::from_fn(observability::trace_mw)))
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
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, care_pathways::Entity).await?;
        Ok(())
    }
}

/// Service defaults. The environment overrides them; see `axa::config`.
fn defaults() -> Defaults {
    Defaults {
        database: "care_pathway_service",
        port: 5150,
        production_port: None,
        body_limit: 2 * MB,
        max_connections: 1,
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
            Command::Start { worker, all, .. } => {
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
        let config = ServiceConfig::from_env(&environment, &defaults())?;
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
