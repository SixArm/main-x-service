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

    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        Ok(ctx)
    }

    async fn truncate(_ctx: &AppContext) -> Result<()> {
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::seed::Seed);
    }
}
