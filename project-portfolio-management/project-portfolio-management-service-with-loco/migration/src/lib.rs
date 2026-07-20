//! `sea-orm-migration` schema migrations for `project-portfolio-management-service`.
//!
//! The [`Migrator`] runs the ordered list below at boot (or via
//! `cargo loco db migrate`): the `work_items` table (the registry across
//! the four collections), then the `audit_logs` and `merge_records` side
//! tables. The audit trail these create is the who/what/when record over
//! every change (lead / assignee / member references are personal data).

// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_work_items;
mod m20220101_000002_audit_logs;
mod m20220101_000003_merge_records;
mod m20220101_000004_event_outbox;
mod m20220101_000005_governance;
mod m20220101_000006_visibility;
mod m20220101_000007_strategy;
mod m20260719_000002_insight_columns;
mod m20260719_000003_insight_snapshots;
mod m20260720_000001_engineering;

/// The crate's migrator: drives the ordered migration set for the loco
/// CLI / boot-time migration.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered migration set. Order matters: `work_items` is created
    /// first because the side tables reference its `pid`, then
    /// `audit_logs` and `merge_records`.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_work_items::Migration),
            Box::new(m20220101_000002_audit_logs::Migration),
            Box::new(m20220101_000003_merge_records::Migration),
            Box::new(m20220101_000004_event_outbox::Migration),
            Box::new(m20220101_000005_governance::Migration),
            Box::new(m20220101_000006_visibility::Migration),
            Box::new(m20220101_000007_strategy::Migration),
            Box::new(m20260719_000002_insight_columns::Migration),
            Box::new(m20260719_000003_insight_snapshots::Migration),
            Box::new(m20260720_000001_engineering::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
