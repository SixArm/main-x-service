//! `sea-orm-migration` migration set for the organization service.
//!
//! Defines the tables the service persists to — `organizations`,
//! `audit_logs`, `merge_records`, and `event_outbox` (the durable
//! event-bus Phase-2 hand-off buffer) — and exposes [`Migrator`], which
//! loco's CLI (`db migrate`) and the request-test harness run in order.

// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_organizations;
mod m20220101_000002_audit_logs;
mod m20220101_000003_merge_records;
mod m20220101_000004_event_outbox;

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
            Box::new(m20220101_000004_event_outbox::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
