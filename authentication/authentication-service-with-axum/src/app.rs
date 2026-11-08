//! Application wiring: routes, tasks, table truncation, the command line,
//! and the boot used by tests. The auth service mounts its controller route
//! groups (magic-link auth, the published PASETO key set, docs, Prometheus
//! metrics) and otherwise keeps the surface minimal.

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

use crate::controllers;

/// axa application type. Implements [`Hooks`] to wire up routes,
/// workers, tasks, seeding, and table truncation for the auth service.
pub struct App;
impl App {
    /// Register the HTTP routes: axa's defaults plus our four
    /// controller groups — magic-link auth (`/api/auth`), the public
    /// key set (`/.well-known/paseto-keys`), the docs (`/api-docs` +
    /// `/swagger-ui`), and the Prometheus metrics (`/metrics.prom`).
    #[must_use]
    pub fn routes(_ctx: &AppContext) -> AppRoutes {
        let routes = AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::auth::routes())
            .add_route(controllers::compliance::routes())
            .add_route(controllers::admin::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes());
        // The published PASETO key set exists only with the `paseto`
        // feature; without it `/.well-known/paseto-keys` is a 404.
        #[cfg(feature = "paseto")]
        let routes = routes.add_route(controllers::paseto_keys::routes());
        // EV-2: SAML/OIDC identity federation, opt-in via the `oidc`
        // Cargo feature (see `src/oidc.rs`).
        #[cfg(feature = "oidc")]
        let routes = routes.add_route(controllers::oidc::routes());
        routes
    }

    /// Layer the header-based API-versioning middleware (spec §13 T-13)
    /// onto the router axa just built. This crate's only
    /// router-construction surface — unlike the person-style crates,
    /// there is no second hand-rolled router anywhere in `src/`/`tests/`.
    ///
    /// # Errors
    ///
    /// Never returns an error today; the signature is axa's.
    pub fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        let router = router.layer(axum::middleware::from_fn(
            axa::versioning::require_version_mw,
        ));
        // Outside the routes, so the lockout counts the 401s their
        // session and token checks give.
        crate::client_limits::from_env(router)
    }

    /// Register axa CLI tasks. `user_attributes` is the operator surface
    /// for ABAC attribute assignment (`agents/share/authorization-attributes.md`
    /// §6).
    pub fn register_tasks(tasks: &mut Tasks) {
        tasks.register(crate::tasks::attributes::UserAttributes);
        // tasks-inject (do not remove)
    }

    /// Truncate all application tables (used by the test harness between
    /// runs). Order matters only insofar as we clear the audit/session
    /// children before users; none carry FK constraints that forbid it.
    ///
    /// # Errors
    ///
    /// Propagates any truncate failure from the database.
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
        database: "authentication_service",
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
    pub fn router(ctx: &AppContext) -> Result<AxumRouter> {
        let router = Self::routes(ctx).into_router(ctx.clone());
        let router = Self::after_routes(router, ctx)?;
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
        // SEC-A1: stop here rather than on the first token.
        #[cfg(feature = "paseto")]
        crate::auth::ensure_signing_seed(environment.is_production_like())
            .map_err(|error| Error::Message(error.to_string()))?;
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
                crate::compliance::address_retention::spawn(ctx.db.clone())?;
                let router = Self::router(&ctx)?;
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
        let router = Self::router(&ctx)?;
        Ok(BootResult {
            app_context: ctx,
            router: Some(router),
        })
    }
}
