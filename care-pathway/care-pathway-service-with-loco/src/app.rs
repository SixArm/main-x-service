//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, background workers, and the truncate/seed
//! lifecycle for `care-pathway-service`.

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

#[allow(unused_imports)]
use crate::{
    auth, controllers,
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
/// [`auth::enforce`]: when enforcement is off (the default) or the path
/// is public it is a near-noop; otherwise an absent/invalid bearer
/// token yields `401`, and a valid token the ABAC policy denies yields
/// `403` (see `agents/share/authorization-attributes.md`).
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    match auth::enforce(
        auth::require_auth(),
        req.method(),
        &path,
        req.headers(),
        // Per-request snapshots, so the refresh loop and the policy
        // watcher reach the guard too — not just the handlers.
        &auth::verifier().current(),
        &auth::policy().current(),
    ) {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}

/// The loco.rs application hooks for `care-pathway-service`.
///
/// Implements [`Hooks`] to register routes, background workers, and the
/// truncate/seed lifecycle; the binary entrypoint drives it via the
/// loco CLI.
pub struct App;
#[async_trait]
impl Hooks for App {
    /// The loco application name, taken from the crate name at compile time.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    /// Human-readable version string: the crate `version` plus the build
    /// commit SHA (`BUILD_SHA`/`GITHUB_SHA` at compile time, else `dev`).
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    /// Boot the application: delegates to loco's [`create_app`] with this
    /// crate's [`App`] hooks and the database [`Migrator`].
    ///
    /// # Errors
    ///
    /// Propagates any loco boot error (config, DB connection, …).
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// Install this crate's own logging + `OpenTelemetry` stack (repo
    /// `tasks.md` PRO-H12), returning `true` so loco does **not** also
    /// install its own. Ported from organization-service's
    /// `App::init_logger` (itself course's, itself person's) — the
    /// `Hooks::init_logger` seam is identical across crates' shapes
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

    /// Register boot-time initializers — none for the MVP.
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    /// Assemble the route table: loco's default routes (`/_health`,
    /// `/_ping`, …) plus the care-pathway controller, the API-docs
    /// controller, and the root-level Prometheus `/metrics.prom` endpoint.
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::insights::routes())
            .add_route(controllers::tba::pathway_routes())
            .add_route(controllers::instances::pathway_routes())
            .add_route(controllers::links::routes())
            .add_route(controllers::tba::routes())
            .add_route(controllers::instances::routes())
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
    /// Infallible here; the signature is loco's.
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
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

    /// Register background workers with the queue — currently just
    /// [`BulkExportWorker`].
    ///
    /// # Errors
    ///
    /// Propagates queue-registration errors.
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(BulkExportWorker::build(ctx)).await?;
        Ok(())
    }

    /// Register CLI tasks — none for the MVP (the inject marker is kept so
    /// `cargo loco generate task` can splice new tasks in).
    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::search::SearchReindex);
        tasks.register(crate::tasks::integrity_key::IntegrityKey);
        tasks.register(crate::tasks::integrity_resign::IntegrityResign);
        // tasks-inject (do not remove)
    }
    /// Truncate all tables for the loco test harness, in
    /// foreign-key-safe order: history/audit children first, then the
    /// `care_pathways` parent.
    ///
    /// # Errors
    ///
    /// Propagates DB truncation errors.
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, care_pathways::Entity).await?;
        Ok(())
    }
    /// Seed reference/fixture data — none for the MVP.
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
