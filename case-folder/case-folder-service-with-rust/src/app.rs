use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::Queue,
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;

use crate::{controllers, initializers, tasks};

/// Zero-sized application type that carries the Loco `Hooks`
/// implementation for the Case-Folder Service.
///
/// Loco is generic over a type implementing `Hooks`; `App` is that type
/// for this crate. It holds no state — all runtime state lives in the
/// `AppContext` Loco builds during boot. The binary
/// (`src/bin/main.rs`) hands `App` to `cli::main` to drive the whole
/// service.
pub struct App;

#[async_trait]
impl Hooks for App {
    /// The application's stable name, used by Loco for logging and CLI
    /// banners. Resolved at compile time from the crate name.
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    /// Human-readable version string: the crate's semver plus a build
    /// SHA.
    ///
    /// The SHA is read at compile time from `BUILD_SHA`, falling back to
    /// `GITHUB_SHA` (CI), and finally to the literal `"dev"` for local
    /// builds where neither is set. Surfaced in logs and the `routes`
    /// banner so a running instance can be traced back to a commit.
    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    /// Boot the application for the given start mode and environment.
    ///
    /// Delegates to Loco's `create_app`, which builds the `AppContext`
    /// (database connection, config, queue), runs initializers, mounts
    /// routes, and returns a `BootResult`. `StartMode` selects whether
    /// to bring up the server, workers, or both.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be reached, migrations
    /// (via `Migrator`) fail, or any initializer returns an error.
    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    /// Build the ordered list of boot-time initializers.
    ///
    /// **Order is load-bearing** — Loco runs them in sequence and each
    /// wraps the request pipeline / extends the context built so far:
    ///
    /// 1. The **five upstream service clients** (event, patient, place,
    ///    thing, worker) each install a `RoutingClient` extension so
    ///    controllers can reach the corresponding Main-X-Service.
    /// 2. **`StubsInitializer`** runs *after* those five so the
    ///    `RoutingClient` slots already exist; in dev/test it swaps the
    ///    real HTTP clients for in-memory stubs.
    /// 3. **`AuthInitializer`** runs **last** so its session guard
    ///    middleware wraps every other route mounted by `routes`.
    ///
    /// # Errors
    ///
    /// Propagates any error from constructing an initializer (none of
    /// the current ones fail here; they configure at apply time).
    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![
            Box::new(initializers::main_event_service_client::ClientInitializer),
            Box::new(initializers::main_patient_service_client::ClientInitializer),
            Box::new(initializers::main_place_service_client::ClientInitializer),
            Box::new(initializers::main_thing_service_client::ClientInitializer),
            Box::new(initializers::main_worker_service_client::ClientInitializer),
            // Must run *after* the five client initializers above so the
            // RoutingClient slot is in place when we swap in stubs.
            Box::new(initializers::bootstrap_stubs::StubsInitializer),
            // Last, so its session guard wraps every other route.
            Box::new(initializers::auth::AuthInitializer),
        ])
    }

    /// Assemble the full route table for the service.
    ///
    /// Starts from Loco's default routes (ping, request-id, etc.) and
    /// mounts each controller's sub-router:
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
    fn routes(_ctx: &AppContext) -> AppRoutes {
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

    /// Final hook to adjust the `AppContext` after it is built.
    ///
    /// This service has nothing to add, so it returns the context
    /// unchanged.
    ///
    /// # Errors
    ///
    /// Never errors in this implementation; the signature is fallible to
    /// satisfy the `Hooks` trait.
    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        Ok(ctx)
    }

    /// Truncate application tables (used by test harnesses between runs).
    ///
    /// No-op here: this consumer app keeps its data only in its own
    /// tables and the test suite manages fixtures itself.
    ///
    /// # Errors
    ///
    /// Never errors in this implementation.
    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }

    /// Seed the database from on-disk fixtures during `db seed`.
    ///
    /// No-op here; seeding is driven by the `tasks::seed::Seed` task
    /// instead (see `register_tasks`).
    ///
    /// # Errors
    ///
    /// Never errors in this implementation.
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }

    /// Register background-queue workers with the Loco queue.
    ///
    /// No-op: this service runs no background workers (move ingestion is
    /// synchronous via the API).
    ///
    /// # Errors
    ///
    /// Never errors in this implementation.
    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    /// Register CLI tasks runnable via `cargo loco task`.
    ///
    /// Registers the `Seed` task, which populates demo/development data.
    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::seed::Seed);
    }
}
