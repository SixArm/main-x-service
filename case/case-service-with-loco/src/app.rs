//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, background workers, and the truncate/seed
//! lifecycle for `case-service`.

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
    models::_entities::{audit_logs, cases, entity_links, merge_records},
    observability::{self, Telemetry, TelemetryConfig},
    tasks,
};

/// The installed telemetry providers, so `on_shutdown` can flush what the
/// batch processor is still holding. Set once by [`App::init_logger`].
static TELEMETRY: OnceLock<Telemetry> = OnceLock::new();

/// Blanket auth-enforcement middleware. Reads the `CASE_REQUIRE_AUTH`
/// flag per request via [`auth::require_auth`] and delegates the decision
/// to the pure [`auth::enforce`]: public paths and the disabled flag pass
/// through; otherwise a valid bearer token is required (`401`) and the
/// token's `attrs` claim must satisfy the process-wide ABAC policy
/// ([`auth::policy`]) for the action derived from the method + path
/// (`403` with the deciding rule otherwise). Off by default (see
/// `auth.rs` and `agents/share/authorization-attributes.md`).
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
    // Snapshot the current (hot-reloadable) policy and verifier for this
    // request; a concurrent policy reload or key-set refresh does not
    // affect a decision/verification already in flight.
    let policy = auth::policy().current();
    let verifier = auth::verifier().current();
    match auth::enforce(
        auth::require_auth(),
        &method,
        &path,
        req.headers(),
        &verifier,
        &policy,
    ) {
        Ok(()) => next.run(req).await,
        Err((status, msg)) => (status, msg).into_response(),
    }
}

/// The loco.rs application hooks for `case-service`.
///
/// Implements [`Hooks`] to register routes, background workers, and the
/// truncate/seed lifecycle; the binary entrypoint drives it via the
/// loco CLI.
pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// Install this crate's own logging + `OpenTelemetry` stack (repo
    /// `tasks.md` PRO-H12), returning `true` so loco does **not** also
    /// install its own. Ported from care-pathway-service's
    /// `App::init_logger` (itself organization's, itself course's,
    /// itself person's) — the `Hooks::init_logger` seam is identical
    /// across crates' shapes despite the differing router layout (see
    /// `src/observability.rs`'s module docs and `after_routes` below for
    /// where this crate's shape actually diverges).
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

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::cases::routes())
            .add_route(crate::bulk::handlers::routes())
            .add_route(controllers::links::routes())
            .add_route(controllers::compliance::routes())
            .add_route(controllers::fhir::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
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
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue
            .register(crate::bulk::worker::BulkJobWorker::build(ctx))
            .await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::search::SearchReindex);
        tasks.register(crate::tasks::integrity_key::IntegrityKey);
        tasks.register(crate::tasks::integrity_resign::IntegrityResign);
        tasks.register(crate::tasks::seed_examples::SeedExamples);
        // tasks-inject (do not remove)
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, entity_links::Entity).await?;
        truncate_table(&ctx.db, merge_records::Entity).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        truncate_table(&ctx.db, cases::Entity).await?;
        Ok(())
    }
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
