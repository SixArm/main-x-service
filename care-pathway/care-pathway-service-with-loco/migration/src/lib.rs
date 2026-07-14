//! Database migrations for `care-pathway-service`.
//!
//! Driven by loco's CLI via [`Migrator`] (`sea-orm-migration`). The
//! migrations create the three tables: `care_pathways` (the entity, with
//! the `CarePathway` payload as JSONB), `audit_logs` (the CRUD trail), and
//! `merge_records` (the merge history).

// SEC-I3: migrators run pure SQL orchestration; forbid unsafe.
#![forbid(unsafe_code)]
#![allow(elided_lifetimes_in_paths)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_care_pathways;
mod m20220101_000002_audit_logs;
mod m20220101_000003_merge_records;
mod m20220101_000004_event_outbox;

/// The migration runner this crate exposes to loco / `sea-orm-migration`.
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    /// The ordered list of migrations to apply (oldest first). The
    /// inject marker is kept so `cargo loco generate migration` can splice
    /// new migrations in.
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_care_pathways::Migration),
            Box::new(m20220101_000002_audit_logs::Migration),
            Box::new(m20220101_000003_merge_records::Migration),
            Box::new(m20220101_000004_event_outbox::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
