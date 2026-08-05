//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes and the truncate lifecycle for `link-graph-service`.
//!
//! Read-only to the world: the graph read controllers, `OpenAPI`/Swagger,
//! and Prometheus metrics are mounted (plus loco's default `/_health` /
//! `/_ping`); `after_routes` adds the blanket read guard and spawns the
//! periodic reconciliation worker when a source is configured. The live
//! **Fluvio bus consumer** is the remaining v1-deferred piece (spec §13).

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
    bgworker::Queue,
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

use crate::auth;
use crate::controllers;
use crate::models::_entities::{
    audit_log, consumer_offsets, edges, entity_presence, suggestion_runs,
};
use crate::observability::{self, Telemetry, TelemetryConfig};
use crate::reconcile;
use crate::suggest::job as suggest_job;

/// Blanket read-guard middleware (spec §9.4 / T-19). Delegates to the pure
/// [`auth::enforce`]: with `LINK_GRAPH_REQUIRE_AUTH` off, or on a public
/// path, it passes through; otherwise a valid bearer token is required
/// (`401`) and the token's `attrs` must satisfy the ABAC policy for a
/// `read` on the aggregator (`403`). The per-record `case↔person`
/// concealment (§10) stacks on top of this in the handlers.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    match auth::enforce(
        auth::require_auth(),
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

/// The installed telemetry providers, so `on_shutdown` can flush what the
/// batch processor is still holding. Set once by [`App::init_logger`].
static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

/// The loco.rs application hooks for `link-graph-service`.
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

    /// Install this crate's own logging + `OpenTelemetry` stack (spec §13
    /// T-22), returning `true` so loco does **not** also install its own.
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
        // export would be silent (observed, not assumed) unless widened.
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

    /// Register boot-time initializers — none for the v1 core.
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    /// Assemble the route table: loco's default routes (`/_health`,
    /// `/_ping`, …) plus the read-only graph controller (spec §9).
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::graph::routes())
            .add_route(controllers::compliance::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    /// Wrap the router in the blanket read-guard layer. The
    /// `LINK_GRAPH_REQUIRE_AUTH` flag is read per request inside
    /// [`require_auth_mw`], so the layer is a no-op until a deployment
    /// activates enforcement.
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        // Seed the verifier from the published key set before serving, then
        // keep it current: this aggregator previously had no boot fetch at
        // all, so its key set could only come from the environment. Both
        // loops are no-ops unless their source is configured
        // (`LINK_GRAPH_PASETO_KEYS_URL` / `LINK_GRAPH_ABAC_POLICY_FILE`).
        auth::init().await;
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        // Periodic reconciliation (design §8): pull each configured
        // service's authoritative entity_links and repair the read-model.
        // One worker per entity that sets `LINK_GRAPH_RECONCILE_URL_<ENTITY>`
        // (case ships `subject_of`; person + worker ship the two directions of
        // `same_identity`); a no-op for any entity without a configured source.
        for entity in ["case", "person", "worker"] {
            if let Some(source) = reconcile::HttpAuthoritativeSource::from_env_for(entity) {
                tokio::spawn(reconcile::run_periodic(ctx.db.clone(), source));
            }
        }
        // Cross-service `same_identity` suggestion (T-31, design §16
        // OQ-9(c)/(d)): pull person + worker, block + score, POST
        // `matcher_suggested` edges to person. A no-op unless
        // `LINK_GRAPH_SUGGEST_URL_PERSON` (+ `_WORKER`) is configured. The
        // job's own work is pure HTTP client traffic between two peers,
        // not a read-model repair — but since T-33 it also durably
        // records each completed pass's counts to `suggestion_runs`
        // (OQ-9(d) "audit every run's counts"), which is the one thing
        // here that does need a database handle.
        if let Some((persons, workers, sink)) = suggest_job::sources_from_env() {
            tokio::spawn(suggest_job::run_periodic(
                persons,
                workers,
                sink,
                ctx.db.clone(),
            ));
        }
        // Real bus consumption (BUS-2, spec §13 T-6): a no-op unless
        // `LINK_GRAPH_FLUVIO_ENDPOINT` is configured, so lazy verify-on-read
        // + reconciliation remain the read-model's integrity path with no
        // broker set up, exactly as before this task.
        crate::consumer::spawn(ctx.db.clone());
        // Outermost layer runs first, so the request span wraps the guard
        // too — a 401/403 is part of the trace, not invisible to it.
        Ok(router
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn(observability::trace_mw)))
    }

    /// Register background workers — none in the v1 core (the bus
    /// consumer and reconciliation worker are deferred, spec §13).
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    /// Register CLI tasks — none for the v1 core.
    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }

    /// Truncate all read-model tables for the loco test harness. These
    /// tables have no cross-table FKs, so order is unconstrained.
    ///
    /// # Errors
    ///
    /// Propagates DB truncation errors.
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, edges::Entity).await?;
        truncate_table(&ctx.db, entity_presence::Entity).await?;
        truncate_table(&ctx.db, consumer_offsets::Entity).await?;
        truncate_table(&ctx.db, audit_log::Entity).await?;
        truncate_table(&ctx.db, suggestion_runs::Entity).await?;
        Ok(())
    }

    /// Seed reference/fixture data — none (the read-model is derived).
    ///
    /// # Errors
    ///
    /// Infallible here; the signature is loco's.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
