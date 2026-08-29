//! Loco application wiring: the [`App`] type and its `Hooks` impl.
//!
//! This is the composition root. `Hooks` is where loco asks the
//! application to register its routes, background workers, CLI tasks,
//! seed data, and table-truncation logic. The auth service mounts four
//! controller route groups (magic-link auth, the published PASETO key
//! set, docs, Prometheus metrics) and otherwise keeps the surface
//! minimal.

use crate::migration::Migrator;
use async_trait::async_trait;
use loco_rs::{
    Result,
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{BootResult, StartMode, create_app},
    config::Config,
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    task::Tasks,
};
use std::path::Path;

#[allow(unused_imports)]
use crate::{
    controllers,
    models::_entities::{auth_events, sessions, users},
    tasks,
};

/// Loco application type. Implements [`Hooks`] to wire up routes,
/// workers, tasks, seeding, and table truncation for the auth service.
pub struct App;
#[async_trait]
impl Hooks for App {
    /// Loco application name (the crate name), used in logs and banners.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    /// Human-readable version: the crate version plus the build commit
    /// SHA when one was injected at build time (`BUILD_SHA` /
    /// `GITHUB_SHA`), else `dev`. Surfaced in startup logs so a deployed
    /// instance is traceable to a commit.
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    /// Boot the loco app for the given run mode/environment/config,
    /// binding our `Migrator` so `cargo loco db migrate` runs this
    /// crate's migrations.
    ///
    /// # Errors
    ///
    /// Propagates any loco boot error (config load, DB connect, …).
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// App initializers. None are needed; key material is lazily loaded
    /// on first use (see [`crate::auth::keys`]) rather than via an
    /// initializer.
    ///
    /// # Errors
    ///
    /// Never returns an error today; the signature is loco's.
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![])
    }

    /// Register the HTTP routes: loco's defaults plus our four
    /// controller groups — magic-link auth (`/api/auth`), the public
    /// key set (`/.well-known/paseto-keys`), the docs (`/api-docs` +
    /// `/swagger-ui`), and the Prometheus metrics (`/metrics.prom`).
    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::auth::routes())
            .add_route(controllers::compliance::routes())
            .add_route(controllers::admin::routes())
            .add_route(controllers::paseto_keys::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }
    /// Register background workers on the Postgres-backed queue. None are
    /// wired today; the queue config stays in place for a future worker.
    ///
    /// # Errors
    ///
    /// Propagates queue registration failures.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    /// Register loco CLI tasks. `user_attributes` is the operator surface
    /// for ABAC attribute assignment (`agents/share/authorization-attributes.md`
    /// §6).
    fn register_tasks(tasks: &mut Tasks) {
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
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, auth_events::Entity).await?;
        truncate_table(&ctx.db, sessions::Entity).await?;
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }
    /// Seed the database from `base/users.yaml` (loco seed convention).
    ///
    /// # Errors
    ///
    /// Propagates seed/parse/insert failures.
    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        db::seed::<users::ActiveModel>(&ctx.db, &base.join("users.yaml").display().to_string())
            .await?;
        Ok(())
    }
}
