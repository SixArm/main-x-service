//! Loco application hooks for the organization service.
//!
//! The `App` type implements loco's `Hooks`, the seam the CLI drives to boot the
//! service, register routes (`routes`), layer middleware (`after_routes`),
//! and truncate tables between tests (`truncate`). It carries no state.
//! The blanket auth enforcement layer (PASETO authentication + ABAC
//! authorization) is wired here via `require_auth_mw`.

use async_trait::async_trait;
use axum::{
    Router as AxumRouter,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use loco_rs::{
    Result,
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{BootResult, StartMode, create_app},
    config::Config,
    controller::AppRoutes,
    db::truncate_table,
    environment::Environment,
    task::Tasks,
};
use migration::Migrator;
use std::path::Path;
use std::sync::OnceLock;

use crate::{
    auth, controllers,
    models::_entities::{audit_logs, bulk_jobs, event_outbox, merge_records, organizations},
    observability::{self, Telemetry, TelemetryConfig},
};

/// The installed telemetry providers, so `on_shutdown` can flush what the
/// batch processor is still holding. Set once by [`App::init_logger`].
static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

/// Blanket auth-enforcement middleware: authentication (PASETO bearer)
/// then ABAC authorization. Reads the flag, verifier, and policy per
/// request (all cached `OnceLock`s), so the layer is wired
/// unconditionally and is a near-noop when `ORGANIZATION_REQUIRE_AUTH`
/// is off. See [`auth::enforce`] for the pure decision.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let decision = auth::enforce(
        auth::require_auth(),
        req.method(),
        req.uri().path(),
        req.headers(),
        // Per-request snapshots, so the refresh loop and the policy
        // watcher reach the guard too — not just the handlers.
        &auth::verifier().current(),
        &auth::policy().current(),
    );
    match decision {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}

/// The loco application type. Carries no state; it exists to implement
/// [`Hooks`], which loco's CLI drives to boot the service, register
/// routes, and run migrations.
pub struct App;
#[async_trait]
impl Hooks for App {
    /// The app name loco uses in logs and the CLI banner — the crate name.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    /// Human-readable version: the crate version plus the build SHA when
    /// available (`BUILD_SHA`/`GITHUB_SHA`), else `dev` for local builds.
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    /// Boot the app: delegate to loco's `create_app`, wiring this `App`'s
    /// hooks and the migration `Migrator` for the given start mode and
    /// environment.
    ///
    /// # Errors
    ///
    /// Propagates loco boot failures (config load, DB connect, migration).
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// Install this crate's own logging + `OpenTelemetry` stack (repo
    /// `tasks.md` PRO-H12), returning `true` so loco does **not** also
    /// install its own. Ported from course-service's `App::init_logger`
    /// (itself ported from person-service's, itself link-graph-service's)
    /// — the `Hooks::init_logger` seam is identical across crates' shapes
    /// despite the differing router layout (see `src/observability.rs`'s
    /// module docs and `after_routes` below for where this crate's shape
    /// actually diverges).
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

    /// App initializers run at boot. None are needed here; auth and the
    /// event publisher are process-wide `OnceLock`s, not loco state.
    ///
    /// # Errors
    ///
    /// Never (returns `Ok`); the signature is loco's.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    /// Register the controller route trees on top of loco's defaults
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
    fn routes(_ctx: &AppContext) -> AppRoutes {
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
    /// Never (returns `Ok`); the signature is loco's.
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
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
        Ok(router
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn(
                crate::version::require_version_mw,
            ))
            // Outermost layer runs first, so the request span wraps CORS
            // (absent here), versioning, and the auth guard too — a
            // 401/403 is part of the trace, not invisible to it (same
            // reasoning as person-service's and link-graph-service's
            // `after_routes`). This is this crate's only
            // router-construction surface — see `src/observability.rs`'s
            // module docs.
            .layer(axum::middleware::from_fn(observability::trace_mw)))
    }

    /// Register background-queue workers: the BLK-5 bulk import/export
    /// worker, which drains `bulk_jobs` under `queue.kind: Postgres`
    /// (`workers.mode: BackgroundQueue` in `config/*.yaml`; the loco
    /// scaffold's unrelated `DownloadWorker` stub was removed — see
    /// entity spec §13 T-12).
    ///
    /// # Errors
    ///
    /// Never (returns `Ok`); the signature is loco's.
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue
            .register(crate::bulk::worker::BulkJobWorker::build(ctx))
            .await?;
        Ok(())
    }

    /// Register CLI tasks. None are registered; the inject marker is kept
    /// so the loco generator can splice tasks in later.
    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
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
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, event_outbox::Entity).await?;
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, bulk_jobs::Entity).await?;
        truncate_table(&ctx.db, organizations::Entity).await?;
        Ok(())
    }
    /// Seed the database. No seed data for this service.
    ///
    /// # Errors
    ///
    /// Never (returns `Ok`); the signature is loco's.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
