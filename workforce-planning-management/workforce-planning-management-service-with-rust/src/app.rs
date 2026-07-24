//! loco.rs application wiring: the [`App`] `Hooks` implementation that
//! registers routes, the blanket auth guard, the API-version
//! middleware, and the truncate lifecycle for
//! `workforce-planning-management-service`.

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

use crate::{auth, controllers, models::_entities::prelude::*, tasks};

/// Blanket auth-enforcement middleware: reads `WPM_REQUIRE_AUTH` per
/// request and delegates to the pure [`auth::enforce`] (public paths
/// and the disabled flag pass through; otherwise a valid bearer PASETO
/// is required (`401`) and its `attrs` must satisfy the ABAC policy
/// for the derived action (`403`)). Off by default — see `auth.rs` and
/// `agents/share/security.md` §4.
async fn require_auth_mw(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let method = req.method().clone();
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

/// The loco.rs application hooks for `workforce-planning-management-service`.
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
        Ok(vec![])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes()
            .add_route(controllers::hr_core::routes())
            .add_route(controllers::acquisition::routes())
            .add_route(controllers::assessments::routes())
            .add_route(controllers::workforce::routes())
            .add_route(controllers::development::routes())
            .add_route(controllers::learning::routes())
            .add_route(controllers::talent::routes())
            .add_route(controllers::intelligence::routes())
            .add_route(controllers::payroll::routes())
            .add_route(controllers::wellbeing::routes())
            .add_route(controllers::audits::routes())
            .add_route(controllers::docs::routes())
            .add_route(controllers::metrics::routes())
    }

    async fn after_routes(router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        // Seed the PASETO verifier (boot-time key fetch when
        // `WPM_PASETO_KEYS_URL` is set; env fallback — the service
        // always boots), then keep keys + policy fresh.
        auth::init().await;
        auth::spawn_key_refresh();
        auth::spawn_policy_watcher();
        Ok(router
            .layer(axum::middleware::from_fn(require_auth_mw))
            .layer(axum::middleware::from_fn(
                crate::version::require_version_mw,
            )))
    }

    async fn connect_workers(_ctx: &AppContext, _queue: &Queue) -> Result<()> {
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        tasks.register(tasks::seed::Seed);
        // tasks-inject (do not remove)
    }

    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, EventOutbox).await?;
        truncate_table(&ctx.db, AuditLogs).await?;
        truncate_table(&ctx.db, EntitlementAcknowledgements).await?;
        truncate_table(&ctx.db, WellbeingEntitlements).await?;
        truncate_table(&ctx.db, ProgramPlacements).await?;
        truncate_table(&ctx.db, EarlyCareerPrograms).await?;
        truncate_table(&ctx.db, PipelineMembers).await?;
        truncate_table(&ctx.db, TalentPipelines).await?;
        truncate_table(&ctx.db, DevelopmentPlanItems).await?;
        truncate_table(&ctx.db, DevelopmentPlans).await?;
        truncate_table(&ctx.db, AssessmentResults).await?;
        truncate_table(&ctx.db, Assessments).await?;
        truncate_table(&ctx.db, AssessmentInstruments).await?;
        truncate_table(&ctx.db, Benchmarks).await?;
        truncate_table(&ctx.db, Payslips).await?;
        truncate_table(&ctx.db, PayrollRuns).await?;
        truncate_table(&ctx.db, SuccessionCandidates).await?;
        truncate_table(&ctx.db, SuccessionPlans).await?;
        truncate_table(&ctx.db, TrainingEnrollments).await?;
        truncate_table(&ctx.db, FeedbackEntries).await?;
        truncate_table(&ctx.db, Goals).await?;
        truncate_table(&ctx.db, Reviews).await?;
        truncate_table(&ctx.db, ReviewCycles).await?;
        truncate_table(&ctx.db, BenefitEnrollments).await?;
        truncate_table(&ctx.db, BenefitPlans).await?;
        truncate_table(&ctx.db, ShiftAssignments).await?;
        truncate_table(&ctx.db, Shifts).await?;
        truncate_table(&ctx.db, LeaveRequests).await?;
        truncate_table(&ctx.db, LeaveEntitlements).await?;
        truncate_table(&ctx.db, TimeEntries).await?;
        truncate_table(&ctx.db, OnboardingItems).await?;
        truncate_table(&ctx.db, Interviews).await?;
        truncate_table(&ctx.db, Applications).await?;
        truncate_table(&ctx.db, Candidates).await?;
        truncate_table(&ctx.db, Requisitions).await?;
        truncate_table(&ctx.db, Employees).await?;
        Ok(())
    }

    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
