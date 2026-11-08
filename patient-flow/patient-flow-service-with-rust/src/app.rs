//! axa application wiring: the [`App`] `Hooks` implementation that
//! registers routes, the blanket auth guard, the API-version
//! middleware, and the truncate lifecycle for `patient-flow-service`.

use crate::{auth, controllers, tasks};
use axa::{
    AppContext, Environment, Error, Result,
    boot::{BootResult, TestApp},
    cli::{Cli, Command, DbCommand, parse_vars},
    config::{Config as ServiceConfig, Defaults, MB},
    routes::AppRoutes,
    server,
    task::Tasks,
};
use axum::{
    Router as AxumRouter,
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Blanket auth-enforcement middleware: reads `PATIENT_FLOW_REQUIRE_AUTH`
/// per request and delegates to [`auth::enforce_request`] (public
/// paths and the disabled flag pass through; otherwise a valid bearer
/// PASETO (or Keycloak access token) is required (`401`) and its `attrs` must satisfy the ABAC
/// policy for the derived action (`403`)). Off by default — see
/// `auth.rs` and `agents/share/security.md` §4.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
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

/// The axa application hooks for `patient-flow-service`.
pub struct App;
impl App {
    /// The service's route table.
    #[must_use]
    pub fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::topology::routes())
            .add_route(controllers::stays::routes())
            .add_route(controllers::bed_requests::routes())
            .add_route(controllers::boards::routes())
            .add_route(controllers::audits::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    /// Add this service's state and layers to the router.
    ///
    /// # Errors
    ///
    /// Fails when the service's setup fails.
    pub async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        // Seed the PASETO verifier (boot-time key fetch when
        // `PATIENT_FLOW_PASETO_KEYS_URL` is set; env fallback — the
        // service always boots), then keep keys + policy fresh.
        auth::init().await;
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        let router = router
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn(
                axa::versioning::require_version_mw,
            ));
        // After the auth guard, so the lockout counts its 401s.
        crate::client_limits::from_env(router)
    }

    /// Register command-line tasks.
    pub fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::seed::Seed);
        // tasks-inject (do not remove)
    }

    /// Empty the tables.
    ///
    /// # Errors
    ///
    /// Propagates the database error.
    pub async fn truncate(ctx: &AppContext) -> Result<()> {
        crate::models::delete_all_rows(&ctx.db).await?;
        Ok(())
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
        database: "patient_flow_service",
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
        axa::logger::init(env!("CARGO_CRATE_NAME"), &ctx.config.logger);
        if ctx.config.database.auto_migrate && !matches!(cli.command, Command::Db { .. }) {
            Self::migrate(&ctx).await?;
        }
        match cli.command {
            Command::Check => Ok(()),
            Command::Start { .. } => {
                let router = Self::router(&ctx).await?;
                server::serve(router, &ctx.config.server).await?;
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
