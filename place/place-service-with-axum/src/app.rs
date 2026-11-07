//! Application wiring: configuration, the router, the command line, and
//! the boot used by tests. Boot-time singletons are built in
//! [`App::after_routes`] and carried in shared state.

use axa::opentelemetry::Telemetry;
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
use std::sync::OnceLock;

use crate::{
    api::rest::{ApiDoc, AppState, auth, metrics_routes, places_routes},
    config::Config,
    matching::PlaceMatcher,
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
            axa::opentelemetry::Config::from_env("place-service", env!("CARGO_PKG_VERSION"));
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

    /// Register the application's controller routes: axa's default routes
    /// plus this service's `places_routes()` (the `/api` surface) and the
    /// root-level `metrics_routes()` (`/metrics.prom` scrape path).
    #[must_use]
    pub fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(places_routes())
            .add_route(crate::controllers::fhir::routes())
            .add_route(metrics_routes())
    }

    /// Post-routing hook: construct boot-time singletons and merge the
    /// hand-written Axum surface onto axa's router.
    ///
    /// Builds the domain `Config` from the environment, opens the Tantivy
    /// `SearchEngine`, constructs the `PlaceMatcher`, and stuffs the resulting
    /// `AppState` into the context's shared store so `FromRef` handler
    /// extraction can reach it. Then mounts Swagger UI, the blanket
    /// auth-enforcement middleware (default-off, gated by
    /// `PLACE_REQUIRE_AUTH` — read here, at construction, so changing the
    /// flag requires a restart). [`App::router`] adds axa's CORS layer outside it.
    ///
    /// The PASETO verifier is finalised **first** via
    /// [`crate::api::rest::state::boot_verifier`] — when
    /// `PLACE_PASETO_KEYS_URL` is set the key set is fetched over HTTP
    /// once, here, in async context (fetch failure falls back to the
    /// `PLACE_PASETO_KEYS` env path; the service always boots) —
    /// **before** the enforcement middleware and the shared store
    /// capture the state, so both router surfaces consult the fetched
    /// key set.
    ///
    /// # Errors
    ///
    /// Returns an axa error if config loading or search-index creation fails
    /// (mapped from the domain `Error` via its string form).
    pub async fn after_routes(router: AxumRouter, ctx: &AppContext) -> Result<AxumRouter> {
        let config = Config::from_env().map_err(|e| axa::Error::string(&e.to_string()))?;
        let search_engine = SearchEngine::new(&config.search.index_path)
            .map_err(|e| axa::Error::string(&e.to_string()))?;
        let matcher = PlaceMatcher::new(&config.matching);
        auth::init().await;
        // Key rotation and policy edits without a restart: both loops are
        // no-ops unless their source is configured (`PLACE_PASETO_KEYS_URL`
        // / `PLACE_ABAC_POLICY_FILE`).
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        let state = AppState::new(ctx.db.clone(), search_engine, matcher, config);
        let enforcement = auth::EnforcementState::from_env();
        ctx.shared_store.insert(state);
        // Durable event bus Phase 3: start the outbox relay worker. A no-op
        // unless PLACE_EVENT_TRANSPORT=outbox and PLACE_EVENT_RELAY are set,
        // so the default `memory` transport is unaffected.
        crate::relay::spawn(ctx.db.clone());
        let router = router
            .merge(axa::openapi::router(
                axa::openapi::spec_of::<ApiDoc>(),
                "Place Service API",
            ))
            .layer(axum::middleware::from_fn_with_state(
                enforcement,
                auth::require_auth_middleware,
            ))
            .layer(axum::middleware::from_fn(
                axa::versioning::require_version_mw,
            ));
        // Outermost of these, so the metric and `traceparent` cover
        // versioning and the auth guard too: a 401 or 403 is measured.
        // After the auth guard, so the lockout counts its 401s; inside the
        // telemetry layer, so a 429 is measured.
        let router = crate::client_limits::from_env(router)?;
        crate::cursor_key::init_from_env()?;
        Ok(Self::with_telemetry(router, TELEMETRY.get()))
    }
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
        database: "place_service",
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
        Ok(AppContext::new(environment, db, config))
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

    /// Apply pending migrations ([`crate::db::MIGRATOR`]).
    ///
    /// # Errors
    ///
    /// Fails when a migration fails, or an applied one no longer matches
    /// its file.
    pub async fn migrate(ctx: &AppContext) -> Result<()> {
        axa::sqlx::migrate(&ctx.db, &crate::db::MIGRATOR)
            .await
            .map_err(migrate_error)
    }

    /// Run a database command. `down`, `reset` and `truncate` destroy
    /// data, so `axa::cli::parse` has already refused them outside
    /// development and test.
    async fn db(ctx: &AppContext, command: DbCommand) -> Result<()> {
        let migrator = &crate::db::MIGRATOR;
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
            Self::migrate(&ctx).await?;
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
        Self::migrate(&ctx).await?;
        let router = Self::router(&ctx).await?;
        Ok(BootResult {
            app_context: ctx,
            router: Some(router),
        })
    }
}
