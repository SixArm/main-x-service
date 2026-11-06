//! Application wiring: configuration, routes, the command line, and the
//! boot used by tests. Everything here is an ordinary function; the router
//! that [`router`] returns is a plain Axum router.

use axa::{
    AppContext, Environment, Error, Result,
    boot::{BootResult, TestApp},
    cli::{Cli, Command, parse_vars},
    config::{Config, Defaults},
    middleware,
    routes::AppRoutes,
    server,
    task::Tasks,
};
use axum::Router;

use crate::{auth::AuthConfig, controllers, initializers, tasks};

/// Service defaults. An empty `database` means this service has none: all
/// of its data lives in the upstream services it calls.
fn defaults() -> Defaults {
    Defaults {
        database: "",
        port: 5150,
        ..Defaults::default()
    }
}

/// The service's route table.
///
/// Starts from the shared `/_ping`, `/_health`, and `/_readiness` routes and
/// mounts each controller:
///
/// - `healthz` — `GET /healthz` liveness probe (no downstream calls).
/// - `auth` — `/api/auth/*` passwordless magic-link sign-in.
/// - `alerts` — `GET /api/alerts` geofence-breach alerts.
/// - `stats` — `/api/stats` dashboard counters/aggregates.
/// - `folders` — `/api/folders` case-folder CRUD + current location.
/// - `moves` — `/api/moves` scan/move log (the audit trail).
/// - `patients` — `/api/patients` patient lookups (upstream-backed).
/// - `places` — `/api/places` building/room/cabinet hierarchy.
/// - `volumes` — `/api/volumes` per-folder physical volumes.
/// - `workers` — `/api/workers` worker lookups (upstream-backed).
fn routes() -> AppRoutes {
    AppRoutes::with_default_routes()
        .add_route(controllers::healthz::routes())
        .add_route(controllers::auth::routes())
        .add_route(controllers::alerts::routes())
        .add_route(controllers::stats::routes())
        .add_route(controllers::folders::routes())
        .add_route(controllers::moves::routes())
        .add_route(controllers::patients::routes())
        .add_route(controllers::places::routes())
        .add_route(controllers::volumes::routes())
        .add_route(controllers::workers::routes())
}

/// Build the full router.
///
/// **Order is load-bearing.** The five upstream clients are layered first,
/// then the optional offline stubs, and the session guard last so it wraps
/// every route. The standard middleware goes outside all of them.
///
/// # Errors
///
/// Fails when authentication is misconfigured or the stubs cannot seed.
pub async fn router(ctx: &AppContext) -> Result<Router> {
    let auth = AuthConfig::from_env(&ctx.environment)?;
    let router = routes().into_router(ctx.clone());
    let router = initializers::main_event_service_client::install(router);
    let router = initializers::main_patient_service_client::install(router);
    let router = initializers::main_place_service_client::install(router);
    let router = initializers::main_thing_service_client::install(router);
    let router = initializers::main_worker_service_client::install(router);
    initializers::bootstrap_stubs::install().await?;
    let router = initializers::auth::install(router, auth);
    middleware::apply(router, &ctx.config.http)
}

/// The tasks this service offers.
fn tasks() -> Tasks {
    let mut tasks = Tasks::default();
    tasks.register(tasks::seed::Seed);
    tasks
}

/// Run the command line.
///
/// # Errors
///
/// Fails when boot, a task, or the server fails.
pub async fn run(cli: Cli) -> Result<()> {
    let environment = cli
        .environment
        .as_deref()
        .map_or_else(Environment::from_env, Environment::parse);
    // `check` reads settings only, so it runs before anything connects.
    if matches!(cli.command, Command::Check) {
        return axa::cli::check(&environment, &defaults());
    }
    let mut config = Config::from_env(&environment, &defaults())?;
    axa::logger::init(env!("CARGO_CRATE_NAME"), &config.logger);
    match cli.command {
        Command::Check => Ok(()),
        Command::Start { binding, port, .. } => {
            if let Some(binding) = binding {
                config.server.binding = binding;
            }
            if let Some(port) = port {
                config.server.port = port;
            }
            let server_config = config.server.clone();
            let ctx = AppContext::new(environment, (), config);
            server::serve(router(&ctx).await?, &server_config).await
        }
        Command::Task { name, params } => {
            let tasks = tasks();
            let Some(name) = name else {
                for task in tasks.list() {
                    println!("{}\t{}", task.name, task.detail);
                }
                return Ok(());
            };
            let ctx = AppContext::new(environment, (), config);
            tasks.run(&name, &ctx, &parse_vars(&params)?).await
        }
        Command::Db { .. } => Err(Error::string("this service has no database")),
    }
}

/// The application type tests boot through `axa::testing`.
pub struct App;

impl TestApp for App {
    async fn boot_test() -> Result<BootResult> {
        let environment = Environment::Test;
        let config = Config::from_env(&environment, &defaults())?;
        let ctx = AppContext::new(environment, (), config);
        let router = router(&ctx).await?;
        Ok(BootResult {
            app_context: ctx,
            router: Some(router),
        })
    }
}
