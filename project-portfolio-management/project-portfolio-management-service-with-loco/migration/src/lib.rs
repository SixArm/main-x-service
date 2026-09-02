//! `sea-orm-migration` schema migrations for `project-portfolio-management-service`.
//!
//! The [`Migrator`] runs the ordered list below at boot (or via
//! `cargo loco db migrate`): the `plans` table (the registry across
//! the four collections), then the `audit_logs` and `merge_records` side
//! tables. The audit trail these create is the who/what/when record over
//! every change (lead / assignee / member references are personal data).

// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_plans;
mod m20220101_000002_audit_logs;
mod m20220101_000003_merge_records;
mod m20220101_000004_event_outbox;
mod m20220101_000005_governance;
mod m20220101_000006_visibility;
mod m20220101_000007_strategy;
mod m20260719_000002_insight_columns;
mod m20260719_000003_insight_snapshots;
mod m20260720_000001_engineering;
mod m20260720_000002_engineering_moderate;
mod m20260722_000001_capabilities;
mod m20260728_000001_integrity_digests;
mod m20260823_000001_time_based_analysis;
mod m20260825_000001_total_project_control;
mod m20260825_000002_controls;
mod m20260825_000003_phase_transitions;
mod m20260825_000004_flow_type;
mod m20260825_000005_workflows;
mod m20260825_000006_key_results;
mod m20260826_000001_effort;
mod m20260826_000002_ceremonies;
mod m20260826_000003_value;
mod m20260902_000001_automation_multi_action;
mod m20260902_000002_automation_milestone_fires;

/// The crate's migrator: drives the ordered migration set for the loco
/// CLI / boot-time migration.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered migration set. Order matters: `plans` is created
    /// first because the side tables reference its `pid`, then
    /// `audit_logs` and `merge_records`.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_plans::Migration),
            Box::new(m20220101_000002_audit_logs::Migration),
            Box::new(m20220101_000003_merge_records::Migration),
            Box::new(m20220101_000004_event_outbox::Migration),
            Box::new(m20220101_000005_governance::Migration),
            Box::new(m20220101_000006_visibility::Migration),
            Box::new(m20220101_000007_strategy::Migration),
            Box::new(m20260719_000002_insight_columns::Migration),
            Box::new(m20260719_000003_insight_snapshots::Migration),
            Box::new(m20260720_000001_engineering::Migration),
            Box::new(m20260720_000002_engineering_moderate::Migration),
            Box::new(m20260722_000001_capabilities::Migration),
            Box::new(m20260728_000001_integrity_digests::Migration),
            Box::new(m20260823_000001_time_based_analysis::Migration),
            Box::new(m20260825_000001_total_project_control::Migration),
            Box::new(m20260825_000002_controls::Migration),
            Box::new(m20260825_000003_phase_transitions::Migration),
            Box::new(m20260825_000004_flow_type::Migration),
            Box::new(m20260825_000005_workflows::Migration),
            Box::new(m20260825_000006_key_results::Migration),
            Box::new(m20260826_000001_effort::Migration),
            Box::new(m20260826_000002_ceremonies::Migration),
            Box::new(m20260826_000003_value::Migration),
            Box::new(m20260902_000001_automation_multi_action::Migration),
            Box::new(m20260902_000002_automation_milestone_fires::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
