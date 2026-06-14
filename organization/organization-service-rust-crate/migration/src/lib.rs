//! `sea-orm-migration` migration set for the organization service.
//!
//! Defines the three tables the service persists to — `organizations`,
//! `audit_logs`, `merge_records` — and exposes [`Migrator`], which loco's
//! CLI (`db migrate`) and the request-test harness run in order.

#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_organizations;
mod m20220101_000002_audit_logs;
mod m20220101_000003_merge_records;

/// The migration runner for this crate. Lists every migration in apply
/// order; loco's CLI and the test harness drive it via `MigratorTrait`.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered list of migrations to apply (oldest first). The
    /// `inject-above` marker lets the loco generator splice new
    /// migrations in without hand-editing this list.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_organizations::Migration),
            Box::new(m20220101_000002_audit_logs::Migration),
            Box::new(m20220101_000003_merge_records::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
